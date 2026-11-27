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
use super::enchant_text;
use super::gear_effects;
use crate::Result;
use crate::data::game_data::{EnchantSide, GameData, TokCond, TokValue};
use crate::data::json::{self, At};
use crate::data::stat_labels;
use crate::fmt;
use crate::game::WornItem;
use crate::game::client_reader;
use crate::text::eq_ic;

/// In the note of an effect that needs a stat the calculator wasn't given ("100 ÷ Luck: depends on your Luck").
pub const NEEDS_STATS_NOTE: &str = ": depends on your ";

static ACTION_NAMES: [&str; 12] = ["Attack", "Spin", "Rush", "Shot", "Rapid", "Guard", "Counter", "Dodge", "Talk", "Intimidate", "Taunt", "Support"];

/// An affinity number (skill affinity, and the element an AFFINITY_CAP_MAX raises) as the game names it; `None` if unknown.
/// A demon family by its number (libcomp devildata.xml MiDCategoryData family, the names the Demon Database shows).
pub fn demon_family_name(id: i32) -> Option<&'static str> {
    const FAMILIES: [&str; 14] = ["", "Human", "God", "Aerial", "Guardian", "Demoniac", "Dragon", "Magica", "Bird", "Beast", "Evil Spirit", "Foul", "Elemental", "Machine"];
    FAMILIES.get(usize::try_from(id).ok()?).copied().filter(|n| !n.is_empty())
}

/// The demons that carry a title (reference\devil-titles.tsv, from DevilData basic.title), for a DEMON_TITLE condition.
pub fn demon_title_names(id: i32) -> Option<Vec<String>> {
    static TITLES: std::sync::OnceLock<std::collections::HashMap<i32, Vec<String>>> = std::sync::OnceLock::new();
    TITLES
        .get_or_init(|| {
            let mut m = std::collections::HashMap::new();
            if let Ok(Some(lines)) = crate::data::game_data::reference_lines("devil-titles.tsv") {
                for l in lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    if let Some((k, v)) = l.split_once('\t') {
                        if let Some(k) = crate::fmt::parse_i32(k.trim()) {
                            m.insert(k, v.trim().split(';').filter(|s| !s.is_empty()).map(str::to_string).collect());
                        }
                    }
                }
            }
            m
        })
        .get(&id)
        .cloned()
        .filter(|v: &Vec<String>| !v.is_empty())
}

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
/// "A", "A or B", "A, B or C".
fn or_list(names: &[String]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// "A", "A and B", "A, B and C".
pub fn and_list(names: &[String]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// What a tokusei's ACTIVE_TOKUSEI skill conditions ("tokusei N is active", NewMoon's own) depend on: each N is named by
/// the skill whose tokusei list holds it ([`GameData::tokusei_owner`]). A switch skill decides by its state (on or off,
/// which only the caller can tell: the Bonuses and Expertise pages read it from the game); any other skill (a passive)
/// is just named; a tokusei no skill lists is unknown.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ActiveNeeds {
    /// Switch skills (id, game name): every one must be on.
    pub switches: Vec<(i32, String)>,
    /// Other skills the effect goes with, by name.
    pub skills: Vec<String>,
    /// Some ACTIVE_TOKUSEI condition names no skill.
    pub unknown: bool,
}

impl ActiveNeeds {
    /// The ACTIVE_TOKUSEI conditions among a tokusei's skill conditions, resolved to skills.
    pub fn of(data: &GameData, skill_conditions: &[(Option<String>, i32)]) -> Result<ActiveNeeds> {
        let mut needs = ActiveNeeds::default();
        let names = data.skill_names()?;
        for (_, tid) in skill_conditions.iter().filter(|s| s.0.as_deref() == Some("ACTIVE_TOKUSEI")) {
            let Some(owner) = data.tokusei_owner(*tid)? else {
                needs.unknown = true;
                continue;
            };
            let name = names.get(&owner.skill).cloned().flatten().unwrap_or_else(|| "a skill the game data has no name for".to_string());
            if owner.switch {
                if !needs.switches.iter().any(|s| s.0 == owner.skill) {
                    needs.switches.push((owner.skill, name));
                }
            } else if !needs.skills.contains(&name) {
                needs.skills.push(name);
            }
        }
        Ok(needs)
    }

    pub fn is_empty(&self) -> bool {
        self.switches.is_empty() && self.skills.is_empty() && !self.unknown
    }

    /// Something other than switches that the reader can't decide (a passive's own state, an unnamed tokusei): the effect
    /// stays "in some situations" even when every switch is on.
    pub fn undecided(&self) -> bool {
        self.unknown || !self.skills.is_empty()
    }

    /// The words for the effect's note, with the switches' states when `on` knows them (None = not known: gear can't tell).
    /// "while Radiant Hands is on"; a switch that is off: "only while Radiant Hands is on, and it is off now: not counted".
    pub fn wording(&self, on: impl Fn(i32) -> Option<bool>) -> String {
        let mut parts: Vec<String> = Vec::new();
        let off: Vec<String> = self.switches.iter().filter(|s| on(s.0) == Some(false)).map(|s| s.1.clone()).collect();
        if !off.is_empty() {
            parts.push(format!("only while {} {} on, and {} off now: not counted", and_list(&off), if off.len() == 1 { "is" } else { "are" }, if off.len() == 1 { "it is" } else { "they are" }));
        } else if !self.switches.is_empty() {
            let names: Vec<String> = self.switches.iter().map(|s| s.1.clone()).collect();
            parts.push(format!("while {} {} on", and_list(&names), if names.len() == 1 { "is" } else { "are" }));
        }
        if !self.skills.is_empty() {
            parts.push(format!("with {}", and_list(&self.skills)));
        }
        if self.unknown {
            parts.push(stat_labels::sealed_skill_condition("ACTIVE_TOKUSEI", 0).unwrap_or_else(|| "while another effect is active".to_string()));
        }
        parts.join(", ")
    }
}

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
    /// The final stats of whoever gets the effects (CorrectTbl id -> value, e.g. "LUCK"), for the CORRECT_TABLE(_DIVIDE)
    /// multipliers ("Strength +100 ÷ Luck"); empty = not known here.
    stats: IndexMap<String, f64>,
}

impl ItemEffects {
    pub fn new(data: Arc<GameData>, alignment: &str, level: i32, expertise_ranks: IndexMap<i32, i32>) -> ItemEffects {
        ItemEffects { data, level, expertise_ranks, lnc: alignment.to_uppercase(), stats: IndexMap::new() }
    }

    /// The same calculator, told the final stats of whoever gets the effects (CorrectTbl id -> value).
    pub fn with_stats(mut self, stats: IndexMap<String, f64>) -> ItemEffects {
        self.stats = stats;
        self
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
        self.read_text_where_no_data(side, src, item_name, &mut fx)?;
        Ok(fx)
    }

    /// A crystal whose tokusei have no data at all (not in comp_hack, the website or the client's per-tokusei text) gave
    /// "Unknown effect" rows, which the Bonuses page drops, so its bonuses showed only while a fresh tooltip of the worn item
    /// was saved. The client's description of the crystal is one block for all of its tokusei, so it is read once here
    /// (`enchant_text`) and stands in for them: put on the lowest id that has no data, the others left empty. What the data
    /// already gives is not counted again (an effect of the same value, percent and side as one the data gave is skipped).
    /// Nothing changes when the text has no readable value: the rows stay as they were and the tooltip may still stand in.
    fn read_text_where_no_data(&self, side: &EnchantSide, src: &str, item_name: &str, fx: &mut Vec<Effect>) -> Result<()> {
        let is_gap = |e: &Effect| e.missing_data && e.cond == "unparsed" && e.source == src && e.tokusei != 0;
        let Some(lowest) = fx.iter().filter(|e| is_gap(e)).map(|e| e.tokusei).min() else { return Ok(()) };
        if side.text.is_empty() {
            return Ok(());
        }
        let known = !self.expertise_ranks.is_empty();
        let rank_of = |name: &str| -> Option<i32> {
            if !known {
                return None;
            }
            let id = self.data.expertises_client().ok()?.values().find(|d| eq_ic(&d.name, name))?.id;
            Some(self.expertise_ranks.get(&id).copied().unwrap_or(0))
        };
        let read = enchant_text::effects(&side.text, src, item_name, &enchant_text::Rules { alignment: &self.lnc, rank_of: &rank_of });
        if !read.iter().any(|e| e.value.is_some()) {
            return Ok(());
        }
        // What the data already gives is not counted twice: an effect on the same side with the same value and percent, or that
        // names the same stat ("Long-Range damage +15%" when the data has "Long-range Damage"), is skipped. Skipping too much
        // undercounts, which is the safer way to be wrong here.
        let family = |label: &str| {
            let l: String = label.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
            let l = l.strip_suffix("dealt").unwrap_or(&l).to_string();
            for tail in ["damage", "boost", "power", "effects", "affinity"] {
                if let Some(h) = l.strip_suffix(tail) {
                    if !h.is_empty() {
                        return h.to_string();
                    }
                }
            }
            l
        };
        // Each data effect covers one text effect at most, by name first, then by value: with the sealed tokusei a
        // crystal can have most of its tokusei in the data, and Executioner's Death +10% from the data must not also take
        // away the text's "Blunt-based Skill effects +10%" (363485, still without data).
        fx.retain(|e| !is_gap(e));
        let mut used = vec![false; fx.len()];
        let mut keep = vec![true; read.len()];
        for by_name in [true, false] {
            for (i, e) in read.iter().enumerate() {
                if !keep[i] || e.value.is_none() {
                    continue;
                }
                let hit = fx.iter().enumerate().position(|(j, h)| {
                    !used[j]
                        && h.cond != "unparsed"
                        && h.target == e.target
                        && if by_name { family(&h.label) == family(&e.label) } else { h.value == e.value && h.pct == e.pct }
                });
                if let Some(j) = hit {
                    used[j] = true;
                    keep[i] = false;
                }
            }
        }
        // a line the text can't put a number on, when the data already has that stat ("Limit Break Cap + (STR x 1)")
        let named = |e: &Effect| {
            let l = e.label.to_lowercase();
            e.value.is_none() && fx.iter().any(|h| !h.label.is_empty() && l.starts_with(&h.label.to_lowercase()))
        };
        let fresh: Vec<Effect> =
            read.into_iter().zip(keep).filter(|(e, k)| *k && !named(e)).map(|(e, _)| Effect { tokusei: lowest, ..e }).collect();
        fx.extend(fresh);
        Ok(())
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
        // Named skills (EXPLICIT_SKILL, one or several in an option group: any of them) by the game's names
        let skill_names = self.data.skill_names()?;
        let named: Vec<String> = d
            .skill_conditions
            .iter()
            .filter(|s| s.0.as_deref() == Some("EXPLICIT_SKILL"))
            .filter_map(|s| skill_names.get(&s.1).cloned().flatten())
            .fold(Vec::new(), |mut v, n| {
                if !v.contains(&n) {
                    v.push(n);
                }
                v
            });
        let explicit = d.skill_conditions.iter().any(|s| s.0.as_deref() == Some("EXPLICIT_SKILL"));
        let mut other_skill: Vec<String> = Vec::new();
        if explicit {
            other_skill.push(match named.len() {
                0 => "certain skills only".to_string(),
                1..=4 => format!("only with {}", or_list(&named)),
                n => format!("only with {}, {} and {} more skills", named[0], named[1], n - 2),
            });
        }
        // The target demon's race or kind (SkillManager.cpp: DEMON_RACE / DEMON_TYPE), several = any of them
        let races: Vec<String> = d.skill_conditions.iter().filter(|s| s.0.as_deref() == Some("DEMON_RACE")).map(|s| self.race_text(s.1)).fold(Vec::new(), |mut v, n| {
            if !v.contains(&n) {
                v.push(n);
            }
            v
        });
        if !races.is_empty() {
            other_skill.push(format!("vs {} demons", or_list(&races)));
        }
        let demon_names: Vec<String> = d
            .skill_conditions
            .iter()
            .filter(|s| s.0.as_deref() == Some("DEMON_TYPE"))
            .map(|s| self.data.demons().ok().and_then(|t| t.devils.get(&s.1).map(|v| v.name.clone())).unwrap_or_else(|| "a certain demon".into()))
            .fold(Vec::new(), |mut v, n| {
                if !v.contains(&n) {
                    v.push(n);
                }
                v
            });
        if !demon_names.is_empty() {
            other_skill.push(format!("vs {}", or_list(&demon_names)));
        }
        // ACTIVE_TOKUSEI ("while tokusei N is active"): worded by the skill that lists it, a switch as "while Radiant Hands is on".
        // Gear can't tell whether the switch is on (the Expertise rows can), so the effect stays situational.
        let needs = ActiveNeeds::of(&self.data, &d.skill_conditions)?;
        if !needs.is_empty() {
            other_skill.push(needs.wording(|_| None));
        }
        let expertises = self.data.expertises().ok();
        other_skill.extend(d
            .skill_conditions
            .iter()
            .filter(|s| !matches!(s.0.as_deref(), Some("ACTION_TYPE") | Some("EXPLICIT_SKILL") | Some("DEMON_RACE") | Some("DEMON_TYPE") | Some("ACTIVE_TOKUSEI")))
            .map(|s| match s.0.as_deref() {
                // SkillManager.cpp: 0 = the target is of another faction, 1 = the same faction
                Some("ENEMY_FACTION") => if s.1 == 1 { "on allies".to_string() } else { "on enemies".to_string() },
                Some("ENEMY_STATUS_ACTIVE") => format!("vs enemies under {}", crate::data::game_data::status_target(s.1)),
                // the skill's expertise (TokuseiManager, SKILL_EXPERTISE: the skill belongs to that expertise)
                Some("SKILL_EXPERTISE") => match expertises.as_ref().and_then(|e| json::str(e.at(&s.1.to_string()).at("name"))) {
                    Some(name) => format!("{name} skills only"),
                    None => "skills of a certain expertise only".to_string(),
                },
                Some("ENEMY_LNC") => format!(
                    "vs {} enemies",
                    match s.1 {
                        0 => "LAW".to_string(),
                        2 => "NEUTRAL".to_string(),
                        4 => "CHAOS".to_string(),
                        v => format!("alignment {v}"),
                    }
                ),
                // the target demon's family (SkillManager.cpp: DEMON_FAMILY = MiDCategoryData family, libcomp devildata.xml;
                // checked on Epitaph Parts [Demoniac-Slayer Fruit], whose game text says "the Magica family" for family 7)
                Some("DEMON_FAMILY") if demon_family_name(s.1).is_some() => format!("vs {}-family demons", demon_family_name(s.1).unwrap_or_default()),
                // the target demon's title (DEMON_TITLE: DevilData basic.title), named by the demons that carry it
                Some("DEMON_TITLE") if demon_title_names(s.1).is_some() => {
                    let names = demon_title_names(s.1).unwrap_or_default();
                    if names.len() > 4 { format!("vs {}, {}, {} and {} more", names[0], names[1], names[2], names.len() - 3) } else { format!("vs {}", or_list(&names)) }
                }
                Some("DEMON_TITLE") => "vs certain demons (no demon in the game data has that title)".to_string(),
                // SkillManager.cpp SKILL_CLASS: 1 magic (spell and support), 2 physical (close and long range), 3 the rest
                Some("SKILL_CLASS") if (1..=3).contains(&s.1) => ["Magic skills only", "Physical skills only", "Skills that are neither magic nor physical"][(s.1 - 1) as usize].to_string(),
                // ENEMY_GENDER: MiNPCBasicData gender MALE 0, FEMALE 1, NONE 2
                Some("ENEMY_GENDER") if (0..=2).contains(&s.1) => ["vs male enemies", "vs female enemies", "vs enemies without a gender"][s.1 as usize].to_string(),
                Some("AFFINITY") if affinity_name(s.1).is_some() => format!("{} skills only", affinity_name(s.1).unwrap_or_default()),
                t => stat_labels::sealed_skill_condition(t.unwrap_or(""), s.1).unwrap_or_else(|| format!("skill: {} {}", title(t.unwrap_or("")), s.1)),
            }));
        other_skill.dedup();
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
                    // the Value is a status's id (or a whole group): said by the game's name, never as the number
                    output.push(mk(format!("Immune to {}", crate::data::game_data::status_target(a.value as i32)), None, false, &cond));
                    continue;
                }
                "STATUS_INFLICT_ADJUST" => {
                    output.push(mk(
                        if a.value == 0.0 || a.value == -1.0 {
                            "Status ailment success rate".to_string()
                        } else {
                            let what = crate::data::game_data::status_target(a.value as i32);
                            // a named status: "Charm success rate"; one the data can't name: a plain sentence
                            if crate::data::game_data::status_name(a.value as i32).is_some() { format!("{what} success rate") } else { format!("Success rate of {what}") }
                        },
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
                _ => match stat_labels::aspect_show(id, a.value) {
                    None => {
                        // a named aspect (Chance to Null Knockback, All Skill Stack, Soul Points Earned...): counted like the
                        // others; a stack adjustment is a number of stacks, not a percent
                        lab = stat_labels::name(id, 0);
                        pct = !id.ends_with("STACK_ADJUST");
                    }
                    Some(stat_labels::AspectShow::Sentence(s)) => {
                        output.push(mk(s, None, false, &cond));
                        continue;
                    }
                    Some(stat_labels::AspectShow::Uncounted(name)) => {
                        // shown with its number, never counted: nothing says how the server applies it
                        let mut e = mk(name.to_string(), if ok { Some(value) } else { None }, false, if acond == "inactive" { "inactive" } else { "situational" });
                        e.cond_note = join(&anote, stat_labels::UNCOUNTED_NOTE);
                        output.push(e);
                        continue;
                    }
                    Some(stat_labels::AspectShow::Unknown) => {
                        // no wording for it (the value is a skill, an action, an element...): like a tokusei without data, so
                        // a saved tooltip or the game's own description can stand in, and no internal name is shown
                        output.push(Effect { label: format!("Unknown effect (tokusei {tid})"), missing_data: true, ..mk(String::new(), None, false, "unparsed") });
                        continue;
                    }
                },
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
            // TokuseiManager::CalculateAttributeValue: the value times, or divided by, one CorrectTbl value of the
            // entity's calculated state (MultiplierValue = its index: 5 = LUCK), then cut to a whole number
            // (ActiveEntityState casts it to int32). "Strength +100 ÷ Luck" with Luck 57 is +1; over 100 it is 0.
            m @ ("CORRECT_TABLE" | "CORRECT_TABLE_DIVIDE") => {
                let sid = super::digitalize::correct_id(v.mult_value.max(0) as usize);
                let name = stat_labels::name(&sid, 0);
                let divide = m == "CORRECT_TABLE_DIVIDE";
                let how = if divide { format!("{} ÷ {name}", num(x)) } else { format!("{} × {name}", num(x)) };
                match self.stats.get(&sid) {
                    Some(&s) => {
                        let r = if divide { if s != 0.0 { x / s } else { 0.0 } } else { x * s };
                        (r.trunc(), format!("{how} ({name} {})", num(s)), true)
                    }
                    None => (x, format!("{how}{NEEDS_STATS_NOTE}{name}"), false),
                }
            }
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
        if id.starts_with("NRA_") {
            // type 0 with value 1/2/3 = always null/reflect/drain, 100% (ActiveEntityState.cpp: "the NRA value becomes 100%
            // and CANNOT be reduced"); any other shape isn't read by comp_hack, so the server does something of its own
            if typ == 0 && (1..=3).contains(&(c.value as i32)) {
                return mk(format!("100% {}", stat_labels::name(id, c.value as i32 + 2)), None, false);
            }
            let element = stat_labels::name(id, 3).strip_prefix("Chance to Null ").map(str::to_string).unwrap_or_else(|| "an element".into());
            let mut e = mk(format!("{element} null/reflect/drain"), Some(value), false);
            e.cond = if cond == "inactive" { "inactive".into() } else { "situational".into() };
            e.cond_note = join(&e.cond_note, stat_labels::UNCOUNTED_NOTE);
            return e;
        }
        if id == "RES_STATUS" {
            // "Ailment Resistance +20" for -20; a plain number, not a percent
            let (label, shown) = stat_labels::displayed(id, 0, value);
            return mk(label, Some(shown), false);
        }
        if id.starts_with("BOOST_") || id.starts_with("RES_") {
            return mk(stat_labels::name(id, 0), Some(value), true);
        }
        mk(stat_labels::name(id, 0), Some(value), typ == 1 || gear_effects::is_pct_correct(id))
    }

    /// A race by the game's name ("Foul"), else its DevilData name in words.
    fn race_text(&self, race_id: i32) -> String {
        match self.data.demons() {
            Ok(d) => d.game_race_name(race_id).unwrap_or_else(|| crate::build::xml::title(&d.race_name(race_id))),
            Err(_) => "certain race".into(),
        }
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
            Some("EXPERTISE") => {
                // The two lowest digits are the expertise id, the rest the rank value (class x 10 + rank), compared with
                // the character's (TokuseiManager.cpp, TokuseiConditionType::EXPERTISE): 9035 = Pursuit at Class 9
                let id = c.value % 100;
                let want = c.value / 100;
                let name = self.data.expertises().ok().and_then(|e| json::str(e.at(&id.to_string()).at("name"))).unwrap_or_else(|| "a certain expertise".into());
                let rank = |r: i32| if r % 10 == 0 { format!("Class {}", r / 10) } else { format!("Class {} Rank {}", r / 10, r % 10) };
                // comparators: EQUALS, NOT_EQUAL, GTE, LTE (comp_hack libcomp\schema\tokusei.xml)
                let what = match c.cmp.as_str() {
                    "GTE" => format!("needs {} {name}", rank(want)),
                    "LTE" => format!("only up to {} {name}", rank(want)),
                    "NOT_EQUAL" => format!("not at {} {name}", rank(want)),
                    _ => format!("only at {} {name}", rank(want)),
                };
                if self.expertise_ranks.is_empty() {
                    return (None, what);
                }
                let have = self.expertise_ranks.get(&id).copied().unwrap_or(0);
                let met = match c.cmp.as_str() {
                    "GTE" => have >= want,
                    "LTE" => have <= want,
                    "NOT_EQUAL" => have != want,
                    _ => have == want,
                };
                (Some(met), format!("{what} (you have {}-{})", have / 10, have % 10))
            }
            // The partner (TokuseiManager.cpp, the PARTNER_* conditions): familiarity and Mitama type compared as numbers,
            // the demon (type = its id) and race matched
            Some("PARTNER_FAMILIARITY") => (
                None,
                match c.cmp.as_str() {
                    "GTE" => format!("with partner familiarity {}+", c.value),
                    "LTE" => format!("with partner familiarity up to {}", c.value),
                    "NOT_EQUAL" => format!("unless partner familiarity is {}", c.value),
                    _ => format!("with partner familiarity {}", c.value),
                },
            ),
            Some("PARTNER_MITAMA") => {
                let name = |v: i32| crate::calc::digitalize::MITAMA_NAMES.get(v as usize).filter(|n| !n.is_empty()).map(|n| format!("{n} Mitama"));
                (
                    None,
                    match (c.cmp.as_str(), c.value) {
                        ("GTE", 1) | ("NOT_EQUAL", 0) => "with a Mitama-fused partner".to_string(),
                        ("NOT_EQUAL", v) => format!("unless the partner is {}", name(v).unwrap_or_else(|| "a certain Mitama".into())),
                        (_, v) => format!("with a partner fused with {}", name(v).unwrap_or_else(|| "a certain Mitama".into())),
                    },
                )
            }
            Some(t @ ("PARTNER_TYPE" | "PARTY_DEMON_TYPE")) => {
                let demon = self.data.demons().ok().and_then(|d| d.devils.get(&c.value).map(|v| v.name.clone())).unwrap_or_else(|| "a certain demon".into());
                (
                    None,
                    match (t, eq) {
                        ("PARTNER_TYPE", true) => format!("with {demon} as partner"),
                        ("PARTNER_TYPE", false) => format!("unless {demon} is your partner"),
                        (_, true) => format!("with {demon} summoned in the party"),
                        _ => format!("unless {demon} is summoned in the party"),
                    },
                )
            }
            Some("PARTNER_RACE") => {
                let race = self.race_text(c.value);
                (None, if eq { format!("with a {race} partner") } else { format!("unless the partner is a {race}") })
            }
            Some("PARTNER_FAMILY") => (None, "with a partner of a certain family".into()),
            // the server switches these on and off with the world clock (value HHMM for the time, 0-15 for the moon)
            Some("GAME_TIME") => {
                let hm = format!("{:02}:{:02}", c.value / 100, c.value % 100);
                (
                    None,
                    match c.cmp.as_str() {
                        "GTE" => format!("from {hm} game time"),
                        "LTE" => format!("until {hm} game time"),
                        _ => format!("at {hm} game time"),
                    },
                )
            }
            Some("MOON_PHASE") => (
                None,
                match c.value {
                    0 => "at new moon".to_string(),
                    8 => "at full moon".to_string(),
                    v => format!("in moon phase {v} of 16"),
                },
            ),
            Some("STATUS_ACTIVE") => {
                let what = crate::data::game_data::status_target(c.value);
                (None, if eq { format!("while under {what}") } else { format!("unless under {what}") })
            }
            Some(t @ ("CURRENT_HP" | "CURRENT_MP")) => {
                // the percent of max HP/MP, rounded down (TokuseiManager.cpp, CURRENT_HP / CURRENT_MP)
                let what = if t == "CURRENT_HP" { "HP" } else { "MP" };
                let v = c.value;
                (
                    None,
                    match c.cmp.as_str() {
                        "GTE" => format!("at {v}% {what} or more"),
                        "LTE" => format!("at {v}% {what} or less"),
                        "NOT_EQUAL" => format!("unless at {v}% {what}"),
                        _ => format!("at {v}% {what}"),
                    },
                )
            }
            Some("EQUIPPED_WEAPON_TYPE") => {
                // The value is the weapon's item class (what the client calls "One-Handed Sword"), not a stat to show
                let what = crate::data::game_data::item_class_name(c.value).map(|n| format!("{n} equipped")).unwrap_or_else(|| "a certain weapon equipped".into());
                (None, if eq { what } else { format!("not {what}") })
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

    /// White Eternal Sleep's tokusei 1010001: STR 100, CORRECT_TABLE_DIVIDE by CorrectTbl 5 (LUCK), cut to a whole number.
    #[test]
    fn correct_table_divide_uses_the_stat() {
        let mut v = TokValue::new(Some("STR"), 0, 100.0, "CORRECT_TABLE_DIVIDE", 0);
        v.mult_value = 5;
        let fx = ItemEffects::new(Arc::new(GameData::new()), "NEUTRAL", 99, IndexMap::new());
        let (_, note, ok) = fx.scaled(&v);
        assert!(!ok && note.contains(NEEDS_STATS_NOTE), "{note}");
        let with = |v: &TokValue, luck: f64| fx.clone().with_stats(IndexMap::from([("LUCK".to_string(), luck)])).scaled(v);
        assert_eq!(with(&v, 30.0).0, 3.0);
        assert_eq!(with(&v, 57.0).0, 1.0);
        assert_eq!(with(&v, 101.0).0, 0.0);
        assert_eq!(with(&v, 0.0).0, 0.0);
        assert!(with(&v, 57.0).2);
        v.mult = "CORRECT_TABLE".into();
        v.value = 2.0;
        assert_eq!(with(&v, 57.0).0, 114.0);
    }

    fn crystal_effects(ranks: &[(i32, i32)], id: i32, tarot: bool) -> Vec<Effect> {
        let ranks: IndexMap<i32, i32> = ranks.iter().copied().collect();
        ItemEffects::new(Arc::new(GameData::new()), "NEUTRAL", 95, ranks).enchant(id, tarot, "Weapon").unwrap()
    }

    // The next three need the sealed data (data::pack) and are skipped in a build without its key. They check what the
    // game's own text also says.

    /// Ogami's Crystal as a soul fusion (enchant 3105, "of the Warmonger"): sealed tokusei. The game's text: "While
    /// Digitalized, Player/Demon's Final Critical rate correction +5%, Limit Break chance +5%, Limit Break power +5%. Player's
    /// Damage to Neutral enemies +20%." All of it now comes from the server data, nothing from the text.
    #[test]
    fn ogamis_crystal_soul_comes_from_the_server_data() {
        if !crate::data::pack::available() {
            return;
        }
        let list = crystal_effects(&[], 3105, false);
        assert!(list.iter().all(|e| !e.missing_data && e.cond != "unparsed" && e.source == "soul" && !e.cond_note.contains("text")), "{list:#?}");
        let find = |label: &str, target: &str| list.iter().filter(|e| e.label == label && e.target == target).collect::<Vec<_>>();
        // the player's and the digitalize demon's side, only while digitalized, +5% each, counted once
        for target in ["self", "partner"] {
            for label in ["Final Crit", "Limit Break Chance", "Limit Break Power"] {
                let hit = find(label, target);
                assert_eq!(hit.len(), 1, "{label} / {target}: {list:#?}");
                assert_eq!((hit[0].value, hit[0].pct, hit[0].cond.as_str()), (Some(5.0), true, "digitalized"), "{label} / {target}");
            }
        }
        // the damage depends on the enemy's alignment, so it is never counted in a total
        let damage: Vec<&Effect> = list.iter().filter(|e| e.cond_note.contains("NEUTRAL")).collect();
        assert!(!damage.is_empty(), "{list:#?}");
        assert!(damage.iter().all(|d| (d.value, d.cond.as_str(), d.target.as_str()) == (Some(20.0), "situational", "self")), "{damage:#?}");
    }

    /// Ogami's tarot: "If Player has Pursuit Class 9: Digitalize Duration +2%, Limit Break Cap +25, MAX HP +25". The server's
    /// condition is EXPERTISE 9035 GTE (expertise 35 at rank value 90 or more). With the character's ranks unknown the
    /// requirement stays a note; known, it is decided.
    #[test]
    fn a_tarot_with_a_class_requirement_follows_the_ranks() {
        let unknown = crystal_effects(&[], 3105, true);
        assert_eq!(unknown.len(), 3, "{unknown:#?}");
        assert!(unknown.iter().all(|e| e.cond == "situational" && e.cond_note.starts_with("needs Class 9 Pursuit") && e.tokusei == 2000191), "{unknown:#?}");
        let met = crystal_effects(&[(35, 90)], 3105, true);
        assert!(met.iter().all(|e| e.cond == "always" && e.value.is_some()), "{met:#?}");
        let short = crystal_effects(&[(35, 84)], 3105, true);
        assert!(short.iter().all(|e| e.cond == "inactive" && e.cond_note.contains("you have 8-4")), "{short:#?}");
    }

    /// Executioner (soul 3108, "STR +7, Blunt-based Skill effects +10%, For all Death-based skills, Death Boost +10% and Limit
    /// Break Cap + (STR x 1)"): one of its two tokusei is in the sealed data (the Death part), the other (363485) has none,
    /// so the text stands in for that one with only what the data doesn't already give: the data's Death +10% covers the
    /// text's "Death Boost +10%" (same stat), not its "Blunt-based Skill effects +10%" (same number, another stat), and the
    /// cap is not there twice.
    #[test]
    fn a_crystal_half_in_the_data_takes_the_rest_from_its_text() {
        if !crate::data::pack::available() {
            return;
        }
        let list = crystal_effects(&[], 3108, false);
        let from_text: Vec<(&str, f64)> = list.iter().filter(|e| e.tokusei == 363485 && e.value.is_some()).map(|e| (e.label.as_str(), e.value.unwrap())).collect();
        assert_eq!(from_text, vec![("STR", 7.0), ("Blunt-based Skill effects", 10.0)], "{list:#?}");
        assert_eq!(list.iter().filter(|e| e.label.contains("Limit Break Cap")).count(), 1, "{list:#?}");
        assert!(list.iter().all(|e| !e.missing_data), "{list:#?}");
    }

    /// A crystal whose effects are statuses and server-only aspects (enchant 3113): a status by the game's name, never a
    /// number, and what can't be worded stays an effect without data (a saved tooltip or the game's description may stand
    /// in), never the server's internal name.
    #[test]
    fn statuses_by_name_and_no_internal_names() {
        if !crate::data::pack::available() {
            return;
        }
        let list = crystal_effects(&[], 3113, false);
        assert!(list.iter().any(|e| e.label.starts_with("Inflicts ") && e.value.is_none() && !e.missing_data), "{list:#?}");
        assert!(list.iter().all(|e| !e.label.contains('_')), "{list:#?}");
    }
}

#[cfg(test)]
mod active_needs_tests {
    use super::*;

    #[test]
    fn switch_conditions_are_worded_by_their_state() {
        let radiant = ActiveNeeds { switches: vec![(5485, "Radiant Hands".into())], ..Default::default() };
        assert_eq!(radiant.wording(|_| None), "while Radiant Hands is on");
        assert_eq!(radiant.wording(|_| Some(true)), "while Radiant Hands is on");
        assert_eq!(radiant.wording(|_| Some(false)), "only while Radiant Hands is on, and it is off now: not counted");
        assert!(!radiant.undecided(), "a switch alone is decided by its state");
        // two switches: the ones that are off are named
        let two = ActiveNeeds { switches: vec![(1, "Einheri".into()), (2, "Gale Form".into())], ..Default::default() };
        assert_eq!(two.wording(|_| Some(true)), "while Einheri and Gale Form are on");
        assert_eq!(two.wording(|id| Some(id == 1)), "only while Gale Form is on, and it is off now: not counted");
        assert_eq!(two.wording(|_| Some(false)), "only while Einheri and Gale Form are on, and they are off now: not counted");
        // a passive or an unnamed tokusei can't be decided: the effect stays situational
        let mixed = ActiveNeeds { switches: vec![(1, "Einheri".into())], skills: vec!["Limit Break Up: Strength".into()], unknown: false };
        assert!(mixed.undecided());
        assert_eq!(mixed.wording(|_| Some(true)), "while Einheri is on, with Limit Break Up: Strength");
        let unknown = ActiveNeeds { unknown: true, ..Default::default() };
        assert!(unknown.undecided() && !unknown.is_empty());
        assert!(!unknown.wording(|_| None).is_empty());
        assert!(ActiveNeeds::default().is_empty());
        assert_eq!(and_list(&["A".to_string(), "B".to_string(), "C".to_string()]), "A, B and C");
    }

    /// The conditions are resolved through the skills that list the tokusei (public tables): Radiant Hands' 2100039 is a switch.
    #[test]
    fn conditions_resolve_to_the_skills_that_list_them() {
        let d = GameData::with_paths(None, None);
        let cond = |t: i32| (Some("ACTIVE_TOKUSEI".to_string()), t);
        let needs = ActiveNeeds::of(&d, &[cond(2100039), (Some("EXPLICIT_SKILL".to_string()), 5001), cond(2100039)]).unwrap();
        assert_eq!(needs.switches, vec![(5485, "Radiant Hands".to_string())], "listed once; other conditions are not its business");
        assert!(needs.skills.is_empty() && !needs.unknown);
        assert!(ActiveNeeds::of(&d, &[(Some("EXPLICIT_SKILL".to_string()), 5001)]).unwrap().is_empty());
        assert!(ActiveNeeds::of(&d, &[cond(999_999_999)]).unwrap().unknown, "no skill lists it");
    }
}
