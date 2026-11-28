//! Port of `src/Build/Dungeons.cs`.
//!
//! The Dungeon Database page's data (app\ui\dungeons.json): every dungeon and field, and the enemies in it that can't
//! be contracted, with what the game data says about each (level, stats, affinities, skills, traits, drops).
//!
//! Where things come from:
//!   server datastore  data\zoneinstance (a dungeon = an instance: its areas in order and the zone you enter it from),
//!                     zones\*.xml (each area's spawns: enemy, level, talk resist, drops), zones\partial\spawns (more
//!                     spawns laid over areas by their dynamic map), data\dropset (drop sets). This is the public
//!                     server data, not IMAGINE's live server.
//!   IMAGINE client    zone.xml (ZoneData: area names and kinds), DevilData.xml (names, race, base stats, growth,
//!                     skills, enemy-only skills, traits, extra enemy HP), citem.tsv (item names)
//!   reference\        devil-growth.tsv (level-up table), skill names and descriptions (GameData), devil icons
//!
//! "Can't be contracted" is the server's own rule (comp_hack SkillManager): talking fails when the spawn's TalkResist is
//! 100 or more, and an enemy whose TalkResults lacks the "join" flag (1) never joins.
//!
//! Stats are the server's for an enemy (ZoneManager::CreateEnemy): the demon's base stats grown to the spawn's level
//! (GetDemonBaseStats), the demon HP/MP and combat formulas (CalculateDependentStats, demon branch), plus the
//! enemy-only HP of DevilData (ActiveEntityState: added to every enemy). Passive skills and traits are listed, not
//! added in: what they do depends on the fight.
//!
//! The C# kept `XElement`s of spawns, chests and overlays; here each is read into plain values once (the chests and
//! spawns in two arenas, so that what the C# shared by reference is shared by index).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::anyhow;
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use roxmltree::Node;
use serde_json::{Map, Value, json};

use super::item_icons;
use super::xml::{self, elements, element, m, mem, obj};
use crate::Result;
use crate::console;
use crate::data::game_data::GameData;
use crate::data::json::{self, Escape};
use crate::fmt;
use crate::text::{cmp_invariant, ic_key};

/// `Int`: 0 for nothing, the number, or (`(int)double.Parse`) the integer part of a decimal number; 0 when it is no number
/// (the C# threw there).
fn int<S: AsRef<str>>(s: Option<S>) -> i32 {
    let Some(s) = s.as_ref().map(|s| s.as_ref().trim()) else { return 0 };
    if s.is_empty() {
        return 0;
    }
    fmt::parse_i32(s).or_else(|| fmt::parse_f64(s).map(|d| d as i32)).unwrap_or(0)
}

fn int_of(o: Option<Node>, name: &str) -> i32 {
    int(m(o, name).as_deref())
}

/// `Dbl`.
fn dbl(s: Option<&str>) -> f64 {
    match s.map(str::trim) {
        Some(t) if !t.is_empty() => fmt::parse_f64(t).unwrap_or(0.0),
        _ => 0.0,
    }
}

fn ints(member: Option<Node>) -> Vec<i32> {
    member.map(|n| elements(n, "element").map(|e| int(Some(xml::value(e)))).collect()).unwrap_or_default()
}

/// The plain values (members and list elements without children) under an object: event ids, among others.
fn leaves(o: Node) -> Vec<String> {
    o.descendants()
        .filter(|e| e.is_element() && !e.children().any(|c| c.is_element()) && matches!(e.tag_name().name(), "member" | "element"))
        .map(|e| xml::value(e).trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// `Math.Round(x, MidpointRounding.AwayFromZero)` as an int.
fn round_away(x: f64) -> i32 {
    x.round() as i32
}

// CorrectTbl: RES_* (resist %, + takes less, - weak) at 26.., NRA_* (null/reflect/absorb, "percent x 10 + kind") at 49..
const AFFINITIES: [(usize, usize, &str); 21] = [
    (28, 51, "Slash"),
    (29, 52, "Thrust"),
    (30, 53, "Strike"),
    (31, 54, "Long-range"),
    (32, 55, "Pierce"),
    (33, 56, "Spread"),
    (34, 57, "Fire"),
    (35, 58, "Ice"),
    (36, 59, "Electric"),
    (37, 60, "Almighty"),
    (38, 61, "Force"),
    (39, 62, "Expel"),
    (40, 63, "Curse"),
    (41, 64, "Healing"),
    (42, 65, "Support"),
    (43, 66, "Magic force"),
    (44, 67, "Nerve"),
    (45, 68, "Mind"),
    (46, 69, "Word"),
    (47, 70, "Special"),
    (48, 71, "Suicide"),
];
const NRA_KINDS: [&str; 4] = ["", "Null", "Reflect", "Absorb"];

struct Devil {
    id: i32,
    name: String,
    race: Option<String>,
    growth_type: i32,
    base_level: i32,
    enemy_hp: i32,
    correct: Vec<i32>,
    skills: Vec<i32>,
    enemy_skills: Vec<i32>,
    traits: Vec<i32>,
}

type Drop = (i32, f64, i32);

#[derive(Clone)]
struct SpawnInfo {
    devil: i32,
    level: i32,
    talk_resist: i32,
    talk_results: i32,
    xp: i64,
    boss: bool,
    /// InheritDrops (default true): the enemy also gets its zone's drop sets and the game-wide ones (SkillManager).
    inherit: bool,
    drops: Vec<Drop>,
    drop_sets: Vec<i32>,
}

/// A chest an object makes (ActionCreateLoot).
struct Loot {
    drop_set_ids: Vec<i32>,
    drops: Vec<Drop>,
    is_boss_box: bool,
    /// a transform script makes the contents and its parameters do not list them
    made_by_script: bool,
    /// what a transform script puts in it when its parameters list it (transforma_addItemDropRandom: one of the items)
    script_drops: Vec<Drop>,
}

/// transforma_addItemDropRandom's parameters, (item, amount) pairs: one of them is given, each with 100 / n %.
fn random_drops(ps: &[i32]) -> Vec<Drop> {
    let n = ps.len() / 2;
    ps.chunks_exact(2).filter(|p| p[0] > 0).map(|p| (p[0], 100.0 / n as f64, p[1].max(1))).collect()
}

/// The items an event gives by script with their chance, as tools\dcmexport reads them ("a reward roll"):
/// action_flagRangeSet (params: [0] flag key, [1] values after the item per entry, [2] percent digits (0 = always), then
/// entries (item, values..., [chance]); a roll succeeds with chance / (10 x 10^digits)), each with its flag key: only the
/// keys an event turns into items count (`item_flag_keys`); and transforma_addItemDropRandom on any other action (one of
/// the listed items, key 0). Also the flag keys this event turns into items (ActionAddRemoveItems with
/// transforma_addItemFlagRange, its first parameter).
fn script_rewards(ev: Node) -> (Vec<(i32, Drop)>, Vec<i32>) {
    let (mut got, mut keys) = (Vec::new(), Vec::new());
    for o in ev.descendants().filter(|x| x.is_element() && x.tag_name().name() == "object") {
        let name = o.attribute("name").unwrap_or("");
        let run = name == "ActionRunScript";
        let script = m(Some(o), if run { "scriptID" } else { "transformScriptID" }).unwrap_or_default();
        let ps = ints(mem(Some(o), if run { "params" } else { "transformScriptParams" }));
        match script.as_str() {
            "action_flagRangeSet" if run && ps.len() >= 4 => {
                let digits = ps[2];
                let size = (1 + ps[1] + i32::from(digits >= 1)).max(1) as usize;
                let precision = 10.0 * 10f64.powi(digits);
                let mut i = 3;
                while i + size - 1 < ps.len() {
                    let rate = if digits >= 1 { (f64::from(ps[i + size - 1]) / precision * 100.0).min(100.0) } else { 100.0 };
                    if ps[i] > 0 && rate > 0.0 {
                        got.push((ps[0], (ps[i], rate, 1)));
                    }
                    i += size;
                }
            }
            "transforma_addItemFlagRange" if name == "ActionAddRemoveItems" && ps.first().is_some_and(|k| *k > 0) => keys.push(ps[0]),
            "transforma_addItemDropRandom" if name != "ActionCreateLoot" => got.extend(random_drops(&ps).into_iter().map(|d| (0, d))),
            _ => {}
        }
    }
    (got, keys)
}

/// One area: a zone file, i.e. a zone (Zone) as one of its map variants (Id = the dynamic map). The difficulties of a
/// dungeon share zone ids and differ by map (zone-540101_5401004 is Suginami Tunnels Bronze, _5421005 Silver), so
/// areas are kept per map, never merged by zone id. Its name and kind are the zone's; its spawns and drop sets the
/// file's, with the overlays for that map applied.
struct Area {
    id: usize,
    zone: i32,
    name: String,
    ty: Option<String>,
    /// spawn id -> index in the spawn arena
    spawns: IndexMap<i32, usize>,
    drop_sets: IndexSet<i32>,
    /// Every plain value in the zone file and its overlays (event ids among them): where its events start.
    refs: IndexSet<String>,
    /// Chests the zone file and its overlays make themselves (ActionCreateLoot outside any event); indexes of the chest arena.
    loot: Vec<usize>,
}

struct Arenas {
    spawns: Vec<SpawnInfo>,
    loot: Vec<Loot>,
}

/// A spawn as read from a file (C# `ReadSpawn`).
fn read_spawn(s: Node) -> Result<SpawnInfo> {
    let mut si = SpawnInfo {
        devil: int_of(Some(s), "EnemyType"),
        level: match m(Some(s), "Level") {
            Some(l) => int(Some(l)),
            None => -1,
        },
        talk_resist: int_of(Some(s), "TalkResist"),
        talk_results: match m(Some(s), "TalkResults") {
            Some(t) => int(Some(t)),
            None => 3,
        },
        xp: match m(Some(s), "XP") {
            Some(x) => x.parse::<i64>().map_err(|_| anyhow!("The input string '{x}' was not in a correct format."))?,
            None => -1,
        },
        boss: m(Some(s), "Category").as_deref() == Some("BOSS"),
        inherit: m(Some(s), "InheritDrops").as_deref() != Some("false"),
        drops: Vec::new(),
        drop_sets: Vec::new(),
    };
    si.drops = drops_of(mem(Some(s), "Drops"));
    si.drop_sets = ints(mem(Some(s), "DropSetIDs"));
    Ok(si)
}

/// The `<element><object>` drops of a member: (item, rate, max stack).
fn drops_of(member: Option<Node>) -> Vec<Drop> {
    member
        .map(|d| {
            elements(d, "element")
                .filter_map(|e| element(e, "object"))
                .map(|d| (int_of(Some(d), "ItemType"), dbl(m(Some(d), "Rate").as_deref()), int_of(Some(d), "MaxStack").max(1)))
                .collect()
        })
        .unwrap_or_default()
}

/// `SpawnsOf(top)`: (spawn id, spawn) of a zone or overlay object.
fn spawns_of(top: Node) -> Result<Vec<(i32, SpawnInfo)>> {
    let mut res = Vec::new();
    if let Some(sp) = mem(Some(top), "Spawns") {
        for p in elements(sp, "pair") {
            let key = int(element(p, "key").map(xml::value));
            let Some(o) = element(p, "value").and_then(|v| element(v, "object")) else { continue };
            res.push((key, read_spawn(o)?));
        }
    }
    Ok(res)
}

/// `LootIn(o)`: the chests an object makes, read into the arena; their indexes.
fn loot_in(o: Node, loot: &mut Vec<Loot>) -> Vec<usize> {
    let mut res = Vec::new();
    for x in o.descendants().skip(1).filter(|x| x.is_element() && x.tag_name().name() == "object" && x.attribute("name") == Some("ActionCreateLoot")) {
        let script = m(Some(x), "transformScriptID").unwrap_or_default();
        let script_drops = if script == "transforma_addItemDropRandom" { random_drops(&ints(mem(Some(x), "transformScriptParams"))) } else { Vec::new() };
        loot.push(Loot {
            drop_set_ids: ints(mem(Some(x), "dropSetIDs")),
            drops: drops_of(mem(Some(x), "drops")),
            is_boss_box: m(Some(x), "isBossBox").as_deref() == Some("true"),
            made_by_script: !script.is_empty() && script_drops.is_empty(),
            script_drops,
        });
        res.push(loot.len() - 1);
    }
    res
}

/// `Load(f)`: the file's text when it parses, a bare "&" fixed when that makes it; else a note on the error output.
fn load(f: &Path) -> Result<Option<String>> {
    let text = xml::read_text(f)?;
    if xml::parse(&text).is_ok() {
        return Ok(Some(text));
    }
    let fixed = xml::fix_bare_ampersands(&text);
    match xml::parse(&fixed) {
        Ok(_) => Ok(Some(fixed)),
        Err(e) => {
            console::error_line(&format!("skipped {}: {e}", f.display()));
            Ok(None)
        }
    }
}

struct Overlay {
    id: i32,
    dynamic_map_ids: Vec<i32>,
    drop_set_ids: Vec<i32>,
    leaves: Vec<String>,
    loot: Vec<usize>,
    spawns: Vec<(i32, SpawnInfo)>,
}

struct Instance {
    zones: Vec<i32>,
    maps: Vec<i32>,
    lobby: i32,
    msg: i32,
}

struct Dungeon {
    name: String,
    kind: &'static str,
    from: String,
    areas: Vec<usize>,
}

/// One enemy line of a place: its spawn (arena index), its mob key and the areas (indexes in the place) it is in.
struct Entry {
    spawn: usize,
    key: String,
    wher: BTreeSet<usize>,
}

pub fn build(data: &GameData, decoded: &Path, store: &Path, out_file: &Path, client_dir: Option<&Path>) -> Result<()> {
    // BABEL's English for the demon names the game has only in Japanese or garbled (reference\demon-names-en.tsv)
    let demon_en = crate::data::game_data::name_translations("demon-names-en.tsv")?;

    // ---- client: zone names and kinds ----
    let mut zones: HashMap<i32, (String, Option<String>)> = HashMap::new();
    {
        let text = xml::read_text(&decoded.join("zone.xml"))?;
        let doc = xml::parse(&text)?;
        for z in elements(doc.root_element(), "object") {
            let b = obj(Some(z), "basic");
            if b.is_some() {
                zones.insert(int_of(b, "id"), (m(b, "name").unwrap_or_default().trim().to_string(), m(b, "type")));
            }
        }
    }
    // ---- client: item names ----
    let mut item_name: HashMap<i32, String> = HashMap::new();
    {
        let text = xml::read_text(&decoded.join("citem.tsv"))?;
        for line in json::lines(&text) {
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() >= 2 {
                if let Some(id) = fmt::parse_i32(p[0]) {
                    if !p[1].trim().is_empty() {
                        item_name.insert(id, p[1].trim().to_string());
                    }
                }
            }
        }
    }

    // ---- server: zone files and partials ----
    // An area is a zone with one map: a key of its own (Id, a running number), found by (zone, map). Different zones
    // can share a map (Floor of Eternity 1061501 uses the map of 1061401), so neither one alone tells areas apart.
    let mut arenas = Arenas { spawns: Vec::new(), loot: Vec::new() };
    let mut areas: Vec<Area> = Vec::new(); // areas[id - 1]
    let mut area_of: HashMap<(i32, i32), usize> = HashMap::new(); // -> id
    let mut areas_of_map: HashMap<i32, Vec<usize>> = HashMap::new(); // an overlay names maps: it goes on every zone using one

    for f in xml::enumerate_xml_files(&store.join("zones"), false)? {
        let Some(text) = load(&f)? else { continue };
        let doc = xml::parse(&text)?;
        for top in elements(doc.root_element(), "object").filter(|o| o.attribute("name") == Some("ServerZone")) {
            let id = int_of(Some(top), "ID");
            let mut map = int_of(Some(top), "DynamicMapID");
            if map <= 0 {
                map = id;
            }
            let aid = match area_of.get(&(id, map)) {
                Some(a) => *a,
                None => {
                    let aid = areas.len() + 1;
                    areas.push(Area {
                        id: aid,
                        zone: id,
                        name: zones.get(&id).map(|z| z.0.clone()).unwrap_or_default(),
                        ty: zones.get(&id).and_then(|z| z.1.clone()),
                        spawns: IndexMap::new(),
                        drop_sets: IndexSet::new(),
                        refs: IndexSet::new(),
                        loot: Vec::new(),
                    });
                    area_of.insert((id, map), aid);
                    areas_of_map.entry(map).or_default().push(aid);
                    aid
                }
            };
            for (sid, s) in spawns_of(top)? {
                if !areas[aid - 1].spawns.contains_key(&sid) {
                    arenas.spawns.push(s);
                    let idx = arenas.spawns.len() - 1;
                    areas[aid - 1].spawns.insert(sid, idx);
                }
            }
            let a = &mut areas[aid - 1];
            a.refs.extend(leaves(top));
            a.loot.extend(loot_in(top, &mut arenas.loot));
            a.drop_sets.extend(ints(mem(Some(top), "DropSetIDs")));
        }
    }
    // Overlays (ServerZonePartial) laid over areas by dynamic map, applied as the server does
    // (comp_hack ServerDataManager::GetZoneData / ApplyZonePartial), in order of overlay id:
    //   - only overlays that list dynamic maps and are not AutoApply=false are put on a zone by themselves
    //     (partial 0 in global.xml has no maps: it is the game-wide drop list, not shown here);
    //   - its DropSetIDs are added to the zone's own (drops every enemy there shares);
    //   - a spawn that names an enemy replaces the zone's spawn of that id; one that doesn't adds its drops to it.
    // All of zones\partial counts, not only zones\partial\spawns (global.xml adds "Instance global drops" to 842
    // maps, empty in the public data). The seasonal overlays in partials\<event>\ are switched on by events: left out.
    let mut overlays: Vec<Overlay> = Vec::new();
    let mut global_sets: Vec<i32> = Vec::new();
    let partial_dir = store.join("zones").join("partial");
    if partial_dir.is_dir() {
        for f in xml::enumerate_xml_files(&partial_dir, true)? {
            let Some(text) = load(&f)? else { continue };
            let doc = xml::parse(&text)?;
            for top in elements(doc.root_element(), "object").filter(|o| o.attribute("name") == Some("ServerZonePartial")) {
                let map_ids = ints(mem(Some(top), "DynamicMapIDs"));
                if m(Some(top), "AutoApply").as_deref() != Some("false") && !map_ids.is_empty() {
                    overlays.push(Overlay {
                        id: int_of(Some(top), "ID"),
                        dynamic_map_ids: map_ids,
                        drop_set_ids: ints(mem(Some(top), "DropSetIDs")),
                        leaves: leaves(top),
                        loot: loot_in(top, &mut arenas.loot),
                        spawns: spawns_of(top)?,
                    });
                }
                // partial 0 is the game-wide list (GetZonePartialData(0)): every enemy that inherits drops gets its sets
                if int_of(Some(top), "ID") == 0 {
                    global_sets.extend(ints(mem(Some(top), "DropSetIDs")));
                }
            }
        }
    }
    let (mut replaced_spawns, mut added_spawns, mut drop_add_ons, mut zone_drop_sets) = (0, 0, 0, 0);
    let mut order: Vec<usize> = (0..overlays.len()).collect();
    order.sort_by_key(|&i| overlays[i].id); // stable, like OrderBy
    for oi in order {
        let top = &overlays[oi];
        let mut seen_maps = HashSet::new();
        for &dm in top.dynamic_map_ids.iter().filter(|d| seen_maps.insert(**d)) {
            for &aid in areas_of_map.get(&dm).map(Vec::as_slice).unwrap_or(&[]) {
                for &ds in &top.drop_set_ids {
                    if areas[aid - 1].drop_sets.insert(ds) {
                        zone_drop_sets += 1;
                    }
                }
                let a = &mut areas[aid - 1];
                a.refs.extend(top.leaves.iter().cloned());
                a.loot.extend(top.loot.iter().copied());
                for (sid, s) in &top.spawns {
                    // read again per area: each gets its own copy
                    if s.devil > 0 {
                        if a.spawns.contains_key(sid) {
                            replaced_spawns += 1;
                        } else {
                            added_spawns += 1;
                        }
                        arenas.spawns.push(s.clone());
                        a.spawns.insert(*sid, arenas.spawns.len() - 1);
                    } else if let Some(&existing) = a.spawns.get(sid) {
                        arenas.spawns[existing].drops.extend(s.drops.iter().copied());
                        arenas.spawns[existing].drop_sets.extend(s.drop_sets.iter().copied());
                        drop_add_ons += 1;
                    }
                }
            }
        }
    }
    console::write_line(&format!(
        "Overlays: {}; {added_spawns} spawns added, {replaced_spawns} replaced, {drop_add_ons} drop add-ons, {zone_drop_sets} drop sets added to zones.",
        overlays.len()
    ));

    // ---- server: events, for the chests (ActionCreateLoot) a place's events open ----
    // A zone file's triggers, objects and spawn groups start events; events name further events (next, branches,
    // start-event actions). Every event an area reaches this way is its own; the chests they make are its chests:
    // the end-of-stage boss chest (isBossBox) and others. The seasonal partials\<event>\ are left out.
    let mut event_refs: HashMap<String, Vec<String>> = HashMap::new();
    let mut event_loot: HashMap<String, Vec<usize>> = HashMap::new();
    // the items its scripts give (reward rolls), with the flag key each needs (0: none), and the keys turned into items
    let mut event_rewards: HashMap<String, Vec<(i32, Drop)>> = HashMap::new();
    let mut item_flag_keys: HashSet<i32> = HashSet::new();
    let event_dir = store.join("events");
    if event_dir.is_dir() {
        for f in xml::enumerate_xml_files(&event_dir, true)? {
            let Some(text) = load(&f)? else { continue };
            let doc = xml::parse(&text)?;
            for ev in elements(doc.root_element(), "object") {
                let Some(eid) = m(Some(ev), "ID").filter(|e| !e.is_empty()) else { continue };
                if event_refs.contains_key(&eid) {
                    continue;
                }
                let mut seen = HashSet::new();
                let refs: Vec<String> = leaves(ev).into_iter().filter(|v| seen.insert(v.clone())).collect();
                event_refs.insert(eid.clone(), refs);
                let l = loot_in(ev, &mut arenas.loot);
                let (rewards, keys) = script_rewards(ev);
                item_flag_keys.extend(keys);
                if !rewards.is_empty() {
                    event_rewards.insert(eid.clone(), rewards);
                }
                if !l.is_empty() {
                    event_loot.insert(eid, l);
                }
            }
        }
    }
    // every event an area reaches (its own and the ones they name), in the order found
    let reached = |a: &Area| -> Vec<String> {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut order = Vec::new();
        let mut todo: Vec<&str> = a.refs.iter().map(String::as_str).filter(|r| event_refs.contains_key(*r)).collect();
        while let Some(e) = todo.pop() {
            if !seen.insert(e) {
                continue;
            }
            order.push(e.to_string());
            for r in &event_refs[e] {
                if event_refs.contains_key(r.as_str()) && !seen.contains(r.as_str()) {
                    todo.push(r.as_str());
                }
            }
        }
        order
    };
    let chests_of = |a: &Area| -> Vec<usize> {
        let mut res = a.loot.clone();
        for e in reached(a) {
            if let Some(l) = event_loot.get(&e) {
                res.extend(l.iter().copied());
            }
        }
        res
    };
    // what an area's events give by script: End of Days' reward rolls (Demon Souls, God's Majesty 0.1%...)
    let rewards_of = |a: &Area| -> Vec<Drop> {
        reached(a)
            .iter()
            .filter_map(|e| event_rewards.get(e))
            .flatten()
            .filter(|(key, _)| *key == 0 || item_flag_keys.contains(key))
            .map(|(_, d)| *d)
            .collect()
    };

    // ---- server: drop sets ----
    let mut drop_sets: HashMap<i32, Vec<Drop>> = HashMap::new();
    for f in xml::enumerate_xml_files(&store.join("data").join("dropset"), true)? {
        let Some(text) = load(&f)? else { continue };
        let doc = xml::parse(&text)?;
        for ds in doc.root().descendants().filter(|o| o.is_element() && o.tag_name().name() == "object" && o.attribute("name") == Some("DropSet")) {
            drop_sets.insert(int_of(Some(ds), "ID"), drops_of(mem(Some(ds), "Drops")));
        }
    }

    // ---- the enemies that can't be contracted ----
    let no_contract = |s: &SpawnInfo| s.talk_resist >= 100 || (s.talk_results & 1) == 0;
    let wanted: HashSet<i32> = areas.iter().flat_map(|a| a.spawns.values()).map(|&i| &arenas.spawns[i]).filter(|s| no_contract(s)).map(|s| s.devil).collect();

    // ---- client: those demons ----
    let mut devils: HashMap<i32, Devil> = HashMap::new();
    {
        let text = xml::read_text(&decoded.join("DevilData.xml"))?;
        let doc = xml::parse(&text)?;
        for o in xml::records(&doc) {
            let basic = obj(Some(o), "basic");
            let id = int_of(basic, "ID");
            if !wanted.contains(&id) {
                continue;
            }
            let growth = obj(Some(o), "growth");
            let battle = obj(Some(o), "battleData");
            let mut correct = vec![0; 126];
            for (i, v) in ints(mem(battle, "correct")).into_iter().take(126).enumerate() {
                correct[i] = v;
            }
            devils.insert(
                id,
                Devil {
                    id,
                    name: m(basic, "name").unwrap_or_default().trim().to_string(),
                    race: obj(Some(o), "category").and_then(|c| m(Some(c), "race")),
                    growth_type: int_of(growth, "growthType"),
                    base_level: int_of(growth, "baseLevel"),
                    enemy_hp: ints(mem(battle, "enemyHP")).first().copied().unwrap_or(0),
                    correct,
                    skills: ints(mem(growth, "skills")).into_iter().filter(|x| *x > 0).collect(),
                    enemy_skills: ints(mem(growth, "enemyOnlySkills")).into_iter().filter(|x| *x > 0).collect(),
                    traits: ints(mem(growth, "traits")).into_iter().filter(|x| *x > 0).collect(),
                },
            );
        }
    }

    // ---- dungeons: an instance is one dungeon (its areas in order); areas outside any instance group by name ----
    let mut dungeons: Vec<Dungeon> = Vec::new();
    let mut in_instance: HashSet<usize> = HashSet::new();
    let mut instances: Vec<Instance> = Vec::new();
    for f in xml::enumerate_xml_files(&store.join("data").join("zoneinstance"), false)? {
        let Some(text) = load(&f)? else { continue };
        let doc = xml::parse(&text)?;
        for o in elements(doc.root_element(), "object").filter(|o| o.attribute("name") == Some("ServerZoneInstance")) {
            instances.push(Instance {
                zones: ints(mem(Some(o), "ZoneIDs")),
                maps: ints(mem(Some(o), "DynamicMapIDs")),
                lobby: int_of(Some(o), "LobbyID"),
                msg: int_of(Some(o), "CreateMessageID"),
            });
        }
    }
    // The game names a difficulty only in the line it shows when the dungeon is made: an instance's CreateMessageID
    // is a CEventMessageData line such as "Suginami Tunnels (Bronze)" has been created. / Suginami Tunnels (Gray)
    // created. Instances of one difficulty share it (5401-5403 are all Bronze).
    let mut created_name: HashMap<i32, String> = HashMap::new();
    let msg_file = decoded.join("CEventMessageData.xml");
    if msg_file.is_file() {
        let wanted_msg: HashSet<i32> = instances.iter().map(|i| i.msg).filter(|m| *m > 0).collect();
        let text = xml::read_text(&msg_file)?;
        let doc = xml::parse(&text)?;
        let ws = Regex::new(r"\s+").expect("whitespace");
        let quoted = Regex::new(r#"^"(.+?)"\s*has been created"#).expect("quoted");
        let plain = Regex::new(r"^(.+?)\s+(?:has been\s+)?created\.?$").expect("plain");
        let paren = Regex::new(r"(\S)\(").expect("paren");
        let japanese = Regex::new("[\u{3000}-\u{30FF}\u{4E00}-\u{9FFF}\u{FF00}-\u{FFEF}]").expect("japanese");
        for o in xml::records(&doc) {
            let id = int_of(Some(o), "ID");
            if !wanted_msg.contains(&id) {
                continue;
            }
            let joined = mem(Some(o), "lines").map(|l| elements(l, "element").map(xml::value).collect::<Vec<_>>().join(" ")).unwrap_or_default();
            let line = ws.replace_all(&joined, " ").trim().to_string();
            let cap = quoted.captures(&line).or_else(|| plain.captures(&line));
            let Some(cap) = cap else { continue };
            let n = paren.replace_all(cap[1].trim(), "${1} (").into_owned(); // "Old Ichigaya Camp(Bronze)"
            // a line left in Japanese ("『シンジュク』has been ...") names nothing: the zone name is used instead
            let units = n.encode_utf16().count();
            if units > 0 && units < 60 && !japanese.is_match(&n) {
                created_name.insert(id, n);
            }
        }
    } else {
        console::error_line("CEventMessageData.xml is not in the decoded folder: dungeons are not split by difficulty.");
    }
    let zname = |id: i32| -> Option<String> { zones.get(&id).filter(|z| !z.0.is_empty()).map(|z| z.0.clone()) };
    // the name most of its areas have; areas named per floor ("Denshi Kairo 001F") share the words before the floor
    let stem_floor = Regex::new(r"\s*(\d+F|B\d+F?|\d+)$").expect("floor");
    let name_of = |ids: &[usize]| -> Option<String> {
        let names: Vec<String> = ids.iter().filter_map(|k| zname(areas[k - 1].zone)).collect();
        if names.is_empty() {
            return None;
        }
        // GroupBy keeps the first appearance of each name; the largest group, the first of equals, wins
        let mut counts: IndexMap<&str, usize> = IndexMap::new();
        for n in &names {
            *counts.entry(n.as_str()).or_default() += 1;
        }
        let (top_key, top_count) = counts.iter().fold(None::<(&str, usize)>, |best, (k, c)| match best {
            Some((_, bc)) if bc >= *c => best,
            _ => Some((k, *c)),
        })?;
        if top_count > 1 || names.len() == 1 {
            return Some(top_key.to_string());
        }
        let mut stems: IndexSet<String> = IndexSet::new();
        for n in &names {
            stems.insert(stem_floor.replace(n, "").trim().to_string());
        }
        Some(if stems.len() == 1 && !stems[0].is_empty() { stems[0].clone() } else { top_key.to_string() })
    };
    // instances of one dungeon (other difficulties, other entrances) are merged by name and entrance
    let mut by_key: HashMap<String, usize> = HashMap::new();
    for inst in &instances {
        // the instance's own map of each zone (DynamicMapIDs pairs with ZoneIDs); without one, every map of the zone
        let mut ids: IndexSet<usize> = IndexSet::new();
        for (i, z) in inst.zones.iter().enumerate() {
            match inst.maps.get(i).and_then(|mp| area_of.get(&(*z, *mp))) {
                Some(own) => {
                    ids.insert(*own);
                }
                None => ids.extend(areas.iter().filter(|a| a.zone == *z).map(|a| a.id)),
            }
        }
        let ids: Vec<usize> = ids.into_iter().collect();
        if ids.is_empty() {
            continue;
        }
        let Some(name) = created_name.get(&inst.msg).cloned().or_else(|| name_of(&ids)) else { continue };
        in_instance.extend(ids.iter().copied());
        let from = zname(inst.lobby).unwrap_or_default();
        let key = format!("{name}|{from}");
        if let Some(&at) = by_key.get(&key) {
            for z in ids {
                if !dungeons[at].areas.contains(&z) {
                    dungeons[at].areas.push(z);
                }
            }
        } else {
            by_key.insert(key, dungeons.len());
            dungeons.push(Dungeon { name, kind: "Dungeon", from, areas: ids });
        }
    }
    // areas outside instances, one entry per name; numbered floors ("Logic Space Floor 12") are one place
    let stem_re = Regex::new(r"(?i)\s+(Floor\s*\d+|\d+F|B\d+F?)$").expect("stem");
    let stem = |n: &str| stem_re.replace(n, "").trim().to_string();
    // (a map of a zone that a dungeon already uses, which no instance points at, is a spare variant: left out)
    let instanced_zones: HashSet<i32> = areas.iter().filter(|a| in_instance.contains(&a.id)).map(|a| a.zone).collect();
    let mut groups: IndexMap<String, Vec<&Area>> = IndexMap::new();
    for a in areas.iter().filter(|a| !in_instance.contains(&a.id) && !instanced_zones.contains(&a.zone) && !a.name.is_empty()) {
        let s = stem(&a.name);
        groups.entry(if s.is_empty() { a.name.clone() } else { s }).or_default().push(a);
    }
    let trailing_number = Regex::new(r"\d+$").expect("number");
    for (key, g) in groups {
        let ty = g.iter().find_map(|a| a.ty.as_deref());
        let kind = match ty {
            Some("DUNGEON") => "Dungeon",
            Some("FIELD") => "Field",
            Some("TOWN" | "PUBLIC" | "SHOP") => "Town",
            Some("PVP") => "PvP",
            _ => "Other",
        };
        let mut sorted: Vec<&Area> = g;
        sorted.sort_by_key(|a| (trailing_number.find(&a.name).map(|m| m.as_str().parse::<i32>().unwrap_or(0)).unwrap_or(0), a.id));
        dungeons.push(Dungeon { name: key, kind, from: String::new(), areas: sorted.iter().map(|a| a.id).collect() });
    }

    // ---- output ----
    let mut mobs: IndexMap<String, Value> = IndexMap::new();
    let mut mob_meta: HashMap<String, (String, i32)> = HashMap::new();
    let mut icons: IndexMap<i32, Option<String>> = IndexMap::new();
    let icon_dir = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default().join("dungeon-icons");
    // a skill's picture: the client's icon (GameData.SkillIconAsset, the same as the Expertise page's) from
    // Interface\tga, else tga2 (IMAGINE's added icons), saved once beside the enemy pictures; null if it has none
    let mut skill_icons: HashMap<String, Option<String>> = HashMap::new();
    let skill_icon_of = |id: i32, skill_icons: &mut HashMap<String, Option<String>>| -> Result<Option<String>> {
        let Some(client) = client_dir else { return Ok(None) };
        let Some(asset) = data.skill_icon_asset(id)? else { return Ok(None) };
        if let Some(known) = skill_icons.get(&asset) {
            return Ok(known.clone());
        }
        let png = icon_dir.join(format!("{asset}.png"));
        let mut rel = None;
        if png.is_file() {
            rel = Some(format!("dungeon-icons/{asset}.png"));
        } else if let Some(bytes) = item_icons::client_png(client, &asset, &[]) {
            // .tga, or .dds for the few kept that way
            std::fs::create_dir_all(&icon_dir)?;
            std::fs::write(&png, bytes)?;
            rel = Some(format!("dungeon-icons/{asset}.png"));
        }
        skill_icons.insert(asset, rel.clone());
        Ok(rel)
    };
    let icon_of = |devil: i32, icons: &mut IndexMap<i32, Option<String>>| -> Result<Option<String>> {
        if let Some(known) = icons.get(&devil) {
            return Ok(known.clone());
        }
        let mut rel = None;
        if let Some(client) = client_dir {
            if let Some((name, folder)) = data.devil_icon_file(devil)? {
                let tga = client.join("Interface").join(&folder).join(format!("{name}.tga"));
                // named after the picture: many enemy versions of one demon share it
                let png = icon_dir.join(format!("{name}.png"));
                if png.is_file() {
                    rel = Some(format!("dungeon-icons/{name}.png"));
                } else if tga.is_file() {
                    if let Some(bytes) = crate::data::tga::to_png(&std::fs::read(&tga)?) {
                        std::fs::create_dir_all(&icon_dir)?;
                        std::fs::write(&png, bytes)?;
                        rel = Some(format!("dungeon-icons/{name}.png"));
                    }
                }
            }
        }
        icons.insert(devil, rel.clone());
        Ok(rel)
    };
    // each skill once (name, description), the enemies point at it by id: most enemies share their skills
    let mut skill_table: BTreeMap<i32, Vec<Option<String>>> = BTreeMap::new();
    let skills_of = |ids: &[i32], skill_table: &mut BTreeMap<i32, Vec<Option<String>>>, skill_icons: &mut HashMap<String, Option<String>>| -> Result<Vec<i32>> {
        let mut seen = HashSet::new();
        let mut res = Vec::new();
        for &id in ids.iter().filter(|id| seen.insert(**id)) {
            if !skill_table.contains_key(&id) {
                let n = data.skill_names()?.get(&id).cloned().flatten();
                let Some(n) = n.filter(|n| !n.trim().is_empty()) else { continue };
                let mut row = vec![Some(n.trim().to_string()), Some(data.skill_text(id)?.unwrap_or_default()), skill_icon_of(id, skill_icons)?];
                // a 4th entry: the game's Japanese when the name is BABEL's translation (skill-names-en.tsv; the page greys it
                // out); a 5th: the game's Japanese description when the description is BABEL's translation (desc-en.tsv)
                let (name_ja, desc_ja) = (data.skill_name_ja(id)?, data.skill_text_ja(id)?);
                if name_ja.is_some() || desc_ja.is_some() {
                    row.push(name_ja);
                }
                if desc_ja.is_some() {
                    row.push(desc_ja);
                }
                skill_table.insert(id, row);
            }
            res.push(id);
        }
        Ok(res)
    };

    // ---- dropped items: each once in "items" (name, the game's description, picture); drops point at it by id ----
    struct ItemInfo {
        name: String,
        desc: String,
        icon: i32,
    }
    let mut item_info: HashMap<i32, ItemInfo> = HashMap::new();
    let item_file = decoded.join("CItemData.xml");
    if item_file.is_file() {
        let dropped: HashSet<i32> = areas
            .iter()
            .flat_map(|a| a.spawns.values())
            .flat_map(|&i| arenas.spawns[i].drops.iter().map(|d| d.0))
            .chain(drop_sets.values().flat_map(|l| l.iter().map(|d| d.0)))
            // what chests list themselves or get by script, and the scripted rewards: their pictures and descriptions too
            .chain(arenas.loot.iter().flat_map(|l| l.drops.iter().chain(l.script_drops.iter()).map(|d| d.0)))
            .chain(event_rewards.values().flatten().map(|(_, d)| d.0))
            .collect();
        let text = xml::read_text(&item_file)?;
        let doc = xml::parse(&text)?;
        for o in xml::records(&doc) {
            let b = obj(Some(o), "baseData");
            let id = int_of(b, "ID");
            if dropped.contains(&id) {
                item_info.insert(
                    id,
                    ItemInfo {
                        name: m(b, "name").unwrap_or_default().trim().to_string(),
                        desc: m(b, "desc").unwrap_or_default().trim().to_string(),
                        icon: int_of(b, "icon"),
                    },
                );
            }
        }
    } else {
        console::error_line("CItemData.xml is not in the decoded folder: drops have no pictures or descriptions.");
    }
    let mut item_table: BTreeMap<i32, Vec<Option<String>>> = BTreeMap::new();
    let mut item_pics = 0;
    let mut item_picture = |icon: i32, skill_icons: &mut HashMap<String, Option<String>>| -> Result<Option<String>> {
        let Some(client) = client_dir else { return Ok(None) };
        let Some(file) = item_icons::item_icon_file(client, icon) else { return Ok(None) };
        if let Some(known) = skill_icons.get(&file) {
            return Ok(known.clone()); // the same file-name cache as the skill pictures
        }
        let png = icon_dir.join(format!("{file}.png"));
        let mut rel = None;
        if png.is_file() {
            rel = Some(format!("dungeon-icons/{file}.png"));
        } else if let Some(bytes) = item_icons::client_png(client, &file, &[]) {
            std::fs::create_dir_all(&icon_dir)?;
            std::fs::write(&png, bytes)?;
            rel = Some(format!("dungeon-icons/{file}.png"));
        }
        if rel.is_some() {
            item_pics += 1;
        }
        skill_icons.insert(file, rel.clone());
        Ok(rel)
    };
    let item_name_of = |id: i32, item_info: &HashMap<i32, ItemInfo>| -> Option<String> {
        (match item_info.get(&id) {
            Some(i) if !i.name.is_empty() => Some(i.name.clone()),
            _ => item_name.get(&id).cloned(),
        })
        // BABEL's English where the client names it only in Japanese (item-names-en.tsv)
        .map(|n| crate::data::game_data::item_name_en(id, &n).unwrap_or(n))
    };
    let mut item_ref = |id: i32, item_table: &mut BTreeMap<i32, Vec<Option<String>>>, skill_icons: &mut HashMap<String, Option<String>>| -> Result<i32> {
        if !item_table.contains_key(&id) {
            let (game_desc, icon) = item_info.get(&id).map(|i| (i.desc.clone(), i.icon)).unwrap_or((String::new(), 0));
            // BABEL's English where the game's description is Japanese (desc-en.tsv); the Japanese goes in a 5th entry
            let desc_en = crate::data::game_data::desc_en(&game_desc);
            let desc = desc_en.clone().unwrap_or_else(|| game_desc.clone());
            let pic = if icon > 0 { item_picture(icon, skill_icons)? } else { None };
            let shown = item_name_of(id, &item_info);
            // a 4th entry: the client's Japanese when the name is BABEL's translation (item-names-en.tsv)
            let ja = crate::data::game_data::item_translations().get(&id).filter(|(_, en)| shown.as_ref() == Some(en)).map(|(ja, _)| ja.clone());
            let mut row = vec![shown, Some(desc), pic];
            if ja.is_some() || desc_en.is_some() {
                row.push(ja);
            }
            if desc_en.is_some() {
                row.push(Some(game_desc));
            }
            item_table.insert(id, row);
        }
        Ok(id)
    };
    let drop_lines = |list: Vec<Drop>,
                      item_table: &mut BTreeMap<i32, Vec<Option<String>>>,
                      skill_icons: &mut HashMap<String, Option<String>>,
                      item_ref: &mut dyn FnMut(i32, &mut BTreeMap<i32, Vec<Option<String>>>, &mut HashMap<String, Option<String>>) -> Result<i32>|
     -> Result<Vec<Value>> {
        let filtered: Vec<Drop> = list.into_iter().filter(|x| item_name_of(x.0, &item_info).is_some() && x.1 > 0.0).collect();
        // GroupBy item (first appearance), the highest rate of each (the first of equals)
        let mut best: IndexMap<i32, Drop> = IndexMap::new();
        for x in filtered {
            match best.get(&x.0) {
                Some(b) if b.1 >= x.1 => {}
                _ => {
                    best.insert(x.0, x);
                }
            }
        }
        let mut v: Vec<Drop> = best.into_values().collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)); // OrderByDescending, stable
        let mut res = Vec::new();
        for x in v {
            let i = item_ref(x.0, item_table, skill_icons)?;
            let mut o = Map::new();
            o.insert("i".into(), json!(i));
            o.insert("rate".into(), json!(fmt::round(x.1, 3)));
            if x.2 > 1 {
                o.insert("max".into(), json!(x.2));
            }
            res.push(Value::Object(o));
        }
        Ok(res)
    };

    let race_text = |r: Option<&str>| -> Option<String> {
        let r = r.filter(|r| !r.is_empty())?;
        Some(
            r.split('_')
                .filter(|w| !w.is_empty())
                .map(|w| {
                    let mut c = w.chars();
                    let first = c.next().map(|f| f.to_string()).unwrap_or_default();
                    if w.chars().count() <= 1 { w.to_string() } else { format!("{first}{}", c.as_str().to_lowercase()) }
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    };

    let mut out_list: Vec<(&'static str, String, Value)> = Vec::new();
    let (mut mob_refs, mut chest_count, mut script_chests, mut reward_count) = (0usize, 0usize, 0usize, 0usize);
    for dungeon in &dungeons {
        let (name, kind, from, ids) = (&dungeon.name, dungeon.kind, &dungeon.from, &dungeon.areas);
        let mut entries: IndexMap<String, Entry> = IndexMap::new();
        for (i, aid) in ids.iter().enumerate() {
            let spawn_ids: Vec<usize> = areas[aid - 1].spawns.values().copied().filter(|&si| no_contract(&arenas.spawns[si])).collect();
            for si in spawn_ids {
                let s = arenas.spawns[si].clone();
                let Some(d) = devils.get(&s.devil) else { continue };
                if d.name.is_empty() {
                    continue;
                }
                // ---- MobKey ----
                let level = if s.level > 0 { s.level } else { d.base_level };
                let key = format!("{}-{}", d.id, level);
                if !mobs.contains_key(&key) {
                    let mut st = d.correct.clone();
                    if let Some(g) = data.demons()?.growth.get(&d.growth_type) {
                        grow(&mut st, g, level);
                    }
                    let lv = level as f64;
                    let hp = st[6]
                        + (st[6] as f64 * 0.03 * lv).ceil() as i32
                        + (st[0] as f64 * 0.3).ceil() as i32
                        + ((st[6] as f64 * 0.01 + 0.5) * st[2] as f64).ceil() as i32
                        + d.enemy_hp;
                    let mp = st[7]
                        + (st[7] as f64 * 0.03 * lv).ceil() as i32
                        + (st[1] as f64 * 0.3).ceil() as i32
                        + ((st[7] as f64 * 0.01 + 0.5) * st[3] as f64).ceil() as i32;
                    let dep = |own: i32, src: usize, k: f64| own + round_away(st[src] as f64 * k + 0.5 + lv * 0.1);
                    let mut aff: Vec<Value> = Vec::new();
                    for (res, nra, nm) in AFFINITIES {
                        let v = st[nra];
                        let (kind_n, pct) = if v > 0 { (v % 10, v / 10) } else { (0, 0) };
                        if (1..=3).contains(&kind_n) && pct > 0 {
                            aff.push(json!({ "el": nm, "kind": NRA_KINDS[kind_n as usize], "pct": pct }));
                        } else if st[res] != 0 {
                            aff.push(json!({ "el": nm, "res": st[res] }));
                        }
                    }
                    // Izar ﾎｱ (a garbled α), 非凡なマッハ: BABEL's English (demon-names-en.tsv), the game's name kept as `ja`
                    let en = crate::data::game_data::translated_name(&demon_en, d.id, &d.name);
                    mob_meta.insert(key.clone(), (en.clone().unwrap_or_else(|| d.name.clone()), level));
                    let mut o = Map::new();
                    o.insert("name".into(), json!(en.as_deref().unwrap_or(&d.name)));
                    if en.is_some() {
                        o.insert("ja".into(), json!(d.name));
                    }
                    if let Some(r) = race_text(d.race.as_deref()) {
                        o.insert("race".into(), json!(r));
                    }
                    o.insert("level".into(), json!(level));
                    if let Some(ic) = icon_of(d.id, &mut icons)? {
                        o.insert("icon".into(), json!(ic));
                    }
                    o.insert("hp".into(), json!(hp));
                    o.insert("mp".into(), json!(mp));
                    o.insert("stats".into(), json!([st[0], st[1], st[2], st[3], st[4], st[5]]));
                    o.insert(
                        "combat".into(),
                        json!([dep(st[12], 0, 0.5), dep(st[13], 4, 0.5), dep(st[14], 1, 0.5), dep(st[15], 3, 0.5), dep(st[19], 2, 0.1), dep(st[20], 3, 0.1)]),
                    );
                    o.insert("aff".into(), Value::Array(aff));
                    o.insert("skills".into(), json!(skills_of(&d.skills, &mut skill_table, &mut skill_icons)?));
                    o.insert("enemySkills".into(), json!(skills_of(&d.enemy_skills, &mut skill_table, &mut skill_icons)?));
                    o.insert("traits".into(), json!(skills_of(&d.traits, &mut skill_table, &mut skill_icons)?));
                    mobs.insert(key.clone(), Value::Object(o));
                }
                let ek = format!("{key}{}", if s.boss { "|boss" } else { "" });
                let e = entries.entry(ek).or_insert_with(|| Entry { spawn: si, key: key.clone(), wher: BTreeSet::new() });
                e.wher.insert(i);
                // the same enemy at the same level in another area may carry other drops: keep them all
                if e.spawn != si {
                    let (extra_drops, extra_sets) = (s.drops.clone(), s.drop_sets.clone());
                    arenas.spawns[e.spawn].drops.extend(extra_drops);
                    arenas.spawns[e.spawn].drop_sets.extend(extra_sets);
                }
            }
        }
        if entries.is_empty() {
            continue;
        }
        // area names, when they say more than the dungeon's own name
        let area_names: Vec<&str> = ids.iter().map(|z| areas[z - 1].name.as_str()).collect();
        let named = area_names.iter().collect::<HashSet<_>>().len() > 1;
        struct Line {
            m: String,
            boss: bool,
            xp: i64,
            talks: bool,
            wher: Option<String>,
            drops: Vec<Value>,
            own: bool,
        }
        let mut list: Vec<Line> = Vec::new();
        for e in entries.values() {
            // the areas it is in, when they have names of their own ("Denshi Kairo 012F" -> "012F"); areas that
            // only carry the dungeon's name say nothing more, and instance order is lost once instances are merged
            let mut wher = None;
            if named && e.wher.len() < ids.len() {
                let mut seen = HashSet::new();
                let own: Vec<String> = e
                    .wher
                    .iter()
                    .map(|&i| area_names[i])
                    .filter(|n| !n.is_empty() && *n != name.as_str())
                    .filter(|n| seen.insert(*n))
                    .map(|n| match n.strip_prefix(&format!("{name} ")) {
                        Some(rest) => rest.to_string(),
                        None => n.to_string(),
                    })
                    .collect();
                if !own.is_empty() {
                    wher = Some(if own.len() > 8 { format!("{} and {} more", own[..8].join(", "), own.len() - 8) } else { own.join(", ") });
                }
            }
            let s = arenas.spawns[e.spawn].clone();
            let mut seen_sets = HashSet::new();
            let mut all_drops = s.drops.clone();
            for x in s.drop_sets.iter().filter(|x| seen_sets.insert(**x)).filter(|x| drop_sets.contains_key(x)) {
                all_drops.extend(drop_sets[x].iter().copied());
            }
            let drops = drop_lines(all_drops, &mut item_table, &mut skill_icons, &mut item_ref)?;
            list.push(Line { m: e.key.clone(), boss: s.boss, xp: s.xp, talks: s.talk_resist < 100, wher, drops, own: !s.inherit });
        }
        list.sort_by(|a, b| {
            let (ma, mb) = (&mob_meta[&a.m], &mob_meta[&b.m]);
            (a.boss as i32).cmp(&(b.boss as i32)).then(ma.1.cmp(&mb.1)).then(cmp_invariant(&ma.0, &mb.0))
        });
        mob_refs += list.len();
        let mut seen_sets = HashSet::new();
        let mut shared_list: Vec<Drop> = Vec::new();
        for x in ids.iter().flat_map(|z| areas[z - 1].drop_sets.iter().copied()).filter(|x| seen_sets.insert(*x)).filter(|x| drop_sets.contains_key(x)) {
            shared_list.extend(drop_sets[&x].iter().copied());
        }
        let shared = drop_lines(shared_list, &mut item_table, &mut skill_icons, &mut item_ref)?;
        // chests its events open: their drop sets and listed drops (a chest made by a transform script names no
        // contents in the data: counted, not shown)
        let mut seen_loot = HashSet::new();
        let chest_acts: Vec<usize> = ids.iter().flat_map(|z| chests_of(&areas[z - 1])).filter(|l| seen_loot.insert(*l)).collect();
        let chest_drops = |acts: Vec<usize>| -> Vec<Drop> {
            let mut res = Vec::new();
            for c in acts {
                let loot = &arenas.loot[c];
                for x in loot.drop_set_ids.iter().filter(|x| drop_sets.contains_key(x)) {
                    res.extend(drop_sets[x].iter().copied());
                }
                res.extend(loot.drops.iter().copied());
                res.extend(loot.script_drops.iter().copied());
            }
            res
        };
        let boss_chest = drop_lines(chest_drops(chest_acts.iter().copied().filter(|&c| arenas.loot[c].is_boss_box).collect()), &mut item_table, &mut skill_icons, &mut item_ref)?;
        let chests = drop_lines(chest_drops(chest_acts.iter().copied().filter(|&c| !arenas.loot[c].is_boss_box).collect()), &mut item_table, &mut skill_icons, &mut item_ref)?;
        chest_count += chest_acts.len();
        // the items its events give by script (reward rolls; each its chance per roll)
        let mut seen_area = HashSet::new();
        let reward_list: Vec<Drop> = ids.iter().filter(|z| seen_area.insert(**z)).flat_map(|z| rewards_of(&areas[z - 1])).collect();
        reward_count += usize::from(!reward_list.is_empty());
        let rewards = drop_lines(reward_list, &mut item_table, &mut skill_icons, &mut item_ref)?;
        script_chests += chest_acts.iter().filter(|&&c| arenas.loot[c].made_by_script).count();
        let min_l = list.iter().map(|x| mob_meta[&x.m].1).min().unwrap_or(0);
        let max_l = list.iter().map(|x| mob_meta[&x.m].1).max().unwrap_or(0);
        let mobs_json: Vec<Value> = list
            .into_iter()
            .map(|x| {
                let mut o = Map::new();
                o.insert("m".into(), json!(x.m));
                if x.boss {
                    o.insert("boss".into(), json!(true));
                }
                if x.xp >= 0 {
                    o.insert("xp".into(), json!(x.xp));
                }
                if x.talks {
                    o.insert("talk".into(), json!("talks, never joins"));
                }
                if let Some(w) = x.wher {
                    o.insert("where".into(), json!(w));
                }
                if !x.drops.is_empty() {
                    o.insert("drops".into(), Value::Array(x.drops));
                }
                if x.own {
                    o.insert("own".into(), json!(true)); // only its own drops, not the place's or the game-wide ones
                }
                Value::Object(o)
            })
            .collect();
        let mut o = Map::new();
        // the entrance is kept even when it has the place's own name: the page groups places by it
        o.insert("name".into(), json!(name));
        o.insert("kind".into(), json!(kind));
        if !from.is_empty() {
            o.insert("from".into(), json!(from));
        }
        o.insert("areas".into(), json!(ids.len()));
        o.insert("levels".into(), json!(if min_l == max_l { format!("{min_l}") } else { format!("{min_l}\u{2013}{max_l}") }));
        o.insert("mobs".into(), Value::Array(mobs_json));
        if !shared.is_empty() {
            o.insert("shared".into(), Value::Array(shared));
        }
        if !boss_chest.is_empty() {
            o.insert("bossChest".into(), Value::Array(boss_chest));
        }
        if !chests.is_empty() {
            o.insert("chests".into(), Value::Array(chests));
        }
        if !rewards.is_empty() {
            o.insert("rewards".into(), Value::Array(rewards)); // given by its events' scripts: the chance per roll
        }
        out_list.push((kind, name.clone(), Value::Object(o)));
    }
    out_list.sort_by(|a, b| {
        let rank = |k: &str| if k == "Dungeon" { 0 } else if k == "Field" { 1 } else { 2 };
        rank(a.0).cmp(&rank(b.0)).then_with(|| ic_key(&a.1).cmp(&ic_key(&b.1)))
    });

    let mut seen_sets = HashSet::new();
    let mut everywhere_list: Vec<Drop> = Vec::new();
    for x in global_sets.iter().copied().filter(|x| seen_sets.insert(*x)).filter(|x| drop_sets.contains_key(x)) {
        everywhere_list.extend(drop_sets[&x].iter().copied());
    }
    // the game-wide drops (partial 0)
    let everywhere = drop_lines(everywhere_list, &mut item_table, &mut skill_icons, &mut item_ref)?;

    let table_json = |t: &BTreeMap<i32, Vec<Option<String>>>| -> Value {
        Value::Object(t.iter().map(|(k, v)| (k.to_string(), Value::Array(v.iter().map(|x| json!(x)).collect()))).collect())
    };
    let mut result = Map::new();
    result.insert("generatedAt".into(), json!(chrono::Local::now().format("%Y-%m-%d").to_string()));
    result.insert("dungeons".into(), Value::Array(out_list.iter().map(|x| x.2.clone()).collect()));
    result.insert("mobs".into(), Value::Object(mobs.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
    result.insert("skills".into(), table_json(&skill_table));
    result.insert("items".into(), table_json(&item_table));
    result.insert("everywhere".into(), Value::Array(everywhere));
    std::fs::write(out_file, json::serialize(&Value::Object(result), Escape::Relaxed, false))?;
    console::write_line(&format!(
        "{} places with enemies that can't be contracted ({} dungeons), {} enemy kinds, {mob_refs} entries; {} of {} icons; {} dropped items, {item_pics} with a picture; {chest_count} chests ({script_chests} made by a script, contents not in the data); {reward_count} places with scripted rewards. Wrote {} ({} KB).",
        out_list.len(),
        out_list.iter().filter(|x| x.0 == "Dungeon").count(),
        mobs.len(),
        icons.values().filter(|v| v.is_some()).count(),
        icons.len(),
        item_table.len(),
        out_file.display(),
        std::fs::metadata(out_file)?.len() / 1024
    ));
    Ok(())
}

/// GetDemonBaseStats: growth table stage A for boost levels 1-6, B 7-11, C 12-16, D 17+ (boost level = (level+3)/4).
fn grow(s: &mut [i32], g: &[Vec<i32>], level: i32) {
    let boost = (level + 3) / 4;
    let stage = (boost - 1) / 5;
    let mut add = |k: usize, n: i32| {
        if k < g.len() && n > 0 {
            for i in 0..6 {
                s[i] += g[k].get(i).copied().unwrap_or(0) * n;
            }
        }
    };
    match stage {
        0 | 1 => add(0, boost),
        2 => {
            add(0, 6);
            add(1, boost - 6);
        }
        3 => {
            add(0, 6);
            add(1, 5);
            add(2, boost - 11);
        }
        4 => {
            add(0, 6);
            add(1, 5);
            add(2, 5);
            add(3, boost - 16);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_stages() {
        let g = vec![vec![1, 2, 3, 4, 5, 6], vec![10; 6], vec![100; 6], vec![1000; 6]];
        let stats_at = |level: i32| {
            let mut s = vec![0; 126];
            grow(&mut s, &g, level);
            s[0]
        };
        assert_eq!(stats_at(1), 1); // boost level (level + 3) / 4 = 1
        assert_eq!(stats_at(40), 10); // boost 10: still the first table
        assert_eq!(stats_at(41), 6 + 10 * 5); // boost 11: six steps of the first table, five of the second
        assert_eq!(stats_at(61), 6 + 10 * 5 + 100 * 5); // boost 16
        assert_eq!(stats_at(81), 6 + 10 * 5 + 100 * 5 + 1000 * 5); // boost 21
    }

    #[test]
    fn ints_and_leaves() {
        assert_eq!(int(Some(" 12 ")), 12);
        assert_eq!(int(Some("12.9")), 12);
        assert_eq!(int(Some("")), 0);
        assert_eq!(int(None::<&str>), 0);
        let doc = xml::parse(r#"<o><member name="a"> x </member><member name="b"><element>1</element><element/><element>2</element></member></o>"#).unwrap();
        assert_eq!(leaves(doc.root_element()), vec!["x", "1", "2"]);
    }

    #[test]
    fn spawns_and_chests() {
        let text = r#"<objects><object name="ServerZone">
            <member name="Spawns"><pair><key>3</key><value><object name="Spawn">
              <member name="EnemyType">9</member><member name="TalkResist">100</member><member name="XP">250</member><member name="InheritDrops">false</member>
              <member name="Drops"><element><object name="D"><member name="ItemType">5</member><member name="Rate">0.25</member><member name="MaxStack">0</member></object></element></member>
              <member name="DropSetIDs"><element>4</element></member>
            </object></value></pair></member>
            <member name="Events"><object name="ActionCreateLoot"><member name="isBossBox">true</member><member name="dropSetIDs"><element>8</element></member></object></member>
          </object></objects>"#;
        let doc = xml::parse(text).unwrap();
        let top = doc.root_element().first_element_child().unwrap();
        let spawns = spawns_of(top).unwrap();
        assert_eq!(spawns.len(), 1);
        let (id, s) = &spawns[0];
        assert_eq!((*id, s.devil, s.level, s.talk_resist, s.talk_results, s.xp), (3, 9, -1, 100, 3, 250));
        assert!(!s.boss && !s.inherit);
        assert_eq!(s.drops, vec![(5, 0.25, 1)]); // a stack of 0 counts as 1
        assert_eq!(s.drop_sets, vec![4]);
        let mut loot = Vec::new();
        assert_eq!(loot_in(top, &mut loot), vec![0]);
        assert!(loot[0].is_boss_box && !loot[0].made_by_script);
        assert_eq!(loot[0].drop_set_ids, vec![8]);
    }
}
