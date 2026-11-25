//! imagine-reader command line (port of src\Program.cs).
//!
//! imagine-reader: read-only stat/gear snapshots of an IMAGINE character, plus the data files for the build-tracker page.
//!
//!   clients                                                     list the running clients and who is logged in to each
//!   snapshot  [--name N] [--csv path] [--ignore-window]         read the game, append a CSV row, rebuild the page data
//!   rebuild   [--name N] [--csv path]                           rebuild the page data from saved files (no game needed)
//!   recalc    [--name N] [--csv path]                           recompute every CSV row's gear bonuses, then rebuild
//!   gear-names [--name N] [--csv path]                          name every CSV row's gear the game's way (backup first), then rebuild
//!   sets      [--name N]                                        show the sets/appearance bonuses for the worn gear
//!   simulate  [--spec build.json | --init | --catalog N]        the Build Planner tool: check it, or work out another build (nothing is written)
//!   serve     [--port N] [--root dir] [--also "Name=folder"]    the app's page in a browser with a working Build Planner (testing)
//!   alignment [LAW|NEUTRAL|CHAOS] [--name N]                    show or set the character's alignment (recalculates when it changes)
//!   extras    [--quests N|none] [--compendium N|none] [--compendium-v2 yes|no] [--name N]
//!                                                               show the clan; set the quest/compendium counts for their bonuses
//!
//! Without --name, the character is read from the running client (or is the only one with saved data).
//! Each character keeps its own folder and CSV, listed in data\characters.json.
//! --data <dir> uses that folder as-is instead (for tests on copies): its config.json and characters.json decide the CSV, never the program's own.
//! The desktop app (app\) runs the same steps through Build\Pipeline.cs.
//!
//! Commands ported: everything except `simulate` and `serve` (wave 4: they need the Simulator and the PreviewServer).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use imagine::build::pipeline::{self, Pipeline, default_csv, default_name, find_data_dir};
use imagine::build::snapshot::SnapshotResult;
use imagine::build::{demon_db, demon_model, digi_planner, doctor, dungeons, gear_db, item_icons, preview_server, skill_db};
use imagine::calc::character_extras;
use imagine::calc::character_hp_mp::{self, Calibration};
use imagine::calc::digitalize::{Digitalize, MITAMA_NAMES, RUNE_NAMES, set_name};
use imagine::calc::effect::Effect;
use imagine::calc::item_effects::ItemEffects;
use imagine::console;
use imagine::diag::{about, crash, log};
use imagine::data::characters::{Characters, get_directory_name, path_combine, safe_file_name};
use imagine::data::config;
use imagine::data::demon_tables::Assist;
use imagine::data::game_data::GameData;
use imagine::data::json::{self, At};
use imagine::data::nif::{NifBlock, NifError, NifFile, NifKind, NifSequence};
use imagine::data::stat_labels;
use imagine::data::store::Store;
use imagine::fmt;
use imagine::game::progress_reader::{self, ProgressState};
use imagine::game::{DemonRecord, client_reader, demon_reader};
use imagine::memory::GameMemory;
use imagine::text::{cmp_invariant, eq_ic};
use imagine::{Result, UserError, user_error};
use serde_json::Value;

/// What a .NET process returns when an exception nobody caught ends it (0xE0434352).
const UNHANDLED_EXIT: i32 = 0xE043_4352_u32 as i32;

fn main() {
    // The C# command line never registered the code pages, so a model's Shift-JIS node names stay as they were read.
    demon_model::set_shift_jis_names(false);
    about::init(about::BuildInfo {
        program: "imagine-reader".to_string(),
        version: env!("IMAGINE_BUILD_VERSION").to_string(),
        commit: env!("IMAGINE_BUILD_COMMIT").to_string(),
        built: env!("IMAGINE_BUILD_DATE").to_string(),
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `imagine-reader --version`: which build is this (the line a bug report starts with)
    if matches!(args.first().map(String::as_str), Some("--version" | "-V" | "version")) {
        console::write_line(&about::build_info().line());
        std::process::exit(0);
    }
    // a crash writes logs\crash-<time>.txt once logging has a folder (commands that touch a character's data); the usual message still prints
    crash::install(None);
    let opts = parse_args(&args);
    let started = std::time::Instant::now();
    let code = match run(&opts) {
        Ok(code) => {
            log::write(log::Level::Info, &format!("finished: exit {code} after {} ms", started.elapsed().as_millis()));
            code
        }
        Err(e) => match e.downcast_ref::<UserError>() {
            Some(u) => {
                log::write(log::Level::Info, &format!("stopped with a message for the person: {}", u.0));
                console::error_line(&u.0);
                1
            }
            None => {
                // not a UserError: the C# let it end the process
                log::write(log::Level::Error, &format!("failed: {e:#}"));
                console::error_line(&format!("Unhandled exception. {e:#}"));
                UNHANDLED_EXIT
            }
        },
    };
    std::process::exit(code);
}

fn run(opts: &HashMap<String, String>) -> Result<i32> {
    let cmd = opts.get("_cmd").map(String::as_str).unwrap_or("help");
    let get = |k: &str| opts.get(k).map(String::as_str);
    let get_or = |k: &str, d: String| opts.get(k).cloned().unwrap_or(d);

    // --data <folder>: that folder is the data folder for the whole run. Its config.json (stats folder, paths) replaces the
    // program's own, before anything reads the settings, so no read or write falls back to the program's own data.
    if let Some(d) = opts.get("data") {
        if !config::use_data_dir(Path::new(d)) {
            return Err(user_error("--data could not be applied: the settings were already loaded. Nothing was read or written."));
        }
    }

    // The local log (<data folder>\logs): for the commands that touch a character's data or the game, not for the database builders.
    // --log-level debug|info|warn (default info).
    let level = match opts.get("log-level") {
        None => log::Level::Info,
        Some(v) => match log::Level::parse(v) {
            Some(l) if l != log::Level::Error => l,
            _ => return Err(user_error(format!("--log-level must be debug, info or warn, not '{v}'."))),
        },
    };
    if matches!(cmd, "snapshot" | "rebuild" | "recalc" | "gear-names" | "sets" | "alignment" | "demon" | "expertise" | "toggles" | "extras" | "simulate" | "serve" | "clients" | "digilevels" | "doctor" | "crash-test") {
        let root = opts.get("data").map(PathBuf::from).unwrap_or_else(find_data_dir);
        let logs = root.join("logs");
        if log::init(&logs, level, &format!("{}, command '{cmd}'", about::build_info().line())) {
            crash::set_dir(&logs);
            log::write(log::Level::Info, &format!("Windows: {}", about::windows_info().text()));
        }
    }

    if cmd == "doctor" {
        // The self-check (the same as Settings > Check my setup in the app): one pass over the files, the game and each reader.
        //   doctor [--data <folder>] [--json]
        let data_dir = opts.get("data").map(PathBuf::from).unwrap_or_else(find_data_dir);
        let report = doctor::run(&doctor::Input { data_dir, webview2: None, settings_path: None, probe_write: true, game: true });
        if opts.contains_key("json") {
            console::write_line(&serde_json::to_string_pretty(&report.to_json())?);
        } else {
            console::write(&report.to_text());
        }
        return Ok(if report.worst() == doctor::Status::Fail { 1 } else { 0 });
    }

    if cmd == "symbolize" {
        // Read a crash report offline: turn its "BABEL.exe+0xOFFSET" frames into functions and lines with the .pdb of the
        // build that crashed (tools\publish-portable.ps1 writes it to dist-symbols\; the release workflow keeps it).
        //   symbolize <crash-....txt> --exe <the exe of that build> [--pdb <folder with its .pdb>]
        let usage = "symbolize needs a crash report and the exe that made it: symbolize <crash-....txt> --exe <BABEL.exe> [--pdb <folder with the .pdb>].";
        let file = opts.get("_arg").ok_or_else(|| user_error(usage))?;
        let exe = get("exe").ok_or_else(|| user_error(usage))?;
        let report = std::fs::read_to_string(file).map_err(|e| user_error(format!("{file} can't be read: {e}")))?;
        let out = imagine::diag::symbolize::symbolize(&report, Path::new(exe), get("pdb").map(Path::new)).map_err(user_error)?;
        console::write(&out);
        return Ok(0);
    }

    #[cfg(feature = "crash-test")]
    if cmd == "crash-test" {
        // Test builds only: panic on purpose, to look at the crash report (logs\crash-*.txt) and its stack trace.
        fn deepest(n: u32) -> u32 {
            if n == 0 { panic!("deliberate test panic (crash-test)") } else { deepest(n - 1) + 1 }
        }
        deepest(3);
    }

    if cmd == "geardb" {
        // The Gear Database page's data, from the client's decoded tables and the community sheets (no game, no character).
        //   geardb --decoded <dir with ItemData.xml, SItemData.xml, EquipmentSetData.xml, CItemData.xml> [--sheets sheet_gear.tsv] [--client <game folder, for icons>] [--out app\ui\gear-db.json]
        let decoded = get("decoded").ok_or_else(|| user_error("geardb needs --decoded <folder with the decoded client tables>."))?;
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "gear-db.json"));
        // --ids-out: the ids for tools\dcmexport --ids; --game-sources: its output
        let client = client_arg(get("client"))?;
        gear_db::build(Arc::new(GameData::new()), decoded, get("sheets"), &out, get("ids-out"), get("game-sources"), client.as_deref())?;
        return Ok(0);
    }
    if cmd == "dungeons" {
        // The Dungeon Explorer page's data: every dungeon and its enemies that can't be contracted (no game, no character).
        //   dungeons --decoded <dir with zone.xml, DevilData.xml, citem.tsv> [--datastore <server datastore>] [--client <game folder, for icons>] [--out app\ui\dungeons.json]
        let decoded = get("decoded").ok_or_else(|| user_error("dungeons needs --decoded <folder with the decoded client tables>."))?;
        let datastore = match get("datastore") {
            Some(d) => d.to_string(),
            // beside the server's config folder
            None => {
                let constants = config::current().server_constants.clone().ok_or_else(null_path)?;
                let up = get_directory_name(&constants).and_then(|d| get_directory_name(&d)).ok_or_else(null_path)?;
                path_combine(&up, "datastore")
            }
        };
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "dungeons.json"));
        dungeons::build(&GameData::new(), Path::new(decoded), Path::new(&datastore), Path::new(&out), get("client").map(Path::new))?;
        return Ok(0);
    }
    if cmd == "demondb" {
        // The Demon Database page's data, from the client's decoded tables and the server's zone files (no game, no character).
        //   demondb --decoded <dir with DevilData.xml, ZoneData.xml> --store <server datastore> [--client <game folder>] [--out app\ui\demon-db.json]
        let decoded = get("decoded").ok_or_else(|| user_error("demondb needs --decoded <folder with DevilData.xml and ZoneData.xml>."))?;
        let store_dir = get("store").ok_or_else(|| user_error("demondb needs --store <the server's datastore folder>."))?;
        let client = client_arg(get("client"))?;
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "demon-db.json"));
        demon_db::build(&GameData::new(), Path::new(decoded), Path::new(store_dir), client.as_deref(), Path::new(&out))?;
        return Ok(0);
    }
    if cmd == "digiplanner" {
        // The Digitalize Planner page's data, from the reference tables (no game, no character).
        //   digiplanner [--out app\ui\digi-planner.json]
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "digi-planner.json"));
        digi_planner::build(&GameData::new(), Path::new(&out))?;
        return Ok(0);
    }
    if cmd == "skilldb" {
        // The Skill Database page's data: every skill grouped by action type, from the client's decoded tables (no game, no character).
        //   skilldb --decoded <dir with SkillData.xml, CHelpData.xml> [--client <game folder, for icons>] [--out app\ui\skill-db.json]
        let decoded = get("decoded").ok_or_else(|| user_error("skilldb needs --decoded <folder with SkillData.xml and CHelpData.xml>."))?;
        let client = client_arg(get("client"))?;
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "skill-db.json"));
        skill_db::build(&GameData::new(), Path::new(decoded), client.as_deref(), Path::new(&out))?;
        return Ok(0);
    }
    if cmd == "page-icons" {
        // reference\page-icons.tsv: which client picture each Tools page picture (app\ui\*-icons) is, by pixels, so a
        // player's app can make them from their own game folder (no game, no character).
        //   page-icons [--client <game folder>] [--ui app\ui] [--out reference\page-icons.tsv]
        let client = client_arg(get("client"))?.ok_or_else(|| user_error("page-icons needs --client <game folder> (or a saved one in data\\cache\\client-dir.txt)."))?;
        let ui = get_or("ui", path_combine("app", "ui"));
        let out = get_or("out", path_combine("reference", imagine::build::page_icons::LIST));
        let r = imagine::build::page_icons::build_list(Path::new(&ui), &client, Path::new(&out))?;
        println!("{} pictures listed in {out}; {} without a matching client picture", r.listed, r.unmatched.len());
        for u in r.unmatched.iter().take(30) {
            println!("  {u}");
        }
        return Ok(0);
    }
    if cmd == "model" && opts.contains_key("check") {
        return model_check(opts);
    }
    if cmd == "model" {
        // model [name] [--client <game folder>] [--out file]: the assistant's 3D model (Black Frost unless named, e.g. D2_04_009a)
        // from the client's Devil\bone files into <data>\cache\models\<name>.json, with a summary of what was read.
        let root = get_or("data", find_data_dir().to_string_lossy().into_owned());
        let model = get_or("_arg", demon_model::ASSISTANT.to_string());
        let client = match get("client") {
            Some(c) => PathBuf::from(c),
            None => item_icons::client_dir(&Store::new(&root), 0)
                .ok_or_else(|| user_error("The game's folder isn't known yet: start the game once, or pass --client <folder>."))?,
        };
        let mut say = |s: &str| console::write_line(s);
        let json = demon_model::export(&client, &model, Some(&mut say))?
            .ok_or_else(|| user_error(format!("{} has no model named {model} (Devil\\bone\\{model}_Base.nif).", client.display())))?;
        let to = get_or("out", demon_model::cache_path(Path::new(&root), &model).to_string_lossy().into_owned());
        if let Some(dir) = std::path::absolute(&to)?.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&to, &json)?;
        console::write_line(&format!("Wrote {to} ({} KB)", json.encode_utf16().count() / 1024));
        return Ok(0);
    }
    if cmd == "clients" {
        // Who is logged in where (read-only). The name is read from each client, nothing needs to be typed.
        let list = client_reader::list_clients();
        if list.is_empty() {
            return Err(user_error("ImagineClient is not running."));
        }
        for c in list {
            let who = match &c.character {
                None => "nobody logged in".to_string(),
                Some(n) => format!("{n}  level {}", c.level),
            };
            console::write_line(&format!("pid {:<6} {:<14} {who}", c.pid, c.folder));
        }
        return Ok(0);
    }
    // The character a command works on, with its pipeline (the C# `name`, `csv` and `pipe`).
    let mut chosen: Option<(Target, Pipeline)> = None;
    if matches!(cmd, "snapshot" | "rebuild" | "recalc" | "gear-names" | "sets" | "alignment" | "demon" | "expertise" | "toggles" | "extras" | "simulate" | "serve") {
        // Which character: --name, else the one logged in (snapshot, sets) or the only one with saved data (rebuild, recalc).
        // Each character has its own folder and CSV (data\characters.json). --data / --csv point somewhere else instead,
        // which is how tests run on copies.
        let target = resolve_character(cmd, opts)?;
        let pipe = Pipeline::new(target.data_dir.clone(), None, None, None);
        chosen = Some((target, pipe));
    }

    // the switch of Program.cs
    match cmd {
        "snapshot" => {
            let (t, pipe) = need(&chosen)?;
            let (r, ranks) = pipe.take_snapshot(&t.name, &t.csv, opts.contains_key("ignore-window"))?;
            print_snapshot(&t.name, &r);
            console::write_line(&format!("Expertise: {ranks} ranked (incl. computed chains)"));
            rebuild(t, pipe)?;
            Ok(0)
        }
        "toggles" => {
            let (t, pipe) = need(&chosen)?;
            toggles(opts, t, pipe)
        }
        "expertise" => {
            // Re-read expertise ranks and Digitalization Levels from the game and rebuild the page data (no CSV row)
            let (t, pipe) = need(&chosen)?;
            let ranks = pipe.exp().read(&t.name)?;
            console::write_line(&format!("Expertise: {} ranked (incl. computed chains)", ranks.len()));
            rebuild(t, pipe)?;
            Ok(0)
        }
        "rebuild" => {
            let (t, pipe) = need(&chosen)?;
            rebuild(t, pipe)?;
            Ok(0)
        }
        "simulate" => {
            let (_, pipe) = need(&chosen)?;
            simulate(opts, pipe)
        }
        "serve" => {
            let (t, pipe) = need(&chosen)?;
            serve(opts, t, pipe)
        }
        "gear-names" => {
            // Rename the gear in every CSV row to the game's item names (old rows hold the website's); numbers stay.
            let (t, pipe) = need(&chosen)?;
            let (log, backup) = pipe.rename_gear(&t.csv)?;
            for line in &log {
                console::write_line(line);
            }
            match backup {
                Some(b) => console::write_line(&format!("{} rows renamed. The CSV before: {}", log.len(), b.display())),
                None => console::write_line("Every row already names its gear the game's way: nothing changed."),
            }
            rebuild(t, pipe)?;
            Ok(0)
        }
        "recalc" => {
            let (t, pipe) = need(&chosen)?;
            for line in pipe.recalc(&t.csv)? {
                console::write_line(&line);
            }
            rebuild(t, pipe)?;
            Ok(0)
        }
        "extras" => {
            let (t, pipe) = need(&chosen)?;
            extras(opts, t, pipe)
        }
        "alignment" => {
            let (t, pipe) = need(&chosen)?;
            alignment(opts, t, pipe)
        }
        "sets" => {
            let (t, pipe) = need(&chosen)?;
            sets(t, pipe)
        }
        "demon" => {
            let (t, pipe) = need(&chosen)?;
            demon(opts, t, pipe)
        }
        "labels" => {
            // imagine-reader labels: writes reference\stat-labels-game.tsv, the game's own name for each stat id
            // (CorrectTbl / aspect), taken from the client's tokusei text (no game or character needed).
            let gd = GameData::new();
            let (rows, path) = stat_labels::generate(&gd)?;
            console::write_line(&format!("{rows} stat labels written to {}", path.display()));
            Ok(0)
        }
        "digilevels" => digilevels(opts),
        _ => {
            console::write_line("usage: imagine-reader clients | snapshot|rebuild|recalc|sets [--name N] [--csv path] [--data dir] [--ignore-window]");
            Ok(if cmd == "help" { 0 } else { 2 })
        }
    }
}

fn need(chosen: &Option<(Target, Pipeline)>) -> Result<(&Target, &Pipeline)> {
    chosen.as_ref().map(|(t, p)| (t, p)).ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))
}

/// The Build Planner tool from the command line (no game, nothing written):
///   simulate                      check: the build as read must give back the saved page data and the readings
///   simulate --spec build.json    the result of that build (the page's format, Simulator.Parse), as JSON
///   simulate --catalog N          what can go in slot N (0 Weapon ... 14 Extra), as JSON
///   simulate --init               the build as read, the expertise list and its result, as JSON (what the page starts from)
fn simulate(opts: &HashMap<String, String>, pipe: &Pipeline) -> Result<i32> {
    if let Some(spec_file) = opts.get("spec") {
        let text = json::read_all_text(Path::new(spec_file))?;
        let spec = json::parse(&text)?;
        console::write_line(&pipe.sim.run(Some(&spec))?);
        return Ok(0);
    }
    if opts.contains_key("init") {
        console::write_line(&pipe.sim.init(None)?);
        return Ok(0);
    }
    if let Some(slot_text) = opts.get("catalog") {
        let slot = fmt::parse_i32(slot_text).ok_or_else(|| anyhow::anyhow!("The input string '{slot_text}' was not in a correct format."))?;
        console::write_line(&pipe.sim.catalog(slot, None)?);
        return Ok(0);
    }
    let watch = std::time::Instant::now();
    let diffs = pipe.sim.self_check()?;
    if diffs.is_empty() {
        console::write_line(&format!("Build Planner: the build as read gives back the readings and the saved bonus list ({} ms).", watch.elapsed().as_millis()));
    } else {
        console::write_line(&format!("Build Planner differs from the saved data:\n  {}", diffs.join("\n  ")));
    }
    let watch = std::time::Instant::now();
    pipe.sim.init(None)?;
    console::write_line(&format!("A second run: {} ms.", watch.elapsed().as_millis()));
    Ok(if diffs.is_empty() { 0 } else { 1 })
}

/// The page in a plain browser with a working Build Planner: PreviewServer (for testing; see its notes).
/// --also "Name=folder": a second character to base builds on (for testing; the app has every saved character).
fn serve(opts: &HashMap<String, String>, t: &Target, pipe: &Pipeline) -> Result<i32> {
    let data = Arc::clone(pipe.data());
    let dir = pipe.store().dir.to_string_lossy().into_owned();
    // a build with no base: no character behind it; the set table comes from this character's saved snapshot
    let sets_store = Store::new(&dir);
    let blank_sets: pipeline::BlankSets = Box::new(move || if sets_store.set_table_path().exists() { sets_store.load_set_table() } else { Ok(None) });
    let blank = Arc::new(Pipeline::new(path_combine(&path_combine(&dir, "characters"), "_blank"), Some(Arc::clone(&data)), Some(blank_sets), Some(Store::new(&dir))));
    let mut also: Option<(String, Arc<Pipeline>)> = None;
    if let Some(a) = opts.get("also") {
        if let Some(eq) = a.find('=') {
            also = Some((a[..eq].to_string(), Arc::new(Pipeline::new(a[eq + 1..].to_string(), Some(Arc::clone(&data)), None, None))));
        }
    }
    let main_pipe = Arc::new(Pipeline::new(t.data_dir.clone(), Some(Arc::clone(&data)), None, None));
    let pipe_of = |who: &str| -> Result<Arc<Pipeline>> {
        if who.is_empty() {
            Ok(Arc::clone(&blank))
        } else if eq_ic(who, &t.name) {
            Ok(Arc::clone(&main_pipe))
        } else if let Some((n, p)) = also.as_ref().filter(|(n, _)| eq_ic(who, n)) {
            let _ = n;
            Ok(Arc::clone(p))
        } else {
            Err(user_error(format!("This preview has no saved character called {who}.")))
        }
    };
    let root = match opts.get("root") {
        Some(r) => r.clone(),
        None => get_directory_name(&find_data_dir().to_string_lossy()).ok_or_else(null_path)?,
    };
    let port_text = opts.get("port").map(String::as_str).unwrap_or("18770");
    let port = fmt::parse_i32(port_text).ok_or_else(|| anyhow::anyhow!("The input string '{port_text}' was not in a correct format."))?;
    let port = u16::try_from(port).map_err(|_| anyhow::anyhow!("Value was either too large or too small for a port."))?;
    preview_server::run(&pipe_of, &t.name, Path::new(&path_combine(&dir, "sim-builds.json")), &root, port, &dir)?;
    Ok(0)
}

/// `Path.Combine(null, ...)` / `Path.GetDirectoryName(null)` on a setting data\config.json does not have.
fn null_path() -> anyhow::Error {
    anyhow::anyhow!("Value cannot be null. (Parameter 'path1')")
}

/// `--client`, else the folder saved in data\cache\client-dir.txt (relative to the current folder), as the C# did.
fn client_arg(client: Option<&str>) -> Result<Option<PathBuf>> {
    if let Some(c) = client {
        return Ok(Some(PathBuf::from(c)));
    }
    let saved = Path::new("data").join("cache").join("client-dir.txt");
    if saved.is_file() {
        let text = imagine::data::json::read_all_text(&saved)?;
        return Ok(Some(PathBuf::from(text.trim())));
    }
    Ok(None)
}

/// C# `ParseArgs`: the first word is the command (`_cmd`), the second its argument (`_arg`); `--key value`, or a flag
/// (`"true"`) when the next word is another `--` option or there is none.
fn parse_args(a: &[String]) -> HashMap<String, String> {
    let mut d = HashMap::new();
    let mut i = 0;
    while i < a.len() {
        if let Some(k) = a[i].strip_prefix("--") {
            if i + 1 < a.len() && !a[i + 1].starts_with("--") {
                i += 1;
                d.insert(k.to_string(), a[i].clone());
            } else {
                d.insert(k.to_string(), "true".to_string());
            }
        } else if !d.contains_key("_cmd") {
            d.insert("_cmd".to_string(), a[i].clone());
        } else if !d.contains_key("_arg") {
            d.insert("_arg".to_string(), a[i].clone());
        }
        i += 1;
    }
    d
}

/// The character a command works on (the C# `name` and `csv`, and the folder its Pipeline is made for).
struct Target {
    name: String,
    csv: String,
    data_dir: PathBuf,
}

/// The running clients that have a character logged in.
fn logged_in() -> Vec<client_reader::ClientInfo> {
    client_reader::list_clients().into_iter().filter(|c| c.character.is_some()).collect()
}

/// The shared block of Program.cs (lines 146-177) that picks the character.
fn resolve_character(cmd: &str, opts: &HashMap<String, String>) -> Result<Target> {
    let root = opts.get("data").cloned().unwrap_or_else(|| find_data_dir().to_string_lossy().into_owned());
    // With --data the folder is used as it is: its characters.json (read, never written or added to) says who is in it and
    // where each one's CSV is, and its config.json (see `config::use_data_dir`) says where a CSV that nothing names goes.
    let with_data = opts.contains_key("data");
    let mut known = if with_data { Some(Characters::read_only(&root)) } else { Some(Characters::new(&root, &default_name(), &default_csv()?)?) };
    let mut name = opts.get("name").cloned();
    if name.is_none() && matches!(cmd, "snapshot" | "sets" | "demon" | "expertise") {
        let in_game = logged_in();
        if in_game.is_empty() {
            return Err(user_error("No running client has a character logged in."));
        }
        if in_game.len() > 1 {
            let who: Vec<String> = in_game.iter().map(|c| format!("{} ({})", c.character.as_deref().unwrap_or(""), c.folder)).collect();
            return Err(user_error(format!("Several characters are logged in: {}. Pick one with --name.", who.join(", "))));
        }
        name = in_game[0].character.clone();
    }
    // alignment / toggles: the logged-in character when there's exactly one, else the saved data decides (offline use)
    if name.is_none() && matches!(cmd, "alignment" | "toggles" | "extras") {
        let in_game = logged_in();
        if in_game.len() == 1 {
            name = in_game[0].character.clone();
        }
    }
    let name = match name {
        Some(n) => n,
        None => match &known {
            Some(k) if k.all.len() > 1 => {
                let names: Vec<&str> = k.all.iter().map(|e| e.name.as_deref().unwrap_or("")).collect();
                return Err(user_error(format!("Data is saved for several characters: {}. Pick one with --name.", names.join(", "))));
            }
            Some(k) if k.all.len() == 1 => k.all[0].name.clone().unwrap_or_default(),
            _ => default_name(),
        },
    };
    let (name, entry_csv, data_dir) = match known.as_mut() {
        Some(k) if with_data => match k.find(&name).cloned() {
            Some(e) => {
                let dir = k.data_dir(&e);
                (e.name.clone().unwrap_or(name), e.csv.clone(), dir)
            }
            None => (name, None, PathBuf::from(&root)),
        },
        Some(k) => {
            let e = k.get_or_add(&name)?.clone();
            let dir = k.data_dir(&e);
            (e.name.clone().unwrap_or(name), e.csv.clone(), dir)
        }
        None => (name, None, PathBuf::from(&root)),
    };
    let csv = match opts.get("csv") {
        Some(c) => c.clone(),
        None => match entry_csv {
            Some(c) => c,
            // the folder's own stats folder (its config.json), never the program's
            None if with_data => {
                let stats = config::current().stats_folder.clone().ok_or_else(null_path)?;
                path_combine(&stats, &format!("{}-stats.csv", safe_file_name(&name).to_lowercase()))
            }
            None => default_csv()?,
        },
    };
    Ok(Target { name, csv, data_dir })
}

// ---------- the commands ----------

/// `Rebuild()`: the page data from the CSV and the saved files.
fn rebuild(t: &Target, pipe: &Pipeline) -> Result<()> {
    let c = pipe.rebuild(&t.csv)?;
    console::write_line(&format!(
        "Page data in {}: snapshots {}, gear {}, expertise {}, bonuses {}",
        pipe.store().dash_dir().display(),
        c.snapshots,
        c.gear,
        c.expertise,
        c.bonuses
    ));
    Ok(())
}

fn print_snapshot(name: &str, r: &SnapshotResult) {
    console::write_line(&format!("{name}  level {}  HP {}/{}  MP {}/{}  (stats: {}; source {})", r.level, r.hp, r.max_hp, r.mp, r.max_mp, r.stats_note, r.source));
    if !r.missing.is_empty() {
        console::write_line(&format!("No saved tooltip for: {}", r.missing));
    }
    for i in 0..12 {
        console::write(&format!("{} {}  ", client_reader::STAT_NAMES[i], r.pairs[i * 2].wrapping_add(r.pairs[i * 2 + 1])));
    }
    console::blank_line();
    for c in r.combat.iter().filter(|c| c.always != 0.0 || c.digi != 0.0) {
        console::write_line(&format!("  {:<30} {:>8} {:>8}", c.stat, fmt::custom(c.always, "0.##"), fmt::custom(c.digi, "0.##")));
    }
    for s in r.gear.sets.iter().filter(|s| s.active && !s.appearance) {
        console::write_line(&format!("  set {} [{}] {}", s.set_id, s.worn, s.pieces));
    }
    if let Some(dm) = r.demon.as_ref().and_then(|d| d.demon.as_ref()) {
        console::write_line(&format!(
            "Digitalize demon: {} Lv{}{}, {}% of its stats",
            dm.name,
            dm.level,
            if r.demon.as_ref().is_some_and(|d| d.digitalized_when_taken) { " (digitalized now)" } else { "" },
            r.digitalize_result.as_ref().map(|d| d.rate.to_string()).unwrap_or_default()
        ));
        for (stat, v) in r.demon_combat.iter().filter(|(_, v)| **v != 0.0) {
            console::write_line(&format!("  {:<30} +{} while digitalized", stat, fmt::custom(*v, "0.##")));
        }
    }
    if r.while_digitalized {
        console::write_line("You were digitalized: the window's stats include the demon, so the saved normal stats were used.");
    }
}

/// The rows of dash\expertise.json that are toggle skills.
fn toggle_rows(pipe: &Pipeline) -> Result<Vec<Value>> {
    let doc = json::load(pipe.store().dash_dir().join("expertise.json"))?;
    Ok(json::arr(Some(&doc)).filter(|r| json::truthy(r.at("toggle"))).cloned().collect())
}

/// `SidOf(row)`: the skill id at the end of the row's id ("exp-12-5").
fn sid_of(r: &Value) -> Result<i32> {
    let row = json::str(r.at("row")).unwrap_or_default();
    let last = row.rsplit('-').next().unwrap_or("");
    fmt::parse_i32(last).ok_or_else(|| anyhow::anyhow!("The input string '{last}' was not in a correct format."))
}

fn toggles(opts: &HashMap<String, String>, t: &Target, pipe: &Pipeline) -> Result<i32> {
    // imagine-reader toggles ["Name, Name, ..." | none]: show or set which toggle (switch) skills are on
    let store = pipe.store();
    let data = pipe.data();
    let mut rows = toggle_rows(pipe)?;
    let arg = opts.get("_arg").map(String::as_str);
    let mut from_game = false;
    if arg.is_none() {
        // The game decides when it's running: the learned-skill map's "on" flag (DemonReader.ReadSkillMap)
        match client_reader::find_client(&t.name) {
            Ok(p) => {
                let gm = GameMemory::new(p.id())?;
                if let Some(on) = demon_reader::read_progress(&gm).and_then(|p| p.switched_on) {
                    let demons = data.demons()?;
                    let mut v: Vec<i32> = on.into_iter().filter(|id| demons.skills.get(id).is_some_and(|s| s.0 == 2)).collect();
                    v.sort();
                    store.save_toggles_from_game(v)?;
                    from_game = true;
                    rebuild(t, pipe)?;
                    rows = toggle_rows(pipe)?;
                }
            }
            Err(e) if imagine::error::is_user_error(&e) => {}
            Err(e) => return Err(e),
        }
    }
    if let Some(arg) = arg {
        let mut want: Vec<i32> = Vec::new();
        if !arg.eq_ignore_ascii_case("none") {
            for n in arg.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let hit = rows.iter().find(|r| json::str(r.at("skill")).is_some_and(|s| eq_ic(&s, n)));
                let Some(hit) = hit else {
                    let mut names: Vec<String> = Vec::new();
                    for r in &rows {
                        let s = json::str(r.at("skill")).unwrap_or_default();
                        if !names.contains(&s) {
                            names.push(s);
                        }
                    }
                    return Err(user_error(format!("No toggle skill called \"{n}\". Toggles: {}.", names.join(", "))));
                };
                want.push(sid_of(hit)?);
            }
        }
        store.save_toggles(want)?;
        rebuild(t, pipe)?;
        rows = toggle_rows(pipe)?;
    }
    let on = store.active_toggles()?;
    let mut seen: Vec<i32> = Vec::new();
    for r in &rows {
        let sid = sid_of(r)?;
        if seen.contains(&sid) {
            continue;
        }
        seen.push(sid);
        console::write_line(&format!(
            "{}  {:<28} {}",
            if on.contains(&sid) { "ON " } else { "off" },
            json::str(r.at("skill")).unwrap_or_default(),
            json::str(r.at("expertise")).unwrap_or_default()
        ));
    }
    console::write_line(if from_game {
        "Read from the game (switched on right now)."
    } else {
        "Saved state (the game isn't running). Override with: imagine-reader toggles \"Name, Name, ...\" (or none); the next snapshot reads the game again."
    });
    Ok(0)
}

fn extras(opts: &HashMap<String, String>, t: &Target, pipe: &Pipeline) -> Result<i32> {
    // Quest bonus: the count is read from the game's completed-quest map once the game has loaded the quest list
    // (e.g. the Quest Completion Bonuses window was opened); --quests sets it by hand. The compendium count isn't
    // found in memory yet: set by hand. The clan (name and level) is read from the game; its bonuses are the
    // learned clan skills.
    let store = pipe.store();
    let data = pipe.data();
    let (mut quests, mut book, mut book_v2) = store.counts()?;
    let mut changed = false;
    let mut quests_from_game = false;
    let mut book_from_game = false;
    let mut count = |key: &str, current: Option<i32>| -> Result<Option<i32>> {
        let Some(v) = opts.get(key) else { return Ok(current) };
        changed = true;
        if v.eq_ignore_ascii_case("none") {
            return Ok(None);
        }
        match fmt::parse_i32(v).filter(|n| *n >= 0) {
            Some(n) => Ok(Some(n)),
            None => Err(user_error(format!("--{key} takes a number (or none), not \"{v}\"."))),
        }
    };
    quests = count("quests", quests)?;
    book = count("compendium", book)?;
    if let Some(v2s) = opts.get("compendium-v2") {
        changed = true;
        book_v2 = match v2s.to_lowercase().as_str() {
            "yes" | "true" => Some(true),
            "no" | "false" => Some(false),
            "none" => None,
            _ => return Err(user_error("--compendium-v2 takes yes, no or none.")),
        };
    }
    match client_reader::find_client(&t.name) {
        Ok(p) => {
            let gm = GameMemory::new(p.id())?;
            if let Some(st) = progress_reader::read(&gm, None, None, None) {
                let level = character_extras::clan_level(data, Some(&st))?.map(|l| l.to_string()).unwrap_or_else(|| "?".to_string());
                console::write_line(&format!("Clan: {}, level {level} (its bonuses are the learned clan skills, on the Expertise tab)", st.clan_name.as_deref().unwrap_or("none")));
            }
            if !opts.contains_key("quests") {
                let done = demon_reader::read_progress(&gm).and_then(|p| p.completed_quests);
                if let Some(qc) = character_extras::quest_bonus_count(done)? {
                    if Some(qc) != quests {
                        changed = true;
                    }
                    quests = Some(qc);
                    quests_from_game = true;
                }
            }
            if !opts.contains_key("compendium") {
                if let Some(cc) = progress_reader::read_compendium_count(&gm) {
                    if Some(cc) != book {
                        changed = true;
                    }
                    book = Some(cc);
                    book_from_game = true;
                }
            }
        }
        Err(e) if imagine::error::is_user_error(&e) => console::write_line("Clan: the game isn't running, not read."),
        Err(e) => return Err(e),
    }
    if changed || quests_from_game || book_from_game {
        store.save_counts(quests, book, book_v2, quests_from_game, book_from_game)?;
    }
    if changed {
        rebuild(t, pipe)?;
    }
    let qsrc = store.quest_count_source()?;
    console::write_line(&format!(
        "Completed quests: {}{}",
        quests.map(|q| q.to_string()).unwrap_or_else(|| "not set (open the Quest Completion Bonuses window in game, then run this again)".to_string()),
        match (&quests, &qsrc) {
            (Some(_), Some(s)) => format!(" ({s})"),
            _ => String::new(),
        }
    ));
    let csrc = store.compendium_count_source()?;
    console::write_line(&format!(
        "Compendium entries: {}{}   Compendium v2: {}",
        book.map(|q| q.to_string()).unwrap_or_else(|| "not set (open the Demonic Compendium window in game, then run this again)".to_string()),
        match (&book, &csrc) {
            (Some(_), Some(s)) => format!(" ({s})"),
            _ => String::new(),
        },
        match book_v2 {
            Some(true) => "yes",
            Some(false) => "no",
            None => "not set (assumed yes)",
        }
    ));
    let state = ProgressState { quest_bonus_count: quests, compendium_count: book, has_compendium_v2: book_v2, ..Default::default() };
    for e in character_extras::compute(data, None, Some(&state), false)? {
        console::write_line(&format!(
            "  {:<60} {}{}{}{}",
            e.item,
            if e.target == "partner" { "Partner's " } else { "" },
            e.label,
            match e.value {
                Some(x) => format!(" {}{}{}", if x >= 0.0 { "+" } else { "" }, fmt::g(x), if e.pct { "%" } else { "" }),
                None => String::new(),
            },
            if e.cond == "always" { String::new() } else { format!("  [{}{}]", e.cond, if e.cond_note.is_empty() { String::new() } else { format!(": {}", e.cond_note) }) }
        ));
    }
    Ok(0)
}

fn alignment(opts: &HashMap<String, String>, t: &Target, pipe: &Pipeline) -> Result<i32> {
    // imagine-reader alignment [LAW|NEUTRAL|CHAOS] [--name N]
    // No value: read it from the game when it's running (and save it), else show the saved one. A value overrides
    // until the next snapshot reads the game again.
    let store = pipe.store();
    let name = &t.name;
    let Some(value) = opts.get("_arg") else {
        let mut lnc: Option<i32> = None;
        match client_reader::find_client(name) {
            Ok(p) => {
                let am = GameMemory::new(p.id())?;
                lnc = client_reader::read_lnc(&am);
            }
            Err(e) if imagine::error::is_user_error(&e) => {}
            Err(e) => return Err(e),
        }
        if let Some(v) = lnc {
            let g = client_reader::lnc_name(v);
            if g != store.alignment()? || !store.profile_path().exists() {
                let ar = pipe.set_alignment(&t.csv, g)?;
                for line in &ar.log {
                    console::write_line(line);
                }
            }
            console::write_line(&format!("{name}: {g} ({v} alignment points, read from the game; LAW at -5000 or less, CHAOS at 5000 or more)"));
        } else {
            console::write_line(&format!(
                "{name}: {}{}",
                store.alignment()?,
                if store.profile_path().exists() { " (saved; the game isn't running)" } else { " (assumed; not set)" }
            ));
        }
        return Ok(0);
    };
    let r = pipe.set_alignment(&t.csv, value)?;
    for line in &r.log {
        console::write_line(line);
    }
    console::write_line(&if !r.changed {
        format!("{name} is already {}.", store.alignment()?)
    } else {
        format!(
            "{name} is now {}. {} of {} snapshots changed.{}",
            store.alignment()?,
            r.changed_rows,
            r.rows,
            match &r.backup {
                Some(b) => format!(" Spreadsheet as it was: {b}"),
                None => String::new(),
            }
        )
    });
    Ok(0)
}

fn sets(t: &Target, pipe: &Pipeline) -> Result<i32> {
    let store = pipe.store();
    let data = pipe.data();
    let name = &t.name;
    let proc = client_reader::find_client(name)?;
    let mem = GameMemory::new(proc.id())?;
    let grid = client_reader::live_gear(&mem).ok_or_else(|| user_error("Couldn't find the equipped gear."))?;
    let ids = grid.ids.clone();
    let tips = store.load_tooltip_cache()?;
    let lvl = match client_reader::read_nameplate(&mem, name) {
        Some(n) => n.level,
        None => store.tooltips_level()?,
    };
    let texts: Vec<String> = ids.iter().map(|&i| if i > 0 { tips.get(&i).and_then(|t| t.text.clone()).unwrap_or_default() } else { String::new() }).collect();
    let has_ench = !grid.tarot.is_empty();
    let g = pipe.ctx.gear()?.compute(&texts, Some(&ids), has_ench.then_some(&grid.tarot[..]), has_ench.then_some(&grid.soul[..]), grid.details.as_deref(), lvl, None)?;
    if grid.details.is_some() {
        console::write_line(&format!("  item bonuses from each item's record and the server data ({})", g.item_source));
    }
    if !has_ench {
        console::write_line("  (tarot/soul ids couldn't be read from memory, so fusion sets go by the crystal names in saved tooltips)");
    }
    for k in 0..15 {
        let tarot = grid.tarot.get(k).copied().unwrap_or(0);
        let soul = grid.soul.get(k).copied().unwrap_or(0);
        let item_name = if ids[k] > 0 { data.item_name(ids[k])?.unwrap_or_else(|| "unknown item".to_string()) } else { "(empty)".to_string() };
        let tarot_name = if tarot > 0 { data.enchant_name(tarot)? } else { String::new() };
        let soul_name = if soul > 0 { data.enchant_name(soul)? } else { String::new() };
        console::write_line(&format!(
            "  slot {k:>2} {:<9} item {:>6} {:<34} tarot {:>5} {:<16} soul {:>5} {}",
            client_reader::SLOT_NAMES[k],
            ids[k],
            item_name,
            tarot,
            tarot_name,
            soul,
            soul_name
        ));
    }
    for s in g.sets.iter().filter(|s| s.active || s.fusion) {
        let effects: Vec<String> =
            s.effects.iter().map(|e| format!("{} {}{} [{}/{}]", e.label, e.value.map(fmt::g).unwrap_or_default(), if e.pct { "%" } else { "" }, e.cond, e.target)).collect();
        console::write_line(&format!(
            "{:<10} {:>5} [{}] {} {}: {}",
            if s.appearance {
                "appearance"
            } else if s.fusion {
                "fusion"
            } else {
                "set"
            },
            s.set_id,
            s.worn,
            if s.active { "ACTIVE" } else { "inactive" },
            s.pieces,
            effects.join("; ")
        ));
    }
    Ok(0)
}

fn demon(opts: &HashMap<String, String>, t: &Target, pipe: &Pipeline) -> Result<i32> {
    // imagine-reader demon [demon name] [--assists all|TYPE,...|id,...] [--mitama-boost] [--name N]
    let store = pipe.store();
    let data = pipe.data();
    let name = &t.name;
    let proc = client_reader::find_client(name)?;
    let mem = GameMemory::new(proc.id())?;
    if opts.contains_key("picker") {
        // what the app's demon dropdown gets after "Load demons from the game" (this saves the list, like the app does)
        for c in pipe.comp_demons(name, true)? {
            console::write_line(&format!(
                "{:<22} Lv{:<3} {:<12} {}{}{}  {}{}",
                c.name,
                c.level,
                c.race,
                if c.current { "current " } else { "" },
                if c.in_game { "in COMP " } else { "" },
                if c.loaded {
                    format!(
                        "{}% stats, {} skills",
                        c.offer.as_ref().map(|o| o.rate.to_string()).unwrap_or_default(),
                        c.offer.as_ref().map(|o| o.skills.len().to_string()).unwrap_or_default()
                    )
                } else {
                    "no details".to_string()
                },
                match &c.unlock {
                    None => String::new(),
                    Some(u) if u.listed => format!(
                        "{}: {} DLv {} (yours {})",
                        if u.reached() { "unlocked" } else { "locked" },
                        u.race,
                        u.level,
                        u.race_level.map(|l| l.to_string()).unwrap_or_default()
                    ),
                    Some(_) => "not on any Digitalization Level".to_string(),
                },
                if c.in_comp == Some(false) { "  [not in COMP]" } else { "" }
            ));
            // its key, and the skills in its slots (two demons of one kind and level differ there)
            let slots: Vec<String> = c.offer.as_ref().map_or(Vec::new(), |o| {
                o.skills.iter().filter(|s| !matches!(json::str(s.get("kind")).as_deref(), Some("Feature" | "Equipment"))).filter_map(|s| json::str(s.get("name"))).collect()
            });
            console::write_line(&format!("    {}  {}", c.key, slots.join(", ")));
        }
        return Ok(0);
    }
    if opts.contains_key("comp") {
        // the COMP's slots as the game lists them, with or without details
        let all = demon_reader::read_demons(&mem, true);
        let Some(comp) = demon_reader::read_comp(&mem, &all) else {
            console::write_line("The game's COMP slot list wasn't found.");
            return Ok(0);
        };
        for d in &comp {
            console::write_line(&format!(
                "{:<28} Lv{:<3} type {:<6} obj {:<9} HP {}/{}{}{}",
                d.name,
                d.level,
                d.r#type,
                d.object_id,
                d.hp,
                d.max_hp,
                if d.summoned() { " summoned" } else { "" },
                if d.loaded() { "" } else { "  (no details loaded)" }
            ));
        }
        return Ok(0);
    }
    if opts.contains_key("raw") {
        // every demon record in memory, also the ones the reader leaves out (no stats and no skills loaded)
        let mut all = demon_reader::read_demons(&mem, true);
        all.sort_by_key(|d| d.address);
        for d in &all {
            let mut line = format!(
                "@{:08X} flags {:02X} obj {:<9} {:<22} Lv{:<3} type {:<6} HP {}/{} stats {} slots {} learned {} growth {} mitama {}",
                d.address,
                d.flags,
                d.object_id,
                d.name,
                d.level,
                d.r#type,
                d.hp,
                d.max_hp,
                d.live_value.iter().take(6).map(i32::to_string).collect::<Vec<_>>().join(","),
                d.skills.iter().filter(|&&x| x > 0).count(),
                d.acquired.len(),
                d.growth_type,
                d.mitama_type
            );
            if opts.get("raw").map(String::as_str) == Some("data") {
                if let Some(rec) = mem.read(d.address, 36) {
                    if let Some(dat) = mem.read(u32::from_le_bytes([rec[32], rec[33], rec[34], rec[35]]) as i64, 64) {
                        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02X}")).collect::<String>();
                        line.push_str(&format!("\n      rec {}\n      data+20 {}", hex(&rec[4..36]), hex(&dat[20..64])));
                    }
                }
            }
            console::write_line(&line);
        }
        return Ok(0);
    }
    let demons = demon_reader::read_demons(&mem, false);
    if demons.is_empty() {
        return Err(user_error("No COMP demons found in the client's memory."));
    }
    let filter = opts.get("_arg").map(String::as_str);
    let tables = data.demons()?;
    let Some(filter) = filter else {
        let mut sorted: Vec<&DemonRecord> = demons.iter().collect();
        sorted.sort_by(|a, b| b.level.cmp(&a.level)); // OrderByDescending: stable
        for d in sorted {
            let mitama = if d.mitama_type > 0 { format!(" {} Mitama", MITAMA_NAMES[d.mitama_type as usize]) } else { String::new() };
            let unlock = match pipe.unlock_for(d.r#type)? {
                Some(u) if u.listed => format!(
                    "  {}: {} DLv {} (yours {})",
                    if u.reached() { "unlocked" } else { "locked" },
                    u.race,
                    u.level,
                    u.race_level.map(|l| l.to_string()).unwrap_or_else(|| "not read".to_string())
                ),
                Some(_) => "  not on any Digitalization Level".to_string(),
                None => String::new(),
            };
            console::write_line(&format!(
                "{:<28} Lv{:<3} {:<9} type {:<6}{:<13}{}{unlock}",
                d.name,
                d.level,
                tables.devils.get(&d.r#type).map(|v| v.race.as_str()).unwrap_or(""),
                d.r#type,
                mitama,
                if d.summoned() { " summoned" } else { "" }
            ));
        }
        console::write_line("Details and digitalize boosts: imagine-reader demon \"<name>\" [--assists all|TYPE,...]");
        return Ok(0);
    };
    let picked: Vec<&DemonRecord> = demons.iter().filter(|d| d.name.to_uppercase().contains(&filter.to_uppercase())).collect();
    if picked.is_empty() {
        return Err(user_error(format!("No COMP demon matches \"{filter}\". In the COMP: {}.", demons.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", "))));
    }
    let lvl = match client_reader::read_nameplate(&mem, name) {
        Some(n) => n.level,
        None => store.tooltips_level()?,
    };
    let calc = pipe.ctx.digitalize(lvl)?;
    // Learned assists: read from the game unless --assists overrides them
    let assist_arg = opts.get("assists").map(String::as_str);
    let progress = demon_reader::read_progress(&mem);
    let learned: Option<Vec<i32>> = if assist_arg.is_none() { progress.as_ref().map(|p| p.assists.clone()) } else { None };
    if assist_arg.is_none() && learned.is_none() {
        console::write_line("(couldn't read the learned digitalize assists; pass --assists)");
    }
    // Boosted Mitama sets: --mitama-boost [yes|no] overrides; otherwise whether the character has skill 5133
    let boost = match opts.get("mitama-boost") {
        Some(mb) => !matches!(mb.as_str(), "no" | "false"),
        None => match &progress {
            Some(p) => p.has_skill(data.mitama_set_boost_skill()?),
            None => false,
        },
    };
    if !opts.contains_key("mitama-boost") && progress.as_ref().is_none_or(|p| p.skills.is_none()) {
        console::write_line("(couldn't read the learned skills: Mitama set boost assumed off; pass --mitama-boost)");
    }
    if opts.contains_key("use") {
        // Make this the demon the Digitalized view adds (a snapshot taken while digitalized does this by itself)
        if picked.len() != 1 {
            return Err(user_error(format!(
                "\"{filter}\" matches {} demons: {}. Be more specific.",
                picked.len(),
                picked.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
        // Same as the app's demon picker: saved demon, the latest CSV row's demon columns (CSV copied first), page data
        pipe.comp_demons(name, true)?;
        // its key in the list (a second demon of its kind and level has "#2")
        let seen = pipeline::load_seen(store)?;
        let key = pipeline::find_key(seen.iter().map(|(k, s)| (k, &s.demon)), picked[0]).unwrap_or_else(|| pipeline::key_of(picked[0]));
        let sw = pipe.use_demon(name, &t.csv, Some(&key), true)?;
        console::write_line(&format!(
            "{} is now the digitalize demon.{}",
            sw.name,
            if sw.row_updated { format!(" The latest snapshot uses it (copy of the CSV before: {}).", sw.backup.as_deref().unwrap_or("")) } else { String::new() }
        ));
        return Ok(0);
    }
    // The character's normal Max HP/MP and STR/MAG/VIT/INT (Information window when open and not digitalized,
    // else the last saved copy): the base for "Max HP/MP while digitalized".
    let hpmp: Option<Calibration>;
    let hpmp_from: Option<String>;
    {
        let known_items = data.items()?;
        let win = if progress.as_ref().is_some_and(|p| p.digitalized()) { None } else { client_reader::read_info_window(&mem, name, |id| known_items.contains_key(&id)) };
        // Percent Max HP/MP from the toggles on (e.g. -10% MP each for Gunslinger, Extended Scope...): live from the game
        let toggles_on: Vec<i32> = match progress.as_ref().and_then(|p| p.switched_on.as_ref()) {
            Some(s) => s.iter().copied().collect(),
            None => store.active_toggles()?,
        };
        if let Some(win) = win {
            hpmp = character_hp_mp::calibrate_from_reading(data.passives()?, win.level, &win.pairs, win.max_hp, win.max_mp, &toggles_on, &toggles_on);
            hpmp_from = Some("Information window".to_string());
        } else {
            // Saved stats were measured with the toggles on at that time (saved with them)
            let (cal, at) = character_hp_mp::from_saved_stats(&store.last_stats_path(), data.passives()?, Some(&toggles_on))?;
            hpmp = cal;
            hpmp_from = at.map(|a| format!("saved stats of {a}"));
        }
    }
    for d in &picked {
        print_demon(d, &calc, data, assist_arg, learned.as_deref(), boost, name, lvl, hpmp.as_ref(), hpmp_from.as_deref())?;
    }
    // While digitalized: what the game shows now, to check the prediction against (the Information window if
    // it's open, else the nameplate)
    if let Some(p) = progress.as_ref().filter(|p| p.digitalized()) {
        let known_items = data.items()?;
        let dw = client_reader::read_info_window(&mem, name, |id| known_items.contains_key(&id));
        let np = client_reader::read_nameplate(&mem, name);
        let with = demons.iter().find(|x| x.object_id == p.digitalized_demon).map(|x| x.name.clone()).unwrap_or_else(|| "a demon".to_string());
        let stats4 = |pairs: &[i32]| (0..4).map(|k| pairs[2 * k].wrapping_add(pairs[2 * k + 1]).to_string()).collect::<Vec<_>>().join(" ");
        if let Some(dw) = dw {
            console::write_line(&format!("Digitalized with {with} now. The game shows Max HP {}, Max MP {} (Information window); STR..INT {}", dw.max_hp, dw.max_mp, stats4(&dw.pairs)));
        } else if let Some(os) = client_reader::read_own_status(&mem, true) {
            console::write_line(&format!("Digitalized with {with} now. The game shows Max HP {}, Max MP {} (character data); STR..INT {}", os.max_hp, os.max_mp, stats4(&os.pairs)));
        } else if let Some(np) = np {
            console::write_line(&format!("Digitalized with {with} now. The game shows Max HP {}, Max MP {} (nameplate)", np.max_hp, np.max_mp));
        }
    }
    Ok(0)
}

#[allow(clippy::too_many_arguments)]
fn print_demon(
    d: &DemonRecord,
    calc: &Digitalize,
    data: &GameData,
    assist_arg: Option<&str>,
    learned: Option<&[i32]>,
    mitama_boost: bool,
    who: &str,
    lvl: i32,
    hpmp: Option<&Calibration>,
    hpmp_from: Option<&str>,
) -> Result<()> {
    let t = data.demons()?;
    let devil = t.devils.get(&d.r#type);
    let sk = |id: i32| -> Result<String> { Ok(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };
    const SHORT_STAT: [&str; 21] = ["STR", "MAG", "VIT", "INT", "SPD", "LUCK", "HP", "MP", "", "", "", "", "CLS-R", "LNG-R", "Spell", "Support", "", "", "", "P.Def", "M.Def"];
    let say = |s: String| console::write_line(&s);
    let assist_text = |a: &Assist| format!("L{} {}{}", (a.id - 1) / 72 + 2, a.r#type, if a.value != 0 { format!(" {}", a.value) } else { String::new() });

    // Assists: read from the game (learned), or --assists: "all", assist types (STAT_RATE,PASSIVES,...) or ids;
    // types/all pick that race's assists.
    let mut assists: Option<Vec<i32>> = learned.map(<[i32]>::to_vec);
    if let (Some(arg), Some(devil)) = (assist_arg, devil) {
        let race = t.assists_of_race(devil.race_id);
        let mut v: Vec<i32> = Vec::new();
        for tok in arg.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if tok.eq_ignore_ascii_case("all") {
                v.extend(race.iter().map(|a| a.id));
            } else if let Some(id) = fmt::parse_i32(tok) {
                v.push(id);
            } else {
                v.extend(race.iter().filter(|a| a.r#type.eq_ignore_ascii_case(tok)).map(|a| a.id));
            }
        }
        assists = Some(v);
    }
    let r = calc.compute(d, assists.as_deref(), mitama_boost)?;

    say(format!(
        "{}  Lv{}  {} (race {})  type {}{}  growth {} {}{}",
        d.name,
        d.level,
        devil.map(|v| v.race.as_str()).unwrap_or(""),
        devil.map(|v| v.race_id.to_string()).unwrap_or_default(),
        d.r#type,
        if d.mitama_type > 0 { format!("  {} Mitama rank {}", MITAMA_NAMES[d.mitama_type as usize], d.mitama_rank) } else { String::new() },
        r.growth,
        t.growth_names.get(&r.growth).map(String::as_str).unwrap_or(""),
        if d.summoned() { "  (summoned)" } else { "" }
    ));
    say(format!(
        "  Window: HP {}/{}  MP {}/{}  {}  {}",
        d.hp,
        d.max_hp,
        d.mp,
        d.max_mp,
        (0..6).map(|i| format!("{} {}+{}", SHORT_STAT[i], d.live_value[i], d.live_bonus[i])).collect::<Vec<_>>().join("  "),
        [12usize, 13, 14, 15, 19, 20].iter().map(|&i| format!("{} {}", SHORT_STAT[i], d.live_value[i])).collect::<Vec<_>>().join("  ")
    ));
    say(format!("  Rebirth: {}", (0..12).map(|i| format!("{} {}", RUNE_NAMES[i], d.reunion[i])).collect::<Vec<_>>().join(" ")));
    if let Some(fv) = &d.force_values {
        // DEMON_FORCE_CONVERSION order; each 100000 points is +1 (only for the summoned demon, not when digitalizing)
        const FORCE_NAMES: [&str; 20] = [
            "HP", "MP", "STR", "MAG", "VIT", "INT", "SPD", "LUCK", "HP regen", "MP regen", "CLS-R", "LNG-R", "Spell", "Support", "Critical", "P.Def", "M.Def", "Crit Def", "XP %", "Ailment Def",
        ];
        let force: Vec<String> = (0..20).filter(|&i| fv[i] >= 100000).map(|i| format!("{} +{}", FORCE_NAMES[i], fv[i] / 100000)).collect();
        say(format!("  Demon Force (summoned only): {}", if force.is_empty() { "none".to_string() } else { force.join(", ") }));
        let stack: Vec<String> = d.force_stack.clone().unwrap_or_default().into_iter().filter(|&x| x > 0).map(|x| x.to_string()).collect();
        say(format!("  Force stack: {}  (digitalize: needs the race's FORCE_STACK assist)", if stack.is_empty() { "empty".to_string() } else { stack.join(", ") }));
    }
    if let Some(eq) = d.equipment.as_ref().filter(|e| e.iter().any(|&x| x > 0)) {
        let devil_traits: Vec<i32> = t.devils.get(&d.r#type).map(|v| v.traits.clone()).unwrap_or_default();
        let mut parts: Vec<String> = Vec::new();
        for i in 0..4 {
            if eq[i] <= 0 {
                continue;
            }
            let skill = match t.equipment_skill.get(&eq[i]) {
                Some(&s) => sk(s)?,
                None => "?".to_string(),
            };
            let replaces = if i < devil_traits.len() && devil_traits[i] > 0 { format!(" (replaces {})", sk(devil_traits[i])?) } else { String::new() };
            parts.push(format!("slot {} item {} = {skill}{replaces}", i + 1, eq[i]));
        }
        say(format!("  Equipment: {}", parts.join(", ")));
    }
    say(format!("  Skills: {}", d.skills.iter().filter(|&&s| s > 0).map(|&s| sk(s)).collect::<Result<Vec<_>>>()?.join(", ")));
    say(format!("  Learned ({}): {}", d.acquired.len(), d.acquired.iter().map(|&s| sk(s)).collect::<Result<Vec<_>>>()?.join(", ")));
    if let Some(dv) = devil {
        say(format!("  Traits: {}", dv.traits.iter().filter(|&&x| x > 0).map(|&s| sk(s)).collect::<Result<Vec<_>>>()?.join(", ")));
    }
    if d.mitama_type > 0 {
        let mut counts = [0; 4];
        for &b in &d.mitama_reunion {
            if b != 0 {
                counts[(b / 32).min(3) as usize] += 1;
            }
        }
        say(format!("  Mitama slots: Ara {}  Nigi {}  Kusi {}  Saki {}", counts[0], counts[1], counts[2], counts[3]));
        for s in &r.sets {
            say(format!("    set {:<10} {}", set_name(s.id), if mitama_boost { &s.ex_description } else { &s.description }));
        }
    }
    let Some(devil) = devil else {
        for n in &r.notes {
            say(format!("  {n}"));
        }
        return Ok(());
    };

    console::blank_line();
    say(format!(
        "Digitalized with {}: {}% of its stats (base {}%{}; up to {}% with all of the race's stat-rate assists)",
        d.name,
        r.rate,
        r.base_rate,
        if devil.is_mitama() { ", Mitama-fused type" } else { "" },
        r.max_rate
    ));
    say(format!("  {}", r.stats.iter().map(|(k, v)| format!("{} {}->{}", SHORT_STAT[*k as usize], r.base[*k as usize], v)).collect::<Vec<_>>().join("  ")));
    if let Some(hpmp) = hpmp {
        let (df_hp, df_mp) = character_hp_mp::digitalize_only_flat(&r.effects);
        let (d_hp, d_mp) = character_hp_mp::digitalized_with_result(hpmp, &r.stats, &r.effects, r.hp(), r.mp(), df_hp, df_mp);
        say(format!(
            "  Max HP/MP while digitalized: {d_hp} / {d_mp}  (normal {} / {} from the {}: formula {} / {} + flat {} / {}; digitalized formula on the new STR/MAG/VIT/INT, + demon {} / {}{}; gear that only works digitalized isn't included)",
            hpmp.max_hp,
            hpmp.max_mp,
            hpmp_from.unwrap_or(""),
            hpmp.formula_hp,
            hpmp.formula_mp,
            hpmp.flat_hp(),
            hpmp.flat_mp(),
            r.hp(),
            r.mp(),
            if df_hp != 0 || df_mp != 0 { format!(" + digitalize-only {df_hp} / {df_mp}") } else { String::new() }
        ));
    }
    // Self-check: the window's STR..LUCK is the same base (Mitama sets included) plus demon force, which digitalizing leaves out.
    // (the COMP window shows the normal set bonuses, boosted or not)
    let mitama_set_ids: Vec<i32> = t.assists_of_race(devil.race_id).iter().filter(|a| a.r#type == "MITAMA_SET").map(|a| a.id).collect();
    let full = calc.compute(d, Some(&mitama_set_ids), false)?;
    say(format!(
        "  Check vs window (base + Mitama sets; a gap is demon force): {}",
        (0..6).map(|i| format!("{} {}{}{}", SHORT_STAT[i], full.base[i], if full.base[i] == d.live_value[i] { "=" } else { "≠" }, d.live_value[i])).collect::<Vec<_>>().join("  ")
    ));
    // Assist id = race slot + 36 j; level L offers j = 2(L-2) and 2(L-2)+1
    let mine: Option<Vec<&Assist>> = assists.as_ref().map(|ids| {
        let mut m: Vec<&Assist> = ids.iter().filter_map(|id| t.assists.get(id)).filter(|a| a.race_id == devil.race_id).collect();
        m.sort_by_key(|a| a.id);
        m
    });
    say(format!(
        "  Assists ({}): {}",
        if assist_arg.is_some() { "--assists" } else { "learned, from the game" },
        match &mine {
            None => "not known (pass --assists): parts that need one are listed, not counted".to_string(),
            Some(m) if m.is_empty() => "none".to_string(),
            Some(m) => m.iter().map(|a| assist_text(a)).collect::<Vec<_>>().join(", "),
        }
    ));
    if d.mitama_type > 0 {
        say(format!("  Mitama set bonuses: {}", if mitama_boost { "boosted (Mitama Demon Enhancement - Iron Man)" } else { "normal" }));
    }
    if r.extend_time != 0 || r.reduce_wait != 0 {
        say(format!("  Digitalize time +{}s, wait -{}s", r.extend_time, r.reduce_wait));
    }
    if !r.active_skills.is_empty() {
        say(format!("  Usable skills: {}", r.active_skills.iter().map(|&s| sk(s)).collect::<Result<Vec<_>>>()?.join(", ")));
    }
    // GroupBy(Item): first appearance
    let mut groups: Vec<(&str, Vec<&Effect>)> = Vec::new();
    for e in r.effects.iter().filter(|e| !e.cond_note.starts_with("stats")) {
        match groups.iter_mut().find(|g| g.0 == e.item) {
            Some(g) => g.1.push(e),
            None => groups.push((e.item.as_str(), vec![e])),
        }
    }
    for (item, g) in &groups {
        say(format!("  {item}"));
        for e in g {
            say(format!(
                "    {} {}{}{}",
                if e.counted() { "+" } else { " " },
                e.label,
                match e.value {
                    Some(v) => format!(" {}{}{}", if v >= 0.0 { "+" } else { "" }, fmt::custom(v, "0.##"), if e.pct { "%" } else { "" }),
                    None => String::new(),
                },
                if e.counted() { String::new() } else { format!("  [{}{}]", e.cond, if e.cond_note.is_empty() { String::new() } else { format!(": {}", e.cond_note) }) }
            ));
        }
    }
    say(format!(
        "  Race assists ({}): {}",
        devil.race,
        t.assists_of_race(devil.race_id).iter().map(|a| format!("{} {}{}", a.id, a.r#type, if a.value != 0 { format!(" {}", a.value) } else { String::new() })).collect::<Vec<_>>().join(", ")
    ));
    for n in &r.notes {
        say(format!("  note: {n}"));
    }
    say(format!("  (+ = counted while digitalized; level-scaled effects use {who}'s level {lvl})"));
    Ok(())
}

fn digilevels(opts: &HashMap<String, String>) -> Result<i32> {
    // imagine-reader digilevels [--name N]: Digitalization Level of each race, learned assists and the extend
    // skills those levels give (read-only; independent of the saved data).
    let who = match opts.get("name") {
        Some(n) => n.clone(),
        None => {
            let in_game = logged_in();
            if in_game.is_empty() {
                return Err(user_error("No running client has a character logged in."));
            }
            if in_game.len() > 1 {
                let names: Vec<String> = in_game.iter().map(|c| format!("{} ({})", c.character.as_deref().unwrap_or(""), c.folder)).collect();
                return Err(user_error(format!("Several characters are logged in: {}. Pick one with --name.", names.join(", "))));
            }
            in_game[0].character.clone().unwrap_or_default()
        }
    };
    let proc = client_reader::find_client(&who)?;
    let mem = GameMemory::new(proc.id())?;
    let gd = Arc::new(GameData::new());
    let t = gd.demons()?;
    let progress = demon_reader::read_progress(&mem).ok_or_else(|| user_error("Couldn't read the character's progress (digitalize data) from memory."))?;
    let Some(levels) = progress.levels.as_ref() else { return Err(user_error("Couldn't read the Digitalization Levels from memory.")) };
    let lvl = client_reader::read_nameplate(&mem, &who).map(|n| n.level).unwrap_or(0);
    let fx = ItemEffects::new(Arc::clone(&gd), "NEUTRAL", lvl, Default::default());
    let sk = |id: i32| -> Result<String> { Ok(gd.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };
    let effects = |sid: i32| -> Result<String> {
        let Some(p) = gd.passives()?.get(&sid.to_string()).filter(|v| !v.is_null()) else { return Ok("no effect data".to_string()) };
        let mut list: Vec<String> = Vec::new();
        for c in json::arr(p.at("correctTbl")) {
            let e = fx.correct(&json::str(c.at("id")).unwrap_or_default(), json::int(c.at("type")), json::num(c.at("value")).unwrap_or(0.0), "always", "digitalize", "");
            let v = e.value.unwrap_or(0.0);
            list.push(format!("{} {}{}{}", e.label, if v >= 0.0 { "+" } else { "" }, fmt::custom(v, "0.##"), if e.pct { "%" } else { "" }));
        }
        for tn in json::arr(p.at("tokuseiIds")) {
            let tid = json::int(Some(tn));
            if tid != 0 {
                for e in fx.tokusei(tid, "digitalize", "")? {
                    list.push(format!(
                        "{}{}{}{}",
                        if e.target != "self" { format!("{} ", e.target) } else { String::new() },
                        e.label,
                        match e.value {
                            Some(v) => format!(" {}{}{}", if v >= 0.0 { "+" } else { "" }, fmt::custom(v, "0.##"), if e.pct { "%" } else { "" }),
                            None => String::new(),
                        },
                        if e.cond == "always" || e.cond == "digitalized" { String::new() } else { format!(" [{}{}]", e.cond, if e.cond_note.is_empty() { String::new() } else { format!(": {}", e.cond_note) }) }
                    ));
                }
            }
        }
        Ok(if list.is_empty() { "no effect data".to_string() } else { list.join("; ") })
    };
    let assists: Vec<&Assist> = progress.assists.iter().filter_map(|id| t.assists.get(id)).collect();
    console::write_line(&format!("{who}: Digitalization Levels (read from the game)"));
    for r in t.race_levels(Some(levels), progress.points.as_ref()) {
        let mut mine: Vec<&Assist> = assists.iter().copied().filter(|a| a.race_id == r.race_id).collect();
        mine.sort_by_key(|a| a.id);
        if r.level == 0 && mine.is_empty() {
            continue;
        }
        // % as the window shows it: truncated to 2 decimals
        let pct = match r.percent_to_next {
            Some(d) => format!("{}% ({}/{})", fmt::custom((d * 100.0).floor() / 100.0, "0.00"), r.points, t.levels[&r.race_id][r.level as usize].next_xp),
            None => "max".to_string(),
        };
        let top_assist = mine.iter().map(|a| (a.id - 1) / 72 + 2).max().unwrap_or(0);
        console::write_line(&format!(
            "{:<14} race {:>3}  DLv{:>2}  {pct}{}",
            r.race,
            r.race_id,
            r.level,
            if top_assist > r.level { format!("  (!) an assist of L{top_assist} is above the level") } else { String::new() }
        ));
        if !mine.is_empty() {
            console::write_line(&format!(
                "    assists: {}",
                mine.iter().map(|a| format!("L{} {}{}", (a.id - 1) / 72 + 2, a.r#type, if a.value != 0 { format!(" {}", a.value) } else { String::new() })).collect::<Vec<_>>().join(", ")
            ));
        }
        for &sid in &r.extend_skills {
            let at = t.levels[&r.race_id].iter().find(|x| x.extend_skill_id == sid).map(|x| x.level).ok_or_else(|| anyhow::anyhow!("Sequence contains no matching element"))?;
            let known = progress.skills.as_ref().is_some_and(|s| s.contains(&sid));
            console::write_line(&format!("    L{at:<2} {sid} {}: {}{}", sk(sid)?, effects(sid)?, if progress.skills.is_some() && !known { "  (not in the learned skills!)" } else { "" }));
        }
    }
    let all = t.all_extend_skills(Some(levels));
    // Cross-check: extend skills in the learned-skill map that the levels don't explain
    let extend_ids: std::collections::HashSet<i32> = t.levels.values().flatten().filter(|x| x.extend_skill_id > 0).map(|x| x.extend_skill_id).collect();
    let extra: Option<Vec<i32>> = progress.skills.as_ref().map(|sk| sk.iter().copied().filter(|s| extend_ids.contains(s) && !all.contains(s)).collect());
    let missing: Option<Vec<i32>> = progress.skills.as_ref().map(|sk| all.iter().copied().filter(|s| !sk.contains(s)).collect());
    let list = |v: &[i32]| v.iter().map(i32::to_string).collect::<Vec<_>>().join(", ");
    console::write_line(&format!(
        "{} extend skills from the levels; {}",
        all.len(),
        match (&missing, &extra) {
            (Some(m), Some(e)) if m.is_empty() && e.is_empty() => "all of them, and no other, are in the learned skills".to_string(),
            (Some(m), Some(e)) => format!("cross-check with the learned skills: {} missing [{}], {} learned but not explained by the levels [{}]", m.len(), list(m), e.len(), list(e)),
            _ => "learned skills unreadable (no cross-check)".to_string(),
        }
    ));
    Ok(0)
}

// ---------- model --check ----------

/// `float.ToString(pattern)`: .NET formats a float's first 7 significant digits.
fn f32_text(v: f32, pattern: &str) -> String {
    let seven: f64 = format!("{:.6e}", v as f64).parse().unwrap_or(v as f64);
    fmt::custom(seven, pattern)
}

fn f32_list(v: &[f32], pattern: &str) -> String {
    v.iter().map(|x| f32_text(*x, pattern)).collect::<Vec<_>>().join(",")
}

/// model --check <folder or file>: read every .nif / .kf there and say which ones this reader can't (no output files).
fn model_check(opts: &HashMap<String, String>) -> Result<i32> {
    let target = &opts["check"];
    let target_path = Path::new(target);
    let files: Vec<PathBuf> = if target_path.is_dir() {
        let mut v = Vec::new();
        for entry in std::fs::read_dir(target_path)? {
            let entry = entry?;
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let lower = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if lower.ends_with(".nif") || lower.ends_with(".kf") {
                v.push(p);
            }
        }
        v
    } else {
        vec![target_path.to_path_buf()]
    };
    let mut ok = 0;
    let mut why: Vec<(String, i32)> = Vec::new(); // insertion order, like the C# Dictionary
    let strip = regex::Regex::new(r"^.*?: |block \d+ | at byte \d+|^\S+ is ").expect("message pattern");
    for f in &files {
        match NifFile::load(f) {
            Ok(nif) => {
                ok += 1;
                if files.len() == 1 {
                    print_nif(&nif);
                }
            }
            Err(e) => match e.downcast_ref::<NifError>() {
                Some(ne) => {
                    let m = strip.replace_all(&ne.0, "").into_owned();
                    match why.iter_mut().find(|(k, _)| *k == m) {
                        Some(w) => w.1 += 1,
                        None => why.push((m, 1)),
                    }
                    if files.len() == 1 || opts.contains_key("verbose") {
                        console::write_line(&ne.0);
                    }
                }
                None => return Err(e),
            },
        }
    }
    console::write_line(&format!("{ok} of {} files read.", files.len()));
    why.sort_by_key(|w| std::cmp::Reverse(w.1)); // OrderByDescending: stable
    for (m, n) in why {
        console::write_line(&format!("{n:>5}  {m}"));
    }
    Ok(0)
}

/// What a single file holds: its block types, and what moves each node of an animation (key types: 1 linear, 2 curve,
/// 3 TBC, 4 x/y/z angles).
fn print_nif(nif: &NifFile) {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for b in &nif.blocks {
        match counts.iter_mut().find(|(t, _)| *t == b.type_name) {
            Some(c) => c.1 += 1,
            None => counts.push((b.type_name.as_str(), 1)),
        }
    }
    counts.sort_by(|a, b| cmp_invariant(a.0, b.0));
    for (t, n) in counts {
        console::write_line(&format!("{n:>5}  {t}"));
    }
    // an animation: what moves each node
    for seq in nif.all::<NifSequence>() {
        for l in &seq.links {
            let how = match nif.get::<NifBlock>(l.interpolator) {
                Some(NifBlock { kind: NifKind::SplineInterpolator(s), .. }) => format!(
                    "spline {}-{} t@{} r@{} s@{} static t[{}] q[{}] s {}",
                    f32_text(s.start, "0.##"),
                    f32_text(s.stop, "0.##"),
                    s.translation_at as i32,
                    s.rotation_at as i32,
                    s.scale_at as i32,
                    f32_list(&s.t, "0.##"),
                    f32_list(&s.q, "0.###"),
                    f32_text(s.s, "0.##")
                ),
                Some(NifBlock { kind: NifKind::TransformInterpolator(t), .. }) => match nif.get::<imagine::data::nif::NifTransformData>(t.data) {
                    Some(d) => {
                        let rot = if let Some(r) = &d.rotation {
                            format!("type {} x{}", d.rotation_type, r.times.len())
                        } else if let Some(e) = &d.euler {
                            let lens: Vec<String> = e.iter().map(|k| k.as_ref().map(|k| k.times.len()).unwrap_or(0).to_string()).collect();
                            let types: Vec<String> = e.iter().map(|k| k.as_ref().map(|k| k.type_).unwrap_or(0).to_string()).collect();
                            format!("angles x{} types {}", lens.join("/"), types.join("/"))
                        } else {
                            "-".to_string()
                        };
                        let keys = |k: &Option<imagine::data::nif::NifKeys>| match k {
                            Some(k) => format!("type {} x{}", k.type_, k.times.len()),
                            None => "-".to_string(),
                        };
                        format!(
                            "keys rot {rot}  pos {}  scale {}  static t[{}] s {}",
                            keys(&d.translation),
                            keys(&d.scale),
                            f32_list(&t.t, "0.##"),
                            f32_text(t.s, "0.##")
                        )
                    }
                    None => format!("fixed t[{}] q[{}] s {}", f32_list(&t.t, "0.##"), f32_list(&t.q, "0.###"), f32_text(t.s, "0.##")),
                },
                Some(o) => o.type_name.clone(),
                None => "nothing".to_string(),
            };
            console::write_line(&format!("  {:<22} {:<22} {how}", l.node.as_deref().unwrap_or(""), l.controller_type.as_deref().unwrap_or("")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> HashMap<String, String> {
        parse_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parse_args_like_the_csharp() {
        let o = args(&["model", "D2_04_009a", "--client", "C:\\x", "--verbose", "--out", "a.json", "extra"]);
        assert_eq!(o["_cmd"], "model");
        assert_eq!(o["_arg"], "D2_04_009a");
        assert_eq!(o["client"], "C:\\x");
        assert_eq!(o["verbose"], "true");
        assert_eq!(o["out"], "a.json");
        assert_eq!(o.len(), 5); // the third word is dropped
        let o = args(&["--flag", "--name", "Rubito", "sets"]);
        assert_eq!(o["flag"], "true");
        assert_eq!(o["name"], "Rubito");
        assert_eq!(o["_cmd"], "sets");
        assert!(!o.contains_key("_arg"));
        assert!(args(&[]).is_empty());
        // a value that starts with a single dash is a value
        assert_eq!(args(&["--n", "-5"])["n"], "-5");
        // "--" alone is an option with an empty name
        assert_eq!(args(&["--", "x"])[""], "x");
    }

    /// `--data <folder>`: the folder's characters.json and config.json decide the CSV, never the program's own data folder
    /// (the bug: a snapshot taken with `--data <copy>` was appended to the real CSV, named by the program's own config.json).
    /// One test, because the settings are loaded once per process.
    #[test]
    fn data_flag_decides_the_csv() {
        let root = std::env::temp_dir().join(format!("imagine-data-flag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("copy");
        let stats = root.join("copy-stats");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(data.join("dash")).unwrap(); // what makes Characters::new want to write a list
        let js = |p: &Path| serde_json::to_string(&p.to_string_lossy()).unwrap();
        std::fs::write(data.join("config.json"), format!("{{\"statsFolder\": {}}}", js(&stats))).unwrap();
        let listed = stats.join("listed.csv");
        let list = format!("[{{\"Name\":\"Tester\",\"Dir\":\".\",\"Csv\":{}}},{{\"Name\":\"Sub\",\"Dir\":\"characters\\\\Sub\",\"Csv\":{}}}]", js(&listed), js(&stats.join("sub.csv")));
        std::fs::write(data.join("characters.json"), &list).unwrap();
        let o = |extra: &[&str]| {
            let mut a = vec!["rebuild", "--data", data.to_str().unwrap()];
            a.extend_from_slice(extra);
            args(&a)
        };
        assert!(config::use_data_dir(&data), "the settings were loaded before --data was applied");
        assert_eq!(config::current().stats_folder.as_deref(), stats.to_str());
        assert_eq!(config::file_path(), data.join("config.json"));

        // a listed character: its own CSV and folder, from the folder's list
        let t = resolve_character("rebuild", &o(&["--name", "Tester"])).unwrap();
        assert_eq!((t.name.as_str(), t.csv.as_str(), t.data_dir.as_path()), ("Tester", listed.to_str().unwrap(), data.as_path()));
        let t = resolve_character("rebuild", &o(&["--name", "sub"])).unwrap();
        assert_eq!((t.name.as_str(), t.csv.as_str(), t.data_dir), ("Sub", stats.join("sub.csv").to_str().unwrap(), data.join("characters\\Sub")));
        // an unlisted name: the folder is used as it is, the CSV goes to the folder's stats folder
        let t = resolve_character("rebuild", &o(&["--name", "Nobody"])).unwrap();
        assert_eq!((t.csv.as_str(), t.data_dir.as_path()), (stats.join("nobody-stats.csv").to_str().unwrap(), data.as_path()));
        // --csv still wins
        let t = resolve_character("rebuild", &o(&["--name", "Tester", "--csv", "x.csv"])).unwrap();
        assert_eq!(t.csv, "x.csv");
        // no name: several are listed, so it asks
        assert!(resolve_character("rebuild", &o(&[])).is_err());
        // nothing was added to the list or created
        assert_eq!(std::fs::read_to_string(data.join("characters.json")).unwrap(), list);
        assert!(!stats.exists());
        // without a list, the folder's config.json still decides, and the program's own default CSV is never used
        std::fs::remove_file(data.join("characters.json")).unwrap();
        let t = resolve_character("rebuild", &o(&["--name", "Tester"])).unwrap();
        assert_eq!(t.csv, stats.join("tester-stats.csv").to_str().unwrap());
        assert!(!data.join("characters.json").exists(), "--data must not create a character list");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn float_text_uses_seven_digits() {
        assert_eq!(f32_text(1.005, "0.##"), "1.01");
        assert_eq!(f32_text(0.5, "0.##"), "0.5");
        assert_eq!(f32_text(-0.001, "0.###"), "-0.001");
        assert_eq!(f32_list(&[1.0, 2.5], "0.##"), "1,2.5");
    }

    #[test]
    fn unknown_command_prints_usage_and_exits_2() {
        assert_eq!(run(&args(&["frobnicate"])).unwrap(), 2);
        assert_eq!(run(&args(&[])).unwrap(), 0);
        assert_eq!(run(&args(&["help"])).unwrap(), 0);
    }

    #[test]
    fn commands_without_input_say_so() {
        let e = run(&args(&["dungeons"])).unwrap_err();
        assert_eq!(e.downcast_ref::<UserError>().unwrap().0, "dungeons needs --decoded <folder with the decoded client tables>.");
    }
}
