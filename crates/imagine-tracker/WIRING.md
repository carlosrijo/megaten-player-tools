# Wiring the tracker app to the library

Every call from the app into the reader library goes through `src/backend.rs`. `Backend` keeps what MainForm kept:
the shared `GameData` (`_gameData`, an `Arc<GameData>` made on first use: `Backend::game_data()`), `_pipes` (one
`Pipeline` per character folder, made on first use and kept for the life of the app, all sharing that `GameData`) and
`_blank` (the Simulator's "no base" pipeline). Callers already serialize reader work (`MainForm::guarded` / `calc`
lock), so `Backend` methods take `&self`.

## Wired (wave 3)

| backend.rs | Library | C# it stands for |
|---|---|---|
| `Characters`, `CharacterEntry` (re-export; `EntryExt::name()` for the always-set name) | `data::characters` | `Characters`, `CharacterEntry` (`new`/`get_or_add`/`save` now return `Result`: `MainForm::new` and the dispatch arms propagate it, as the C# threw) |
| `Store` (re-export) | `data::store::Store` | `Store` (`load_data` uses `dash_dir`, `profile_path`, `expertise_ranks_path`, `alignment()?`) |
| `find_data_dir()`, `default_name()`, `default_csv()` | `build::pipeline::{find_data_dir, default_name, default_csv}` | `Pipeline.FindDataDir/DefaultName/DefaultCsv` (wave 4). `default_csv` returns a `PathBuf`: the library's `Result` error (no statsFolder, which the C# turns into a crash) becomes an empty path |
| `site.rs`: `repo_root()`, `program_dir()` | `data::config::{repo_root, program_dir}` | `Config.RepoRoot/ProgramDir` |
| `Backend::list_clients()` | `game::client_reader::list_clients` | `ClientReader.ListClients()` (`ClientInfo` is the library's, with the unused `started`) |
| `Backend::find_client(name)` | `game::client_reader::find_client` | `ClientReader.FindClient(name)`; `ClientProcess::main_window_handle()` is the library's (`snapshot` picture, `pictureCapture`) |
| `client_pids()`, `process_exe_dir(pid)` | `client_reader::{client_processes, process_exe_path}` | `MainForm.ClientPids()`, `Process.MainModule.FileName`'s folder |
| `Backend::item_icons_client_dir(dir)` | `build::item_icons::client_dir(&Store, 0)` | `ItemIcons.ClientDir(new Store(dir))` |
| `Backend::demon_model_load(root, client, save)` | `build::demon_model::load(.., ASSISTANT, save)` | `DemonModel.Load(...)`; the "assistant" command turns `io::Error` and `data::nif::is_nif_error` into `"null"` (C# `catch NifException, IOException, UnauthorizedAccessException`) |
| `Backend::game_data()` | `data::game_data::GameData::new()` in an `Arc` | `_gameData ??= new GameData()` (shared by every `Pipeline`) |


## Wired (wave 4): the Pipeline commands

`Backend::pipe(character_dir)` is `MainForm.Pipe(e)`: `Pipeline::new(dir, Some(self.game_data()), None, None)`, kept in a map
keyed by the folder (C# keys by character name; a folder belongs to one name), so the `Pipeline` and its caches live as long
as the app. The calls below are what `MainForm::dispatch` (`snapshot`, `rebuild`, `setAlignment`, `compDemons`, `useDemon`) makes;
the page's JSON shapes are built in `main_form.rs` (`snapshot`, `rebuild`, `setAlignment`) and `backend.rs` (`compDemons`, `useDemon`).

| backend.rs | Library | C# it stands for / JSON sent |
|---|---|---|
| `take_snapshot(name, dir, csv)` | `Pipeline::take_snapshot(name, csv, false)` -> `(SnapshotResult, ranks)` | `Pipe(e).TakeSnapshot(e.Name, e.Csv, false)`; the `snapshot` command then calls `rebuild` and sends `{source, statsNote, missing, level, ranks, picture}` |
| `rebuild(dir, csv)` | `Pipeline::rebuild(csv)?.snapshots` | `Pipe(e).Rebuild(e.Csv).Snapshots`; `rebuild` sends `{snapshots}` |
| `set_alignment(dir, csv, value)` | `Pipeline::set_alignment(csv, value.unwrap_or(""))` | `Pipe(e).SetAlignment(e.Csv, value)`; sends `{changed, rows, changedRows, backup}` (file name only) |
| `comp_demons(name, dir, scan)` | `Pipeline::comp_demons` + `CompDemon::to_value` | `Pipe(e).CompDemons(e.Name, scan)` as camelCase JSON array |
| `use_demon(name, dir, csv, key)` | `Pipeline::use_demon(.., scan: false)` | `Pipe(e).UseDemon(..)` as `{name, changed, rowUpdated, backup}` |

## Simulator (wired)

| backend.rs | Stands for |
|---|---|
| `Backend::sim_pipe(&SimRequest, who) -> Result<Arc<Pipeline>>` | `SimPipe(who)` of `MainForm.Dispatch`: `who != ""` -> `Pipe(_chars.Find(who))` or `UserError("There is no saved character called <who>.")`; `who == ""` -> the lazily made `_blank` pipeline: `Pipeline::new(<data>\characters\_blank, game_data, Some(blank_sets), Some(Store::new(active_dir)))`. `blank_sets` = the first saved character's set table (`client-sets.json` exists, `load_set_table` non-empty), reading the character folders of the latest request (C# reads `_chars.All` live) |
| `SimRequest.owner`, `.skill_icons`, `.builds_path`, `.default_base` | computed in `main_form.rs`'s `simInit/simulate/...` arm exactly as the C# does (`owner` = the character whose `Dir` is `.`/empty/null, else the active one; `skill_icons(who)` = `folder_url(entry, dash\skill-icons)`); the arm already holds the `calc` lock and refuses `simSaveBuild`/`simDeleteBuild` under NoWrites |
| `Backend::sim_command(&SimRequest) -> Result<String>` | `PreviewServer.SimCommand(SimPipe, e.Name, <data>\sim-builds.json, cmd, args, owner, SkillIcons)` (src\Build\PreviewServer.cs:35). Calls `imagine::build::preview_server::sim_command(&|who| self.sim_pipe(req, who), default_base, builds_path, cmd, Some(args), Some(owner), Some(skill_icons))`; its None (not a Simulator command) is posted as JSON `null` |who| self.sim_pipe(req, who), req.default_base, &req.builds_path, req.cmd, req.args, Some(req.owner), Some(req.skill_icons))` (adapt to the real signature) |

## Implemented in the app (nothing to wire)

Pictures (`character_picture.rs`), the page bridge (`main_form.rs`), settings (`settings.rs`), the tours (`tour.rs`,
`live_tour.rs`) are complete.

## Troubleshooting commands (support.rs, 2026-11-24)

New page commands, not in the C#. The work is in the library (`imagine::build::doctor`, `build::bundle`, `imagine::diag`);
`support.rs` is the app's side (WebView2 version, the settings file path, the page-error limit). See HANDOVER.md section 21.

| Page command | Does | Notes |
|---|---|---|
| `about` | `{ version, commit, built, windows, webview2, text, dataDir, logDir, logLevel }`; `text` is what "Copy" copies | no writes |
| `doctor` | `doctor::run(..).to_json()`: the self-check, one pass | inside `guarded` (one piece of reader work at a time); the data-folder write test is skipped in a capture run on real data |
| `saveBundle` `{ includeNames }` | save dialog (default `imagine-support-<yyyyMMdd-HHmm>.zip` on the Desktop), fresh `doctor`, `bundle::write`; `{ path, zipBytes, files, includeNames }`, `null` when cancelled | refused in a capture run without `--data` (NoWrites); a capture run with `--data` writes `support-bundle.zip` beside its pictures instead of asking |
| `buildCheck` | `{ clients, known, unknown: [{ folder, pid, fingerprint, same }] }`: each running client's exe against `doctor::KNOWN_CLIENTS` (size + SHA-256, cached) | the page warns once per fingerprint |
| `log` `{ kind, message, source, stack }` | the page's JavaScript errors into the log (`PageLog`: 20 a minute, 200 a run, texts cut) | does nothing under NoWrites; always answers `true`; the browser stand-in answers `true` too |
| `reveal` `{ what: "logs" \| "bundle" }` | opens the log folder / selects the bundle this run saved | never a path from the page |
| `debugPanic` | panics on purpose | only in debug builds and with `--features crash-test` |

Every command's duration and outcome (not its arguments) goes into the log (`log_command` in `main_form.rs`).
