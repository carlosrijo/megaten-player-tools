//! Port of `src/Calc/ItemEffects.cs`.
//!
//! One worn item's effects worked out from its record (ids) and the server data, the way the server applies them
//! (comp_hack CharacterState.cpp / TokuseiManager.cpp), with no tooltip needed:
//!   stats       the correct table of the basic-effect item (its own id when not fused)        Source "item"
//!   fuse bonus  fusion great-success bonuses through the server's growth tables               Source "item"
//!   special     the SItem tokusei of the special-effect item (its own id when not fused)       Source "characteristic"
//!   mods        modification slot effects (weapon table, or the extension table)              Source "mod"
//!   tarot/soul  the crystal's tokusei, plus its extra tokusei whose requirement is met         Source "tarot" / "soul"
//! A broken item (durability 0) gives nothing, as on the server. Requirements the reader can check (alignment, level,
//! expertise rank, digitalized) are decided; others (partner, HP, time, enemy, skill type) are kept as situational.
//!
//! Differences from C#, forced by the language: the data lookups can fail (a missing website file), so the methods return
//! [`Result`]; `GameData` is shared (`Arc`) because the calculators are used from worker threads; where a C# string could be
//! null and the C# would have thrown (a missing stat id, a missing condition type) the text is empty here.

// The checks keep the C# shape (nested ifs, index loops) so each line can be compared with the original.
#![allow(clippy::collapsible_if, clippy::needless_range_loop, clippy::type_complexity, clippy::too_many_arguments, clippy::manual_range_contains)]

use std::sync::Arc;

use indexmap::IndexMap;

use super::effect::Effect;
use super::gear_effects;
use crate::Result;
use crate::data::game_data::{GameData, TokCond, TokValue};
use crate::data::json::{self, At};
use crate::data::stat_labels;
use crate::fmt;
use crate::game::WornItem;
use crate::game::client_reader;

static ACTION_NAMES: [&str; 12] = ["Attack", "Spin", "Rush", "Shot", "Rapid", "Guard", "Counter", "Dodge", "Talk", "Intimidate", "Taunt", "Support"];

/// An affinity number (skill affinity, and the element an AFFINITY_CAP_MAX raises) as the game names it; `None` if unknown.
pub fn affinity_name(id: i32) -> Option<String> {
    stat_labels::element_by_number(id)
}

/// AFFINITY_CAP_MAX raises one element's boost cap (the 100% a boost stops at): its Value is the element (the skill
/// affinity numbers: 9 = Ice) and its Modifier the amount in %. Checked against the game's own text for all 247 of
/// them ("Ice Affinity Cap +50%" is Value 9, Modifier 50). The label is the game's "<element> Affinity Cap".
pub fn affinity_cap_label(element: i32) -> String {
    format!("{} Affinity Cap", affinity_name(element).unwrap_or_else(|| format!("Element {element}")))
}

/// `v.ToString("0.##", InvariantCulture)`.
fn num(v: f64) -> String {
    fmt::custom(v, "0.##")
}

/// A char of the Unicode "word separator" categories .NET's `ToTitleCase` ends a word at (space, punctuation, symbols, controls).
fn is_word_separator(c: char) -> bool {
    c.is_whitespace() || c.is_control() || (!c.is_alphanumeric() && !c.is_ascii_digit() && !matches!(c, '\'' | '\u{200d}') && !is_mark(c))
}

fn is_mark(c: char) -> bool {
    // combining marks stay inside a word
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

/// .NET `TextInfo.ToTitleCase` (en-US and the invariant culture agree): the first letter of every word upper case, the
/// rest of the word lower case. Words end at spaces, punctuation and symbols; digits and apostrophes stay inside a word.
pub fn to_title_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() {
            // the title-case form of the first letter (Lu/Lt/Ll/Lo: to_uppercase is the title form for all but the digraphs)
            out.extend(c.to_uppercase());
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                if d.is_alphabetic() {
                    out.extend(d.to_lowercase());
                    i += 1;
                } else if d == '\'' {
                    out.push(d);
                    i += 1;
                } else if is_word_separator(d) {
                    break;
                } else {
                    out.push(d);
                    i += 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `Title(s)`: `ToTitleCase(s.ToLowerInvariant().Replace('_', ' '))`.
pub(crate) fn title(s: &str) -> String {
    to_title_case(&s.to_lowercase().replace('_', " "))
}

/// `Join(a, b)`: two notes with "; " between, skipping an empty one.
pub(crate) fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.to_string()
    } else if b.is_empty() {
        a.to_string()
    } else {
        format!("{a}; {b}")
    }
}

/// C# `ItemEffects(GameData data, string alignment, int level, Dictionary<int,int> expertiseRanks)`.
#[derive(Clone)]
pub struct ItemEffects {
    data: Arc<GameData>,
    level: i32,
    expertise_ranks: IndexMap<i32, i32>,
    /// The alignment in upper case.
    lnc: String,
}

impl ItemEffects {
    pub fn new(data: Arc<GameData>, alignment: &str, level: i32, expertise_ranks: IndexMap<i32, i32>) -> ItemEffects {
        ItemEffects { data, level, expertise_ranks, lnc: alignment.to_uppercase() }
    }

    /// The game data this calculator reads.
    pub fn data(&self) -> &Arc<GameData> {
        &self.data
    }

    /// The item's effects. tooltip (optional): the effects read from its tooltip; a part whose server data is missing
    /// (an enchant, item or tokusei the data files don't have) uses the tooltip's lines for that part instead.
    /// Mods never do: the tooltip shows a mod's level, not its value.
    pub fn compute(&self, w: Option<&WornItem>, slot: i32, item_name: &str, tooltip: Option<&[Effect]>) -> Result<Vec<Effect>> {
        let fx = self.parts(w, slot, item_name)?;
        let Some(tooltip) = tooltip else { return Ok(fx) };
        let mut res = Vec::new();
        // GroupBy(e => e.Source): first-appearance order of the keys
        let mut groups: IndexMap<String, Vec<Effect>> = IndexMap::new();
        for e in fx {
            groups.entry(e.source.clone()).or_default().push(e);
        }
        for (key, part) in groups {
            if key != "mod" && part.iter().any(|e| e.missing_data) {
                let tip: Vec<&Effect> = tooltip.iter().filter(|e| e.source == key).collect();
                if !tip.is_empty() {
                    // The tooltip shows the part as a whole (for "item": fusion bonuses included), so it replaces the part.
                    for e in tip {
                        let mut e = e.clone();
                        e.cond_note = join(&e.cond_note, "from the tooltip: no server data");
                        res.push(e);
                    }
                    continue;
                }
            }
            res.extend(part);
        }
        Ok(res)
    }

    fn parts(&self, w: Option<&WornItem>, slot: i32, item_name: &str) -> Result<Vec<Effect>> {
        let data = &*self.data;
        let mut fx = Vec::new();
        let Some(w) = w else { return Ok(fx) };
        if w.id <= 0 {
            return Ok(fx);
        }
        let bullets = slot == 1;
        if !bullets && w.durability == 0 && w.max_durability > 0 {
            fx.push(Effect { label: "Broken (durability 0): no bonuses".into(), cond: "inactive".into(), source: "item".into(), item: item_name.into(), ..Default::default() });
            return Ok(fx);
        }

        // Stats: the basic-effect item's correct table
        let basic = if w.basic > 0 { w.basic } else { w.id };
        match data.item_correct(basic)? {
            None => fx.push(Effect {
                label: format!("Stats of item {basic} (not in the item list)"),
                cond: "unparsed".into(),
                source: "item".into(),
                item: item_name.into(),
                missing_data: true,
                ..Default::default()
            }),
            Some(correct) => {
                for c in correct {
                    fx.push(self.correct_effect(&TokValue::new(c.id.as_deref(), c.r#type, c.value, "NONE", 0), "always", "", "self", "item", item_name, 0));
                }
            }
        }

        // Fusion great-success bonuses
        for (stat, boost, _) in self.fuse_bonus(w)? {
            fx.push(Effect {
                label: gear_effects::correct_label_of(stat),
                value: Some(boost as f64),
                source: "item".into(),
                item: item_name.into(),
                cond_note: "fusion great success".into(),
                ..Default::default()
            });
        }

        // Special effects
        let special = if w.special > 0 { w.special } else { w.id };
        if !data.items()?.contains_key(&special) {
            fx.push(Effect {
                label: format!("Special effects of item {special} (not in the item list)"),
                cond: "unparsed".into(),
                source: "characteristic".into(),
                item: item_name.into(),
                missing_data: true,
                ..Default::default()
            });
        }
        for t in data.sitem_tokusei(special)? {
            fx.extend(self.tokusei(t, "characteristic", item_name)?);
        }

        // Modification slots
        let equip_type = client_reader::equip_type_of_slot(slot);
        let weapon = equip_type == 13;
        for &m in &w.mods {
            if m == 0 || m == 255 {
                continue;
            }
            let t = data.mod_tokusei(weapon, data.item_sub_category(w.id)?, equip_type, m)?;
            if t == 0 {
                fx.push(Effect {
                    label: format!("Modification effect {m} (not in the mod tables)"),
                    cond: "unparsed".into(),
                    source: "mod".into(),
                    item: item_name.into(),
                    missing_data: true,
                    ..Default::default()
                });
            } else {
                fx.extend(self.tokusei(t, "mod", item_name)?);
            }
        }

        // Tarot and soul
        for (id, is_tarot) in [(w.tarot, true), (w.soul, false)] {
            if id > 0 {
                fx.extend(self.enchant(id, is_tarot, item_name)?);
            }
        }
        Ok(fx)
    }

    /// What one crystal gives as a tarot (isTarot) or as a soul fusion: its tokusei, plus its extra tokusei by requirement.
    pub fn enchant(&self, id: i32, is_tarot: bool, item_name: &str) -> Result<Vec<Effect>> {
        let mut fx = Vec::new();
        let src = if is_tarot { "tarot" } else { "soul" };
        let Some(side) = self.data.enchant_effect(id, is_tarot)? else {
            fx.push(Effect {
                label: format!("{src} enchant {id} (not in the enchant list)"),
                cond: "unparsed".into(),
                source: src.into(),
                item: item_name.into(),
                missing_data: true,
                ..Default::default()
            });
            return Ok(fx);
        };
        for &t in &side.tokusei {
            fx.extend(self.tokusei(t, src, item_name)?);
        }
        for (typ, p1, p2, toks) in &side.conditions {
            let (met, note) = self.enchant_condition(*typ, *p1, *p2)?;
            for &t in toks {
                for mut e in self.tokusei(t, src, item_name)? {
                    let counted = e.cond == "always" || e.cond == "digitalized";
                    if met == Some(false) && counted {
                        e.cond = "inactive".into();
                    } else if met.is_none() && counted {
                        e.cond = "situational".into();
                    }
                    if !note.is_empty() {
                        e.cond_note = if e.cond_note.is_empty() { note.clone() } else { format!("{note}; {}", e.cond_note) };
                    }
                    fx.push(e);
                }
            }
        }
        Ok(fx)
    }

    /// Requirement of an enchant's extra tokusei (TokuseiManager: 0 none, 1 level, 2 alignment, 10+stat, 100+expertise).
    fn enchant_condition(&self, typ: i32, p1: i32, p2: i32) -> Result<(Option<bool>, String)> {
        Ok(match typ {
            0 => (Some(true), String::new()),
            1 => (
                Some((p1 == 0 || self.level >= p1) && (p2 == 0 || self.level <= p2)),
                if p2 == 0 { format!("level {p1}+") } else { format!("level {p1}-{p2}") },
            ),
            2 => {
                let mut names = Vec::new();
                if (p1 & 4) != 0 {
                    names.push("LAW");
                }
                if (p1 & 2) != 0 {
                    names.push("NEUTRAL");
                }
                if (p1 & 1) != 0 {
                    names.push("CHAOS");
                }
                (Some(names.contains(&self.lnc.as_str())), format!("if you are {}", names.join(" or ")))
            }
            100..=158 => {
                let id = typ - 100;
                let have = self.expertise_ranks.get(&id).copied().unwrap_or(0);
                let name = json::str(self.data.expertises()?.at(&id.to_string()).at("name")).unwrap_or_else(|| format!("expertise {id}"));
                (Some(have >= p1), format!("needs Class {} {name} (you have {})", p1 / 10, have / 10))
            }
            _ => {
                if (10..30).contains(&typ) {
                    (None, format!("needs a core stat of {p1}+"))
                } else {
                    (None, format!("special condition {typ}"))
                }
            }
        })
    }

    /// Fusion great-success bonus per stat (CharacterState::AdjustFuseBonus growth tables). Max = the bonus is at the
    /// top of its table (fuse value 50), which the game marks "MAX" in the item's Features tab.
    pub fn fuse_bonus(&self, w: &WornItem) -> Result<Vec<(&'static str, i32, bool)>> {
        let mut res = Vec::new();
        if w.fuse.iter().all(|&f| f <= 0) {
            return Ok(res);
        }
        const MINOR: [[i32; 2]; 16] = [[0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [5, 2], [10, 3], [15, 4], [20, 5], [25, 7], [30, 10], [35, 13], [40, 16], [50, 20]];
        const WEAPON_G: [[i32; 2]; 16] = [[2, 2], [4, 3], [6, 4], [8, 5], [10, 6], [12, 7], [14, 8], [16, 9], [18, 12], [21, 15], [24, 20], [27, 25], [30, 30], [35, 35], [40, 40], [50, 45]];
        const ARMOR_G: [[i32; 2]; 16] = [[0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [0, 1], [5, 2], [10, 3], [15, 5], [20, 7], [25, 10], [30, 13], [35, 16], [40, 19], [50, 25]];
        let mut table = &MINOR;
        let types: [Option<&'static str>; 3];
        match self.data.item_equip_type(w.id)?.as_deref() {
            Some("EQUIP_TYPE_WEAPON") => {
                types = [
                    Some(if self.data.item_weapon_type(w.id)?.as_deref() == Some("CLOSE_RANGE") { "CLSR" } else { "LNGR" }),
                    Some("SPELL"),
                    Some("SUPPORT"),
                ];
                table = &WEAPON_G;
            }
            Some("EQUIP_TYPE_TOP") | Some("EQUIP_TYPE_BOTTOM") => {
                types = [Some("PDEF"), Some("MDEF"), None];
                table = &ARMOR_G;
            }
            Some("EQUIP_TYPE_HEAD") | Some("EQUIP_TYPE_ARMS") | Some("EQUIP_TYPE_FEET") => types = [Some("PDEF"), Some("MDEF"), None],
            Some("EQUIP_TYPE_RING") | Some("EQUIP_TYPE_EARRING") | Some("EQUIP_TYPE_EXTRA") | Some("EQUIP_TYPE_TALISMAN") => types = [None, Some("MDEF"), None],
            _ => return Ok(res),
        }
        for i in 0..3 {
            let bonus = w.fuse.get(i).copied().unwrap_or(0);
            let Some(typ) = types[i] else { continue };
            if bonus <= 0 {
                continue;
            }
            let mut boost = 0;
            for k in 0..16 {
                if table[k][0] == bonus {
                    boost = table[k][1];
                    break;
                }
                if table[k][0] > bonus {
                    boost = if k > 0 { table[k - 1][1] } else { 1 };
                    break;
                }
            }
            if boost != 0 {
                res.push((typ, boost, bonus >= table[15][0]));
            }
        }
        Ok(res)
    }

    /// One tokusei as effects, with its requirements decided where the reader can.
    pub fn tokusei(&self, tid: i32, source: &str, item: &str) -> Result<Vec<Effect>> {
        let mut output = Vec::new();
        let Some(d) = self.data.tokusei_full(tid)? else {
            output.push(Effect {
                label: format!("Unknown effect (tokusei {tid})"),
                cond: "unparsed".into(),
                source: source.into(),
                item: item.into(),
                tokusei: tid,
                missing_data: true,
                ..Default::default()
            });
            return Ok(output);
        };
        let target = if matches!(d.target.as_str(), "PARTNER" | "PARTY" | "SUMMONER") { "partner" } else { "self" };

        // Requirements: conditions in the same option group are alternatives, the rest must all hold.
        let mut cond = "always".to_string();
        let mut notes: Vec<String> = Vec::new();
        // GroupBy keeps the first-appearance order of the keys; a condition without a group is its own group, named by
        // the index of the first condition equal to it (a record: IndexOf compares values)
        let mut groups: IndexMap<String, Vec<&TokCond>> = IndexMap::new();
        for c in &d.conditions {
            let key = if c.group == 0 { format!("single{}", d.conditions.iter().position(|x| x == c).map_or(-1, |p| p as i32)) } else { format!("group{}", c.group) };
            groups.entry(key).or_default().push(c);
        }
        for grp in groups.values() {
            let mut met = Some(false);
            let mut gnotes: Vec<String> = Vec::new();
            for c in grp {
                let (m, n) = self.condition(c);
                gnotes.push(n);
                if m == Some(true) {
                    met = Some(true);
                    break;
                }
                if m.is_none() {
                    met = None;
                }
            }
            if grp.iter().any(|c| c.r#type.as_deref() == Some("DIGITALIZED")) {
                if cond == "always" {
                    cond = "digitalized".into();
                }
                continue;
            }
            if met == Some(false) {
                cond = "inactive".into();
                notes.push(gnotes.join(" or "));
            } else if met.is_none() {
                if cond != "inactive" {
                    cond = "situational".into();
                }
                notes.push(gnotes.join(" or "));
            }
        }

        // Skill requirements: an Action type names the skill kind; anything else depends on the skill or the enemy.
        let actions: Vec<String> = d
            .skill_conditions
            .iter()
            .filter(|s| s.0.as_deref() == Some("ACTION_TYPE"))
            .map(|s| if s.1 >= 0 && (s.1 as usize) < ACTION_NAMES.len() { ACTION_NAMES[s.1 as usize].to_string() } else { format!("action {}", s.1) })
            .collect();
        let other_skill: Vec<String> = d
            .skill_conditions
            .iter()
            .filter(|s| s.0.as_deref() != Some("ACTION_TYPE"))
            .map(|s| match s.0.as_deref() {
                Some("ENEMY_LNC") => format!(
                    "vs {} enemies",
                    match s.1 {
                        0 => "LAW".to_string(),
                        2 => "NEUTRAL".to_string(),
                        4 => "CHAOS".to_string(),
                        v => format!("alignment {v}"),
                    }
                ),
                Some("AFFINITY") if affinity_name(s.1).is_some() => format!("{} skills only", affinity_name(s.1).unwrap_or_default()),
                t => format!("skill: {} {}", title(t.unwrap_or("")), s.1),
            })
            .collect();
        if !other_skill.is_empty() || d.target_conditions {
            if cond != "inactive" {
                cond = "situational".into();
            }
            notes.push(if !other_skill.is_empty() { other_skill.join(", ") } else { "depends on the enemy".to_string() });
        }
        if d.from_client_text {
            notes.push("from the client's text (no server data)".into());
        }
        let note = notes.iter().filter(|n| !n.is_empty()).cloned().collect::<Vec<_>>().join("; ");
        let counted = |c: &str| c == "always" || c == "digitalized";

        for c in &d.correct {
            let mut e = self.correct_effect(c, &cond, &note, target, source, item, tid);
            if !actions.is_empty() && counted(&e.cond) {
                e.cond = "situational".into();
                e.cond_note = join(&e.cond_note, &format!("{} skills only", actions.join("/")));
            }
            output.push(e);
        }
        for a in &d.aspects {
            let (value, mnote, ok) = self.scaled(a);
            let id = a.id.as_deref().unwrap_or("");
            let lab: String;
            let mut pct = true;
            let mut acond = if ok { cond.clone() } else { "unparsed".to_string() };
            let mut anote = join(&note, &mnote);
            let mk = |label: String, value: Option<f64>, pct: bool, cond: &str| Effect {
                label,
                value,
                pct,
                cond: cond.into(),
                cond_note: note.clone(),
                target: target.into(),
                source: source.into(),
                item: item.into(),
                tokusei: tid,
                ..Default::default()
            };
            match id {
                "EFFECT_POWER" => {
                    if actions.len() == 1 {
                        lab = format!("{} Damage", actions[0]);
                    } else {
                        lab = if actions.len() > 1 { format!("{} Damage", actions.join("/")) } else { "Skill Power".to_string() };
                        if counted(&acond) {
                            acond = "situational".into();
                        }
                    }
                }
                "DAMAGE_DEALT" => lab = stat_labels::of(id).unwrap_or("Damage Dealt").to_string(), // no client text names it alone
                "LIMIT_BREAK_MAX" => {
                    lab = stat_labels::name(id, 0);
                    pct = false;
                }
                "TECH_ATTACK_RATE" | "TECH_ATTACK_POWER" | "PURSUIT_RATE" | "PURSUIT_POWER" | "DIGITALIZE_POINT_RATE" | "DIGITALIZE_LENGTH" | "MP_COST_ADJUST" | "HP_COST_ADJUST"
                | "BOOST_WEAPON" | "CLENCH_CHANCE" => lab = stat_labels::name(id, 0),
                "STATUS_NULL" => {
                    output.push(mk(format!("Immune to status {}", num(a.value)), None, false, &cond));
                    continue;
                }
                "STATUS_INFLICT_ADJUST" => {
                    output.push(mk(
                        if a.value == 0.0 { "Status ailment success rate".to_string() } else { format!("Status {} success rate", num(a.value)) },
                        Some(a.modifier.unwrap_or(0.0)),
                        true,
                        &cond,
                    ));
                    continue;
                }
                "AFFINITY_CAP_MAX" => {
                    output.push(mk(affinity_cap_label(a.value as i32), a.modifier, true, if a.modifier.is_none() { "unparsed" } else { &cond }));
                    continue;
                }
                "SKILL_ADD" => {
                    let sn = match self.data.skill_names()?.get(&(a.value as i32)) {
                        Some(n) => n.clone().unwrap_or_default(),
                        None => num(a.value),
                    };
                    output.push(mk(format!("Adds skill {sn}"), None, false, &cond));
                    continue;
                }
                _ => {
                    let mut e = mk(format!("{id} {}", num(a.value)), None, false, "unparsed");
                    e.pct = false;
                    output.push(e);
                    continue;
                }
            }
            if !actions.is_empty() && id != "EFFECT_POWER" && counted(&acond) {
                acond = "situational".into();
                anote = join(&anote, &format!("{} skills only", actions.join("/")));
            }
            output.push(Effect {
                label: lab,
                value: if ok { Some(value) } else { None },
                pct,
                cond: acond,
                cond_note: anote,
                target: target.into(),
                source: source.into(),
                item: item.into(),
                tokusei: tid,
                ..Default::default()
            });
        }
        Ok(output)
    }

    /// A value with its multiplier applied (TokuseiManager::CalculateAttributeValue). ok = false when it can't be worked out here.
    fn scaled(&self, v: &TokValue) -> (f64, String, bool) {
        let x = if v.precision > 0 { v.value / 10f64.powi(v.precision) } else { v.value };
        match v.mult.as_str() {
            "NONE" | "" => (x, String::new(), true),
            "LEVEL" => (x * self.level as f64, format!("LvL × {}", num(x)), true),
            m => (x, format!("× {}", title(m)), false),
        }
    }

    /// One CorrectTbl line (a skill's or item's correct table) as an effect. type 1 = percent.
    pub fn correct(&self, id: &str, typ: i32, value: f64, cond: &str, source: &str, item: &str) -> Effect {
        self.correct_effect(&TokValue::new(Some(id), typ, value, "NONE", 0), cond, "", "self", source, item, 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn correct_effect(&self, c: &TokValue, cond: &str, note: &str, target: &str, source: &str, item: &str, tid: i32) -> Effect {
        let typ = if c.r#type >= 100 { c.r#type - 100 } else { c.r#type };
        let (value, mnote, ok) = self.scaled(c);
        let note = join(note, &mnote);
        let id = c.id.as_deref().unwrap_or("");
        let mk = |label: String, v: Option<f64>, pct: bool| Effect {
            label,
            value: if ok { v } else { None },
            pct,
            cond: if ok { cond.to_string() } else { "unparsed".to_string() },
            cond_note: note.clone(),
            target: target.into(),
            source: source.into(),
            item: item.into(),
            tokusei: tid,
            ..Default::default()
        };
        // NRA type 3/4/5 = null/reflect/drain (CorrectTbl NRA type = NRA index + 2, ActiveEntityState.cpp): "50% Chance to Null Fire"
        if id.starts_with("NRA_") && (3..=5).contains(&typ) {
            return mk(format!("{}% {}", num(value), stat_labels::name(id, typ)), None, false);
        }
        if id.starts_with("BOOST_") || id.starts_with("RES_") {
            return mk(stat_labels::name(id, 0), Some(value), true);
        }
        mk(stat_labels::name(id, 0), Some(value), typ == 1 || gear_effects::is_pct_correct(id))
    }

    /// A tokusei condition the reader can decide (true/false), or `None` when it depends on the fight or partner.
    fn condition(&self, c: &TokCond) -> (Option<bool>, String) {
        let eq = matches!(c.cmp.as_str(), "EQUALS" | "");
        match c.r#type.as_deref() {
            Some("DIGITALIZED") => (None, "while digitalized".into()),
            Some("LNC") => {
                // Linear mode: L/N/C = 0/2/4, 1 = L or N, 3 = N or C, 5 = not N (ActiveEntityState::IsLNCType)
                let me = match self.lnc.as_str() {
                    "LAW" => 0,
                    "CHAOS" => 4,
                    _ => 2,
                };
                let contains = match c.value {
                    1 => me <= 2,
                    3 => me >= 2,
                    5 => me != 2,
                    v => me == v,
                };
                let want = match c.value {
                    0 => "LAW".to_string(),
                    1 => "LAW or NEUTRAL".to_string(),
                    2 => "NEUTRAL".to_string(),
                    3 => "NEUTRAL or CHAOS".to_string(),
                    4 => "CHAOS".to_string(),
                    5 => "LAW or CHAOS".to_string(),
                    v => format!("alignment {v}"),
                };
                (Some(contains == eq), if eq { format!("if you are {want}") } else { format!("unless you are {want}") })
            }
            t => {
                // .Trim() of "{Title(type)} {cmp }{value}"
                let cmp = if c.cmp == "EQUALS" { String::new() } else { format!("{} ", c.cmp) };
                (None, format!("{} {}{}", title(t.unwrap_or("")), cmp, c.value).trim().to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_case_like_dotnet() {
        assert_eq!(title("ENEMY_LNC"), "Enemy Lnc");
        assert_eq!(title("LEVEL"), "Level");
        assert_eq!(to_title_case("close range"), "Close Range");
        assert_eq!(to_title_case("rock'n roll 3d"), "Rock'n Roll 3D");
    }

    #[test]
    fn join_skips_empty() {
        assert_eq!(join("", "b"), "b");
        assert_eq!(join("a", ""), "a");
        assert_eq!(join("a", "b"), "a; b");
    }
}
