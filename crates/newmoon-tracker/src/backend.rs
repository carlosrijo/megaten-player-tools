//! Everything MainForm.cs needs from the reader library, behind one module.
//!
//! The library (crates\newmoon) is being ported in parallel. Until its pieces exist, every call into it is a function
//! here named after what MainForm does, documented with the C# call it stands for. Calls that are not wired yet
//! return `Err("not ported yet: <C# call>")` (or a harmless default where the C# would also cope, e.g. no clients).
//! `WIRING.md` (next to Cargo.toml) lists each function, the C# it replaces (file:line) and what the real
//! implementation must return.
//!
//! Section 1: what is really implemented here because it is the app's own business (the ClientPids/MainWindowHandle
//!            process helpers and the data folder lookup).
//! Section 2: TEMPORARY stand-ins for `data::characters`, `data::config` and the tiny part of `data::store`
//!            MainForm reads. Replace by the library's types when they exist.
//! Section 3: the library calls (stubs).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

// ====================================================================================================================
// 1. The app's own helpers
// ====================================================================================================================

/// `Pipeline.FindDataDir()` = `Path.Combine(Config.HomeDir(), "data")`: the data folder next to the repo (found by
/// walking up from the program's folder, then from the current folder), else beside the exe itself (portable release).
pub fn find_data_dir() -> PathBuf {
    home_dir().join("data")
}

/// `Config.RepoRoot()`: the repo folder, walking up from the program, else from the current folder. In the Rust repo
/// the marker is `crates\newmoon\Cargo.toml` (the C# marker `NewMoonReader.csproj` counts too).
pub fn repo_root() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            starts.push(d.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    for start in starts {
        let mut d: Option<&Path> = Some(start.as_path());
        while let Some(dir) = d {
            if dir.join("crates").join("newmoon").join("Cargo.toml").is_file() || dir.join("NewMoonReader.csproj").is_file() {
                return Some(dir.to_path_buf());
            }
            d = dir.parent();
        }
    }
    None
}

/// `Config.HomeDir()`: the repo when run from it, else the folder the program itself sits in.
pub fn home_dir() -> PathBuf {
    repo_root().unwrap_or_else(program_dir)
}

/// `Config.ProgramDir()`: the folder of the running exe.
pub fn program_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `Process.GetProcessesByName("ImagineClient")` as ids (MainForm.ClientPids).
pub fn client_pids() -> std::collections::HashSet<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::*;
    let mut ids = std::collections::HashSet::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return ids };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let name = String::from_utf16_lossy(&e.szExeFile[..len]);
                if name.eq_ignore_ascii_case("ImagineClient.exe") || name.eq_ignore_ascii_case("ImagineClient") {
                    ids.insert(e.th32ProcessID);
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    ids
}

/// `Process.GetProcessById(pid).MainModule.FileName`'s folder: where that program's exe lives (None when it can't be asked).
pub fn process_exe_dir(pid: u32) -> Option<PathBuf> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::*;
    use windows::core::PWSTR;
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let p = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
        p.parent().map(|d| d.to_path_buf())
    }
}

/// .NET `Process.MainWindowHandle`: the first visible, unowned top-level window of the process (0 when it has none).
pub fn main_window_handle(pid: u32) -> isize {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::*;
    struct Find {
        pid: u32,
        found: isize,
    }
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let f = &mut *(lparam.0 as *mut Find);
            let mut p = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut p));
            if p == f.pid && GetWindow(hwnd, GW_OWNER).map(|h| h.0.is_null()).unwrap_or(true) && IsWindowVisible(hwnd).as_bool() {
                f.found = hwnd.0 as isize;
                return BOOL(0);
            }
            BOOL(1)
        }
    }
    let mut f = Find { pid, found: 0 };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut f as *mut Find as isize));
    }
    f.found
}

// ====================================================================================================================
// 2. TEMPORARY stand-ins (replace by data::characters, data::config, data::store of the library)
// ====================================================================================================================

/// TEMPORARY: `Config.Current` as far as the app needs it (src\Data\Config.cs): `<home>\data\config.json`
/// "statsFolder" / "defaultCharacter". Unlike the C# it does not write the file when it is missing.
struct ConfigLite {
    stats_folder: PathBuf,
    default_character: Option<String>,
}

fn config() -> ConfigLite {
    // C#: Documents\smt-stats unless config.json says otherwise
    let documents = known_documents_dir();
    let mut c = ConfigLite { stats_folder: documents.join("smt-stats"), default_character: None };
    let path = home_dir().join("data").join("config.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(j) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
            let s = |k: &str| j.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
            if let Some(v) = s("statsFolder") {
                c.stats_folder = PathBuf::from(v);
            }
            c.default_character = s("defaultCharacter");
        }
    }
    c
}

fn known_documents_dir() -> PathBuf {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            let s = p.to_string().unwrap_or_default();
            CoTaskMemFree(Some(p.as_ptr() as *const _));
            if !s.is_empty() {
                return PathBuf::from(s);
            }
        }
    }
    std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("Documents")).unwrap_or_else(|| PathBuf::from("."))
}

/// `Pipeline.DefaultName`: who the data is for when nothing else says (the defaultCharacter setting in data/config.json).
pub fn default_name() -> String {
    config().default_character.unwrap_or_else(|| "Character".to_string())
}

/// `Pipeline.DefaultCsv`: the default character's CSV, in the stats folder.
pub fn default_csv() -> PathBuf {
    config().stats_folder.join(format!("{}-stats.csv", default_name().to_lowercase()))
}

/// TEMPORARY: `CharacterEntry` (src\Data\Characters.cs). Property names as the C# serializer wrote them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CharacterEntry {
    pub name: String,
    /// Folder for this character's files, relative to the data folder ("." = the data folder itself).
    #[serde(default)]
    pub dir: Option<String>,
    #[serde(default)]
    pub csv: Option<String>,
    /// The app only: false = keep the picture the person chose; otherwise a snapshot taken with the Information
    /// window open replaces the picture with that window's character view.
    #[serde(default)]
    pub auto_picture: Option<bool>,
}

/// TEMPORARY: `Characters` (src\Data\Characters.cs): the list of known characters (data\characters.json).
pub struct Characters {
    root: PathBuf,
    default_csv: String,
    pub all: Vec<CharacterEntry>,
}

impl Characters {
    pub fn new(data_root: &Path, default_name: &str, default_csv: &str) -> Characters {
        let mut c = Characters { root: data_root.to_path_buf(), default_csv: default_csv.to_string(), all: Vec::new() };
        let file = c.file_path();
        if file.is_file() {
            if let Ok(text) = std::fs::read_to_string(&file) {
                c.all = serde_json::from_str::<Vec<CharacterEntry>>(text.trim_start_matches('\u{feff}')).unwrap_or_default();
            }
            c.all.retain(|e| !e.name.trim().is_empty());
        } else if data_root.join("dash").is_dir() || data_root.join("cache").is_dir() {
            c.all.push(CharacterEntry {
                name: c.existing_name().unwrap_or_else(|| default_name.to_string()),
                dir: Some(".".to_string()),
                csv: Some(default_csv.to_string()),
                auto_picture: None,
            });
            c.save();
        }
        c
    }

    fn file_path(&self) -> PathBuf {
        self.root.join("characters.json")
    }

    /// Whose data is already in the data folder: the character named in its latest saved snapshot.
    fn existing_name(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.root.join("dash").join("snapshots.json")).ok()?;
        let j: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
        let n = j.as_array()?.last()?.get("character")?.as_str()?.to_string();
        if n.trim().is_empty() { None } else { Some(n) }
    }

    pub fn find(&self, name: &str) -> Option<&CharacterEntry> {
        self.all.iter().find(|e| newmoon::text::eq_ic(&e.name, name))
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut CharacterEntry> {
        self.all.iter_mut().find(|e| newmoon::text::eq_ic(&e.name, name))
    }

    /// The entry for a character, created with its own folder and CSV the first time the name is seen.
    pub fn get_or_add(&mut self, name: &str) -> CharacterEntry {
        if let Some(e) = self.find(name) {
            return e.clone();
        }
        let safe: String = name
            .chars()
            .map(|c| if c == '.' || (c as u32) < 32 || "\"<>|:*?\\/".contains(c) { '_' } else { c })
            .collect();
        let first_csv = self.all.first().and_then(|e| e.csv.clone()).unwrap_or_else(|| self.default_csv.clone());
        let csv_dir = Path::new(&first_csv).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.root.clone());
        let e = CharacterEntry {
            name: name.to_string(),
            dir: Some(format!("characters\\{safe}")),
            csv: Some(csv_dir.join(format!("{}-stats.csv", safe.to_lowercase())).to_string_lossy().into_owned()),
            auto_picture: None,
        };
        self.all.push(e.clone());
        self.save();
        e
    }

    pub fn data_dir(&self, e: &CharacterEntry) -> PathBuf {
        match e.dir.as_deref() {
            None | Some("") | Some(".") => self.root.clone(),
            Some(d) => self.root.join(d),
        }
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(&self.root);
        if let Ok(json) = serde_json::to_string_pretty(&self.all) {
            let _ = std::fs::write(self.file_path(), json);
        }
    }
}

/// TEMPORARY: the few `Store` paths and the alignment MainForm.LoadData reads (src\Data\Store.cs).
pub struct StoreLite {
    pub dir: PathBuf,
}

impl StoreLite {
    pub fn new(dir: impl Into<PathBuf>) -> StoreLite {
        StoreLite { dir: dir.into() }
    }
    pub fn dash_dir(&self) -> PathBuf {
        self.dir.join("dash")
    }
    pub fn profile_path(&self) -> PathBuf {
        self.dir.join("cache").join("profile.json")
    }
    pub fn expertise_ranks_path(&self) -> PathBuf {
        self.dir.join("expertise-ranks.json")
    }
    /// `Store.Alignment()`
    pub fn alignment(&self) -> String {
        let p = self.profile_path();
        let Ok(text) = std::fs::read_to_string(p) else { return "NEUTRAL".to_string() };
        let a = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))
            .ok()
            .and_then(|j| crate::jsonx::str(j.get("alignment")))
            .unwrap_or_default();
        if a.is_empty() { "NEUTRAL".to_string() } else { a.to_uppercase() }
    }
}

// ====================================================================================================================
// 3. Library calls (stubs until the library is ported)
// ====================================================================================================================

fn not_ported<T>(what: &str) -> Result<T> {
    Err(anyhow!("not ported yet: {what}"))
}

/// `ClientReader.ClientInfo`: one running client and who is logged in (None = nobody yet).
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub pid: u32,
    pub folder: String,
    pub character: Option<String>,
    pub level: i32,
}

/// What `ClientReader.FindClient` returns (a `Process`): only the id is needed, the window handle is looked up here.
#[derive(Debug, Clone)]
pub struct ClientProcess {
    pub pid: u32,
}

impl ClientProcess {
    /// `Process.MainWindowHandle`
    pub fn main_window_handle(&self) -> isize {
        main_window_handle(self.pid)
    }
}

/// `Snapshot.Result` as MainForm.snapshot sends it, plus the expertise ranks count of `Pipeline.TakeSnapshot`.
#[derive(Debug, Clone)]
pub struct SnapshotOutcome {
    pub source: String,
    pub stats_note: String,
    /// `Result.Missing` (null in C# -> "" in the page's JSON)
    pub missing: Option<String>,
    pub level: i32,
    pub ranks: i32,
}

/// `Pipeline.AlignmentResult` (the Log is not sent to the page).
#[derive(Debug, Clone)]
pub struct AlignmentOutcome {
    pub changed: bool,
    pub rows: i32,
    pub changed_rows: i32,
    pub backup: Option<PathBuf>,
}

/// What `MainForm.SimPipe` needs to find or make a Pipeline: the characters (name -> folder) and the data folder.
pub struct SimRequest<'a> {
    /// `e.Name` of the active character (the default "base")
    pub default_base: &'a str,
    pub data_dir: &'a Path,
    /// every saved character: (name, its data folder), for `Pipe(_chars.Find(who))` and the "_blank" pipeline's sets
    pub characters: &'a [(String, PathBuf)],
    /// the active character's data folder (`new Store(_chars.DataDir(e))`, the blank pipeline's icon cache)
    pub active_dir: &'a Path,
    /// `<data>\sim-builds.json`
    pub builds_path: PathBuf,
    pub cmd: &'a str,
    pub args: &'a Value,
    /// the character whose folder is the data folder (builds saved before they had bases were theirs)
    pub owner: &'a str,
    /// `SkillIcons(who)`: address of that character's dash\skill-icons folder for the page
    pub skill_icons: &'a (dyn Fn(&str) -> Option<String> + Sync),
}

/// The reader. In C# this is MainForm's `_pipes` (one `Pipeline` per character folder), `_gameData` (the reference
/// data shared by all of them) and `_blank`; the real implementation keeps those inside this struct.
#[derive(Default)]
pub struct Backend {}

impl Backend {
    pub fn new() -> Backend {
        Backend {}
    }

    /// C# `ClientReader.ListClients()` (Task.Run'd by the "clients" command). Read-only look at every running client:
    /// who is logged in where, sorted by folder then pid. About a second per client.
    /// STUB: no clients (the same as no game running).
    pub fn list_clients(&self) -> Result<Vec<ClientInfo>> {
        Ok(Vec::new())
    }

    /// C# `ClientReader.FindClient(name)`: the client this character is logged in to. Throws UserError
    /// "ImagineClient is not running." / "No running client has '{name}' logged in."
    pub fn find_client(&self, _name: &str) -> Result<ClientProcess> {
        not_ported("ClientReader.FindClient")
    }

    /// C# `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)` (src\Build\Pipeline.cs:44): expertise first, then the snapshot
    /// into the character's CSV. `character_dir` = `_chars.DataDir(e)` (the Pipeline is created per folder and kept).
    pub fn take_snapshot(&self, _name: &str, _character_dir: &Path, _csv: &str) -> Result<SnapshotOutcome> {
        not_ported("Pipeline.TakeSnapshot")
    }

    /// C# `Pipe(e).Rebuild(e.Csv).Snapshots` (src\Build\Pipeline.cs:348): rebuilds the dash\*.json files from the CSV;
    /// returns the number of snapshots (`Counts.Snapshots`).
    pub fn rebuild(&self, _character_dir: &Path, _csv: &str) -> Result<i32> {
        not_ported("Pipeline.Rebuild")
    }

    /// C# `Pipe(e).SetAlignment(e.Csv, value)` (src\Build\Pipeline.cs:78). `value` may be null; throws UserError
    /// "Alignment must be LAW, NEUTRAL or CHAOS." for anything else.
    pub fn set_alignment(&self, _character_dir: &Path, _csv: &str, _value: Option<&str>) -> Result<AlignmentOutcome> {
        not_ported("Pipeline.SetAlignment")
    }

    /// C# `Pipe(e).CompDemons(e.Name, scan)` (src\Build\Pipeline.cs:200), serialized with camelCase property names.
    /// Returns the JSON array text/value the page shows (`[CompDemon...]`).
    pub fn comp_demons(&self, _name: &str, _character_dir: &Path, _scan: bool) -> Result<Value> {
        not_ported("Pipeline.CompDemons")
    }

    /// C# `Pipe(e).UseDemon(e.Name, e.Csv, key, scan: false)` (src\Build\Pipeline.cs:305): the `DemonSwitch` record as camelCase JSON.
    pub fn use_demon(&self, _name: &str, _character_dir: &Path, _csv: &str, _key: Option<&str>) -> Result<Value> {
        not_ported("Pipeline.UseDemon")
    }

    /// C# `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)`
    /// (src\Build\PreviewServer.cs:35) for simInit/simulate/simCatalog/simDemons/simBuilds/simSaveBuild/simDeleteBuild.
    /// Returns the JSON text for the page. See `SimRequest` for how `SimPipe` is expressed.
    pub fn sim_command(&self, _req: &SimRequest) -> Result<String> {
        not_ported("PreviewServer.SimCommand")
    }

    /// C# `ItemIcons.ClientDir(new Store(dir))` (src\Build\ItemIcons.cs:135): the game client's folder as last seen
    /// running (cache\client-dir.txt), or None (remembers "not found" for as long as the app runs).
    pub fn item_icons_client_dir(&self, _store_dir: &Path) -> Result<Option<PathBuf>> {
        Ok(None)
    }

    /// C# `DemonModel.Load(dataRoot, clientDir, DemonModel.Assistant, save)` (src\Build\DemonModel.cs:29): the assistant
    /// demon's 3D model as one JSON text, built from the client's files the first time and kept in
    /// `<data>\cache\models`. None = "null" (client folder unknown or the model can't be read: C# catches NifException,
    /// IOException and UnauthorizedAccessException into "null").
    /// STUB: None (the page shows no assistant).
    pub fn demon_model_load(&self, _data_root: &Path, _client_dir: Option<&Path>, _save: bool) -> Result<Option<String>> {
        Ok(None)
    }
}
