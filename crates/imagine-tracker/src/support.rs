//! The page's troubleshooting commands: `about` (version block), `doctor` (the self-check), `saveBundle` (the support
//! bundle), `buildCheck` (is the game the build the reader was made for) and `log` (the page's JavaScript errors into the
//! local log). The work itself is in the library (`imagine::build::doctor`, `bundle`, `imagine::diag`); this is the
//! app's side: the WebView2 version, the app's settings file, the page's error reports and how much of them to take.

use anyhow::Result;
use imagine::build::{bundle, doctor};
use imagine::data::config;
use imagine::diag::{about, log};
use imagine::user_error;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::host;
use crate::settings::AppSettings;

/// The WebView2 line for the version block: the runtime's version, or why there is none.
fn webview2_text() -> String {
    match host::webview2_version() {
        Ok(v) => v,
        Err(why) => format!("not found ({why})"),
    }
}

/// Which build this is, which Windows and WebView2 it runs on, and where its files are; `text` is the short plain-text
/// block that Settings copies for a bug report.
pub fn about_json(data_dir: &Path, log_level: &str) -> Value {
    let b = about::build_info();
    let w = about::windows_info();
    json!({
        "program": b.program, "version": b.version, "commit": b.commit, "built": b.built,
        "windows": w.text(), "webview2": webview2_text(),
        "text": about::report_block(Some(&webview2_text())),
        "dataDir": data_dir.to_string_lossy(), "logDir": data_dir.join("logs").to_string_lossy(), "logLevel": log_level,
    })
}

/// One pass of the self-check. `no_writes`: a capture run on the person's own data, which must not even make the test file.
pub fn run_doctor(data_dir: &Path, no_writes: bool) -> doctor::Report {
    doctor::run(&doctor::Input {
        data_dir: data_dir.to_path_buf(),
        webview2: Some(host::webview2_version()),
        settings_path: Some(AppSettings::file_path()),
        probe_write: !no_writes,
        game: true,
    })
}

/// A fresh self-check and the zip written to `out`; the page's JSON for it.
pub fn save_bundle(data_dir: &Path, include_names: bool, out: &Path) -> Result<Value> {
    let report = run_doctor(data_dir, false);
    let input = bundle::Input {
        data_dir,
        config_path: config::file_path(),
        settings_path: Some(AppSettings::file_path()),
        include_names,
        report: &report,
        about: about::report_block(Some(&webview2_text())),
    };
    let s = bundle::write(out, &input)?;
    log::write(
        log::Level::Info,
        &format!("support bundle saved: {} files, {} bytes, character names {}", s.files.len(), s.zip_bytes, if include_names { "included" } else { "left out" }),
    );
    Ok(json!({
        "path": out.to_string_lossy(), "zipBytes": s.zip_bytes, "includeNames": include_names,
        "files": s.files.iter().map(|(n, b)| json!({ "name": n, "bytes": b })).collect::<Vec<_>>(),
    }))
}

/// Is every running client the build the memory layouts were made for? Reads the exes (cached), not the game's memory.
/// `unknown` lists the ones that are not (a changed exe counts too); the page warns once per `fingerprint`.
pub fn build_check() -> Value {
    let builds = doctor::client_builds();
    let known = builds.iter().filter(|b| matches!(b.result, Some(doctor::BuildMatch::Known(_)))).count();
    let unknown: Vec<Value> = builds
        .iter()
        .filter(|b| !matches!(b.result, Some(doctor::BuildMatch::Known(_))) && b.identity.is_some())
        .map(|b| {
            let id = b.identity.as_ref().expect("filtered");
            json!({
                "folder": b.folder, "pid": b.pid, "fingerprint": id.sha256.chars().take(16).collect::<String>(),
                "same": matches!(b.result, Some(doctor::BuildMatch::SameShape(_))),
            })
        })
        .collect();
    log::write(
        log::Level::Info,
        &format!("game clients: {} running, {} the known build, {} not", builds.len(), known, unknown.len()),
    );
    json!({ "clients": builds.len(), "known": known, "unknown": unknown, "warning": doctor::UPDATED_WARNING })
}

/// What the page reports about its own JavaScript errors goes into the log, but only a little of it: a minute holds at
/// most [`PER_MINUTE`] reports and a run at most [`TOTAL`], and each text is cut short. (The page asks nothing back.)
pub struct PageLog {
    state: Mutex<(Instant, u32, u32, bool)>,
}

const PER_MINUTE: u32 = 20;
const TOTAL: u32 = 200;

fn flat(s: &str, max: usize) -> String {
    let mut out: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).take(max + 1).collect();
    if out.chars().count() > max {
        out = out.chars().take(max).collect();
        out.push('…');
    }
    out.trim().to_string()
}

impl PageLog {
    pub fn new() -> PageLog {
        PageLog { state: Mutex::new((Instant::now(), 0, 0, false)) }
    }

    /// Log one report from the page ({ kind, message, source, stack }); false when it was left out for the limit.
    pub fn record(&self, args: &Value) -> bool {
        {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if s.0.elapsed() >= Duration::from_secs(60) {
                *s = (Instant::now(), 0, s.2, false);
            }
            if s.1 >= PER_MINUTE || s.2 >= TOTAL {
                if !s.3 {
                    s.3 = true;
                    log::write(log::Level::Warn, "page errors: too many reports, the rest of this minute is left out");
                }
                return false;
            }
            s.1 += 1;
            s.2 += 1;
        }
        let text = |k: &str, max: usize| args.get(k).and_then(Value::as_str).map(|v| flat(v, max)).unwrap_or_default();
        let kind = match args.get("kind").and_then(Value::as_str) {
            Some("unhandledrejection") => "promise rejected",
            _ => "JavaScript error",
        };
        let (message, source, stack) = (text("message", 500), text("source", 200), text("stack", 1500));
        let mut line = format!("page {kind}: {message}");
        if !source.is_empty() {
            line.push_str(&format!(" at {source}"));
        }
        if !stack.is_empty() {
            line.push_str(&format!(" | {stack}"));
        }
        log::write(log::Level::Error, &line);
        true
    }
}

/// Check the arguments of a bundle request: the page may only ask for the names switch.
pub fn bundle_request(args: &Value) -> Result<bool> {
    match args.get("includeNames") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(user_error("The support bundle request was not understood.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_errors_are_cut_and_limited() {
        let p = PageLog::new();
        let big = json!({ "kind": "error", "message": "x".repeat(2000), "source": "https://imagine.app/app.js:1:1", "stack": "a\nb\tc" });
        let mut taken = 0;
        for _ in 0..50 {
            if p.record(&big) {
                taken += 1;
            }
        }
        assert_eq!(taken, PER_MINUTE);
        assert_eq!(flat("a\r\nb\tc", 10), "a  b c");
        assert_eq!(flat(&"é".repeat(600), 500).chars().count(), 501);
    }

    #[test]
    fn the_bundle_request_takes_only_the_names_switch() {
        assert!(!bundle_request(&json!({})).unwrap());
        assert!(bundle_request(&json!({ "includeNames": true })).unwrap());
        assert!(bundle_request(&json!({ "includeNames": "yes" })).is_err());
    }
}
