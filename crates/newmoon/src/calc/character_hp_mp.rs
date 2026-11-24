//! Port of `src/Calc/CharacterHpMp.cs`.
//!
//! The character's Max HP / Max MP, normal and digitalized, following comp_hack CharacterState::RecalculateStats:
//! 1. Start from HP_MAX 70, MP_MAX 10 (GetCharacterBaseStats). The digitalize STR..LUCK go into the base stats first,
//!    so the window's digitalized STR/MAG/VIT/INT are what the formula sees.
//! 2. CalculateDependentStats (character: each part rounded half away from zero), with the FINAL STR/MAG/VIT/INT:
//!    HP = 70 + round(70 * 0.03 * level) + round(STR * 0.3) + round(1.2 * VIT)
//!    MP = 10 + round(10 * 0.03 * level) + round(MAG * 0.3) + round(0.6 * INT)
//! 3. Digitalized: + the demon's HP/MP at the stat rate (Digitalize.Result.Hp / Mp).
//! 4. AdjustStats (non-base): HP_MAX/MP_MAX percent (layer 1), then flat, then percent (layer 2). Flat sources include
//!    gear, expertise passives (Max MP Up) and other learned passives the reader doesn't count yet (e.g. "Additional HP
//!    Lv1-6", "Kindred Support", "Additional MP Lv1-3", "Noble Heart"), so the reader can't get the flat part from
//!    inputs alone: [`calibrate`] takes it from a normal reading instead.
//!
//! Checked on Rubito (Lv95, Specter Lv90 100%): normal STR 96 MAG 71 VIT 64 INT 154 -> formula 376 / 152, window
//! 702 / 291, so flat +326 / +139. Digitalized STR 235 MAG 195 VIT 165 INT 266 -> formula 539 / 258; + demon 1630 + 326
//! = 2495 = the window's Max HP. MP gives 258 + 653 + 139 = 1050, but the reported digitalized Max MP was 662: not
//! explained by any additive model (the demon would have to add at most 371), so that reading needs checking.
//!
//! Differences in shape from the C#, because the pieces it took are ported elsewhere:
//! - a `Digitalize.Result` is passed as its parts: `stats` (CorrectTbl index -> value), `effects`, and `hp` / `mp`
//!   ([`digitalized_with_result`], [`digitalize_only_stats`], [`digitalize_only_flat`]);
//! - `GameData.Passives` is the JSON object it holds (`passives`: keyed by the skill id as text);
//! - `Store.LastStatsPath` is the path itself ([`from_saved_stats`]).

use super::effect::Effect;
use crate::fmt;
use crate::Result;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub const BASE_HP: i32 = 70;
pub const BASE_MP: i32 = 10;

/// `(int)Math.Round(x, MidpointRounding.AwayFromZero)`
fn r(x: f64) -> i32 {
    fmt::round_away(x, 0) as i32
}

/// CalculateDependentStats for a character (HP/MP part) from the final STR/MAG/VIT/INT.
pub fn formula(level: i32, str_: i32, mag: i32, vit: i32, intel: i32) -> (i32, i32) {
    let (bh, bm) = (BASE_HP as f64, BASE_MP as f64);
    (
        BASE_HP.wrapping_add(r(bh * 0.03 * level as f64)).wrapping_add(r(str_ as f64 * 0.3)).wrapping_add(r((bh * 0.01 + 0.5) * vit as f64)),
        BASE_MP.wrapping_add(r(bm * 0.03 * level as f64)).wrapping_add(r(mag as f64 * 0.3)).wrapping_add(r((bm * 0.01 + 0.5) * intel as f64)),
    )
}

/// AdjustStats order for one of HP_MAX/MP_MAX: percent layer 1, flat, percent layer 2 (each truncated).
pub fn adjust(value: i32, flat: f64, pct1: f64, pct2: f64) -> i32 {
    let mut v = value as i64;
    if pct1 != 0.0 {
        v = if pct1 <= -100.0 { 0 } else { v.wrapping_add((v as f64 * (pct1 * 0.01)) as i32 as i64) };
    }
    v = v.wrapping_add(flat as i64);
    if pct2 != 0.0 {
        v = if pct2 <= -100.0 { 0 } else { v.wrapping_add((v as f64 * (pct2 * 0.01)) as i32 as i64) };
    }
    v.clamp(1, i32::MAX as i64) as i32
}

/// Max HP and MP from inputs: final STR/MAG/VIT/INT, flat and percent HP_MAX/MP_MAX, demon HP/MP (0 = normal).
#[allow(clippy::too_many_arguments)]
pub fn compute(level: i32, str_: i32, mag: i32, vit: i32, intel: i32, flat_hp: f64, flat_mp: f64, pct_hp: f64, pct_mp: f64, demon_hp: i32, demon_mp: i32) -> (i32, i32) {
    let (hp, mp) = formula(level, str_, mag, vit, intel);
    (adjust(hp.wrapping_add(demon_hp), flat_hp, pct_hp, 0.0), adjust(mp.wrapping_add(demon_mp), flat_mp, pct_mp, 0.0))
}

/// A normal (not digitalized) reading split into the formula part, percent HP_MAX/MP_MAX bonuses (layer 1, e.g. the
/// MP-lowering toggles, -10% each) and everything else (flat).
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    pub level: i32,
    pub str_: i32,
    pub mag: i32,
    pub vit: i32,
    pub int: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    pub formula_hp: i32,
    pub formula_mp: i32,
    pub pct_hp: f64,
    pub pct_mp: f64,
    /// The percent Max HP/MP of the toggles on now, when it differs from the readings (saved stats taken with
    /// other toggles); None = the same as the reading.
    pub now_pct_hp: Option<f64>,
    pub now_pct_mp: Option<f64>,
}

impl Calibration {
    /// Max HP minus the formula after percent bonuses: gear, expertise and other flat bonuses together.
    pub fn flat_hp(&self) -> i32 {
        self.max_hp.wrapping_sub(adjust(self.formula_hp, 0.0, self.pct_hp, 0.0))
    }
    /// Max MP minus the formula after percent bonuses.
    pub fn flat_mp(&self) -> i32 {
        self.max_mp.wrapping_sub(adjust(self.formula_mp, 0.0, self.pct_mp, 0.0))
    }
}

/// Takes the flat part from the character's normal Max HP/MP and final STR/MAG/VIT/INT (the window's values).
#[allow(clippy::too_many_arguments)]
pub fn calibrate(level: i32, str_: i32, mag: i32, vit: i32, intel: i32, max_hp: i32, max_mp: i32, pct_hp: f64, pct_mp: f64) -> Calibration {
    let (hp, mp) = formula(level, str_, mag, vit, intel);
    Calibration { level, str_, mag, vit, int: intel, max_hp, max_mp, formula_hp: hp, formula_mp: mp, pct_hp, pct_mp, now_pct_hp: None, now_pct_mp: None }
}

/// Max HP/MP while digitalized, from a normal reading: the formula on the stats plus the demon's STR..LUCK gains, plus
/// the demon's HP/MP (both before percent bonuses, as RecalculateStats adds them before the non-base AdjustStats),
/// then the same percent and flat parts, plus HP/MP bonuses that only work digitalized (digiFlatHp/Mp).
/// Checked: Rubito + Specter 2495 / 662 with the five MP-lowering toggles on (-50% MP).
#[allow(clippy::too_many_arguments)]
pub fn digitalized(normal: &Calibration, str_gain: i32, mag_gain: i32, vit_gain: i32, int_gain: i32, demon_hp: i32, demon_mp: i32, digi_flat_hp: i32, digi_flat_mp: i32) -> (i32, i32) {
    let (hp, mp) = formula(normal.level, normal.str_.wrapping_add(str_gain), normal.mag.wrapping_add(mag_gain), normal.vit.wrapping_add(vit_gain), normal.int.wrapping_add(int_gain));
    (
        adjust(hp.wrapping_add(demon_hp), normal.flat_hp().wrapping_add(digi_flat_hp) as f64, normal.now_pct_hp.unwrap_or(normal.pct_hp), 0.0),
        adjust(mp.wrapping_add(demon_mp), normal.flat_mp().wrapping_add(digi_flat_mp) as f64, normal.now_pct_mp.unwrap_or(normal.pct_mp), 0.0),
    )
}

/// The same, with the gains and the demon's HP/MP taken from a digitalize result: `stats` (CorrectTbl index -> value),
/// `effects`, and `hp` / `mp` (the result's `Hp` / `Mp`).
pub fn digitalized_with_result(normal: &Calibration, stats: &BTreeMap<i32, i32>, effects: &[Effect], hp: i32, mp: i32, digi_flat_hp: i32, digi_flat_mp: i32) -> (i32, i32) {
    // The demon's stats at the digitalize rate, plus flat STR/MAG/VIT/INT from the digitalize effects (force stack
    // "Intelligence +1", passives): base adjustments, so they feed the formula too. Checked: Rubito + Shikigami
    // (force stack INT +1) 1317 / 453; without the +1 the MP came out 452.
    let extra = digitalize_only_stats(effects);
    let g = |i: i32| stats.get(&i).copied().unwrap_or(0).wrapping_add(extra.get(&i).copied().unwrap_or(0));
    digitalized(normal, g(0), g(1), g(2), g(3), hp, mp, digi_flat_hp, digi_flat_mp)
}

// ---- the few loosely-typed JSON reads (Data.Json) this file does ----

/// `Json.Num`
fn json_num(n: Option<&Value>) -> Option<f64> {
    match n? {
        Value::Number(x) => x.as_f64(),
        Value::String(s) => fmt::parse_f64(s),
        _ => None,
    }
}

/// `Json.Int(n)`: `(int)d`, 0 when there is no number.
fn json_int(n: Option<&Value>) -> i32 {
    json_num(n).map(|d| d as i32).unwrap_or(0)
}

/// `Json.Str`
fn json_str(n: Option<&Value>) -> Option<String> {
    match n? {
        Value::String(s) => Some(s.clone()),
        Value::Number(x) => x.as_f64().map(fmt::g),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        _ => None,
    }
}

/// Percent HP_MAX / MP_MAX of the given switched-on skills (e.g. -10% MP for each MP-lowering toggle).
/// `passives` is `GameData.Passives` (skill id as text -> the passive's JSON); `stat` is "HP_MAX" or "MP_MAX".
pub fn toggle_pct(passives: &serde_json::Map<String, Value>, on: &[i32], stat: &str) -> f64 {
    let mut total = 0.0f64;
    for id in on {
        let mut inner = 0.0f64;
        if let Some(Value::Array(tbl)) = passives.get(&id.to_string()).and_then(|p| p.get("correctTbl")) {
            for c in tbl.iter().filter(|c| !c.is_null()) {
                if json_str(c.get("id")).as_deref() == Some(stat) && json_int(c.get("type")) == 1 {
                    inner += json_num(c.get("value")).unwrap_or(0.0);
                }
            }
        }
        total += inner;
    }
    total
}

/// A calibration from a normal reading (level, the 12 base/bonus pairs, Max HP/MP). The percent part is that of the
/// toggles on when it was read (toggles_then); predictions use the toggles on now (toggles_now). None when the reading
/// is too short or has no Max HP.
pub fn calibrate_from_reading(
    passives: &serde_json::Map<String, Value>,
    level: i32,
    pairs: &[i32],
    max_hp: i32,
    max_mp: i32,
    toggles_then: &[i32],
    toggles_now: &[i32],
) -> Option<Calibration> {
    if pairs.len() < 8 || max_hp <= 0 {
        return None;
    }
    let s = |k: usize| pairs[2 * k].wrapping_add(pairs[2 * k + 1]);
    let mut cal = calibrate(level, s(0), s(1), s(2), s(3), max_hp, max_mp, toggle_pct(passives, toggles_then, "HP_MAX"), toggle_pct(passives, toggles_then, "MP_MAX"));
    cal.now_pct_hp = Some(toggle_pct(passives, toggles_now, "HP_MAX"));
    cal.now_pct_mp = Some(toggle_pct(passives, toggles_now, "MP_MAX"));
    Some(cal)
}

/// The calibration from the last saved normal stats (cache\last-stats.json, written by every snapshot that could read
/// them), with the toggles saved alongside (older saves: the toggles of now). None when nothing is saved.
/// `last_stats_path` is `Store.LastStatsPath`; `toggles_now` None is a null list. Returns (calibration, taken_at).
pub fn from_saved_stats(last_stats_path: &Path, passives: &serde_json::Map<String, Value>, toggles_now: Option<&[i32]>) -> Result<(Option<Calibration>, Option<String>)> {
    if !last_stats_path.exists() {
        return Ok((None, None));
    }
    let text = std::fs::read_to_string(last_stats_path)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let last: Value = serde_json::from_str(text)?;
    let pairs: Vec<i32> = match last.get("pairs") {
        Some(Value::Array(a)) => a.iter().filter(|x| !x.is_null()).map(|p| json_int(Some(p))).collect(),
        _ => Vec::new(),
    };
    let then: Vec<i32> = match last.get("toggles_on") {
        Some(Value::Array(ta)) => ta.iter().map(|n| json_int(Some(n))).collect(),
        _ => toggles_now.map(|t| t.to_vec()).unwrap_or_default(),
    };
    let now: Vec<i32> = toggles_now.map(|t| t.to_vec()).unwrap_or_default();
    Ok((
        calibrate_from_reading(passives, json_int(last.get("level")), &pairs, json_int(last.get("max_hp")), json_int(last.get("max_mp")), &then, &now),
        json_str(last.get("taken_at")),
    ))
}

/// Flat STR/MAG/VIT/INT (CorrectTbl 0-3) from the digitalize result's counted effects, not its own stats.
pub fn digitalize_only_stats(effects: &[Effect]) -> HashMap<i32, i32> {
    let mut res: HashMap<i32, i32> = HashMap::new();
    for e in effects {
        if !e.counted() || e.pct || e.value.is_none() || e.cond_note.starts_with("stats") {
            continue;
        }
        let i = match e.label.as_str() {
            "Strength" | "STR" => 0,
            "Magic" | "MAG" => 1,
            "Vitality" | "VIT" => 2,
            "Intelligence" | "INT" => 3,
            _ => -1,
        };
        if i >= 0 {
            let cur = res.get(&i).copied().unwrap_or(0);
            res.insert(i, cur.wrapping_add(e.value.unwrap_or(0.0) as i32));
        }
    }
    res
}

/// Flat Max HP / Max MP from the digitalize result's own counted effects (demon passives, set effects).
pub fn digitalize_only_flat(effects: &[Effect]) -> (i32, i32) {
    let (mut hp, mut mp) = (0.0f64, 0.0f64);
    for e in effects {
        if !e.counted() || e.pct || e.value.is_none() || e.cond_note.starts_with("stats") {
            continue;
        }
        if e.label == "Max HP" {
            hp += e.value.unwrap_or(0.0);
        } else if e.label == "Max MP" {
            mp += e.value.unwrap_or(0.0);
        }
    }
    (hp as i32, mp as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formula_matches_the_checked_reading() {
        // Rubito Lv95, normal STR 96 MAG 71 VIT 64 INT 154 -> formula 376 / 152 (see the module comment)
        assert_eq!(formula(95, 96, 71, 64, 154), (376, 152));
        // digitalized STR 235 MAG 195 VIT 165 INT 266 -> 539 / 258
        assert_eq!(formula(95, 235, 195, 165, 266), (539, 258));
    }

    #[test]
    fn adjust_truncates_and_clamps() {
        assert_eq!(adjust(1000, 100.0, 0.0, 0.0), 1100);
        assert_eq!(adjust(1000, 0.0, -10.0, 0.0), 900);
        assert_eq!(adjust(1000, 5.9, 12.5, 3.3), 1000 + 125 + 5 + ((1130.0f64 * 0.033) as i32));
        assert_eq!(adjust(10, -50.0, 0.0, 0.0), 1);
        assert_eq!(adjust(10, 0.0, -100.0, 0.0), 1);
    }

    #[test]
    fn calibration_and_digitalized() {
        let c = calibrate(95, 96, 71, 64, 154, 702, 291, 0.0, 0.0);
        assert_eq!((c.flat_hp(), c.flat_mp()), (326, 139));
        let (hp, _) = digitalized(&c, 139, 124, 101, 112, 1630, 0, 0, 0);
        assert_eq!(hp, 2495);
        assert_eq!(compute(95, 96, 71, 64, 154, 326.0, 139.0, 0.0, 0.0, 0, 0), (702, 291));
    }

    #[test]
    fn toggles_and_effects() {
        let passives = json!({"5": {"correctTbl": [{"id": "MP_MAX", "type": 1, "value": -10}, {"id": "HP_MAX", "type": 2, "value": 5}]}});
        let map = passives.as_object().unwrap();
        assert_eq!(toggle_pct(map, &[5, 5, 6], "MP_MAX"), -20.0);
        assert_eq!(toggle_pct(map, &[5], "HP_MAX"), 0.0);
        let fx = vec![
            Effect { label: "INT".into(), value: Some(1.0), ..Default::default() },
            Effect { label: "Max HP".into(), value: Some(50.9), ..Default::default() },
            Effect { label: "Max HP".into(), value: Some(7.0), cond_note: "stats of the demon".into(), ..Default::default() },
        ];
        assert_eq!(digitalize_only_stats(&fx)[&3], 1);
        assert_eq!(digitalize_only_flat(&fx), (50, 0));
    }
}
