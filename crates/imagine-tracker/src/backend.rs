//! Everything MainForm.cs needs from the reader library, behind one module.
//!
//! Section 1: the app's own helpers (the `ClientPids` of MainForm.cs).
//! Section 2: the library, re-exported under the names the app uses (data::characters, data::store,
//!            game::client_reader) and the thin helpers the C# `Pipeline` statics gave MainForm (now build::pipeline's).
//! Section 3: the library calls: `Backend` keeps what MainForm kept (`_gameData`, `_pipes`, `_blank`) and wires the
//!            Pipeline commands (snapshot, rebuild, setAlignment, compDemons, useDemon) and the Simulator
//!            (`sim_command` -> build::preview_server::sim_command).
//!            `WIRING.md` (next to Cargo.toml) lists what is where.

use anyhow::Result;
use imagine::build::pipeline::{self, Pipeline};
use imagine::build::{demon_model, item_icons};
use imagine::data::game_data::GameData;
use imagine::game::SetRecord;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

pub use imagine::data::characters::{CharacterEntry, Characters};
pub use imagine::data::store::Store;
pub use imagine::game::client_reader::{ClientInfo, ClientProcess};

// ====================================================================================================================
// 1. The app's own helpers
// ====================================================================================================================

/// `Process.GetProcessesByName("ImagineClient")` as ids (MainForm.ClientPids).
pub fn client_pids() -> std::collections::HashSet<u32> {
    imagine::game::client_reader::client_processes().into_iter().map(|p| p.pid).collect()
}

/// `Path.GetDirectoryName(Process.GetProcessById(pid).MainModule.FileName)`: where that program's exe lives (None when it can't be asked).
pub fn process_exe_dir(pid: u32) -> Option<PathBuf> {
    imagine::game::client_reader::process_exe_path(pid).and_then(|p| p.parent().map(Path::to_path_buf))
}

// ====================================================================================================================
// 2. The library under the app's names
// ====================================================================================================================

/// `Pipeline.FindDataDir()`.
pub fn find_data_dir() -> PathBuf {
    pipeline::find_data_dir()
}

/// `Pipeline.DefaultName`.
pub fn default_name() -> String {
    pipeline::default_name()
}

/// `Pipeline.DefaultCsv`. The C# throws (ArgumentNullException from `Path.Combine`) when data\config.json has no
/// statsFolder; the config always fills it in, so the library's error can only mean a null written by hand: an empty
/// path then (the first character's CSV is then simply not found, never a crash on start).
pub fn default_csv() -> PathBuf {
    PathBuf::from(pipeline::default_csv().unwrap_or_default())
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
// 3. Library calls
// ====================================================================================================================

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

/// `_pipes` and `_blank`, together under one lock (C# `lock (_pipes)`).
#[derive(Default)]
struct Pipes {
    /// one Pipeline per character folder (C# keys by character name, case-insensitively; a folder belongs to one name)
    by_dir: HashMap<String, Arc<Pipeline>>,
    /// the Simulator's "no base": a pipeline with no character behind it
    blank: Option<Arc<Pipeline>>,
}

/// The reader. In C# this is MainForm's `_pipes` (one `Pipeline` per character folder), `_gameData` (the reference
/// data shared by all of them) and `_blank`.
#[derive(Default)]
pub struct Backend {
    /// `_gameData ??= new GameData()`: the reference data, loaded on first use (it fills its tables lazily) and shared.
    game_data: OnceLock<Arc<GameData>>,
    pipes: Mutex<Pipes>,
    /// The data folders of every saved character as of the last Simulator request: what the blank pipeline's
    /// `blank_sets` reads (the C# closure reads `_chars.All` each time it runs; the characters are only handed to the
    /// backend with a request).
    char_dirs: Arc<Mutex<Vec<PathBuf>>>,
}

impl Backend {
    pub fn new() -> Backend {
        Backend::default()
    }

    /// C# `_gameData ??= new GameData()`: the one reference data every pipeline shares.
    pub fn game_data(&self) -> Arc<GameData> {
        self.game_data.get_or_init(|| Arc::new(GameData::new())).clone()
    }

    fn pipes(&self) -> MutexGuard<'_, Pipes> {
        self.pipes.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// C# `Pipe(e)`: the reader for one character's folder, made on first use and kept. The reference data is shared.
    fn pipe(&self, character_dir: &Path) -> Arc<Pipeline> {
        let key = character_dir.to_string_lossy().to_lowercase();
        let mut p = self.pipes();
        p.by_dir.entry(key).or_insert_with(|| Arc::new(Pipeline::new(character_dir, Some(self.game_data()), None, None))).clone()
    }

    /// C# `SimPipe(who)` of MainForm.Dispatch (the `pipeOf` PreviewServer.SimCommand is given): the pipeline of the
    /// character a build stands on, or for `who == ""` the "_blank" one: a pipeline with no character behind it
    /// (folder `<data>\characters\_blank`), whose set table is the first saved character's that has one and whose
    /// icon cache is the active character's. Throws a UserError for a name that is not a saved character.

    pub fn sim_pipe(&self, req: &SimRequest, who: &str) -> Result<Arc<Pipeline>> {
        // what the blank pipeline's closure reads (C# reads `_chars.All` live)
        *self.char_dirs.lock().unwrap_or_else(|p| p.into_inner()) = req.characters.iter().map(|(_, d)| d.clone()).collect();
        if !who.is_empty() {
            let dir = req
                .characters
                .iter()
                .find(|(n, _)| imagine::text::eq_ic(n, who))
                .map(|(_, d)| d.clone())
                .ok_or_else(|| imagine::user_error(format!("There is no saved character called {who}.")))?;
            return Ok(self.pipe(&dir));
        }
        let mut p = self.pipes();
        if p.blank.is_none() {
            let dirs = self.char_dirs.clone();
            // `_chars.All.Select(c => new Store(DataDir(c))).Where(s => File.Exists(s.SetTablePath)).Select(s => s.LoadSetTable())
            //  .FirstOrDefault(t => t is { Count: > 0 })`: lazily, the first one that has sets
            let blank_sets: pipeline::BlankSets = Box::new(move || -> Result<Option<Vec<SetRecord>>> {
                let dirs = dirs.lock().unwrap_or_else(|p| p.into_inner()).clone();
                for d in dirs {
                    let store = Store::new(d);
                    if !store.set_table_path().is_file() {
                        continue;
                    }
                    if let Some(t) = store.load_set_table()?.filter(|t| !t.is_empty()) {
                        return Ok(Some(t));
                    }
                }
                Ok(None)
            });
            p.blank = Some(Arc::new(Pipeline::new(
                req.data_dir.join("characters").join("_blank"),
                Some(self.game_data()),
                Some(blank_sets),
                Some(Store::new(req.active_dir)),
            )));
        }
        Ok(p.blank.clone().expect("just made"))
    }

    /// C# `ClientReader.ListClients()` (Task.Run'd by the "clients" command). Read-only look at every running client:
    /// who is logged in where, sorted by folder then pid. About a second per client.
    pub fn list_clients(&self, keep_known: bool) -> Result<Vec<ClientInfo>> {
        Ok(imagine::game::client_reader::list_clients_with(keep_known))
    }

    /// C# `ClientReader.FindClient(name)`: the client this character is logged in to. Throws UserError
    /// "ImagineClient is not running." / "No running client has '{name}' logged in."
    pub fn find_client(&self, name: &str) -> Result<ClientProcess> {
        imagine::game::client_reader::find_client(name)
    }

    /// C# `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)` (src\Build\Pipeline.cs:44): expertise first, then the snapshot
    /// into the character's CSV. `character_dir` = `_chars.DataDir(e)` (the Pipeline is created per folder and kept).
    pub fn take_snapshot(&self, name: &str, character_dir: &Path, csv: &str) -> Result<SnapshotOutcome> {
        let (r, ranks) = self.pipe(character_dir).take_snapshot(name, csv, false)?;
        Ok(SnapshotOutcome { source: r.source, stats_note: r.stats_note, missing: Some(r.missing), level: r.level, ranks: ranks as i32 })
    }

    /// C# `Pipe(e).Rebuild(e.Csv).Snapshots` (src\Build\Pipeline.cs:348): rebuilds the dash\*.json files from the CSV;
    /// returns the number of snapshots (`Counts.Snapshots`).
    pub fn rebuild(&self, character_dir: &Path, csv: &str) -> Result<i32> {
        Ok(self.pipe(character_dir).rebuild(csv)?.snapshots as i32)
    }

    /// C# `Pipe(e).SetAlignment(e.Csv, value)` (src\Build\Pipeline.cs:78). `value` may be null; throws UserError
    /// "Alignment must be LAW, NEUTRAL or CHAOS." for anything else.
    pub fn set_alignment(&self, character_dir: &Path, csv: &str, value: Option<&str>) -> Result<AlignmentOutcome> {
        let r = self.pipe(character_dir).set_alignment(csv, value.unwrap_or(""))?;
        Ok(AlignmentOutcome { changed: r.changed, rows: r.rows as i32, changed_rows: r.changed_rows as i32, backup: r.backup.map(PathBuf::from) })
    }

    /// C# `Pipe(e).CompDemons(e.Name, scan)` (src\Build\Pipeline.cs:200), serialized with camelCase property names.
    /// Returns the JSON array the page shows (`[CompDemon...]`).
    pub fn comp_demons(&self, name: &str, character_dir: &Path, scan: bool) -> Result<Value> {
        let list = self.pipe(character_dir).comp_demons(name, scan)?;
        Ok(Value::Array(list.iter().map(|d| d.to_value()).collect()))
    }

    /// C# `Pipe(e).UseDemon(e.Name, e.Csv, key, scan: false)` (src\Build\Pipeline.cs:305): the `DemonSwitch` record as camelCase JSON.
    pub fn use_demon(&self, name: &str, character_dir: &Path, csv: &str, key: Option<&str>) -> Result<Value> {
        let r = self.pipe(character_dir).use_demon(name, csv, key, false)?;
        Ok(json!({ "name": r.name, "changed": r.changed, "rowUpdated": r.row_updated, "backup": r.backup }))
    }

    /// Delete one earlier snapshot (`Pipeline::delete_snapshot`): `{ backup }`, the copy's file name.
    pub fn delete_snapshot(&self, character_dir: &Path, csv: &str, taken_at: &str) -> Result<Value> {
        let p = self.pipe(character_dir);
        let backup = p.delete_snapshot(csv, taken_at)?;
        Ok(json!({ "backup": backup }))
    }

    /// Take a demon off the picker's list (`Pipeline::forget_demon`).
    pub fn forget_demon(&self, character_dir: &Path, key: &str) -> Result<()> {
        self.pipe(character_dir).forget_demon(key)
    }

    /// C# `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)`
    /// (src\Build\PreviewServer.cs:35) for simInit/simulate/simCatalog/simDemons/simBuilds/simSaveBuild/simDeleteBuild.
    /// Returns the JSON text for the page. See `SimRequest` for how `SimPipe` is expressed.
    pub fn sim_command(&self, req: &SimRequest) -> Result<String> {
        // MainForm holds the calc lock and has checked NoWrites before this.
        let pipe_of = |who: &str| self.sim_pipe(req, who);
        let skill_icons = |who: &str| (req.skill_icons)(who);
        let result = imagine::build::preview_server::sim_command(
            &pipe_of,
            req.default_base,
            &req.builds_path,
            req.cmd,
            Some(req.args),
            Some(req.owner),
            Some(&skill_icons),
        )?;
        // C# null (not a Simulator command; none reach here) is posted as the JSON null
        Ok(result.unwrap_or_else(|| "null".to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(data: &'a Path, characters: &'a [(String, PathBuf)], args: &'a Value, icons: &'a (dyn Fn(&str) -> Option<String> + Sync)) -> SimRequest<'a> {
        SimRequest {
            default_base: "Rubito",
            data_dir: data,
            characters,
            active_dir: data,
            builds_path: data.join("sim-builds.json"),
            cmd: "simInit",
            args,
            owner: "Rubito",
            skill_icons: icons,
        }
    }

    #[test]
    fn sim_pipe_finds_characters_and_keeps_pipelines() {
        let data = std::env::temp_dir().join(format!("imagine-simpipe-test-{}", std::process::id()));
        let chars = vec![("Rubito".to_string(), data.clone()), ("Rokujuu".to_string(), data.join("characters").join("Rokujuu"))];
        let args = Value::Null;
        let icons = |_: &str| None;
        let req = request(&data, &chars, &args, &icons);
        let b = Backend::new();
        // a name is found case-insensitively and its pipeline is made once
        let a1 = b.sim_pipe(&req, "rokujuu").unwrap();
        let a2 = b.sim_pipe(&req, "Rokujuu").unwrap();
        assert!(Arc::ptr_eq(&a1, &a2));
        assert!(Arc::ptr_eq(&a1, &b.pipe(&chars[1].1)));
        // the blank pipeline is its own, made once, in <data>\characters\_blank
        let blank = b.sim_pipe(&req, "").unwrap();
        assert!(Arc::ptr_eq(&blank, &b.sim_pipe(&req, "").unwrap()));
        assert!(!Arc::ptr_eq(&blank, &a1));
        assert_eq!(blank.store().dir, data.join("characters").join("_blank"));
        // an unknown name is a UserError
        let e = b.sim_pipe(&req, "Nobody").err().unwrap();
        assert!(imagine::error::is_user_error(&e));
        assert_eq!(e.to_string(), "There is no saved character called Nobody.");
        assert!(!data.exists()); // nothing was written
    }
}
