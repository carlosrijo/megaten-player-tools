//! Turning a crash report's frames (`BABEL.exe+0x1a2b3c`) into functions and lines, offline, with the `.pdb` of
//! the build that crashed (`tools\publish-portable.ps1` writes it to dist-symbols\, the release workflow keeps it as an
//! artifact). Uses the `dbghelp.dll` every Windows has: the exe is loaded at an invented base address, its `.pdb` is looked
//! for in the folder given (and beside the exe), and each offset is looked up. Used by `imagine-reader symbolize`.
//!
//! The exe and the `.pdb` must be the very build that crashed (same version and commit, see the crash report's
//! "Version:" line); dbghelp refuses a `.pdb` of another build.

use std::path::Path;

/// One frame line of a crash report: its number, the module's name and the offset from the module's base.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Frame {
    pub index: u32,
    pub module: String,
    pub offset: u64,
}

/// The frames listed in a crash report: lines like `  #3  BABEL.exe+0x1a2b3c`.
pub fn frames_in(report: &str) -> Vec<Frame> {
    let mut v = Vec::new();
    for line in report.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix('#') else { continue };
        let mut parts = rest.split_whitespace();
        let (Some(n), Some(at)) = (parts.next(), parts.next()) else { continue };
        let Some((module, off)) = at.rsplit_once("+0x") else { continue };
        if let (Ok(index), Ok(offset)) = (n.parse::<u32>(), u64::from_str_radix(off, 16)) {
            v.push(Frame { index, module: module.to_string(), offset });
        }
    }
    v
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Diagnostics::Debug::{
        IMAGEHLP_LINEW64, SYMBOL_INFOW, SYMOPT_LOAD_LINES, SYMOPT_UNDNAME, SymCleanup, SymFromAddrW, SymGetLineFromAddrW64,
        IMAGEHLP_MODULEW64, SymGetModuleInfoW64, SymInitializeW, SymLoadModuleExW, SymSetOptions, SymSetSearchPathW,
    };
    use windows::core::PCWSTR;

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    const BASE: u64 = 0x1_0000_0000;

    /// Function and `file:line` for each offset of the exe (None where dbghelp knows nothing). Err says why the exe or
    /// its `.pdb` could not be loaded.
    pub fn resolve(exe: &Path, pdb_dir: Option<&Path>, offsets: &[u64]) -> Result<Vec<Option<String>>, String> {
        // an invented handle stands for "the process": no real process is looked at
        let h = HANDLE(0x4e4d as *mut c_void);
        let exe_w = wide(exe.as_os_str());
        // SAFETY: plain dbghelp calls with buffers of the sizes passed; the session is closed at the end.
        unsafe {
            SymSetOptions(SYMOPT_LOAD_LINES | SYMOPT_UNDNAME);
            SymInitializeW(h, PCWSTR::null(), false).map_err(|e| format!("dbghelp could not start: {e}"))?;
            let result = (|| {
                let mut path = exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
                if let Some(d) = pdb_dir {
                    path = format!("{};{}", d.display(), path);
                }
                let path_w = wide(std::ffi::OsStr::new(&path));
                let _ = SymSetSearchPathW(h, PCWSTR(path_w.as_ptr()));
                let base = SymLoadModuleExW(h, None, PCWSTR(exe_w.as_ptr()), PCWSTR::null(), BASE, 0, None, Default::default());
                if base == 0 {
                    return Err(format!("{} could not be loaded by dbghelp ({})", exe.display(), windows::core::Error::from_thread()));
                }
                // symbols loaded? SymPdb (3) or SymDia (7); anything else means no .pdb matching this exe was found
                let mut info = IMAGEHLP_MODULEW64 { SizeOfStruct: size_of::<IMAGEHLP_MODULEW64>() as u32, ..Default::default() };
                let _ = SymGetModuleInfoW64(h, base, &mut info);
                if info.SymType.0 != 3 && info.SymType.0 != 7 {
                    return Err(format!(
                        "no .pdb that matches {} was found (looked in {path}). It must be the .pdb of that very build: the same version and commit as the report's Version line.",
                        exe.display()
                    ));
                }
                let mut out = Vec::new();
                for &off in offsets {
                    // a frame holds the address the call returns to, one past the call: look the call up
                    let addr = BASE + off.saturating_sub(1);
                    // a SYMBOL_INFOW with room for a 1000-character name
                    let mut buf = vec![0u8; size_of::<SYMBOL_INFOW>() + 2 * 1024];
                    let sym = buf.as_mut_ptr() as *mut SYMBOL_INFOW;
                    (*sym).SizeOfStruct = size_of::<SYMBOL_INFOW>() as u32;
                    (*sym).MaxNameLen = 1000;
                    let mut disp = 0u64;
                    let name = if SymFromAddrW(h, addr, Some(&mut disp), sym).is_ok() {
                        let n = (*sym).NameLen as usize;
                        let chars = std::slice::from_raw_parts((*sym).Name.as_ptr(), n.min(1000));
                        Some(String::from_utf16_lossy(chars))
                    } else {
                        None
                    };
                    let mut line = IMAGEHLP_LINEW64 { SizeOfStruct: size_of::<IMAGEHLP_LINEW64>() as u32, ..Default::default() };
                    let mut ldisp = 0u32;
                    let at = if SymGetLineFromAddrW64(h, addr, &mut ldisp, &mut line).is_ok() && !line.FileName.is_null() {
                        let mut len = 0;
                        while *line.FileName.0.add(len) != 0 {
                            len += 1;
                        }
                        Some(format!("{}:{}", String::from_utf16_lossy(std::slice::from_raw_parts(line.FileName.0, len)), line.LineNumber))
                    } else {
                        None
                    };
                    out.push(match (name, at) {
                        (Some(n), Some(a)) => Some(format!("{n}  ({a})")),
                        (Some(n), None) => Some(format!("{n}  (no line information)")),
                        (None, _) => None,
                    });
                }
                Ok(out)
            })();
            let _ = SymCleanup(h);
            result
        }
    }
}

#[cfg(windows)]
pub use imp::resolve;

/// The crash report with each `#n module+0xoffset` line followed by what the `.pdb` says about it.
pub fn symbolize(report: &str, exe: &Path, pdb_dir: Option<&Path>) -> Result<String, String> {
    let exe_name = exe.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let frames: Vec<Frame> = frames_in(report).into_iter().filter(|f| f.module.to_ascii_lowercase() == exe_name).collect();
    if frames.is_empty() {
        return Err(format!("The report has no frames in {exe_name}: lines like \"  #3  {exe_name}+0x1a2b3c\"."));
    }
    let found = resolve(exe, pdb_dir, &frames.iter().map(|f| f.offset).collect::<Vec<_>>())?;
    let mut out = String::new();
    for (f, what) in frames.iter().zip(found) {
        out.push_str(&format!("#{:<2} {}+0x{:x}  {}\n", f.index, f.module, f.offset, what.unwrap_or_else(|| "(not found in the symbols: wrong .pdb for this exe?)".to_string())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_read_from_a_report() {
        let r = "Frames (...):\r\n  #0  BABEL.exe+0x1a2b3c\r\n  #1  0x7ff812345678 (outside BABEL.exe)\r\n  #12 BABEL.exe+0x10\r\nBacktrace:\r\n   0: <unknown>\r\n";
        assert_eq!(
            frames_in(r),
            vec![Frame { index: 0, module: "BABEL.exe".into(), offset: 0x1a2b3c }, Frame { index: 12, module: "BABEL.exe".into(), offset: 0x10 }]
        );
        assert!(symbolize("nothing here", Path::new("X.exe"), None).is_err());
    }
}
