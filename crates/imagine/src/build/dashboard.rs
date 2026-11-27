//! Port of `src/Build/Dashboard.cs`.
//!
//! Data files for the "Rubito Build Tracker" page: snapshots.json (one row per CSV snapshot), gear.json (what each piece
//! and set adds) and bonuses.json (every bonus from gear and expertise, merged, with its sources).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::NaiveDateTime;
use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::Result;
use crate::calc::bonuses;
use crate::calc::character_extras;
use crate::calc::character_hp_mp;
use crate::calc::digitalize::{self, DigitalizeResult, MITAMA_NAMES};
use crate::calc::effect::{Effect, rx};
use crate::calc::gear_effects::GearResult;
use crate::calc::item_effects::{ItemEffects, title};
use crate::calc::tooltip_parser::TooltipParser;
use crate::context::Context;
use crate::data::demon_tables::Assist;
use crate::data::game_data::GameData;
use crate::data::json::{self, At, Row};
use crate::data::stat_labels::GameStatLabels;
use crate::data::store::{SlotTip, Store};
use crate::fmt;
use crate::game::DemonRecord;
use crate::text::{cmp_invariant, eq_ic, ic_key};

use super::csv::CsvTable;
use super::demon_model::base64;
use super::digitalize_demon::DigitalizeSave;
use super::item_icons;
use super::snapshot::{DEMON_COLUMN, demon_col};

/// The digitalize window's picture for each kind of Digitalize boost (Interface\tga2\<name>.tga), by assist type.
fn assist_icon_name(typ: &str) -> Option<&'static str> {
    Some(match typ {
        "STAT_RATE" => "digitalize_assisticon",
        "EXTEND_TIME" => "digitalize_timeupicon",
        "REDUCE_WAIT" => "digitalize_latencydownicon",
        "AFFINITIES" => "digitalize_defenseicon",
        "FORCE_STACK" => "digitalize_forceicon",
        "PASSIVES" => "digitalize_passiveicon",
        "TRAITS" => "digitalize_featureicon",
        "ACTIVES" => "digitalize_activeicon",
        "MITAMA_SET" => "digitalize_mitamaicon",
        _ => return None,
    })
}

const STAT_KEYS: [(&str, &str); 12] = [
    ("STR", "str"),
    ("MAG", "mag"),
    ("VIT", "vit"),
    ("INT", "int"),
    ("SPD", "spd"),
    ("LUCK", "luck"),
    ("CLS-R", "clsr"),
    ("LNG-R", "lngr"),
    ("Spell", "spell"),
    ("Support", "support"),
    ("P.Def", "pdef"),
    ("M.Def", "mdef"),
];

pub const COMBAT_KEYS: [(&str, &str); 24] = [
    ("Critical", "crit"),
    ("Final critical correction", "final_crit"),
    ("Limit break chance", "lb_chance"),
    ("Limit break power", "lb_power"),
    ("Pursuit chance", "pursuit_chance"),
    ("Pursuit power", "pursuit_power"),
    ("Technical attack chance", "ta_chance"),
    ("Technical attack power", "ta_power"),
    ("Close-range damage dealt %", "dmg_close"),
    ("Long-range damage dealt %", "dmg_long"),
    ("Spell damage dealt %", "dmg_spell"),
    ("Support damage dealt %", "dmg_support"),
    ("Close-range damage taken %", "taken_close"),
    ("Long-range damage taken %", "taken_long"),
    ("Spell damage taken %", "taken_spell"),
    ("Support damage taken %", "taken_support"),
    ("Spin skill boost %", "boost_spin"),
    ("Attack skill boost %", "boost_attack"),
    ("Rush skill boost %", "boost_rush"),
    ("Rapid skill boost %", "boost_rapid"),
    ("Shot skill boost %", "boost_shot"),
    ("Cooldown %", "cooldown"),
    ("Movement speed %", "move_speed"),
    ("Limit break cap", "lb_cap"),
];

/// `Num(v)`: empty = none, else `double.Parse(v, InvariantCulture)`.
fn num(v: &str) -> Result<Value> {
    if v.is_empty() {
        return Ok(Value::Null);
    }
    match fmt::parse_f64(v) {
        Some(d) => Ok(json!(d)),
        None => Err(anyhow::anyhow!("The input string '{v}' was not in a correct format.")),
    }
}

/// `DateTime.Parse(text, InvariantCulture)` for the forms the tool writes (and a few more): date, optional time, optional fraction.
pub fn parse_datetime(text: &str) -> Result<NaiveDateTime> {
    let s = text.trim();
    for f in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"] {
        if let Ok(t) = NaiveDateTime::parse_from_str(s, f) {
            return Ok(t);
        }
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).expect("midnight"));
    }
    Err(anyhow::anyhow!("The string '{s}' was not recognized as a valid DateTime."))
}

pub(crate) fn iso(v: &str) -> Result<String> {
    Ok(parse_datetime(v)?.format("%Y-%m-%dT%H:%M:%S").to_string())
}

fn obj(v: Value) -> Row {
    match v {
        Value::Object(m) => m,
        _ => Row::new(),
    }
}

fn rows_value(rows: &[Row]) -> Value {
    Value::Array(rows.iter().cloned().map(Value::Object).collect())
}

/// `SumFx(fx, rx, cond)`.
fn sum_fx(fx: &[&Effect], pattern: &str, cond: &str) -> f64 {
    let mut s = 0.0f64;
    for e in fx {
        if e.target == "self" && e.value.is_some() && rx::is(&e.label, pattern) && (if cond == "any" { e.counted() } else { e.cond == cond }) {
            s += e.value.unwrap_or(0.0);
        }
    }
    s
}

/// What an item's or set's effects add to the page's key stats, as (key, value) pairs in the page's order.
fn contrib(fx: &[&Effect]) -> Vec<(&'static str, f64)> {
    let p = bonuses::pattern;
    vec![
        ("crit", sum_fx(fx, p("Critical"), "always")),
        ("crit_digi", sum_fx(fx, p("Critical"), "digitalized")),
        ("lb_power_digi", sum_fx(fx, p("Limit break power"), "digitalized")),
        ("ta_chance", sum_fx(fx, p("Technical attack chance"), "any")),
        ("ta_power", sum_fx(fx, p("Technical attack power"), "any")),
        ("dmg_close", sum_fx(fx, p("Close-range damage dealt %"), "any")),
        ("dmg_long", sum_fx(fx, p("Long-range damage dealt %"), "any")),
        ("dmg_spell", sum_fx(fx, p("Spell damage dealt %"), "any")),
        ("dmg_support", sum_fx(fx, p("Support damage dealt %"), "any")),
        ("taken", sum_fx(fx, p("Close-range damage taken %"), "any")),
        ("cooldown", sum_fx(fx, p("Cooldown %"), "any")),
    ]
}

/// The label the page shows for a fusion bonus's stat.
pub fn fuse_stat_label(stat: &str) -> String {
    match stat {
        "CLSR" => "Close-Range",
        "LNGR" => "Long-Range",
        "SPELL" => "Spell",
        "SUPPORT" => "Support",
        "PDEF" => "Physical Defense",
        "MDEF" => "Magical Defense",
        _ => stat,
    }
    .to_string()
}

/// A demon effect's note without the part name ("passive", "trait"...), which its source already says.
fn demon_note(note: &str) -> String {
    note.split("; ").filter(|s| !matches!(*s, "stats" | "passive" | "trait" | "mitama set" | "affinity")).collect::<Vec<_>>().join("; ")
}

/// `Title1`: the first letter upper case.
fn title1(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn stat_short(i: i32) -> String {
    match i {
        0 => "STR".into(),
        1 => "MAG".into(),
        2 => "VIT".into(),
        3 => "INT".into(),
        4 => "SPD".into(),
        5 => "LUCK".into(),
        6 => "Max HP".into(),
        7 => "Max MP".into(),
        12 => "CLS-R".into(),
        13 => "LNG-R".into(),
        14 => "Spell".into(),
        15 => "Support".into(),
        _ => digitalize::correct_id(i as usize),
    }
}

fn assist_text(a: &Assist) -> String {
    match a.r#type.as_str() {
        "STAT_RATE" => format!("Stats +{}%", a.value),
        "EXTEND_TIME" => format!("Duration +{} min", a.value / 60),
        "REDUCE_WAIT" => format!("Cooldown -{} min", a.value / 60), // the game: "Cool-down time -15 minutes"
        "AFFINITIES" => "Resistances".into(),
        "FORCE_STACK" => "Demon Force".into(),
        "PASSIVES" => "Passive skills".into(),
        "TRAITS" => "Features".into(),
        "ACTIVES" => "Active skills".into(),
        "MITAMA_SET" => "Mitama sets".into(),
        t => title(t),
    }
}

/// `Fx(Effect e)`: "<label> +N%" or just the label.
fn fx_text(e: &Effect) -> String {
    match e.value {
        Some(v) => format!("{} {}{}{}", e.label, if v >= 0.0 { "+" } else { "" }, fmt::g(v), if e.pct { "%" } else { "" }),
        None => e.label.clone(),
    }
}

/// What digitalizing with one COMP demon would give, for the Digitalize page's grid of demons.
#[derive(Debug, Clone, PartialEq)]
pub struct DemonOffer {
    pub alignment: Option<String>,
    pub rate: i32,
    pub max_rate: i32,
    pub skills: Vec<Row>,
    pub stats: Vec<Row>,
    pub combat: IndexMap<String, f64>,
}

impl DemonOffer {
    /// The page's JSON for it (camelCase names, the order of the C# record).
    pub fn to_value(&self) -> Value {
        json!({
            "alignment": self.alignment,
            "rate": self.rate,
            "maxRate": self.max_rate,
            "skills": rows_value(&self.skills),
            "stats": rows_value(&self.stats),
            "combat": Value::Object(self.combat.iter().map(|(k, v)| (k.clone(), json!(v))).collect()),
        })
    }
}

/// One bonus line from one source, as the bonus list counts it.
#[derive(Debug, Clone, Default)]
pub struct Part {
    pub kind: String,
    pub label: String,
    pub target: String,
    pub cond: String,
    pub note: String,
    pub from: String,
    pub group: String,
    pub bonus: String,
    pub value: f64,
    pub pct: bool,
    /// The skill whose icon stands for this part (demon skills, expertise skills), or 0; `demon`: the demon's own icon.
    pub skill_id: i32,
    /// A summoned demon's equipment: the item (Epitaph Parts) whose picture stands in when the skill has none. picture: a
    /// client interface picture that stands for the part (Demon Force, Mitama set).
    pub item_id: i32,
    pub picture: String,
    pub demon: bool,
    /// `fuse`: a fusion great-success bonus (the server adds it before a stat's percent step). `demon_stat`: one of
    /// the digitalize demon's own stats (added to the character's base stats). Both matter to the simulator only.
    pub fuse: bool,
    pub demon_stat: bool,
    /// From a toggle (switch) skill that is switched on, or from another skill's effect that needs one on. Toggles that are off never get this far (they have no effects).
    pub toggle: bool,
}

/// `new Dashboard(store, data, ctx)`: the store and the data are the context's.
pub struct Dashboard {
    ctx: Arc<Context>,
}

impl Dashboard {
    pub fn new(ctx: Arc<Context>) -> Dashboard {
        Dashboard { ctx }
    }

    fn store(&self) -> &Store {
        &self.ctx.store
    }

    fn data(&self) -> &GameData {
        &self.ctx.data
    }

    pub fn build_snapshots(&self, csv_path: &str) -> Result<Vec<Row>> {
        let t = CsvTable::load(csv_path)?;
        let mut rows = Vec::new();
        for r in &t.rows {
            let g = |c: &str| t.get(r, c);
            let src = g("Stats source");
            let stats_from = if g("Stats from").is_empty() { g("Timestamp") } else { g("Stats from") };
            let mut o = obj(json!({
                "taken_at": iso(&g("Timestamp"))?, "character": g("Character"), "level": num(&g("Level"))?,
                "hp": num(&g("HP"))?, "max_hp": num(&g("MaxHP"))?, "mp": num(&g("MP"))?, "max_mp": num(&g("MaxMP"))?,
                "stats_live": src.is_empty() || src == "Information window" || src == "Character data",
                "stats_from": iso(&stats_from)?,
                // saved stats kept because the game showed digitalized ones (Snapshot: "... (digitalized: ...)"), not because they could not be read
                "stats_digitalized": src.contains("(digitalized"),
                "gear_missing": g("Gear bonuses missing"),
            }));
            for (csv, k) in STAT_KEYS {
                o.insert(k.into(), num(&g(csv))?);
                o.insert(format!("{k}_base"), num(&g(&format!("{csv} base")))?);
                o.insert(format!("{k}_bonus"), num(&g(&format!("{csv} bonus")))?);
            }
            for (csv, k) in COMBAT_KEYS {
                o.insert(k.into(), num(&g(&format!("{csv} (gear)")))?);
                o.insert(format!("{k}_digi"), num(&g(&format!("{csv} (gear, digitalized)")))?);
                o.insert(format!("{k}_demon"), num(&g(&demon_col(csv)))?);
            }
            o.insert("digi_demon".into(), json!(g(DEMON_COLUMN)));
            rows.push(o);
        }
        json::save(self.store().dash_dir().join("snapshots.json"), &rows_value(&rows))?;
        Ok(rows)
    }

    /// gear.json from the latest snapshot's tooltips (tooltips.txt) and the CSV's last row (slot names).
    pub fn build_gear(&self, csv_path: &str) -> Result<Vec<Row>> {
        let store = self.store();
        let data = self.data();
        let t = CsvTable::load(csv_path)?;
        let last = t.rows.last().ok_or_else(|| anyhow::anyhow!("Index was out of range. Must be non-negative and less than the size of the collection."))?;
        let slots: Vec<&String> = t.columns.iter().filter(|c| c.starts_with("Gear: ")).collect();
        let recs = store.load_tooltips()?;
        let (texts, ids, tarot, soul) = Store::unpack(&recs);
        let details = Store::unpack_details(&recs);
        let all = self.ctx.gear()?.compute(&texts, Some(&ids), tarot.as_deref(), soul.as_deref(), details.as_deref(), store.tooltips_level()?, None)?;
        let mut gear: Vec<Row> = Vec::new();
        let fuse_fx = ItemEffects::new(Arc::clone(&self.ctx.data), "NEUTRAL", 0, IndexMap::new()); // only for the fusion bonus tables
        // Items with a tooltip, or with a record (bonuses from ids; named from the item list until its tooltip is seen).
        for rec in recs.iter().filter(|r| r.id > 0 && (!r.text.is_empty() || r.detail.is_some())) {
            let info = if rec.text.is_empty() { None } else { Some(TooltipParser::info(&rec.text)) };
            let name = match info.as_ref().map(|i| i.base_name.clone()) {
                Some(n) => n,
                None => data.item_name(rec.id)?.unwrap_or_else(|| format!("item {}", rec.id)),
            };
            let fx: Vec<&Effect> = all.effects.iter().filter(|e| eq_ic(&e.item, &name)).collect();
            let slot = if (rec.index as usize) < slots.len() && rec.index >= 0 { slots[rec.index as usize][6..].to_string() } else { format!("#{}", rec.index) };
            let tarot_name = match info.as_ref().and_then(|i| i.tarot.clone()) {
                Some(n) => Some(n),
                None => match &rec.detail {
                    Some(d) if d.tarot > 0 => Some(data.crystal_label(d.tarot, true)?),
                    _ => None,
                },
            };
            let soul_name = match info.as_ref().and_then(|i| i.soul.clone()) {
                Some(n) => Some(n),
                None => match &rec.detail {
                    Some(d) if d.soul > 0 => Some(data.crystal_label(d.soul, false)?),
                    _ => None,
                },
            };
            let mut o = obj(json!({
                // the item list's name (the game's) by id; the CSV's last row only for an item the list lacks (rows saved
                // before 2026-11-24 hold the website's names)
                "row": format!("item-{}", rec.index), "kind": "item", "slot": slot,
                "item": data.item_name(rec.id)?.filter(|n| !n.is_empty()).unwrap_or_else(|| t.get(last, &format!("Gear: {slot}"))),
                "shown_as": info.as_ref().map(|i| i.name.clone()).unwrap_or_else(|| name.clone()), "tarot": tarot_name, "soul": soul_name,
                "item_id": rec.id, "tip": if rec.text.is_empty() { Value::Null } else { json!(rec.text) },
                "icon": item_icons::data_uri(store, data, rec.id, 0)?,
            }));
            // Fusion great-success bonuses, as the game's Features tab shows them: "Long-Range: +127 (82 + 45) MAX".
            // "stat" is the label of that line in the tooltip.
            if let Some(d) = &rec.detail {
                let fuse: Vec<Row> = fuse_fx.fuse_bonus(d)?.into_iter().map(|(stat, boost, max)| obj(json!({ "stat": fuse_stat_label(stat), "bonus": boost, "max": max }))).collect();
                o.insert("fuse".into(), rows_value(&fuse));
            }
            for (k, v) in contrib(&fx) {
                o.insert(k.into(), json!(v));
            }
            gear.push(o);
        }
        // Enchants worn as tarot and as soul fusion: ids from the records, else the crystal names on the tooltips.
        let mut tarot_worn: HashSet<i32> = HashSet::new();
        let mut soul_worn: HashSet<i32> = HashSet::new();
        for rec in recs.iter().filter(|r| r.id > 0) {
            let info = if rec.text.is_empty() { None } else { Some(TooltipParser::info(&rec.text)) };
            let add = |to: &mut HashSet<i32>, id: i32, crystal: Option<&str>| -> Result<()> {
                if id > 0 {
                    to.insert(id);
                } else if let Some(c) = crystal {
                    to.extend(data.enchant_ids_by_crystal(c)?);
                }
                Ok(())
            };
            add(&mut tarot_worn, rec.tarot.unwrap_or_else(|| rec.detail.as_ref().map_or(0, |d| d.tarot)), info.as_ref().and_then(|i| i.tarot.as_deref()))?;
            add(&mut soul_worn, rec.soul.unwrap_or_else(|| rec.detail.as_ref().map_or(0, |d| d.soul)), info.as_ref().and_then(|i| i.soul.as_deref()))?;
        }
        let mut si = 0;
        for s in all.sets.iter().filter(|s| s.active && !s.appearance) {
            let first = rx::rep(s.pieces.split(", ").next().unwrap_or(""), "[♂♀]", "");
            let sid = if s.set_id != 0 { format!(", set {}", s.set_id) } else { String::new() };
            let mut o = if s.fusion {
                obj(json!({
                    "row": format!("set-{si}"), "kind": "set", "slot": "Fusion set",
                    "item": format!("{} ({}, fusion set {})", s.pieces.replace(", ", " + "), s.worn, s.set_id), "shown_as": s.pieces, "tarot": Value::Null,
                }))
            } else {
                obj(json!({
                    "row": format!("set-{si}"), "kind": "set", "slot": "Set bonus", "item": format!("Set with {first} ({}{sid})", s.worn), "shown_as": s.pieces, "tarot": Value::Null,
                }))
            };
            // The icons shown as a cluster: a set's pieces; for a fusion set, the usual tarot icon and / or the usual
            // soul fusion icon, by how its enchants are worn (soul fusions only: just the soul one).
            let (tarot_item, soul_item) = data.fusion_icon_items()?;
            let icon_items: Vec<i32> = if !s.fusion {
                s.piece_ids.clone()
            } else {
                vec![if s.piece_ids.iter().any(|p| tarot_worn.contains(p)) { tarot_item } else { 0 }, if s.piece_ids.iter().any(|p| soul_worn.contains(p)) { soul_item } else { 0 }]
            };
            o.insert("piece_ids".into(), if s.fusion { json!(Vec::<i32>::new()) } else { json!(s.piece_ids) }); // which worn pieces (item_id) make the set
            let mut icons: Vec<String> = Vec::new();
            for id in icon_items {
                if let Some(u) = item_icons::data_uri(store, data, id, 0)? {
                    icons.push(u);
                }
            }
            o.insert("icons".into(), json!(icons));
            let sfx: Vec<&Effect> = s.effects.iter().collect();
            for (k, v) in contrib(&sfx) {
                o.insert(k.into(), json!(v));
            }
            gear.push(o);
            si += 1;
        }
        json::save(store.dash_dir().join("gear.json"), &rows_value(&gear))?;
        Ok(gear)
    }

    /// dash\digitalize.json: which demon the Digitalized view adds, and on what terms.
    /// `gear_digi_flat`: flat Max HP / Max MP from gear that only works while digitalized.
    fn build_digitalize(&self, s: Option<&DigitalizeSave>, r: Option<&DigitalizeResult>, gear_digi_flat: (f64, f64)) -> Result<()> {
        let store = self.store();
        let data = self.data();
        let path = store.dash_dir().join("digitalize.json");
        let (Some(s), Some(r)) = (s, r) else {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        };
        let (Some(demon), Some(devil)) = (s.demon.as_ref(), r.devil.as_ref()) else {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        };
        let t = data.demons()?;
        let learned_assists: Vec<i32> = s.assists.clone().unwrap_or_default();
        let mut mine: Vec<&Assist> = learned_assists.iter().filter_map(|id| t.assists.get(id)).filter(|a| a.race_id == devil.race_id).collect();
        mine.sort_by_key(|a| a.id);
        let assists: Vec<String> = mine.iter().map(|a| format!("L{} {}", (a.id - 1) / 72 + 2, assist_text(a))).collect();
        // each learned boost's picture from the digitalize window (same order as `assists`), and its lock for a level with none learned
        let assist_icons: Vec<Option<String>> = mine.iter().map(|a| assist_icon_name(&a.r#type).and_then(|n| item_icons::client_asset(store, n, 0))).collect();
        let lock_icon = item_icons::client_asset(store, "digitalize_assist_lockicon", 0);
        let learned = |typ: &str| learned_assists.iter().any(|id| t.assists.get(id).is_some_and(|a| a.race_id == devil.race_id && a.r#type == typ));
        let skill_name = |id: i32| -> Result<String> { Ok(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };

        // Demon Force stack: what each entry gives while digitalized (the FORCE_STACK assist), as Digitalize.Compute lists it
        let stack_fx: Vec<&Effect> = r.effects.iter().filter(|e| e.cond_note.starts_with("force stack")).collect();
        let force_stack = obj(json!({
            "count": demon.force_stack.as_ref().map_or(0, |f| f.iter().filter(|&&x| x > 0).count()), "counted": learned("FORCE_STACK"),
            "effects": stack_fx.iter().filter(|e| e.target == "self" && e.cond != "unparsed").map(|e| fx_text(e)).collect::<Vec<_>>(),
            "other": stack_fx.iter().filter(|e| e.target != "self" || e.cond == "unparsed").map(|e| fx_text(e)).collect::<Vec<_>>(),
        }));
        // Demon equipment: the item's skill replaces the trait of its slot (counted with the TRAITS assist)
        let mut equipment: Vec<Row> = Vec::new();
        for i in 0..4usize {
            let Some(item) = demon.equipment.as_ref().and_then(|e| e.get(i).copied()).filter(|&x| x > 0) else { continue };
            let eq_skill = t.equipment_skill.get(&item).copied();
            let name = match eq_skill {
                Some(sk) => skill_name(sk)?,
                None => format!("item {item}"),
            };
            let replaces = match devil.traits.get(i) {
                Some(&tr) if tr > 0 => Some(skill_name(tr)?),
                _ => None,
            };
            // e.Item.EndsWith(name or "\0"): the culture-sensitive "\0" is ignorable, so it ends every text
            let suffix = match eq_skill {
                Some(sk) => Some(skill_name(sk)?),
                None => None,
            };
            let effects: Vec<String> = r
                .effects
                .iter()
                .filter(|e| e.cond_note.starts_with("equipment") && suffix.as_deref().is_none_or(|sfx| e.item.ends_with(sfx)))
                .map(fx_text)
                .collect();
            equipment.push(obj(json!({ "slot": i + 1, "item": item, "name": name, "replaces": replaces, "effects": effects, "counted": learned("TRAITS") })));
        }
        // Demon Force values: the summoned demon's own stats only (digitalizing uses the base stats, without them)
        const FORCE_NAMES: [&str; 20] = [
            "HP", "MP", "STR", "MAG", "VIT", "INT", "SPD", "LUCK", "HP regen", "MP regen", "CLS-R", "LNG-R", "Spell", "Support", "Critical", "P.Def", "M.Def", "Crit Def", "XP %", "Ailment Def",
        ];
        let force: Vec<String> = match &demon.force_values {
            None => Vec::new(),
            Some(fv) => (0..20).filter(|&i| fv.get(i).copied().unwrap_or(0) >= 100000).map(|i| format!("{} +{}", FORCE_NAMES[i], fv[i] / 100000)).collect(),
        };

        let skills = self.demon_skills(s, r)?;

        // Max HP / Max MP while digitalized, from the last saved normal stats (CharacterHpMp; checked to the point on
        // Specter 2495/662 and Shikigami 1317/453). Percent toggles: the ones on at the last snapshot.
        let mut hpmp: Value = Value::Null;
        let toggles = store.active_toggles()?;
        let (cal, at) = character_hp_mp::from_saved_stats(&store.last_stats_path(), data.passives()?, Some(&toggles))?;
        if let Some(cal) = cal {
            let (f_hp, f_mp) = character_hp_mp::digitalize_only_flat(&r.effects);
            let (d_hp, d_mp) =
                character_hp_mp::digitalized_with_result(&cal, &r.stats, &r.effects, r.hp(), r.mp(), f_hp.wrapping_add(gear_digi_flat.0 as i32), f_mp.wrapping_add(gear_digi_flat.1 as i32));
            hpmp = Value::Object(obj(json!({ "hp": d_hp, "mp": d_mp, "normal_hp": cal.max_hp, "normal_mp": cal.max_mp, "from": at })));
        }
        let summary = bonuses::demon_summary(&r.effects);
        let combat: serde_json::Map<String, Value> = COMBAT_KEYS.iter().map(|(csv, key)| ((*key).to_string(), json!(summary.get(*csv).copied().unwrap_or(0.0)))).collect();
        let stats: Vec<Row> = r.stats.iter().filter(|(_, v)| **v != 0).map(|(k, v)| obj(json!({ "stat": stat_short(*k), "value": v }))).collect();
        json::save(
            &path,
            &json!({
                "demon": demon.name, "level": demon.level, "race": title(&devil.race), "rate": r.rate, "max_rate": r.max_rate,
                "mitama": if demon.mitama_type > 0 && demon.mitama_type < 5 { json!(MITAMA_NAMES[demon.mitama_type as usize]) } else { Value::Null },
                "alignment": data.devil_alignment(demon.r#type)?,
                "assists": assists, "assist_icons": assist_icons, "assist_lock_icon": lock_icon,
                "assists_known": s.assists.is_some(), "mitama_boost": s.mitama_boost,
                "stats": rows_value(&stats),
                // what it adds to each key stat (page keys), for the Overview while digitalized
                "combat": Value::Object(combat),
                "taken_at": s.taken_at, "digitalized_when_taken": s.digitalized_when_taken,
                "force_stack": force_stack, "equipment": rows_value(&equipment), "force": force, "hp_mp": hpmp,
                "skills": rows_value(&skills),
                "notes": r.notes,
            }),
        )
    }

    /// A demon's own skills, for the page: the 8 skill slots (what digitalizing uses; Digitalize.Compute), then its
    /// features (a worn demon equipment's skill stands in the feature slot it replaces). Each says whether you get
    /// it while digitalized and, when not, what is missing. Effects are the ones Compute listed for that skill.
    fn demon_skills(&self, s: &DigitalizeSave, r: &DigitalizeResult) -> Result<Vec<Row>> {
        let store = self.store();
        let data = self.data();
        let t = data.demons()?;
        let demon = s.demon.as_ref().ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
        let devil = r.devil.as_ref().ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
        let assists = s.assists.clone().unwrap_or_default();
        let learned = |typ: &str| assists.iter().any(|id| t.assists.get(id).is_some_and(|a| a.race_id == devil.race_id && a.r#type == typ));
        let skill_name = |id: i32| -> Result<String> { Ok(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };
        let known = s.assists.is_some();
        let race = title(&devil.race);
        let needs = |typ: &str, boost: &str| -> Option<String> {
            if !known {
                Some("Your Digitalize boosts couldn’t be read".to_string())
            } else if learned(typ) {
                None
            } else {
                Some(format!("Needs the {race} {boost} boost")) // the race is named: the page shows this in the icon's tooltip
            }
        };
        let fx_of = |id: i32, part: &str| -> Result<Vec<String>> {
            let name = skill_name(id)?;
            let mut res: IndexSet<String> = IndexSet::new();
            for e in r.effects.iter().filter(|e| e.skill_id == id && e.cond_note.starts_with(part)) {
                let mut text = fx_text(e);
                if let Some(rest) = text.strip_prefix(&format!("{name}: ")) {
                    text = title1(rest); // "<name>: no stat effect (...)"
                }
                let note = demon_note(&e.cond_note).split("; ").filter(|x| !x.is_empty() && *x != part && !x.starts_with("needs the ")).collect::<Vec<_>>().join("; ");
                res.insert(if !note.is_empty() { format!("{text} ({note})") } else { text });
            }
            Ok(res.into_iter().collect())
        };
        let skill_row = |kind: &str, id: i32, part: Option<&str>, why: Option<String>, replaces: Option<String>| -> Result<Row> {
            Ok(obj(json!({
                "kind": kind, "id": id, "name": skill_name(id)?, "icon": item_icons::skill_asset(store, data, id, 0)?,
                "counted": why.is_none(), "why": why, "replaces": replaces,
                "effects": match part { Some(p) => fx_of(id, p)?, None => Vec::new() }, "tip": data.skill_text(id)?,
            })))
        };
        let mut skills: Vec<Row> = Vec::new();
        for &id in demon.skills.iter().filter(|&&x| x > 0) {
            let (cat, restricted) = t.skills.get(&id).copied().unwrap_or((-1, false));
            let kind = match cat {
                0 => "Passive",
                1 => "Active",
                2 => "Switch",
                _ => "Skill",
            };
            let why = if restricted {
                Some("Digitalizing leaves this skill out".to_string())
            } else if cat == 0 {
                needs("PASSIVES", "Passive skills")
            } else if cat == 1 {
                needs("ACTIVES", "Active skills")
            } else if cat == 2 {
                Some("Switch skills are not carried over when digitalizing".to_string())
            } else {
                Some("The game data does not say what kind of skill this is".to_string())
            };
            skills.push(skill_row(kind, id, if cat == 0 && !restricted { Some("passive") } else { None }, why, None)?);
        }
        for i in 0..4usize {
            let equip = demon.equipment.as_ref().and_then(|e| e.get(i).copied()).unwrap_or(0);
            let tr = devil.traits.get(i).copied().unwrap_or(0);
            if equip > 0 && t.equipment_skill.contains_key(&equip) {
                let eq = t.equipment_skill[&equip];
                let mut row = skill_row("Equipment", eq, Some("equipment"), needs("TRAITS", "Features"), if tr > 0 { Some(skill_name(tr)?) } else { None })?;
                // its picture: the Epitaph Parts item it is made from (the game has no icon for the skill itself)
                if let Some(a) = item_icons::item_asset(store, data, equip, 0)? {
                    row.insert("icon".to_string(), json!(a));
                }
                skills.push(row);
            } else if equip <= 0 && tr > 0 {
                let mut row = skill_row("Feature", tr, Some("trait"), needs("TRAITS", "Features"), None)?;
                // the game has no picture for a feature itself (CSkillData iconID 0): the digitalize window's Features boost picture stands in
                if row.get("icon").is_none_or(Value::is_null) {
                    if let Some(a) = item_icons::client_asset(store, "digitalize_featureicon", 0) {
                        row.insert("icon".to_string(), json!(a));
                    }
                }
                skills.push(row);
            }
        }
        Ok(skills)
    }

    /// The same numbers `build_digitalize` writes for the chosen demon (stat rate, the stats it adds, its key bonuses, its
    /// skills), for any demon: the learned assists and the Mitama boost belong to the character. None when the demon
    /// isn't in the demon table.
    pub fn offer(&self, demon: &DemonRecord, assists: Option<&[i32]>, mitama_boost: bool) -> Result<Option<DemonOffer>> {
        let s = DigitalizeSave { taken_at: None, digitalized_when_taken: false, demon: Some(demon.clone()), assists: assists.map(<[i32]>::to_vec), mitama_boost };
        let r = s.compute(&self.ctx, self.store().tooltips_level()?)?;
        if r.devil.is_none() {
            return Ok(None);
        }
        let sum = bonuses::demon_summary(&r.effects);
        Ok(Some(DemonOffer {
            alignment: self.data().devil_alignment(demon.r#type)?.map(str::to_string),
            rate: r.rate,
            max_rate: r.max_rate,
            skills: self.demon_skills(&s, &r)?,
            stats: r.stats.iter().filter(|(_, v)| **v != 0).map(|(k, v)| obj(json!({ "stat": stat_short(*k), "value": v }))).collect(),
            combat: COMBAT_KEYS.iter().map(|(csv, key)| ((*key).to_string(), sum.get(*csv).copied().unwrap_or(0.0))).collect(),
        }))
    }

    /// dash\digi-unlocks.json: the demons each Digitalization Level unlocks (GuardianLevelData "demonIds",
    /// reference\guardian-levels.tsv), for every race the character has enabled (its levels as last read,
    /// expertise-ranks.json "digitalize_levels"). One entry per race: its level and its demons, each with the level
    /// that unlocks it, whether that level is reached, its icon and whether one is in the COMP demons seen.
    pub fn build_unlocks(&self) -> Result<()> {
        let store = self.store();
        let data = self.data();
        let path = store.dash_dir().join("digi-unlocks.json");
        let t = data.demons()?;
        let levels: Vec<Value> = if store.expertise_ranks_path().exists() { json::arr(json::load(store.expertise_ranks_path())?.at("digitalize_levels")).cloned().collect() } else { Vec::new() };
        let mut in_comp: HashSet<String> = HashSet::new(); // by name: a COMP demon can be a Mitama-fused type of the same demon
        if store.comp_demons_path().exists() {
            // catch (JsonException) { }
            if let Ok(Value::Array(a)) = json::load(store.comp_demons_path()) {
                for e in &a {
                    if let Some(nm) = json::str(e.at("Demon").at("Name")).filter(|n| !n.is_empty()) {
                        in_comp.insert(ic_key(&nm));
                    }
                }
            }
        }
        let mut races: Vec<Row> = Vec::new();
        for dl in &levels {
            let race = json::int(dl.at("race_id"));
            let level = json::int(dl.at("level"));
            if level < 1 {
                continue;
            }
            let Some(lv) = t.levels.get(&race) else { continue };
            let mut demons: Vec<Row> = Vec::new();
            for l in lv.iter().filter(|l| l.level >= 1) {
                for &id in l.demon_ids.iter().filter(|&&id| id > 0) {
                    let devil = t.devils.get(&id);
                    let Some(devil) = devil else { continue }; // .Where(d => d["name"] != null)
                    let png = DigitalizeSave::icon(store, data, 0, id)?;
                    demons.push(obj(json!({
                        "level": l.level, "type": id, "name": devil.name, "base_level": devil.base_level,
                        "alignment": data.devil_alignment(id)?, "unlocked": l.level <= level, "in_comp": in_comp.contains(&ic_key(&devil.name)),
                        "icon": png.map(|p| format!("data:image/png;base64,{}", base64(&p))),
                    })));
                }
            }
            races.push(obj(json!({ "race_id": race, "race": title(&t.race_name(race)), "level": level, "demons": rows_value(&demons) })));
        }
        json::save_if_changed(path, &rows_value(&races)) // ~1 MB (icons inside) and nearly always the same
    }

    /// The names the bonus list shows for the worn pieces: tooltip name (with tarot / soul words) -> plain item name.
    pub fn short_names(&self, recs: &[SlotTip]) -> Result<IndexMap<String, String>> {
        let data = self.data();
        let mut short: IndexMap<String, String> = IndexMap::new(); // OrdinalIgnoreCase: keys are ic_key(name)
        for r in recs.iter().filter(|r| r.id > 0) {
            let full = if !r.text.is_empty() { Some(TooltipParser::info(&r.text).base_name) } else { data.item_name(r.id)? };
            if let Some(full) = full {
                let short_name = data.item_name(r.id)?.unwrap_or_else(|| full.clone());
                short.insert(ic_key(&full), short_name);
            }
        }
        Ok(short)
    }

    /// bonuses.json: gear + expertise effects merged by bonus, with gear / expertise / total / digitalized and sources.
    pub fn build_bonuses(&self, expertise_rows: &[Row]) -> Result<Vec<Row>> {
        let store = self.store();
        let recs = store.load_tooltips()?;
        let (texts, ids, tarot, soul) = Store::unpack(&recs);
        let details = Store::unpack_details(&recs);
        let level = store.tooltips_level()?;
        let gear = self.ctx.gear()?.compute(&texts, Some(&ids), tarot.as_deref(), soul.as_deref(), details.as_deref(), level, None)?;
        // The digitalize demon: everything it gives applies only while digitalized
        let dsave = DigitalizeSave::load(store)?;
        let dres = match dsave.as_ref().filter(|d| d.demon.is_some()) {
            Some(d) => Some(d.compute(&self.ctx, store.tooltips_level()?)?),
            None => None,
        };
        let gear_digi = |label: &str| -> f64 {
            let mut s = 0.0f64;
            for e in gear.effects.iter().filter(|e| e.cond == "digitalized" && !e.pct && e.value.is_some() && e.target == "self" && e.label == label) {
                s += e.value.unwrap_or(0.0);
            }
            s
        };
        self.build_digitalize(dsave.as_ref(), dres.as_ref(), (gear_digi("Max HP"), gear_digi("Max MP")))?;
        let mut icons: IndexMap<String, Option<String>> = IndexMap::new(); // icons.json: "quest" / "clan" / "compendium" / "mitama" -> data: URI
        icons.insert("mitama".into(), item_icons::mitama_mark_data_uri(store, 0)); // the mark on a Mitama demon's picture (every page that shows one)
        // the Rebirth Planner's pictures: the twelve rune glyphs and the four Mitama kinds, from the game's Rebirth window
        for i in 0..12 {
            icons.insert(format!("rune{i}"), item_icons::rune_data_uri(store, i, 0));
        }
        for k in 0..4 {
            icons.insert(format!("kind{k}"), item_icons::mitama_kind_data_uri(store, k, 0));
        }
        // ... and the four Mitama demons' own faces (32 x 32, the sharpest the client has: reference\devil-icons.tsv,
        // 624 Ara, 623 Nigi, 622 Kusi, 621 Saki), for the bigger tiles
        for (k, name) in ["icon_D2_07_004a", "icon_D2_07_003a", "icon_D2_07_002a", "icon_D2_07_001a"].iter().enumerate() {
            icons.insert(format!("face{k}"), item_icons::client_data_uri(store, name, 0));
        }
        // ... and its cost pictures: the Magical Golden Apple's item icon and the Macca item's (799: the gold coin the
        // inventory shows by the macca count; Interface\tga\win_macca.tga is only the small "M" sign)
        icons.insert("apple".into(), item_icons::data_uri(store, self.data(), 21941, 0)?);
        icons.insert("macca".into(), item_icons::data_uri(store, self.data(), 799, 0)?);
        // ... and the rebirth cost items (runestones, incense, soul stones, relics: reference\reunion-growth.tsv): their
        // icons go beside the skill icons (dash\skill-icons\<name>.png, item_asset); icons.json holds the name per item
        if let Some(lines) = crate::data::game_data::reference_lines("reunion-growth.tsv")? {
            let mut ids: Vec<i32> = Vec::new();
            for line in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')) {
                if let Some(col) = line.split('\t').nth(4) {
                    for id in col.split(';').filter_map(|p| p.split_once(':')).filter_map(|(a, _)| crate::fmt::parse_i32(a)) {
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                    }
                }
            }
            for id in ids {
                icons.insert(format!("item{id}"), item_icons::item_asset(store, self.data(), id, 0)?);
            }
        }
        // ... and the Summon PGs the Rebirth Planner lists as the Mitamas' sources: the four kinds' (reference\mitama-pg.tsv)
        // and NewMoon's Sacred Soul, with the PG item's own picture (user, 2026-11-27: not the Mitama's face), the gems
        // those trades ask, and the Fighting Spirit Saien asks (the cost card shows them as item tiles)
        let mut pgs: Vec<i32> = vec![super::rebirth_planner::SACRED_SOUL_PG, super::rebirth_planner::FIGHTING_SPIRIT];
        if let Some(lines) = crate::data::game_data::reference_lines("mitama-pg.tsv")? {
            for line in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')) {
                let cols: Vec<&str> = line.split('\t').collect();
                let gems = cols.get(4).map(|c| c.split(';').filter_map(|p| p.split_once(':')).filter_map(|(a, _)| crate::fmt::parse_i32(a)).collect::<Vec<i32>>()).unwrap_or_default();
                for id in cols.get(1).and_then(|c| crate::fmt::parse_i32(c)).into_iter().chain(gems) {
                    if !pgs.contains(&id) {
                        pgs.push(id);
                    }
                }
            }
        }
        // ... and every item an NPC asks for a demon skill (reference\skill-barter-items.tsv, made by skilldb): the planner's
        // cost card shows a skill's price as item pictures
        if let Some(lines) = crate::data::game_data::reference_lines("skill-barter-items.tsv")? {
            for id in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| l.split('\t').next().and_then(crate::fmt::parse_i32)) {
                if !pgs.contains(&id) {
                    pgs.push(id);
                }
            }
        }
        // ... and every demon equipment item (reference\devil-equipment-items.tsv): the planner's equipment step shows them
        if let Some(lines) = crate::data::game_data::reference_lines("devil-equipment-items.tsv")? {
            for id in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| l.split('\t').next().and_then(crate::fmt::parse_i32)) {
                if !pgs.contains(&id) {
                    pgs.push(id);
                }
            }
        }
        // ... and every Demon Force item that puts an effect in a stack slot (reference\devil-boost-items.tsv): the
        // planner's Demon Force bar shows the items' pictures
        if let Some(lines) = crate::data::game_data::reference_lines("devil-boost-items.tsv")? {
            for id in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| l.split('\t').next().and_then(crate::fmt::parse_i32)) {
                if !pgs.contains(&id) {
                    pgs.push(id);
                }
            }
        }
        for id in pgs {
            icons.insert(format!("item{id}"), item_icons::item_asset(store, self.data(), id, 0)?);
        }
        let mut parts = Dashboard::parts(&gear, &self.short_names(&recs)?, dres.as_ref(), expertise_rows);
        // What VIT, INT and SPD take off cooldown and incantation time (from the last saved normal stats)
        if store.last_stats_path().exists() {
            let last = json::load(store.last_stats_path())?;
            let pairs: Vec<i32> = json::arr(last.at("pairs")).map(|p| json::int(Some(p))).collect();
            if pairs.len() >= 10 {
                let s = |k: usize| pairs[2 * k].wrapping_add(pairs[2 * k + 1]);
                let normal = [s(2), s(3), s(4)];
                parts.extend(Dashboard::stat_time_parts(normal, dres.as_ref().map(|d| Dashboard::digitalized_time_stats(normal, d, &gear.effects))));
            }
        }
        // The demon summoned beside the character (as last seen in the game) gives it what its tokusei aim at the summoner
        let seen = super::pipeline::load_seen(store)?;
        if let Some(partner) = Dashboard::summoned_demon(seen.values().map(|s| &s.demon)) {
            let boost = match DigitalizeSave::load_progress(store) {
                Some(p) => p.mitama_boost,
                None => dsave.as_ref().is_some_and(|s| s.mitama_boost),
            };
            parts.extend(self.summoner_parts(partner, boost)?);
        }
        let sorted = self.bonus_rows(parts, Some(&mut icons))?;
        json::save(store.dash_dir().join("bonuses.json"), &rows_value(&sorted))?;
        let icons_json: serde_json::Map<String, Value> = icons.iter().filter_map(|(k, v)| v.as_ref().map(|v| (k.clone(), json!(v)))).collect();
        json::save(store.dash_dir().join("icons.json"), &Value::Object(icons_json))?;
        Ok(sorted)
    }

    /// Every bonus line of a build, source by source: the gear's, the digitalize demon's, then the expertise rows'.
    pub fn parts(gear: &GearResult, short_name: &IndexMap<String, String>, dres: Option<&DigitalizeResult>, expertise_rows: &[Row]) -> Vec<Part> {
        let mut all = Vec::new();
        for e in &gear.effects {
            if e.value.is_none() || matches!(e.cond.as_str(), "inactive" | "unparsed" | "fusion-set") {
                continue;
            }
            let from = if e.item.starts_with("Set: ") || e.item.starts_with("Fusion set: ") { e.item.clone() } else { short_name.get(&ic_key(&e.item)).cloned().unwrap_or_else(|| e.item.clone()) };
            let tag = match e.source.as_str() {
                "appearance" => " (appearance)",
                "tarot" => " (tarot)",
                "soul" => " (soul)",
                "mod" => " (mod)",
                _ => "",
            };
            all.push(Part {
                kind: "gear".into(),
                label: e.label.clone(),
                value: e.value.unwrap_or(0.0),
                pct: e.pct,
                target: e.target.clone(),
                cond: e.cond.clone(),
                note: e.cond_note.clone(),
                from: format!("{from}{tag}"),
                fuse: e.source == "item" && e.cond_note == "fusion great success",
                ..Default::default()
            });
        }
        if let Some(d) = dres {
            for e in &d.effects {
                if e.value.is_none() || matches!(e.cond.as_str(), "inactive" | "unparsed") {
                    continue;
                }
                all.push(Part {
                    kind: "demon".into(),
                    label: e.label.clone(),
                    value: e.value.unwrap_or(0.0),
                    pct: e.pct,
                    target: e.target.clone(),
                    cond: e.cond.clone(),
                    note: demon_note(&e.cond_note),
                    from: e.item.clone(),
                    skill_id: e.skill_id,
                    demon: true,
                    demon_stat: e.cond_note.starts_with("stats"),
                    ..Default::default()
                });
            }
        }
        for r in expertise_rows {
            let Some(Value::Array(fx)) = r.get("fx") else { continue };
            let text = |k: &str| match r.get(k) {
                Some(Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            for f in fx {
                all.push(Part {
                    kind: "expertise".into(),
                    label: json::str(f.at("label")).unwrap_or_default(),
                    value: json::num(f.at("value")).unwrap_or(0.0),
                    pct: f.at("pct").and_then(Value::as_bool).unwrap_or(false),
                    target: json::str(f.at("target")).unwrap_or_default(),
                    cond: json::str(f.at("cond")).unwrap_or_default(),
                    note: f.at("note").and_then(Value::as_str).unwrap_or("").to_string(),
                    from: format!("{} ({})", text("skill"), text("expertise")),
                    skill_id: r.get("skill_id").and_then(Value::as_i64).map_or(0, |i| i as i32),
                    // a switch skill that is on, or a line of another skill that needs one on (Cataclysmic Mind's +500% Spell damage)
                    toggle: r.get("toggle_on").and_then(Value::as_bool).unwrap_or(false) || f.at("toggle").and_then(Value::as_bool).unwrap_or(false),
                    ..Default::default()
                });
            }
        }
        all
    }

    /// VIT, INT and SPD while digitalized: the normal ones plus the demon's stats at the digitalize rate, the flat stat
    /// effects the digitalize gives (force stack, passives) and gear stats that only work digitalized.
    pub fn digitalized_time_stats(normal: [i32; 3], d: &DigitalizeResult, gear: &[Effect]) -> [i32; 3] {
        let mut out = normal;
        for (i, idx) in [2, 3, 4].into_iter().enumerate() {
            out[i] = out[i].wrapping_add(d.stats.get(&idx).copied().unwrap_or(0));
        }
        let mut add = |e: &Effect| {
            if e.pct || e.target != "self" {
                return;
            }
            let Some(v) = e.value else { return };
            let v = v as i32;
            match bonuses::canonical(&e.label, &GameStatLabels).0.as_str() {
                "VIT" => out[0] = out[0].wrapping_add(v),
                "INT" => out[1] = out[1].wrapping_add(v),
                "SPD" => out[2] = out[2].wrapping_add(v),
                "All stats" => out.iter_mut().for_each(|o| *o = o.wrapping_add(v)),
                _ => {}
            }
        };
        for e in d.effects.iter().filter(|e| e.counted() && !e.cond_note.starts_with("stats")) {
            add(e);
        }
        for e in gear.iter().filter(|e| e.cond == "digitalized") {
            add(e);
        }
        out
    }

    /// What VIT, INT and SPD take off cooldown and incantation time (kind "stats", see [`bonuses::stat_time_cut`]):
    /// a normal part ("always", counted in the normal total only) and, with `digi`, a digitalized one (counted in the
    /// digitalized total only). `normal` / `digi`: final [VIT, INT, SPD].
    pub fn stat_time_parts(normal: [i32; 3], digi: Option<[i32; 3]>) -> Vec<Part> {
        let mut all = Vec::new();
        let mut push = |stats: [i32; 3], cond: &str| {
            for (label, main, name) in [("Cooldown", stats[0], "VIT"), ("Incantation", stats[1], "INT")] {
                all.push(Part {
                    kind: "stats".into(),
                    label: label.into(),
                    value: bonuses::stat_time_cut(main, stats[2]),
                    pct: true,
                    target: "self".into(),
                    cond: cond.into(),
                    note: format!("2.5% per 10 {name} and 1.5% per 10 SPD, rounded down"),
                    from: format!("{name} {main}, SPD {}", stats[2]),
                    ..Default::default()
                });
            }
        };
        push(normal, "always");
        if let Some(d) = digi {
            push(d, "digitalized");
        }
        all
    }

    /// The demon summoned beside the character among the demons seen: the game's mark at the last read, with its details.
    pub fn summoned_demon<'a>(demons: impl IntoIterator<Item = &'a DemonRecord>) -> Option<&'a DemonRecord> {
        demons.into_iter().find(|d| d.summoned() && d.loaded())
    }

    /// The bonus lines the summoned partner `demon` gives its summoner (kind "summoned", source "<name> (summoned)"): see
    /// `Digitalize::summoner_effects`. Counted in a normal total, never in a digitalized one (no partner while digitalized).
    /// Kept apart from [`Dashboard::parts`]: the caller adds them to a build's parts.
    pub fn summoner_parts(&self, demon: &DemonRecord, mitama_boost: bool) -> Result<Vec<Part>> {
        let effects = self.ctx.digitalize(self.store().tooltips_level()?)?.summoner_effects(demon, mitama_boost)?;
        Ok(Dashboard::summoner_parts_of(&effects))
    }

    /// The parts of effects already worked out by `Digitalize::summoner_effects`.
    pub fn summoner_parts_of(effects: &[Effect]) -> Vec<Part> {
        effects
            .iter()
            .filter(|e| e.value.is_some() && !matches!(e.cond.as_str(), "inactive" | "unparsed"))
            .map(|e| Part {
                kind: "summoned".into(),
                label: e.label.clone(),
                value: e.value.unwrap_or(0.0),
                pct: e.pct,
                target: e.target.clone(),
                cond: e.cond.clone(),
                note: e.cond_note.clone(),
                from: e.item.clone(),
                skill_id: e.skill_id,
                item_id: e.item_id,
                picture: e.picture.clone(),
                ..Default::default()
            })
            .collect()
    }

    /// The picture of a stat-up skill ("Vitality UP1", "Speed UP1", "Intelligence UP1": what the Expertise page draws them with), found
    /// by the skill's name in the skill list; saved beside the skill icons. None when the game has no such skill or picture.
    fn stat_up_icon(&self, stat: &str) -> Result<Option<String>> {
        let want = format!("{stat} UP1");
        let id = self.data().skill_names()?.iter().filter(|(_, n)| n.as_deref() == Some(want.as_str())).map(|(&id, _)| id).min();
        match id {
            Some(id) => item_icons::skill_asset(self.store(), self.data(), id, 0),
            None => Ok(None),
        }
    }

    /// The first step of `bonus_rows`: each part's `group` and `bonus` (the bonus list's names). The C# `Part` is a class,
    /// so the Simulator sees these two fields on its own list after `BonusRows` has run; here that is a separate call
    /// (idempotent: it only reads the other fields).
    pub fn label_parts(all: &mut [Part]) {
        for p in all.iter_mut() {
            let (c_name, c_group) = bonuses::canonical(&p.label, &GameStatLabels);
            p.group = if p.cond == "situational" {
                "Only in some fights".to_string()
            } else if p.target != "self" {
                "Partner".to_string()
            } else {
                c_group.to_string()
            };
            p.bonus = if p.cond == "situational" && !p.note.is_empty() {
                format!("{c_name} ({})", rx::rep(&rx::rep(&p.note, "^When ", "when "), "^Against", "against"))
            } else {
                c_name
            };
        }
    }

    /// The bonus list's rows from the parts: one row per bonus, with gear / expertise / demon / totals and its sources.
    /// icons: where the quest, clan and compendium pictures go (icons.json); None = no pictures looked up at all
    /// (the simulator: nothing is written).
    pub fn bonus_rows(&self, mut all: Vec<Part>, mut icons: Option<&mut IndexMap<String, Option<String>>>) -> Result<Vec<Row>> {
        let store = self.store();
        let data = self.data();
        let group_order = |g: &str| match g {
            "Combat" => 0,
            "Affinity" => 1,
            "Stats" => 2,
            "Other" => 3,
            "Partner" => 4,
            _ => 5, // "Only in some fights"
        };
        let combat_order: HashMap<String, usize> = bonuses::STAT_MAP.iter().enumerate().map(|(i, s)| (rx::rep(s.0, " %$", ""), i)).collect();
        Dashboard::label_parts(&mut all);

        let r2 = |v: f64| fmt::g(fmt::round(v, 2));
        let mut rows: Vec<(Row, i32, f64, String)> = Vec::new();

        // GroupBy (group, bonus, pct) in lower case: first appearance of each key
        let mut groups: IndexMap<(String, String, bool), Vec<usize>> = IndexMap::new();
        for (i, p) in all.iter().enumerate() {
            groups.entry((p.group.to_lowercase(), p.bonus.to_lowercase(), p.pct)).or_default().push(i);
        }
        for (_, idx) in &groups {
            let list: Vec<&Part> = idx.iter().map(|&i| &all[i]).collect();
            let f = list[0];
            let counted = f.group != "Only in some fights";
            let sum = |kind: &str, conds: &[&str]| -> f64 {
                let mut s = 0.0f64;
                for p in list.iter().filter(|p| p.kind == kind && conds.contains(&p.cond.as_str())) {
                    s += p.value;
                }
                s
            };
            let mut gear_a = sum("gear", &["always"]);
            let mut gear_d = sum("gear", &["always", "digitalized"]);
            let mut exp_a = sum("expertise", &["always"]);
            let mut demon_d = sum("demon", &["digitalized"]);
            // what the summoned partner gives its summoner (kind "summoned"): in the normal totals, not the digitalized ones
            let mut summoned_a = sum("summoned", &["always"]);
            let has_summoned = list.iter().any(|p| p.kind == "summoned");
            // what the character's own stats take off (kind "stats"): one part per mode, each in its own total
            let has_stats = list.iter().any(|p| p.kind == "stats");
            let stats_a = sum("stats", &["always"]);
            let stats_d = if list.iter().any(|p| p.kind == "stats" && p.cond == "digitalized") { sum("stats", &["digitalized"]) } else { stats_a };
            if !counted {
                // Fight-dependent bonuses usually exclude each other (one partner race, one enemy alignment): show the largest.
                let best = |kind: &str| -> f64 {
                    let mut best: Option<f64> = None;
                    for p in list.iter().filter(|p| p.kind == kind) {
                        if best.is_none_or(|b| p.value.abs() > b.abs()) {
                            best = Some(p.value);
                        }
                    }
                    best.unwrap_or(0.0)
                };
                gear_a = best("gear");
                gear_d = gear_a;
                exp_a = best("expertise");
                demon_d = best("demon");
                summoned_a = best("summoned");
            }
            let unit = if f.pct { "%" } else { "" };
            let fmt_v = |v: f64| format!("{}{}{unit}", if v > 0.0 { "+" } else { "" }, r2(v));
            let from_text = list
                .iter()
                .map(|p| format!("{} {}{}", p.from, fmt_v(fmt::round(p.value, 2)), if p.cond == "digitalized" { " (digi)" } else { "" }))
                .collect::<Vec<_>>()
                .join("; ");
            let mut parts: Vec<Row> = Vec::new();
            for p in &list {
                let mut src = p.from.clone();
                let mut tag = String::new();
                if let Some(m) = rx::m(&p.from, r"^(.*) \((appearance|tarot|soul|mod)\)$") {
                    src = m[1].to_string();
                    tag = m[2].to_string();
                }
                // Digitalization Level skills count with expertise (permanent passives) but are labelled apart
                // Quest bonus and compendium rows ride along with expertise (Expertise.Build) but are labelled apart too
                let mut kind: String = if p.kind == "expertise" && p.from.contains("(Digitalize: ") {
                    "dlevel".into()
                } else if p.kind == "expertise" && p.from.contains("(Quest bonus") {
                    "quest".into()
                } else if p.kind == "expertise" && p.from.contains("(Demonic Compendium") {
                    "compendium".into()
                } else if p.kind == "expertise" || p.kind == "demon" || p.kind == "summoned" || p.kind == "stats" {
                    p.kind.clone()
                } else if src.to_uppercase().starts_with("SET: ") {
                    "set".into()
                } else if src.to_uppercase().starts_with("FUSION SET: ") {
                    "fusion set".into()
                } else if !tag.is_empty() {
                    tag.clone()
                } else {
                    "item".into()
                };
                let src = rx::rep(&src, "^(Fusion set|Set): ", "");
                // A set's label ends in its internal number, "(set 1792)": it tells the sets apart for the page (which worn
                // set a bonus belongs to, "set_key") but is not a name, so the shown source is the name alone.
                let (src, set_key) = match rx::m(&src, r"^(.*) \((fusion set|set) (\d+)\)$") {
                    Some(m) if kind == "set" || kind == "fusion set" => (m[1].to_string(), Some(format!("{}:{}", &m[2], &m[3]))),
                    _ => (src, None),
                };
                // Its icon, as a key into icons.json: the skill's own, else the demon's picture for what the demon itself gives.
                let mut icon: Option<String> = None;
                if icons.is_some() && p.skill_id > 0 {
                    // "skill:<icon name>": the page loads dash\skill-icons\<icon name>.png (the same files as Expertise and Digitalize)
                    if let Some(asset) = item_icons::skill_asset(store, data, p.skill_id, 0)? {
                        icon = Some(format!("skill:{asset}"));
                    }
                }
                if icons.is_some() && icon.is_none() {
                    // the summoned demon's equipment has no picture for its skill: the Epitaph Parts item it is made from; its Demon
                    // Force and Mitama set: the digitalize window's picture for that boost (both saved beside the skill icons)
                    let asset = if p.item_id > 0 { item_icons::item_asset(store, data, p.item_id, 0)? } else { item_icons::client_asset(store, &p.picture, 0) };
                    icon = asset.map(|a| format!("skill:{a}"));
                }
                if icons.is_some() && icon.is_none() && p.kind == "stats" {
                    // "Your stats": the stat-up skills' picture of the stat that mostly sets it (cooldown: VIT, incantation: INT; SPD is the second one)
                    let stat = match p.label.as_str() {
                        "Cooldown" => Some("Vitality"),
                        "Incantation" => Some("Intelligence"),
                        _ => None,
                    };
                    icon = match stat {
                        Some(s) => self.stat_up_icon(s)?.map(|a| format!("skill:{a}")),
                        None => None,
                    };
                }
                if icon.is_none() && p.demon {
                    icon = Some("demon".into());
                }
                if let Some(icons) = icons.as_mut() {
                    if kind == "quest" {
                        if !icons.contains_key("quest") {
                            icons.insert("quest".into(), item_icons::quest_data_uri(store, 0));
                        }
                        if icons["quest"].is_some() {
                            icon = Some("quest".into());
                        }
                    }
                }
                // Clan level skills are learned passives like the rest, but are labelled apart ("Clan bonuses")
                let clan = kind == "expertise" && p.skill_id > 0 && character_extras::is_clan_skill(data, p.skill_id)?;
                if clan {
                    kind = "clan".into();
                }
                if let Some(icons) = icons.as_mut() {
                    if icon.is_none() && clan {
                        // clan skills have no icon of their own
                        if !icons.contains_key("clan") {
                            icons.insert("clan".into(), item_icons::clan_data_uri(store, 0));
                        }
                        if icons["clan"].is_some() {
                            icon = Some("clan".into());
                        }
                    }
                    if kind == "compendium" {
                        if !icons.contains_key("compendium") {
                            icons.insert("compendium".into(), item_icons::compendium_data_uri(store, 0));
                        }
                        if icons["compendium"].is_some() {
                            icon = Some("compendium".into());
                        }
                    }
                }
                // A summoned demon's Demon Force entry or epitaph is an item (the Demon Soul it is drawn with, the Epitaph Parts it is made from):
                // its name goes after a Demon Force note and heads the icon's tooltip ("item_name")
                let item_name = if p.kind == "summoned" && p.item_id > 0 { data.item_name(p.item_id)? } else { None };
                let note = match &item_name {
                    Some(n) if p.note == "Demon Force" => format!("{} · {n}", p.note),
                    _ => p.note.clone(),
                };
                let mut part = obj(json!({
                    "source": src, "kind": kind, "value": fmt::round(p.value, 2), "digi": p.cond == "digitalized", "note": note, "icon": icon,
                    "tip": if p.skill_id > 0 { data.skill_text(p.skill_id)? } else { None }, // the game's description, for the hover tooltip
                }));
                if let Some(n) = &item_name {
                    part.insert("item_name".into(), json!(n));
                    // the tooltip's effects: a Demon Force stack entry lists all its tokusei in the game's words ("* " before each, as the
                    // game's tooltip has it); an epitaph shows its skill's description without the first line (the demon's own flavour)
                    let tables = data.demons()?;
                    let stack = tables.force_stack_item.iter().find(|&(_, &i)| i == p.item_id).map(|(&s, _)| s);
                    if let Some(toks) = stack.and_then(|s| tables.force_stack_tokusei.get(&s)) {
                        let mut lines: Vec<String> = Vec::new();
                        for &tid in toks {
                            if let Some(text) = data.tokusei_text(tid)? {
                                lines.extend(text.split('\n').enumerate().map(|(i, l)| if i == 0 { format!("* {l}") } else { l.to_string() }));
                            }
                        }
                        if !lines.is_empty() {
                            part.insert("item_tip".into(), json!(lines.join("\n")));
                        }
                    } else if let Some(text) = if p.skill_id > 0 { data.skill_text(p.skill_id)? } else { None } {
                        if let Some((_, effects)) = text.split_once('\n') {
                            part.insert("tip".into(), json!(effects.trim()));
                        }
                    }
                }
                if let Some(k) = &set_key {
                    part.insert("set_key".into(), json!(k));
                }
                if p.toggle {
                    part.insert("toggle".into(), json!(true)); // a toggle skill that is on: the page tags it, and says how much of a total the toggles give
                }
                parts.push(part);
            }
            let id_text = rx::rep(&format!("{}-{}-{unit}", f.group, f.bonus), "[^A-Za-z0-9]+", "-");
            let mut row = obj(json!({
                "row": id_text.trim_matches('-').to_lowercase(),
                "group": f.group, "bonus": f.bonus, "unit": unit, "counted": counted,
                "gear": fmt::round(gear_a, 2), "gear_digi": fmt::round(gear_d, 2), "expertise": fmt::round(exp_a, 2), "demon": fmt::round(demon_d, 2),
                "total": if counted { json!(fmt::round(gear_a + exp_a + summoned_a + stats_a, 2)) } else { Value::Null },
                "total_digi": if counted { json!(fmt::round(gear_d + exp_a + demon_d + stats_d, 2)) } else { Value::Null },
                "from": from_text, "parts": rows_value(&parts),
            }));
            if has_stats {
                // what VIT/INT and SPD take off (in "total" and "total_digi", each with the stats of its mode)
                row.insert("stats".into(), json!(fmt::round(stats_a, 2)));
                row.insert("stats_digi".into(), json!(fmt::round(stats_d, 2)));
            }
            if has_summoned {
                // only with a partner out: what it gives the character (in "total", never in "total_digi")
                row.insert("summoned".into(), json!(fmt::round(summoned_a, 2)));
            }
            let s2 = match combat_order.get(&f.bonus) {
                Some(&ci) if f.group == "Combat" => ci as f64,
                _ => 1000.0 - (gear_d + exp_a + demon_d + summoned_a + stats_a).abs().min(999.0),
            };
            rows.push((row, group_order(&f.group), s2, f.bonus.clone()));
        }
        // en-US, ignoring case
        rows.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then(a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal))
                .then_with(|| cmp_invariant(&a.3.to_lowercase(), &b.3.to_lowercase()))
        });
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}
