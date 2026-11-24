//! Port of `src/Build/DemonDb.cs`.
//!
//! The Demon Database page's data (app\ui\demon-db.json) and its pictures (app\ui\demon-db-icons\): every demon of the
//! compendium with its stats, affinities, features (traits), skills, tarot and soul, and where it can be contracted.
//!
//! Which demons: the compendium (reference\devil-book.tsv, DevilBookData): one row per demon type, baseID1 = the demon
//! it is registered as. Its 363 distinct base demons are "the demons"; the other 5,000 DevilData records are enemy
//! versions (dungeon and boss copies with their own level and stats), which are folded into their base demon
//! (DevilData unionData.baseDemonID) for "where it spawns" and are not listed on their own.
//!
//! Inputs:
//!   decoded dir   DevilData.xml (devil) and ZoneData.xml (zone names), decoded from the IMAGINE client (smt-decoded\server)
//!   datastore     the server's zones and partials (ServerZone / ServerZonePartial: Spawn, SpawnGroup)
//!   client dir    Interface\tga, tga2: demon faces (reference\devil-icons.tsv) and skill icons (reference\skill-icons.tsv)
//!   reference     devil-book.tsv, enchants-game.tsv (tarot / soul), devil-growth.tsv (stats per level), skill-text.tsv
//!
//! Contract rule (comp_hack SkillManager talk): an enemy can join when its spawn is not a BOSS, its TalkResist is below
//! 100 and its TalkResults has the JOIN flag (1; default 3). Contracting gives the spawned demon type itself.
//! Spawns are public server data: IMAGINE's live server may differ.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use anyhow::anyhow;
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use roxmltree::Node;
use serde_json::{Map, Value, json};

use super::item_icons;
use super::xml::{self, elements, element, m, mem, obj, title};
use crate::Result;
use crate::console;
use crate::data::game_data::{GameData, reference_file};
use crate::data::json::{self, Escape};
use crate::data::stat_labels;
use crate::fmt;
use crate::text::cmp_invariant;

/// `Int`: 0 for nothing, a `0x` number as hex, else the number's integer part (`(int)double`), 0 when it isn't one.
/// (A bad hex number threw in C#; here it reads 0.)
fn int<S: AsRef<str>>(s: Option<S>) -> i32 {
    let Some(s) = s.as_ref().map(|s| s.as_ref()) else { return 0 };
    if s.trim().is_empty() {
        return 0;
    }
    let s = s.trim();
    if s.len() >= 2 && s[..2].eq_ignore_ascii_case("0x") {
        return u32::from_str_radix(&s[2..], 16).map(|v| v as i32).unwrap_or(0);
    }
    fmt::parse_f64(s).map(|d| d as i32).unwrap_or(0)
}

fn int_of(o: Option<Node>, name: &str) -> i32 {
    int(m(o, name).as_deref())
}

/// `Ints(member)`: the numbers of a member's `<element>` list.
fn ints(member: Option<Node>) -> Vec<i32> {
    member.map(|n| elements(n, "element").map(|e| int(Some(&xml::value(e)))).collect()).unwrap_or_default()
}

struct Devil {
    name: String,
    gender: String,
    lnc: i32,
    family: String,
    race: String,
    fusion_options: i32,
    base_id: i32,
    growth: i32,
    base_level: i32,
    skills: Vec<i32>,
    traits: Vec<i32>,
    learned: Vec<(i32, i32)>,
    correct: Vec<i32>,
    mitama_id: i32,
}

struct Spawn {
    base: i32,
    level: i32,
    moon: i32,
    join: bool,
    zone: String,
    season: Option<String>,
}

const SLOTS: [&str; 14] = ["Head", "Face", "Neck", "Top", "Arms", "Bottom", "Feet", "COMP", "Ring", "Earring", "Extra", "Back", "Talisman", "Weapon"];

/// "200710_halloween" -> "Halloween 2007". Partials under partials\<event>\ are switched on for that event.
fn season_name(folder: &str) -> String {
    let re = Regex::new(r"^(\d{4})(\d{2})_(.+)$").expect("season");
    let Some(c) = re.captures(folder) else { return title(folder) };
    let post = Regex::new(r"[_-]?post$").expect("post");
    let what = post.replace(&c[3], "").into_owned();
    let what = match what.as_str() {
        "xmas" => "Christmas".to_string(),
        "xmasnewyears" => "Christmas and New Year".to_string(),
        "newyears" => "New Year".to_string(),
        "whiteday" => "White Day".to_string(),
        w => title(&w.replace('_', " ")),
    };
    format!("{what} {}", &c[1])
}

/// One zone or partial file's object, read into plain values.
struct Top {
    rel: String,
    partial: bool,
    id: i32,
    dynamic_map_id: i32,
    dynamic_map_ids: Vec<i32>,
    /// (moon mask, spawn ids) of each spawn group that is not disabled
    groups: Vec<(i32, Vec<i32>)>,
    spawns: Vec<TopSpawn>,
}

struct TopSpawn {
    id: String,
    enemy_type: i32,
    level: Option<String>,
    category: Option<String>,
    talk_resist: Option<String>,
    talk_results: Option<String>,
}

fn read_top(rel: &str, top: Node) -> Top {
    let partial = top.attribute("name") == Some("ServerZonePartial");
    // the spawn groups a spawn is in, with their moon restriction (a disabled group never spawns)
    let mut groups = Vec::new();
    if let Some(sgs) = mem(Some(top), "SpawnGroups") {
        for sg in elements(sgs, "pair").map(|p| element(p, "value").and_then(|v| element(v, "object"))) {
            let r = obj(sg, "Restrictions");
            if r.is_some() && m(r, "Disabled").as_deref() == Some("true") {
                continue;
            }
            let moon = match (r, m(r, "MoonRestriction")) {
                (Some(_), Some(mr)) => int(Some(&mr)),
                _ => 0xFFFF,
            };
            let ids = mem(sg, "Spawns")
                .map(|s| elements(s, "pair").map(|p| int(element(p, "key").map(xml::value).as_deref())).collect())
                .unwrap_or_default();
            groups.push((moon, ids));
        }
    }
    let mut spawns = Vec::new();
    if let Some(ss) = mem(Some(top), "Spawns") {
        for sp in elements(ss, "pair").map(|p| element(p, "value").and_then(|v| element(v, "object"))) {
            spawns.push(TopSpawn {
                id: m(sp, "ID").unwrap_or_default(),
                enemy_type: int_of(sp, "EnemyType"),
                level: m(sp, "Level"),
                category: m(sp, "Category"),
                talk_resist: m(sp, "TalkResist"),
                talk_results: m(sp, "TalkResults"),
            });
        }
    }
    Top {
        rel: rel.to_string(),
        partial,
        id: int_of(Some(top), "ID"),
        dynamic_map_id: int_of(Some(top), "DynamicMapID"),
        dynamic_map_ids: ints(mem(Some(top), "DynamicMapIDs")),
        groups,
        spawns,
    }
}

type Enchant = (i32, String, i32, i32, String);

pub fn build(data: &GameData, decoded: &Path, store: &Path, client_dir: Option<&Path>, out_file: &Path) -> Result<()> {
    let sw = Instant::now();
    let ms = || sw.elapsed().as_millis();
    let reference = |f: &str| reference_file(f).ok_or_else(|| anyhow!("reference\\{f} is missing."));

    // ---- every demon type (DevilData) ----
    let mut devils: HashMap<i32, Devil> = HashMap::new();
    {
        let text = xml::read_text(&decoded.join("DevilData.xml"))?;
        let doc = xml::parse(&text)?;
        for o in xml::records(&doc) {
            let b = obj(Some(o), "basic");
            let c = obj(Some(o), "category");
            let u = obj(Some(o), "unionData");
            let g = obj(Some(o), "growth");
            let bd = obj(Some(o), "battleData");
            let id = int_of(b, "ID");
            let learned: Vec<(i32, i32)> = mem(g, "acquisitionSkills")
                .map(|a| {
                    elements(a, "element")
                        .map(|e| element(e, "object"))
                        .map(|x| (int_of(x, "ID"), int_of(x, "level")))
                        .filter(|x| x.0 > 0)
                        .collect()
                })
                .unwrap_or_default();
            devils.insert(
                id,
                Devil {
                    name: m(b, "name").unwrap_or_default(),
                    gender: m(b, "gender").unwrap_or_default(),
                    lnc: int_of(b, "LNC"),
                    family: m(c, "family").unwrap_or_default(),
                    race: m(c, "race").unwrap_or_default(),
                    fusion_options: int_of(u, "fusionOptions"),
                    base_id: int_of(u, "baseDemonID"),
                    growth: int_of(g, "growthType"),
                    base_level: int_of(g, "baseLevel"),
                    skills: ints(mem(g, "skills")).into_iter().filter(|x| *x > 0).collect(),
                    traits: ints(mem(g, "traits")),
                    learned,
                    correct: ints(mem(bd, "correct")),
                    mitama_id: int_of(u, "mitamaFusionID"), // the demon it becomes when Mitama-fused (its own record: features, stats, affinities)
                },
            );
        }
    }
    console::write_line(&format!("{} demon types read ({} ms)", devils.len(), ms()));

    // ---- the compendium: demon type -> the demon it is registered as ----
    let mut book_base: HashMap<i32, i32> = HashMap::new();
    let mut book_group: HashMap<i32, i32> = HashMap::new();
    for line in reference("devil-book.tsv")?.lines()? {
        if line.starts_with('#') {
            continue;
        }
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 6 {
            continue;
        }
        book_base.insert(int(Some(p[0])), int(Some(p[2])));
        if int(Some(p[0])) == int(Some(p[2])) {
            book_group.insert(int(Some(p[2])), int(Some(p[4])));
        }
    }
    let bases: HashSet<i32> = book_base.values().copied().filter(|id| devils.get(id).is_some_and(|d| !d.name.trim().is_empty())).collect();
    let base_of = |ty: i32| -> i32 {
        if bases.contains(&ty) {
            return ty;
        }
        if let Some(d) = devils.get(&ty) {
            if bases.contains(&d.base_id) {
                return d.base_id;
            }
        }
        match book_base.get(&ty) {
            Some(b) if bases.contains(b) => *b,
            _ => 0,
        }
    };

    // ---- zone names (client) ----
    let mut zone_name: HashMap<i32, String> = HashMap::new();
    let mut open_zones: HashSet<String> = HashSet::new(); // the open world (the client's zone type FIELD): where most players contract
    {
        let text = xml::read_text(&decoded.join("ZoneData.xml"))?;
        let doc = xml::parse(&text)?;
        for z in elements(doc.root_element(), "object") {
            let b = obj(Some(z), "basic");
            if b.is_some() && m(b, "name").is_some_and(|n| !n.trim().is_empty()) {
                zone_name.insert(int_of(b, "id"), m(b, "name").unwrap_or_default());
            }
            if b.is_some() && m(b, "type").as_deref() == Some("FIELD") && m(b, "name").is_some_and(|n| !n.trim().is_empty()) {
                open_zones.insert(m(b, "name").unwrap_or_default());
            }
        }
    }

    // ---- spawns: every zone and partial file of the server data ----
    let mut files = Vec::new();
    for d in ["zones", "partials"] {
        let dir = store.join(d);
        if dir.is_dir() {
            files.extend(xml::enumerate_xml_files(&dir, true)?);
        }
    }
    let mut tops: Vec<Top> = Vec::new();
    for f in &files {
        let Some(text) = xml::loadable_text(f)? else { continue };
        let doc = xml::parse(&text)?;
        let rel = f.strip_prefix(store).unwrap_or(f).to_string_lossy().replace('\\', "/");
        for top in elements(doc.root_element(), "object").filter(|o| matches!(o.attribute("name"), Some("ServerZone" | "ServerZonePartial"))) {
            tops.push(read_top(&rel, top));
        }
    }
    // a partial has no name of its own: it is laid over the zones that use one of its dynamic maps
    let mut dyn_zones: IndexMap<i32, IndexSet<String>> = IndexMap::new();
    for top in tops.iter().filter(|t| !t.partial) {
        if let Some(zn) = zone_name.get(&top.id) {
            if top.dynamic_map_id > 0 {
                dyn_zones.entry(top.dynamic_map_id).or_default().insert(zn.clone());
            }
        }
    }
    let trace = std::env::var("DEMONDB_TRACE").ok();
    let mut spawns: Vec<Spawn> = Vec::new();
    let mut unplaced = 0;
    for top in &tops {
        let zones: Vec<String> = if top.partial {
            let mut seen = IndexSet::new();
            for mid in &top.dynamic_map_ids {
                if let Some(s) = dyn_zones.get(mid) {
                    seen.extend(s.iter().cloned());
                }
            }
            seen.into_iter().collect()
        } else {
            zone_name.get(&top.id).map(|zn| vec![zn.clone()]).unwrap_or_default()
        };
        if zones.is_empty() {
            continue; // a zone the client has no name for, or a partial applied by other rules
        }
        let season = if top.rel.starts_with("partials/") && top.rel.split('/').count() > 2 { Some(season_name(top.rel.split('/').nth(1).unwrap_or(""))) } else { None };
        // the spawn groups a spawn is in, with their moon restriction (a disabled group never spawns)
        let mut groups_of: HashMap<i32, Vec<i32>> = HashMap::new(); // spawn id -> moon masks of its groups
        for (moon, ids) in &top.groups {
            for k in ids {
                groups_of.entry(*k).or_default().push(*moon);
            }
        }
        for sp in &top.spawns {
            let ty = sp.enemy_type;
            let b = base_of(ty);
            if b == 0 {
                continue;
            }
            // only a spawn some group places can appear (a partial may fill groups of its base zone: kept)
            let moons = match groups_of.get(&int(Some(&sp.id))) {
                Some(m) => m.clone(),
                None => {
                    if !top.partial {
                        unplaced += 1;
                        continue;
                    }
                    vec![0xFFFF]
                }
            };
            let level = match &sp.level {
                Some(lv) if int(Some(lv)) > 0 => int(Some(lv)),
                _ => devils.get(&ty).map(|d| d.base_level).unwrap_or(0),
            };
            let join = sp.category.as_deref() != Some("BOSS")
                && int(sp.talk_resist.as_deref()) < 100
                && (sp.talk_results.as_deref().map(|tr| int(Some(tr))).unwrap_or(3) & 1) != 0;
            let moon = moons.iter().fold(0, |a, x| a | x);
            for z in &zones {
                spawns.push(Spawn { base: b, level, join, zone: z.clone(), season: season.clone(), moon });
            }
            if let Some(tw) = &trace {
                if devils.get(&b).is_some_and(|d| &d.name == tw) {
                    let shown: Vec<&str> = zones.iter().take(4).map(String::as_str).collect();
                    console::write_line(&format!(
                        "  trace {} spawn {} type {ty} Lv{level} join={join} cat={} talk={}/{} -> {}{}",
                        top.rel,
                        sp.id,
                        sp.category.as_deref().unwrap_or(""),
                        sp.talk_resist.as_deref().unwrap_or(""),
                        sp.talk_results.as_deref().unwrap_or(""),
                        shown.join(", "),
                        if zones.len() > 4 { format!(" +{}", zones.len() - 4) } else { String::new() }
                    ));
                }
            }
        }
    }
    console::write_line(&format!(
        "{} spawns of compendium demons in {} zone files ({unplaced} spawns no group places, skipped; {} ms)",
        spawns.len(),
        tops.len(),
        ms()
    ));

    // ---- tarot and soul (client EnchantData via reference\enchants-game.tsv) ----
    let mut enchants: HashMap<(i32, String), Enchant> = HashMap::new();
    for line in reference("enchants-game.tsv")?.lines()? {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 10 {
            continue;
        }
        enchants
            .entry((int(Some(p[1])), p[3].to_string()))
            // the description is the rest of the line: a few hold tabs (indented lines in the game's text, e.g. Alciel's
            // tarot "While Digitalized: / <tab>Demon Partner's Pursuit..."), which split it into more columns
            .or_insert_with(|| (int(Some(p[2])), p[4].to_string(), int(Some(p[5])), int(Some(p[6])), p[9..].concat()));
    }
    let does_nothing = Regex::new("(?i)does nothing").expect("does nothing");
    let enchant = |demon: i32, side: &str| -> Result<Value> {
        let Some(e) = enchants.get(&(demon, side.to_string())) else { return Ok(Value::Null) };
        let mut lines: Vec<String> = e.4.split(" / ").map(|x| x.trim_end().to_string()).collect();
        // the soul's first line repeats where it goes ("- Ring / Earring / Extra -" comes out split on " / ")
        if !lines.is_empty() && lines[0].trim_start().starts_with("- ") {
            if let Some(end) = lines.iter().position(|x| x.trim_end().ends_with(" -") || x.trim() == "-") {
                lines = lines.split_off(end + 1);
            }
        }
        let mut out_lines: Vec<String> = Vec::new();
        for l in &lines {
            if l.trim().is_empty() {
                continue;
            }
            out_lines.push(format!("{}{}", if l.starts_with(' ') { "  " } else { "" }, l.trim()));
        }
        if out_lines.len() == 1 && does_nothing.is_match(&out_lines[0]) {
            out_lines = vec!["No effect".to_string()];
        }
        let name = e.1.replace('@', "").trim().to_string();
        let slots: Vec<&str> = (0..SLOTS.len()).filter(|i| (e.3 & (1 << i)) != 0).map(|i| SLOTS[i]).collect();
        let slots_shown: Vec<&str> =
            if (slots.len() == SLOTS.len() - 1 && !slots.contains(&"COMP")) || slots.len() == SLOTS.len() { vec!["Any gear"] } else { slots };
        let mut r = Map::new();
        r.insert("name".into(), json!(name));
        r.insert("prefix".into(), json!(!e.1.starts_with('@') && !e.1.trim_start().starts_with("of ")));
        r.insert("lines".into(), json!(out_lines));
        r.insert("level".into(), json!(e.2));
        r.insert("slots".into(), json!(slots_shown));
        r.insert("crystal".into(), json!(data.item_name(e.0)?.unwrap_or_default()));
        Ok(Value::Object(r))
    };

    // ---- icons ----
    let out_dir = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default();
    let icon_dir = out_dir.join("demon-db-icons");
    std::fs::create_dir_all(&icon_dir)?;
    let mut icon_done: HashMap<String, Option<String>> = HashMap::new();
    let mut picture = |asset: &str, folder: Option<&str>, file: &str| -> Result<Option<String>> {
        let Some(client) = client_dir else { return Ok(None) };
        if asset.is_empty() {
            return Ok(None);
        }
        if let Some(have) = icon_done.get(file) {
            return Ok(have.clone());
        }
        let mut res = None;
        let dest = icon_dir.join(file);
        if dest.is_file() {
            res = Some(format!("demon-db-icons/{file}"));
        } else {
            let folders: Vec<&str> = folder.into_iter().collect();
            if let Some(png) = item_icons::client_png(client, asset, &folders) {
                std::fs::write(&dest, png)?;
                res = Some(format!("demon-db-icons/{file}"));
            }
        }
        icon_done.insert(file.to_string(), res.clone());
        Ok(res)
    };
    if let Some(client) = client_dir {
        let tga_dir = client.join("Interface").join("tga");
        let skull = tga_dir.join("icon_face_skull.tga");
        if tga_dir.is_dir() && !icon_dir.join("none.png").is_file() && skull.is_file() {
            if let Some(png) = crate::data::tga::to_png(&std::fs::read(&skull)?) {
                std::fs::write(icon_dir.join("none.png"), png)?;
            }
        }
    }

    let mut skills_seen: IndexMap<i32, Map<String, Value>> = IndexMap::new();
    let mut skill_ref = |id: i32, picture: &mut dyn FnMut(&str, Option<&str>, &str) -> Result<Option<String>>| -> Result<i32> {
        if !skills_seen.contains_key(&id) {
            let mut s = Map::new();
            s.insert("name".into(), json!(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| "A skill the game data does not name".to_string())));
            s.insert("desc".into(), json!(data.skill_text(id)?.unwrap_or_default()));
            let icon = match data.skill_icon_asset(id)? {
                Some(a) => picture(&a, None, &format!("{a}.png"))?,
                None => None,
            };
            s.insert("icon".into(), json!(icon));
            skills_seen.insert(id, s);
        }
        Ok(id)
    };

    // ---- the demons ----
    let mut growth_used: BTreeSet<i32> = BTreeSet::new();
    let mut list: Vec<Map<String, Value>> = Vec::new();
    let mut by_base: HashMap<i32, Vec<usize>> = HashMap::new();
    for (i, s) in spawns.iter().enumerate() {
        by_base.entry(s.base).or_default().push(i);
    }
    let mut base_ids: Vec<i32> = bases.iter().copied().collect();
    base_ids.sort();
    const STAT_IDX: [usize; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 12, 13, 14, 15, 19, 20];
    const CRIT_IDX: usize = 16;
    let pad = |v: &[i32]| -> Vec<i32> {
        let mut c = v.to_vec();
        if c.len() < 126 {
            c.resize(126, 0);
        }
        c
    };
    let skill_pairs = |ids: &[i32], learned: &[(i32, i32)], skill_ref: &mut dyn FnMut(i32) -> Result<i32>| -> Result<Vec<Value>> {
        let mut res = Vec::new();
        for s in ids {
            res.push(json!([skill_ref(*s)?, 0]));
        }
        let mut sorted = learned.to_vec();
        sorted.sort_by_key(|l| l.1);
        for l in &sorted {
            res.push(json!([skill_ref(l.0)?, l.1]));
        }
        Ok(res)
    };
    for id in base_ids {
        let d = &devils[&id];
        let c = pad(&d.correct);
        growth_used.insert(d.growth);
        // Mitama-fused, a demon becomes another demon type (unionData.mitamaFusionID, e.g. Metatron 21 -> 10021) with
        // its own features, stats and affinities. Skills only when they differ from the demon's own.
        let mut mt: Value = Value::Null;
        if d.mitama_id > 0 && d.mitama_id != id {
            if let Some(md) = devils.get(&d.mitama_id) {
                let mc = pad(&md.correct);
                growth_used.insert(md.growth);
                let mut dl = d.learned.clone();
                dl.sort_by_key(|l| l.1);
                let mut ml = md.learned.clone();
                ml.sort_by_key(|l| l.1);
                let same_skills = md.skills == d.skills && ml == dl;
                let mut t = Map::new();
                t.insert("level".into(), json!(md.base_level));
                t.insert("growth".into(), json!(md.growth));
                t.insert("stats".into(), json!(STAT_IDX.iter().map(|&i| mc[i]).collect::<Vec<_>>()));
                t.insert("crit".into(), json!(mc[CRIT_IDX]));
                t.insert("res".into(), json!((2..23).map(|i| mc[26 + i]).collect::<Vec<_>>()));
                t.insert("nra".into(), json!((2..23).map(|i| mc[49 + i]).collect::<Vec<_>>()));
                let mut traits = Vec::new();
                for tr in md.traits.iter().filter(|t| **t > 0) {
                    traits.push(skill_ref(*tr, &mut picture)?);
                }
                t.insert("traits".into(), json!(traits));
                t.insert(
                    "skills".into(),
                    if same_skills {
                        Value::Null
                    } else {
                        let mut r = |x: i32| skill_ref(x, &mut picture);
                        Value::Array(skill_pairs(&md.skills, &md.learned, &mut r)?)
                    },
                );
                mt = Value::Object(t);
            }
        }
        let mine: Vec<&Spawn> = by_base.get(&id).map(|v| v.iter().map(|&i| &spawns[i]).collect()).unwrap_or_default();
        // contract: the join spawns by (zone, season), in order of first appearance
        let mut groups: IndexMap<(String, Option<String>), Vec<&Spawn>> = IndexMap::new();
        for s in mine.iter().filter(|s| s.join) {
            groups.entry((s.zone.clone(), s.season.clone())).or_default().push(s);
        }
        struct Contract {
            zone: String,
            min: i32,
            max: i32,
            moon: i32,
            season: Option<String>,
            field: bool,
        }
        let mut contract: Vec<Contract> = groups
            .iter()
            .map(|((zone, season), g)| {
                let m = g.iter().fold(0, |a, s| a | s.moon);
                Contract {
                    zone: zone.clone(),
                    min: g.iter().map(|s| s.level).min().unwrap_or(0),
                    max: g.iter().map(|s| s.level).max().unwrap_or(0),
                    moon: if m != 0xFFFF && m != 0 { m } else { 0 },
                    season: season.clone(),
                    field: open_zones.contains(zone),
                }
            })
            .collect();
        contract.sort_by(|a, b| {
            a.season.is_some().cmp(&b.season.is_some()).then((!a.field).cmp(&(!b.field))).then(a.min.cmp(&b.min)).then(cmp_invariant(&a.zone, &b.zone))
        });
        let join_zones: HashSet<&str> = contract.iter().filter(|x| x.season.is_none()).map(|x| x.zone.as_str()).collect();
        let mut enemy_groups: IndexMap<&str, Vec<&Spawn>> = IndexMap::new();
        for s in mine.iter().filter(|s| !s.join && s.season.is_none() && !join_zones.contains(s.zone.as_str())) {
            enemy_groups.entry(s.zone.as_str()).or_default().push(s);
        }
        let mut enemy: Vec<(String, i32, i32)> = enemy_groups
            .iter()
            .map(|(z, g)| (z.to_string(), g.iter().map(|s| s.level).min().unwrap_or(0), g.iter().map(|s| s.level).max().unwrap_or(0)))
            .collect();
        enemy.sort_by_key(|x| x.1);

        let mut e = Map::new();
        e.insert("id".into(), json!(id));
        e.insert("name".into(), json!(d.name));
        e.insert("race".into(), json!(title(&d.race)));
        e.insert("family".into(), json!(title(&d.family)));
        e.insert("level".into(), json!(d.base_level));
        e.insert("align".into(), json!(if d.lnc >= 5000 { "Chaos" } else if d.lnc <= -5000 { "Law" } else { "Neutral" }));
        e.insert("lnc".into(), json!(d.lnc));
        e.insert("gender".into(), json!(title(&d.gender)));
        e.insert("growth".into(), json!(d.growth));
        e.insert("fusion".into(), json!((d.fusion_options & 0x02) != 0));
        e.insert("mitama".into(), json!((d.fusion_options & 0x08) != 0));
        e.insert("counted".into(), json!(book_group.get(&id).copied().unwrap_or(0) != 0));
        let icon = match data.devil_icon_file(id)? {
            Some((name, folder)) => picture(&name, Some(&folder), &format!("d{id}.png"))?,
            None => None,
        };
        e.insert("icon".into(), json!(icon));
        e.insert("stats".into(), json!(STAT_IDX.iter().map(|&i| c[i]).collect::<Vec<_>>()));
        // Critical (CorrectTbl::CRITICAL): fixed, it does not grow with level. A crit roll adds Luck to it (SkillManager).
        e.insert("crit".into(), json!(c[CRIT_IDX]));
        e.insert("res".into(), json!((2..23).map(|i| c[26 + i]).collect::<Vec<_>>()));
        e.insert("nra".into(), json!((2..23).map(|i| c[49 + i]).collect::<Vec<_>>()));
        let mut traits = Vec::new();
        for tr in d.traits.iter().filter(|t| **t > 0) {
            traits.push(skill_ref(*tr, &mut picture)?);
        }
        e.insert("traits".into(), json!(traits));
        let mut r = |x: i32| skill_ref(x, &mut picture);
        e.insert("skills".into(), Value::Array(skill_pairs(&d.skills, &d.learned, &mut r)?));
        e.insert("tarot".into(), enchant(id, "tarot")?);
        e.insert("soul".into(), enchant(id, "soul")?);
        e.insert(
            "contract".into(),
            Value::Array(
                contract
                    .iter()
                    .map(|x| json!({ "zone": x.zone, "min": x.min, "max": x.max, "moon": x.moon, "season": x.season, "field": x.field }))
                    .collect(),
            ),
        );
        e.insert("enemy".into(), Value::Array(enemy.iter().map(|x| json!({ "zone": x.0, "min": x.1, "max": x.2 })).collect()));
        e.insert("mt".into(), mt);
        list.push(e);
    }

    let mut result = Map::new();
    result.insert("generatedAt".into(), json!(chrono::Utc::now().format("%Y-%m-%d").to_string()));
    result.insert("note".into(), json!("Demons, stats, skills, tarot and soul: IMAGINE client. Spawns: public server data, not IMAGINE's live server."));
    result.insert(
        "statNames".into(),
        json!(["STR", "MAG", "VIT", "INT", "SPD", "LUCK", "Max HP", "Max MP", "CLS-R", "LNG-R", "Spell", "Support", "P.Def", "M.Def"]),
    );
    let elements_text: Vec<String> = (2..23)
        .map(|i| {
            let id = stat_labels::ELEMENT_IDS[i].unwrap_or("");
            stat_labels::element(id).unwrap_or_else(|| title(id))
        })
        .collect();
    result.insert("elements".into(), json!(elements_text));
    let mut growth = Map::new();
    for g in growth_used.iter().filter(|g| data.demons().is_ok_and(|t| t.growth.contains_key(&**g))) {
        growth.insert(g.to_string(), json!(data.demons()?.growth[g]));
    }
    result.insert("growth".into(), Value::Object(growth));
    result.insert("skills".into(), Value::Object(skills_seen.iter().map(|(k, v)| (k.to_string(), Value::Object(v.clone()))).collect()));
    result.insert("noIcon".into(), json!(if icon_dir.join("none.png").is_file() { "demon-db-icons/none.png" } else { "" }));
    result.insert("demons".into(), Value::Array(list.iter().cloned().map(Value::Object).collect()));
    std::fs::write(out_file, json::serialize(&Value::Object(result), Escape::Relaxed, false))?;
    let with_join = list.iter().filter(|x| x["contract"].as_array().is_some_and(|c| !c.is_empty())).count();
    console::write_line(&format!(
        "{} demons, {with_join} can be contracted somewhere, {} with a Mitama form, {} with a picture, {} skills. Written to {} ({} KB, {} ms)",
        list.len(),
        list.iter().filter(|x| !x["mt"].is_null()).count(),
        list.iter().filter(|x| !x["icon"].is_null()).count(),
        skills_seen.len(),
        out_file.display(),
        std::fs::metadata(out_file)?.len() / 1024,
        ms()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_names() {
        assert_eq!(season_name("200710_halloween"), "Halloween 2007");
        assert_eq!(season_name("200812_xmasnewyears_post"), "Christmas and New Year 2008");
        assert_eq!(season_name("201003_whiteday"), "White Day 2010");
        assert_eq!(season_name("201112_some_event-post"), "Some Event 2011");
        assert_eq!(season_name("misc"), "Misc");
    }

    #[test]
    fn int_reads_numbers_the_way_the_csharp_did() {
        assert_eq!(int(None::<&str>), 0);
        assert_eq!(int(Some("  ")), 0);
        assert_eq!(int(Some(" 42 ")), 42);
        assert_eq!(int(Some("-5000")), -5000);
        assert_eq!(int(Some("0x1F")), 31);
        assert_eq!(int(Some("0xFFFFFFFF")), -1);
        assert_eq!(int(Some("3.9")), 3);
        assert_eq!(int(Some("-3.9")), -3);
        assert_eq!(int(Some("1e3")), 1000);
        assert_eq!(int(Some("abc")), 0);
    }

    #[test]
    fn a_zone_file_is_read_into_plain_values() {
        let text = r#"<objects><object name="ServerZone">
            <member name="ID">1</member><member name="DynamicMapID">5</member>
            <member name="SpawnGroups"><pair><key>1</key><value><object name="SpawnGroup">
              <member name="Restrictions"><object name="SpawnRestriction"><member name="MoonRestriction">3</member></object></member>
              <member name="Spawns"><pair><key>10</key><value>1</value></pair></member></object></value></pair>
              <pair><key>2</key><value><object name="SpawnGroup">
              <member name="Restrictions"><object name="SpawnRestriction"><member name="Disabled">true</member></object></member>
              <member name="Spawns"><pair><key>11</key><value>1</value></pair></member></object></value></pair></member>
            <member name="Spawns"><pair><key>10</key><value><object name="Spawn"><member name="ID">10</member><member name="EnemyType">7</member><member name="Level">12</member></object></value></pair></member>
          </object></objects>"#;
        let doc = xml::parse(text).unwrap();
        let top = read_top("zones/a.xml", doc.root_element().first_element_child().unwrap());
        assert!(!top.partial);
        assert_eq!((top.id, top.dynamic_map_id), (1, 5));
        assert_eq!(top.groups, vec![(3, vec![10])]); // the disabled group is left out
        assert_eq!(top.spawns.len(), 1);
        assert_eq!((top.spawns[0].enemy_type, top.spawns[0].level.as_deref()), (7, Some("12")));
    }
}
