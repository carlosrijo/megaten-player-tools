//! Everything MainForm.cs needs from the reader library, behind one module.
//!
//! Section 1: the app's own helpers (the `ClientPids` of MainForm.cs).
//! Section 2: the library, re-exported under the names the app uses (data::characters, data::config, data::store,
//!            game::client_reader) and the thin helpers the C# `Pipeline` statics gave MainForm.
//! Section 3: the library calls that need the Pipeline / Snapshot / Dashboard / Expertise / Simulator / PreviewServer
//!            (not ported yet): stubs that return `Err("not ported yet: <C# call>")`, each marked `// WAVE4:` with the
//!            call it will make. `WIRING.md` (next to Cargo.toml) lists them.

use anyhow::{anyhow, Result};
use newmoon::build::{demon_model, item_icons};
use newmoon::data::config;
use newmoon::data::game_data::GameData;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

pub use newmoon::data::characters::{CharacterEntry, Characters};
pub use newmoon::data::store::Store;
pub use newmoon::game::client_reader::{ClientInfo, ClientProcess};

// ====================================================================================================================
// 1. The app's own helpers
// ====================================================================================================================

/// `Process.GetProcessesByName("ImagineClient")` as ids (MainForm.ClientPids).
pub fn client_pids() -> std::collections::HashSet<u32> {
    newmoon::game::client_reader::client_processes().into_iter().map(|p| p.pid).collect()
}

/// `Path.GetDirectoryName(Process.GetProcessById(pid).MainModule.FileName)`: where that program's exe lives (None when it can't be asked).
pub fn process_exe_dir(pid: u32) -> Option<PathBuf> {
    newmoon::game::client_reader::process_exe_path(pid).and_then(|p| p.parent().map(Path::to_path_buf))
}

// ====================================================================================================================
// 2. The library under the app's names
// ====================================================================================================================

/// `Pipeline.FindDataDir()` = `Path.Combine(Config.HomeDir(), "data")`.
// WAVE4: use build::pipeline::{find_data_dir, default_name, default_csv}
pub fn find_data_dir() -> PathBuf {
    config::home_dir().join("data")
}

/// `Pipeline.DefaultName` = `Config.Current.DefaultCharacter ?? "Character"`.
// WAVE4: use build::pipeline::{find_data_dir, default_name, default_csv}
pub fn default_name() -> String {
    config::current().default_character.clone().unwrap_or_else(|| "Character".to_string())
}

/// `Pipeline.DefaultCsv` = `Path.Combine(Config.Current.StatsFolder, DefaultName.ToLowerInvariant() + "-stats.csv")`.
// WAVE4: use build::pipeline::{find_data_dir, default_name, default_csv}
pub fn default_csv() -> PathBuf {
    PathBuf::from(config::current().stats_folder.clone().unwrap_or_default()).join(format!("{}-stats.csv", default_name().to_lowercase()))
}

/// `CharacterEntry.Name` is never blank for an entry in a `Characters` list (`new` drops blank names, `get_or_add`
/// sets one); the library keeps it an `Option` because the file may hold anything.
pub trait EntryExt {
    fn name(&self) -> &str;
}

impl EntryExt for CharacterEntry {
    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}

// ====================================================================================================================
// 3. Library calls: wired where the library has the piece, stubs (`// WAVE4:`) for the Pipeline / PreviewServer ones
// ====================================================================================================================

fn not_ported<T>(what: &str) -> Result<T> {
    Err(anyhow!("not ported yet: {what}"))
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
#[allow(dead_code)] // read by the real implementation
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
/// data shared by all of them) and `_blank`. The shared `GameData` is kept here; the pipelines come with the Pipeline port.
#[derive(Default)]
pub struct Backend {
    /// `_gameData ??= new GameData()`: the reference data, loaded on first use (it fills its tables lazily) and shared.
    game_data: OnceLock<Arc<GameData>>,
}

impl Backend {
    pub fn new() -> Backend {
        Backend::default()
    }

    /// C# `_gameData ??= new GameData()`: the one reference data every pipeline shares.
    #[allow(dead_code)] // WAVE4: handed to every `Pipeline::new(dir, game_data)` and to the blank (Simulator) pipeline
    pub fn game_data(&self) -> Arc<GameData> {
        self.game_data.get_or_init(|| Arc::new(GameData::new())).clone()
    }

    /// C# `ClientReader.ListClients()` (Task.Run'd by the "clients" command). Read-only look at every running client:
    /// who is logged in where, sorted by folder then pid. About a second per client.
    pub fn list_clients(&self) -> Result<Vec<ClientInfo>> {
        Ok(newmoon::game::client_reader::list_clients())
    }

    /// C# `ClientReader.FindClient(name)`: the client this character is logged in to. Throws UserError
    /// "ImagineClient is not running." / "No running client has '{name}' logged in."
    pub fn find_client(&self, name: &str) -> Result<ClientProcess> {
        newmoon::game::client_reader::find_client(name)
    }

    /// C# `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)` (src\Build\Pipeline.cs:44): expertise first, then the snapshot
    /// into the character's CSV. `character_dir` = `_chars.DataDir(e)` (the Pipeline is created per folder and kept).
    pub fn take_snapshot(&self, _name: &str, _character_dir: &Path, _csv: &str) -> Result<SnapshotOutcome> {
        // WAVE4: pipe(character_dir).take_snapshot(name, csv, false) -> ((Snapshot.Result), ranks) (build::pipeline::Pipeline, shared self.game_data())
        not_ported("Pipeline.TakeSnapshot")
    }

    /// C# `Pipe(e).Rebuild(e.Csv).Snapshots` (src\Build\Pipeline.cs:348): rebuilds the dash\*.json files from the CSV;
    /// returns the number of snapshots (`Counts.Snapshots`).
    pub fn rebuild(&self, _character_dir: &Path, _csv: &str) -> Result<i32> {
        // WAVE4: pipe(character_dir).rebuild(csv)?.snapshots (build::pipeline::Pipeline::rebuild)
        not_ported("Pipeline.Rebuild")
    }

    /// C# `Pipe(e).SetAlignment(e.Csv, value)` (src\Build\Pipeline.cs:78). `value` may be null; throws UserError
    /// "Alignment must be LAW, NEUTRAL or CHAOS." for anything else.
    pub fn set_alignment(&self, _character_dir: &Path, _csv: &str, _value: Option<&str>) -> Result<AlignmentOutcome> {
        // WAVE4: pipe(character_dir).set_alignment(csv, value) (build::pipeline::Pipeline::set_alignment)
        not_ported("Pipeline.SetAlignment")
    }

    /// C# `Pipe(e).CompDemons(e.Name, scan)` (src\Build\Pipeline.cs:200), serialized with camelCase property names.
    /// Returns the JSON array text/value the page shows (`[CompDemon...]`).
    pub fn comp_demons(&self, _name: &str, _character_dir: &Path, _scan: bool) -> Result<Value> {
        // WAVE4: pipe(character_dir).comp_demons(name, scan) -> camelCase JSON array (build::pipeline::Pipeline::comp_demons)
        not_ported("Pipeline.CompDemons")
    }

    /// C# `Pipe(e).UseDemon(e.Name, e.Csv, key, scan: false)` (src\Build\Pipeline.cs:305): the `DemonSwitch` record as camelCase JSON.
    pub fn use_demon(&self, _name: &str, _character_dir: &Path, _csv: &str, _key: Option<&str>) -> Result<Value> {
        // WAVE4: pipe(character_dir).use_demon(name, csv, key, false) -> camelCase JSON object (build::pipeline::Pipeline::use_demon)
        not_ported("Pipeline.UseDemon")
    }

    /// C# `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)`
    /// (src\Build\PreviewServer.cs:35) for simInit/simulate/simCatalog/simDemons/simBuilds/simSaveBuild/simDeleteBuild.
    /// Returns the JSON text for the page. See `SimRequest` for how `SimPipe` is expressed.
    pub fn sim_command(&self, _req: &SimRequest) -> Result<String> {
        // WAVE4: build::preview_server::sim_command(sim_pipe, default_base, builds_path, cmd, args, owner, skill_icons)
        // with sim_pipe(who) = pipe(characters.find(who)) or the "_blank" pipeline (see SimRequest)
        not_ported("PreviewServer.SimCommand")
    }

    /// C# `ItemIcons.ClientDir(new Store(dir))` (src\Build\ItemIcons.cs:135): the game client's folder as last seen
    /// running (cache\client-dir.txt), or None (remembers "not found" for as long as the app runs).
    pub fn item_icons_client_dir(&self, store_dir: &Path) -> Result<Option<PathBuf>> {
        Ok(item_icons::client_dir(&Store::new(store_dir), 0))
    }

    /// C# `DemonModel.Load(dataRoot, clientDir, DemonModel.Assistant, save)` (src\Build\DemonModel.cs:29): the assistant
    /// demon's 3D model as one JSON text, built from the client's files the first time and kept in
    /// `<data>\cache\models`. None = "null" (client folder unknown, or no such model). The callers turn the errors C#
    /// catches (NifException, IOException, UnauthorizedAccessException) into "null".
    pub fn demon_model_load(&self, data_root: &Path, client_dir: Option<&Path>, save: bool) -> Result<Option<String>> {
        demon_model::load(data_root, client_dir, demon_model::ASSISTANT, save)
    }
}
