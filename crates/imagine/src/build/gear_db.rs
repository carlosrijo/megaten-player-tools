//! Port of `src/Build/GearDb.cs`.
//!
//! The Item Database page's data (app\ui\gear-db.json; the file keeps its old name): every item in the IMAGINE client
//! with what it gives or does, plus where the game data and the community's spreadsheets say it comes from. Gear is
//! what it always was (stats, effects, sets); the other kinds (consumables, materials, vouchers, tarot and soul crystals,
//! demon equipment, appearance items, key items) carry the game's own description, and a voucher or box lists what it
//! can give.
//!
//! What an item gives is read from the client's CURRENT tables, decoded to XML beforehand (reference\README.md):
//!   ItemData.xml          equip type, gender, alignment, weapon type and affinity, mod slots, the correct table (stats),
//!                         the sub-category (the item class the game shows, reference\item-classes.tsv), stack and price
//!   SItemData.xml         the item's special-effect tokusei
//!   EquipmentSetData.xml  sets (several pieces) and appearance bonuses (one-piece sets)
//!   CItemData.xml         names, descriptions and icon numbers
//!   CKeyItemData.xml      key items (plugins, passes): their own id space, name and description; how to get a fusion
//!                         plugin comes from the Fusion Planner's data (app\ui\fusion-data.json beside the output)
//! Tarot and soul crystals: the enchant table (`GameData::enchant_sides`) names the crystal item of each enchant side;
//! the side's effects are worded by `ItemEffects::enchant`, as on the Gear page for a fused crystal.
//! Icons: the icon number goes through the client's CIconData_Item.bin to its picture (Interface\tga or tga2, as for
//! the Gear page's icons), saved once as app\ui\gear-db-icons\<picture>.png beside the json (game art, not in git).
//! Tokusei are turned into text by the reader's own ItemEffects (the code that works out the worn gear's bonuses).
//! The website's items.json is NOT used here: on 2026-11-23 it was two months older than the client and wrong for
//! items IMAGINE had rebalanced since (Renki Crystallized Spirit 48074: damage 44% there, 30% in the client).
//!
//! The community notes are SnowSheet's gear reference (smt-gear-analysis\data\sheet_gear.tsv, rows with source_sheet
//! "gear_reference"; the Re:FC rows in that file are another server's and are skipped): matched by item id where the
//! sheet gives one, else by name. Sheet rows that match no game item
//! are kept as entries of their own, marked as coming from the sheet only.
//!
//! Differences from C#, forced by the language: the decoded tables are parsed whole (roxmltree) and walked record by
//! record instead of streamed; the C# statics (`Openers`, `Names`, `Raw`) are a [`Sources`] value passed down.

// The checks keep the C# shape (nested ifs, index loops) so each line can be compared with the original.
#![allow(clippy::collapsible_if, clippy::needless_range_loop, clippy::type_complexity, clippy::too_many_arguments, clippy::manual_range_contains)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::item_icons;
use std::sync::{Arc, LazyLock};

use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::Result;
use crate::calc::effect::Effect;
use crate::calc::item_effects::{ItemEffects, affinity_name, to_title_case};
use crate::data::game_data::{GameData, int_parse, member_text, parse_xml, xml_children, xml_value};
use crate::data::json::{self, At, Escape};
use crate::fmt;

type Obj = Map<String, Value>;
type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

// "stats" = the item's own CorrectTbl lines (the tooltip's Basic Features); "effects" = its special-effect tokusei (Characteristics).

fn slot_name(equip: &str) -> Option<&'static str> {
    Some(match equip {
        "EQUIP_TYPE_HEAD" => "Head",
        "EQUIP_TYPE_FACE" => "Face",
        "EQUIP_TYPE_NECK" => "Neck",
        "EQUIP_TYPE_TOP" => "Top",
        "EQUIP_TYPE_ARMS" => "Arms",
        "EQUIP_TYPE_BOTTOM" => "Bottom",
        "EQUIP_TYPE_FEET" => "Feet",
        "EQUIP_TYPE_COMP" => "COMP",
        "EQUIP_TYPE_RING" => "Ring",
        "EQUIP_TYPE_EARRING" => "Earring",
        "EQUIP_TYPE_EXTRA" => "Extra",
        "EQUIP_TYPE_BACK" => "Back",
        "EQUIP_TYPE_TALISMAN" => "Talisman",
        "EQUIP_TYPE_WEAPON" => "Weapon",
        "EQUIP_TYPE_BULLETS" => "Bullets",
        _ => return None,
    })
}

/// The page's kind of a non-gear item, from the game's sub-category (the item class, reference\item-classes.tsv) and the
/// main category (3 = the tarot and soul crystals, which the client classes as consumables). Gear is decided by its equip
/// type before this is asked. The kinds are the page's filter chips; the class stays beside it as the finer filter.
fn kind_name(sub: i32, main: i32, equip: &str) -> &'static str {
    if equip.starts_with("VIS_") || sub == 18 {
        return "Appearance"; // hair, face and eye types and colours
    }
    match sub {
        60 if main == 3 => "Enchant",
        60 | 81 | 82 => "Consumable",             // Consumable, Demon Dungeon Only, Diaspora Only
        63 | 64 | 65 | 66 | 69 | 70 | 84 => "Material", // Junk, Material, Disassembly Tool, Weapon/Armor Mod Tool, Synthesis Support, Spiritual Infusion
        19 | 67 | 68 | 102 => "Voucher",          // Ticket, Item Voucher, Demon Voucher, Voucher
        83 => "Demon equipment",
        _ => "Other", // Quest Item, Money, Mount Item, Other and anything the client has no class for
    }
}

static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("ws"));
static RE_COUNT_X: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\d,]+ × ").expect("count x"));
static RE_NON_KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").expect("key"));
static RE_THE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^the\s+").expect("the"));
static RE_NON_LOOSE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-rt-z0-9]+").expect("loose key"));
static RE_NO_WORDS_SP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z][A-Z0-9_]{3,} ").expect("no words"));
static RE_NO_WORDS_B: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z][A-Z0-9_]{3,}\b").expect("no words"));
static RE_TICKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)ticket|\bDT\b|voucher|plate").expect("ticket"));
static RE_ITEM_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Item ID:\s*(\d+)").expect("item id"));
static RE_HOW_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*(?:\|\||;;|>)\s*").expect("how split"));
static RE_DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").expect("digits"));
static RE_JP_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"JP name:[^;]*;?").expect("jp name"));
static RE_NOTE_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[;]|\s/\s").expect("note split"));
static RE_SEASON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{4})\d{2}_(.+)$").expect("season"));
/// "Hand over Abyss Seed to Yagiya in Suginami Tunnels", "Hand over A + 20 × B to ...": what a fusion plugin is traded
/// for (the items are split on " + " and their counts dropped by the caller)
static RE_HAND_OVER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Hand over (.+?) to ").expect("hand over"));
static RE_COUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d[\d,]* × ").expect("count"));

/// `Regex.Replace(s, @"\s+", " ").Trim()`
fn squash(s: &str) -> String {
    RE_WS.replace_all(s, " ").trim().to_string()
}

/// `Int(s)`: `int.TryParse` or 0.
fn int(s: Option<&str>) -> i32 {
    s.and_then(fmt::parse_i32).unwrap_or(0)
}

fn key(name: &str) -> String {
    RE_NON_KEY.replace_all(&name.to_lowercase(), "").into_owned()
}

/// A looser name key for a second try: sheet names differ from the client's in "The", plurals and possessives.
fn loose_key(name: &str) -> String {
    let lower = name.to_lowercase();
    RE_NON_LOOSE.replace_all(&RE_THE.replace(&lower, ""), "").into_owned()
}

/// The first `member` of `o` called `name`.
fn member<'a, 'i>(o: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    xml_children(o, "member").find(|m| m.attribute("name") == Some(name))
}

/// `M(o, name)`: the member's text, trimmed.
fn m(o: Option<Node>, name: &str) -> Option<String> {
    member_text(o?, name)
}

/// `Obj(o, name)`: the object inside the member.
fn obj<'a, 'i>(o: Option<Node<'a, 'i>>, name: &str) -> Option<Node<'a, 'i>> {
    xml_children(member(o?, name)?, "object").next()
}

/// The `element` children of a member, as numbers (`Int(e.Value)`), zeros dropped.
fn int_list(o: Node, name: &str) -> Option<Vec<i32>> {
    Some(xml_children(member(o, name)?, "element").map(|e| int(Some(&xml_value(e)))).filter(|&x| x != 0).collect())
}

/// `Descendants("member").FirstOrDefault(m => name == ...)`: the first member (at any depth) called `name`.
fn first_member_desc<'a, 'i>(o: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    o.descendants().find(|n| n.is_element() && n.tag_name().name() == "member" && n.attribute("name") == Some(name))
}

/// Top-level records of a big decoded table.
fn records<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    doc.root_element().children().filter(|n| n.is_element() && n.tag_name().name() == "object")
}

/// A line the reader has no words for comes out as its raw table id ("TALK_RATE 2"): such lines are not shown.
pub(crate) fn no_words(line: &str) -> bool {
    RE_NO_WORDS_B.is_match(line)
}

pub(crate) fn text(e: &Effect, data: &GameData) -> Result<String> {
    // an effect the server data lacks: the game's own description of it, never the internal number
    if e.label.starts_with("Unknown effect") && e.tokusei > 0 {
        let own = squash(&data.client_tokusei_text(e.tokusei)?.unwrap_or_default());
        return Ok(if !own.is_empty() { own } else { "An effect the game data does not describe".to_string() });
    }
    let v = match e.value {
        Some(d) => format!(" {}{}{}", if d > 0.0 { "+" } else if d < 0.0 { "−" } else { "" }, fmt::custom(d.abs(), "0.##"), if e.pct { "%" } else { "" }),
        None => String::new(),
    };
    let who = if e.target == "self" || e.target.is_empty() {
        String::new()
    } else {
        let mut cs = e.target.chars();
        let first = cs.next().map(upper_char).unwrap_or_default();
        format!("{first}{}: ", cs.as_str())
    };
    let note = if e.cond == "always" {
        String::new()
    } else if !e.cond_note.is_empty() {
        e.cond_note.clone()
    } else if e.cond == "digitalized" {
        "while digitalized".to_string()
    } else if e.cond == "situational" {
        "depends on the fight".to_string()
    } else {
        String::new()
    };
    Ok(format!("{who}{}{v}{}", e.label, if !note.is_empty() { format!(" ({note})") } else { String::new() }))
}

/// `char.ToUpperInvariant(c)`: one char to one char (a char with a longer upper-case form stays as it is).
fn upper_char(c: char) -> char {
    let mut u = c.to_uppercase();
    match (u.next(), u.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

/// `Distinct(JsonArray)`: the first of each equal element.
fn distinct(a: Vec<Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    a.into_iter().filter(|x| seen.insert(x.to_string())).collect()
}

/// The in-game sources of the items, and the names and opening items they refer to (C# statics `Openers`, `Names`, `Raw`).
#[derive(Default)]
struct Sources {
    /// random box number -> the items that open it
    openers: IndexMap<i32, Vec<i32>>,
    /// (zone, spawn location group) -> the items whose use summons it there (skill function 322); zone 0: a global
    /// partial group or location group, summoned wherever the item is used (function 324: the Demon Eggs)
    summoners: HashMap<(i32, i32), Vec<i32>>,
    /// a demon -> its special fusions (the Fusion Planner's fusion-data.json beside the output): the demons fused, and
    /// the plugin it needs with the ways to get it. "Crystallize <demon>" shows them: crystallizing needs the demon.
    special_fusions: HashMap<i32, Vec<SpecialFusion>>,
    /// item name -> id, to follow "Hand over <item> to ..." in a plugin's ways to get it
    item_by_name: HashMap<String, i32>,
    names: IndexMap<i32, String>,
    /// item id -> its raw in-game sources
    raw: HashMap<i32, Value>,
}

/// `JsonNode.ToString()` of a property as `Str(o, k)`: a string as is, other JSON as its text, "" when missing.
fn str_of(o: &Obj, k: &str) -> String {
    match o.get(k) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

fn dbl(o: &Obj, k: &str) -> f64 {
    fmt::parse_f64(&str_of(o, k)).unwrap_or(0.0)
}

fn pct(r: f64) -> String {
    format!("{}%", fmt::custom(r, "0.##"))
}

/// The distinct non-empty values of a field over a group, first appearance order.
fn distinct_field(g: &[&Obj], k: &str) -> Vec<String> {
    let mut seen = IndexSet::new();
    for x in g {
        let s = str_of(x, k);
        if !s.is_empty() {
            seen.insert(s);
        }
    }
    seen.into_iter().collect()
}

fn zones(g: &[&Obj]) -> String {
    let z = distinct_field(g, "zone");
    if z.len() > 5 { format!("{} and {} more", z[..5].join(", "), z.len() - 5) } else { z.join(", ") }
}

fn season(o: &Obj) -> String {
    match RE_SEASON.captures(&str_of(o, "season")) {
        Some(c) => {
            let g2 = &c[2];
            let mut cs = g2.chars();
            let first = cs.next().map(upper_char).unwrap_or_default();
            format!(" ({first}{} {} event)", cs.as_str().replace('_', " "), &c[1])
        }
        None => String::new(),
    }
}

/// `GroupBy(key)`: the groups in first-appearance order.
fn group_by<'a>(src: &[&'a Obj], kind: impl Fn(&Obj) -> bool, key: impl Fn(&Obj) -> String) -> Vec<(String, Vec<&'a Obj>)> {
    let mut groups: IndexMap<String, Vec<&'a Obj>> = IndexMap::new();
    for x in src.iter().filter(|x| kind(x)) {
        groups.entry(key(x)).or_default().push(x);
    }
    groups.into_iter().collect()
}

/// One line of the "how to get it" list, with its sort keys and the lines under it.
struct Line {
    rank: i32,
    sort: f64,
    obj: Obj,
    kids: Option<Vec<Obj>>,
}

fn line(depth: i32, k: &str, t: &str, s: &str, v: &str) -> Obj {
    let mut o = Obj::new();
    o.insert("k".into(), json!(k));
    o.insert("t".into(), json!(t));
    if !s.is_empty() {
        o.insert("s".into(), json!(s));
    }
    if !v.is_empty() {
        o.insert("v".into(), json!(v));
    }
    if depth > 0 {
        o.insert("d".into(), json!(depth));
    }
    o
}

struct SpecialFusion {
    demons: Vec<String>,
    plugin: Option<(String, Vec<String>)>,
}

/// The raw in-game sources of one item (tools\dcmexport --ids: drops, chests, shops, events, boxes from the server's
/// files) as lines a player can act on: { k: kind, t: who or where, s: detail, v: rate or price }. Best first.
fn get_lines(sources: &Sources, raw: &Value, depth: i32, visited: &mut HashSet<i32>) -> Vec<Obj> {
    let src_all: Vec<&Obj> = match raw {
        Value::Array(a) => a.iter().filter_map(Value::as_object).collect(),
        _ => Vec::new(),
    };
    let mut res: Vec<Line> = Vec::new();
    let no_place = "The data does not say where";
    let ln = |k: &str, t: &str, s: &str, v: &str| line(depth, k, t, s, v);
    let kind_is = |k: &'static str| move |x: &Obj| str_of(x, "kind") == k;

    for (_, g) in group_by(&src_all, kind_is("shop"), |x| format!("{}|{}", str_of(x, "shopId"), str_of(x, "price"))) {
        // the shop's name, then where it is and who runs it (the NPC and zone whose event opens the shop)
        let f = g[0];
        let at = if !str_of(f, "zone").is_empty() { format!(" in {}", str_of(f, "zone")) } else { String::new() };
        let detail = [
            if !str_of(f, "npc").is_empty() { format!("talk to {}", str_of(f, "npc")) } else { String::new() },
            if !str_of(f, "tab").is_empty() { format!("tab: {}", str_of(f, "tab")) } else { String::new() },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        res.push(Line {
            rank: 1,
            sort: dbl(f, "price"),
            obj: ln("Buy", &format!("{}{at}", str_of(f, "shop")), &detail, &format!("{} {}", fmt::n0(dbl(f, "price")), str_of(f, "currency"))),
            kids: None,
        });
    }
    // an NPC exchange: who, where, and what it costs (the client's own exchange tables). One line per cost:
    // the same trade is offered by the same NPC in several places.
    for (cost, g) in group_by(&src_all, kind_is("barter"), |x| str_of(x, "cost")) {
        let with_place: Vec<&Obj> = g.iter().copied().filter(|x| !str_of(x, "npc").is_empty() || !str_of(x, "zone").is_empty()).collect();
        let mut by_npc: IndexMap<String, Vec<&Obj>> = IndexMap::new();
        for x in with_place {
            by_npc.entry(str_of(x, "npc")).or_default().push(x);
        }
        let people: Vec<String> = by_npc
            .iter()
            .map(|(npc, p)| {
                let zs = distinct_field(p, "zone");
                let place = if zs.is_empty() {
                    String::new()
                } else {
                    format!(
                        " in {}{}",
                        zs.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
                        if zs.len() > 3 { format!(" and {} more place{}", zs.len() - 3, if zs.len() - 3 == 1 { "" } else { "s" }) } else { String::new() }
                    )
                };
                format!("{}{place}", if !npc.is_empty() { npc.as_str() } else { "an NPC" })
            })
            .collect();
        let who = if !people.is_empty() {
            format!(
                "Talk to {}{}",
                people.iter().take(3).cloned().collect::<Vec<_>>().join("; or "),
                if people.len() > 3 { format!("; or {} others", people.len() - 3) } else { String::new() }
            )
        } else {
            "An exchange NPC the data does not place".to_string()
        };
        res.push(Line { rank: if !people.is_empty() { 1 } else { 6 }, sort: 0.5, obj: ln("Exchange", &who, &if !cost.is_empty() { format!("for {cost}") } else { String::new() }, ""), kids: None });
    }
    // the items that summon the enemy of a drop (dcmexport "sg" / "slg": its spawn group and location groups; "global":
    // in the global partial), empty for an enemy that spawns by itself
    let summoned_by = |x: &Obj| -> Vec<i32> {
        let ids = |k: &str| -> Vec<i32> { x.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).map(|v| v as i32).collect()).unwrap_or_default() };
        let mut res: Vec<i32> = Vec::new();
        if x.get("global").and_then(Value::as_bool) == Some(true) {
            for g in ids("slg").into_iter().chain(ids("sg")) {
                res.extend(sources.summoners.get(&(0, g)).map(Vec::as_slice).unwrap_or(&[]));
            }
        } else if dbl(x, "zoneId") as i32 != 0 {
            // (a drop written in a zone partial has zone 0: that is not the global partial, so it matches nothing)
            let zone = dbl(x, "zoneId") as i32;
            for g in ids("slg") {
                res.extend(sources.summoners.get(&(zone, g)).map(Vec::as_slice).unwrap_or(&[]));
            }
        }
        res.sort_unstable();
        res.dedup();
        res
    };
    let summoned: Vec<&Obj> = src_all.iter().copied().filter(|x| str_of(x, "kind") == "drop" && !summoned_by(x).is_empty()).collect();
    for x in &summoned {
        let by = summoned_by(x);
        let names: Vec<String> = by.iter().take(2).map(|i| sources.names.get(i).cloned().unwrap_or_else(|| "an item".to_string())).collect();
        let global = x.get("global").and_then(Value::as_bool) == Some(true);
        let z = str_of(x, "zone");
        let s = format!("summoned with {}{}", names.join(" or "), if global { " (used in a field or dungeon)".to_string() } else if !z.is_empty() { format!(" in {z}") } else { String::new() });
        let mut below: Vec<Obj> = Vec::new();
        for &it in by.iter().take(2) {
            if depth < 3 && visited.insert(it) {
                if let Some(r) = sources.raw.get(&it) {
                    below.extend(get_lines(sources, r, depth + 1, visited));
                }
            }
        }
        res.push(Line { rank: 2, sort: -dbl(x, "rate"), obj: ln("Enemy drop", &format!("{}{}", str_of(x, "enemy"), season(x)), &s, &pct(dbl(x, "rate"))), kids: if below.is_empty() { None } else { Some(below) } });
    }
    let plain: Vec<&Obj> = src_all.iter().copied().filter(|x| !summoned.iter().any(|y| std::ptr::eq(*y, *x))).collect();
    for (_, g) in group_by(&plain, kind_is("drop"), |x| format!("{}|{}|{}", str_of(x, "enemy"), str_of(x, "rate"), str_of(x, "season"))) {
        let z = zones(&g);
        res.push(Line {
            rank: 2,
            sort: -dbl(g[0], "rate"),
            obj: ln("Enemy drop", &format!("{}{}", str_of(g[0], "enemy"), season(g[0])), if !z.is_empty() { &z } else { no_place }, &pct(dbl(g[0], "rate"))),
            kids: None,
        });
    }
    for (_, g) in group_by(&src_all, |x| matches!(str_of(x, "kind").as_str(), "loot" | "chest"), |x| format!("{}{}|{}", str_of(x, "ref"), str_of(x, "dropSetId"), str_of(x, "rate"))) {
        let z = zones(&g);
        let desc = str_of(g[0], "desc");
        let t = if !z.is_empty() {
            format!("{z}{}", season(g[0]))
        } else if !desc.is_empty() {
            desc
        } else {
            no_place.to_string()
        };
        res.push(Line { rank: 3, sort: -dbl(g[0], "rate"), obj: ln("Chest", &t, "", &pct(dbl(g[0], "rate"))), kids: None });
    }
    for (_, g) in group_by(&src_all, kind_is("plasma"), |x| format!("{}|{}", str_of(x, "zone"), str_of(x, "rate"))) {
        let z = str_of(g[0], "zone");
        res.push(Line { rank: 3, sort: -dbl(g[0], "rate"), obj: ln("Plasma", if !z.is_empty() { &z } else { no_place }, "", &pct(dbl(g[0], "rate"))), kids: None });
    }
    for (_, g) in group_by(&src_all, kind_is("event"), |x| format!("{}|{}", str_of(x, "zone"), str_of(x, "season"))) {
        let who = distinct_field(&g, "npc");
        let z = str_of(g[0], "zone");
        let t = format!("{}{}", if !z.is_empty() { z.as_str() } else { no_place }, season(g[0]));
        let s = if !who.is_empty() {
            format!("Talk to {}{}", who.iter().take(4).cloned().collect::<Vec<_>>().join(", "), if who.len() > 4 { format!(" and {} more", who.len() - 4) } else { String::new() })
        } else {
            String::new()
        };
        res.push(Line { rank: 4, sort: 0.0, obj: ln("Event or NPC", &t, &s, ""), kids: None });
    }
    // crystallizing a demon (gear_db's own, from EnchantData): the demon it is made from
    for x in src_all.iter().filter(|x| str_of(x, "kind") == "crystallize") {
        // the demon itself: its special fusions, each with the plugin it needs and the plugin's ways to get it; a way
        // that hands over an item ("Hand over Abyss Seed to Yagiya ...") has where that item comes from under it
        let demon = str_of(x, "demon");
        let mut below: Vec<Obj> = Vec::new();
        if depth < 2 {
            // a plugin's ways once per crystal: a later fusion that needs the same plugin points back to them
            let mut shown: HashSet<String> = HashSet::new();
            for sp in sources.special_fusions.get(&(dbl(x, "demonId") as i32)).map(Vec::as_slice).unwrap_or(&[]) {
                let first = sp.plugin.as_ref().is_none_or(|(n, _)| shown.insert(n.clone()));
                let needs = sp.plugin.as_ref().map_or("no plugin needed".to_string(), |(n, _)| format!("needs {n}{}", if first { "" } else { " (ways above)" }));
                below.push(line(depth + 1, "Special fusion", &sp.demons.join(" + "), &needs, ""));
                if let Some((_, how)) = sp.plugin.as_ref().filter(|_| first) {
                    for w in how {
                        below.push(line(depth + 2, "Plugin", w, "", ""));
                        let handed: Vec<i32> = RE_HAND_OVER
                            .captures(w)
                            .map(|c| c[1].split(" + ").filter_map(|x| sources.item_by_name.get(RE_COUNT.replace(x.trim(), "").trim()).copied()).collect())
                            .unwrap_or_default();
                        for it in handed {
                            if depth + 3 <= 3 && visited.insert(it) {
                                if let Some(r) = sources.raw.get(&it) {
                                    // several items: say which one each block is about
                                    let lines = get_lines(sources, r, depth + 3, visited);
                                    if !lines.is_empty() && w.contains(" + ") {
                                        below.push(line(depth + 3, "For", sources.names.get(&it).map_or("", String::as_str), "", ""));
                                    }
                                    below.extend(lines);
                                }
                            }
                        }
                    }
                }
            }
        }
        res.push(Line { rank: 2, sort: 0.0, obj: ln("Crystallize", &demon, "crystallize this demon", ""), kids: if below.is_empty() { None } else { Some(below) } });
    }
    // synthesis (gear_db's own, from SynthesisData): the materials and the crafting skill
    for x in src_all.iter().filter(|x| str_of(x, "kind") == "craft") {
        let (skill, recipe) = (str_of(x, "skill"), str_of(x, "recipe"));
        let n = dbl(x, "count");
        let s = [
            if !skill.is_empty() { format!("with {skill}") } else { String::new() },
            if !recipe.is_empty() { format!("recipe “{recipe}”") } else { String::new() },
            if n > 1.0 { format!("makes {}", fmt::n0(n)) } else { String::new() },
        ]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let mats = str_of(x, "materials");
        res.push(Line { rank: 2, sort: 0.0, obj: ln("Craft", if !mats.is_empty() { &mats } else { "Materials the data does not list" }, &s, ""), kids: None });
    }
    // items an event gives by script (dcmexport kind "reward": a dungeon's reward roll, a mailed item, one of a few in
    // the loot), with their chance; one line per place, way and chance
    for (_, g) in group_by(&src_all, kind_is("reward"), |x| format!("{}|{}|{}|{}", str_of(x, "zone"), str_of(x, "season"), str_of(x, "how"), str_of(x, "rate"))) {
        let z = str_of(g[0], "zone");
        let t = format!("{}{}", if !z.is_empty() { z.as_str() } else { no_place }, season(g[0]));
        let who = distinct_field(&g, "npc");
        let how = str_of(g[0], "how");
        let s = [how, if !who.is_empty() { format!("from {}", who.iter().take(3).cloned().collect::<Vec<_>>().join(", ")) } else { String::new() }]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let rate = dbl(g[0], "rate");
        res.push(Line { rank: 4, sort: -rate, obj: ln("Event reward", &t, &s, &pct(rate)), kids: None });
    }
    for x in src_all.iter().filter(|x| str_of(x, "kind") == "box") {
        // the item that opens this box, and where that item comes from in turn
        if let Some(ops) = sources.openers.get(&(dbl(x, "giftBox") as i32)) {
            for &op in ops.iter().take(3) {
                let open = ln("Open", sources.names.get(&op).map_or("a box", String::as_str), "", &pct(dbl(x, "rate")));
                let mut below: Vec<Obj> = Vec::new();
                if depth < 3 && visited.insert(op) {
                    if let Some(op_raw) = sources.raw.get(&op) {
                        below.extend(get_lines(sources, op_raw, depth + 1, visited));
                    }
                }
                if below.is_empty() {
                    let mut o = Obj::new();
                    o.insert("k".into(), json!(""));
                    o.insert("t".into(), json!("The game data does not say where this comes from"));
                    o.insert("d".into(), json!(depth + 1));
                    below.push(o);
                }
                res.push(Line { rank: 5, sort: -dbl(x, "rate"), obj: open, kids: Some(below) });
            }
            if ops.len() > 3 {
                res.push(Line { rank: 5, sort: 1.0, obj: ln("", &format!("or {} other items that open the same box", ops.len() - 3), "", ""), kids: None });
            }
        } else {
            res.push(Line { rank: 5, sort: -dbl(x, "rate"), obj: ln("Random box", "A box the game data does not link to an item", "", &pct(dbl(x, "rate"))), kids: None });
        }
    }
    for (_, g) in group_by(&src_all, |x| matches!(str_of(x, "kind").as_str(), "destiny" | "demonquest" | "dropset"), |x| format!("{}|{}|{}", str_of(x, "kind"), str_of(x, "desc"), str_of(x, "rate"))) {
        let kind = str_of(g[0], "kind");
        let desc = str_of(g[0], "desc");
        res.push(Line {
            rank: 6,
            sort: -dbl(g[0], "rate"),
            obj: ln(
                if kind == "destiny" {
                    "Destiny box"
                } else if kind == "demonquest" {
                    "Demon quest"
                } else {
                    "Drop table"
                },
                if !desc.is_empty() { &desc } else { "A drop table" },
                if kind == "dropset" { "The data does not say which enemy uses this table" } else { "" },
                &pct(dbl(g[0], "rate")),
            ),
            kids: None,
        });
    }
    for x in src_all.iter().filter(|x| str_of(x, "kind") == "present") {
        res.push(Line { rank: 7, sort: 0.0, obj: ln("Demon present", &format!("From {} as your partner", str_of(x, "demon")), &format!("{} present", str_of(x, "tier")), ""), kids: None });
    }

    // OrderBy(rank).ThenBy(sort): stable
    res.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.sort.partial_cmp(&b.sort).unwrap_or(std::cmp::Ordering::Equal)));
    let mut ordered = res;
    let key_of = |l: &Line| Value::Object(l.obj.clone()).to_string();
    // "Open X" with nothing known about X is noise next to a line that does say where to go
    let dead_end = |l: &Line| l.kids.as_ref().is_some_and(|b| b.len() == 1 && b[0].get("k").and_then(Value::as_str) == Some(""));
    if ordered.iter().any(|l| !dead_end(l)) {
        ordered.retain(|l| !dead_end(l));
    }
    let cap = if depth == 0 { 14 } else { 5 };
    let mut parents = 0;
    let mut seen: HashSet<String> = HashSet::new();
    let mut outp: Vec<Obj> = Vec::new();
    for l in &ordered {
        if !seen.insert(key_of(l)) || parents >= cap {
            continue;
        }
        parents += 1;
        outp.push(l.obj.clone());
        for c in l.kids.iter().flatten() {
            outp.push(c.clone());
        }
    }
    let distinct_count = ordered.iter().map(key_of).collect::<HashSet<_>>().len() as i64;
    let left = distinct_count - parents as i64;
    if left > 0 {
        outp.push(line(depth, "", &format!("and {left} more places"), "", ""));
    }
    outp
}

/// `ids_out`: write the ids to trace (every item the client names) to this file and stop: the input of tools\dcmexport --ids.
/// `game_sources`: dcmexport's output for those ids: where each drops, is sold or handed out in the server's files.
pub fn build(data: Arc<GameData>, decoded: &str, sheet_tsv: Option<&str>, out_file: &str, ids_out: Option<&str>, game_sources: Option<&str>, client_dir: Option<&Path>) -> Result<()> {
    let mut sources = Sources::default();
    if let Some(gs) = game_sources.filter(|p| Path::new(p).is_file()) {
        let root = json::load(gs)?;
        for it in json::arr(root.at("items")) {
            if let Some(sa) = it.at("sources").filter(|s| s.as_array().is_some_and(|a| !a.is_empty())) {
                sources.raw.insert(json::int(it.at("id")), sa.clone());
            }
        }
    }
    // Neutral, level 99, no expertise: a requirement the item states stays visible as a note on the line.
    let fx = ItemEffects::new(Arc::clone(&data), "NEUTRAL", 99, IndexMap::new());
    let dec = Path::new(decoded);

    let mut name: IndexMap<i32, String> = IndexMap::new();
    let mut icon_key: HashMap<i32, i32> = HashMap::new();
    let mut desc: HashMap<i32, Vec<Value>> = HashMap::new();
    // the game's description, one line per line of the tooltip (the client breaks lines itself)
    let desc_lines = |s: &str| -> Vec<Value> { s.lines().map(|l| squash(l)).filter(|l| !l.is_empty()).map(|l| json!(l)).collect() };
    {
        let xml = json::read_all_text(&dec.join("CItemData.xml"))?;
        let doc = parse_xml(&xml)?;
        for c in records(&doc) {
            let b = obj(Some(c), "baseData");
            let n = squash(&m(b, "name").unwrap_or_default());
            if !n.is_empty() {
                let cid = int(m(b, "ID").as_deref());
                name.insert(cid, n);
                icon_key.insert(cid, int(m(b, "icon").as_deref()));
                let d = desc_lines(&m(b, "desc").unwrap_or_default());
                if !d.is_empty() {
                    desc.insert(cid, d);
                }
            }
        }
    }
    // Two ways the game makes items that no drop, shop or event lists (comp_hack DemonCrystallize, Synthesize):
    // crystallizing a demon gives the crystal EnchantData names for it (devilCrystal demonID -> itemID), and synthesis
    // crafts SynthesisData's item from its materials with a crafting skill. They join dcmexport's raw sources as kinds
    // "crystallize" and "craft" (get_lines words them). A table missing from the decoded folder is skipped.
    {
        let mut add = |item: i32, src: Value| {
            if item <= 0 {
                return;
            }
            let e = sources.raw.entry(item).or_insert_with(|| json!([]));
            if let Some(a) = e.as_array_mut() {
                a.push(src);
            }
        };
        if let Ok(xml) = json::read_all_text(&dec.join("EnchantData.xml")) {
            let mut demon_name: HashMap<i32, String> = HashMap::new();
            for line in crate::data::game_data::reference_lines("devils-game.tsv")?.unwrap_or_default() {
                let p: Vec<&str> = line.split('\t').collect();
                if p.len() > 1 && !line.starts_with('#') {
                    demon_name.insert(int(Some(p[0])), p[1].trim().to_string());
                }
            }
            let doc = parse_xml(&xml)?;
            for e in records(&doc) {
                let dc = obj(Some(e), "devilCrystal");
                let (demon, item) = (int(m(dc, "demonID").as_deref()), int(m(dc, "itemID").as_deref()));
                if demon > 0 {
                    let who = demon_name.get(&demon).cloned().unwrap_or_else(|| "a demon the data does not name".to_string());
                    add(item, json!({ "kind": "crystallize", "demonId": demon, "demon": who }));
                }
            }
        }
        if let Ok(xml) = json::read_all_text(&dec.join("SynthesisData.xml")) {
            let skills = data.skill_names()?;
            let doc = parse_xml(&xml)?;
            for r in records(&doc) {
                let item = int(m(Some(r), "itemID").as_deref());
                let mats: Vec<String> = member(r, "materials")
                    .map(|ms| {
                        ms.descendants()
                            .filter(|n| n.is_element() && n.tag_name().name() == "object")
                            .filter_map(|o| {
                                let (id, n) = (int(m(Some(o), "itemID").as_deref()), int(m(Some(o), "amount").as_deref()));
                                (id > 0 && n > 0).then(|| format!("{n} × {}", name.get(&id).cloned().unwrap_or_else(|| "an item the client does not name".to_string())))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // baseSkillID is the crafting skill ("Blacksmith"), skillID the recipe learned for this item ("Willow")
                let skill_of = |k: &str| skills.get(&int(m(Some(r), k).as_deref())).cloned().flatten().unwrap_or_default();
                add(item, json!({ "kind": "craft", "materials": mats.join(", "), "skill": skill_of("baseSkillID"), "recipe": skill_of("skillID"), "count": int(m(Some(r), "count").as_deref()) }));
            }
        }
    }
    // the crystal item of each enchant side: a tarot or soul crystal's item id -> (enchant id, is tarot)
    let mut enchant_of_item: HashMap<i32, Vec<(i32, bool)>> = HashMap::new();
    for (&(eid, is_tarot), side) in data.enchant_sides()?.iter() {
        if side.crystal_item > 0 {
            enchant_of_item.entry(side.crystal_item).or_default().push((eid, is_tarot));
        }
    }
    let mut special: HashMap<i32, Vec<i32>> = HashMap::new();
    {
        let xml = json::read_all_text(&dec.join("SItemData.xml"))?;
        let doc = parse_xml(&xml)?;
        for s in records(&doc) {
            if let Some(t) = int_list(s, "tokusei").filter(|t| !t.is_empty()) {
                special.insert(int(m(Some(s), "ID").as_deref()), t);
            }
        }
    }
    struct SetRow {
        pieces: Vec<i32>,
        tokusei: Vec<i32>,
    }
    let mut sets: Vec<SetRow> = Vec::new();
    {
        let xml = json::read_all_text(&dec.join("EquipmentSetData.xml"))?;
        let doc = parse_xml(&xml)?;
        for s in records(&doc) {
            let pieces = int_list(s, "equipment").unwrap_or_default();
            let tok = int_list(s, "tokusei").unwrap_or_default();
            if !pieces.is_empty() && !tok.is_empty() {
                sets.push(SetRow { pieces, tokusei: tok });
            }
        }
    }
    let mut sets_of: HashMap<i32, Vec<usize>> = HashMap::new();
    for (i, s) in sets.iter().enumerate() {
        for &p in &s.pieces {
            sets_of.entry(p).or_default().push(i);
        }
    }

    // A random box (the server's drop sets with a GiftBoxID) is opened by using an item: the item's use-skill has
    // function 320 and the box number as its first special parameter (SkillData). Data tickets work this way too.
    let mut box_of_skill: HashMap<i32, i32> = HashMap::new();
    // An item can also summon enemies (comp_hack SkillManager): function 322 spawns a zone's spawn location group
    // (special params: zone, location group; Kymopoleia's Hair Accessory -> Hecatonchires in Shinjuku), 324 a group of
    // the global partial next to the player (param: group or location group; the Demon Eggs). skill -> (zone, group).
    let mut summon_of_skill: HashMap<i32, (i32, i32)> = HashMap::new();
    let skill_file = dec.join("SkillData.xml");
    if skill_file.is_file() {
        let xml = json::read_all_text(&skill_file)?;
        let doc = parse_xml(&xml)?;
        let first_member = first_member_desc;
        for sk in records(&doc) {
            let func = first_member(sk, "functionID").map(|n| xml_value(n).trim().to_string()).unwrap_or_default();
            if func == "322" || func == "324" {
                let ps: Vec<i32> = first_member(sk, "specialParams").map(|sp| xml_children(sp, "element").map(|e| int(Some(&xml_value(e)))).collect()).unwrap_or_default();
                let at = if func == "322" { (ps.first().copied().unwrap_or(0), ps.get(1).copied().unwrap_or(0)) } else { (0, ps.first().copied().unwrap_or(0)) };
                if at.1 != 0 {
                    summon_of_skill.insert(int(m(obj(Some(sk), "common"), "id").as_deref()), at);
                }
                continue;
            }
            if func != "320" {
                continue;
            }
            let bx = first_member(sk, "specialParams")
                .map(|sp| xml_children(sp, "element").map(|e| int(Some(&xml_value(e)))).find(|&v| v != 0).unwrap_or(0))
                .unwrap_or(0);
            if bx != 0 {
                box_of_skill.insert(int(m(obj(Some(sk), "common"), "id").as_deref()), bx);
            }
        }
    }
    sources.names = name.clone();
    for (&id, n) in &name {
        sources.item_by_name.entry(n.clone()).or_insert(id);
    }
    // special fusions from the Fusion Planner's data (made by fusionexport from the client's tables)
    let fusion_file = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default().join("fusion-data.json");
    if fusion_file.is_file() {
        let root = json::load(&fusion_file)?;
        let demon_name: HashMap<i64, String> = json::arr(root.at("demons"))
            .filter_map(|d| Some((d.get(0)?.as_i64()?, d.get(1)?.as_str()?.to_string())))
            .collect();
        for sp in json::arr(root.at("specials")) {
            let result = json::int(sp.at("result"));
            // each source: [id, variant] or "id variant" (variant 1: a variant of that demon does too)
            let demons: Vec<String> = json::arr(sp.at("src"))
                .map(|s| {
                    let (id, variant) = match s {
                        Value::Array(a) => (a.first().and_then(Value::as_i64).unwrap_or(0), a.get(1).and_then(Value::as_i64).unwrap_or(0)),
                        Value::String(t) => {
                            let mut p = t.split_whitespace();
                            (p.next().and_then(|x| x.parse().ok()).unwrap_or(0), p.next().and_then(|x| x.parse().ok()).unwrap_or(0))
                        }
                        _ => (0, 0),
                    };
                    format!("{}{}", demon_name.get(&id).cloned().unwrap_or_else(|| "a demon the data does not name".to_string()), if variant != 0 { " (or a variant)" } else { "" })
                })
                .collect();
            let pid = json::int(sp.at("plugin"));
            let plugin = (pid > 0).then(|| {
                let p = root.at("plugins").and_then(|ps| ps.get(pid.to_string()));
                let name = p.and_then(|p| json::str(p.at("name"))).unwrap_or_else(|| "a fusion plugin".to_string());
                let how = p.and_then(|p| json::str(p.at("how"))).map(|h| h.split(" · ").map(str::to_string).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
                (name, how)
            });
            sources.special_fusions.entry(result).or_default().push(SpecialFusion { demons, plugin });
        }
    }

    let mut items: Vec<Obj> = Vec::new();
    let mut by_id: IndexMap<i32, usize> = IndexMap::new();
    let mut gear_ids: HashSet<i32> = HashSet::new(); // the items with a slot: the sheets and the ticket rule are about these
    {
        let xml = json::read_all_text(&dec.join("ItemData.xml"))?;
        let doc = parse_xml(&xml)?;
        for it in records(&doc) {
            let common = obj(Some(it), "common");
            let basic = obj(Some(it), "basic");
            let restr = obj(Some(it), "restriction");
            let poss = obj(Some(it), "possession");
            let id = int(m(common, "id").as_deref());
            let equip = m(basic, "equipType").unwrap_or_default();
            let category = obj(common, "category");
            let sub = int(m(category, "subCategory").as_deref());
            let main_cat = int(m(category, "mainCategory").as_deref());
            let use_skill = int(it
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "member" && n.attribute("name") == Some("useSkill"))
                .map(xml_value)
                .as_deref());
            if use_skill != 0 {
                if let Some(&opens) = box_of_skill.get(&use_skill) {
                    if name.contains_key(&id) {
                        sources.openers.entry(opens).or_default().push(id);
                    }
                }
                if let Some(&at) = summon_of_skill.get(&use_skill) {
                    if name.contains_key(&id) {
                        sources.summoners.entry(at).or_default().push(id);
                    }
                }
            }
            let Some(nm) = name.get(&id).cloned() else { continue };
            let slot = slot_name(&equip);
            let mut o = Obj::new();
            o.insert("id".into(), json!(id));
            o.insert("name".into(), json!(nm));
            // the game's own class of the item ("One-Handed Sword", "Consumable", "Weapon Mod Tool"); the page's kind groups them
            if let Some(class) = crate::data::game_data::item_class_name(sub) {
                o.insert("class".into(), json!(class));
            }
            match slot {
                Some(s) => {
                    o.insert("kind".into(), json!("Gear"));
                    o.insert("slot".into(), json!(s));
                    gear_ids.insert(id);
                }
                None => {
                    o.insert("kind".into(), json!(kind_name(sub, main_cat, &equip)));
                    if let Some(d) = desc.get(&id) {
                        o.insert("desc".into(), Value::Array(d.clone()));
                    }
                    let stack = int(m(poss, "stackSize").as_deref());
                    if stack > 1 {
                        o.insert("stack".into(), json!(stack));
                    }
                    let sell = int(m(basic, "sellPrice").as_deref());
                    if sell > 0 {
                        o.insert("sell".into(), json!(sell));
                    }
                }
            }
            let lvl = int(m(restr, "level").as_deref());
            if lvl > 0 {
                o.insert("level".into(), json!(lvl));
            }
            let g = int(m(restr, "gender").as_deref());
            o.insert("gender".into(), json!(if g == 0 { "Male" } else if g == 1 { "Female" } else { "Any" }));
            let al = m(restr, "alignment").unwrap_or_else(|| "ALL".into());
            if al != "ALL" {
                o.insert("alignment".into(), json!(to_title_case(&al.to_lowercase().replace('_', " "))));
            }
            // a tarot or soul crystal: what each side gives when fused into a piece (the Gear page's words for it)
            if let Some(sides) = enchant_of_item.get(&id) {
                for &(eid, is_tarot) in sides {
                    let mut lines: Vec<Value> = Vec::new();
                    for e in fx.enchant(eid, is_tarot, &nm)? {
                        let line = text(&e, &data)?;
                        if !no_words(&line) {
                            lines.push(json!(line));
                        }
                    }
                    let lines = distinct(lines);
                    if !lines.is_empty() {
                        o.insert(if is_tarot { "tarot" } else { "soul" }.into(), Value::Array(lines));
                    }
                }
            }
            if slot == Some("Weapon") {
                let wt = m(basic, "weaponType");
                o.insert(
                    "weapon".into(),
                    json!(if wt.as_deref() == Some("LONG_RANGE") {
                        "Long range"
                    } else if wt.as_deref() == Some("CLOSE_RANGE") {
                        "Close range"
                    } else {
                        ""
                    }),
                );
                if let Some(aff) = affinity_name(int(m(common, "affinity").as_deref())) {
                    o.insert("affinity".into(), json!(aff));
                }
            }
            // ItemData keeps modSlots in the use restrictions (MiUseRestrictionsData), not on the record or its basic data
            let mods = int(m(restr, "modSlots").or_else(|| m(Some(it), "modSlots")).or_else(|| m(basic, "modSlots")).as_deref());
            if mods > 0 {
                o.insert("mods".into(), json!(mods));
            }

            let mut stats: Vec<Value> = Vec::new();
            let mut effects: Vec<Value> = Vec::new();
            if let Some(tbl) = common.and_then(|c| member(c, "correctTbl")) {
                for c in tbl.descendants().filter(|n| n.is_element() && n.tag_name().name() == "object") {
                    let cid = member_text(c, "ID").unwrap_or_default();
                    let val = member_text(c, "Value").and_then(|s| fmt::parse_f64(&s)).unwrap_or(0.0);
                    if cid.is_empty() || cid == "NONE" || val == 0.0 {
                        continue;
                    }
                    let e = fx.correct(&cid, int(member_text(c, "Type").as_deref()), val, "always", "item", &nm);
                    let line = text(&e, &data)?;
                    // no words for it: its raw table id ("TALK_RATE 2") is not shown
                    if !RE_NO_WORDS_SP.is_match(&line) {
                        stats.push(json!(line)); // every line of the item's own table: the tooltip's Basic Features
                    }
                }
            }
            if let Some(toks) = special.get(&id) {
                for &t in toks {
                    for e in fx.tokusei(t, "characteristic", &nm)? {
                        let line = text(&e, &data)?;
                        if !RE_NO_WORDS_SP.is_match(&line) {
                            effects.push(json!(line));
                        }
                    }
                }
            }
            o.insert("stats".into(), Value::Array(distinct(stats)));
            o.insert("effects".into(), Value::Array(distinct(effects)));

            // sets this piece belongs to; a one-piece set is the item's appearance bonus
            let mut set_list: Vec<Obj> = Vec::new();
            let mut appearance: Vec<Value> = Vec::new();
            let mut seen_sets: HashSet<String> = HashSet::new();
            for &si in sets_of.get(&id).map(Vec::as_slice).unwrap_or(&[]) {
                let s = &sets[si];
                let mut bonus: Vec<Value> = Vec::new();
                for &t in &s.tokusei {
                    for e in fx.tokusei(t, if s.pieces.len() == 1 { "appearance" } else { "set" }, &nm)? {
                        let line = text(&e, &data)?;
                        // a line the reader has no words for comes out as its raw table id ("TALK_RATE 0 (skill: Demon Race 11)"): not shown
                        if !no_words(&line) {
                            bonus.push(json!(line));
                        }
                    }
                }
                let bonus = distinct(bonus);
                if bonus.is_empty() {
                    continue;
                }
                if s.pieces.len() == 1 {
                    appearance.extend(bonus);
                    continue;
                }
                let mut with: Vec<String> = Vec::new();
                for p in s.pieces.iter().filter(|&&p| p != id) {
                    if let Some(pn) = name.get(p) {
                        if !with.contains(pn) {
                            with.push(pn.clone());
                        }
                    }
                }
                let bonus_text = Value::Array(bonus.clone()).to_string();
                if with.is_empty() || !seen_sets.insert(format!("{}#{bonus_text}", with.join("|"))) {
                    continue; // gendered copies of one set
                }
                // the same bonus with one other piece, offered by many pieces (a COMP with any of twenty weapons): one entry listing them
                let same = if with.len() == 1 {
                    set_list.iter_mut().find(|x| x.get("any").is_some() && x.get("effects").map(Value::to_string) == Some(bonus_text.clone()))
                } else {
                    None
                };
                if let Some(same) = same {
                    if let Some(Value::Array(w)) = same.get_mut("with") {
                        w.push(json!(with[0]));
                    }
                    continue;
                }
                let mut entry = Obj::new();
                entry.insert("with".into(), Value::Array(with.iter().map(|x| json!(x)).collect()));
                entry.insert("effects".into(), Value::Array(bonus));
                if with.len() == 1 {
                    entry.insert("any".into(), json!(true));
                }
                set_list.push(entry);
            }
            if !set_list.is_empty() {
                o.insert("sets".into(), Value::Array(set_list.into_iter().map(Value::Object).collect()));
            }
            if !appearance.is_empty() {
                o.insert("appearance".into(), Value::Array(distinct(appearance)));
            }
            o.insert("sources".into(), Value::Array(Vec::new()));
            items.push(o);
            by_id.insert(id, items.len() - 1); // byId[id] = o: a repeated id points at the later record
        }
    }
    // What a voucher, ticket or random box can give: every item whose sources include a box this item opens.
    let mut contents: IndexMap<i32, Vec<i32>> = IndexMap::new();
    for o in items.iter_mut() {
        let id = json::int(o.get("id"));
        if let Some(rs) = sources.raw.get(&id) {
            for x in json::arr(Some(rs)).filter_map(Value::as_object) {
                if str_of(x, "kind") == "box" {
                    for &op in sources.openers.get(&(dbl(x, "giftBox") as i32)).map(Vec::as_slice).unwrap_or(&[]) {
                        let list = contents.entry(op).or_default();
                        if !list.contains(&id) {
                            list.push(id);
                        }
                    }
                }
            }
            let gl = get_lines(&sources, rs, 0, &mut HashSet::new());
            if !gl.is_empty() {
                o.insert("get".into(), Value::Array(gl.into_iter().map(Value::Object).collect()));
            }
        }
    }
    // Tickets: items that are not gear and are called a ticket ("... Data Ticket", "... DT"); gear comes out of them.
    let mut ticket_by_key: IndexMap<String, i32> = IndexMap::new();
    for (&kid, kname) in &name {
        if !gear_ids.contains(&kid) && RE_TICKET.is_match(kname) {
            ticket_by_key.entry(key(kname)).or_insert(kid); // TryAdd
        }
    }
    if let Some(ids_out) = ids_out {
        // every item the client names: tools\dcmexport traces them all (gear, tickets, consumables, materials...)
        let mut all: Vec<i32> = name.keys().copied().collect();
        all.sort();
        let mut text_out = String::new();
        for x in &all {
            text_out.push_str(&format!("{x}\r\n"));
        }
        std::fs::write(ids_out, text_out)?;
        println!("{} item ids ({} of them gear) written to {}.", all.len(), gear_ids.len(), ids_out);
        return Ok(());
    }

    // ---- icons: the picture the game shows for the item ----
    let (mut with_icon, mut no_picture) = (0, 0);
    if let Some(client) = client_dir {
        let icon_dir = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default().join("gear-db-icons");
        std::fs::create_dir_all(&icon_dir)?;
        let mut done: HashMap<String, bool> = HashMap::new();
        for o in items.iter_mut() {
            let id = json::int(o.get("id"));
            let Some(file) = icon_key.get(&id).and_then(|&k| item_icons::item_icon_file(client, k)) else {
                no_picture += 1;
                continue;
            };
            let have = match done.get(&file) {
                Some(&b) => b,
                None => {
                    let dest = icon_dir.join(format!("{file}.png"));
                    let b = dest.is_file()
                        || match item_icons::client_png(client, &file, &[]) {
                            Some(png) => {
                                std::fs::write(&dest, png)?;
                                true
                            }
                            None => false,
                        };
                    done.insert(file.clone(), b);
                    b
                }
            };
            if have {
                o.insert("icon".into(), json!(format!("gear-db-icons/{file}.png")));
                with_icon += 1;
            } else {
                no_picture += 1;
            }
        }
    }

    // ---- key items (plugins, passes): their own ids, with the Fusion Planner's ways to get a plugin ----
    let mut key_items: Vec<Obj> = Vec::new();
    let key_file = dec.join("CKeyItemData.xml");
    if key_file.is_file() {
        let mut plugin_how: HashMap<i32, Vec<String>> = HashMap::new();
        let fusion = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default().join("fusion-data.json");
        if fusion.is_file() {
            let root = json::load(&fusion)?;
            if let Some(plugins) = root.at("plugins").and_then(Value::as_object) {
                for (k, p) in plugins {
                    if let (Some(id), Some(how)) = (fmt::parse_i32(k), json::str(p.at("how"))) {
                        plugin_how.insert(id, how.split(" · ").map(str::to_string).filter(|s| !s.is_empty()).collect());
                    }
                }
            }
        }
        let xml = json::read_all_text(&key_file)?;
        let doc = parse_xml(&xml)?;
        for k in records(&doc) {
            let b = obj(Some(k), "itemData");
            let n = squash(&m(b, "name").unwrap_or_default());
            if n.is_empty() {
                continue;
            }
            let kid = int(m(b, "ID").as_deref());
            let mut o = Obj::new();
            o.insert("id".into(), json!(kid));
            o.insert("keyItem".into(), json!(true));
            o.insert("name".into(), json!(n));
            o.insert("kind".into(), json!("Key item"));
            o.insert("class".into(), json!("Key item"));
            let d = desc_lines(&m(b, "desc").unwrap_or_default());
            if !d.is_empty() {
                o.insert("desc".into(), Value::Array(d));
            }
            if let Some(how) = plugin_how.get(&kid) {
                o.insert("how".into(), Value::Array(how.iter().map(|s| json!(s)).collect()));
            }
            o.insert("stats".into(), Value::Array(Vec::new()));
            o.insert("effects".into(), Value::Array(Vec::new()));
            o.insert("sources".into(), Value::Array(Vec::new()));
            key_items.push(o);
        }
    }

    // ---- the spreadsheets: where it comes from (gear only: the sheet is a gear reference) ----
    let gear_idx: HashSet<usize> = items.iter().enumerate().filter(|(_, o)| gear_ids.contains(&json::int(o.get("id")))).map(|(i, _)| i).collect();
    let is_gear = |i: usize| gear_idx.contains(&i);
    let mut by_name: IndexMap<String, Vec<usize>> = IndexMap::new();
    let mut by_loose: IndexMap<String, Vec<usize>> = IndexMap::new();
    for (i, o) in items.iter().enumerate() {
        if !is_gear(i) {
            continue;
        }
        let n = o.get("name").and_then(Value::as_str).unwrap_or("");
        by_name.entry(key(n)).or_default().push(i);
        by_loose.entry(loose_key(n)).or_default().push(i);
    }
    let (mut rows, mut by_id_hits, mut by_name_hits, mut unmatched, mut vanity, mut other_server) = (0, 0, 0, 0, 0, 0);
    let mut sheet_only: Vec<Obj> = Vec::new();
    if let Some(sheet) = sheet_tsv.filter(|p| Path::new(p).is_file()) {
        let text_all = json::read_all_text(Path::new(sheet))?;
        let lines = read_all_lines(&text_all);
        let mut head: HashMap<String, usize> = HashMap::new();
        for (i, h) in lines.first().map(|l| l.split('\t').collect::<Vec<_>>()).unwrap_or_default().into_iter().enumerate() {
            if head.insert(h.to_string(), i).is_some() {
                return Err(anyhow::anyhow!("An item with the same key has already been added. Key: {h}"));
            }
        }
        let f = |p: &[&str], col: &str| -> String { head.get(col).filter(|&&i| i < p.len()).map(|&i| p[i].trim().to_string()).unwrap_or_default() };
        let flat = |s: &str| squash(&s.replace(" ;; ", " · ").replace(" || ", " · "));
        for line in lines.iter().skip(1) {
            let p: Vec<&str> = line.split('\t').collect();
            if f(&p, "item_name").is_empty() {
                continue;
            }
            // Only SnowSheet's gear reference is IMAGINE's. Midori's Post Wipe Re:FC is another server's sheet (the user,
            // 2026-11-23): its rows are in sheet_gear.tsv too and must not be used.
            if f(&p, "source_sheet") != "gear_reference" {
                other_server += 1;
                continue;
            }
            if f(&p, "tab").starts_with("Vanities") {
                vanity += 1; // cosmetic data tickets: not gear, no stats
                continue;
            }
            rows += 1;
            let how_raw = f(&p, "how_obtained");
            let mut src = Obj::new();
            src.insert("sheet".into(), json!("SnowSheet gear reference"));
            src.insert("tab".into(), json!(f(&p, "tab")));
            src.insert("how".into(), json!(flat(&how_raw)));
            // the ticket the sheet names (by "Item ID: n", or by its name), and where the game data says that ticket comes from
            let mut ticket = 0;
            if let Some(idm) = RE_ITEM_ID.captures(&how_raw) {
                let tid = int_parse(&idm[1])?;
                if name.contains_key(&tid) && !gear_ids.contains(&tid) {
                    ticket = tid;
                }
            }
            if ticket == 0 {
                for part in RE_HOW_SPLIT.split(&how_raw) {
                    if let Some(&tk) = ticket_by_key.get(&key(part)) {
                        ticket = tk;
                        break;
                    }
                }
            }
            if ticket != 0 {
                src.insert("ticket".into(), json!(name[&ticket]));
                if let Some(ts) = sources.raw.get(&ticket) {
                    src.insert("ticketGet".into(), Value::Array(get_lines(&sources, ts, 0, &mut HashSet::new()).into_iter().map(Value::Object).collect()));
                }
            }
            let mut hit: Vec<usize> = Vec::new();
            for mm in RE_DIGITS.find_iter(&f(&p, "item_id")) {
                if let Some(&i) = by_id.get(&int_parse(mm.as_str())?).filter(|&&i| is_gear(i)) {
                    hit.push(i);
                }
            }
            if !hit.is_empty() {
                by_id_hits += 1;
            } else {
                // "Lover's Suit (Blue) / Lover's Dress (Blue)": the male and the female piece; the notes column lists more names
                let item_name = f(&p, "item_name");
                let notes = RE_JP_NAME.replace_all(&f(&p, "notes"), "").into_owned();
                let names: IndexSet<String> = item_name.split(" / ").map(key).chain(RE_NOTE_SPLIT.split(&notes).map(key)).filter(|k| k.len() > 2).collect();
                for k in &names {
                    if let Some(l) = by_name.get(k) {
                        hit.extend(l);
                    }
                }
                if hit.is_empty() {
                    for raw in item_name.split(" / ") {
                        let lk = loose_key(raw);
                        if lk.len() > 3 {
                            if let Some(l) = by_loose.get(&lk) {
                                hit.extend(l);
                            }
                        }
                    }
                }
                if !hit.is_empty() {
                    by_name_hits += 1;
                }
            }
            if hit.is_empty() {
                unmatched += 1;
                let mut slots: Vec<Value> = Vec::new();
                for col in ["slot1_text", "slot2_text", "slot3_text"] {
                    if !f(&p, col).is_empty() {
                        slots.push(json!(flat(&f(&p, col))));
                    }
                }
                let mut so = Obj::new();
                so.insert("id".into(), json!(0));
                so.insert("name".into(), json!(f(&p, "item_name")));
                so.insert("kind".into(), json!("Gear"));
                so.insert("slot".into(), json!(f(&p, "equip_slot_type")));
                so.insert("gender".into(), json!(f(&p, "mf_restriction")));
                so.insert("sheetOnly".into(), json!(true));
                so.insert("stats".into(), Value::Array(Vec::new()));
                so.insert("effects".into(), Value::Array(slots));
                so.insert("sources".into(), Value::Array(vec![Value::Object(src)]));
                sheet_only.push(so);
                continue;
            }
            let src_text = Value::Object(src.clone()).to_string();
            let mut done: HashSet<usize> = HashSet::new();
            for i in hit {
                if !done.insert(i) {
                    continue; // Distinct
                }
                // the sheet says this piece comes in that ticket: the ticket lists the piece among what it gives
                if ticket != 0 {
                    let pid = json::int(items[i].get("id"));
                    let list = contents.entry(ticket).or_default();
                    if !list.contains(&pid) {
                        list.push(pid);
                    }
                }
                if let Some(Value::Array(list)) = items[i].get_mut("sources") {
                    if !list.iter().any(|x| x.to_string() == src_text) {
                        list.push(Value::Object(src.clone()));
                    }
                }
            }
        }
    }

    // Quests that hand the item over: the Quest Database's data (app\ui\quest-guide.json beside the output) words every
    // reward as "N × <name>" or the bare name in its "get" lists (steps, ways, walkthrough); the names are matched to items.
    // The page links each quest to the Quest Database by its title.
    const QUESTS_CAP: usize = 40;
    let quest_file = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default().join("quest-guide.json");
    if quest_file.is_file() {
        // a name with no Latin letters or digits (Japanese names) has an empty key: never matched, or every such item would match
        let mut by_key_all: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, o) in items.iter().enumerate() {
            let k = key(o.get("name").and_then(Value::as_str).unwrap_or(""));
            if !k.is_empty() {
                by_key_all.entry(k).or_default().push(i);
            }
        }
        fn gets<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
            match v {
                Value::Array(a) => a.iter().for_each(|x| gets(x, out)),
                Value::Object(o) => {
                    for (k, x) in o {
                        if k == "get" {
                            out.extend(json::arr(Some(x)).filter_map(Value::as_str));
                        } else {
                            gets(x, out);
                        }
                    }
                }
                _ => {}
            }
        }
        let mut quests_of: HashMap<usize, Vec<String>> = HashMap::new();
        let root = json::load(&quest_file)?;
        for q in json::arr(root.at("quests")) {
            let Some(title) = json::str(q.at("title")).filter(|t| !t.is_empty()) else { continue };
            let mut rewards: Vec<&str> = Vec::new();
            gets(q, &mut rewards);
            for r in rewards {
                // "10 × Ointment" -> Ointment; XP, expertise and valuables are not items
                let nm = RE_COUNT_X.replace(r, "");
                if nm.is_empty() || nm.starts_with("the valuable ") || nm.ends_with(" XP") || nm.contains(" expertise ") || nm.contains("XP (") {
                    continue;
                }
                for &i in by_key_all.get(&key(&nm)).map(Vec::as_slice).unwrap_or(&[]) {
                    let list = quests_of.entry(i).or_default();
                    if !list.contains(&title) {
                        list.push(title.clone());
                    }
                }
            }
        }
        for (i, mut titles) in quests_of {
            if titles.len() > QUESTS_CAP {
                items[i].insert("questsMore".into(), json!(titles.len() - QUESTS_CAP));
                titles.truncate(QUESTS_CAP);
            }
            items[i].insert("quests".into(), Value::Array(titles.into_iter().map(|t| json!(t)).collect()));
        }
    }

    // What a voucher, ticket or box can give: the box tables above, and the pieces the sheet files under the ticket
    const GIVES_CAP: usize = 120;
    for o in items.iter_mut() {
        if let Some(list) = contents.get(&json::int(o.get("id"))) {
            let mut names: Vec<String> = list.iter().filter_map(|i| name.get(i).cloned()).collect::<IndexSet<_>>().into_iter().collect();
            names.sort_by(|a, b| crate::text::cmp_invariant(a, b));
            if names.len() > GIVES_CAP {
                o.insert("givesMore".into(), json!(names.len() - GIVES_CAP));
                names.truncate(GIVES_CAP);
            }
            o.insert("gives".into(), Value::Array(names.into_iter().map(|n| json!(n)).collect()));
        }
    }

    let in_game = items.iter().filter(|o| o.get("get").is_some()).count();
    let src_len = |o: &Obj| o.get("sources").and_then(Value::as_array).map_or(0, Vec::len);
    let via_ticket = items
        .iter()
        .filter(|o| o.get("get").is_none() && o.get("sources").and_then(Value::as_array).is_some_and(|a| a.iter().any(|s| s.get("ticketGet").is_some_and(|v| !v.is_null()))))
        .count();
    let with_src = items.iter().filter(|o| src_len(o) > 0).count();
    let item_count = items.len();
    let gear_count = gear_ids.len();
    let with_gives = items.iter().filter(|o| o.get("gives").is_some()).count();
    let with_quests = items.iter().filter(|o| o.get("quests").is_some()).count();
    let mut per_kind: IndexMap<String, usize> = IndexMap::new();
    for o in &items {
        *per_kind.entry(o.get("kind").and_then(Value::as_str).unwrap_or("").to_string()).or_default() += 1;
    }

    let mut all: Vec<Value> = items.into_iter().map(Value::Object).collect();
    all.extend(sheet_only.into_iter().map(Value::Object));
    let key_count = key_items.len();
    all.extend(key_items.into_iter().map(Value::Object));
    let mut result = Obj::new();
    result.insert("generatedAt".into(), json!(chrono::Utc::now().format("%Y-%m-%d").to_string()));
    result.insert("note".into(), json!("Names, descriptions, stats, effects and sets: the IMAGINE client's current tables. Community notes: SnowSheet's gear reference for IMAGINE."));
    result.insert("items".into(), Value::Array(all));
    let out_path = Path::new(out_file);
    let full = std::path::absolute(out_path)?;
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_path, json::serialize(&Value::Object(result), Escape::Relaxed, false))?;
    if client_dir.is_some() {
        println!("Icons: {with_icon} pieces have one; {no_picture} have no picture in the client.");
    } else {
        println!("No icons: the game folder is not known (pass --client <game folder>).");
    }
    println!("In-game source found for {in_game} items directly and {via_ticket} more through their ticket; {with_gives} vouchers and boxes list what they can give; {with_quests} items are quest rewards.");
    println!(
        "{item_count} items from the client ({gear_count} gear; {}) and {key_count} key items, {with_src} pieces of gear with a source from the sheets. {} KB.",
        per_kind.iter().filter(|(k, _)| k.as_str() != "Gear").map(|(k, n)| format!("{n} {}", k.to_lowercase())).collect::<Vec<_>>().join(", "),
        std::fs::metadata(out_path)?.len() / 1024
    );
    println!(
        "Sheet rows: {rows}; matched by id {by_id_hits}, by name {by_name_hits}, not matched {unmatched} (kept as sheet-only entries). {vanity} cosmetic ticket rows (Vanities tabs) left out. {other_server} rows of the other server's sheet (Re:FC) ignored."
    );
    Ok(())
}

/// `File.ReadAllLines`: lines ended by \r\n, \n or \r (no empty last line after a final break).
fn read_all_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        any = true;
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut cur));
                any = false;
            }
            '\n' => {
                lines.push(std::mem::take(&mut cur));
                any = false;
            }
            _ => cur.push(c),
        }
    }
    if any {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(key("Lover's Suit (Blue)"), "loverssuitblue");
        // the loose key drops the "s" too (the character class a-r, t-z)
        assert_eq!(loose_key("The Lover's Suit"), "loveruit");
    }

    #[test]
    fn lines_like_read_all_lines() {
        assert_eq!(read_all_lines("a\r\nb\nc\rd"), vec!["a", "b", "c", "d"]);
        assert_eq!(read_all_lines("a\n"), vec!["a"]);
        assert_eq!(read_all_lines("a\n\n"), vec!["a", ""]);
    }

    #[test]
    fn effect_text() {
        let data = GameData::with_paths(None, None);
        let e = Effect { label: "Close-Range".into(), value: Some(24.0), ..Default::default() };
        assert_eq!(text(&e, &data).unwrap(), "Close-Range +24");
        let e = Effect { label: "Max HP".into(), value: Some(-2.5), pct: true, cond: "situational".into(), target: "partner".into(), ..Default::default() };
        assert_eq!(text(&e, &data).unwrap(), "Partner: Max HP −2.5% (depends on the fight)");
        assert!(no_words("TALK_RATE 2"));
        assert!(!no_words("Talk Rate +2"));
    }

    #[test]
    fn box_lines_nest_and_dead_ends_drop() {
        let mut sources = Sources::default();
        sources.openers.insert(7, vec![100]);
        sources.names.insert(100, "Mystery Box".into());
        let raw = json!([{ "kind": "box", "giftBox": 7, "rate": 5 }, { "kind": "shop", "shopId": 1, "price": 1500, "shop": "General Store", "currency": "Macca", "zone": "Ginza" }]);
        let lines = get_lines(&sources, &raw, 0, &mut HashSet::new());
        // the shop comes first; the box's opener has no known source, so its dead-end "Open" line is dropped
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["k"], "Buy");
        assert_eq!(lines[0]["v"], "1,500 Macca");
        sources.raw.insert(100, json!([{ "kind": "drop", "enemy": "Pixie", "rate": 2.5, "zone": "Ginza" }]));
        let lines = get_lines(&sources, &raw, 0, &mut HashSet::new());
        assert_eq!(lines.iter().map(|l| l["k"].as_str().unwrap()).collect::<Vec<_>>(), vec!["Buy", "Open", "Enemy drop"]);
        assert_eq!(lines[2]["d"], 1);
    }
}
