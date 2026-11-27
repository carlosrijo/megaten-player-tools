//! The self-check ("Check my setup" in the app's Settings, `imagine-reader doctor`): one pass over everything that
//! can go wrong, each line pass / warn / fail with a plain explanation and what to do about it.
//!
//! * App: version, Windows, (app only) the WebView2 runtime.
//! * Files: the data folder is writable, the log folder, the settings and config files, the folders `config.json` names
//!   (missing optional ones are a warning), the saved characters and their spreadsheets.
//! * Game: how many clients are running and who is logged in on each; for each client its exe against the build the
//!   memory layouts were made for, whether it opens read-only, and whether each reader finds its structure
//!   (character, nameplate, own stats, gear, Information window, demons, progress) with how long it took.
//!
//! The result is a [`Report`] that renders to text (`to_text`) and to JSON for the page (`to_json`). Everything here
//! is read-only toward the game; the only thing written is a one-byte test file in the data folder (and only when
//! `probe_write` is set), deleted at once.
//!
//! The client's identity: ImagineClient.exe has no version resource, so the build is recognised by its size and SHA-256
//! (and the PE header's time stamp and image size, which say "same build, file changed" when only the hash differs).
//! [`KNOWN_CLIENTS`] lists the builds the layouts were made and checked against; to add the next one after the reader
//! has been updated, run `imagine-reader doctor` on the new client and copy its line.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

use chrono::{DateTime, Local};
use serde_json::{Value, json};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW};
use windows::core::{PCWSTR, w};

use crate::data::config;
use crate::diag::{about, crash, log, sha256};
use crate::game::client_reader::{self, ClientProcess};
use crate::game::demon_reader;
use crate::memory::{self, GameMemory};

// ---------- the known client build ----------

/// A client build the memory layouts were made for.
pub struct KnownClient {
    /// what the client calls itself on its title screen
    pub name: &'static str,
    pub size: u64,
    /// lower-case hex SHA-256 of ImagineClient.exe
    pub sha256: &'static str,
    /// PE header: TimeDateStamp and SizeOfImage
    pub pe_timestamp: u32,
    pub size_of_image: u32,
}

/// Recorded 2026-11-24 from the IMAGINE client both running clients used ("IMAGINE Version 1.670", file date 2023-09-10).
pub const KNOWN_CLIENTS: &[KnownClient] = &[KnownClient {
    name: "IMAGINE Version 1.670",
    size: 23_423_280,
    sha256: "e8248800b2f20ec1d147ae22e3991765a8ac1b28ec8929121bce3555b850b598",
    pe_timestamp: 1_461_056_115,
    size_of_image: 0x0851_6000,
}];

/// What identifies one ImagineClient.exe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientIdentity {
    pub path: PathBuf,
    pub size: u64,
    /// the file's date, "2023-09-10"
    pub modified: String,
    pub sha256: String,
    pub pe_timestamp: Option<u32>,
    pub size_of_image: Option<u32>,
    /// the exe's version resource, when it has one (this client has none)
    pub file_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildMatch {
    /// the build the layouts were made for (its name)
    Known(&'static str),
    /// the PE header and size match a known build but the file's digest does not: modified or repacked
    SameShape(&'static str),
    Unknown,
}

pub fn match_build(id: &ClientIdentity, known: &[KnownClient]) -> BuildMatch {
    if let Some(k) = known.iter().find(|k| k.size == id.size && k.sha256.eq_ignore_ascii_case(&id.sha256)) {
        return BuildMatch::Known(k.name);
    }
    if let Some(k) = known.iter().find(|k| k.size == id.size && Some(k.pe_timestamp) == id.pe_timestamp && Some(k.size_of_image) == id.size_of_image) {
        return BuildMatch::SameShape(k.name);
    }
    BuildMatch::Unknown
}

/// The PE header's TimeDateStamp and SizeOfImage (None when the file isn't a PE file).
pub fn pe_header(bytes: &[u8]) -> Option<(u32, u32)> {
    let u32_at = |o: usize| -> Option<u32> { bytes.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes"))) };
    if bytes.get(0..2)? != b"MZ" {
        return None;
    }
    let pe = u32_at(0x3c)? as usize;
    if bytes.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    Some((u32_at(pe + 8)?, u32_at(pe + 24 + 56)?))
}

fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

/// The exe's version resource as "a.b.c.d", None when it has none.
fn file_version(path: &Path) -> Option<String> {
    let name = wide(path);
    // SAFETY: the buffers are as big as the calls are told; the version block lives as long as `buf`.
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(name.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(name.as_ptr()), None, size, buf.as_mut_ptr() as *mut _).ok()?;
        let mut p: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        if !VerQueryValueW(buf.as_ptr() as *const _, w!("\\"), &mut p, &mut len).as_bool() || p.is_null() || (len as usize) < size_of::<VS_FIXEDFILEINFO>() {
            return None;
        }
        let fi = &*(p as *const VS_FIXEDFILEINFO);
        Some(format!("{}.{}.{}.{}", fi.dwFileVersionMS >> 16, fi.dwFileVersionMS & 0xffff, fi.dwFileVersionLS >> 16, fi.dwFileVersionLS & 0xffff))
    }
}

type IdKey = (PathBuf, u64, Option<SystemTime>);
static IDENTITIES: Mutex<Option<HashMap<IdKey, ClientIdentity>>> = Mutex::new(None);

/// Size, date, SHA-256, PE header and version resource of an exe. The digest of a 23 MB file takes about a tenth of a
/// second; it is remembered per file (path, size, date), so asking again is free. Setting IMAGINE_TEST_CLIENT_SHA256
/// replaces the digest and ignores the PE header, so the client looks like a build nobody has seen (to try the "game
/// was updated" paths without another client).
pub fn identify(path: &Path) -> std::io::Result<ClientIdentity> {
    let meta = std::fs::metadata(path)?;
    let key: IdKey = (path.to_path_buf(), meta.len(), meta.modified().ok());
    if let Some(known) = IDENTITIES.lock().ok().and_then(|g| g.as_ref().and_then(|m| m.get(&key).cloned())) {
        return Ok(known);
    }
    let fake = std::env::var("IMAGINE_TEST_CLIENT_SHA256").ok().map(|f| f.trim().to_ascii_lowercase()).filter(|f| !f.is_empty());
    let digest = match &fake {
        Some(f) => f.clone(),
        None => sha256::file_sha256(path)?,
    };
    let mut head = vec![0u8; 4096];
    let n = std::fs::File::open(path).and_then(|mut f| std::io::Read::read(&mut f, &mut head)).unwrap_or(0);
    let pe = if fake.is_some() { None } else { pe_header(&head[..n]) };
    let id = ClientIdentity {
        path: path.to_path_buf(),
        size: meta.len(),
        modified: meta.modified().ok().map(|t| DateTime::<Local>::from(t).format("%Y-%m-%d").to_string()).unwrap_or_else(|| "unknown".to_string()),
        sha256: digest,
        pe_timestamp: pe.map(|p| p.0),
        size_of_image: pe.map(|p| p.1),
        file_version: file_version(path),
    };
    if let Ok(mut g) = IDENTITIES.lock() {
        g.get_or_insert_with(HashMap::new).insert(key, id.clone());
    }
    Ok(id)
}

/// One running client and what is known of its exe, for the app's start-up warning.
#[derive(Clone, Debug)]
pub struct ClientBuild {
    pub pid: u32,
    pub folder: String,
    pub identity: Option<ClientIdentity>,
    pub result: Option<BuildMatch>,
}

/// Every running client with its build checked against [`KNOWN_CLIENTS`]. Reads the exes, never the game's memory.
pub fn client_builds() -> Vec<ClientBuild> {
    client_reader::client_processes()
        .into_iter()
        .map(|p| {
            let identity = p.exe_path.as_deref().and_then(|e| identify(e).ok());
            let result = identity.as_ref().map(|i| match_build(i, KNOWN_CLIENTS));
            ClientBuild { pid: p.pid, folder: p.folder.clone(), identity, result }
        })
        .collect()
}

/// What the person is told when a client is not the known build.
pub const UPDATED_WARNING: &str = "The game was updated; readings may be wrong until the reader is updated.";

// ---------- the report ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub section: String,
    pub name: String,
    pub status: Status,
    /// what was found, in plain words
    pub detail: String,
    /// what to do about it (empty for a pass)
    pub advice: String,
    pub ms: u64,
}

#[derive(Clone, Debug)]
pub struct Report {
    /// "BABEL 1.2.0 (commit 356636c, built 2026-11-24 03:41 UTC)"
    pub program: String,
    pub windows: String,
    pub checked_at: String,
    pub checks: Vec<Check>,
    /// the characters seen (saved, or logged in on a client), for hiding their names in a bundle
    pub names: Vec<String>,
}

impl Report {
    pub fn counts(&self) -> (usize, usize, usize) {
        let n = |s: Status| self.checks.iter().filter(|c| c.status == s).count();
        (n(Status::Pass), n(Status::Warn), n(Status::Fail))
    }

    /// The worst line's status.
    pub fn worst(&self) -> Status {
        let (_, w, f) = self.counts();
        if f > 0 { Status::Fail } else if w > 0 { Status::Warn } else { Status::Pass }
    }

    fn summary(&self) -> String {
        let (p, w, f) = self.counts();
        format!("{p} passed, {w} {}, {f} failed", if w == 1 { "warning" } else { "warnings" })
    }

    /// Plain text for the console, the bundle and "Copy".
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        s.push_str("IMAGINE self-check\r\n");
        s.push_str(&format!("{}\r\n{}\r\nChecked {}\r\n", self.program, self.windows, self.checked_at));
        let width = self.checks.iter().map(|c| c.name.chars().count()).max().unwrap_or(10);
        let mut section = "";
        for c in &self.checks {
            if c.section != section {
                section = &c.section;
                s.push_str(&format!("\r\n{section}\r\n"));
            }
            let pad = " ".repeat(width - c.name.chars().count());
            s.push_str(&format!("  [{}] {}{pad}  {}\r\n", c.status.word(), c.name, c.detail));
            if !c.advice.is_empty() {
                s.push_str(&format!("         {}-> {}\r\n", " ".repeat(width + 2), c.advice));
            }
        }
        s.push_str(&format!("\r\nResult: {}\r\n", self.summary()));
        s
    }

    /// The page's JSON.
    pub fn to_json(&self) -> Value {
        let (p, w, f) = self.counts();
        json!({
            "program": self.program, "windows": self.windows, "checkedAt": self.checked_at,
            "summary": { "pass": p, "warn": w, "fail": f, "worst": self.worst().word(), "text": self.summary() },
            "checks": self.checks.iter().map(|c| json!({
                "section": c.section, "name": c.name, "status": c.status.word(), "detail": c.detail, "advice": c.advice, "ms": c.ms,
            })).collect::<Vec<_>>(),
            "text": self.to_text(),
        })
    }
}

// ---------- the checks ----------

/// What the check is given.
pub struct Input {
    pub data_dir: PathBuf,
    /// the app's WebView2 runtime: Some(Ok(version)) / Some(Err(why)); None for the command line (no such line)
    pub webview2: Option<Result<String, String>>,
    /// %AppData%\BABEL\settings.json for the app (None for the command line)
    pub settings_path: Option<PathBuf>,
    /// make the one-byte write test in the data folder (false during a capture run on the person's own data)
    pub probe_write: bool,
    /// look at the running game (false: leave out the Game section, for tests of the file checks)
    pub game: bool,
}

fn took(ms: u64) -> String {
    if ms >= 1000 { format!("{:.1} s", ms as f64 / 1000.0) } else { format!("{ms} ms") }
}

fn ck(section: &str, name: &str, status: Status, detail: impl Into<String>, advice: &str) -> Check {
    Check { section: section.to_string(), name: name.to_string(), status, detail: detail.into(), advice: advice.to_string(), ms: 0 }
}

fn kb(bytes: u64) -> String {
    if bytes >= 1_000_000 { format!("{:.1} MB", bytes as f64 / 1_000_000.0) } else { format!("{} KB", bytes.div_ceil(1000)) }
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Run every check once.
pub fn run(input: &Input) -> Report {
    let start = Instant::now();
    let b = about::build_info();
    let win = about::windows_info();
    let mut checks: Vec<Check> = Vec::new();
    let mut names: Vec<String> = Vec::new();

    // ---- App ----
    const APP: &str = "App";
    checks.push(ck(
        APP,
        "Program version",
        Status::Pass,
        format!("{}{}", b.short(), if b.commit.ends_with("-dirty") { ", built from source with changes" } else { "" }),
        "",
    ));
    checks.push(if win.build >= 17763 {
        ck(APP, "Windows", Status::Pass, win.text(), "")
    } else {
        ck(APP, "Windows", Status::Warn, win.text(), "This program is made for Windows 10 (version 1809) or Windows 11; on anything older it may not start.")
    });
    match &input.webview2 {
        Some(Ok(v)) => checks.push(ck(APP, "WebView2", Status::Pass, format!("Microsoft Edge WebView2 Runtime {v}"), "")),
        Some(Err(why)) => checks.push(ck(
            APP,
            "WebView2",
            Status::Fail,
            format!("Not found ({why})"),
            "Install the Microsoft Edge WebView2 Runtime (a free download from Microsoft; it comes with Windows 11), then start the app again.",
        )),
        None => {}
    }

    // ---- Files ----
    const FILES: &str = "Files";
    checks.push(check_data_dir(&input.data_dir, input.probe_write));
    checks.push(check_logs(&input.data_dir));
    if let Some(sp) = &input.settings_path {
        checks.push(check_json_file(FILES, "Settings file", sp, "Preferences are not saved yet; the defaults are used.", "Delete the file and the app starts with the defaults."));
    }
    let cfg_path = config::file_path();
    checks.push(check_json_file(
        FILES,
        "Config file",
        &cfg_path,
        "Not made yet; it is created with detected paths when the program first needs it.",
        "Fix the file by hand or delete it: a new one is made with detected paths.",
    ));
    let cfg = config::current();
    checks.push(check_path(
        "Spreadsheet folder",
        cfg.stats_folder.as_deref(),
        true,
        "Where each character's snapshot spreadsheet (CSV) goes.",
        "It is created when the first snapshot is saved. To keep the spreadsheets elsewhere, set statsFolder in the config file.",
    ));
    checks.push(check_bundled_path(
        "Server constants",
        cfg.server_constants.as_deref(),
        false,
        "The local server's constants.xml (digitalize rates, Mitama set boost).",
        "Not needed: the reader has a copy of IMAGINE's constants built in. Set serverConstants in the config file only to read a newer one.",
    ));
    checks.push(check_bundled_path(
        "Website content",
        cfg.website_content.as_deref(),
        true,
        "The website's content folder (item, skill and effect data).",
        "Not needed: the reader has a copy of the files it uses built in. Set websiteContent in the config file only to read a newer one.",
    ));
    checks.push(check_bundled_path(
        "Effect files",
        cfg.tokusei_xml.as_deref(),
        true,
        "comp_hack's tokusei XML folder (effect numbers only the server has).",
        "Not needed: the reader has a copy of the effect files built in. Set tokuseiXml in the config file only to read a newer one.",
    ));
    let (chars, char_names) = check_characters(&input.data_dir);
    checks.push(chars);
    names.extend(char_names);

    // ---- Game ----
    if input.game {
        let (game, seen) = check_game();
        checks.extend(game);
        for n in seen {
            if !names.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
                names.push(n);
            }
        }
    }

    log::write(log::Level::Info, &format!("self-check finished in {}", took(start.elapsed().as_millis() as u64)));
    Report { program: b.line(), windows: win.text(), checked_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), checks, names }
}

fn check_data_dir(dir: &Path, probe: bool) -> Check {
    const N: &str = "Data folder";
    let shown = dir.display().to_string();
    let t = Instant::now();
    let write_test = |d: &Path| -> std::io::Result<()> {
        let f = d.join(format!(".imagine-write-test-{}", std::process::id()));
        std::fs::write(&f, b"x")?;
        std::fs::remove_file(&f)
    };
    let mut c = if !dir.exists() {
        // the nearest folder that does exist is where it would be made
        let parent = dir.ancestors().skip(1).find(|p| p.exists());
        match (probe, parent) {
            (true, Some(p)) if write_test(p).is_err() => ck(
                "Files",
                N,
                Status::Fail,
                format!("{shown} does not exist and can't be made: Windows won't let this program save in {}.", p.display()),
                "Put the program in a folder you can write to (not Program Files), or fix that folder's permissions.",
            ),
            _ => ck("Files", N, Status::Warn, format!("{shown} does not exist yet."), "It is made when something is first saved. Nothing to do."),
        }
    } else if !dir.is_dir() {
        ck("Files", N, Status::Fail, format!("{shown} is a file, not a folder."), "Move or rename that file so the folder can be made.")
    } else if !probe {
        ck("Files", N, Status::Pass, format!("{shown} (not tested for writing in this run)"), "")
    } else {
        match write_test(dir) {
            Ok(()) => ck("Files", N, Status::Pass, format!("{shown} (writable)"), ""),
            Err(e) => ck(
                "Files",
                N,
                Status::Fail,
                format!("{shown} can't be written to ({e})."),
                "Snapshots and settings can't be saved there. Put the program in a folder you can write to (not Program Files), or fix the folder's permissions.",
            ),
        }
    };
    c.ms = t.elapsed().as_millis() as u64;
    c
}

fn check_logs(data_dir: &Path) -> Check {
    let dir = log::dir().unwrap_or_else(|| data_dir.join("logs"));
    let files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("log")) && p.is_file()).collect())
        .unwrap_or_default();
    let bytes: u64 = files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum();
    let crashes = crash::reports(&dir);
    let crash_text = match crashes.last() {
        None => "no crash reports".to_string(),
        Some(last) => format!(
            "{} crash report{} (latest {})",
            crashes.len(),
            if crashes.len() == 1 { "" } else { "s" },
            last.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        ),
    };
    if log::dir().is_none() {
        return ck("Files", "Log files", Status::Warn, format!("Logging is off in this run ({} has {} log file(s), {}; {crash_text}).", dir.display(), files.len(), kb(bytes)), "");
    }
    ck("Files", "Log files", Status::Pass, format!("{}: {} file(s), {}; {crash_text}", dir.display(), files.len(), kb(bytes)), "")
}

fn check_json_file(section: &str, name: &str, path: &Path, missing: &str, broken_advice: &str) -> Check {
    let shown = path.display().to_string();
    if !path.exists() {
        return ck(section, name, Status::Warn, format!("{shown}: {missing}"), "");
    }
    match std::fs::read_to_string(path) {
        Err(e) => ck(section, name, Status::Warn, format!("{shown} can't be read ({e})."), broken_advice),
        Ok(text) => match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
            Ok(_) => ck(section, name, Status::Pass, shown, ""),
            Err(e) => ck(section, name, Status::Warn, format!("{shown} is damaged ({e}); the defaults are used."), broken_advice),
        },
    }
}

fn check_path(name: &str, value: Option<&str>, dir: bool, what: &str, advice_missing: &str) -> Check {
    match value {
        None => ck("Files", name, Status::Warn, format!("Not set. {what}"), advice_missing),
        Some(p) => {
            let path = Path::new(p);
            let ok = if dir { path.is_dir() } else { path.is_file() };
            if ok {
                ck("Files", name, Status::Pass, p.to_string(), "")
            } else {
                ck("Files", name, Status::Warn, format!("{p} does not exist. {what}"), advice_missing)
            }
        }
    }
}

/// A server or website folder the reader has a built-in copy of (reference\bundled\): set and there, it is read;
/// otherwise the built-in copy is used, which is fine.
fn check_bundled_path(name: &str, value: Option<&str>, dir: bool, what: &str, advice: &str) -> Check {
    let there = value.is_some_and(|p| if dir { Path::new(p).is_dir() } else { Path::new(p).is_file() });
    match value {
        Some(p) if there => ck("Files", name, Status::Pass, p.to_string(), ""),
        _ => ck("Files", name, Status::Pass, format!("Using the built-in copy. {what}"), advice),
    }
}

/// The saved characters (characters.json) and whether their spreadsheets are there.
fn check_characters(data_dir: &Path) -> (Check, Vec<String>) {
    const N: &str = "Saved characters";
    let file = data_dir.join("characters.json");
    if !file.exists() {
        return (ck("Files", N, Status::Warn, "None saved yet.", "Start the game, log in and take a snapshot: the character is saved then."), Vec::new());
    }
    let list: Vec<Value> = match std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok()) {
        Some(Value::Array(a)) => a,
        _ => {
            return (
                ck("Files", N, Status::Warn, format!("{} can't be read.", file.display()), "Delete it: the program lists the characters again from the data it finds."),
                Vec::new(),
            );
        }
    };
    let mut names = Vec::new();
    let mut parts = Vec::new();
    let mut missing = 0;
    for e in &list {
        let name = e.get("Name").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let csv = e.get("Csv").and_then(Value::as_str).unwrap_or("");
        let has_csv = !csv.is_empty() && Path::new(csv).is_file();
        let dir = match e.get("Dir").and_then(Value::as_str) {
            None | Some("") | Some(".") => data_dir.to_path_buf(),
            Some(d) => data_dir.join(d),
        };
        let has_data = dir.join("dash").join("snapshots.json").is_file();
        if has_data && !has_csv {
            missing += 1;
        }
        parts.push(format!("{name} ({}{})", if has_data { "data saved" } else { "no data yet" }, if has_csv { ", spreadsheet found" } else { ", no spreadsheet yet" }));
        names.push(name);
    }
    if parts.is_empty() {
        return (ck("Files", N, Status::Warn, "None saved yet.", "Start the game, log in and take a snapshot: the character is saved then."), names);
    }
    let detail = format!("{}: {}", parts.len(), parts.join("; "));
    if missing > 0 {
        (
            ck("Files", N, Status::Warn, detail, "A spreadsheet is missing for a character that has saved data. Settings > Spreadsheet > Change... points the character at it; \"rebuild\" and \"recalc\" need it."),
            names,
        )
    } else {
        (ck("Files", N, Status::Pass, detail, ""), names)
    }
}

// ---------- the game ----------

/// Run a reader; a panic inside it is a failed line, not a crashed check.
fn guard<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| {
        p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "it stopped unexpectedly".to_string())
    })
}

/// One client's lines, who is logged in on it (name, level) and the process.
type ClientChecks = (Vec<Check>, Option<(String, i32)>, ClientProcess);

fn check_game() -> (Vec<Check>, Vec<String>) {
    const G: &str = "Game";
    let procs = client_reader::client_processes();
    let mut per_client: Vec<ClientChecks> = Vec::new();
    for p in procs {
        let (checks, who) = check_client(&p);
        per_client.push((checks, who, p));
    }
    let mut out = Vec::new();
    let mut names = Vec::new();
    // the summary line comes first
    if per_client.is_empty() {
        out.push(ck(
            G,
            "Game running",
            Status::Warn,
            "No ImagineClient is running.",
            "Start IMAGINE and log in to take a snapshot. Everything else (the saved data, the tools) works without the game.",
        ));
    } else {
        let list: Vec<String> = per_client
            .iter()
            .map(|(_, who, p)| match who {
                Some((n, l)) => format!("{} -> {n} (level {l})", p.folder),
                None => format!("{} -> nobody logged in", p.folder),
            })
            .collect();
        let nobody = per_client.iter().any(|(_, who, _)| who.is_none());
        out.push(ck(
            G,
            "Game running",
            if nobody { Status::Warn } else { Status::Pass },
            format!("{} client{}: {}", per_client.len(), if per_client.len() == 1 { "" } else { "s" }, list.join("; ")),
            if nobody { "A client with nobody logged in (the title screen?) can't be read. Log in to a character." } else { "" },
        ));
    }
    for (i, (checks, who, p)) in per_client.into_iter().enumerate() {
        let section = format!("Client {}: {}{}", i + 1, p.folder, who.as_ref().map(|(n, _)| format!(", {n}")).unwrap_or_default());
        if let Some((n, _)) = &who {
            names.push(n.clone());
        }
        out.extend(checks.into_iter().map(|mut c| {
            c.section = section.clone();
            c
        }));
    }
    (out, names)
}

fn check_client(p: &ClientProcess) -> (Vec<Check>, Option<(String, i32)>) {
    let mut v: Vec<Check> = Vec::new();
    let sec = "";

    // the exe against the build the layouts were made for
    let t = Instant::now();
    match p.exe_path.as_deref() {
        None => v.push(ck(sec, "Client build", Status::Warn, "The client's exe path can't be read, so its build can't be checked.", "")),
        Some(exe) => match identify(exe) {
            Err(e) => v.push(ck(sec, "Client build", Status::Warn, format!("{} can't be read ({e}).", exe.display()), "")),
            Ok(id) => {
                let what = format!(
                    "{}: {} bytes, dated {}, SHA-256 {}..., {}",
                    exe.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    thousands(id.size),
                    id.modified,
                    &id.sha256[..id.sha256.len().min(16)],
                    id.file_version.as_ref().map(|f| format!("file version {f}")).unwrap_or_else(|| "no version number inside the exe".to_string()),
                );
                match match_build(&id, KNOWN_CLIENTS) {
                    BuildMatch::Known(name) => v.push(ck(sec, "Client build", Status::Pass, format!("{what}. This is the build the reader was made for ({name})."), "")),
                    BuildMatch::SameShape(name) => v.push(ck(
                        sec,
                        "Client build",
                        Status::Warn,
                        format!("{what}. It looks like {name} but the file is different."),
                        "The exe was changed or repacked. Readings are probably fine; if they are wrong, the reader may need an update.",
                    )),
                    BuildMatch::Unknown => v.push(ck(
                        sec,
                        "Client build",
                        Status::Warn,
                        format!("{what}. This is not the build the reader was made for ({}).", KNOWN_CLIENTS.first().map(|k| k.name).unwrap_or("none")),
                        UPDATED_WARNING,
                    )),
                }
            }
        },
    }
    if let Some(c) = v.last_mut() {
        c.ms = t.elapsed().as_millis() as u64;
    }

    // read-only access
    let mem = match GameMemory::new(p.pid) {
        Ok(m) => {
            v.push(ck(sec, "Read access", Status::Pass, "Opened read-only (PROCESS_QUERY_INFORMATION | PROCESS_VM_READ); nothing else is ever asked for.", ""));
            m
        }
        Err(e) => {
            let msg = e.to_string();
            let denied = msg.to_ascii_lowercase().contains("denied") || msg.contains("0x80070005");
            v.push(ck(
                sec,
                "Read access",
                Status::Fail,
                format!("Windows refused to open the client for reading ({msg})."),
                if denied { "The game probably runs as administrator: start this program as administrator too." } else { "Close the client and start it again, then check again." },
            ));
            return (v, None);
        }
    };

    // one sweep of the game's memory shared by every reader below
    let _sweep = memory::share_sweeps();

    let t = Instant::now();
    let who = guard(|| client_reader::detect_character(&mem));
    let ms = t.elapsed().as_millis() as u64;
    let (name, level) = match who {
        Ok(Some((n, l))) => {
            let mut c = ck(sec, "Character", Status::Pass, format!("{n}, level {l} ({})", took(ms)), "");
            c.ms = ms;
            v.push(c);
            (n, l)
        }
        Ok(None) => {
            let mut c = ck(
                sec,
                "Character",
                Status::Warn,
                format!("Nobody is logged in on this client ({}).", took(ms)),
                "Log in to a character (at the title screen the game has nothing to read). If someone is logged in, the client may have been updated.",
            );
            c.ms = ms;
            v.push(c);
            return (v, None);
        }
        Err(e) => {
            v.push(ck(sec, "Character", Status::Fail, format!("The reader stopped unexpectedly ({e})."), "Save a support bundle and send it."));
            return (v, None);
        }
    };

    macro_rules! timed {
        ($name:expr, $body:expr, $found:expr, $missing:expr) => {{
            let t = Instant::now();
            let r = guard(|| $body);
            let ms = t.elapsed().as_millis() as u64;
            let mut c = match r {
                Ok(val) => match ($found)(&val, ms) {
                    Some(c) => c,
                    None => ($missing)(ms),
                },
                Err(e) => ck(sec, $name, Status::Fail, format!("The reader stopped unexpectedly ({e})."), "Save a support bundle and send it."),
            };
            c.ms = ms;
            v.push(c);
        }};
    }

    timed!(
        "Nameplate",
        client_reader::read_nameplate(&mem, &name),
        |r: &Option<crate::game::Nameplate>, ms: u64| r.map(|n| ck(sec, "Nameplate", Status::Pass, format!("Found: level {} ({})", n.level, took(ms)), "")),
        |ms: u64| ck(sec, "Nameplate", Status::Fail, format!("Not found ({}).", took(ms)), "The level is read from it. The client may have been updated; check the Client build line above.")
    );
    let _ = level;

    timed!(
        "Own stats",
        client_reader::read_own_status(&mem, false).map(|s| (s, false)).or_else(|| client_reader::read_own_status(&mem, true).map(|s| (s, true))),
        |r: &Option<(client_reader::OwnStatus, bool)>, ms: u64| r.as_ref().map(|(_, digi)| ck(
            sec,
            "Own stats",
            Status::Pass,
            format!("Found: level, HP, MP and the 12 stats{} ({})", if *digi { " (you look digitalized, so these include the demon)" } else { "" }, took(ms)),
            ""
        )),
        |ms: u64| ck(sec, "Own stats", Status::Fail, format!("Not found ({}).", took(ms)), "The character's stats can't be read, so snapshots fall back to saved stats. The client may have been updated.")
    );

    timed!(
        "Gear",
        (client_reader::live_gear(&mem), client_reader::read_gear_grids(&mem).len()),
        |r: &(Option<crate::game::GearGrid>, usize), ms: u64| r.0.as_ref().map(|g| {
            let worn = g.ids.iter().filter(|&&i| i > 0).count();
            let records = g.details.as_ref().map(|d| d.iter().filter(|x| x.is_some()).count()).unwrap_or(0);
            let grids = format!("{} gear-grid candidate{}", r.1, if r.1 == 1 { "" } else { "s" });
            let why = g.details_missing.unwrap_or("no reason");
            if g.details.is_some() {
                ck(sec, "Gear", Status::Pass, format!("Found: {worn} of 15 slots worn, each item's own record read ({records}); {grids} ({})", took(ms)), "")
            } else if r.1 > 0 {
                ck(
                    sec,
                    "Gear",
                    Status::Pass,
                    format!("Found: {worn} of 15 slots worn; the items' own records were not read ({why}), so tarot and soul come from the gear grid ({grids}) ({})", took(ms)),
                    "",
                )
            } else {
                ck(
                    sec,
                    "Gear",
                    Status::Warn,
                    format!("Found: {worn} of 15 slots worn, but neither the items' own records ({why}) nor a gear grid ({}).", took(ms)),
                    "Tarot, soul and mods of the worn gear can't be read, so snapshots take them from the item tooltips and the Build Planner starts without them. Please send this through Send feedback with the diagnostics files attached.",
                )
            }
        }),
        |ms: u64| ck(sec, "Gear", Status::Fail, format!("Not found ({}).", took(ms)), "The worn gear can't be found in the client's memory. Open the Information window in game and try again; if it still fails, the client may have been updated.")
    );

    timed!(
        "Information window",
        client_reader::read_info_window(&mem, &name, |_| true),
        |r: &Option<crate::game::InfoWindow>, ms: u64| r.as_ref().map(|w| ck(sec, "Information window", Status::Pass, format!("Open: level {} ({})", w.level, took(ms)), "")),
        |ms: u64| ck(sec, "Information window", Status::Pass, format!("Not open, which is fine: it is only used for the most exact stats ({}).", took(ms)), "")
    );

    timed!(
        "Demons",
        {
            let all = demon_reader::read_demons(&mem, true);
            let comp = demon_reader::read_comp(&mem, &all);
            let loaded = all.iter().filter(|d| d.loaded()).count();
            (all.len(), loaded, comp.map(|c| c.len()))
        },
        |r: &(usize, usize, Option<usize>), ms: u64| if r.0 > 0 {
            Some(ck(
                sec,
                "Demons",
                Status::Pass,
                format!(
                    "Found: {} demon record{}, {} with details; {} ({})",
                    r.0,
                    if r.0 == 1 { "" } else { "s" },
                    r.1,
                    match r.2 {
                        Some(n) => format!("{n} in the COMP"),
                        None => "the COMP list was not found (open the COMP in game)".to_string(),
                    },
                    took(ms)
                ),
                "",
            ))
        } else {
            None
        },
        |ms: u64| ck(sec, "Demons", Status::Warn, format!("No demon records found ({}).", took(ms)), "Normal for a character with no demons. Otherwise open the COMP in game and check again; if it still fails, the client may have been updated.")
    );

    timed!(
        "Progress",
        demon_reader::read_progress(&mem),
        |r: &Option<demon_reader::Progress>, ms: u64| r.as_ref().map(|p| ck(
            sec,
            "Progress",
            Status::Pass,
            format!(
                "Found: {} entries in the learned-skill list, Digitalization Levels in {} races{} ({})",
                p.skills.as_ref().map(|s| s.len()).unwrap_or(0),
                p.levels.as_ref().map(|l| l.values().filter(|&&v| v > 0).count()).unwrap_or(0),
                if p.digitalized() { ", digitalized right now" } else { "" },
                took(ms)
            ),
            ""
        )),
        |ms: u64| ck(sec, "Progress", Status::Fail, format!("Not found ({}).", took(ms)), "Expertise, digitalize levels and learned skills can't be read. The client may have been updated.")
    );

    (v, Some((name, level)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(size: u64, sha: &str, pe: Option<(u32, u32)>) -> ClientIdentity {
        ClientIdentity {
            path: PathBuf::from("C:\\g\\ImagineClient.exe"),
            size,
            modified: "2023-09-10".into(),
            sha256: sha.into(),
            pe_timestamp: pe.map(|p| p.0),
            size_of_image: pe.map(|p| p.1),
            file_version: None,
        }
    }

    fn known() -> KnownClient {
        KnownClient { name: "IMAGINE Version 9.9", size: 1000, sha256: "abcd", pe_timestamp: 5, size_of_image: 77 }
    }

    #[test]
    fn the_known_build_is_recognised_and_other_files_are_not() {
        let k = [known()];
        assert_eq!(match_build(&id(1000, "ABCD", Some((5, 77))), &k), BuildMatch::Known("IMAGINE Version 9.9"));
        // same header, different bytes: modified
        assert_eq!(match_build(&id(1000, "ffff", Some((5, 77))), &k), BuildMatch::SameShape("IMAGINE Version 9.9"));
        // another size, or no PE header: unknown
        assert_eq!(match_build(&id(1001, "ffff", Some((5, 77))), &k), BuildMatch::Unknown);
        assert_eq!(match_build(&id(1000, "ffff", None), &k), BuildMatch::Unknown);
        assert_eq!(match_build(&id(1000, "ffff", Some((6, 77))), &k), BuildMatch::Unknown);
    }

    #[test]
    fn the_recorded_client_is_this_one() {
        let k = &KNOWN_CLIENTS[0];
        assert_eq!(k.sha256.len(), 64);
        assert_eq!(k.sha256, k.sha256.to_ascii_lowercase());
        assert_eq!(k.size_of_image, 0x8516000);
        // a PE header like ImagineClient's: image base 0x400000, time stamp 2016-04-19
        let mut b = vec![0u8; 512];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3c] = 0x80;
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x88..0x8c].copy_from_slice(&k.pe_timestamp.to_le_bytes());
        b[0x80 + 24 + 56..0x80 + 24 + 60].copy_from_slice(&k.size_of_image.to_le_bytes());
        assert_eq!(pe_header(&b), Some((k.pe_timestamp, k.size_of_image)));
        assert_eq!(pe_header(b"not an exe"), None);
    }

    fn report() -> Report {
        let c = |section: &str, name: &str, status, detail: &str, advice: &str| Check {
            section: section.into(),
            name: name.into(),
            status,
            detail: detail.into(),
            advice: advice.into(),
            ms: 12,
        };
        Report {
            program: "BABEL 1.0.0 (commit abc1234, built 2026-11-24 03:41 UTC)".into(),
            windows: "Windows 11 Pro 24H2 (10.0.26100.1), 64-bit".into(),
            checked_at: "2026-11-24 04:00:00".into(),
            checks: vec![
                c("App", "Program version", Status::Pass, "1.0.0", ""),
                c("Files", "Data folder", Status::Fail, "C:\\x can't be written to.", "Move the exe."),
                c("Files", "Config file", Status::Warn, "missing", "It is made at first run."),
            ],
            names: vec!["Rubito".into()],
        }
    }

    #[test]
    fn the_report_renders_as_text() {
        let t = report().to_text();
        assert!(t.starts_with("IMAGINE self-check\r\nBABEL 1.0.0"), "{t}");
        assert!(t.contains("\r\nApp\r\n  [pass] Program version  1.0.0\r\n"), "{t}");
        assert!(t.contains("\r\nFiles\r\n  [fail] Data folder      C:\\x can't be written to.\r\n"), "{t}");
        assert!(t.contains("-> Move the exe.\r\n"), "{t}");
        assert!(t.contains("[warn] Config file      missing"), "{t}");
        assert!(t.ends_with("Result: 1 passed, 1 warning, 1 failed\r\n"), "{t}");
    }

    #[test]
    fn the_report_renders_as_json_for_the_page() {
        let r = report();
        assert_eq!(r.worst(), Status::Fail);
        let j = r.to_json();
        assert_eq!(j["summary"]["pass"], 1);
        assert_eq!(j["summary"]["warn"], 1);
        assert_eq!(j["summary"]["fail"], 1);
        assert_eq!(j["summary"]["worst"], "fail");
        assert_eq!(j["checks"][1]["status"], "fail");
        assert_eq!(j["checks"][1]["advice"], "Move the exe.");
        assert_eq!(j["checks"].as_array().unwrap().len(), 3);
        assert_eq!(j["text"], r.to_text());
    }

    #[test]
    fn a_data_folder_that_can_be_written_passes_and_one_that_is_missing_warns() {
        let dir = std::env::temp_dir().join(format!("imagine-doctor-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let missing = check_data_dir(&dir, true);
        assert_eq!(missing.status, Status::Warn, "{missing:?}");
        std::fs::create_dir_all(&dir).unwrap();
        let ok = check_data_dir(&dir, true);
        assert_eq!(ok.status, Status::Pass, "{ok:?}");
        assert!(ok.detail.contains("writable"));
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "the test file is deleted again");
        let untested = check_data_dir(&dir, false);
        assert!(untested.detail.contains("not tested"));
        let file = dir.join("afile");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(check_data_dir(&file, true).status, Status::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_characters_are_listed_with_what_is_missing() {
        let dir = std::env::temp_dir().join(format!("imagine-doctor-chars-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("dash")).unwrap();
        assert_eq!(check_characters(&dir).0.status, Status::Warn);
        std::fs::write(dir.join("dash").join("snapshots.json"), "[]").unwrap();
        let csv = dir.join("a.csv");
        std::fs::write(&csv, "x").unwrap();
        let list = json!([{ "Name": "Ann", "Dir": ".", "Csv": csv.to_string_lossy() }, { "Name": "Bob", "Dir": "characters\\Bob", "Csv": "C:\\nope\\bob.csv" }]);
        std::fs::write(dir.join("characters.json"), list.to_string()).unwrap();
        let (c, names) = check_characters(&dir);
        assert_eq!(c.status, Status::Pass, "Bob has no data yet, so a missing sheet is no problem: {c:?}");
        assert_eq!(names, vec!["Ann", "Bob"]);
        assert!(c.detail.contains("Ann (data saved, spreadsheet found)"), "{}", c.detail);
        // now Ann loses her spreadsheet
        std::fs::remove_file(&csv).unwrap();
        assert_eq!(check_characters(&dir).0.status, Status::Warn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn numbers_read_well() {
        assert_eq!(thousands(23_423_280), "23,423,280");
        assert_eq!(thousands(12), "12");
        assert_eq!(took(450), "450 ms");
        assert_eq!(took(1500), "1.5 s");
    }
}
