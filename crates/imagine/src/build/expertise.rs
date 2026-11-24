//! Port of `src/Build/Expertise.cs`.
//!
//! Expertise ranks from the client's expertise tree (regular ids 0-37), chain expertises computed with the game's chain
//! formula (expertises.json), and the passive skills they unlock with what each adds.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, RwLock};

use indexmap::IndexMap;
use regex::Regex;
use serde_json::{Value, json};

use crate::Result;
use crate::calc::character_extras;
use crate::calc::effect::rx;
use crate::calc::item_effects::{affinity_cap_label, affinity_name, title};
use crate::context::Context;
use crate::data::game_data::GameData;
use crate::data::json::{self, At, Row};
use crate::data::stat_labels;
use crate::data::store::Store;
use crate::fmt;
use crate::game::progress_reader::{self, ProgressState};
use crate::game::{client_reader, demon_reader};
use crate::memory::{self, GameMemory};
use crate::user_error;

use super::item_icons;

/// The columns every expertise row carries, in order (the page's combat stats).
const COLS: [&str; 16] = [
    "crit", "final_crit", "lb_chance", "lb_power", "pursuit_chance", "pursuit_power", "ta_chance", "ta_power", "dmg_close", "dmg_long", "dmg_spell", "taken_close", "taken_long",
    "taken_spell", "cooldown", "lb_cap",
];

fn correct_to_col(id: &str) -> Option<&'static str> {
    Some(match id {
        "CRITICAL" => "crit",
        "FINAL_CRIT_CHANCE" => "final_crit",
        "LB_CHANCE" => "lb_chance",
        "LB_DAMAGE" => "lb_power",
        "RATE_CLSR" => "dmg_close",
        "RATE_LNGR" => "dmg_long",
        "RATE_SPELL" => "dmg_spell",
        "RATE_CLSR_TAKEN" => "taken_close",
        "RATE_LNGR_TAKEN" => "taken_long",
        "RATE_SPELL_TAKEN" => "taken_spell",
        "COOLDOWN_TIME" => "cooldown",
        _ => return None,
    })
}

fn aspect_to_col(id: &str) -> Option<&'static str> {
    Some(match id {
        "LIMIT_BREAK_MAX" => "lb_cap",
        "PURSUIT_RATE" => "pursuit_chance",
        "PURSUIT_POWER" => "pursuit_power",
        "TECH_ATTACK_RATE" => "ta_chance",
        "TECH_ATTACK_POWER" => "ta_power",
        _ => return None,
    })
}

fn tbl_name(i: i32) -> &'static str {
    match i {
        0 => "STR",
        1 => "MAG",
        2 => "VIT",
        3 => "INT",
        4 => "SPD",
        5 => "LUCK",
        12 => "CLS-R",
        13 => "LNG-R",
        14 => "Spell",
        15 => "Support",
        _ => "",
    }
}

/// The sums per column of one skill row.
#[derive(Default)]
struct Sums([f64; 16]);

impl Sums {
    fn add(&mut self, col: &str, v: f64) {
        if let Some(i) = COLS.iter().position(|c| *c == col) {
            self.0[i] += v;
        }
    }
}

/// `new Row { ... }` from a `json!` object (the members keep their order).
fn obj(v: Value) -> Row {
    match v {
        Value::Object(m) => m,
        _ => Row::new(),
    }
}

fn n(v: f64) -> String {
    fmt::g(v)
}

/// A stat or aspect id as the game names it (StatLabels: the client's own wording).
fn label_of(id: Option<&str>) -> String {
    stat_labels::name_opt(id, 0).unwrap_or_default()
}

/// Percent or flat, the way gear decides it (item_effects::correct_effect): type 1, a skill boost, or a stat the server
/// always treats as a percent whatever the type. Limit Break Chance Up Improvement 1 is "LB_CHANCE:0:5": +5% in the
/// game's own text, so a type-0 LB_CHANCE is a percent too.
fn correct_is_pct(id: &str, typ: i32) -> bool {
    typ == 1 || id.starts_with("BOOST_") || crate::calc::gear_effects::is_pct_correct(id)
}

fn fmt_correct(c: &Value, who: &str) -> String {
    let v = json::num(c.at("value")).unwrap_or(0.0);
    format!(
        "{}{} {}{}{}",
        if who.is_empty() { String::new() } else { format!("{who} ") },
        label_of(json::str(c.at("id")).as_deref()),
        if v >= 0.0 { "+" } else { "" },
        n(v),
        if correct_is_pct(&json::str(c.at("id")).unwrap_or_default(), json::int(c.at("type"))) { "%" } else { "" }
    )
}

fn sign(v: f64) -> &'static str {
    if v >= 0.0 { "+" } else { "" }
}

fn trim_cond(s: &str) -> String {
    s.trim_matches(|c| c == ' ' || c == '(' || c == ')').to_string()
}

// ---------- case-sensitive regexes (System.Text.RegularExpressions.Regex.Match, as ParseDescription used them) ----------

// Shared references, not clones (a cloned Regex starts with an empty search cache; see `rx::r`).
static CS_CACHE: LazyLock<RwLock<HashMap<String, &'static Regex>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

fn re(p: &str) -> &'static Regex {
    if let Some(r) = CS_CACHE.read().expect("regex cache").get(p) {
        return *r;
    }
    let mut cache = CS_CACHE.write().expect("regex cache");
    *cache.entry(p.to_string()).or_insert_with(|| Box::leak(Box::new(Regex::new(p).unwrap_or_else(|e| panic!("bad pattern {p:?}: {e}")))))
}

/// How a description names a stat for the server's stat id ("long range" -> LNGR; with rate: "lng-r" -> RATE_LNGR).
fn desc_id(name: &str, rate: bool) -> Option<String> {
    let id = match name.replace(' ', "-").as_str() {
        "long-range" | "lng-r" | "lngr" => "LNGR",
        "close-range" | "cls-r" | "clsr" => "CLSR",
        "spell" | "magic" => "SPELL",
        "support" => "SUPPORT",
        _ => return None,
    };
    Some(format!("{}{id}", if rate { "RATE_" } else { "" }))
}

fn desc_stat(s: &str) -> &'static str {
    match s {
        "str" => "STR",
        "mag" => "MAG",
        "spd" => "SPD",
        "vit" => "VIT",
        "int" => "INT",
        "lck" => "LUCK",
        "cls-r" => "Close-range damage taken",
        "lng-r" => "Long-range damage taken",
        "spell" => "Spell damage taken",
        _ => "",
    }
}

/// One effect read from a description. `id` is the server's stat id for the character's own effects; partner effects only have a label.
struct DescEffect {
    id: Option<String>,
    label: String,
    value: f64,
    pct: bool,
    partner: bool,
    mitama: bool,
}

fn dv(s: &str) -> f64 {
    fmt::parse_f64(s).unwrap_or(0.0)
}

/// The effects of a skill whose tokusei the server data lacks, read from the game's description of the skill
/// (user's request: the client has no values for these, only this text). The description is cut into items (a
/// bullet, a [header] or an empty line starts one; other lines continue it, as do "- " sub-points). An item that is
/// one of a few plain shapes becomes a number; an item saying what the skill already gave (its own stat table, or
/// its other tokusei) is dropped; headers and footnotes are dropped; everything else - conditions, formulas, procs -
/// is returned as text (Left) and shown as the game says it. An item naming a stat the skill already has with
/// another value stays text too, so nothing is counted twice. None when no item became a number.
/// Checked: Rubito LNG-R 473 = floor(SPD 240 x 0.5 + 9.5) + 344 flat, which needs Deadeye +20 and Locked On +15 from here
/// (438 without them); CLS-R 116, Spell 228 and Support 176 match the same way. Critical and damage rates can't be read back.
fn parse_description(text: &str, already: &[Row]) -> Option<(Vec<DescEffect>, Vec<String>)> {
    if text.is_empty() {
        return None;
    }
    // 1. items
    let mut items: Vec<String> = Vec::new();
    let mut fresh = true;
    for raw_line in text.split("\\n") {
        let line = raw_line.replace('\u{3000}', " ").replace('\u{FFFD}', "・");
        let t = line.trim();
        if t.is_empty() {
            fresh = true;
            continue;
        }
        let bullet = re(r"^\s*(\.\s|[・*•·※])").is_match(&line);
        let header = re(r"^\[[^\]]+\]$").is_match(t);
        if fresh || bullet || header {
            items.push(t.to_string());
        } else if let Some(last) = items.last_mut() {
            last.push(' ');
            last.push_str(t);
        }
        fresh = header;
    }
    // 2. each item: a number, a repeat, or text
    let mut effects: Vec<DescEffect> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    // Has(label, v, out other)
    let has = |label: &str, v: f64| -> (bool, bool) {
        let same: Vec<&Row> =
            already.iter().filter(|r| r.get("target").and_then(Value::as_str) == Some("self") && r.get("label").and_then(Value::as_str).is_some_and(|l| crate::text::eq_ic(l, label))).collect();
        let hit = same.iter().any(|r| (r.get("value").and_then(Value::as_f64).unwrap_or(0.0) - v).abs() < 1e-9);
        (hit, !same.is_empty() && !hit)
    };
    let mut conditional = false; // inside a "When ... :" block: its points only apply then, so they stay text
    for item in &items {
        if re(r"^\[[^\]]+\]$").is_match(item) {
            conditional = false;
            continue;
        }
        if item.starts_with('※') {
            continue;
        }
        let shown = re(r"^[\s.・*•·]+|\s+$").replace_all(item, "").into_owned();
        let mut l = re(r"[\s.]+$").replace_all(&shown.to_lowercase(), "").into_owned();
        l = re(r"^(passive|switch) skill\s*[.:-]?\s*").replace_all(&l, "").into_owned();
        if re(r"^(when|if|while|under|during|at|for) .*:$").is_match(&l) {
            conditional = true;
        }
        if conditional {
            rest.push(shown);
            continue;
        }
        if l.is_empty() {
            continue;
        }
        // for the partner
        if let Some(a) = re(r"^(?:summoned )?partner's ([a-z]+) boost \+(\d+)%?$").captures(&l) {
            let w = &a[1];
            let mut cs = w.chars();
            let first: String = cs.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            effects.push(DescEffect { id: None, label: format!("{first}{} boost", cs.as_str()), value: dv(&a[2]), pct: false, partner: true, mitama: false });
        } else if let Some(b) = re(r"^(?:summoned )?partner's movement speed \+(\d+)%$").captures(&l) {
            effects.push(DescEffect { id: None, label: "Movement speed".into(), value: dv(&b[1]), pct: true, partner: true, mitama: false });
        } else if let Some(c) = re(r"^xp to summoned demon \+(\d+)%$").captures(&l) {
            effects.push(DescEffect { id: None, label: "XP earned".into(), value: dv(&c[1]), pct: true, partner: true, mitama: false });
        } else if let Some(d) = re(r"^experience gained by mitama fused demons \+(\d+)%$").captures(&l) {
            effects.push(DescEffect { id: None, label: "XP earned".into(), value: dv(&d[1]), pct: true, partner: true, mitama: true });
        } else if let Some(e) = re(r"^-(\d+)% (cls-r|lng-r|spell) damage taken by mitama fused demon$").captures(&l) {
            effects.push(DescEffect { id: None, label: desc_stat(&e[2]).into(), value: -dv(&e[1]), pct: true, partner: true, mitama: true });
        } else if let Some(f) = re(r"^((?:str|mag|spd|vit|int|lck)(?:, (?:str|mag|spd|vit|int|lck))*) \+(\d+) to mitama fused demon$").captures(&l) {
            for s in f[1].split(", ") {
                effects.push(DescEffect { id: None, label: desc_stat(s).into(), value: dv(&f[2]), pct: false, partner: true, mitama: true });
            }
        } else if let Some(g) = re(r"^increases limit break cap by (\d+) for your summoned mitama demon$").captures(&l) {
            effects.push(DescEffect { id: None, label: "Limit break cap".into(), value: dv(&g[1]), pct: false, partner: true, mitama: true });
        }
        // the same thing said in words above the numbers ("reduces Close-Range damage taken of your summoned Demon")
        else if re(r"^(reduces|increases) [a-z ,-]+ of your summoned demon( by \d+)?$").is_match(&l) {
        } else {
            // for the character: a plain stat, damage rate, Max HP/MP or critical line
            let mut id: Option<String> = None;
            let mut v = 0.0;
            let mut pct = false;
            if let Some(s1) = re(r"^(?:gains )?\+(\d+) (lng-?r|cls-?r|spell|support)$").captures(&l) {
                id = desc_id(&s1[2], false);
                v = dv(&s1[1]);
            } else if let Some(s2) = re(r"^increases? (long range|close range|spell|support) by (\d+)$").captures(&l) {
                id = desc_id(&s2[1], false);
                v = dv(&s2[2]);
            } else if let Some(s3) = re(r"^(lng-r|long-range|cls-r|close-range|spell|support) damage (?:is increased by |\+)(\d+)%$").captures(&l) {
                id = desc_id(&s3[1], true);
                v = dv(&s3[2]);
                pct = true;
            } else if let Some(s4) = re(r"^(lng-r|long-range|cls-r|close-range|spell|support) damage -(\d+)%$").captures(&l) {
                id = desc_id(&s4[1], true);
                v = -dv(&s4[2]);
                pct = true;
            } else if let Some(s5) = re(r"^-(\d+)% (lng-r|long-range|cls-r|close-range|spell|support) damage$").captures(&l) {
                id = desc_id(&s5[2], true);
                v = -dv(&s5[1]);
                pct = true;
            } else if let Some(s6) = re(r"^max (hp|mp) ([+-]\d+)%$").captures(&l) {
                id = Some(if &s6[1] == "hp" { "HP_MAX" } else { "MP_MAX" }.to_string());
                v = dv(&s6[2]);
                pct = true;
            } else if let Some(s7) = re(r"^crit(?:ical)? \+(\d+)$").captures(&l) {
                id = Some("CRITICAL".to_string());
                v = dv(&s7[1]);
            }
            // "Support and Close-range Damage -200%": a repeat when the skill already has both
            else if let Some(s8) = re(r"^(lng-r|long-range|cls-r|close-range|spell|support) and (lng-r|long-range|cls-r|close-range|spell|support) damage -(\d+)%$").captures(&l).filter(|s8| {
                has(&label_of(desc_id(&s8[1], true).as_deref()), -dv(&s8[3])).0 && has(&label_of(desc_id(&s8[2], true).as_deref()), -dv(&s8[3])).0
            }) {
                let _ = s8;
                continue;
            }
            // any other "X +N" the skill already gave under the same name ("Death Boost +10%" beside Death boost +10)
            else if re(r"^([a-z -]+?) ([+-]\d+)%?$").captures(&l).is_some_and(|s9| has(&s9[1], dv(&s9[2])).0) {
                continue;
            }
            let Some(id) = id else {
                rest.push(shown);
                continue;
            };
            let (hit, other) = has(&label_of(Some(&id)), v);
            if hit || effects.iter().any(|x| !x.partner && x.id.as_deref() == Some(id.as_str())) {
                continue; // already counted
            }
            if other {
                rest.push(shown); // the skill has this stat with another value: don't guess
                continue;
            }
            let label = label_of(Some(&id));
            effects.push(DescEffect { id: Some(id), label, value: v, pct, partner: false, mitama: false });
        }
    }
    if effects.is_empty() { None } else { Some((effects, rest)) }
}

/// The server's tokusei-less passive ids: see `Expertise.Cols`. (Kept as a function so the Simulator can ask the same.)
pub fn cols() -> &'static [&'static str] {
    &COLS
}

/// What one reached expertise rank gives (see `Expertise::granted`).
struct Grant {
    /// class x 10 + rank
    rank: i32,
    /// the skills that count: every skill of a rank with no choice, the learned pick of a rank with one
    skills: Vec<i32>,
    /// a rank with a choice: all the skills it offers (empty when there is none)
    options: Vec<i32>,
    /// a rank with a choice, and the learned skills were never read, so the pick isn't known
    unknown_pick: bool,
}

/// C# `Expertise(store, data)`: it holds the context the store and the data come from.
pub struct Expertise {
    ctx: Arc<Context>,
}

impl Expertise {
    pub fn new(ctx: Arc<Context>) -> Expertise {
        Expertise { ctx }
    }

    fn store(&self) -> &Store {
        &self.ctx.store
    }

    fn data(&self) -> &GameData {
        &self.ctx.data
    }

    pub fn read(&self, name: &str) -> Result<Vec<Row>> {
        let store = self.store();
        let data = self.data();
        let proc = client_reader::find_client(name)?;
        let _shared = memory::share_sweeps(); // points, progress and compendium from one copy of memory
        let mem = GameMemory::new(proc.id())?;
        let pts = client_reader::read_expertise_points(&mem);
        if pts.len() < 30 {
            return Err(user_error("Couldn't find the expertise data in the client's memory (is the character logged in?)."));
        }

        let defs = data.expertises_client()?; // IMAGINE's own names and chain formulas (client data); the website's as fallback
        let mut ranks: Vec<Row> = Vec::new();
        let mut ids: Vec<i32> = pts.keys().copied().collect();
        ids.sort();
        for id in ids {
            if pts[&id] < 10000 {
                continue;
            }
            let ri = pts[&id] / 10000;
            ranks.push(obj(json!({ "id": id, "name": name_of(data, id)?, "class": ri / 10, "rank": ri % 10, "points": pts[&id], "source": "memory" })));
        }
        ranks.extend(Expertise::chain_ranks(data, &pts)?);
        let _ = defs;

        // Digitalization Levels (DemonReader.ReadProgress); kept from the last read when they can't be read now
        let prog = demon_reader::read_progress(&mem);
        let demons = data.demons()?;
        // Which toggle (switch) skills are on, from the game (learned-skill map flag); kept as they were when unreadable
        if let Some(on) = prog.as_ref().and_then(|p| p.switched_on.as_ref()) {
            let mut v: Vec<i32> = on.iter().copied().filter(|id| demons.skills.get(id).is_some_and(|s| s.0 == 2)).collect();
            v.sort();
            store.save_toggles(v)?;
        }
        // Quest-bonus count from the completed-quest map (filled once the game has loaded the quest list); kept otherwise
        // and the compendium count from the Demonic Compendium window's header text, when it has been opened
        {
            let (q, book, v2) = store.counts()?;
            let quest_count = character_extras::quest_bonus_count(prog.as_ref().and_then(|p| p.completed_quests.as_ref()).map(|s| s.iter().copied()))?;
            let book_count = progress_reader::read_compendium_count(&mem);
            if quest_count.is_some() || book_count.is_some() {
                store.save_counts(quest_count.or(q), book_count.or(book), v2, quest_count.is_some(), book_count.is_some())?;
            }
        }
        let existing = || -> Result<Option<Value>> { if store.expertise_ranks_path().exists() { Ok(Some(json::load(store.expertise_ranks_path())?)) } else { Ok(None) } };
        let dlv: Value = match prog.as_ref().and_then(|p| p.levels.as_ref()) {
            None => existing()?.and_then(|v| v.at("digitalize_levels").cloned()).unwrap_or(Value::Null),
            Some(levels) => {
                let mut v: Vec<(i32, i32)> = levels.iter().filter(|(_, v)| **v > 0).map(|(k, v)| (*k, *v)).collect();
                v.sort_by_key(|x| x.0);
                Value::Array(
                    v.into_iter()
                        .map(|(k, level)| {
                            Value::Object(obj(json!({
                                "race_id": k, "race": demons.race_name(k), "level": level,
                                "points": prog.as_ref().and_then(|p| p.points.as_ref()).and_then(|p| p.get(&k).copied()).unwrap_or(0),
                            })))
                        })
                        .collect(),
                )
            }
        };
        // Every other learned passive (items, quests, clan...): learned skills that no expertise rank table and no
        // Digitalization Level gives (those are counted, rank-gated, above). The server counts all learned passives.
        let other: Value = match prog.as_ref().and_then(|p| p.skills.as_ref()) {
            None => existing()?.and_then(|v| v.at("learned_passives").cloned()).unwrap_or(Value::Null),
            Some(skills) => json!(self.other_passives(skills.iter().copied().collect())?),
        };
        // Every learned skill, as the game holds it. Some expertise ranks offer several skills and let the character learn
        // only one of them (the server's LearnSkill), so which one it learned is what says which one counts (`granted`).
        // Kept from the last read when the learned skills can't be read now.
        let learned_all: Value = match prog.as_ref().and_then(|p| p.skills.as_ref()) {
            None => existing()?.and_then(|v| v.at("learned_skills").cloned()).unwrap_or(Value::Null),
            Some(skills) => {
                let mut v: Vec<i32> = skills.iter().copied().collect();
                v.sort();
                json!(v)
            }
        };
        // Expertise points spent (the regular expertises only; chains are computed from them and cost nothing) and the
        // cap: CharacterManager::GetMaxExpertisePoints = 1,700,000 + floor(level / 10) x 100,000 (+100,000 at level 99)
        // + ExpertiseExtension x 100,000; extension -1 = no cap. The game shows points / 100 ("52000.00pt").
        let total: i64 = pts.iter().filter(|(k, _)| **k <= 37).map(|(_, v)| (*v).max(0) as i64).sum();
        let points = match prog.as_ref().and_then(|p| p.level.zip(p.expertise_extension)) {
            Some((lvl, ext)) => {
                let cap: i64 = (1_700_000 + lvl / 10 * 100_000 + if lvl == 99 { 100_000 } else { 0 }) as i64 + (ext.max(0) as i64) * 100_000;
                obj(json!({ "total": total, "cap": if ext < 0 { Value::Null } else { json!(cap) }, "level": lvl, "extension": ext }))
            }
            None => obj(json!({ "total": total, "cap": Value::Null })),
        };
        json::save(
            store.expertise_ranks_path(),
            &json!({
                "source": format!("Client memory, {}", chrono::Local::now().format("%Y-%m-%d %H:%M")),
                "points": points,
                "ranks": ranks,
                "digitalize_levels": dlv,
                "learned_passives": other,
                "learned_skills": learned_all,
            }),
        )?;
        Ok(ranks)
    }

    /// The chain expertises a set of regular expertise points gives (id -> points, 10000 per rank): computed with the
    /// server's formula and IMAGINE's chain definitions (the website's when the reference file is missing).
    /// Checked: Ashes and Dust 9-4, Curse of the Wretched 1-5, Sharpshooter 8-4, as the game shows for Rubito.
    pub fn chain_ranks(data: &GameData, pts: &IndexMap<i32, i32>) -> Result<Vec<Row>> {
        let ex = data.expertises()?;
        let defs = data.expertises_client()?;
        let mut ranks = Vec::new();
        let chains: Vec<(i32, Vec<(i32, i32, f64)>)> = if !defs.is_empty() {
            defs.values().filter(|d| d.is_chain).map(|d| (d.id, d.chain.clone())).collect()
        } else {
            let mut v = Vec::new();
            if let Some(m) = ex.as_object() {
                for (k, e) in m {
                    if json::truthy(e.at("isChain")) && e.at("chain").is_some_and(Value::is_array) {
                        let id = k.parse::<i32>().map_err(|_| anyhow::anyhow!("The input string '{k}' was not in a correct format."))?;
                        v.push((id, json::arr(e.at("chain")).map(|l| (json::int(l.idx(0)), json::int(l.idx(1)), json::num(l.idx(2)).unwrap_or(0.0))).collect()));
                    }
                }
            }
            v
        };
        for (id, chain) in chains {
            let mut sum: i64 = 0;
            let mut ok = !chain.is_empty();
            for (src, req, pct) in &chain {
                let v = pts.get(src).copied().unwrap_or(0);
                if v / 10000 < *req {
                    ok = false;
                }
                if *pct > 0.0 {
                    sum += (v as f64 * pct).trunc() as i64;
                }
            }
            if !ok || sum < 10000 {
                continue;
            }
            let ri = (sum / 10000) as i32;
            ranks.push(obj(json!({ "id": id, "name": name_of(data, id)?, "class": ri / 10, "rank": ri % 10, "points": sum as i32, "chain": true, "source": "computed" })));
        }
        Ok(ranks)
    }

    /// Learned passive skills with an effect that no expertise rank and no Digitalization Level gives (Additional HP/MP,
    /// affinity "Additional" skills, Quickness, clan skills...): those are counted by Build's own sections, rank-gated.
    fn other_passives(&self, learned: Vec<i32>) -> Result<Vec<i32>> {
        let data = self.data();
        let mut from_expertise: HashSet<i32> = HashSet::new();
        for l in data.rank_skills_client()?.values() {
            for (_, skills) in l {
                from_expertise.extend(skills.iter().copied());
            }
        }
        if let Some(m) = data.rank_skills()?.as_object() {
            for arr in m.values() {
                for en in json::arr(Some(arr)) {
                    for s in json::arr(en.at("skills")) {
                        from_expertise.insert(json::int(Some(s)));
                    }
                }
            }
        }
        let from_levels: HashSet<i32> = data.demons()?.levels.values().flat_map(|l| l.iter().map(|x| x.extend_skill_id)).collect();
        let passives = data.passives()?;
        let mut res: Vec<i32> = learned
            .into_iter()
            .filter(|id| {
                !from_expertise.contains(id)
                    && !from_levels.contains(id)
                    && passives.get(&id.to_string()).filter(|v| !v.is_null()).is_some_and(|p| json::int(p.at("mainCategory")) == 0)
            })
            .collect();
        res.sort();
        Ok(res)
    }

    // ---------- Skill icons for the page ----------

    /// The icon of a skill for the page: its name, with the picture saved as dash\skill-icons\<icon>.png, or None when
    /// there is none. One mechanism for every page (Expertise, Digitalize, Bonuses): `item_icons::skill_asset`.
    fn skill_icon(&self, sid: i32) -> Result<Option<String>> {
        item_icons::skill_asset(self.store(), self.data(), sid, 0)
    }

    /// Skills per rank of an expertise: IMAGINE's own table (client ExpertClassData), the website's when that's
    /// missing; None when neither has it. Rank = class x 10 + rank.
    fn rank_entries(&self, id: i32) -> Result<Option<Vec<(i32, Vec<i32>)>>> {
        let data = self.data();
        if let Some(client_ranks) = data.rank_skills_client()?.get(&id) {
            return Ok(Some(client_ranks.clone()));
        }
        Ok(data
            .rank_skills()?
            .get(id.to_string())
            .filter(|v| v.is_array())
            .map(|js| json::arr(Some(js)).map(|en| (json::int(en.at("rank")), json::arr(en.at("skills")).map(|s| json::int(Some(s))).collect())).collect()))
    }

    /// The skills an expertise at rank `rank_idx` (class x 10 + rank) gives the character, rank by rank.
    ///
    /// A rank lists the skills it can unlock and how many the character may learn (ExpertClassData skillCount; the
    /// server's LearnSkill refuses the rest). Where it lists more than that (Pursuit class 1: Strength, Magic,
    /// Intelligence or Speed) the character holds only its pick, and only a skill it has learned applies. `learned` is
    /// the learned skills as last read (None = never read): without it the pick is not known, so the rank gives nothing
    /// and says so (`Grant::unknown_pick`), instead of giving every option at once.
    fn granted(&self, id: i32, rank_idx: i32, learned: Option<&HashSet<i32>>) -> Result<Vec<Grant>> {
        let mut res = Vec::new();
        for (en_rank, skills) in self.rank_entries(id)?.unwrap_or_default() {
            if en_rank > rank_idx {
                continue;
            }
            match self.data().rank_pick_count(id, en_rank)? {
                None => res.push(Grant { rank: en_rank, skills, options: Vec::new(), unknown_pick: false }),
                Some(_) => {
                    let picked: Vec<i32> = skills.iter().copied().filter(|s| learned.is_some_and(|l| l.contains(s))).collect();
                    res.push(Grant { rank: en_rank, skills: picked, options: skills, unknown_pick: learned.is_none() });
                }
            }
        }
        Ok(res)
    }

    /// The learned skills saved by `read` (expertise-ranks.json "learned_skills"); None when none were read yet.
    fn learned_of(saved: Option<&Value>) -> Option<HashSet<i32>> {
        match saved.and_then(|v| v.at("learned_skills")) {
            Some(Value::Array(a)) => Some(a.iter().map(|s| json::int(Some(s))).collect()),
            _ => None,
        }
    }

    /// For the Overview's Expertise card and the Expertise page summary: every skill the reached ranks of each expertise (chains and regular) give, actives too (the
    /// passive rows of expertise.json leave actives out, as they add no stats). Written into expertise-ranks.json as
    /// "skills" [{id, name, icon, rank, active}] on each entry, so `rebuild` fills it without the game.
    /// Active = not in the passive skill table (data.Passives). Also writes "max_class" on every entry.
    fn save_chain_skills(&self) -> Result<()> {
        let data = self.data();
        let mut doc = json::load(self.store().expertise_ranks_path())?;
        let learned = Expertise::learned_of(Some(&doc));
        let mut changed = false;
        let skill_names = data.skill_names()?;
        let passives = data.passives()?;
        if let Some(Value::Array(entries)) = doc.get_mut("ranks") {
            for e in entries.iter_mut() {
                if e.is_null() {
                    continue;
                }
                // every entry: the highest class the game lets it reach (client ExpertClassData maxClass; 0 = not known),
                // for the Expertise page's summary (Class <max> Rank 0 = maxed)
                if let Some(eo) = e.as_object_mut() {
                    if eo.get("id").is_some_and(|v| !v.is_null()) {
                        if let Some(def) = data.expertises_client()?.get(&json::int(eo.get("id"))) {
                            if def.max_class > 0 && json::int(eo.get("max_class")) != def.max_class {
                                eo.insert("max_class".into(), json!(def.max_class));
                                changed = true;
                            }
                        }
                    }
                } else {
                    continue;
                }
                let Some(o) = e.as_object_mut() else { continue };
                if o.get("id").is_none_or(Value::is_null) {
                    continue;
                }
                let rank_idx = json::int(o.get("class")).wrapping_mul(10).wrapping_add(json::int(o.get("rank")));
                let mut list: Vec<Value> = Vec::new();
                for Grant { rank: en_rank, skills, .. } in self.granted(json::int(o.get("id")), rank_idx, learned.as_ref())? {
                    for sid in skills {
                        list.push(json!({
                            "id": sid,
                            "name": skill_names.get(&sid).cloned().flatten().unwrap_or_else(|| format!("Skill {sid}")),
                            "icon": self.skill_icon(sid)?,
                            "rank": format!("Class {} Rank {}", en_rank / 10, en_rank % 10),
                            "active": passives.get(&sid.to_string()).is_none_or(Value::is_null),
                        }));
                    }
                }
                let list = Value::Array(list);
                if o.get("skills").is_some_and(|s| json::serialize(s, json::Escape::Default, false) == json::serialize(&list, json::Escape::Default, false)) {
                    continue;
                }
                o.insert("skills".into(), list);
                changed = true;
            }
        }
        if changed {
            json::save(self.store().expertise_ranks_path(), &doc)?;
        }
        Ok(())
    }

    /// dash\expertise.json: one row per unlocked passive skill, with the combat stats it adds and structured effects.
    pub fn build(&self, latest_snapshot: Option<&Value>) -> Result<Vec<Row>> {
        let store = self.store();
        let rk: Vec<Value> = json::arr(json::load(store.expertise_ranks_path())?.at("ranks")).cloned().collect();
        self.save_chain_skills()?;
        let rows = self.rows(latest_snapshot, &rk, true)?;
        json::save(store.dash_dir().join("expertise.json"), &Value::Array(rows.iter().cloned().map(Value::Object).collect()))?;
        Ok(rows)
    }

    /// The rows of [`Expertise::build`] for any set of ranks (entries like expertise-ranks.json "ranks": id, name, class,
    /// rank), without writing anything: the simulator asks "what would these ranks give". Everything else (other
    /// learned skills, toggles, quest and compendium counts, Digitalization Levels) is the character's own, as saved.
    /// icons = false leaves the skill pictures alone (making one writes a file).
    pub fn rows(&self, latest_snapshot: Option<&Value>, rk: &[Value], icons: bool) -> Result<Vec<Row>> {
        let store = self.store();
        let data = self.data();
        let s = |k: &str| json::num(latest_snapshot.at(k)).unwrap_or(0.0);
        let stat_by_tbl: HashMap<i32, f64> = HashMap::from([
            (0, s("str")),
            (1, s("mag")),
            (2, s("vit")),
            (3, s("int")),
            (4, s("spd")),
            (5, s("luck")),
            (12, s("clsr")),
            (13, s("lngr")),
            (14, s("spell")),
            (15, s("support")),
        ]);
        let active_toggles: HashSet<i32> = store.active_toggles()?.into_iter().collect(); // toggle skills the person marked as on
        let sctx = SkillCtx { stat_by_tbl, active_toggles, icons };

        // (no ranks file: a simulator base with no character behind it, which has learned nothing)
        let saved: Option<Value> = if store.expertise_ranks_path().exists() { Some(json::load(store.expertise_ranks_path())?) } else { None };
        let learned = Expertise::learned_of(saved.as_ref());

        let mut rows: Vec<Row> = Vec::new();
        for e in rk {
            let cls = json::int(e.at("class"));
            let rank = json::int(e.at("rank"));
            let rank_idx = cls * 10 + rank;
            let name = json::str(e.at("name"));
            if json::truthy(e.at("unknown")) || e.at("id").is_none() {
                let mut o = obj(json!({
                    "row": format!("exp-unknown-{}", rx::rep(name.as_deref().unwrap_or(""), "[^A-Za-z0-9]+", "-").to_lowercase()),
                    "expertise": name, "rank": format!("Class {cls} Rank {rank}"), "skill": "",
                    "effects": "This IMAGINE expertise isn't in the server files I have, so its passives aren't counted.", "counted": false,
                }));
                for c in COLS {
                    o.insert(c.into(), json!(0));
                }
                rows.push(o);
                continue;
            }
            let id = json::int(e.at("id"));
            for g in self.granted(id, rank_idx, learned.as_ref())? {
                let rank_text = format!("Class {} Rank {}", g.rank / 10, g.rank % 10);
                for sid in g.skills {
                    self.add_skill(&sctx, &mut rows, sid, &format!("exp-{id}-{sid}"), name.as_deref(), &rank_text)?;
                }
                // A rank with a choice and no learned pick (never read, or not picked yet): one line that says so, when
                // any of the options would have added something
                if g.unknown_pick || (!g.options.is_empty() && !g.options.iter().any(|s| learned.as_ref().is_some_and(|l| l.contains(s)))) {
                    let passives = data.passives()?;
                    if g.options.iter().any(|s| passives.get(&s.to_string()).is_some_and(|p| !p.is_null())) {
                        let names = data.skill_names()?;
                        let list: Vec<String> = g.options.iter().map(|s| names.get(s).cloned().flatten().unwrap_or_else(|| "a skill the game data has no name for".to_string())).collect();
                        let mut o = obj(json!({
                            "row": format!("exp-{id}-pick-{}", g.rank), "expertise": name, "rank": rank_text, "skill": "",
                            "effects": if g.unknown_pick {
                                format!("Learn one of: {}. Which one you learned isn't known yet, so none of them is counted. Take a snapshot with the game running.", list.join(", "))
                            } else {
                                format!("Nothing learned here yet. Learn one of: {}. Only the one you learn counts.", list.join(", "))
                            },
                            "counted": false,
                        }));
                        for c in COLS {
                            o.insert(c.into(), json!(0));
                        }
                        rows.push(o);
                    }
                }
            }
        }
        for sid_node in json::arr(saved.as_ref().and_then(|v| v.at("learned_passives"))) {
            self.add_skill(&sctx, &mut rows, json::int(Some(sid_node)), &format!("learn-{}", json::int(Some(sid_node))), Some("Other learned skills"), "")?;
        }
        // Quest bonus and Demonic Compendium (Calc\CharacterExtras, the server's QUEST_BONUS / DEMON_BOOK_* rules). The
        // counts aren't in the client's memory, so they come from the profile (`extras` command). Clan bonuses are the
        // learned clan skills 7001-7024, already counted above; titles give nothing.
        let (quest_count, book_count, book_v2) = store.counts()?;
        let state = ProgressState { quest_bonus_count: quest_count, compendium_count: book_count, has_compendium_v2: book_v2, ..Default::default() };
        let extras = character_extras::compute(data, None, Some(&state), false)?;
        let mut extra_idx = 0;
        // GroupBy (Source, Item): first-appearance order
        let mut groups: Vec<((String, String), Vec<&crate::calc::effect::Effect>)> = Vec::new();
        for e in &extras {
            let key = (e.source.clone(), e.item.clone());
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.1.push(e),
                None => groups.push((key, vec![e])),
            }
        }
        for (key, g) in &groups {
            let group = if key.0 == "quest" {
                format!("Quest bonus{}", quest_count.map(|q| format!(" ({q} quests)")).unwrap_or_default())
            } else {
                format!("Demonic Compendium{}", book_count.map(|b| format!(" ({b} entries)")).unwrap_or_default())
            };
            let mut o = obj(json!({ "row": format!("{}-{}", key.0, extra_idx), "expertise": group, "rank": "", "skill": rx_item(&key.1) }));
            extra_idx += 1;
            let mut fx: Vec<String> = Vec::new();
            let mut sfx: Vec<Row> = Vec::new();
            for e in g {
                let Some(v) = e.value.filter(|_| e.counted() || e.cond == "situational") else {
                    fx.push(if e.cond == "unparsed" && e.missing_data {
                        e.label.clone()
                            + if key.0 == "quest" {
                                ". Open the Quest Completion Bonuses window in game once, then take a snapshot."
                            } else {
                                ". Open the Demonic Compendium window in game once, then take a snapshot."
                            }
                    } else {
                        e.label.clone()
                    });
                    continue;
                };
                let who = if e.target == "partner" { "Partner's " } else { "" };
                let note = e.cond_note.as_str();
                fx.push(format!(
                    "{who}{} {}{}{}{}",
                    e.label,
                    sign(v),
                    n(v),
                    if e.pct { "%" } else { "" },
                    if note.is_empty() { String::new() } else { format!(" ({note})") }
                ));
                sfx.push(obj(json!({
                    "label": e.label, "value": v, "pct": e.pct, "target": if e.target == "partner" { "partner" } else { "self" },
                    "cond": if e.cond == "situational" { "situational" } else { "always" }, "note": if e.cond == "situational" { note } else { "" },
                })));
            }
            for c in COLS {
                o.insert(c.into(), json!(0.0));
            }
            o.insert("effects".into(), json!(fx.join("; ")));
            o.insert("toggle".into(), json!(false));
            o.insert("toggle_on".into(), json!(false));
            let counted = !sfx.is_empty();
            o.insert("fx".into(), Value::Array(sfx.into_iter().map(Value::Object).collect()));
            o.insert("counted".into(), json!(counted));
            rows.push(o);
        }
        // Digitalization Levels: each level of a race can teach a passive "extend" skill (GuardianLevelData), learned for
        // good when the level is reached, so they count like expertise passives. Levels are saved by Read().
        let tables = data.demons()?;
        for dl in json::arr(saved.as_ref().and_then(|v| v.at("digitalize_levels"))) {
            let race = json::int(dl.at("race_id"));
            let level = json::int(dl.at("level"));
            let Some(lv) = tables.levels.get(&race) else { continue };
            let group = format!("Digitalize: {}", title(&tables.race_name(race)));
            for l in lv.iter().filter(|l| l.level >= 1 && l.level <= level && l.extend_skill_id > 0) {
                self.add_skill(&sctx, &mut rows, l.extend_skill_id, &format!("dlv-{race}-{}", l.extend_skill_id), Some(&group), &format!("DLv {}", l.level))?;
            }
        }
        Ok(rows)
    }

    /// One passive skill as a row: what it adds (combat columns, effect text, structured effects).
    fn add_skill(&self, c: &SkillCtx, rows: &mut Vec<Row>, sid: i32, row_id: &str, group_name: Option<&str>, rank_text: &str) -> Result<()> {
        let data = self.data();
        let Some(p) = data.passives()?.get(&sid.to_string()).filter(|v| !v.is_null()) else { return Ok(()) }; // active skills add no stats
        let skill_name = data.skill_names()?.get(&sid).cloned().flatten();
        let mut o = obj(json!({
            "row": row_id, "expertise": group_name, "rank": rank_text, "skill": skill_name.clone().unwrap_or_else(|| format!("Skill {sid}")), "skill_id": sid,
            "icon": if c.icons { self.skill_icon(sid)? } else { None },
        }));
        let mut sums = Sums::default();
        let mut fx: Vec<String> = Vec::new();
        let mut sfx: Vec<Row> = Vec::new();
        let mut unknown_tok: Vec<i32> = Vec::new();
        for cc in json::arr(p.at("correctTbl")) {
            fx.push(fmt_correct(cc, ""));
            let cid = json::str(cc.at("id"));
            let v = json::num(cc.at("value")).unwrap_or(0.0);
            sfx.push(obj(json!({ "label": label_of(cid.as_deref()), "value": v, "pct": correct_is_pct(cid.as_deref().unwrap_or(""), json::int(cc.at("type"))), "target": "self", "cond": "always" })));
            if let Some(col) = cid.as_deref().and_then(correct_to_col) {
                sums.add(col, v);
            }
        }
        for tn in json::arr(p.at("tokuseiIds")) {
            let t = json::int(Some(tn));
            if t == 0 {
                continue;
            }
            let d = data.tokusei()?.get(t.to_string()).filter(|v| !v.is_null());
            if let Some(d) = d {
                let who = match json::str(d.at("target")).as_deref() {
                    Some("PARTNER") => "Partner's",
                    Some("PARTY") => "Party's",
                    _ => "",
                };
                let cond = if json::truthy(d.at("conditions")) {
                    " (only with certain partners)"
                } else if json::truthy(d.at("skill")) {
                    " (certain skills)"
                } else {
                    ""
                };
                for cc in json::arr(d.at("correct")) {
                    fx.push(fmt_correct(cc, who) + cond);
                    let cid = json::str(cc.at("id"));
                    let v = json::num(cc.at("value")).unwrap_or(0.0);
                    let pct = correct_is_pct(cid.as_deref().unwrap_or(""), json::int(cc.at("type")));
                    sfx.push(obj(json!({
                        "label": label_of(cid.as_deref()), "value": v, "pct": pct, "target": if who.is_empty() { "self" } else { "partner" },
                        "cond": if cond.is_empty() { "always" } else { "situational" }, "note": trim_cond(cond),
                    })));
                    // The party is the character and the partner: a party effect counts for both
                    if who == "Party's" {
                        sfx.push(obj(json!({
                            "label": label_of(cid.as_deref()), "value": v, "pct": pct, "target": "self",
                            "cond": if cond.is_empty() { "always" } else { "situational" }, "note": trim_cond(cond),
                        })));
                    }
                    if (who.is_empty() || who == "Party's") && cond.is_empty() {
                        if let Some(col) = cid.as_deref().and_then(correct_to_col) {
                            sums.add(col, v);
                        }
                    }
                }
                for a in json::arr(d.at("aspects")) {
                    let typ = json::str(a.at("type"));
                    let mut v = json::num(a.at("value")).unwrap_or(0.0);
                    let attr = a.at("attr");
                    let scaled = attr.is_some() && json::str(attr.at("mult")).as_deref() == Some("CORRECT_TABLE");
                    let mut tbl = 0;
                    let mut factor = 0.0;
                    if scaled {
                        let prec = json::int(attr.at("precision"));
                        tbl = json::int(attr.at("tbl_index"));
                        tbl = match json::str(attr.at("tbl")).as_deref() {
                            Some("STR") => 0,
                            Some("MAGIC") => 1,
                            Some("VIT") => 2,
                            Some("INT") => 3,
                            Some("SPEED") => 4,
                            Some("LUCK") => 5,
                            Some("CLSR") => 12,
                            Some("LNGR") => 13,
                            Some("SPELL") => 14,
                            Some("SUPPORT") => 15,
                            _ => tbl,
                        };
                        factor = v / 10f64.powf(prec as f64);
                        v = fmt::round(factor * c.stat_by_tbl.get(&tbl).copied().unwrap_or(0.0), 2);
                    }
                    if typ.as_deref() == Some("SKILL_ADD") {
                        // grants another skill (the value is its id), not a number
                        fx.push(format!("Adds skill {}", data.skill_names()?.get(&(v as i32)).cloned().flatten().unwrap_or_else(|| n(v))));
                        continue;
                    }
                    if typ.as_deref() == Some("AFFINITY_CAP_MAX") {
                        // the value is the element; the amount (Modifier) is only in the server XML
                        let modifier = data
                            .tokusei_full(t)?
                            .and_then(|f| f.aspects.iter().find(|x| x.id.as_deref() == typ.as_deref() && x.value as i32 == v as i32).and_then(|x| x.modifier));
                        add_cap(&mut fx, &mut sfx, v as i32, modifier, if who.is_empty() { "self" } else { "partner" }, "always", "", false);
                        continue;
                    }
                    let nice = label_of(typ.as_deref());
                    let unit = if typ.as_deref() == Some("LIMIT_BREAK_MAX") { "" } else { "%" };
                    let who_text = if who.is_empty() { String::new() } else { format!("{who} ") };
                    fx.push(if scaled {
                        format!("{who_text}{nice} +{}{unit} ({} x {})", n(v), n(factor), tbl_name(tbl))
                    } else {
                        format!("{who_text}{nice} +{}{unit}", n(v))
                    });
                    if who.is_empty() {
                        if let Some(col) = typ.as_deref().and_then(aspect_to_col) {
                            sums.add(col, v);
                        }
                    }
                    sfx.push(obj(json!({
                        "label": nice, "value": v, "pct": typ.as_deref() != Some("LIMIT_BREAK_MAX"), "target": if who.is_empty() { "self" } else { "partner" }, "cond": "always",
                    })));
                }
            } else {
                let raw = data.raw_tokusei_types(t)?;
                let skill_name_s = skill_name.clone().unwrap_or_default();
                if let Some(am) = raw.as_deref().and_then(|r| rx::m(r, r"PURSUIT_AFFINITY_OVERRIDE=(\d+)")) {
                    // The skill name already says the element ("Mantra of Pursuit: Mind"); prefer it over a guessed code.
                    let el = match rx::m(&skill_name_s, r":\s*(.+)$") {
                        Some(sm) => Some(sm[1].to_string()),
                        None => affinity_name(fmt::parse_i32(&am[1]).ok_or_else(|| anyhow::anyhow!("The input string '{}' was not in a correct format.", &am[1]))?),
                    };
                    fx.push(match el {
                        Some(el) => format!("Pursuit hits become {el}"),
                        None => "Changes pursuit's element".to_string(),
                    });
                } else if raw.as_deref().is_some_and(|r| r.starts_with("TALK")) {
                    fx.push("Better demon negotiation".into());
                } else {
                    let full = data
                        .tokusei_full(t)?
                        .filter(|f| f.correct.len() + f.aspects.len() > 0 && f.correct.iter().chain(f.aspects.iter()).all(|v| v.mult.is_empty() || v.mult == "NONE"));
                    if let Some(full) = full {
                        // Not in the website data: the server XML, else the client's own text for it (GameData.ClientTokusei)
                        let who2 = match full.target.as_str() {
                            "PARTNER" => "Partner's",
                            "PARTY" => "Party's",
                            "SUMMONER" => "Summoner's",
                            _ => "",
                        };
                        let actions: Vec<i32> = full.skill_conditions.iter().filter(|s| s.0.as_deref() == Some("ACTION_TYPE")).map(|s| s.1).collect();
                        let situational = !full.conditions.is_empty() || full.target_conditions || full.skill_conditions.iter().any(|s| s.0.as_deref() != Some("ACTION_TYPE"));
                        let src = if full.from_client_text { " (from the client's text)" } else { "" };
                        let who2_text = if who2.is_empty() { String::new() } else { format!("{who2} ") };
                        for cc in &full.correct {
                            let v = if cc.precision > 0 { cc.value / 10f64.powf(cc.precision as f64) } else { cc.value };
                            let cid = cc.id.as_deref().unwrap_or("");
                            // Percent the same way gear decides it (item_effects::correct_effect): type 1, a skill
                            // boost, or a stat the server always treats as a percent whatever the type (LB_CHANCE...).
                            // Limit Break Chance Up Improvement 1 is "LB_CHANCE:0:5", +5% in the game's own text.
                            let pct_fx = correct_is_pct(cid, cc.r#type);
                            let pct = pct_fx || cid.starts_with("RATE_");
                            fx.push(format!("{who2_text}{} {}{}{}{src}", label_of(cc.id.as_deref()), sign(v), n(v), if pct { "%" } else { "" }));
                            sfx.push(obj(json!({
                                "label": label_of(cc.id.as_deref()), "value": v, "pct": pct_fx, "target": if who2.is_empty() { "self" } else { "partner" },
                                "cond": if situational { "situational" } else { "always" }, "note": trim_cond(src),
                            })));
                            // The party is the character and the partner: a party effect counts for both
                            if who2 == "Party's" {
                                sfx.push(obj(json!({
                                    "label": label_of(cc.id.as_deref()), "value": v, "pct": pct_fx, "target": "self",
                                    "cond": if situational { "situational" } else { "always" }, "note": trim_cond(src),
                                })));
                            }
                            if (who2.is_empty() || who2 == "Party's") && !situational {
                                if let Some(col) = correct_to_col(cid) {
                                    sums.add(col, v);
                                }
                            }
                        }
                        for a in &full.aspects {
                            let v = if a.precision > 0 { a.value / 10f64.powf(a.precision as f64) } else { a.value };
                            let aid = a.id.as_deref().unwrap_or("");
                            const ACT: [&str; 12] = ["Attack", "Spin", "Rush", "Shot", "Rapid", "Guard", "Counter", "Dodge", "Talk", "Intimidate", "Taunt", "Support"];
                            let nice = match aid {
                                "EFFECT_POWER" if actions.len() == 1 && (actions[0] as usize) < ACT.len() => format!("{} damage", ACT[actions[0] as usize]),
                                "EFFECT_POWER" => "Skill power".to_string(),
                                "DAMAGE_DEALT" => stat_labels::of(aid).map(str::to_string).unwrap_or_else(|| "Damage dealt".to_string()),
                                _ => label_of(a.id.as_deref()),
                            };
                            if aid == "SKILL_ADD" {
                                // grants another skill (the value is its id), not a number
                                fx.push(format!("Adds skill {}{src}", data.skill_names()?.get(&(a.value as i32)).cloned().flatten().unwrap_or_else(|| n(a.value))));
                                continue;
                            }
                            if aid == "AFFINITY_CAP_MAX" {
                                // Value = the element, Modifier = the amount in %
                                add_cap(&mut fx, &mut sfx, a.value as i32, a.modifier, if who2.is_empty() { "self" } else { "partner" }, if situational { "situational" } else { "always" }, &trim_cond(src), false);
                                if who2 == "Party's" {
                                    add_cap(&mut fx, &mut sfx, a.value as i32, a.modifier, "self", if situational { "situational" } else { "always" }, &trim_cond(src), true);
                                }
                                continue;
                            }
                            let pct = aid != "LIMIT_BREAK_MAX";
                            let sit = situational || (aid == "EFFECT_POWER" && actions.len() != 1);
                            fx.push(format!("{who2_text}{nice} {}{}{}{src}", sign(v), n(v), if pct { "%" } else { "" }));
                            sfx.push(obj(json!({
                                "label": nice, "value": v, "pct": pct, "target": if who2.is_empty() { "self" } else { "partner" },
                                "cond": if sit { "situational" } else { "always" }, "note": trim_cond(src),
                            })));
                            if who2.is_empty() && !sit {
                                if let Some(col) = aspect_to_col(aid) {
                                    sums.add(col, v);
                                }
                            }
                        }
                    } else if raw.as_deref().is_some_and(|r| !r.is_empty()) {
                        fx.push(raw.clone().unwrap_or_default());
                    } else {
                        unknown_tok.push(t);
                    }
                }
            }
        }
        // Effects neither the server data nor a parsable client line covers: the game's own text (one per skill)
        if !unknown_tok.is_empty() {
            let mut text: Option<String> = None;
            for x in &unknown_tok {
                if let Some(t) = data.client_tokusei_text(*x)?.filter(|t| !t.is_empty()) {
                    text = Some(t);
                    break;
                }
            }
            // Plain "X +N" lines of the description are read as numbers; the rest is shown as the game says it
            let says = |s: &str| -> String {
                let cleaned = s.replace("\\n", " ").replace('\u{FFFD}', "·");
                format!("Not in the server data; the game says: {}", re(r"\s+").replace_all(&cleaned, " ").trim())
            };
            match text.as_deref().and_then(|t| parse_description(t, &sfx)) {
                Some((effects, left)) => {
                    for e in &effects {
                        let note = format!("{}from the game's description", if e.mitama { "Mitama-fused partner only; " } else { "" });
                        fx.push(format!("{}{} {}{}{} ({note})", if e.partner { "Partner's " } else { "" }, e.label, sign(e.value), n(e.value), if e.pct { "%" } else { "" }));
                        sfx.push(obj(json!({
                            "label": e.label, "value": e.value, "pct": e.pct, "target": if e.partner { "partner" } else { "self" },
                            "cond": if e.mitama { "situational" } else { "always" }, "note": if e.mitama { "Mitama-fused partner only" } else { "from the game's description" },
                        })));
                        if !e.partner {
                            if let Some(dcol) = e.id.as_deref().and_then(correct_to_col) {
                                sums.add(dcol, e.value);
                            }
                        }
                    }
                    if !left.is_empty() {
                        fx.push(says(&left.join(" · ")));
                    }
                }
                None => fx.push(match &text {
                    Some(t) => says(t),
                    None => format!("special effect (tokusei {})", unknown_tok.iter().map(i32::to_string).collect::<Vec<_>>().join(", ")),
                }),
            }
        }
        // A switch (toggle) skill only applies while it's on (TokuseiManager / ApplySkillCorrectTbls; the on/off state
        // is read from the game's learned-skill map): a switched-off one is listed, not counted.
        let is_switch = json::int(p.at("mainCategory")) == 2;
        let toggle = is_switch && !c.active_toggles.contains(&sid);
        for (i, col) in COLS.iter().enumerate() {
            o.insert((*col).into(), json!(if toggle { 0.0 } else { sums.0[i] }));
        }
        let mut seen: HashSet<&str> = HashSet::new();
        let distinct: Vec<&str> = fx.iter().map(String::as_str).filter(|s| seen.insert(s)).collect();
        o.insert(
            "effects".into(),
            json!(format!(
                "{}{}",
                if toggle {
                    "Toggle skill, switched off (not counted): "
                } else if is_switch {
                    "Toggle skill, switched on: "
                } else {
                    ""
                },
                distinct.join("; ")
            )),
        );
        o.insert("toggle".into(), json!(is_switch));
        o.insert("toggle_on".into(), json!(is_switch && !toggle));
        o.insert("fx".into(), Value::Array(if toggle { Vec::new() } else { sfx.into_iter().map(Value::Object).collect() }));
        o.insert("counted".into(), json!(!toggle));
        rows.push(o);
        Ok(())
    }
}

/// What `Rows` needs besides the data: the stats the scaled aspects use, the toggles that are on, whether to make pictures.
struct SkillCtx {
    stat_by_tbl: HashMap<i32, f64>,
    active_toggles: HashSet<i32>,
    icons: bool,
}

/// One element's boost cap raised (AFFINITY_CAP_MAX, see ItemEffects.AffinityCapLabel); quiet = the row only
/// (a party effect's second, self copy: the text already says it).
#[allow(clippy::too_many_arguments)]
fn add_cap(fx: &mut Vec<String>, sfx: &mut Vec<Row>, element: i32, amount: Option<f64>, target: &str, cond: &str, note: &str, quiet: bool) {
    let label = affinity_cap_label(element);
    let who_text = if target == "partner" { "Partner's " } else { "" };
    let Some(m) = amount else {
        if !quiet {
            fx.push(format!("{who_text}{label} raised (amount not in the data)"));
        }
        return;
    };
    if !quiet {
        fx.push(format!("{who_text}{label} {}{}%{}", sign(m), n(m), if note.is_empty() { String::new() } else { format!(" ({note})") }));
    }
    sfx.push(obj(json!({ "label": label, "value": m, "pct": true, "target": target, "cond": cond, "note": note })));
}

/// NameOf(id): IMAGINE's own name, else the website's.
fn name_of(data: &GameData, id: i32) -> Result<Option<String>> {
    if let Some(d) = data.expertises_client()?.get(&id) {
        return Ok(Some(d.name.clone()));
    }
    Ok(json::str(data.expertises()?.at(&id.to_string()).at("name")))
}

/// "Quest bonus 20: Strength/Magic/Speed +" -> "20 quests: Strength/Magic/Speed +"
fn rx_item(item: &str) -> String {
    rx::rep(&rx::rep(&rx::rep(item, r"^Quest bonus (\d+): ", "$1 quests: "), r"^Compendium 0 ", "Any count "), r"^Compendium (\d+)", "$1 entries")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_items_become_numbers() {
        let (e0, r0) = parse_description("Passive skill.\\n\\n. STR +10\\n. Max HP +5%\\n. When something happens:\\n. CRIT +9", &[]).unwrap();
        assert_eq!(e0.len(), 1); // only Max HP is a plain shape; STR waits for a number rule, the "When ...:" block stays text
        assert_eq!(r0, vec!["STR +10".to_string(), "When something happens:".to_string(), "CRIT +9".to_string()]);
        let (effects, rest) = parse_description("・Max HP +5%\\n・Critical +3\\n・Does a special thing", &[]).unwrap();
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].id.as_deref(), Some("HP_MAX"));
        assert!(effects[0].pct);
        assert_eq!(effects[1].id.as_deref(), Some("CRITICAL"));
        assert_eq!(rest, vec!["Does a special thing".to_string()]);
    }

    #[test]
    fn chain_skills_text() {
        assert_eq!(rx_item("Quest bonus 20: STR +1"), "20 quests: STR +1");
        assert_eq!(rx_item("Compendium 0 foo"), "Any count foo");
        assert_eq!(rx_item("Compendium 50 foo"), "50 entries foo");
    }
}
