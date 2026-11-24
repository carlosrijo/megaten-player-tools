//! The local log: a few small text files in `<data folder>\logs\`, so a problem can be looked at afterwards.
//!
//! * `imagine-<yyyyMMdd>-<HHmmss>-<ms>.log`, at most [`FILE_BYTES`] each; when the newest is full a new one starts, and
//!   only the newest [`FILES_KEPT`] are kept. So the log can never take more than about 5 MB of the disk.
//! * One line per event: `2026-11-24 03:41:22.123 INFO  [4321] text`. Levels: debug, info, warn, error (error is always
//!   written). The default level is info; `--log-level debug|info|warn` changes it.
//! * Never in the log: what a page command was asked (arguments can hold personal data), the game's memory contents,
//!   pictures. Paths have the Windows user name taken out as they are written (`redact::user_only`).
//! * The app and the command line both append to the same newest file (the file is shared; a line is one write).
//!
//! Writing never fails the program: if the folder or file can't be used, logging is simply off.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::Local;

use super::redact;

/// The most one log file holds (about 1 MB).
pub const FILE_BYTES: u64 = 1_000_000;
/// How many log files are kept.
pub const FILES_KEPT: usize = 5;
/// One line is cut to this many characters.
const LINE_CHARS: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn parse(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" | "warning" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO ",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// `--log-level` from a command line: the level, or an error text for the person when the word isn't one of the three.
pub fn level_from_args(args: &[String]) -> Result<Level, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--log-level" {
            return match args.get(i + 1) {
                Some(v) => match Level::parse(v) {
                    Some(l) if l != Level::Error => Ok(l),
                    _ => Err(format!("--log-level must be debug, info or warn, not '{v}'.")),
                },
                None => Err("--log-level needs debug, info or warn.".to_string()),
            };
        }
        i += 1;
    }
    Ok(Level::Info)
}

// ---------- the rolling files ----------

/// A folder of numbered log files with a size cap per file and a count cap (the part of the logger that can be tested
/// with small numbers).
pub struct RollingLog {
    dir: PathBuf,
    prefix: String,
    max_bytes: u64,
    keep: usize,
    file: Option<File>,
    path: PathBuf,
}

impl RollingLog {
    /// Open the newest file of the folder if it still has room (more than half free), else start a new one; then drop
    /// the files beyond `keep`.
    pub fn open(dir: &Path, prefix: &str, max_bytes: u64, keep: usize) -> std::io::Result<RollingLog> {
        std::fs::create_dir_all(dir)?;
        let mut log = RollingLog { dir: dir.to_path_buf(), prefix: prefix.to_string(), max_bytes, keep: keep.max(1), file: None, path: PathBuf::new() };
        let newest = log.files().pop();
        match newest {
            Some(p) if std::fs::metadata(&p).map(|m| m.len() < max_bytes / 2).unwrap_or(false) => log.attach(p)?,
            _ => log.start_new()?,
        }
        log.prune();
        Ok(log)
    }

    /// The log files of the folder, oldest first (their names sort by time).
    pub fn files(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        let n = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                        n.starts_with(&format!("{}-", self.prefix)) && n.ends_with(".log") && p.is_file()
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    pub fn current(&self) -> &Path {
        &self.path
    }

    fn attach(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.file = Some(OpenOptions::new().create(true).append(true).open(&path)?);
        self.path = path;
        Ok(())
    }

    fn start_new(&mut self) -> std::io::Result<()> {
        let mut now = Local::now();
        loop {
            let name = format!("{}-{}.log", self.prefix, now.format("%Y%m%d-%H%M%S-%3f"));
            let path = self.dir.join(&name);
            // after the current file too: a file pruned this same millisecond frees its name, and reusing it would
            // sort the new file before the current one, so pruning would keep the wrong one
            if !path.exists() && path > self.path {
                return self.attach(path);
            }
            now += chrono::Duration::milliseconds(1);
        }
    }

    fn prune(&mut self) {
        let files = self.files();
        if files.len() > self.keep {
            for old in &files[..files.len() - self.keep] {
                if *old != self.path {
                    let _ = std::fs::remove_file(old);
                }
            }
        }
    }

    /// Append one line (CRLF-ended); a full file is left and a new one started first.
    pub fn write_line(&mut self, line: &str) {
        let mut text = String::with_capacity(line.len() + 2);
        text.push_str(line);
        text.push_str("\r\n");
        let size = self.file.as_ref().and_then(|f| f.metadata().ok()).map(|m| m.len()).unwrap_or(0);
        if size > 0 && size + text.len() as u64 > self.max_bytes {
            if self.start_new().is_err() {
                return;
            }
            self.prune();
        }
        if let Some(f) = self.file.as_mut() {
            let _ = f.write_all(text.as_bytes());
        }
    }
}

// ---------- the program-wide logger ----------

struct Logger {
    level: Level,
    log: Mutex<RollingLog>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Start logging into `dir` at `level`, with `header` (what the program is: name, version) as the first line. Once per
/// program; false (and no logging) when the folder can't be used.
pub fn init(dir: &Path, level: Level, header: &str) -> bool {
    if LOGGER.get().is_some() {
        return true;
    }
    let Ok(log) = RollingLog::open(dir, "imagine", FILE_BYTES, FILES_KEPT) else { return false };
    let first = LOGGER.set(Logger { level, log: Mutex::new(log) }).is_ok();
    if first {
        write(Level::Info, &format!("=== {header}, pid {}, log level {} ===", std::process::id(), level.name()));
    }
    first
}

pub fn enabled(level: Level) -> bool {
    LOGGER.get().is_some_and(|l| level >= l.level)
}

/// The folder the log (and the crash reports) go to, once logging has started.
pub fn dir() -> Option<PathBuf> {
    LOGGER.get().and_then(|l| l.log.lock().ok().map(|g| g.dir.clone()))
}

fn line(level: Level, msg: &str) -> String {
    let msg = redact::user_only(msg);
    // one event, one line: a line break inside the text would look like a second event
    let mut flat: String = msg.chars().map(|c| if c == '\r' || c == '\n' || c == '\t' { ' ' } else { c }).take(LINE_CHARS).collect();
    if msg.chars().count() > LINE_CHARS {
        flat.push('…');
    }
    format!("{} {} [{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), level.label(), std::process::id(), flat)
}

pub fn write(level: Level, msg: &str) {
    let Some(l) = LOGGER.get() else { return };
    if level < l.level {
        return;
    }
    let text = line(level, msg);
    let mut g = l.log.lock().unwrap_or_else(|e| e.into_inner());
    g.write_line(&text);
}

/// Like [`write`] for a panic hook: gives up instead of waiting when something else holds the file.
pub fn write_now_or_never(level: Level, msg: &str) {
    let Some(l) = LOGGER.get() else { return };
    let text = line(level, msg);
    if let Ok(mut g) = l.log.try_lock() {
        g.write_line(&text);
    }
}

/// Run one step of a read, then log how it went and how long it took (info level): `what: how (12 ms)`. `how` says found / not
/// found and counts, never what was read.
pub fn timed<T>(what: &str, step: impl FnOnce() -> T, how: impl FnOnce(&T) -> String) -> T {
    let t = std::time::Instant::now();
    let v = step();
    if enabled(Level::Info) {
        let ms = t.elapsed().as_millis();
        write(Level::Info, &format!("{what}: {} ({ms} ms)", how(&v)));
    }
    v
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { if $crate::diag::log::enabled($crate::diag::log::Level::Debug) { $crate::diag::log::write($crate::diag::log::Level::Debug, &format!($($arg)*)) } };
}
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { if $crate::diag::log::enabled($crate::diag::log::Level::Info) { $crate::diag::log::write($crate::diag::log::Level::Info, &format!($($arg)*)) } };
}
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { if $crate::diag::log::enabled($crate::diag::log::Level::Warn) { $crate::diag::log::write($crate::diag::log::Level::Warn, &format!($($arg)*)) } };
}
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::diag::log::write($crate::diag::log::Level::Error, &format!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("imagine-log-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn total(files: &[PathBuf]) -> u64 {
        files.iter().map(|f| std::fs::metadata(f).unwrap().len()).sum()
    }

    #[test]
    fn rolls_over_at_the_size_cap_and_keeps_the_newest_files() {
        let dir = scratch("roll");
        let mut log = RollingLog::open(&dir, "imagine", 1000, 3).unwrap();
        let line = "x".repeat(98); // 100 bytes with the line end
        for _ in 0..100 {
            log.write_line(&line);
        }
        let files = log.files();
        assert_eq!(files.len(), 3, "only the newest three are kept");
        for f in &files {
            assert!(std::fs::metadata(f).unwrap().len() <= 1000, "{} is over the cap", f.display());
        }
        assert!(total(&files) <= 3000);
        // the newest file is the one being written
        assert_eq!(files.last().unwrap(), log.current());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_run_continues_a_file_with_room_and_starts_a_new_one_when_it_is_nearly_full() {
        let dir = scratch("reopen");
        let mut a = RollingLog::open(&dir, "imagine", 1000, 5).unwrap();
        a.write_line("first");
        let first = a.current().to_path_buf();
        drop(a);
        let mut b = RollingLog::open(&dir, "imagine", 1000, 5).unwrap();
        assert_eq!(b.current(), first, "less than half full: the same file goes on");
        b.write_line(&"y".repeat(700));
        drop(b);
        let c = RollingLog::open(&dir, "imagine", 1000, 5).unwrap();
        assert_ne!(c.current(), first, "more than half full: a new file");
        assert_eq!(c.files().len(), 2);
        let text = std::fs::read_to_string(&first).unwrap();
        assert!(text.starts_with("first\r\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_files_in_the_folder_are_left_alone() {
        let dir = scratch("others");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("crash-20261124-000000.txt"), "x").unwrap();
        std::fs::write(dir.join("notes.log"), "x").unwrap();
        let mut log = RollingLog::open(&dir, "imagine", 100, 1).unwrap();
        for _ in 0..20 {
            log.write_line(&"z".repeat(60));
        }
        assert_eq!(log.files().len(), 1);
        assert!(dir.join("crash-20261124-000000.txt").exists());
        assert!(dir.join("notes.log").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lines_are_one_per_event_with_the_user_name_out() {
        let l = line(Level::Warn, "a\r\nb at C:\\Users\\Someone\\x");
        assert!(l.contains("WARN "));
        assert!(l.ends_with("a  b at %USERPROFILE%\\x"), "{l}");
        assert!(!l.contains('\n'));
    }

    #[test]
    fn level_words_and_the_command_line() {
        assert_eq!(Level::parse("DEBUG"), Some(Level::Debug));
        assert_eq!(Level::parse("warning"), Some(Level::Warn));
        assert_eq!(Level::parse("loud"), None);
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(level_from_args(&a(&["doctor"])), Ok(Level::Info));
        assert_eq!(level_from_args(&a(&["--log-level", "debug", "x"])), Ok(Level::Debug));
        assert!(level_from_args(&a(&["--log-level", "loud"])).is_err());
        assert!(level_from_args(&a(&["--log-level"])).is_err());
    }
}
