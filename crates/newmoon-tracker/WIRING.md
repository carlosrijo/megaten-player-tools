# Wiring the tracker app to the library

Every call from the app into the reader library goes through `src/backend.rs`. `Backend` keeps what MainForm kept:
the shared `GameData` (`_gameData`, an `Arc<GameData>` made on first use: `Backend::game_data()`), and, when the Pipeline
is ported, `_pipes` (one `Pipeline` per character folder, made on first use) and `_blank`. Callers already serialize
reader work (`MainForm::guarded` / `calc` lock), so `Backend` methods may take `&self`.

## Wired (wave 3)

| backend.rs | Library | C# it stands for |
|---|---|---|
| `Characters`, `CharacterEntry` (re-export; `EntryExt::name()` for the always-set name) | `data::characters` | `Characters`, `CharacterEntry` (`new`/`get_or_add`/`save` now return `Result`: `MainForm::new` and the dispatch arms propagate it, as the C# threw) |
| `Store` (re-export) | `data::store::Store` | `Store` (`load_data` uses `dash_dir`, `profile_path`, `expertise_ranks_path`, `alignment()?`) |
| `find_data_dir()`, `default_name()`, `default_csv()` | `data::config::{home_dir, current}` | `Pipeline.FindDataDir/DefaultName/DefaultCsv`. `// WAVE4: use build::pipeline::{find_data_dir, default_name, default_csv}` |
| `site.rs`: `repo_root()`, `program_dir()` | `data::config::{repo_root, program_dir}` | `Config.RepoRoot/ProgramDir` |
| `Backend::list_clients()` | `game::client_reader::list_clients` | `ClientReader.ListClients()` (`ClientInfo` is the library's, with the unused `started`) |
| `Backend::find_client(name)` | `game::client_reader::find_client` | `ClientReader.FindClient(name)`; `ClientProcess::main_window_handle()` is the library's (`snapshot` picture, `pictureCapture`) |
| `client_pids()`, `process_exe_dir(pid)` | `client_reader::{client_processes, process_exe_path}` | `MainForm.ClientPids()`, `Process.MainModule.FileName`'s folder |
| `Backend::item_icons_client_dir(dir)` | `build::item_icons::client_dir(&Store, 0)` | `ItemIcons.ClientDir(new Store(dir))` |
| `Backend::demon_model_load(root, client, save)` | `build::demon_model::load(.., ASSISTANT, save)` | `DemonModel.Load(...)`; the "assistant" command turns `io::Error` and `data::nif::is_nif_error` into `"null"` (C# `catch NifException, IOException, UnauthorizedAccessException`) |
| `Backend::game_data()` | `data::game_data::GameData::new()` in an `Arc` | `_gameData ??= new GameData()` (unused until the Pipeline is wired) |

## Still stubs (`// WAVE4:` in backend.rs): need Pipeline / PreviewServer

All return `Err("not ported yet: ...")` for now.

| backend.rs function | C# it stands for | Called from (app\MainForm.cs) | Real implementation must |
|---|---|---|---|
| `Backend::take_snapshot(name, character_dir, csv) -> Result<SnapshotOutcome>` | `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)` (src\Build\Pipeline.cs:44) | `snapshot`, line 293 | read the game and append to `csv`; return `Snapshot.Result` fields `Source`, `StatsNote`, `Missing` (nullable), `Level` plus the `Ranks` int of the tuple. `character_dir` = `_chars.DataDir(e)`; the pipeline for that folder is created once and kept (with `self.game_data()`). |
| `Backend::rebuild(character_dir, csv) -> Result<i32>` | `Pipe(e).Rebuild(e.Csv).Snapshots` (Pipeline.cs:348) | `snapshot` line 294, `rebuild` line 302 | rebuild dash\*.json from the csv; return `Counts.Snapshots`. |
| `Backend::set_alignment(character_dir, csv, value) -> Result<AlignmentOutcome>` | `Pipe(e).SetAlignment(e.Csv, value)` (Pipeline.cs:78) | `setAlignment`, line 314 | `value` may be None. Return `Changed`, `Rows`, `ChangedRows`, `Backup` (full path or None; the app sends only the file name). `UserError("Alignment must be LAW, NEUTRAL or CHAOS.")` for anything else. |
| `Backend::comp_demons(name, character_dir, scan) -> Result<serde_json::Value>` | `Pipe(e).CompDemons(e.Name, scan)` (Pipeline.cs:200), serialized with `JsonNamingPolicy.CamelCase` | `compDemons`, line 326 | the JSON array of `CompDemon` records with camelCase property names, in C# property order, ints as ints. |
| `Backend::use_demon(name, character_dir, csv, key) -> Result<serde_json::Value>` | `Pipe(e).UseDemon(e.Name, e.Csv, key, scan: false)` (Pipeline.cs:305), camelCase | `useDemon`, line 335 | the `DemonSwitch` record as JSON object, camelCase. `key` may be None (then UserError "That demon isn't in the list. ..."). |
| `Backend::sim_command(&SimRequest) -> Result<String>` | `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)` (src\Build\PreviewServer.cs:35) | `simInit/simulate/simCatalog/simDemons/simBuilds/simSaveBuild/simDeleteBuild`, line 499 | return the JSON text for the page. `SimRequest` carries what the closures `SimPipe` (lines 487-494) and `SkillIcons` (line 498) used: `default_base` (= e.Name), `characters` (name, data folder) so `SimPipe(who)` can find `Pipe(_chars.Find(who))` or raise `UserError("There is no saved character called <who>.")`, and for `who == ""` the "_blank" pipeline `new Pipeline(<data>\characters\_blank, gameData, sets-from-first-character-with-a-set-table, new Store(active_dir))` (`blank_sets` = the first character's `Store::load_set_table()` with `Count > 0` whose `client-sets.json` exists); `skill_icons(who)` is already implemented by the app. |

## Implemented in the app (nothing to wire)

Pictures (`character_picture.rs`), the page bridge (`main_form.rs`), settings (`settings.rs`), the tours (`tour.rs`,
`live_tour.rs`) are complete.
