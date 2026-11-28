//! Community Builds: a player's build or one demon's build, as BABEL publishes it for everyone to see on the website.
//!
//! What goes out is PUBLIC, so it is made from a fixed list of numbers the game shows the player anyway, and nothing
//! else: ids (pieces, crystals, skills, expertises, demons), levels, stats, ranks. No names, no effect text, no totals
//! this reader works out, no paths, no character name, no game state. The website draws the names and effects from its
//! own public files, so nothing made with the sealed data can be in a build.
//!
//! [`check`] is the gate: it accepts exactly the shape [`player_build`] and [`demon_build`] write (every key known,
//! every number in range, the only strings an alignment, a time, a build code and the player's own `about` text) and is
//! run on every build before it leaves the app. The gallery Worker checks the same shape again (a build reaching it
//! may come from anywhere).
//!
//! Format, version 1 (`v`):
//!   player = { v, kind: "player", takenAt, level, alignment, maxHp, maxMp, statsLive, stats, gear, gearPartial,
//!              expertise, cap, toggles, code, partner, digitalize [, about] }
//!     stats      = { str, mag, vit, int, spd, luck, clsr, lngr, spell, support, pdef, mdef: [base, bonus] }
//!     gear       = [ { slot (0-14), id, basic, special, tarot, soul, fuse: [3], mods: [5] } ] (worn pieces only)
//!     expertise  = [ [expertise id, points] ] (ids rising, points > 0); cap = the expertise point cap or null
//!     toggles    = the switch skills that are on (skill ids)
//!     code       = the same gear and expertise as a Build Planner code ("BABEL1-..."), for "copy into my planner"
//!     partner, digitalize = a demon (below) or null: the summoned demon, the saved digitalize demon
//!   demon  = { v, kind: "demon", seenAt, demon [, about] }
//!     a demon = { type, level, growthType, reunion: [12], mitamaType, mitamaRank, mitamaSlots: [96], skills: [8],
//!                 forceStackIds: [8], equipmentIds: [4], stats: [14] (Max HP, Max MP, the 12 stats) }
//!   about  = { name, title, note }: what the player typed in the Publish dialog (plain text; the page shows it as text)
//!   takenAt / seenAt = "YYYY-MM-DDTHH:MM" (to the minute)

use crate::build::pipeline::{CompDemon, Pipeline};
use crate::build::share_code;
use crate::data::json::{self, At};
use crate::{Result, user_error};
use serde_json::{Map, Value, json};

pub const VERSION: i64 = 1;
/// The largest build the gallery takes (a real one is 2-4 KB).
pub const MAX_BYTES: usize = 16 * 1024;
pub const MAX_NAME: usize = 24;
pub const MAX_TITLE: usize = 60;
pub const MAX_NOTE: usize = 500;

/// The stats a player build has, in the order the game's status window lists them, with the snapshot row's keys.
pub const STATS: [&str; 12] = ["str", "mag", "vit", "int", "spd", "luck", "clsr", "lngr", "spell", "support", "pdef", "mdef"];

const MAX_ID: i64 = 1_000_000;
const MAX_STAT: i64 = 99_999;
const MAX_FUSE: i64 = 50;
const MAX_POINTS: i64 = share_code::MAX_POINTS as i64;
const MAX_CAP: i64 = share_code::MAX_CAP as i64;
const MAX_TOGGLES: usize = 64;

// ---------- making a build ----------

/// The player build of the character `pipe` works on: its last snapshot, the gear and expertise the Build Planner
/// starts from, the switch skills that are on, the summoned demon and the digitalize demon (each only when its details
/// were read). From saved files only: no snapshot, and none of the character's data is written.
pub fn player_build(pipe: &Pipeline, name: &str) -> Result<Value> {
    let store = pipe.store();
    let snaps = json::load(store.dash_dir().join("snapshots.json")).map_err(|_| no_snapshot())?;
    let last = json::arr(Some(&snaps))
        .filter(|r| r.is_object())
        .max_by(|a, b| json::str(a.at("taken_at")).unwrap_or_default().cmp(&json::str(b.at("taken_at")).unwrap_or_default()))
        .ok_or_else(no_snapshot)?;

    // the Build Planner's start: the worn pieces as the game last showed them, the expertise points, level, alignment
    let start = json::parse(&pipe.sim.init(None)?)?;
    let build = start.at("build");
    let mut gear = Vec::new();
    for (slot, piece) in json::arr(build.at("slots")).enumerate() {
        if json::int(piece.at("id")) <= 0 {
            continue;
        }
        gear.push(json!({
            "slot": slot, "id": json::int(piece.at("id")), "basic": json::int(piece.at("basic")), "special": json::int(piece.at("special")),
            "tarot": json::int(piece.at("tarot")).max(0), "soul": json::int(piece.at("soul")).max(0),
            "fuse": ints(piece.at("fuse"), 3), "mods": ints(piece.at("mods"), 5),
        }));
    }
    let mut points: Vec<(i32, i32)> = match build.at("points") {
        Some(Value::Object(m)) => m.iter().filter_map(|(k, v)| Some((crate::fmt::parse_i32(k)?, json::int(Some(v))))).filter(|&(_, p)| p > 0).collect(),
        _ => Vec::new(),
    };
    points.sort();
    let ranks = json::load(store.expertise_ranks_path()).unwrap_or(Value::Null);
    let cap = json::num(ranks.at("points").at("cap")).filter(|c| *c >= 0.0).map(|c| c as i64);
    let code_spec = json!({ "slots": build.at("slots"), "points": build.at("points"), "cap": cap });
    let code = json::str(json::parse(&pipe.sim.share(Some(&code_spec))?)?.at("code")).ok_or_else(|| anyhow::anyhow!("no build code"))?;

    let mut stats = Map::new();
    for s in STATS {
        stats.insert(s.to_string(), json!([json::int(last.at(&format!("{s}_base"))), json::int(last.at(&format!("{s}_bonus")))]));
    }
    let mut toggles = store.active_toggles()?;
    toggles.sort_unstable();
    toggles.dedup();
    toggles.truncate(MAX_TOGGLES);

    let demons = pipe.comp_demons(name, false)?;
    let partner = demons.iter().find(|d| d.summoned).and_then(demon_value);
    let digitalize = demons.iter().find(|d| d.current).and_then(demon_value);

    let level = json::int(start.at("meta").at("level"));
    let out = json!({
        "v": VERSION, "kind": "player",
        "takenAt": minute(&json::str(last.at("taken_at")).unwrap_or_default()),
        "level": level,
        "alignment": json::str(start.at("meta").at("alignment")).unwrap_or_else(|| "NEUTRAL".into()).to_uppercase(),
        "maxHp": json::int(last.at("max_hp")), "maxMp": json::int(last.at("max_mp")),
        "statsLive": json::truthy(last.at("stats_live")),
        "stats": stats,
        "gear": gear,
        "gearPartial": json::truthy(start.at("meta").at("gear_partial")),
        "expertise": points.iter().map(|&(id, p)| json!([id, p])).collect::<Vec<_>>(),
        "cap": cap,
        "toggles": toggles,
        "code": code,
        "partner": partner,
        "digitalize": digitalize,
    });
    check(&out).map_err(|why| anyhow::anyhow!("the player build came out wrong ({why}); nothing published"))?;
    Ok(out)
}

/// One demon's build: the demon of the list (`comp_demons`) with this key. Only a demon whose details were read (the
/// summoned one, or one whose status window was opened) has one.
pub fn demon_build(pipe: &Pipeline, name: &str, key: &str) -> Result<Value> {
    let demons = pipe.comp_demons(name, false)?;
    let d = demons.iter().find(|d| d.key == key).ok_or_else(|| user_error("That demon isn't in your demon list any more. Open the COMP and try again."))?;
    let demon = demon_value(d).ok_or_else(|| {
        user_error(format!("BABEL hasn't seen {}'s details yet. Summon it, or open its status window in the COMP, then try again.", d.name))
    })?;
    let out = json!({
        "v": VERSION, "kind": "demon",
        "seenAt": d.seen_at.as_deref().map(minute),
        "demon": demon,
    });
    check(&out).map_err(|why| anyhow::anyhow!("the demon build came out wrong ({why}); nothing published"))?;
    Ok(out)
}

/// The demons of the list that can be published (their details were read), with their keys.
pub fn publishable_demons(pipe: &Pipeline, name: &str) -> Result<Vec<CompDemon>> {
    Ok(pipe.comp_demons(name, false)?.into_iter().filter(|d| demon_value(d).is_some()).collect())
}

/// A demon as a build holds it, or None when only its kind and level are known.
fn demon_value(d: &CompDemon) -> Option<Value> {
    let stats = d.stats.as_ref().filter(|s| s.len() == 14)?;
    if !d.loaded || d.r#type <= 0 {
        return None;
    }
    let fixed = |v: &[i32], n: usize| -> Vec<i32> { (0..n).map(|i| v.get(i).copied().unwrap_or(0).max(0)).collect() };
    Some(json!({
        "type": d.r#type, "level": d.level, "growthType": d.growth_type.max(0),
        "reunion": fixed(&d.reunion, 12), "mitamaType": d.mitama_type.max(0), "mitamaRank": d.mitama_rank.max(0),
        "mitamaSlots": fixed(&d.mitama_slots, 96), "skills": fixed(&d.skills, 8),
        "forceStackIds": fixed(&d.force_stack_ids, 8), "equipmentIds": fixed(&d.equipment_ids, 4),
        "stats": stats,
    }))
}

fn no_snapshot() -> anyhow::Error {
    user_error("Take a snapshot first: a build is published from your character as the game last showed it.")
}

fn ints(v: Option<&Value>, n: usize) -> Vec<i32> {
    let a: Vec<i32> = json::arr(v).map(|x| json::int(Some(x)).max(0)).collect();
    (0..n).map(|i| a.get(i).copied().unwrap_or(0)).collect()
}

/// "2026-11-28T11:43:53" -> "2026-11-28T11:43" (what isn't a time becomes "").
fn minute(t: &str) -> String {
    let m = t.get(..16).unwrap_or("");
    if is_minute(m) { m.to_string() } else { String::new() }
}

fn is_minute(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 16
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 => *c == b':',
            _ => c.is_ascii_digit(),
        })
}

// ---------- the gate ----------

type Check = std::result::Result<(), String>;

/// Whether `v` is a build exactly as this version writes it (see the module's notes), with `Err(what is wrong)` when
/// not. Run before anything is published; the gallery Worker runs the same rules.
pub fn check(v: &Value) -> Check {
    let size = serde_json::to_string(v).map_err(|e| e.to_string())?.len();
    if size > MAX_BYTES {
        return Err(format!("{size} bytes, more than {MAX_BYTES}"));
    }
    let o = object(v, "the build")?;
    if o.get("v").and_then(Value::as_i64) != Some(VERSION) {
        return Err("not a version 1 build".into());
    }
    match o.get("kind").and_then(Value::as_str) {
        Some("player") => check_player(o),
        Some("demon") => check_demon_build(o),
        _ => Err("kind is neither player nor demon".into()),
    }
}

fn check_player(o: &Map<String, Value>) -> Check {
    keys(
        o,
        &["v", "kind", "takenAt", "level", "alignment", "maxHp", "maxMp", "statsLive", "stats", "gear", "gearPartial", "expertise", "cap", "toggles", "code", "partner", "digitalize"],
        &["about"],
        "the build",
    )?;
    time(&o["takenAt"], "takenAt")?;
    int(&o["level"], 1, 99, "level")?;
    match o["alignment"].as_str() {
        Some(a) if share_code::ALIGNMENTS.contains(&a) => {}
        _ => return Err("alignment is not one of the game's".into()),
    }
    int(&o["maxHp"], 0, MAX_STAT, "maxHp")?;
    int(&o["maxMp"], 0, MAX_STAT, "maxMp")?;
    boolean(&o["statsLive"], "statsLive")?;
    let stats = object(&o["stats"], "stats")?;
    keys(stats, &STATS, &[], "stats")?;
    for s in STATS {
        let pair = list(&stats[s], 2, 2, s)?;
        int(&pair[0], 0, MAX_STAT, s)?;
        int(&pair[1], -MAX_STAT, MAX_STAT, s)?;
    }
    let gear = list(&o["gear"], 0, share_code::SLOTS, "gear")?;
    let mut last_slot = -1;
    for p in gear {
        let p = object(p, "a piece")?;
        keys(p, &["slot", "id", "basic", "special", "tarot", "soul", "fuse", "mods"], &[], "a piece")?;
        let slot = int(&p["slot"], 0, share_code::SLOTS as i64 - 1, "a piece's slot")?;
        if slot <= last_slot {
            return Err("pieces not in slot order, or two in one slot".into());
        }
        last_slot = slot;
        for k in ["id", "basic", "special"] {
            int(&p[k], 1, share_code::MAX_NUMBER as i64, "a piece's id")?;
        }
        for k in ["tarot", "soul"] {
            int(&p[k], 0, share_code::MAX_NUMBER as i64, "a piece's crystal")?;
        }
        ints_in(&p["fuse"], 3, 0, MAX_FUSE, "a piece's fusion bonuses")?;
        ints_in(&p["mods"], 5, 0, 0xFFFF, "a piece's mods")?;
    }
    boolean(&o["gearPartial"], "gearPartial")?;
    let mut last_id = -1;
    for e in list(&o["expertise"], 0, share_code::MAX_EXPERTISES, "expertise")? {
        let pair = list(e, 2, 2, "an expertise")?;
        let id = int(&pair[0], 0, share_code::MAX_EXPERTISE_ID as i64, "an expertise id")?;
        if id <= last_id {
            return Err("expertises not in id order".into());
        }
        last_id = id;
        int(&pair[1], 1, MAX_POINTS, "expertise points")?;
    }
    if !o["cap"].is_null() {
        int(&o["cap"], 0, MAX_CAP, "cap")?;
    }
    let toggles = list(&o["toggles"], 0, MAX_TOGGLES, "toggles")?;
    for t in toggles {
        int(t, 1, MAX_ID, "a switch skill")?;
    }
    match o["code"].as_str() {
        Some(c) if c.len() <= share_code::MAX_CODE_CHARS && share_code::decode(c).is_ok() => {}
        _ => return Err("code is not a build code".into()),
    }
    for k in ["partner", "digitalize"] {
        if !o[k].is_null() {
            check_demon(&o[k], k)?;
        }
    }
    about(o)
}

fn check_demon_build(o: &Map<String, Value>) -> Check {
    keys(o, &["v", "kind", "seenAt", "demon"], &["about"], "the build")?;
    if !o["seenAt"].is_null() {
        time(&o["seenAt"], "seenAt")?;
    }
    check_demon(&o["demon"], "demon")?;
    about(o)
}

fn check_demon(v: &Value, what: &str) -> Check {
    let d = object(v, what)?;
    keys(d, &["type", "level", "growthType", "reunion", "mitamaType", "mitamaRank", "mitamaSlots", "skills", "forceStackIds", "equipmentIds", "stats"], &[], what)?;
    int(&d["type"], 1, MAX_ID, "a demon type")?;
    int(&d["level"], 1, 99, "a demon level")?;
    int(&d["growthType"], 0, MAX_ID, "a growth type")?;
    ints_in(&d["reunion"], 12, 0, 100, "rebirth ranks")?;
    int(&d["mitamaType"], 0, 16, "a Mitama type")?;
    int(&d["mitamaRank"], 0, 100, "a Mitama rank")?;
    ints_in(&d["mitamaSlots"], 96, 0, MAX_ID, "Mitama slots")?;
    ints_in(&d["skills"], 8, 0, MAX_ID, "demon skills")?;
    ints_in(&d["forceStackIds"], 8, 0, MAX_ID, "the force stack")?;
    ints_in(&d["equipmentIds"], 4, 0, MAX_ID, "demon equipment")?;
    ints_in(&d["stats"], 14, 0, MAX_STAT, "demon stats")
}

fn about(o: &Map<String, Value>) -> Check {
    let Some(a) = o.get("about") else { return Ok(()) };
    let a = object(a, "about")?;
    keys(a, &["name", "title", "note"], &[], "about")?;
    text(&a["name"], MAX_NAME, false, "the display name")?;
    text(&a["title"], MAX_TITLE, false, "the title")?;
    text(&a["note"], MAX_NOTE, true, "the note")
}

fn object<'a>(v: &'a Value, what: &str) -> std::result::Result<&'a Map<String, Value>, String> {
    v.as_object().ok_or_else(|| format!("{what} is not an object"))
}

/// Every key of `must`, any of `may`, and nothing else.
fn keys(o: &Map<String, Value>, must: &[&str], may: &[&str], what: &str) -> Check {
    if let Some(k) = must.iter().find(|k| !o.contains_key(**k)) {
        return Err(format!("{what} has no {k}"));
    }
    if let Some(k) = o.keys().find(|k| !must.contains(&k.as_str()) && !may.contains(&k.as_str())) {
        return Err(format!("{what} has an unknown field ({} characters)", k.chars().count()));
    }
    Ok(())
}

fn int(v: &Value, lo: i64, hi: i64, what: &str) -> std::result::Result<i64, String> {
    match v.as_i64() {
        Some(n) if (lo..=hi).contains(&n) => Ok(n),
        _ => Err(format!("{what} is not a whole number from {lo} to {hi}")),
    }
}

fn ints_in(v: &Value, n: usize, lo: i64, hi: i64, what: &str) -> Check {
    for x in list(v, n, n, what)? {
        int(x, lo, hi, what)?;
    }
    Ok(())
}

fn list<'a>(v: &'a Value, min: usize, max: usize, what: &str) -> std::result::Result<&'a Vec<Value>, String> {
    match v.as_array() {
        Some(a) if (min..=max).contains(&a.len()) => Ok(a),
        _ => Err(format!("{what} is not a list of {min} to {max}")),
    }
}

fn boolean(v: &Value, what: &str) -> Check {
    if v.is_boolean() { Ok(()) } else { Err(format!("{what} is not true or false")) }
}

fn time(v: &Value, what: &str) -> Check {
    match v.as_str() {
        Some(t) if is_minute(t) => Ok(()),
        _ => Err(format!("{what} is not a time")),
    }
}

/// Plain text the player typed: no control characters (line breaks only in the note), not only spaces, at most `max`
/// characters.
fn text(v: &Value, max: usize, lines: bool, what: &str) -> Check {
    let Some(s) = v.as_str() else { return Err(format!("{what} is not text")) };
    if s.chars().count() > max {
        return Err(format!("{what} is longer than {max} characters"));
    }
    if s.chars().any(|c| c.is_control() && !(lines && c == '\n')) || s.chars().any(|c| matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')) {
        return Err(format!("{what} has characters that aren't text"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demon() -> Value {
        json!({
            "type": 22, "level": 92, "growthType": 161, "reunion": vec![0; 12], "mitamaType": 0, "mitamaRank": 0,
            "mitamaSlots": vec![0; 96], "skills": [637, 0, 0, 0, 0, 0, 0, 0], "forceStackIds": [10144, 0, 0, 0, 0, 0, 0, 0],
            "equipmentIds": vec![0; 4], "stats": [2223, 1052, 144, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        })
    }

    fn player() -> Value {
        let mut slots: [Option<share_code::Piece>; share_code::SLOTS] = Default::default();
        slots[0] = Some(share_code::Piece { id: 2625, basic: 2625, special: 2625, tarot: 0, soul: 0, fuse: [0; 3], mods: [0; 5] });
        let code = share_code::encode(&share_code::Shared { level: 95, alignment: 0, cap: None, slots, points: vec![(3, 900_000)] }).unwrap();
        let stats: Map<String, Value> = STATS.iter().map(|s| (s.to_string(), json!([1, 2]))).collect();
        json!({
            "v": 1, "kind": "player", "takenAt": "2026-11-28T11:43", "level": 95, "alignment": "NEUTRAL",
            "maxHp": 709, "maxMp": 265, "statsLive": true, "stats": stats,
            "gear": [{ "slot": 0, "id": 2625, "basic": 2625, "special": 2625, "tarot": 0, "soul": 0, "fuse": [0, 0, 0], "mods": [0, 0, 0, 0, 0] }],
            "gearPartial": false, "expertise": [[3, 900_000]], "cap": 6_400_000, "toggles": [5450],
            "code": code, "partner": demon(), "digitalize": null,
        })
    }

    #[test]
    fn a_right_build_passes() {
        assert_eq!(check(&player()), Ok(()));
        assert_eq!(check(&json!({ "v": 1, "kind": "demon", "seenAt": "2026-11-24T04:08", "demon": demon() })), Ok(()));
        let mut p = player();
        p["about"] = json!({ "name": "Rubito", "title": "Gun build", "note": "Two lines\nof note" });
        assert_eq!(check(&p), Ok(()));
    }

    #[test]
    fn anything_extra_or_wrong_is_refused() {
        let bad = |f: &dyn Fn(&mut Value)| {
            let mut p = player();
            f(&mut p);
            check(&p).is_err()
        };
        // fields that aren't in the format: a path, a name, effect text, a total
        assert!(bad(&|p| p["dataDir"] = json!("C:\\Users\\Admin\\data")));
        assert!(bad(&|p| p["character"] = json!("Rubito")));
        assert!(bad(&|p| p["gear"][0]["effects"] = json!(["Critical +30"])));
        assert!(bad(&|p| p["stats"]["crit"] = json!([0, 5])));
        assert!(bad(&|p| p["partner"]["name"] = json!("Seraph")));
        // text where a number goes, out of range, wrong lengths
        assert!(bad(&|p| p["level"] = json!("95")));
        assert!(bad(&|p| p["level"] = json!(100)));
        assert!(bad(&|p| p["toggles"] = json!(["Shot Booster"])));
        assert!(bad(&|p| p["gear"][0]["mods"] = json!([0, 0, 0])));
        assert!(bad(&|p| p["gear"][0]["fuse"] = json!([51, 0, 0])));
        assert!(bad(&|p| p["partner"]["skills"] = json!([1, 2])));
        assert!(bad(&|p| p["alignment"] = json!("EVIL")));
        assert!(bad(&|p| p["takenAt"] = json!("yesterday")));
        assert!(bad(&|p| p["code"] = json!("BABEL1-not a code")));
        // two pieces in one slot, expertises out of order
        assert!(bad(&|p| {
            let g = p["gear"][0].clone();
            p["gear"].as_array_mut().unwrap().push(g);
        }));
        assert!(bad(&|p| p["expertise"] = json!([[4, 10], [3, 10]])));
        // the player's own text: limits and no control characters
        assert!(bad(&|p| p["about"] = json!({ "name": "x".repeat(25), "title": "", "note": "" })));
        assert!(bad(&|p| p["about"] = json!({ "name": "a\nb", "title": "", "note": "" })));
        assert!(bad(&|p| p["about"] = json!({ "name": "a\u{202E}b", "title": "", "note": "" })));
        assert!(bad(&|p| p["about"] = json!({ "name": "a", "title": "b", "note": "c", "link": "https://x" })));
        assert!(check(&json!({ "v": 2, "kind": "player" })).is_err());
        assert!(check(&json!([1, 2])).is_err());
    }

    /// The only strings a build can carry are the alignment, the times, the build code and the player's own `about`
    /// text: so no effect line (sealed or not), name or path can be in one.
    #[test]
    fn the_only_strings_are_known_ones() {
        fn strings<'a>(v: &'a Value, path: String, out: &mut Vec<(String, &'a str)>) {
            match v {
                Value::String(s) => out.push((path, s)),
                Value::Array(a) => a.iter().for_each(|x| strings(x, format!("{path}[]"), out)),
                Value::Object(o) => o.iter().for_each(|(k, x)| strings(x, format!("{path}.{k}"), out)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        let p = player();
        strings(&p, String::new(), &mut found);
        let mut paths: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
        paths.sort_unstable();
        assert_eq!(paths, [".alignment", ".code", ".kind", ".takenAt"]);
    }

    #[test]
    fn times_are_cut_to_the_minute() {
        assert_eq!(minute("2026-11-28T11:43:53"), "2026-11-28T11:43");
        assert_eq!(minute("2026-11-28T11:43"), "2026-11-28T11:43");
        assert_eq!(minute("C:\\Users\\x"), "");
        assert_eq!(minute(""), "");
    }

    #[test]
    fn too_big_is_refused() {
        let mut p = player();
        p["about"] = json!({ "name": "a", "title": "b", "note": "c".repeat(MAX_NOTE) });
        assert_eq!(check(&p), Ok(()));
        let mut big = player();
        big["toggles"] = json!(vec![999_999; MAX_TOGGLES]);
        big["partner"]["mitamaSlots"] = json!(vec![999_999; 96]);
        assert_eq!(check(&big), Ok(()));
        assert!(serde_json::to_string(&big).unwrap().len() < MAX_BYTES);
    }
}
