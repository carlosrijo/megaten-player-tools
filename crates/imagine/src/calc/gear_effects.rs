//! Port of `src/Calc/GearEffects.cs`.
//!
//! All effects of the worn gear: item tooltips for the items themselves (stats, characteristics, tarot, soul, mods),
//! and the game's set table for set bonuses and appearance bonuses (tooltips sometimes leave those out).
//!
//! Differences from C#, forced by the language: the data lookups can fail, so the methods return [`Result`]; `GameData`
//! is shared (`Arc`); the fusion parser's crystal test (`data.IsCrystal`) cannot report an error through the parser's
//! `bool` callback, so a data file that cannot be read counts as "not a crystal" there.

// The checks keep the C# shape (nested ifs, index loops) so each line can be compared with the original.
#![allow(clippy::collapsible_if, clippy::needless_range_loop, clippy::type_complexity, clippy::too_many_arguments, clippy::manual_range_contains)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use indexmap::IndexMap;

use super::effect::{Effect, rx};
use super::item_effects::ItemEffects;
use super::tooltip_parser::{TooltipInfo, TooltipParser};
use crate::Result;
use crate::data::game_data::GameData;
use crate::data::stat_labels;
use crate::fmt;
use crate::game::{SetRecord, WornItem};
use crate::text::ic_key;

/// A set (or one-piece appearance bonus) matched against the worn items.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetReport {
    pub set_id: i32,
    pub item_id: i32,
    pub pieces: String,
    pub worn: String,
    pub tokusei: String,
    pub active: bool,
    pub appearance: bool,
    pub fusion: bool,
    pub effects: Vec<Effect>,
    /// The item ids of a set's pieces, in the set's order; for a fusion set, its enchant ids (empty for sets read from tooltip text).
    pub piece_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GearResult {
    pub effects: Vec<Effect>,
    pub sets: Vec<SetReport>,
    pub items: Vec<TooltipInfo>,
    pub set_source: String,
    /// Where the items' own bonuses came from: "tooltips", "game data" (item records) or "game data + tooltips".
    pub item_source: String,
}

impl Default for GearResult {
    fn default() -> Self {
        GearResult { effects: Vec::new(), sets: Vec::new(), items: Vec::new(), set_source: "tooltips".into(), item_source: "tooltips".into() }
    }
}

/// A stat id as the game names it (StatLabels: the client's own wording).
pub fn correct_label_of(id: &str) -> String {
    stat_labels::name(id, 0)
}

/// Stats whose correct-table value is a percent whatever its type (`GearEffects.PctCorrect`).
pub fn is_pct_correct(id: &str) -> bool {
    matches!(
        id,
        "FINAL_CRIT_CHANCE"
            | "LB_CHANCE"
            | "LB_DAMAGE"
            | "RATE_CLSR"
            | "RATE_LNGR"
            | "RATE_SPELL"
            | "RATE_SUPPORT"
            | "RATE_CLSR_TAKEN"
            | "RATE_LNGR_TAKEN"
            | "RATE_SPELL_TAKEN"
            | "RATE_SUPPORT_TAKEN"
            | "RATE_PC_TAKEN"
            | "RATE_DEMON_TAKEN"
            | "COOLDOWN_TIME"
            | "RATE_XP"
            | "RATE_EXPERTISE"
    )
}

/// C# `GearEffects(string alignment, Dictionary<string,int> expertiseClasses, GameData data, List<SetRecord> setTable, Dictionary<int,int> expertiseRanks = null)`.
pub struct GearEffects {
    alignment: String,
    data: Arc<GameData>,
    set_table: Option<Vec<SetRecord>>,
    expertise_ranks: Option<IndexMap<i32, i32>>,
    // Tooltip-only mode keeps "With X:" / soul-stone lines as text conditions; game-data mode leaves them to the fusion-set table.
    parser: TooltipParser,
    fusion_parser: TooltipParser,
    /// the character's level for level-scaled values (set by `compute`)
    level: AtomicI32,
}

/// A map keyed like a C# `Dictionary<string, T>(StringComparer.OrdinalIgnoreCase)`.
type IcMap<T> = HashMap<String, T>;

impl GearEffects {
    pub fn new(
        alignment: &str,
        expertise_classes: &IndexMap<String, i32>,
        data: Arc<GameData>,
        set_table: Option<Vec<SetRecord>>,
        expertise_ranks: Option<IndexMap<i32, i32>>,
    ) -> GearEffects {
        let classes = || expertise_classes.iter().map(|(k, v)| (k.clone(), *v));
        let crystal_data = Arc::clone(&data);
        GearEffects {
            alignment: alignment.to_string(),
            parser: TooltipParser::new(alignment, classes(), None),
            fusion_parser: TooltipParser::new(alignment, classes(), Some(Box::new(move |n: &str| crystal_data.is_crystal(n).unwrap_or(false)))),
            data,
            set_table,
            expertise_ranks,
            level: AtomicI32::new(0),
        }
    }

    /// The same item calculator the gear uses (alignment, level of the last `compute`, expertise ranks).
    fn item_fx(&self) -> ItemEffects {
        ItemEffects::new(Arc::clone(&self.data), &self.alignment, self.level.load(Ordering::Relaxed), self.expertise_ranks.clone().unwrap_or_default())
    }

    /// One tokusei (effect id) as effects. Numbers come from the server data, read the same way as item effects
    /// (ItemEffects.Tokusei: skill types, level multipliers, alignment checks, server XML first).
    pub fn tokusei_effects(&self, tid: i32, item: &str, source: &str) -> Result<Vec<Effect>> {
        self.item_fx().tokusei(tid, source, item)
    }

    /// Sets touched by the worn ids. One-piece entries are appearance bonuses (Item = the tooltip name, Source "appearance").
    /// name_by_id: plain item names for set labels; base_by_id: full tooltip names for appearance effects.
    pub fn set_bonuses(&self, ids: &[i32], name_by_id: &HashMap<i32, String>, base_by_id: &HashMap<i32, String>) -> Result<Vec<SetReport>> {
        let worn: HashSet<i32> = ids.iter().copied().filter(|&i| i > 0).collect();
        let mut report = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let Some(set_table) = &self.set_table else { return Ok(report) };
        let join_ids = |v: &[i32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        for s in set_table {
            let need: Vec<i32> = s.pieces.iter().copied().filter(|&p| p != 0).collect();
            if need.is_empty() {
                continue;
            }
            let have: Vec<i32> = need.iter().copied().filter(|p| worn.contains(p)).collect();
            if have.is_empty() {
                continue;
            }
            if need.len() == 1 {
                let iid = need[0];
                let item_name = match base_by_id.get(&iid) {
                    Some(bn) if !bn.is_empty() => bn.clone(),
                    _ => self.data.item_name(iid)?.unwrap_or_else(|| format!("item {iid}")),
                };
                let mut r1 = SetReport {
                    set_id: s.id,
                    item_id: iid,
                    pieces: item_name.clone(),
                    worn: "1/1".into(),
                    active: true,
                    appearance: true,
                    tokusei: join_ids(&s.tokusei),
                    ..Default::default()
                };
                for &t in &s.tokusei {
                    r1.effects.extend(self.tokusei_effects(t, &item_name, "appearance")?);
                }
                report.push(r1);
                continue;
            }
            let mut nm = Vec::new();
            for p in &need {
                nm.push(match name_by_id.get(p) {
                    Some(n) if !n.is_empty() => n.clone(),
                    _ => self.data.item_name(*p)?.unwrap_or_else(|| format!("item {p}")),
                });
            }
            let active = have.len() == need.len();
            // Gendered copies of a set (same effects, other piece ids) can't both be worn; count the effects once anyway.
            let mut sorted = need.clone();
            sorted.sort();
            let key = format!("{}|{}", join_ids(&s.tokusei), join_ids(&sorted));
            if active && !seen.insert(key) {
                continue;
            }
            let label = format!("Set: {} (set {})", rx::rep(&nm[0], "[♂♀]", ""), s.id);
            let mut r = SetReport {
                set_id: s.id,
                pieces: nm.join(", "),
                worn: format!("{}/{}", have.len(), need.len()),
                active,
                tokusei: join_ids(&s.tokusei),
                piece_ids: need.clone(),
                ..Default::default()
            };
            for &t in &s.tokusei {
                r.effects.extend(self.tokusei_effects(t, &label, "set")?);
            }
            report.push(r);
        }
        Ok(report)
    }

    /// Plain item name: tooltip name without the tarot prefix ("X's ") and soul suffix (" of Y").
    fn plain_name(ti: &TooltipInfo) -> String {
        let mut plain = ti.base_name.clone();
        if let Some(tarot) = &ti.tarot {
            plain = rx::rep(&plain, &format!(r"^{}\s+", fancy_regex::escape(rx::rep(tarot, r"\s*Crystal$", "").trim())), "");
        }
        if ti.soul.is_some() {
            // the last " of " (a look-ahead): .NET pattern `\s+of\s+(?!.*\sof\s)[^()]+$`
            if let Ok(re) = fancy_regex::Regex::new(r"(?i)\s+of\s+(?!.*\sof\s)[^()]+$") {
                if let Ok(Some(m)) = re.find(&plain) {
                    plain.replace_range(m.range(), "");
                }
            }
        }
        plain
    }

    /// Enchant ids on the worn items: from memory when given (tarot[k] / soul[k]), otherwise from the crystal names
    /// on each tooltip's [Tarot Fusion] / [Soul Fusion] line.
    fn worn_enchants(&self, tooltips: &[String], ids: &[i32], tarot: Option<&[i32]>, soul: Option<&[i32]>) -> Result<HashSet<i32>> {
        let mut worn = HashSet::new();
        for k in 0..ids.len() {
            if ids[k] <= 0 {
                continue;
            }
            if let (Some(tarot), Some(soul)) = (tarot, soul) {
                if tarot.get(k).copied().unwrap_or(0) > 0 {
                    worn.insert(tarot[k]);
                }
                if soul.get(k).copied().unwrap_or(0) > 0 {
                    worn.insert(soul[k]);
                }
                continue;
            }
            if k >= tooltips.len() || tooltips[k].is_empty() {
                continue;
            }
            let ti = TooltipParser::info(&tooltips[k]);
            for n in [&ti.tarot, &ti.soul].into_iter().flatten() {
                worn.extend(self.data.enchant_ids_by_crystal(n)?);
            }
        }
        Ok(worn)
    }

    /// Fusion sets touched by the worn enchants: active only when every enchant in the set is worn.
    pub fn fusion_sets(&self, worn: &HashSet<i32>) -> Result<Vec<SetReport>> {
        let mut res = Vec::new();
        for s in self.data.enchant_sets()? {
            let have = s.effects.iter().filter(|e| worn.contains(e)).count();
            if have == 0 {
                continue;
            }
            let mut names = Vec::new();
            for &e in &s.effects {
                names.push(self.data.enchant_name(e)?);
            }
            let label = format!("Fusion set: {} (fusion set {})", names.join(" + "), s.id);
            let mut r = SetReport {
                set_id: s.id,
                pieces: names.join(", "),
                worn: format!("{have}/{}", s.effects.len()),
                active: have == s.effects.len(),
                fusion: true,
                tokusei: s.tokusei.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
                piece_ids: s.effects.clone(),
                ..Default::default()
            };
            for &t in &s.tokusei {
                r.effects.extend(self.tokusei_effects(t, &label, "fusion set")?);
            }
            for (typ, toks) in &s.conditions {
                for &t in toks {
                    for mut e in self.tokusei_effects(t, &label, "fusion set")? {
                        // Type 0 conditions are plain extra effects; others need something the reader can't check.
                        if *typ != 0 && e.cond != "unparsed" {
                            e.cond = "situational".into();
                            e.cond_note = format!("special condition {typ}");
                        }
                        r.effects.push(e);
                    }
                }
            }
            res.push(r);
        }
        Ok(res)
    }

    /// Effects of the worn gear. tooltips[k] and ids[k] describe slot k ("" / 0 for empty); tarot/soul are the slots'
    /// enchant ids from memory (optional). details[k] (optional) is slot k's item record: when given, that item's effects
    /// come from its ids and the server data (ItemEffects) instead of its tooltip; level is needed for level-scaled values.
    /// Without a set table (or ids) it falls back to the tooltip text for sets.
    /// tip_ok(slot, source) (optional; the simulator): false when the slot's tooltip no longer describes that part of the
    /// item (its tarot was swapped, say), so the tooltip must not stand in for it.
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        &self,
        tooltips: &[String],
        ids: Option<&[i32]>,
        tarot: Option<&[i32]>,
        soul: Option<&[i32]>,
        details: Option<&[Option<WornItem>]>,
        level: i32,
        tip_ok: Option<&dyn Fn(i32, &str) -> bool>,
    ) -> Result<GearResult> {
        self.level.store(level, Ordering::Relaxed);
        let mut res = GearResult { items: tooltips.iter().filter(|t| !t.is_empty()).map(|t| TooltipParser::info(t)).collect(), ..Default::default() };
        if let (Some(ids), Some(_), Some(details)) = (ids, &self.set_table, details) {
            let (tarot_v, soul_v): (Vec<i32>, Vec<i32>);
            let (tarot, soul) = if tarot.is_none() || soul.is_none() {
                tarot_v = details.iter().map(|d| d.as_ref().map_or(0, |d| d.tarot)).collect();
                soul_v = details.iter().map(|d| d.as_ref().map_or(0, |d| d.soul)).collect();
                (Some(&tarot_v[..]), Some(&soul_v[..]))
            } else {
                (tarot, soul)
            };
            let mut name_by_id = HashMap::new();
            let mut base_by_id = HashMap::new();
            let mut infos: Vec<Option<TooltipInfo>> = vec![None; ids.len()];
            for k in 0..ids.len() {
                if ids[k] <= 0 || k >= tooltips.len() || tooltips[k].is_empty() {
                    continue;
                }
                let ti = TooltipParser::info(&tooltips[k]);
                name_by_id.insert(ids[k], Self::plain_name(&ti));
                base_by_id.insert(ids[k], ti.base_name.clone());
                infos[k] = Some(ti);
            }
            res.sets = self.set_bonuses(ids, &name_by_id, &base_by_id)?;
            res.set_source = "game data".into();
            res.item_source = "game data".into();
            let item_fx = self.item_fx();
            let app_data = Self::appearance_sets(&res.sets); // an item can have several appearance entries
            let mut use_data: IcMap<bool> = HashMap::new();
            for k in 0..ids.len() {
                if ids[k] <= 0 {
                    continue;
                }
                let ti = infos[k].as_ref();
                let name = match ti {
                    Some(ti) => ti.base_name.clone(),
                    None => self.data.item_name(ids[k])?.unwrap_or_else(|| format!("item {}", ids[k])),
                };
                let mut tip_fx = match ti {
                    Some(ti) => self.fusion_parser.effects(ti, &res.items).0,
                    None => Vec::new(),
                };
                if let Some(tip_ok) = tip_ok {
                    tip_fx.retain(|e| tip_ok(k as i32, &e.source));
                }
                let has_tip_app = tip_fx.iter().any(|e| e.source == "appearance");
                let ds = app_data.get(&ic_key(&name));
                let data_ok = ds.is_some_and(|ds| !ds.iter().flat_map(|&x| &res.sets[x].effects).any(|e| e.cond == "unparsed"));
                let use_it = ds.is_some() && (data_ok || !has_tip_app);
                use_data.insert(ic_key(&name), use_it);
                if k < details.len() && details[k].is_some() {
                    let fx = item_fx.compute(details[k].as_ref(), k as i32, &name, if ti.is_some() { Some(tip_fx.as_slice()) } else { None })?;
                    res.effects.extend(fx);
                    // The tooltip is still the source for an appearance bonus the data can't read.
                    if !use_it {
                        res.effects.extend(tip_fx.iter().filter(|e| e.source == "appearance").cloned());
                    }
                } else {
                    res.item_source = "game data + tooltips".into();
                    for e in tip_fx {
                        if e.source == "appearance" && use_it {
                            continue;
                        }
                        res.effects.push(e);
                    }
                }
            }
            Self::add_active_sets(&mut res, &use_data);
            let fusion_sets = self.fusion_sets(&self.worn_enchants(tooltips, ids, tarot, soul)?)?;
            Self::add_fusion_sets(&mut res, fusion_sets);
            return Ok(res);
        }
        if let (Some(ids), Some(_)) = (ids, &self.set_table) {
            let mut name_by_id = HashMap::new();
            let mut base_by_id = HashMap::new();
            for k in 0..ids.len().min(tooltips.len()) {
                if ids[k] <= 0 || tooltips[k].is_empty() {
                    continue;
                }
                let ti = TooltipParser::info(&tooltips[k]);
                name_by_id.insert(ids[k], Self::plain_name(&ti));
                base_by_id.insert(ids[k], ti.base_name.clone());
            }
            res.sets = self.set_bonuses(ids, &name_by_id, &base_by_id)?;
            res.set_source = "game data".into();
            // Appearance comes from game data per item, unless the data has an effect we can't read and the tooltip shows the section.
            let app_data = Self::appearance_sets(&res.sets); // an item can have several appearance entries
            let mut use_data: IcMap<bool> = HashMap::new();
            let items = res.items.clone();
            for i in &items {
                let tip_fx = self.fusion_parser.effects(i, &items).0;
                let has_tip_app = tip_fx.iter().any(|e| e.source == "appearance");
                let ds = app_data.get(&ic_key(&i.base_name));
                let data_ok = ds.is_some_and(|ds| !ds.iter().flat_map(|&x| &res.sets[x].effects).any(|e| e.cond == "unparsed"));
                let use_it = ds.is_some() && (data_ok || !has_tip_app);
                use_data.insert(ic_key(&i.base_name), use_it);
                for e in tip_fx {
                    if e.source == "appearance" && use_it {
                        continue;
                    }
                    res.effects.push(e);
                }
            }
            Self::add_active_sets(&mut res, &use_data);
            let fusion = self.fusion_sets(&self.worn_enchants(tooltips, ids, tarot, soul)?)?;
            Self::add_fusion_sets(&mut res, fusion);
            return Ok(res);
        }

        // Fallback: set bonuses from the tooltip text
        let mut set_seen: HashSet<String> = HashSet::new();
        let items = res.items.clone();
        for i in &items {
            let (fx, sets) = self.parser.effects(i, &items);
            res.effects.extend(fx);
            for s in sets {
                let mut sorted: Vec<&String> = s.pieces.iter().collect();
                sorted.sort_by_key(|p| ic_key(p));
                let key = sorted.iter().map(|p| p.as_str()).collect::<Vec<_>>().join("|");
                if !set_seen.insert(key) {
                    continue;
                }
                let worn_count = s.pieces.iter().filter(|p| TooltipParser::piece_worn(p, &items)).count();
                res.sets.push(SetReport {
                    pieces: s.pieces.join(", "),
                    worn: format!("{worn_count}/{}", s.pieces.len()),
                    active: worn_count == s.pieces.len(),
                    effects: s.effects,
                    ..Default::default()
                });
            }
        }
        let mut effect_keys: HashSet<String> = HashSet::new();
        let mut added = Vec::new();
        for s in res.sets.iter_mut().filter(|s| s.active) {
            let ek = s.effects.iter().map(|e| format!("{}|{}|{}|{}", e.label, e.value.map(fmt::g).unwrap_or_default(), e.cond, e.target)).collect::<Vec<_>>().join(";");
            if !effect_keys.insert(ek) {
                s.active = false;
                continue;
            }
            // the effects are the same objects in the set and in the result: the item name goes on both
            let item = format!("Set: {}", rx::rep(s.pieces.split(", ").next().unwrap_or(""), "[♂♀]", ""));
            for e in s.effects.iter_mut() {
                e.item = item.clone();
                added.push(e.clone());
            }
        }
        res.effects.extend(added);
        Ok(res)
    }

    /// The appearance sets of a result by item name (case-insensitive), as indexes into `sets`.
    fn appearance_sets(sets: &[SetReport]) -> IcMap<Vec<usize>> {
        let mut m: IcMap<Vec<usize>> = HashMap::new();
        for (i, s) in sets.iter().enumerate() {
            if s.appearance {
                m.entry(ic_key(&s.pieces)).or_default().push(i);
            }
        }
        m
    }

    /// The effects of the active sets; an appearance bonus the tooltip supplies instead is switched off.
    fn add_active_sets(res: &mut GearResult, use_data: &IcMap<bool>) {
        for s in res.sets.iter_mut().filter(|s| s.active) {
            if s.appearance {
                if let Some(false) = use_data.get(&ic_key(&s.pieces)) {
                    s.active = false;
                    continue;
                }
            }
            res.effects.extend(s.effects.iter().cloned());
        }
    }

    fn add_fusion_sets(res: &mut GearResult, fusion_sets: Vec<SetReport>) {
        for s in fusion_sets.iter().filter(|s| s.active) {
            res.effects.extend(s.effects.iter().cloned());
        }
        res.sets.extend(fusion_sets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_correct_ids() {
        assert!(is_pct_correct("LB_CHANCE"));
        assert!(is_pct_correct("RATE_XP"));
        assert!(!is_pct_correct("LNGR"));
    }

    #[test]
    fn plain_name_drops_tarot_prefix_and_soul_suffix() {
        let ti = TooltipInfo {
            base_name: "Orthrus's Machete of Pixie".into(),
            tarot: Some("Orthrus's Crystal".into()),
            soul: Some("Pixie Crystal".into()),
            ..Default::default()
        };
        assert_eq!(GearEffects::plain_name(&ti), "Machete");
    }
}
