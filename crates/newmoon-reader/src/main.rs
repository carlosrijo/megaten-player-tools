//! newmoon-reader command line (port of src\Program.cs).
//!
//! newmoon-reader: read-only stat/gear snapshots of a NewMoon character, plus the data files for the build-tracker page.
//!
//!   clients                                                     list the running clients and who is logged in to each
//!   snapshot  [--name N] [--csv path] [--ignore-window]         read the game, append a CSV row, rebuild the page data
//!   rebuild   [--name N] [--csv path]                           rebuild the page data from saved files (no game needed)
//!   recalc    [--name N] [--csv path]                           recompute every CSV row's gear bonuses, then rebuild
//!   sets      [--name N]                                        show the sets/appearance bonuses for the worn gear
//!   simulate  [--spec build.json | --init | --catalog N]        the Simulator tool: check it, or work out another build (nothing is written)
//!   serve     [--port N] [--root dir] [--also "Name=folder"]    the app's page in a browser with a working Simulator (testing)
//!   alignment [LAW|NEUTRAL|CHAOS] [--name N]                    show or set the character's alignment (recalculates when it changes)
//!   extras    [--quests N|none] [--compendium N|none] [--compendium-v2 yes|no] [--name N]
//!                                                               show the clan; set the quest/compendium counts for their bonuses
//!
//! Without --name, the character is read from the running client (or is the only one with saved data).
//! Each character keeps its own folder and CSV, listed in data\characters.json.
//! --data <dir> uses that folder as-is instead (for tests on copies).
//! The desktop app (app\) runs the same steps through Build\Pipeline.cs.
//!
//! Commands ported so far: clients, model (and model --check), demondb, dungeons, skilldb, labels. The others are marked
//! `// WAVE3:` where they stop with "not ported yet".

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use newmoon::build::{demon_db, demon_model, dungeons, item_icons, skill_db};
use newmoon::console;
use newmoon::data::characters::{Characters, get_directory_name, path_combine};
use newmoon::data::config;
use newmoon::data::game_data::GameData;
use newmoon::data::nif::{NifBlock, NifError, NifFile, NifKind, NifSequence};
use newmoon::data::stat_labels;
use newmoon::data::store::Store;
use newmoon::fmt;
use newmoon::game::client_reader;
use newmoon::text::cmp_invariant;
use newmoon::{Result, UserError, user_error};

/// What a .NET process returns when an exception nobody caught ends it (0xE0434352).
const UNHANDLED_EXIT: i32 = 0xE043_4352_u32 as i32;

fn main() {
    // The C# command line never registered the code pages, so a model's Shift-JIS node names stay as they were read.
    demon_model::set_shift_jis_names(false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&args);
    let code = match run(&opts) {
        Ok(code) => code,
        Err(e) => match e.downcast_ref::<UserError>() {
            Some(u) => {
                console::error_line(&u.0);
                1
            }
            None => {
                // not a UserError: the C# let it end the process
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

    if cmd == "geardb" {
        // The Gear Database page's data, from the client's decoded tables and the community sheets (no game, no character).
        //   geardb --decoded <dir with ItemData.xml, SItemData.xml, EquipmentSetData.xml, CItemData.xml> [--sheets sheet_gear.tsv] [--out app\ui\gear-db.json]
        let _decoded = get("decoded").ok_or_else(|| user_error("geardb needs --decoded <folder with the decoded client tables>."))?;
        // WAVE3: Program.cs lines 46-52: GearDb.Build(new GameData(), decoded, opts["sheets"], opts.GetValueOrDefault("out", Path.Combine("app", "ui", "gear-db.json")),
        //        opts["ids-out"], opts["game-sources"]); return 0;   (--ids-out: the ids for tools\dcmexport --ids; --game-sources: its output).
        //        gear_db.rs is the other agent's port; call its build() here.
        return Err(user_error("not ported yet: geardb"));
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
    if cmd == "skilldb" {
        // The Skill Database page's data: every skill grouped by action type, from the client's decoded tables (no game, no character).
        //   skilldb --decoded <dir with SkillData.xml, CHelpData.xml> [--client <game folder, for icons>] [--out app\ui\skill-db.json]
        let decoded = get("decoded").ok_or_else(|| user_error("skilldb needs --decoded <folder with SkillData.xml and CHelpData.xml>."))?;
        let client = client_arg(get("client"))?;
        let out = get_or("out", path_combine(&path_combine("app", "ui"), "skill-db.json"));
        skill_db::build(&GameData::new(), Path::new(decoded), client.as_deref(), Path::new(&out))?;
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
    if matches!(cmd, "snapshot" | "rebuild" | "recalc" | "sets" | "alignment" | "demon" | "expertise" | "toggles" | "extras" | "simulate" | "serve") {
        // Which character: --name, else the one logged in (snapshot, sets) or the only one with saved data (rebuild, recalc).
        // Each character has its own folder and CSV (data\characters.json). --data / --csv point somewhere else instead,
        // which is how tests run on copies.
        let _target = resolve_character(cmd, opts)?;
        // WAVE3: Program.cs lines 177-179: pipe = new Pipeline(entry != null ? known.DataDir(entry) : root); store = pipe.Store; data = pipe.Data; ctx = pipe.Ctx;
        //        (build them from `_target.data_dir`; every arm below then uses _target.name / _target.csv, as `name` / `csv` in the C#).
    }

    // the switch of Program.cs
    match cmd {
        "snapshot" => {
            // WAVE3: Program.cs lines 182-188: pipe.TakeSnapshot(name, csv, opts.ContainsKey("ignore-window")); PrintSnapshot(name, r) (lines 656-675);
            //        "Expertise: {ranks} ranked (incl. computed chains)"; Rebuild().
            Err(not_ported(cmd))
        }
        "toggles" => {
            // WAVE3: Program.cs lines 190-240: newmoon-reader toggles ["Name, Name, ..." | none]: show or set which toggle (switch) skills are on.
            Err(not_ported(cmd))
        }
        "expertise" => {
            // WAVE3: Program.cs lines 242-248: Re-read expertise ranks and Digitalization Levels from the game and rebuild the page data (no CSV row).
            Err(not_ported(cmd))
        }
        "rebuild" => {
            // WAVE3: Program.cs lines 249-251: Rebuild() (lines 650-654).
            Err(not_ported(cmd))
        }
        "simulate" => {
            // WAVE3: Program.cs lines 252-275: the Simulator tool from the command line (--spec / --init / --catalog N / self check).
            Err(not_ported(cmd))
        }
        "serve" => {
            // WAVE3: Program.cs lines 276-289: the page in a plain browser with a working Simulator: PreviewServer.Run(...).
            Err(not_ported(cmd))
        }
        "recalc" => {
            // WAVE3: Program.cs lines 290-293: foreach line in pipe.Recalc(csv) print it; Rebuild().
            Err(not_ported(cmd))
        }
        "extras" => {
            // WAVE3: Program.cs lines 294-346: quest / compendium counts (--quests, --compendium, --compendium-v2), the clan read from the game, CharacterExtras.Compute.
            Err(not_ported(cmd))
        }
        "alignment" => {
            // WAVE3: Program.cs lines 347-373: newmoon-reader alignment [LAW|NEUTRAL|CHAOS] [--name N].
            Err(not_ported(cmd))
        }
        "sets" => {
            // WAVE3: Program.cs lines 374-397: the sets/appearance bonuses of the worn gear (ctx.Gear().Compute).
            Err(not_ported(cmd))
        }
        "demon" => {
            // WAVE3: Program.cs lines 398-474 (the command) and 678-775 (PrintDemon): newmoon-reader demon [name] [--assists ...] [--mitama-boost] [--picker|--comp|--raw|--use].
            Err(not_ported(cmd))
        }
        "labels" => {
            // newmoon-reader labels: writes reference\stat-labels-game.tsv, the game's own name for each stat id
            // (CorrectTbl / aspect), taken from the client's tokusei text (no game or character needed).
            let gd = GameData::new();
            let (rows, path) = stat_labels::generate(&gd)?;
            console::write_line(&format!("{rows} stat labels written to {}", path.display()));
            Ok(0)
        }
        "digilevels" => {
            // WAVE3: Program.cs lines 485-545: newmoon-reader digilevels [--name N]: Digitalization Level of each race, learned assists and the extend
            //        skills those levels give (read-only; independent of the saved data). Needs ItemEffects (fx.Correct / fx.Tokusei), DemonReader.ReadProgress,
            //        ClientReader.ReadNameplate and DemonTables.RaceLevels / AllExtendSkills.
            Err(not_ported(cmd))
        }
        _ => {
            console::write_line("usage: newmoon-reader clients | snapshot|rebuild|recalc|sets [--name N] [--csv path] [--data dir] [--ignore-window]");
            Ok(if cmd == "help" { 0 } else { 2 })
        }
    }
}

fn not_ported(cmd: &str) -> anyhow::Error {
    user_error(format!("not ported yet: {cmd}"))
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
        let text = newmoon::data::json::read_all_text(&saved)?;
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

// ---------- Pipeline's statics (WAVE3: these belong to build::pipeline) ----------

/// `Pipeline.FindDataDir()`: the "data" folder next to the project file (found by walking up from the program's
/// folder), else beside the exe itself (the portable release).
fn find_data_dir() -> PathBuf {
    config::home_dir().join("data")
}

/// `Pipeline.DefaultName`.
fn default_name() -> String {
    config::current().default_character.clone().unwrap_or_else(|| "Character".to_string())
}

/// `Pipeline.DefaultCsv`.
fn default_csv() -> Result<String> {
    let stats = config::current().stats_folder.clone().ok_or_else(null_path)?;
    Ok(path_combine(&stats, &format!("{}-stats.csv", default_name().to_lowercase())))
}

/// The character a command works on (the C# `name` and `csv`, and the folder its Pipeline is made for).
#[allow(dead_code)]
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
    let mut known = if opts.contains_key("data") { None } else { Some(Characters::new(&root, &default_name(), &default_csv()?)?) };
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
            None => default_csv()?,
        },
    };
    Ok(Target { name, csv, data_dir })
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
                Some(NifBlock { kind: NifKind::TransformInterpolator(t), .. }) => match nif.get::<newmoon::data::nif::NifTransformData>(t.data) {
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
                        let keys = |k: &Option<newmoon::data::nif::NifKeys>| match k {
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
    fn unported_commands_say_so() {
        let e = run(&args(&["digilevels"])).unwrap_err();
        assert_eq!(e.downcast_ref::<UserError>().unwrap().0, "not ported yet: digilevels");
        let e = run(&args(&["dungeons"])).unwrap_err();
        assert_eq!(e.downcast_ref::<UserError>().unwrap().0, "dungeons needs --decoded <folder with the decoded client tables>.");
    }
}
