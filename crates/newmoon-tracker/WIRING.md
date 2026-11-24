# Wiring the tracker app to the library

Every call from the app into the reader library goes through `src/backend.rs`. Section 1 of that file is real code
(the app's own business). Section 2 holds TEMPORARY stand-ins. Section 3 is the list below: stubs to replace once the
library has the piece. `Backend` (a unit struct now) is where the real implementation keeps what MainForm kept:
`_pipes` (one `Pipeline` per character folder, made on first use, shared `GameData`), `_gameData`, `_blank`.
Callers already serialize reader work (`MainForm::guarded` / `calc` lock), so `Backend` methods may take `&self`.

## Section 3: library calls (stubs)

| backend.rs function | C# it stands for | Called from (app\MainForm.cs) | Real implementation must |
|---|---|---|---|
| `Backend::list_clients() -> Result<Vec<ClientInfo>>` | `ClientReader.ListClients()` (src\Game\ClientReader.cs:124) | `clients` command, line 260 | return every running ImagineClient with `pid`, `folder` (name of the exe's folder), `character` (None = nobody logged in), `level`; sorted by folder (OrdinalIgnoreCase) then pid. Stub: empty list. Map `client_reader::ClientInfo` onto `backend::ClientInfo` (drop `Started`) or replace the type. |
| `Backend::find_client(name) -> Result<ClientProcess>` | `ClientReader.FindClient(name)` (ClientReader.cs:55) | `snapshot` (line 291, errors swallowed), `pictureCapture` (line 352) | return the client with that character logged in (OrdinalIgnoreCase); `UserError("ImagineClient is not running.")` / `UserError("No running client has '<name>' logged in.")`. Only the pid is needed: `ClientProcess::main_window_handle()` is already implemented here (.NET `MainWindowHandle` rule). |
| `Backend::take_snapshot(name, character_dir, csv) -> Result<SnapshotOutcome>` | `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)` (src\Build\Pipeline.cs:44) | `snapshot`, line 293 | read the game and append to `csv`; return `Snapshot.Result` fields `Source`, `StatsNote`, `Missing` (nullable), `Level` plus the `Ranks` int of the tuple. `character_dir` = `_chars.DataDir(e)`; the pipeline for that folder is created once and kept. |
| `Backend::rebuild(character_dir, csv) -> Result<i32>` | `Pipe(e).Rebuild(e.Csv).Snapshots` (Pipeline.cs:348) | `snapshot` line 294, `rebuild` line 302 | rebuild dash\*.json from the csv; return `Counts.Snapshots`. |
| `Backend::set_alignment(character_dir, csv, value) -> Result<AlignmentOutcome>` | `Pipe(e).SetAlignment(e.Csv, value)` (Pipeline.cs:78) | `setAlignment`, line 314 | `value` may be None. Return `Changed`, `Rows`, `ChangedRows`, `Backup` (full path or None; the app sends only the file name). `UserError("Alignment must be LAW, NEUTRAL or CHAOS.")` for anything else. |
| `Backend::comp_demons(name, character_dir, scan) -> Result<serde_json::Value>` | `Pipe(e).CompDemons(e.Name, scan)` (Pipeline.cs:200), serialized with `JsonNamingPolicy.CamelCase` | `compDemons`, line 326 | the JSON array of `CompDemon` records with camelCase property names, in C# property order, ints as ints. |
| `Backend::use_demon(name, character_dir, csv, key) -> Result<serde_json::Value>` | `Pipe(e).UseDemon(e.Name, e.Csv, key, scan: false)` (Pipeline.cs:305), camelCase | `useDemon`, line 335 | the `DemonSwitch` record as JSON object, camelCase. `key` may be None (then UserError "That demon isn't in the list. ..."). |
| `Backend::sim_command(&SimRequest) -> Result<String>` | `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)` (src\Build\PreviewServer.cs:35) | `simInit/simulate/simCatalog/simDemons/simBuilds/simSaveBuild/simDeleteBuild`, line 499 | return the JSON text for the page. `SimRequest` carries what the closures `SimPipe` (lines 487-494) and `SkillIcons` (line 498) used: `default_base` (= e.Name), `characters` (name, data folder) so `SimPipe(who)` can find `Pipe(_chars.Find(who))` or raise `UserError("There is no saved character called <who>.")`, and for `who == ""` the "_blank" pipeline `new Pipeline(<data>\characters\_blank, gameData, sets-from-first-character-with-a-set-table, new Store(active_dir))` (`blank_sets` = the first character's `Store.LoadSetTable()` with `Count > 0` whose `client-sets.json` exists); `skill_icons(who)` is already implemented by the app. |
| `Backend::item_icons_client_dir(store_dir) -> Result<Option<PathBuf>>` | `ItemIcons.ClientDir(new Store(dir))` (src\Build\ItemIcons.cs:135) | `assistant`, line 450 | the game client's folder as last seen (cache\client-dir.txt) or None. Stub: None. |
| `Backend::demon_model_load(data_root, client_dir, save) -> Result<Option<String>>` | `DemonModel.Load(_dataDir, client, DemonModel.Assistant, save)` (src\Build\DemonModel.cs:29) | `assistant`, line 458 | the model JSON text, or None for C#'s `null`. The app turns an `std::io::Error` into `"null"`; the library's NIF error type must be mapped the same way (C# catches `NifException`, `IOException`, `UnauthorizedAccessException`: add the NIF case in `main_form.rs` "assistant" or return `Ok(None)` for it). Stub: None. |

## Section 2: TEMPORARY stand-ins to replace by library types

| backend.rs | Library C# | Replace by |
|---|---|---|
| `CharacterEntry`, `Characters` (`new`, `find`, `find_mut`, `get_or_add`, `data_dir`, `save`, `all`) | src\Data\Characters.cs | `newmoon::data::characters`. The JSON file keeps the property names `Name`, `Dir`, `Csv`, `AutoPicture` (null written as `null`), 2-space indent. `main_form.rs` uses `e.dir` (Option), `e.csv` (Option) and `e.auto_picture` (Option<bool>): adapt the accessors. |
| `default_name()`, `default_csv()` | `Pipeline.DefaultName` / `Pipeline.DefaultCsv` (Pipeline.cs:14,16), `Config.Current` | `newmoon::data::config`. The stand-in does not write data\config.json when missing (C# `Config.Load` does). |
| `find_data_dir()`, `repo_root()`, `home_dir()`, `program_dir()` | `Pipeline.FindDataDir()` (Pipeline.cs:363), `Config.RepoRoot/HomeDir/ProgramDir` (Config.cs:30-50) | implemented here with the Rust repo marker `crates\newmoon\Cargo.toml` (and `NewMoonReader.csproj`); use the library's `Config` when it exists, keeping that marker. |
| `StoreLite` (`dash_dir`, `profile_path`, `expertise_ranks_path`, `alignment`) | `Store` (src\Data\Store.cs) | `newmoon::data::store::Store` (`DashDir`, `ProfilePath`, `ExpertiseRanksPath`, `Alignment()`). |

## Section 1: implemented in the app (nothing to wire)

`client_pids()` (`ClientPids`, MainForm.cs:564), `process_exe_dir(pid)` (`Process.MainModule.FileName`'s folder),
`main_window_handle(pid)` (`Process.MainWindowHandle`). Pictures (`character_picture.rs`), the page bridge
(`main_form.rs`), settings (`settings.rs`), the tours (`tour.rs`, `live_tour.rs`) are complete.
