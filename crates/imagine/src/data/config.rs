//! Port of `src/Data/Config.cs`.
//!
//! Machine-specific settings, in data\config.json next to the project or the portable exe (created with detected defaults on first use,
//! then edited by hand when something lives elsewhere):
//!   websiteContent   the website's content folder (item, skill, tokusei data)
//!   tokuseiXml       comp_hack's datastore\data\tokusei folder (the server's tokusei XML)
//!   serverConstants  the local server's config\constants.xml (digitalize rates, Mitama set boost skill...)
//!   statsFolder      where new characters' CSV files go
//!   defaultCharacter the name used when nothing else says who (first run without a client)

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{Value, json};

use super::json::{self, At, Escape};

/// C# `Config` (the settings; `Config.Current` is [`current`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub website_content: Option<String>,
    pub tokusei_xml: Option<String>,
    pub server_constants: Option<String>,
    pub stats_folder: Option<String>,
    pub default_character: Option<String>,
}

static CURRENT: OnceLock<Config> = OnceLock::new();

/// `Config.Current`: loaded on first use (created with defaults when data\config.json is missing).
pub fn current() -> &'static Config {
    CURRENT.get_or_init(Config::load)
}

/// Fix the current config before anything reads it (tests, or a host that wants other paths). False when it was
/// already loaded; the existing value stays.
pub fn set_current(config: Config) -> bool {
    CURRENT.set(config).is_ok()
}

fn has_repo_marker(d: &Path) -> bool {
    d.join("crates").join("imagine").join("Cargo.toml").is_file()
}

/// The folder of the running exe (C# `AppContext.BaseDirectory`).
fn base_directory() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

/// The repo folder (the one with crates\imagine\Cargo.toml), walking up from the program,
/// else from the current folder (tools run from elsewhere); `None` if not found.
pub fn repo_root() -> Option<PathBuf> {
    let starts = [base_directory(), std::env::current_dir().ok()];
    for start in starts.into_iter().flatten() {
        for d in start.ancestors() {
            if has_repo_marker(d) {
                return Some(d.to_path_buf());
            }
        }
    }
    None
}

/// The folder whose data\ holds the user's data: the repo when run from it, else the folder the program itself sits in
/// (the portable release: one exe, with its data folder made beside it).
pub fn home_dir() -> PathBuf {
    repo_root().unwrap_or_else(program_dir)
}

/// The folder of the running exe (for a single-file build, where the exe is, not the temp folder it unpacks itself into).
pub fn program_dir() -> PathBuf {
    base_directory().or_else(|| std::env::current_dir().ok()).unwrap_or_default()
}

/// data\config.json
pub fn file_path() -> PathBuf {
    home_dir().join("data").join("config.json")
}

/// `Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)`.
pub fn user_profile() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default()
}

/// `Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments)`: the Documents known folder (it can be moved, e.g.
/// into OneDrive), else USERPROFILE\Documents.
pub fn my_documents() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = known_folder_documents() {
        return p;
    }
    user_profile().join("Documents")
}

#[cfg(windows)]
fn known_folder_documents() -> Option<PathBuf> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);
    // FOLDERID_Documents {FDD39AD0-238F-46AF-ADB4-6C85480369C7}
    let id = Guid(0xFDD39AD0, 0x238F, 0x46AF, [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7]);

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(rfid: *const Guid, flags: u32, token: *mut c_void, path: *mut *mut u16) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(p: *mut c_void);
    }

    let mut raw: *mut u16 = std::ptr::null_mut();
    // SAFETY: plain Win32 call; `raw` is only read when it succeeded, and freed with CoTaskMemFree as documented.
    unsafe {
        let hr = SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut raw);
        if hr < 0 || raw.is_null() {
            if !raw.is_null() {
                CoTaskMemFree(raw as *mut c_void);
            }
            return None;
        }
        let mut len = 0;
        while *raw.add(len) != 0 {
            len += 1;
        }
        let s = std::ffi::OsString::from_wide(std::slice::from_raw_parts(raw, len));
        CoTaskMemFree(raw as *mut c_void);
        Some(PathBuf::from(s))
    }
}

fn path_string(p: PathBuf) -> String {
    p.to_string_lossy().into_owned()
}

impl Config {
    /// C# `Config.Load`: the defaults, then data\config.json over them; writes the file when it is missing. An unreadable
    /// or malformed file leaves the defaults (the C# caught IOException, JsonException and UnauthorizedAccessException).
    pub fn load() -> Config {
        Config::load_from(repo_root().as_deref(), &file_path(), &user_profile(), &my_documents())
    }

    /// `load` with the places given (so tests can use a temp folder).
    pub fn load_from(root: Option<&Path>, file: &Path, home: &Path, documents: &Path) -> Config {
        let repos = root.and_then(Path::parent);
        let mut c = Config {
            website_content: repos.map(|r| path_string(r.join("ai-custom-smt-server").join("website").join("content"))),
            tokusei_xml: repos.map(|r| path_string(r.join("comp_hack").join("datastore").join("data").join("tokusei"))),
            server_constants: Some(path_string(home.join("smt").join("deploy").join("data").join("config").join("constants.xml"))),
            stats_folder: Some(path_string(documents.join("smt-stats"))),
            default_character: None,
        };
        let _ = c.read_or_create(file);
        c
    }

    fn read_or_create(&mut self, file: &Path) -> crate::Result<()> {
        if file.exists() {
            let j = json::load(file)?;
            let s = |k: &str, d: &Option<String>| match json::str(j.at(k)) {
                Some(v) if !v.is_empty() => Some(v),
                _ => d.clone(),
            };
            self.website_content = s("websiteContent", &self.website_content);
            self.tokusei_xml = s("tokuseiXml", &self.tokusei_xml);
            self.server_constants = s("serverConstants", &self.server_constants);
            self.stats_folder = s("statsFolder", &self.stats_folder);
            self.default_character = s("defaultCharacter", &None);
        } else {
            // Written for the portable release too, so its user has a file beside the exe to point the paths elsewhere
            json::create_parent(file)?;
            json::write_atomic(file, json::serialize(&self.to_json(), Escape::Default, true))?;
        }
        Ok(())
    }

    /// The anonymous object C# wrote to config.json (camelCase names, in this order, nulls written).
    pub fn to_json(&self) -> Value {
        json!({
            "websiteContent": self.website_content,
            "tokuseiXml": self.tokusei_xml,
            "serverConstants": self.server_constants,
            "statsFolder": self.stats_folder,
            "defaultCharacter": self.default_character,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_defaults_then_reads_them_back() {
        let dir = std::env::temp_dir().join(format!("imagine-config-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("repos").join("imagine-reader");
        let file = root.join("data").join("config.json");
        let c = Config::load_from(Some(&root), &file, &dir.join("home"), &dir.join("docs"));
        assert_eq!(c.stats_folder.as_deref(), Some(dir.join("docs").join("smt-stats").to_string_lossy().as_ref()));
        assert!(c.website_content.as_deref().unwrap().ends_with("ai-custom-smt-server\\website\\content") || cfg!(not(windows)));
        assert_eq!(c.default_character, None);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("{\r\n  \"websiteContent\": "));
        assert!(text.contains("\"defaultCharacter\": null\r\n}"));
        // reading what was written gives the same settings
        let again = Config::load_from(None, &file, &dir.join("elsewhere"), &dir.join("elsewhere"));
        assert_eq!(again, c);
        std::fs::write(&file, "{\"statsFolder\": \"D:\\\\x\", \"defaultCharacter\": \"Zed\", \"tokuseiXml\": \"\"}").unwrap();
        let c2 = Config::load_from(Some(&root), &file, &dir.join("home"), &dir.join("docs"));
        assert_eq!(c2.stats_folder.as_deref(), Some("D:\\x"));
        assert_eq!(c2.default_character.as_deref(), Some("Zed"));
        assert_eq!(c2.tokusei_xml, c.tokusei_xml);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_the_repo() {
        let root = repo_root().expect("tests run inside the repo");
        assert!(has_repo_marker(&root));
    }
}
