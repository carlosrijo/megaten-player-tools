//! Crash reports: when the program panics, `logs\crash-<yyyyMMdd-HHmmss>.txt` is written (what went wrong and where,
//! the version, Windows, a stack trace) and the panic is logged; the program then carries on as it did before (the
//! app shows its message box on its main thread, a page command that panicked answers "failed unexpectedly").
//!
//! The release exe ships without symbols, so the stack trace is written twice: the frames as `exe+0xOFFSET` (the
//! offsets can be turned into functions and lines later with the `.pdb` of the same build: `imagine-reader symbolize`,
//! see HANDOVER.md), and the runtime's own text trace, which already names functions and lines when the `.pdb` sits
//! beside the exe (a developer's build).

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::{Local, Utc};
use windows::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use super::{about, log, redact};

/// How many crash reports are kept in the folder.
pub const REPORTS_KEPT: usize = 10;
/// A report is cut to this many bytes.
const REPORT_BYTES: usize = 64 * 1024;

type Shown = Box<dyn Fn(&str, Option<&Path>) + Send + Sync>;

struct Hook {
    /// what to do on the main thread after the report is written: (the panic's text, the report file)
    on_main: Option<Shown>,
    /// where reports go when logging has not started (set by [`set_dir`])
    dir: std::sync::Mutex<Option<PathBuf>>,
}

static HOOK: OnceLock<Hook> = OnceLock::new();

/// Make panics write a report. `on_main`: called on the thread named "main" after the report is written (the app puts
/// its message box here); the old hook's console message is still printed by the runtime's default.
pub fn install(on_main: Option<Shown>) {
    if HOOK.set(Hook { on_main, dir: std::sync::Mutex::new(None) }).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = panic_text(info);
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let text = report(&message, &thread);
        log::write_now_or_never(log::Level::Error, &format!("PANIC on thread '{thread}': {message}"));
        let hook = HOOK.get();
        let dir = log::dir().or_else(|| hook.and_then(|h| h.dir.lock().ok().and_then(|d| d.clone())));
        let saved = dir.and_then(|d| write_report(&d, &text));
        if thread == "main" {
            if let Some(show) = hook.and_then(|h| h.on_main.as_ref()) {
                show(&message, saved.as_deref());
            }
        }
        previous(info); // the runtime's own message on the console, as before
    }));
}

/// Where crash reports go before logging has started (nothing is written when no place is known).
pub fn set_dir(dir: &Path) {
    if let Some(h) = HOOK.get() {
        if let Ok(mut d) = h.dir.lock() {
            *d = Some(dir.to_path_buf());
        }
    }
}

fn panic_text(info: &std::panic::PanicHookInfo<'_>) -> String {
    let what = if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "(no message)".to_string()
    };
    // as the runtime itself words it
    match info.location() {
        Some(l) => format!("panicked at {}:{}:{}:\n{what}", l.file(), l.line(), l.column()),
        None => format!("panicked:\n{what}"),
    }
}

/// The module's load address and the return addresses of the current stack, up to 64 frames.
fn raw_frames() -> (usize, Vec<usize>) {
    // SAFETY: GetModuleHandleW(None) is this exe; RtlCaptureStackBackTrace fills the array it is given and reports how many.
    unsafe {
        let base = GetModuleHandleW(None).map(|m| m.0 as usize).unwrap_or(0);
        let mut frames = [std::ptr::null_mut::<c_void>(); 64];
        let n = RtlCaptureStackBackTrace(0, &mut frames, None) as usize;
        (base, frames[..n].iter().map(|&p| p as usize).collect())
    }
}

/// The crash report text for a panic (`message`: its text and "panicked at file:line:col"), made on the panicking thread.
pub fn report(message: &str, thread: &str) -> String {
    let (base, frames) = raw_frames();
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let exe_name = Path::new(&exe).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "program.exe".to_string());
    let b = about::build_info();
    let mut s = String::new();
    s.push_str(&format!("{} crash report\r\n", b.program));
    s.push_str(&format!("Version: {}\r\n", b.short()));
    s.push_str(&format!("Time: {} local, {} UTC\r\n", Local::now().format("%Y-%m-%d %H:%M:%S"), Utc::now().format("%Y-%m-%d %H:%M:%S")));
    s.push_str(&format!("Windows: {}\r\n", about::windows_info().text()));
    s.push_str(&format!("Process: pid {}, thread '{thread}'\r\n", std::process::id()));
    s.push_str(&format!("Exe: {exe}\r\n"));
    s.push_str(&format!("Module base: 0x{base:x}\r\n\r\n"));
    s.push_str(&message.replace('\n', "\r\n"));
    s.push_str("\r\n\r\nFrames (the offset of each return address from the module base; turn them into functions and lines with the\r\n");
    s.push_str("same build's .pdb: imagine-reader symbolize <this file> --exe <exe> --pdb <folder>):\r\n");
    for (i, f) in frames.iter().enumerate() {
        match f.checked_sub(base).filter(|_| base != 0) {
            Some(off) if off < 0x4000_0000 => s.push_str(&format!("  #{i:<2} {exe_name}+0x{off:x}\r\n")),
            _ => s.push_str(&format!("  #{i:<2} 0x{f:x} (outside {exe_name})\r\n")),
        }
    }
    s.push_str("\r\nBacktrace (as the runtime reads it; names and lines only when the .pdb is beside the exe):\r\n");
    s.push_str(&std::backtrace::Backtrace::force_capture().to_string().replace('\n', "\r\n"));
    let mut s = redact::user_only(&s);
    if s.len() > REPORT_BYTES {
        let mut cut = REPORT_BYTES;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
        s.push_str("\r\n(cut)\r\n");
    }
    s
}

/// Write `crash-<yyyyMMdd-HHmmss>.txt` into `dir` (a number is added when that second has one already) and drop the
/// oldest reports beyond [`REPORTS_KEPT`]. The file's path, or None when it can't be written.
pub fn write_report(dir: &Path, text: &str) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut path = dir.join(format!("crash-{stamp}.txt"));
    let mut n = 1;
    while path.exists() {
        n += 1;
        path = dir.join(format!("crash-{stamp}-{n}.txt"));
    }
    std::fs::write(&path, text).ok()?;
    prune(dir, REPORTS_KEPT);
    Some(path)
}

/// The crash reports of a folder, oldest first.
pub fn reports(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                    n.starts_with("crash-") && n.ends_with(".txt") && p.is_file()
                })
                .collect()
        })
        .unwrap_or_default();
    // by time, then by the number added within one second (so -10 comes after -2)
    v.sort_by_key(|p| {
        let stem = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let rest = stem.strip_prefix("crash-").unwrap_or(&stem);
        let (stamp, n) = match rest.char_indices().filter(|(_, c)| *c == '-').nth(1) {
            Some((i, _)) => (rest[..i].to_string(), rest[i + 1..].parse::<u32>().unwrap_or(1)),
            None => (rest.to_string(), 1),
        };
        (stamp, n)
    });
    v
}

fn prune(dir: &Path, keep: usize) {
    let all = reports(dir);
    if all.len() > keep {
        for old in &all[..all.len() - keep] {
            let _ = std::fs::remove_file(old);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_has_the_version_the_message_and_frames() {
        let r = report("it broke\npanicked at crates\\x\\src\\y.rs:1:2", "main");
        assert!(r.contains("crash report"));
        assert!(r.contains("Version: "));
        assert!(r.contains("it broke\r\npanicked at crates\\x\\src\\y.rs:1:2"));
        assert!(r.contains("Module base: 0x"));
        assert!(r.contains("  #0 "), "{r}");
        assert!(r.contains("Backtrace (as the runtime reads it"));
        assert!(!r.contains(&std::env::var("USERPROFILE").unwrap_or_else(|_| "\u{1}".to_string())), "the profile folder is out");
    }

    #[test]
    fn reports_are_numbered_per_second_and_pruned() {
        let dir = std::env::temp_dir().join(format!("imagine-crash-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for _ in 0..(REPORTS_KEPT + 4) {
            let p = write_report(&dir, "x").unwrap();
            assert!(p.file_name().unwrap().to_string_lossy().starts_with("crash-"));
        }
        assert_eq!(reports(&dir).len(), REPORTS_KEPT);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
