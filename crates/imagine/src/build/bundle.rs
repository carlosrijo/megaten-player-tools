//! The support bundle ("Save a support bundle..." in Settings): one zip a person can send when something is wrong.
//!
//! In it (every file as text, with personal details taken out by `diag::redact`):
//!   about.txt            version, commit, build date, Windows, WebView2, and what the bundle holds and never holds
//!   doctor.txt           a fresh self-check (`build::doctor`)
//!   logs/*.log           the local log files, and logs/crash-*.txt the crash reports
//!   data/config.json     the paths the program was told about
//!   data/characters.json the saved characters (name, folder, spreadsheet path)
//!   settings.json        the app's preferences (%AppData%\BABEL\settings.json)
//!   data-listing.txt     the data folder as a list: relative path, size, date (no contents)
//!
//! Never in it: spreadsheets (CSV), anything read from the game's memory, pictures, tooltips or item data.
//! The Windows user name is out of every path (`C:\Users\<name>` becomes `%USERPROFILE%`), and so are the
//! characters' names (`Character 1`, `Character 2`, ...) unless the person ticked "Include character names".

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local, Timelike};

use super::doctor::Report;
use crate::Result;
use crate::diag::redact::Redactor;

/// The most of one log or other text file that goes into the bundle (the newest part of it).
const FILE_CAP: usize = 3 * 1024 * 1024;
/// The data folder listing stops here.
const LISTING_ENTRIES: usize = 20_000;

pub struct Input<'a> {
    pub data_dir: &'a Path,
    /// the config.json in use (it may sit beside the program rather than in the data folder)
    pub config_path: PathBuf,
    /// %AppData%\BABEL\settings.json
    pub settings_path: Option<PathBuf>,
    pub include_names: bool,
    /// the self-check made for this bundle
    pub report: &'a Report,
    /// the version block (`diag::about::report_block`) and whatever else the program wants at the top of about.txt
    pub about: String,
}

#[derive(Debug, Clone)]
pub struct Summary {
    /// each file in the zip with its size before compression
    pub files: Vec<(String, u64)>,
    /// the zip's own size
    pub zip_bytes: u64,
}

/// `imagine-support-20261124-0341.zip`
pub fn default_name() -> String {
    format!("imagine-support-{}.zip", Local::now().format("%Y%m%d-%H%M"))
}

/// Write the bundle to `out` (made whole, or not at all: it is built beside it and renamed).
pub fn write(out: &Path, input: &Input) -> Result<Summary> {
    write_with(out, input, Redactor::for_this_pc())
}

/// [`write`] with the redactor given (tests use a made-up user).
pub fn write_with(out: &Path, input: &Input, redactor: Redactor) -> Result<Summary> {
    let entries = entries_with(input, redactor);
    let partial = out.with_extension("zip.partial");
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let result = (|| -> Result<()> {
        let file = std::fs::File::create(&partial)?;
        let mut zip = zip::ZipWriter::new(file);
        let now = Local::now();
        let stamp = zip::DateTime::from_date_and_time(now.year() as u16, now.month() as u8, now.day() as u8, now.hour() as u8, now.minute() as u8, now.second() as u8)
            .unwrap_or_default();
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).last_modified_time(stamp);
        for (name, text) in &entries {
            zip.start_file(name.as_str(), options)?;
            zip.write_all(text.as_bytes())?;
        }
        zip.finish()?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    std::fs::rename(&partial, out).inspect_err(|_| {
        let _ = std::fs::remove_file(&partial);
    })?;
    Ok(Summary { files: entries.iter().map(|(n, t)| (n.clone(), t.len() as u64)).collect(), zip_bytes: std::fs::metadata(out).map(|m| m.len()).unwrap_or(0) })
}

/// The bundle's files (name in the zip, text), with personal details taken out as in [`write`], without writing anything:
/// the feedback form attaches the ones the person picks (encrypted in the page before they are sent).
pub fn entries(input: &Input) -> Vec<(String, String)> {
    entries_with(input, Redactor::for_this_pc())
}

fn entries_with(input: &Input, mut redactor: Redactor) -> Vec<(String, String)> {
    // which names to hide: the ones the check saw, then any the saved files name
    let mut names: Vec<String> = input.report.names.clone();
    let characters = read_text(&input.data_dir.join("characters.json"));
    if let Some(Ok(v)) = characters.as_ref().map(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}'))) {
        for e in v.as_array().into_iter().flatten() {
            if let Some(n) = e.get("Name").and_then(|n| n.as_str()) {
                names.push(n.to_string());
            }
        }
    }
    let settings = input.settings_path.as_deref().and_then(read_text);
    if let Some(Ok(v)) = settings.as_ref().map(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}'))) {
        for k in ["ActiveCharacter", "Character"] {
            if let Some(n) = v.get(k).and_then(|n| n.as_str()) {
                names.push(n.to_string());
            }
        }
    }
    if input.include_names {
        redactor.keep_names(&names);
    } else {
        redactor.hide_names(&names);
    }
    let clean = |s: &str| redactor.text(s);

    let mut entries: Vec<(String, String)> = Vec::new();
    let mut about = input.about.trim_end().to_string();
    about.push_str("\r\n");
    about.push_str(&format!("Data folder: {}\r\n", input.data_dir.display()));
    about.push_str(&format!("Character names: {}\r\n", if input.include_names { "included" } else { "left out (Character 1, Character 2, ...)" }));
    about.push_str(&format!("Saved: {}\r\n\r\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
    about.push_str("This bundle holds: this file, doctor.txt (a self-check), the log files and crash reports, config.json, characters.json,\r\n");
    about.push_str("settings.json, and data-listing.txt (the data folder as a list of file names, sizes and dates).\r\n");
    about.push_str("It never holds: spreadsheets (CSV), anything read from the game's memory, pictures, tooltips or item data.\r\n");
    about.push_str("The Windows user name is taken out of every path.\r\n");
    entries.push(("about.txt".into(), clean(&about)));
    entries.push(("doctor.txt".into(), clean(&input.report.to_text())));

    let logs = input.data_dir.join("logs");
    let mut log_files: Vec<PathBuf> = std::fs::read_dir(&logs)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    log_files.sort();
    for p in log_files {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        if !(lower.ends_with(".log") || (lower.starts_with("crash-") && lower.ends_with(".txt"))) {
            continue;
        }
        if let Some(t) = read_text(&p) {
            entries.push((format!("logs/{}", clean(&name)), clean(&tail(&t, FILE_CAP))));
        }
    }
    if let Some(t) = read_text(&input.config_path) {
        entries.push(("data/config.json".into(), clean(&t)));
    }
    if let Some(t) = characters {
        entries.push(("data/characters.json".into(), clean(&t)));
    }
    if let Some(t) = settings {
        entries.push(("settings.json".into(), clean(&t)));
    }
    entries.push(("data-listing.txt".into(), clean(&listing(input.data_dir))));
    entries
}

fn read_text(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// The last `max` bytes of a text (cut at a line start).
fn tail(t: &str, max: usize) -> String {
    if t.len() <= max {
        return t.to_string();
    }
    let mut from = t.len() - max;
    while !t.is_char_boundary(from) {
        from += 1;
    }
    let rest = &t[from..];
    match rest.find('\n') {
        Some(i) => format!("(the start is left out)\r\n{}", &rest[i + 1..]),
        None => rest.to_string(),
    }
}

/// The data folder as text: date, size and relative path of every file (no contents), at most [`LISTING_ENTRIES`].
pub fn listing(dir: &Path) -> String {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, u64, Option<std::time::SystemTime>)>, depth: usize) {
        if out.len() >= LISTING_ENTRIES || depth > 10 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                walk(root, &p, out, depth + 1);
            } else if out.len() < LISTING_ENTRIES {
                let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().into_owned();
                out.push((rel, meta.len(), meta.modified().ok()));
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files, 0);
    files.sort_by_key(|f| f.0.to_lowercase());
    let mut s = format!("The data folder {} (relative path, size in bytes, last change). Contents are not included.\r\n", dir.display());
    for (rel, size, modified) in &files {
        let when = modified.map(|t| DateTime::<Local>::from(t).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "?".to_string());
        s.push_str(&format!("{when}  {size:>10}  {rel}\r\n"));
    }
    if files.len() >= LISTING_ENTRIES {
        s.push_str("(the list stops here)\r\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::doctor::{Check, Status};
    use std::io::Read;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("imagine-bundle-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn read_zip(path: &Path) -> Vec<(String, String)> {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        (0..z.len())
            .map(|i| {
                let mut f = z.by_index(i).unwrap();
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                (f.name().to_string(), s)
            })
            .collect()
    }

    fn fixture(root: &Path) -> (PathBuf, Report) {
        let data = root.join("data");
        std::fs::create_dir_all(data.join("logs")).unwrap();
        std::fs::create_dir_all(data.join("characters").join("Rokujuu").join("dash")).unwrap();
        std::fs::write(data.join("logs").join("imagine-20261124-000000-000.log"), "2026-11-24 00:00:00.000 INFO  [1] snapshot for Rubito at C:\\Users\\Joe\\x\r\n").unwrap();
        std::fs::write(data.join("logs").join("crash-20261124-000001.txt"), "panicked at C:\\Users\\Joe\\.cargo\\a.rs:1:1\r\n").unwrap();
        std::fs::write(data.join("logs").join("other.bin"), "not included").unwrap();
        std::fs::write(data.join("config.json"), "{\"statsFolder\": \"C:\\\\Users\\\\Joe\\\\Documents\\\\smt-stats\"}").unwrap();
        std::fs::write(
            data.join("characters.json"),
            "[{\"Name\":\"Rubito\",\"Dir\":\".\",\"Csv\":\"C:\\\\Users\\\\Joe\\\\Documents\\\\smt-stats\\\\rubito-stats.csv\"},{\"Name\":\"Rokujuu\",\"Dir\":\"characters\\\\Rokujuu\",\"Csv\":\"C:\\\\x\\\\rokujuu-stats.csv\"}]",
        )
        .unwrap();
        std::fs::write(data.join("characters").join("Rokujuu").join("dash").join("snapshots.json"), "[]").unwrap();
        std::fs::write(data.join("secret.csv"), "Timestamp,Character\n1,Rubito\n").unwrap();
        let report = Report {
            program: "BABEL 1.0.0 (commit abc1234, built 2026-11-24 03:41 UTC)".into(),
            windows: "Windows 11".into(),
            checked_at: "2026-11-24 04:00:00".into(),
            checks: vec![Check { section: "Game".into(), name: "Game running".into(), status: Status::Pass, detail: "client1 -> Rubito (level 95)".into(), advice: String::new(), ms: 1 }],
            names: vec!["Rubito".into()],
        };
        (data, report)
    }

    #[test]
    fn the_bundle_holds_the_listed_files_and_no_user_name_or_character_name() {
        let root = scratch("a");
        let (data, report) = fixture(&root);
        let input = Input { data_dir: &data, config_path: data.join("config.json"), settings_path: None, include_names: false, report: &report, about: "BABEL 1.0.0\r\n".into() };
        let out = root.join("bundle.zip");
        let summary = write_with(&out, &input, Redactor::new(Some(r"C:\Users\Joe"), Some("Joe"))).unwrap();
        let files = read_zip(&out);
        let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(
            names,
            ["about.txt", "doctor.txt", "logs/crash-20261124-000001.txt", "logs/imagine-20261124-000000-000.log", "data/config.json", "data/characters.json", "data-listing.txt"]
        );
        assert_eq!(summary.files.len(), files.len());
        assert!(summary.zip_bytes > 0);
        for (name, text) in &files {
            for secret in ["Joe", "Rubito", "Rokujuu", "rubito", "rokujuu"] {
                assert!(!text.contains(secret), "{name} still has {secret}:\n{text}");
            }
        }
        let by = |n: &str| files.iter().find(|f| f.0 == n).unwrap().1.clone();
        assert!(by("logs/imagine-20261124-000000-000.log").contains("snapshot for Character 1 at %USERPROFILE%\\x"));
        assert!(by("data/config.json").contains("%USERPROFILE%\\\\Documents\\\\smt-stats"));
        assert!(by("data/characters.json").contains("\"Name\":\"Character 1\""));
        assert!(by("data/characters.json").contains("characters\\\\Character 2"));
        assert!(by("doctor.txt").contains("client1 -> Character 1 (level 95)"));
        // the listing names the files, not their contents, and the spreadsheet-like file is only a line
        let listing = by("data-listing.txt");
        assert!(listing.contains("secret.csv") && !listing.contains("Timestamp,Character"));
        assert!(listing.contains("characters\\Character 2\\dash\\snapshots.json"));
        // nothing but the bundle was left behind
        assert!(!root.join("bundle.zip.partial").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ticking_include_names_keeps_the_names_but_never_the_user_name() {
        let root = scratch("b");
        let (data, report) = fixture(&root);
        let input = Input { data_dir: &data, config_path: data.join("config.json"), settings_path: None, include_names: true, report: &report, about: "x".into() };
        let out = root.join("bundle.zip");
        write_with(&out, &input, Redactor::new(Some(r"C:\Users\Joe"), Some("Joe"))).unwrap();
        let files = read_zip(&out);
        assert!(files.iter().any(|f| f.1.contains("Rubito")));
        for (name, text) in &files {
            assert!(!text.contains("Joe"), "{name} still has the user name");
            assert!(!text.contains("Character 1"), "{name}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }


    #[test]
    fn a_character_with_the_users_own_name_is_still_a_character() {
        // the Windows user and the character are both "Rubito" (it happens: this PC)
        let root = scratch("c");
        let (data, report) = fixture(&root);
        let out = root.join("bundle.zip");
        let input = Input { data_dir: &data, config_path: data.join("config.json"), settings_path: None, include_names: false, report: &report, about: "x".into() };
        write_with(&out, &input, Redactor::new(Some(r"C:\Users\Joe"), Some("Rubito"))).unwrap();
        let files = read_zip(&out);
        for (name, text) in &files {
            assert!(!text.to_lowercase().contains("rubito"), "{name} still has the name");
            assert!(!text.contains("Joe"), "{name}");
        }
        assert!(files.iter().any(|f| f.1.contains("Character 1")), "the character is Character 1, not <user>");
        // with the names kept, the character stays Rubito everywhere; the profile folder is out all the same
        let input = Input { include_names: true, ..input };
        write_with(&out, &input, Redactor::new(Some(r"C:\Users\Joe"), Some("Rubito"))).unwrap();
        let files = read_zip(&out);
        assert!(files.iter().any(|f| f.0 == "data/characters.json" && f.1.contains("\"Name\":\"Rubito\"")));
        for (name, text) in &files {
            assert!(!text.contains("Joe"), "{name}");
            assert!(!text.contains("<user>"), "{name}: the user name is the character's name here");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn a_big_log_keeps_its_newest_part_and_the_name_is_a_default() {
        let t = "line one\r\nline two\r\nline three\r\n".repeat(10);
        let cut = tail(&t, 50);
        assert!(cut.starts_with("(the start is left out)"));
        assert!(cut.ends_with("line three\r\n") || cut.ends_with("line three\r\n"));
        assert_eq!(tail("short", 50), "short");
        let n = default_name();
        assert!(n.starts_with("imagine-support-") && n.ends_with(".zip") && n.len() == "imagine-support-20261124-0341.zip".len(), "{n}");
    }
}
