//! What the game data says about HOW a player gets a demon that is not a compendium base ("a variant": Open-minded
//! Ishtar, Lucky Pixie, Inexperienced Cerberus, ...), for the Demon Database (demon_db.rs).
//!
//! The compendium (devil-book.tsv) names 363 base demons. The rest of DevilData's 5,411 records are enemy copies, Mitama
//! forms (a demon's own `mitamaFusionID`, kept inside its entry) and variants: demon types with their own name, level,
//! stats and skills that players can own. A variant is recognised by evidence that the game gives exactly that type:
//!
//! * **Exchange item** (ExchangeData, the server's ItemExchange packet): an item whose sub category is 68
//!   (`ITEM_SUBCATEGORY_EXCHANGE_DEMON`, "Summon PG: ..." and "Demon Voucher") lets the player pick an option, and the
//!   server contracts the option's demon types (`ContractDemon`). The item itself comes from an NPC exchange
//!   (NPCBarterData), an event or NPC reward (`ActionAddRemoveItems`) or an enemy or chest drop (`ItemDrop`).
//! * **Special fusion** (TriUnionSpecialData, `enabled`): the result type of a special recipe.
//! * **Event or NPC gift** (`ActionUpdateCOMP.addDemons` in the server's events and seasonal partials).
//! * **Contract** (a joinable spawn of exactly that type): decided in demon_db.rs, where the spawns are read.
//!
//! Where an item or a gift comes from is the zone of the event file (`events\zone_events-<zone>.xml`, `zones\zone-<zone>.xml`)
//! or the seasonal folder (`partials\<event>\`); an event of a dungeon has no zone the data names, so it is not placed.

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use serde_json::{Map, Value, json};

use super::demon_db::season_name;
use super::xml::{self, element, elements, m, mem};
use crate::Result;
use crate::fmt;

/// `ITEM_SUBCATEGORY_EXCHANGE_DEMON` (comp_hack Constants.h.in).
const EXCHANGE_DEMON: i32 = 68;

/// Where a file puts something: a zone the client names, a seasonal event, or nowhere the data names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Zone(i32),
    Season(String),
    Nowhere,
}

/// The place a server data file stands for, from its path under the datastore (`events/zone_events-30101.xml`).
pub fn place_of(rel: &str) -> Place {
    let rel = rel.replace('\\', "/");
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.first() == Some(&"partials") && parts.len() > 2 {
        let folder = parts[1];
        if Regex::new(r"^\d{6}_").expect("season folder").is_match(folder) {
            return Place::Season(season_name(folder));
        }
    }
    let file = parts.last().copied().unwrap_or("");
    let zone = Regex::new(r"^(?:zone_events-|zone-)(\d+)\.xml$").expect("zone file");
    if parts.first().is_some_and(|d| *d == "events" || *d == "zones") && parts.len() == 2 {
        if let Some(c) = zone.captures(file) {
            if let Ok(id) = c[1].parse() {
                return Place::Zone(id);
            }
        }
    }
    Place::Nowhere
}

pub struct ItemInfo {
    pub name: String,
    pub sub: i32,
}

/// One NPC exchange that hands out an item: what it costs and where the list opens (when the data says).
pub struct Barter {
    pub cost: String,
    pub places: Vec<Place>,
}

/// A special fusion recipe: the sources (demon type, any form of it allowed) and the plugin it needs (0 = none).
pub struct Recipe {
    pub sources: Vec<(i32, bool)>,
    pub plugin: i32,
}

#[derive(Default)]
pub struct Evidence {
    pub items: HashMap<i32, ItemInfo>,
    /// exchange item -> its options, each the demon types it contracts
    pub exchange: HashMap<i32, Vec<Vec<i32>>>,
    /// demon type -> the exchange items that give it
    pub exchange_of: HashMap<i32, Vec<i32>>,
    /// result demon type -> special fusion recipes
    pub special: HashMap<i32, Vec<Recipe>>,
    pub plugins: HashMap<i32, String>,
    /// demon type -> the places an event or NPC adds it to the COMP
    pub gifts: HashMap<i32, Vec<Place>>,
    /// item -> places an event or NPC reward gives it
    pub item_rewards: HashMap<i32, Vec<Place>>,
    /// item -> places it drops
    pub item_drops: HashMap<i32, Vec<Place>>,
    /// item -> NPC exchanges that hand it out
    pub item_barters: HashMap<i32, Vec<Barter>>,
}

fn int(s: &str) -> i32 {
    fmt::parse_i32(s.trim()).unwrap_or(0)
}

fn push_place(map: &mut HashMap<i32, Vec<Place>>, key: i32, p: Place) {
    let v = map.entry(key).or_default();
    if !v.contains(&p) {
        v.push(p);
    }
}

/// Reads every table and server file the evidence comes from. `item_lines` = reference\items-game.tsv, `plugin_lines` =
/// reference\fusion-plugins.tsv.
pub fn gather(decoded: &Path, store: &Path, item_lines: &[String], plugin_lines: &[String]) -> Result<Evidence> {
    let mut ev = Evidence::default();
    for line in item_lines {
        let p: Vec<&str> = line.split('\t').collect();
        if line.starts_with('#') || p.len() < 6 {
            continue;
        }
        ev.items.insert(int(p[0]), ItemInfo { name: p[1].trim().to_string(), sub: int(p[5]) });
    }
    for line in plugin_lines {
        let p: Vec<&str> = line.split('\t').collect();
        if line.starts_with('#') || p.len() < 2 {
            continue;
        }
        ev.plugins.insert(int(p[0]), p[1].trim().to_string());
    }

    // ---- exchange items (client table, the server uses it as is) ----
    let file = decoded.join("ExchangeData.xml");
    if file.is_file() {
        let text = xml::read_text(&file)?;
        let doc = xml::parse(&text)?;
        for r in xml::records(&doc).into_iter().filter(|o| o.attribute("name") == Some("MiExchangeData")) {
            let item = int(&m(Some(r), "ID").unwrap_or_default());
            if ev.items.get(&item).map(|i| i.sub) != Some(EXCHANGE_DEMON) {
                continue;
            }
            let mut options = Vec::new();
            if let Some(os) = mem(Some(r), "options") {
                for o in elements(os, "element").filter_map(|e| element(e, "object")) {
                    let mut types = Vec::new();
                    if let Some(is) = mem(Some(o), "items") {
                        for x in elements(is, "element").filter_map(|e| element(e, "object")) {
                            let id = int(&m(Some(x), "ID").unwrap_or_default());
                            let n = int(&m(Some(x), "stackSize").unwrap_or_default());
                            for _ in 0..n.max(0) {
                                if id != 0 {
                                    types.push(id);
                                }
                            }
                        }
                    }
                    if !types.is_empty() {
                        options.push(types);
                    }
                }
            }
            for t in options.iter().flatten() {
                let v = ev.exchange_of.entry(*t).or_default();
                if !v.contains(&item) {
                    v.push(item);
                }
            }
            ev.exchange.insert(item, options);
        }
    }

    // ---- special fusions ----
    let file = decoded.join("TriUnionSpecialData.xml");
    if file.is_file() {
        let text = xml::read_text(&file)?;
        let doc = xml::parse(&text)?;
        for r in xml::records(&doc).into_iter().filter(|o| o.attribute("name") == Some("MiTriUnionSpecialData")) {
            if m(Some(r), "enabled").as_deref() != Some("true") {
                continue;
            }
            let result = int(&m(Some(r), "resultID").unwrap_or_default());
            let mut sources = Vec::new();
            for i in 1..=3 {
                let s = int(&m(Some(r), &format!("sourceID{i}")).unwrap_or_default());
                if s > 0 {
                    sources.push((s, int(&m(Some(r), &format!("variant{i}Allowed")).unwrap_or_default()) == 1));
                }
            }
            if result > 0 && !sources.is_empty() {
                ev.special.entry(result).or_default().push(Recipe { sources, plugin: int(&m(Some(r), "pluginID").unwrap_or_default()) });
            }
        }
    }

    // ---- the server's events, seasonal partials and zones ----
    let re_open = Regex::new(r#"(?s)<object name="EventOpenMenu">(.*?)</object>"#).expect("open");
    let re_shop = Regex::new(r#"<member name="shopID">(\d+)</member>"#).expect("shop");
    let re_add = Regex::new(r#"(?s)<object name="ActionAddRemoveItems">(.*?)</object>"#).expect("add");
    let re_pair = Regex::new(r"<key>(\d+)</key>\s*<value>(-?\d+)</value>").expect("pair");
    let re_comp = Regex::new(r#"(?s)<member name="addDemons">(.*?)</member>"#).expect("comp");
    let re_key = Regex::new(r"<key>(\d+)</key>").expect("key");
    let re_drop = Regex::new(r#"<member name="ItemType">(\d+)</member>"#).expect("drop");
    let mut openers: HashMap<i32, Vec<Place>> = HashMap::new(); // barter list -> where an event opens it
    for dir in ["events", "partials", "zones"] {
        let d = store.join(dir);
        if !d.is_dir() {
            continue;
        }
        for f in xml::enumerate_xml_files(&d, true)? {
            let Ok(text) = xml::read_text(&f) else { continue };
            let rel = f.strip_prefix(store).unwrap_or(&f).to_string_lossy().replace('\\', "/");
            let place = place_of(&rel);
            if dir != "zones" {
                if text.contains("EventOpenMenu") {
                    for c in re_open.captures_iter(&text) {
                        if !c[1].contains(r#"<member name="menuType">808</member>"#) {
                            continue;
                        }
                        if let Some(s) = re_shop.captures(&c[1]) {
                            push_place(&mut openers, int(&s[1]), place.clone());
                        }
                    }
                }
                if text.contains("ActionAddRemoveItems") {
                    for c in re_add.captures_iter(&text) {
                        for p in re_pair.captures_iter(&c[1]) {
                            if int(&p[2]) > 0 {
                                push_place(&mut ev.item_rewards, int(&p[1]), place.clone());
                            }
                        }
                    }
                }
                if text.contains("addDemons") {
                    for c in re_comp.captures_iter(&text) {
                        for k in re_key.captures_iter(&c[1]) {
                            if int(&k[1]) > 0 {
                                push_place(&mut ev.gifts, int(&k[1]), place.clone());
                            }
                        }
                    }
                }
            }
            if text.contains("ItemDrop") {
                for c in re_drop.captures_iter(&text) {
                    push_place(&mut ev.item_drops, int(&c[1]), place.clone());
                }
            }
        }
    }

    // ---- NPC exchanges (client table), only the ones an NPC list offers ----
    let (barter_file, group_file) = (decoded.join("NPCBarterData.xml"), decoded.join("NPCBarterGroupData.xml"));
    if barter_file.is_file() && group_file.is_file() {
        let group_text = xml::read_text(&group_file)?;
        let group_doc = xml::parse(&group_text)?;
        let mut groups_of: HashMap<i32, Vec<i32>> = HashMap::new();
        for g in xml::records(&group_doc).into_iter().filter(|o| o.attribute("name") == Some("MiNPCBarterGroupData")) {
            let gid = int(&m(Some(g), "ID").unwrap_or_default());
            if let Some(es) = mem(Some(g), "entries") {
                for e in elements(es, "element").filter_map(|e| element(e, "object")) {
                    let b = int(&m(Some(e), "barterID").unwrap_or_default());
                    if b != 0 {
                        groups_of.entry(b).or_default().push(gid);
                    }
                }
            }
        }
        let barter_text = xml::read_text(&barter_file)?;
        let barter_doc = xml::parse(&barter_text)?;
        let parts = |o: roxmltree::Node, member: &str| -> Vec<(String, i32, i32)> {
            mem(Some(o), member)
                .map(|ms| {
                    elements(ms, "element")
                        .filter_map(|e| element(e, "object"))
                        .map(|p| {
                            (
                                m(Some(p), "type").unwrap_or_default(),
                                int(&m(Some(p), "subtype").unwrap_or_default()),
                                int(&m(Some(p), "amount").unwrap_or_default()),
                            )
                        })
                        .filter(|(t, _, _)| !t.is_empty() && t != "NONE")
                        .collect()
                })
                .unwrap_or_default()
        };
        for b in xml::records(&barter_doc).into_iter().filter(|o| o.attribute("name") == Some("MiNPCBarterData")) {
            let id = int(&m(Some(b), "ID").unwrap_or_default());
            let Some(groups) = groups_of.get(&id) else { continue };
            let results: Vec<i32> = parts(b, "resultItems").into_iter().filter(|(t, _, _)| t == "ITEM").map(|(_, s, _)| s).collect();
            if results.is_empty() {
                continue;
            }
            let mut cost: Vec<String> = Vec::new();
            for (t, sub, n) in parts(b, "tradeItems") {
                cost.push(match t.as_str() {
                    "ITEM" => {
                        let name = ev.items.get(&sub).map(|i| i.name.clone()).unwrap_or_else(|| "an item the data does not name".to_string());
                        if n > 1 { format!("{} × {name}", fmt::n0(n as f64)) } else { name }
                    }
                    "SOUL_POINT" => format!("{} Soul Points", fmt::n0(n as f64)),
                    "COIN" => format!("{} Casino Coins", fmt::n0(n as f64)),
                    "BETHEL" => format!("{} Bethel", fmt::n0(n as f64)),
                    _ => continue, // event counters, cooldowns: conditions, not a price
                });
            }
            let cost = if cost.is_empty() { "nothing the data names".to_string() } else { cost.join(" + ") };
            let mut places: Vec<Place> = Vec::new();
            for g in groups {
                for p in openers.get(g).into_iter().flatten() {
                    if *p != Place::Nowhere && !places.contains(p) {
                        places.push(p.clone());
                    }
                }
            }
            for item in results {
                let v = ev.item_barters.entry(item).or_default();
                match v.iter_mut().find(|x| x.cost == cost) {
                    Some(x) => {
                        for p in &places {
                            if !x.places.contains(p) {
                                x.places.push(p.clone());
                            }
                        }
                    }
                    None => v.push(Barter { cost: cost.clone(), places: places.clone() }),
                }
            }
        }
    }
    Ok(ev)
}

/// "Shibuya, Ginza and 2 more" / "Summer 2014 event": the places as a person says them; empty when the data names none.
pub fn say_places(places: &[Place], zone_name: &HashMap<i32, String>) -> String {
    let mut zones: Vec<String> = Vec::new();
    let mut seasons: Vec<String> = Vec::new();
    for p in places {
        match p {
            Place::Zone(z) => {
                if let Some(n) = zone_name.get(z) {
                    if !zones.contains(n) {
                        zones.push(n.clone());
                    }
                }
            }
            Place::Season(s) => {
                let t = format!("{s} event");
                if !seasons.contains(&t) {
                    seasons.push(t);
                }
            }
            Place::Nowhere => {}
        }
    }
    let mut out: Vec<String> = Vec::new();
    if !zones.is_empty() {
        let shown: Vec<&str> = zones.iter().take(4).map(String::as_str).collect();
        let more = zones.len().saturating_sub(4);
        out.push(if more > 0 { format!("{} and {more} more", shown.join(", ")) } else { shown.join(", ") });
    }
    out.extend(seasons.into_iter().take(3));
    out.join("; ")
}

/// "How to get it" for a variant, as page lines. `aliases` = every demon type that is this same demon (the data has
/// the same demon under several ids); names are the client's. Each line: `kind`, `text`, an optional `note`, and for an
/// item the `from` lines that say where the item comes from (empty: the data does not say).
pub fn how_to_get(aliases: &[i32], names: &HashMap<i32, String>, ev: &Evidence, zone_name: &HashMap<i32, String>) -> Vec<Value> {
    let name = |id: i32| names.get(&id).cloned().filter(|n| !n.is_empty()).unwrap_or_else(|| "a demon the data does not name".to_string());
    let mut lines: Vec<Value> = Vec::new();
    let mut items: Vec<i32> = Vec::new();
    for a in aliases {
        for i in ev.exchange_of.get(a).into_iter().flatten() {
            if !items.contains(i) {
                items.push(*i);
            }
        }
    }
    items.sort();
    for item in items {
        let item_name = ev.items.get(&item).map(|i| i.name.clone()).unwrap_or_else(|| "an item the data does not name".to_string());
        let options = &ev.exchange[&item];
        let mine = |o: &Vec<i32>| o.iter().any(|t| aliases.contains(t));
        let others: Vec<String> = options.iter().filter(|o| !mine(o)).map(|o| o.iter().map(|t| name(*t)).collect::<Vec<_>>().join(" + ")).collect();
        let mut from: Vec<String> = Vec::new();
        for b in ev.item_barters.get(&item).into_iter().flatten() {
            let at = say_places(&b.places, zone_name);
            from.push(if at.is_empty() { format!("NPC exchange for {}", b.cost) } else { format!("NPC exchange for {}: {at}", b.cost) });
        }
        let rewarded = ev.item_rewards.get(&item).map(|p| say_places(p, zone_name)).unwrap_or_default();
        if ev.item_rewards.contains_key(&item) {
            from.push(if rewarded.is_empty() { "Reward from an event or NPC".to_string() } else { format!("Reward from an event or NPC: {rewarded}") });
        }
        let dropped = ev.item_drops.get(&item).map(|p| say_places(p, zone_name)).unwrap_or_default();
        if ev.item_drops.contains_key(&item) {
            from.push(if dropped.is_empty() { "Dropped by enemies or in chests".to_string() } else { format!("Dropped by enemies or in chests: {dropped}") });
        }
        let mut line = Map::new();
        line.insert("kind".into(), json!("Exchange item"));
        line.insert("text".into(), json!(if options.len() > 1 { format!("Use {item_name} and pick it") } else { format!("Use {item_name}") }));
        line.insert("note".into(), json!(if others.is_empty() { Value::Null } else { json!(format!("The same item also offers: {}", others.join(", "))) }));
        line.insert("from".into(), json!(from));
        lines.push(Value::Object(line));
    }
    for a in aliases {
        for r in ev.special.get(a).into_iter().flatten() {
            let text = r.sources.iter().map(|(s, any)| if *any { format!("{} (or a variant of it)", name(*s)) } else { name(*s) }).collect::<Vec<_>>().join(" + ");
            let mut line = Map::new();
            line.insert("kind".into(), json!("Special fusion"));
            line.insert("text".into(), json!(text));
            let plugin = if r.plugin > 0 {
                Some(match ev.plugins.get(&r.plugin) {
                    Some(p) => format!("Needs the key item {p}"),
                    None => "Needs a fusion plugin the data does not name".to_string(),
                })
            } else {
                None
            };
            line.insert("note".into(), json!(plugin));
            lines.push(Value::Object(line));
        }
    }
    let mut gift_places: Vec<Place> = Vec::new();
    let mut gifted = false;
    for a in aliases {
        if let Some(ps) = ev.gifts.get(a) {
            gifted = true;
            for p in ps {
                if !gift_places.contains(p) {
                    gift_places.push(p.clone());
                }
            }
        }
    }
    if gifted {
        let at = say_places(&gift_places, zone_name);
        let mut line = Map::new();
        line.insert("kind".into(), json!("Gift"));
        line.insert("text".into(), json!(if at.is_empty() { "An event or NPC gives it to you".to_string() } else { format!("An event or NPC gives it to you: {at}") }));
        lines.push(Value::Object(line));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_stands_for_a_zone_or_a_season() {
        assert_eq!(place_of("events/zone_events-30101.xml"), Place::Zone(30101));
        assert_eq!(place_of("zones/zone-1050501.xml"), Place::Zone(1050501));
        assert_eq!(place_of("events/dungeon_events-51.xml"), Place::Nowhere);
        assert_eq!(place_of("zones/partial/global.xml"), Place::Nowhere);
        assert_eq!(place_of("partials/201408_summer/events/extra/201408_summer.xml"), Place::Season("Summer 2014".into()));
    }

    #[test]
    fn how_to_get_names_the_item_and_where_it_comes_from() {
        let mut ev = Evidence::default();
        ev.items.insert(21931, ItemInfo { name: "Summon PG: Open-Minded (♀)".into(), sub: 68 });
        ev.items.insert(21932, ItemInfo { name: "Summon PG: Open-Minded (♂)".into(), sub: 68 });
        ev.exchange.insert(21931, vec![vec![4852], vec![4853], vec![4854]]);
        ev.exchange.insert(21932, vec![vec![4855]]);
        ev.exchange_of.insert(4854, vec![21931]);
        ev.exchange_of.insert(4855, vec![21932]);
        ev.item_barters.insert(21931, vec![Barter { cost: "4,000 × Tears".into(), places: vec![Place::Season("Summer 2014".into())] }]);
        let names: HashMap<i32, String> = [(4852, "Open-minded Lakshmi"), (4853, "Open-minded Lilith"), (4854, "Open-minded Ishtar"), (4855, "Open-minded Rafael")]
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        let lines = how_to_get(&[4854], &names, &ev, &HashMap::new());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["kind"], "Exchange item");
        assert_eq!(lines[0]["text"], "Use Summon PG: Open-Minded (♀) and pick it");
        assert_eq!(lines[0]["note"], "The same item also offers: Open-minded Lakshmi, Open-minded Lilith");
        assert_eq!(lines[0]["from"][0], "NPC exchange for 4,000 × Tears: Summer 2014 event");
        // an item nothing in the data hands out says so with an empty list, and a single-demon item has no choices to name
        let lines = how_to_get(&[4855], &names, &ev, &HashMap::new());
        assert_eq!(lines[0]["text"], "Use Summon PG: Open-Minded (♂)");
        assert!(lines[0]["note"].is_null());
        assert_eq!(lines[0]["from"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn special_fusions_and_gifts_are_ways_too() {
        let mut ev = Evidence::default();
        ev.plugins.insert(3, "Plugin: Zhu Que".into());
        ev.special.insert(10, vec![Recipe { sources: vec![(1, true), (2, false)], plugin: 3 }]);
        ev.gifts.insert(10, vec![Place::Nowhere]);
        let names: HashMap<i32, String> = [(1, "Pixie"), (2, "Angel"), (10, "Lucky Pixie")].into_iter().map(|(k, v)| (k, v.to_string())).collect();
        let lines = how_to_get(&[10], &names, &ev, &HashMap::new());
        assert_eq!(lines[0]["kind"], "Special fusion");
        assert_eq!(lines[0]["text"], "Pixie (or a variant of it) + Angel");
        assert_eq!(lines[0]["note"], "Needs the key item Plugin: Zhu Que");
        assert_eq!(lines[1]["kind"], "Gift");
        assert_eq!(lines[1]["text"], "An event or NPC gives it to you");
    }

    #[test]
    fn places_are_said_in_words() {
        let names: HashMap<i32, String> = [(1, "Shibuya".to_string()), (2, "Ginza".to_string())].into();
        assert_eq!(say_places(&[Place::Zone(1), Place::Zone(2), Place::Season("Summer 2014".into())], &names), "Shibuya, Ginza; Summer 2014 event");
        assert_eq!(say_places(&[Place::Nowhere, Place::Zone(99)], &names), "");
    }
}
