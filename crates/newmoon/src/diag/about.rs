//! Which build this is (version, git commit, build date) and which Windows it runs on: what a bug report needs first.
//!
//! The two programs' build scripts work the version, commit and date out (`crates\newmoon\build_info.rs`, included by
//! each `build.rs`) and hand them over at start with [`init`]; everything else asks [`build_info`].

use std::sync::OnceLock;

use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::PCWSTR;

/// The build: what `build.rs` stamped into the program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInfo {
    /// "NewMoon Build Tracker" or "newmoon-reader"
    pub program: String,
    /// NEWMOON_VERSION when the build was told one, else the crate's version
    pub version: String,
    /// the short git commit, "-dirty" added when the source had changes, "unknown" without git
    pub commit: String,
    /// the build's date and time in UTC, "2026-11-24 03:41 UTC"
    pub built: String,
}

impl BuildInfo {
    /// "NewMoon Build Tracker 1.2.0 (commit 356636c, built 2026-11-24 03:41 UTC)"
    pub fn line(&self) -> String {
        format!("{} {} (commit {}, built {})", self.program, self.version, self.commit, self.built)
    }

    /// The same without the program's name: "1.2.0 (commit 356636c, built 2026-11-24 03:41 UTC)"
    pub fn short(&self) -> String {
        format!("{} (commit {}, built {})", self.version, self.commit, self.built)
    }
}

static INFO: OnceLock<BuildInfo> = OnceLock::new();

/// Hand over what the build script stamped (once, first thing in `main`).
pub fn init(info: BuildInfo) {
    let _ = INFO.set(info);
}

/// The build, or a plain "unknown" one when nothing handed it over (tests).
pub fn build_info() -> BuildInfo {
    INFO.get().cloned().unwrap_or_else(|| BuildInfo {
        program: "newmoon".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: "unknown".to_string(),
        built: "unknown".to_string(),
    })
}

// ---------- Windows ----------

/// The Windows this runs on, from the registry (the numbers a person finds in Settings > About and `winver`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsInfo {
    /// "Windows 11 Pro"
    pub name: String,
    /// "24H2" (None on older Windows 10)
    pub release: Option<String>,
    /// "10.0.26300.1234"
    pub version: String,
    /// the build number, 26300
    pub build: u32,
}

impl WindowsInfo {
    /// "Windows 11 Pro 24H2 (10.0.26300.1234), 64-bit"
    pub fn text(&self) -> String {
        let release = self.release.as_deref().map(|r| format!(" {r}")).unwrap_or_default();
        format!("{}{release} ({}), 64-bit", self.name, self.version)
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

const CURRENT_VERSION_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

fn reg_string(name: &str) -> Option<String> {
    let (key, name) = (wide(CURRENT_VERSION_KEY), wide(name));
    let mut buf = vec![0u16; 256];
    let mut bytes = (buf.len() * 2) as u32;
    // SAFETY: plain registry read into a buffer of the size passed.
    let r = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut bytes))
    };
    if r.is_err() {
        return None;
    }
    let n = (bytes as usize / 2).saturating_sub(1).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..n]);
    let s = s.trim_end_matches('\0').trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn reg_dword(name: &str) -> Option<u32> {
    let (key, name) = (wide(CURRENT_VERSION_KEY), wide(name));
    let mut v = 0u32;
    let mut bytes = 4u32;
    // SAFETY: plain registry read into a u32.
    let r = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut bytes))
    };
    r.is_ok().then_some(v)
}

pub fn windows_info() -> WindowsInfo {
    let major = reg_dword("CurrentMajorVersionNumber").unwrap_or(0);
    let minor = reg_dword("CurrentMinorVersionNumber").unwrap_or(0);
    let build = reg_string("CurrentBuildNumber").and_then(|b| b.parse::<u32>().ok()).unwrap_or(0);
    let ubr = reg_dword("UBR");
    let mut name = reg_string("ProductName").unwrap_or_else(|| "Windows".to_string());
    // Windows 11 still says "Windows 10 ..." in ProductName; its build numbers start at 22000
    if build >= 22000 && name.starts_with("Windows 10") {
        name = name.replacen("Windows 10", "Windows 11", 1);
    }
    let release = reg_string("DisplayVersion").or_else(|| reg_string("ReleaseId"));
    let version = if major == 0 && build == 0 {
        "unknown version".to_string()
    } else {
        format!("{major}.{minor}.{build}{}", ubr.map(|u| format!(".{u}")).unwrap_or_default())
    };
    WindowsInfo { name, release, version, build }
}

// ---------- text for bug reports ----------

/// The short plain-text block Settings copies for a bug report (and the first lines of about.txt).
pub fn report_block(webview2: Option<&str>) -> String {
    let b = build_info();
    let w = windows_info();
    let mut s = String::new();
    s.push_str(&format!("{} {}\r\n", b.program, b.version));
    s.push_str(&format!("Commit: {}\r\n", b.commit));
    s.push_str(&format!("Built: {}\r\n", b.built));
    s.push_str(&format!("Windows: {}\r\n", w.text()));
    if let Some(v) = webview2 {
        s.push_str(&format!("WebView2: {v}\r\n"));
    }
    s
}

/// The build script's own code (`crates\newmoon\build_info.rs`, included by each program's build.rs), tested here.
#[cfg(test)]
#[allow(dead_code)]
mod build_script {
    include!("../../build_info.rs");

    #[test]
    fn dates_in_utc() {
        assert_eq!(utc_minute(0), "1970-01-01 00:00");
        assert_eq!(utc_minute(951_782_400 + 3600 * 12 + 60 * 34), "2000-02-29 12:34");
        assert_eq!(utc_minute(1_795_491_660), "2026-11-24 03:41");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_say_what_they_should() {
        let b = BuildInfo { program: "NewMoon Build Tracker".into(), version: "1.2.0".into(), commit: "abc1234-dirty".into(), built: "2026-11-24 03:41 UTC".into() };
        assert_eq!(b.line(), "NewMoon Build Tracker 1.2.0 (commit abc1234-dirty, built 2026-11-24 03:41 UTC)");
        let w = WindowsInfo { name: "Windows 11 Pro".into(), release: Some("24H2".into()), version: "10.0.26100.1".into(), build: 26100 };
        assert_eq!(w.text(), "Windows 11 Pro 24H2 (10.0.26100.1), 64-bit");
    }

    #[test]
    fn this_pc_has_a_windows_version() {
        let w = windows_info();
        assert!(w.build > 0 && w.name.starts_with("Windows"), "{w:?}");
    }
}
