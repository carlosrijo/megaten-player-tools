//! Port of `src/Calc/Bonuses.cs`.
//!
//! Which effect labels count toward which tracked stat, and how bonuses are named and grouped.
//!
//! Differences in shape from the C#, all because the pieces it used are ported elsewhere:
//! - `Data.StatLabels` is reached through the [`StatLabelSource`] trait (one method, `of`: the game's label for a stat id,
//!   `StatLabels.Of`); [`canonical`] takes it as an argument. The data module's `StatLabels` implements it with one line.
//! - `CombatSummary(GearResult gear)` is [`combat_summary`] over `gear.Effects` (a slice of effects).

use super::effect::{rx, Effect};
use crate::text::eq_ic;
use indexmap::IndexMap;
use std::cmp::Ordering;

/// Tracked combat stats (CSV column stem) and the labels that feed them.
pub const STAT_MAP: &[(&str, &str)] = &[
    ("Critical", "^Critical$"),
    ("Final critical correction", "^Final Crit(ical)?( Rate)?( Correction| Chance)?$"),
    ("Limit break chance", "^(Limit Break|LB) (Chance|Rate)$"),
    ("Limit break power", "^(Limit Break|LB) (Power|Damage)$"),
    ("Pursuit chance", "^Pursuit (Chance|Rate)$"),
    ("Pursuit power", "^Pursuit (Power|Damage)$"),
    ("Technical attack chance", "^(TA|Tech(nical)? Attack) (Chance|Rate)$"),
    ("Technical attack power", "^(TA|Tech(nical)? Attack) (Power|Damage)$"),
    ("Close-range damage dealt %", "^Close-Range Damage( Dealt)?$"),
    ("Long-range damage dealt %", "^Long-Range Damage( Dealt)?$"),
    ("Spell damage dealt %", "^(Spell|Magic) Damage( Dealt)?$"),
    ("Support damage dealt %", "^Support( Magic)? Damage( Dealt)?$"),
    ("Close-range damage taken %", "^Close-Range Damage (Taken|Suffered)$"),
    ("Long-range damage taken %", "^Long-Range Damage (Taken|Suffered)$"),
    ("Spell damage taken %", "^(Spell|Magic) Damage (Taken|Suffered)$"),
    ("Support damage taken %", "^Support( Magic)? Damage (Taken|Suffered)$"),
    ("Spin skill boost %", "^Spin( Skill)? (Damage|Power|Boost)$"),
    ("Attack skill boost %", "^Attack( Skill)? (Damage|Power|Boost)$"),
    ("Rush skill boost %", "^Rush( Skill)? (Damage|Power|Boost)$"),
    ("Rapid skill boost %", "^Rapid( Skill)? (Damage|Power|Boost)$"),
    ("Shot skill boost %", "^Shot( Skill)? (Damage|Power|Boost)$"),
    ("Cooldown %", "^Cooldown$"),
    ("Movement speed %", "^Movement Speed$"),
    ("Limit break cap", "^(Limit Break|LB)( Damage)? (Cap|Upper Limit)$"), // gear says "LB Damage Cap", tooltips "Limit Break Cap", expertise "Limit break cap"
];

/// The pattern of a tracked stat (C# `Pattern(stat)`; `First` throws for a stat that is not in the map: so does this).
pub fn pattern(stat: &str) -> &'static str {
    STAT_MAP.iter().find(|s| s.0 == stat).expect("a stat of Bonuses.STAT_MAP").1
}

pub const AFFINITY_RX: &str = r"^(Slash|Thrust|Strike|Blunt|Gun|Handgun|Pierce|Penetrate|Spread|Fire|Ice|Elec(tric)?|Force|Expel|Curse|Death|Mystic|Almighty|Nerve|Mind|Words? of Power|Word|Magic ?Force|Heal(ing)?|Curative|Support|Special|Suicide|Phys(ical)?)\b.*(Boost|Affinity)$";

/// What this file needs from `Data.StatLabels`.
pub trait StatLabelSource {
    /// `StatLabels.Of(id)`: the game's label for a stat id, or None when the client has no text naming it.
    fn of(&self, id: &str) -> Option<String>;
}

/// `StatLabels.ElementIds`: the elements in the game's affinity order.
pub const ELEMENT_IDS: [Option<&str>; 23] = [
    None,
    None,
    Some("SLASH"),
    Some("THRUST"),
    Some("STRIKE"),
    Some("LNGR"),
    Some("PIERCE"),
    Some("SPREAD"),
    Some("FIRE"),
    Some("ICE"),
    Some("ELEC"),
    Some("ALMIGHTY"),
    Some("FORCE"),
    Some("EXPEL"),
    Some("CURSE"),
    Some("HEAL"),
    Some("SUPPORT"),
    Some("MAGICFORCE"),
    Some("NERVE"),
    Some("MIND"),
    Some("WORD"),
    Some("SPECIAL"),
    Some("SUICIDE"),
];

/// `StatLabels.Element(element)`: an element as the game names it, from its affinity's wording ("Blunt Affinity").
fn element_name(labels: &dyn StatLabelSource, element: &str) -> Option<String> {
    let l = labels.of(&format!("BOOST_{element}"))?;
    l.strip_suffix(" Affinity").map(str::to_string)
}

/// Older or tooltip wordings of an element, to the game's name for it (StatLabels: "Blunt", "Handgun",
/// "Penetrate", "Curative", "Death", "Mystic", "Words of Power"). Parsing only; what is shown is the game's.
const ELEMENT_WORDS: &[(&str, &str)] = &[
    ("^Strike$", "STRIKE"),
    ("^(Gun|Long-?range)$", "LNGR"),
    ("^Pierce$", "PIERCE"),
    ("^Heal(ing)?$", "HEAL"),
    ("^Curse$", "CURSE"),
    ("^Magic ?Force$", "MAGICFORCE"),
    ("^Elec$", "ELEC"),
    ("^Words?$", "WORD"),
];

/// An element word ("Gun", "Handgun", "fire") as the game names it, or None when it is not an element.
fn game_element(labels: &dyn StatLabelSource, word: &str) -> Option<String> {
    for id in ELEMENT_IDS.iter().flatten() {
        if let Some(name) = element_name(labels, id) {
            if eq_ic(&name, word) {
                return Some(name);
            }
        }
    }
    for (p, id) in ELEMENT_WORDS {
        if rx::is(word, p) {
            return element_name(labels, id);
        }
    }
    None
}

const FLAT_STATS: &[(&str, &str)] = &[
    ("STR", "^(STR|Strength)$"),
    ("MAG", "^(MAG|Magic)$"),
    ("VIT", "^(VIT|Vitality)$"),
    ("INT", "^(INT|Intelligence)$"),
    ("SPD", "^(SPD|Speed)$"),
    ("LUCK", "^(LUCK|Luck)$"),
    ("CLS-R", "^(CLS-R|CLSR|Close-Range)$"),
    ("LNG-R", "^(LNG-R|LNGR|Long-Range)$"),
    ("Spell", "^Spell$"),
    ("Support", "^Support$"),
    ("P.Def", r"^(P\.Def|Physical Defense|Phys defense)$"),
    ("M.Def", r"^(M\.Def|Magical Defense|Magic Defense)$"),
    ("Max HP", "^(Max HP|HP Max)$"),
    ("Max MP", "^(Max MP|MP Max)$"),
    ("All stats", "^All parameters$"),
];

/// `char.ToUpperInvariant(s[0]) + s[1..]`: .NET upper-cases one char only, so a letter that would become two stays.
fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let mut up = c.to_uppercase();
            let first = match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            };
            let mut out = String::new();
            out.push(first);
            out.push_str(chars.as_str());
            out
        }
    }
}

/// One display name and group (Combat | Affinity | Stats | Other) per bonus, so gear and expertise lines meet.
pub fn canonical(label: &str, labels: &dyn StatLabelSource) -> (String, &'static str) {
    let l = label.trim();
    for (stat, r) in STAT_MAP {
        if rx::is(l, r) {
            return (rx::rep(stat, " %$", ""), "Combat");
        }
    }
    // One element's affinity (boost), its cap ("Ice Affinity Cap", ItemEffects.AffinityCapLabel) or its resist, under
    // the game's name for the element so every source meets: "Fire affinity", "Fire affinity cap", "Fire resist".
    // Affinities and caps go with the affinities, resists with Other (as before); none is a CSV column.
    if let Some(am) = rx::m(l, r"^(.+?)[\s-]*(affinity cap|affinity|boost|resist(ance)?)$") {
        let word = rx::rep(&am[1], r"\s+", " ");
        if let Some(el) = game_element(labels, word.trim()) {
            let kind = am[2].to_lowercase();
            return if kind == "affinity cap" {
                (format!("{el} affinity cap"), "Affinity")
            } else if kind.starts_with("resist") {
                (format!("{el} resist"), "Other")
            } else {
                (format!("{el} affinity"), "Affinity")
            };
        }
    }
    for (name, r) in FLAT_STATS {
        if rx::is(l, r) {
            return ((*name).to_string(), "Stats");
        }
    }
    // Wordings that mean the same stat meet under the game's name for it (StatLabels; tooltips word some differently)
    let g = |id: &str, fallback: &str| labels.of(id).unwrap_or_else(|| fallback.to_string());
    let o = if rx::is(l, r"^(EXP|XP|Experience)( Points)?\s+(Earned|Gain)$") {
        g("RATE_XP", "EXP Earned")
    } else if rx::is(l, "^Expertise( Points)? earned$") {
        g("RATE_EXPERTISE", "Expertise Earned")
    } else if rx::is(l, "^(Incantation|Cast time|CHANT_TIME)$") {
        g("CHANT_TIME", "Incantation")
    } else if rx::is(l, "^(HP regen|HP Regeneration)$") {
        g("HP_REGEN", "HP Regen")
    } else if rx::is(l, "^(MP regen|MP Regeneration)$") {
        g("MP_REGEN", "MP Regen")
    } else if rx::is(l, "^HP/MP Regeneration$") {
        "HP/MP Regen".to_string()
    } else if rx::is(l, "^RATE_HEAL_TAKEN$") {
        g("RATE_HEAL_TAKEN", "Curative Received") // the client's text: "Summoner Curative Received +20%" (tokusei 130138)
    } else if rx::is(l, "^Critical defense$") {
        g("CRIT_DEF", "Critical Defense")
    } else {
        upper_first(l)
    };
    (o, "Other")
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombatRow {
    pub stat: String,
    pub always: f64,
    pub digi: f64,
    pub sources: String,
}

/// What the digitalize demon adds to each tracked combat stat (its counted effects on the character).
/// The result has one entry per [`STAT_MAP`] stat, in that order.
pub fn demon_summary(fx: &[Effect]) -> IndexMap<String, f64> {
    let own: Vec<&Effect> = fx.iter().filter(|e| e.target == "self" && e.value.is_some() && e.counted()).collect();
    STAT_MAP
        .iter()
        .map(|(stat, p)| {
            let mut sum = 0.0f64;
            for e in own.iter().filter(|e| rx::is(&e.label, p)) {
                sum += e.value.unwrap_or(0.0);
            }
            ((*stat).to_string(), sum)
        })
        .collect()
}

/// `a.encode_utf16().cmp(b...)` of the upper-cased keys: `StringComparer.OrdinalIgnoreCase`.
fn cmp_ordinal_ic(a: &str, b: &str) -> Ordering {
    a.to_uppercase().encode_utf16().cmp(b.to_uppercase().encode_utf16())
}

/// Totals per tracked stat (normal and while digitalized), then affinity rows. Appearance bonuses count.
/// C# `CombatSummary(GearResult gear)`: `gear_effects` is `gear.Effects`.
pub fn combat_summary(gear_effects: &[Effect]) -> Vec<CombatRow> {
    let own: Vec<&Effect> = gear_effects.iter().filter(|e| e.target == "self" && e.value.is_some() && e.counted()).collect();
    let mut rows = Vec::new();
    let sum_of = |v: &[&Effect], cond: &str| {
        let mut s = 0.0f64;
        for e in v.iter().filter(|e| e.cond == cond) {
            s += e.value.unwrap_or(0.0);
        }
        s
    };
    for (stat, r) in STAT_MAP {
        let m: Vec<&Effect> = own.iter().copied().filter(|e| rx::is(&e.label, r)).collect();
        let a = sum_of(&m, "always");
        let d = sum_of(&m, "digitalized");
        let sources = m
            .iter()
            .map(|e| {
                format!(
                    "{} {}{}{}",
                    e.item,
                    calc_fmt::signed(e.value.unwrap_or(0.0)),
                    if e.pct { "%" } else { "" },
                    if e.cond == "digitalized" { " (digi)" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        rows.push(CombatRow { stat: (*stat).to_string(), always: a, digi: a + d, sources });
    }
    // GroupBy (first appearance of each key, keys compared ignoring case) then OrderBy the key (stable)
    let mut groups: Vec<(String, Vec<&Effect>)> = Vec::new();
    for e in own.iter().copied().filter(|e| rx::is(&e.label, AFFINITY_RX)) {
        let key = rx::rep(&rx::rep(&e.label, "-?affinity$", " Affinity"), r"\s+", " ").trim().to_string();
        match groups.iter_mut().find(|g| cmp_ordinal_ic(&g.0, &key) == Ordering::Equal) {
            Some(g) => g.1.push(e),
            None => groups.push((key, vec![e])),
        }
    }
    groups.sort_by(|a, b| cmp_ordinal_ic(&a.0, &b.0));
    for (key, g) in &groups {
        let a = sum_of(g, "always");
        let d = sum_of(g, "digitalized");
        rows.push(CombatRow { stat: format!("Affinity: {key}"), always: a, digi: a + d, sources: g.iter().map(|e| e.item.as_str()).collect::<Vec<_>>().join("; ") });
    }
    rows
}

/// C# `Fmt` (declared at the bottom of Bonuses.cs).
pub mod calc_fmt {
    use crate::fmt;

    /// `v.ToString(InvariantCulture)`
    pub fn n(v: f64) -> String {
        fmt::g(v)
    }

    /// `(v >= 0 ? "+" : "-") + Math.Abs(v).ToString("0.##")`
    pub fn signed(v: f64) -> String {
        format!("{}{}", if v >= 0.0 { "+" } else { "-" }, fmt::custom(v.abs(), "0.##"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Labels(HashMap<&'static str, &'static str>);
    impl StatLabelSource for Labels {
        fn of(&self, id: &str) -> Option<String> {
            self.0.get(id).map(|s| s.to_string())
        }
    }

    fn labels() -> Labels {
        Labels(HashMap::from([("BOOST_STRIKE", "Blunt Affinity"), ("BOOST_FIRE", "Fire Affinity"), ("BOOST_LNGR", "Handgun Affinity"), ("RATE_XP", "EXP Gain")]))
    }

    #[test]
    fn canonical_names() {
        let l = labels();
        assert_eq!(canonical("critical", &l), ("Critical".to_string(), "Combat"));
        assert_eq!(canonical("Cooldown", &l), ("Cooldown".to_string(), "Combat"));
        assert_eq!(canonical("Support Magic damage", &l), ("Support damage dealt".to_string(), "Combat"));
        assert_eq!(canonical("Support Magic damage Taken", &l), ("Support damage taken".to_string(), "Combat"));
        assert_eq!(canonical("Support", &l), ("Support".to_string(), "Stats"));
        assert_eq!(canonical("Fire Affinity", &l), ("Fire affinity".to_string(), "Affinity"));
        assert_eq!(canonical("Strike affinity cap", &l), ("Blunt affinity cap".to_string(), "Affinity"));
        assert_eq!(canonical("gun resist", &l), ("Handgun resist".to_string(), "Other"));
        assert_eq!(canonical("strength", &l), ("STR".to_string(), "Stats"));
        assert_eq!(canonical("EXP Earned", &l), ("EXP Gain".to_string(), "Other"));
        assert_eq!(canonical("foo bar", &l), ("Foo bar".to_string(), "Other"));
    }

    #[test]
    fn summaries() {
        let fx = vec![
            Effect { label: "Critical".into(), value: Some(2.0), item: "Ring".into(), source: "item".into(), ..Default::default() },
            Effect { label: "Critical".into(), value: Some(1.5), pct: true, cond: "digitalized".into(), item: "Hat".into(), ..Default::default() },
            Effect { label: "Fire Affinity".into(), value: Some(3.0), item: "Cape".into(), ..Default::default() },
            Effect { label: "Critical".into(), value: Some(9.0), cond: "inactive".into(), ..Default::default() },
        ];
        let rows = combat_summary(&fx);
        assert_eq!(rows[0], CombatRow { stat: "Critical".into(), always: 2.0, digi: 3.5, sources: "Ring +2; Hat +1.5% (digi)".into() });
        assert_eq!(rows.last().unwrap().stat, "Affinity: Fire Affinity");
        assert_eq!(demon_summary(&fx)["Critical"], 3.5);
        assert_eq!(calc_fmt::signed(-0.456), "-0.46");
    }
}
