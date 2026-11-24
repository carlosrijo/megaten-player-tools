//! Port of `src/Calc/Digitalize.cs`.
//!
//! What digitalizing with one demon gives the character, following the server (comp_hack CharacterState::Digitalize):
//! - Stats: rate% of the demon's base STR..MP and CLS-R..Support. Base = its type's table values + growth by level +
//!   rebirth boosts + Mitama bonuses (no demon force, no familiarity). Rate = 10%, or 30% for a Mitama-fused type,
//!   plus the race's STAT_RATE assists, at most 100%. CLS-R..Support are the table values, not the ones the
//!   window shows (those add STR/SPD/MAG/INT parts the digitalize formula leaves out).
//! - The race's other learned assists add: the slot skills (PASSIVES: passive ones; ACTIVES: usable active ones),
//!   the type's traits (TRAITS), the Mitama sets' effects and their stat parts (MITAMA_SET), affinities (AFFINITIES),
//!   demon force stack effects (FORCE_STACK), longer digitalize time (EXTEND_TIME), shorter wait (REDUCE_WAIT).
//!
//! Which assists are learned is the character's choice (a 256-byte mask the server sends); it isn't read from memory
//! yet, so callers pass it (None = unknown: assist parts are listed but not counted).
//!
//! The C# `Digitalize.Result` is [`DigitalizeResult`]; `character_hp_mp` takes its parts (`stats`, `effects`, `hp()`, `mp()`).

// The checks keep the C# shape (nested ifs, index loops) so each line can be compared with the original.
#![allow(clippy::collapsible_if, clippy::needless_range_loop, clippy::type_complexity, clippy::too_many_arguments, clippy::manual_range_contains)]

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use indexmap::IndexSet;

use super::effect::Effect;
use super::gear_effects;
use super::item_effects::ItemEffects;
use crate::Result;
use crate::data::demon_tables::{DemonTables, Devil, MitamaSet};
use crate::data::game_data::{GameData, int_parse};
use crate::data::json::{self, At};
use crate::data::stat_labels;
use crate::fmt;
use crate::game::DemonRecord;

// CorrectTbl indexes
const STR: usize = 0;
const LUCK: usize = 5;
const HP: usize = 6;
const MP: usize = 7;
const CLSR: usize = 12;
const SUPPORT: usize = 15;

static CORRECT_IDS: [&str; 22] = [
    "STR", "MAGIC", "VIT", "INT", "SPEED", "LUCK", "HP_MAX", "MP_MAX", "HP_REGEN", "MP_REGEN", "MOVE1", "MOVE2", "CLSR", "LNGR", "SPELL", "SUPPORT", "CRITICAL", "SUMMON_SPEED",
    "KNOCKBACK_RESIST", "PDEF", "MDEF", "CRIT_DEF",
];

/// The CorrectTbl id of an index (`CorrectTbl N` beyond the known ones).
pub fn correct_id(i: usize) -> String {
    CORRECT_IDS.get(i).map(|s| s.to_string()).unwrap_or_else(|| format!("CorrectTbl {i}"))
}

pub static RUNE_NAMES: [&str; 12] = ["Tiwaz", "Peorth", "Eoh", "Eihwaz", "Uruz", "Hagalaz", "Laguz", "Ansuz", "Nauthiz", "Ingwaz", "Sigel", "Wyrd"];
pub static MITAMA_NAMES: [&str; 5] = ["", "Ara", "Nigi", "Kusi", "Saki"];

/// Affinities in CorrectTbl order (RES_DEFAULT = 26 ... RES_SUICIDE = 48; NRA_ the same from 49, then PHYS, MAGIC).
static AFF_NAMES: [&str; 23] = [
    "All", "Physical", "Slash", "Thrust", "Blunt", "Gun", "Pierce", "Spread", "Fire", "Ice", "Electric", "Almighty", "Force", "Expel", "Death", "Heal", "Support", "Mystic", "Nerve", "Mind", "Word",
    "Special", "Suicide",
];

static RE_LB_CAP: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"LB Cap\s*\+\s*(\d+)").expect("LB cap pattern"));

/// C# `Digitalize.Result`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DigitalizeResult {
    pub demon: DemonRecord,
    pub devil: Option<Devil>,
    pub growth: i32,
    pub base_rate: i32,
    pub rate: i32,
    pub max_rate: i32,
    /// The demon's base values (126, CorrectTbl order) the rate is applied to.
    pub base: Vec<i32>,
    /// Stats the character gets: CorrectTbl index -> value (STR..MP_MAX, CLSR..SUPPORT).
    pub stats: BTreeMap<i32, i32>,
    pub effects: Vec<Effect>,
    pub active_skills: Vec<i32>,
    /// Mitama sets active by the server rule (GetMitamaBonuses), all of them, tokusei ones included.
    pub sets: Vec<MitamaSet>,
    /// Assist types counted (None = the learned assists are unknown).
    pub assists: Option<IndexSet<String>>,
    pub extend_time: i32,
    pub reduce_wait: i32,
    /// The demon's HP_MAX/MP_MAX before CalculateDependentStats, and right after it (before Mitama mode 2).
    pub pre_hp: i32,
    pub pre_mp: i32,
    pub dependent_hp: i32,
    pub dependent_mp: i32,
    pub notes: Vec<String>,
}

impl DigitalizeResult {
    /// What the character's Max HP/MP get from the demon (added after the character's own formula).
    pub fn hp(&self) -> i32 {
        self.stats.get(&(HP as i32)).copied().unwrap_or(0)
    }

    pub fn mp(&self) -> i32 {
        self.stats.get(&(MP as i32)).copied().unwrap_or(0)
    }
}

/// C# `Digitalize(GameData data, ItemEffects fx)`.
pub struct Digitalize {
    data: Arc<GameData>,
    fx: ItemEffects,
}

impl Digitalize {
    pub fn new(data: Arc<GameData>, fx: ItemEffects) -> Digitalize {
        Digitalize { data, fx }
    }

    /// assists: the learned assist ids (any race; only the demon's race counts), or None when unknown.
    /// mitama_boost: the character has MITAMA_SET_BOOST (skill 5133), which upgrades Mitama set bonuses.
    pub fn compute(&self, d: &DemonRecord, assists: Option<&[i32]>, mitama_boost: bool) -> Result<DigitalizeResult> {
        let data = &*self.data;
        let t = data.demons()?;
        let mut res = DigitalizeResult { demon: d.clone(), ..Default::default() };
        if !t.loaded() {
            res.notes.push("The demon tables (reference\\devils-game.tsv etc.) are missing.".into());
            return Ok(res);
        }
        let Some(devil) = t.devils.get(&d.r#type) else {
            res.notes.push(format!("Demon type {} isn't in the demon table.", d.r#type));
            return Ok(res);
        };
        res.devil = Some(devil.clone());

        // Assists of the demon's race
        let learned: Option<Vec<&crate::data::demon_tables::Assist>> = assists.map(|a| a.iter().filter_map(|id| t.assists.get(id)).filter(|a| a.race_id == devil.race_id).collect());
        res.assists = learned.as_ref().map(|l| l.iter().map(|a| a.r#type.clone()).collect());
        let assist_set = res.assists.clone();
        let has = |ty: &str| assist_set.as_ref().is_some_and(|s| s.contains(ty));
        let sum_of = |l: &Option<Vec<&crate::data::demon_tables::Assist>>, ty: &str| -> i32 {
            l.as_ref().map_or(0, |l| l.iter().filter(|a| a.r#type == ty).fold(0i32, |acc, a| acc.wrapping_add(a.value)))
        };
        // DIGITALIZE_STAT_RATES from the server's constants.xml: "normal,Mitama-fused"
        let rates: Vec<i32> = data.server_constant("DIGITALIZE_STAT_RATES")?.unwrap_or_default().split(',').map(|x| fmt::parse_i32(x.trim()).unwrap_or(-1)).collect();
        if rates.len() >= 2 && rates.iter().all(|&v| v >= 0) {
            res.base_rate = if devil.is_mitama() { rates[1] } else { rates[0] };
        } else {
            res.base_rate = if devil.is_mitama() { 30 } else { 10 }; // comp_hack's default, only when constants.xml can't be read
            res.notes.push("The server's constants.xml wasn't found (data\\config.json \"serverConstants\"): base rate is comp_hack's default.".into());
        }
        res.rate = 100.min(res.base_rate.wrapping_add(sum_of(&learned, "STAT_RATE")));
        res.max_rate = 100.min(res.base_rate.wrapping_add(t.assists_of_race(devil.race_id).iter().filter(|a| a.r#type == "STAT_RATE").fold(0i32, |acc, a| acc.wrapping_add(a.value))));
        res.extend_time = sum_of(&learned, "EXTEND_TIME");
        res.reduce_wait = sum_of(&learned, "REDUCE_WAIT");
        let mitama_set = has("MITAMA_SET");

        // ---- Stats (GetDemonBaseStats, AdjustDemonBaseStats, AdjustMitamaStats, CalculateDependentStats) ----
        let mut s = vec![0i32; 126];
        for (i, v) in devil.correct.iter().take(126).enumerate() {
            s[i] = *v;
        }
        res.growth = if d.growth_type != 0 { d.growth_type } else { devil.growth_type };
        match t.growth.get(&res.growth) {
            Some(g) => grow(&mut s, g, d.level),
            None => res.notes.push(format!("Growth type {} isn't in the growth table: stats use the type's base values only.", res.growth)),
        }
        rebirth(&mut s, &d.reunion);
        let sets = mitama_sets_of(d, t, false);
        res.sets = sets.clone();
        mitama_stats(&mut s, d, t, 1, mitama_set, mitama_boost);
        res.pre_hp = s[HP];
        res.pre_mp = s[MP];
        dependent_hp_mp(&mut s, d.level);
        res.dependent_hp = s[HP];
        res.dependent_mp = s[MP];
        mitama_stats(&mut s, d, t, 2, mitama_set, mitama_boost);
        for i in (STR..=MP).chain(CLSR..=SUPPORT) {
            res.stats.insert(i as i32, (((res.rate as f64 * 0.01) * s[i] as f64) as i32) as i16 as i32);
        }
        let stat_src = format!("{} ({}% of its stats)", d.name, res.rate);
        for (&i, &v) in &res.stats {
            if v != 0 {
                res.effects.push(tag(Effect { label: gear_effects::correct_label_of(&correct_id(i as usize)), value: Some(v as f64), ..Default::default() }, "stats", &stat_src, None));
            }
        }

        // Client cross-check: its active-set flags should match the server rule
        let shown: Vec<i32> = (0..48).filter(|&i| d.mitama_sets_shown.get(i as usize).copied().unwrap_or(false)).collect();
        let mut mine: Vec<i32> = sets.iter().map(|x| x.id).filter(|&id| id < 48).collect();
        mine.sort();
        if d.mitama_type > 0 && shown != mine {
            let j = |v: &[i32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
            res.notes.push(format!("Mitama sets: the client shows [{}], the server rule gives [{}].", j(&shown), j(&mine)));
        }

        // ---- Skills, traits, Mitama set effects ----
        for &id in d.skills.iter().filter(|&&x| x > 0) {
            let (cat, restricted) = t.skills.get(&id).copied().unwrap_or((-1, false));
            if restricted {
                continue;
            }
            if cat == 0 {
                self.skill_effects(&mut res, id, "passive", if has("PASSIVES") { None } else { Some("PASSIVES") })?;
            } else if cat == 1 && has("ACTIVES") {
                res.active_skills.push(id);
            }
        }
        // Traits (GetTraitSkills): demon equipment worn in a trait slot replaces that slot's trait with the item's skill
        // (DevilEquipmentItemData); an item missing from the table leaves the slot empty, as on the server.
        for i in 0..4usize {
            let equip = d.equipment.as_ref().and_then(|e| e.get(i)).copied().unwrap_or(0);
            if equip > 0 {
                match t.equipment_skill.get(&equip) {
                    Some(&eq_skill) => self.skill_effects(&mut res, eq_skill, "equipment", if has("TRAITS") { None } else { Some("TRAITS") })?,
                    None => res.effects.push(tag(
                        Effect { label: format!("Demon equipment {equip}: not in the server data"), cond: "unparsed".into(), ..Default::default() },
                        "equipment",
                        &format!("{} equipment", d.name),
                        None,
                    )),
                }
            } else if i < devil.traits.len() && devil.traits[i] > 0 {
                self.skill_effects(&mut res, devil.traits[i], "trait", if has("TRAITS") { None } else { Some("TRAITS") })?;
            }
        }
        if d.mitama_type > 0 {
            for set in &sets {
                let b = if mitama_boost { &set.bonus_ex } else { &set.bonus };
                let mut i = 0;
                while i + 1 < b.len() {
                    if b[i] == -1 && b[i + 1] != 0 {
                        for e in self.set_tokusei(b[i + 1], if mitama_boost { &set.ex_description } else { &set.description })? {
                            res.effects.push(tag(e, "mitama set", &format!("{} Mitama set {}", d.name, set_name(set.id)), if mitama_set { None } else { Some("MITAMA_SET") }));
                        }
                    }
                    i += 2;
                }
            }
        }
        // Demon Force stack (CharacterState::Digitalize, FORCE_STACK assist): each stack entry's DevilBoostExtraData
        // tokusei go to the digitalized character. An effect aimed at the character's partner has nobody to apply to
        // while digitalized. Demon Force values are not used: digitalizing takes the base stats (AdjustDemonBaseStats
        // with baseCalc), which skip them.
        for &sid in d.force_stack.as_deref().unwrap_or(&[]) {
            if sid <= 0 {
                continue;
            }
            let item = format!("{} force stack {sid}", d.name);
            let Some(toks) = t.force_stack_tokusei.get(&sid) else {
                res.effects.push(tag(Effect { label: format!("Force stack {sid}: not in the server data"), cond: "unparsed".into(), ..Default::default() }, "force stack", &item, None));
                continue;
            };
            for &tid in toks {
                for mut e in self.fx.tokusei(tid, "digitalize", &item)? {
                    if e.target != "self" && (e.cond == "always" || e.cond == "situational") {
                        e.cond = "inactive".into();
                        e.cond_note = "for a partner: none while digitalized".into();
                    }
                    res.effects.push(tag(e, "force stack", &item, if has("FORCE_STACK") { None } else { Some("FORCE_STACK") }));
                }
            }
        }
        // Affinities: the demon's resistances (RES_*, damage reduction %) and natural null/reflect/absorb
        // (NRA_*, stored as chance% * 10 + kind: 1 null, 2 reflect, 3 absorb) are added to the character's.
        let aff_needs = if has("AFFINITIES") { None } else { Some("AFFINITIES") };
        let aff_item = format!("{} affinities", d.name);
        // Names are the game's (StatLabels: "Fire Resist", "Chance to Reflect Fire"); the element of index i is StatLabels.ElementIds[i].
        let elem = |i: usize| -> Option<&'static str> {
            if i < stat_labels::ELEMENT_IDS.len() {
                stat_labels::ELEMENT_IDS[i]
            } else if i == AFF_NAMES.len() {
                Some("PHYS")
            } else {
                Some("MAGIC")
            }
        };
        for i in 0..AFF_NAMES.len() {
            if s[26 + i] != 0 {
                let label = match elem(i) {
                    Some(el) => stat_labels::name(&format!("RES_{el}"), 0),
                    None => format!("{} Resistance", AFF_NAMES[i]),
                };
                res.effects.push(tag(Effect { label, value: Some(s[26 + i] as f64), pct: true, ..Default::default() }, "affinity", &aff_item, aff_needs));
            }
        }
        for i in 0..AFF_NAMES.len() + 2 {
            let v = s[49 + i];
            if v <= 0 || !(1..=3).contains(&(v % 10)) {
                continue;
            }
            // NRA_PHYS / NRA_MAGIC: every non-Almighty hit whose damage is CLS-R/LNG-R-based / Spell/Support-based (SkillManager::SetNRA)
            // kind 1/2/3 here = CorrectTbl NRA type 3/4/5 (null/reflect/drain)
            let label = match elem(i) {
                Some(el) => stat_labels::name(&format!("NRA_{el}"), v % 10 + 2),
                None => format!("Chance to {} {}", ["Null", "Reflect", "Drain"][((v % 10) - 1) as usize], AFF_NAMES[i]),
            };
            res.effects.push(tag(Effect { label: format!("{}% {label}", v / 10), ..Default::default() }, "affinity", &aff_item, aff_needs));
        }
        res.base = s;
        Ok(res)
    }

    /// A Mitama set's tokusei. Some are NewMoon-only (e.g. 2000078, the boosted Wyrd 3 "LB Cap+500") and missing from
    /// the tokusei data; for those the set's own description gives the LB cap number.
    fn set_tokusei(&self, tid: i32, description: &str) -> Result<Vec<Effect>> {
        let list = self.fx.tokusei(tid, "digitalize", "")?;
        if !list.iter().all(|e| e.missing_data) {
            return Ok(list);
        }
        let Some(m) = RE_LB_CAP.captures(description) else { return Ok(list) };
        Ok(vec![Effect {
            label: "LB Damage Cap".into(),
            value: Some(int_parse(&m[1])? as f64),
            tokusei: tid,
            cond_note: format!("from the set description (tokusei {tid} isn't in the data)"),
            ..Default::default()
        }])
    }

    fn skill_effects(&self, res: &mut DigitalizeResult, id: i32, kind: &str, needs: Option<&str>) -> Result<()> {
        let from = res.effects.len();
        self.add_skill_effects(res, id, kind, needs)?;
        for e in &mut res.effects[from..] {
            e.skill_id = id; // for the skill's icon on the page
        }
        Ok(())
    }

    fn add_skill_effects(&self, res: &mut DigitalizeResult, id: i32, kind: &str, needs: Option<&str>) -> Result<()> {
        let data = &*self.data;
        let name = match data.skill_names()?.get(&id) {
            Some(n) => n.clone().unwrap_or_default(),
            None => format!("skill {id}"),
        };
        let item = format!("{} {kind} {name}", res.demon.name);
        let p = data.passives()?.get(&id.to_string()).filter(|v| !v.is_null());
        // A trait with no effect data at all (crafting assists like Moon Goddess, Earth Mother Assist, Swordsmith ones) has
        // no stat effect: shown with the game's own description (reference/traits-unresolved.tsv, client data).
        if p.is_none() {
            if let Some(desc) = data.trait_description(id)? {
                res.effects.push(tag(Effect { label: format!("{name}: no stat effect ({desc})"), cond: "inactive".into(), ..Default::default() }, kind, &item, None));
                return Ok(());
            }
        }
        let Some(p) = p else {
            res.effects.push(tag(Effect { label: format!("{name}: no effect data"), cond: "unparsed".into(), ..Default::default() }, kind, &item, needs));
            return Ok(());
        };
        for c in json::arr(p.at("correctTbl")) {
            let e = self.fx.correct(&json::str(c.at("id")).unwrap_or_default(), json::int(c.at("type")), json::num(c.at("value")).unwrap_or(0.0), "always", "digitalize", &item);
            res.effects.push(tag(e, kind, &item, needs));
        }
        for tn in json::arr(p.at("tokuseiIds")) {
            let tid = json::int(Some(tn));
            if tid != 0 {
                for e in self.fx.tokusei(tid, "digitalize", &item)? {
                    res.effects.push(tag(e, kind, &item, needs));
                }
            }
        }
        Ok(())
    }
}

/// "Tiwaz 3" style name of a Mitama set (4 per rune).
pub fn set_name(id: i32) -> String {
    if id / 4 < RUNE_NAMES.len() as i32 && id >= 0 {
        format!("{} {}", RUNE_NAMES[(id / 4) as usize], id % 4 + 1)
    } else {
        format!("#{id}")
    }
}

/// Everything here applies only while digitalized; a part whose assist isn't learned (or isn't known) isn't counted.
fn tag(mut e: Effect, part: &str, item: &str, needs_assist: Option<&str>) -> Effect {
    e.source = "digitalize".into();
    e.item = item.into();
    e.cond_note = if e.cond_note.is_empty() { part.to_string() } else { format!("{part}; {}", e.cond_note) };
    if e.cond == "always" {
        e.cond = "digitalized".into();
    }
    if let Some(needs) = needs_assist {
        // Not learned (or not known): the character doesn't get it
        if e.cond == "digitalized" || e.cond == "situational" {
            e.cond = "inactive".into();
        }
        e.cond_note.push_str(&format!("; needs the {needs} assist"));
    }
    e
}

// ---------- server stat steps ----------

/// GetDemonBaseStats: growth table stage A for boost levels 1-6, B 7-11, C 12-16, D 17+ (boost level = (level+3)/4).
fn grow(s: &mut [i32], g: &[Vec<i32>], level: i32) {
    let boost = level.wrapping_add(3) / 4;
    let stage = (boost - 1) / 5;
    let mut add = |k: usize, n: i32| {
        if k < g.len() && n > 0 {
            for i in 0..6 {
                s[STR + i] = s[STR + i].wrapping_add(g[k].get(i).copied().unwrap_or(0).wrapping_mul(n));
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

/// AdjustDemonBaseStats (base calculation: no demon force): rank r gives (r-1) boosts per rune.
fn rebirth(s: &mut [i32], ranks: &[i32]) {
    for i in 0..12usize.min(ranks.len()) {
        let b = ranks[i] - 1;
        if b <= 0 {
            continue;
        }
        match i {
            0 => s[CLSR] += 4 * b,
            1 => s[14] += 4 * b,
            2 => s[13] += 4 * b,
            3 => s[SUPPORT] += 4 * b,
            4 => {
                s[CLSR] += 2 * b;
                s[13] += 2 * b;
            }
            5 => {
                s[CLSR] += 2 * b;
                s[14] += 2 * b;
            }
            6 => {
                s[13] += 2 * b;
                s[14] += 2 * b;
            }
            7 => s[20] += 5 * b, // Ansuz: MDEF
            8 => s[19] += 5 * b, // Nauthiz: PDEF
            9 => {}              // Ingwaz: magnetite cost reduction
            10 => {
                for k in STR..=LUCK {
                    s[k] += 3 * b;
                }
            }
            11 => {
                s[HP] += 40 * b;
                s[MP] += 10 * b;
            }
            _ => {}
        }
    }
}

/// GetMitamaBonuses: sets active when each Mitama kind fills enough slots of the set's rune.
fn mitama_sets_of(d: &DemonRecord, t: &DemonTables, exclude_tokusei: bool) -> Vec<MitamaSet> {
    let mut res = Vec::new();
    if d.mitama_type <= 0 || d.mitama_reunion.iter().all(|&b| b == 0) {
        return res;
    }
    let mut map = [[0i32; 4]; 12];
    for i in 0..96usize {
        let b = d.mitama_reunion.get(i).copied().unwrap_or(0);
        if b != 0 {
            map[i / 8][(b as usize / 32).min(3)] += 1;
        }
    }
    for set in &t.mitama_sets {
        let r = set.id / 4;
        if !(0..12).contains(&r) {
            continue;
        }
        let active = (0..4).all(|k| map[r as usize][k] >= set.requirements.get(k).copied().unwrap_or(0));
        if active && (!exclude_tokusei || (!set.bonus.is_empty() && set.bonus[0] >= 0)) {
            res.push(set.clone());
        }
    }
    res
}

/// AdjustMitamaStats: rank bonus (scaled by level), each slot's bonus, and (with includeSets) the sets' stat parts.
/// mode 1 keeps base stats (STR..LUCK), mode 2 the rest.
///
/// The boost (MITAMA_SET_BOOST) works the way comp_hack does it for a summoned demon: mode 1 (before the HP/MP
/// formula) takes the NORMAL set values; mode 2 takes the boosted values for non-base stats and adds the
/// (boosted - normal) delta for STR..LUCK. So the character gets the boosted STR..LUCK, but the demon's HP/MP come
/// from the normal-set VIT/INT. comp_hack's Digitalize instead passes the boost to mode 1 as well (and then adds the
/// delta again). Checked on Rubito + Specter (boosted Tiwaz 3 Spd+50 Int+15 Vit+15, normal Spd+20 Int+10 Vit+0):
/// SPD rose by 182 (152 normal + 30 delta; comp_hack's way gives 212), and Max HP by exactly the HP this gives (1630;
/// a boosted-VIT mode 1 gives 1688, 58 too many). See CharacterHpMp.
fn mitama_stats(s: &mut [i32], d: &DemonRecord, t: &DemonTables, mode: i32, include_sets: bool, boost: bool) {
    if d.mitama_type <= 0 {
        return;
    }
    let mut add: Vec<(i32, i32)> = Vec::new();
    if let Some(u) = t.union_bonus.get(&((d.mitama_type - 1) * 16 + d.mitama_rank)) {
        let mut i = 0;
        while i + 1 < u.len() {
            let bv = u[i + 1];
            if bv != 0 {
                let total = (1.25 * (bv as f64).powi(2) + bv as f64).ceil();
                let v = fmt::round_away(d.level as f64 / 99.0 * total, 0) as i32;
                if v != 0 {
                    add.push((u[i], v));
                }
            }
            i += 2;
        }
    }
    for &b in &d.mitama_reunion {
        if b != 0 {
            if let Some(&rb) = t.reunion_bonus.get(&(b as i32)) {
                if rb.1 != 0 {
                    add.push(rb);
                }
            }
        }
    }
    // Mitama sets come in only after the demon's HP/MP formula (mode 2), all of their values, base stats included:
    // boosted ones with MITAMA_SET_BOOST. So the character gets the boosted STR..LUCK, while the demon's own HP/MP
    // are worked out without any set bonus. Checked on Rubito + Specter (boosted Tiwaz 3: Spd+50 Int+15 Vit+15):
    // STR..LUCK gains exact; Max HP 2495 exact; Max MP 662 exact with the MP-lowering toggles on (demon MP 636,
    // not 653 as with normal set values in mode 1, nor 662 with boosted ones). comp_hack adds sets in mode 1 too.
    let mut sets: Vec<(i32, i32)> = Vec::new();
    if include_sets && mode == 2 {
        for set in mitama_sets_of(d, t, true) {
            let bonus = if boost { &set.bonus_ex } else { &set.bonus };
            let mut i = 0;
            while i + 1 < bonus.len() {
                if bonus[i] >= 0 && bonus[i + 1] != 0 {
                    sets.push((bonus[i], bonus[i + 1]));
                }
                i += 2;
            }
        }
    }
    let chosen = add.into_iter().filter(|a| (a.0 <= LUCK as i32) == (mode == 1)).chain(sets);
    for (typ, v) in chosen {
        if typ >= 0 && (typ as usize) < s.len() {
            s[typ as usize] = s[typ as usize].wrapping_add(v);
        }
    }
}

/// CalculateDependentStats, demon HP/MP part (each part rounded up).
fn dependent_hp_mp(s: &mut [i32], level: i32) {
    let c = |x: f64| x.ceil() as i32;
    let hp = s[HP]
        .wrapping_add(c(s[HP] as f64 * 0.03 * level as f64))
        .wrapping_add(c(s[STR] as f64 * 0.3))
        .wrapping_add(c((s[HP] as f64 * 0.01 + 0.5) * s[2] as f64));
    let mp = s[MP]
        .wrapping_add(c(s[MP] as f64 * 0.03 * level as f64))
        .wrapping_add(c(s[1] as f64 * 0.3))
        .wrapping_add(c((s[MP] as f64 * 0.01 + 0.5) * s[3] as f64));
    s[HP] = if hp < 0 { i32::MAX } else { hp };
    s[MP] = if mp < 0 { i32::MAX } else { mp };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(set_name(0), "Tiwaz 1");
        assert_eq!(set_name(5), "Peorth 2");
        assert_eq!(set_name(48), "#48");
        assert_eq!(correct_id(6), "HP_MAX");
        assert_eq!(correct_id(30), "CorrectTbl 30");
    }

    #[test]
    fn tag_marks_digitalized_and_missing_assist() {
        let e = tag(Effect { label: "x".into(), ..Default::default() }, "passive", "item", Some("PASSIVES"));
        assert_eq!((e.source.as_str(), e.cond.as_str(), e.cond_note.as_str()), ("digitalize", "inactive", "passive; needs the PASSIVES assist"));
        let e = tag(Effect::default(), "stats", "item", None);
        assert_eq!((e.cond.as_str(), e.cond_note.as_str()), ("digitalized", "stats"));
    }

    #[test]
    fn dependent_hp_mp_rounds_up() {
        let mut s = vec![0; 126];
        s[HP] = 100;
        s[STR] = 10;
        s[2] = 10;
        dependent_hp_mp(&mut s, 50);
        // 100 + ceil(150) + ceil(3) + ceil((1+0.5)*10)
        assert_eq!(s[HP], 100 + 150 + 3 + 15);
    }
}
