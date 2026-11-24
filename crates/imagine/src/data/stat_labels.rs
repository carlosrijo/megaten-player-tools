//! Port of `src/Data/StatLabels.cs`.
//!
//! The game's own name for each stat id (CorrectTbl names such as RATE_CLSR or KNOCKBACK_NULL, and aspects such as
//! TECH_ATTACK_RATE), as the client words it in its effect text. Kept in reference\stat-labels-game.tsv, made by
//! `imagine-reader labels`: every tokusei with client text (CMessageData_Charastic) whose server data is a plain
//! self effect lends the words of each line to the stat on that line, when the line's number equals the stat's value
//! ("Slash Affinity +5%" -> BOOST_SLASH = "Slash Affinity"; "100% Chance to Null Knockback" -> KNOCKBACK_NULL).
//! The most used wording wins.

use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

use indexmap::IndexMap;
use regex::Regex;

use super::game_data::{GameData, TokValue, reference_file, reference_file_on_disk};
use crate::Result;

const FILE: &str = "stat-labels-game.tsv";

static LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();

fn labels() -> &'static HashMap<String, String> {
    LABELS.get_or_init(|| {
        let mut d = HashMap::new();
        // C# would throw on an unreadable file; here the table stays empty
        if let Some(lines) = reference_file(FILE).and_then(|f| f.lines().ok()) {
            for line in &lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                if r.len() > 1 && !r[1].is_empty() {
                    d.insert(r[0].to_string(), r[1].to_string());
                }
            }
        }
        d
    })
}

/// The game's label for a stat id, or `None` when the client has no text naming it.
pub fn of(id: &str) -> Option<&'static str> {
    labels().get(id).map(String::as_str)
}

/// Stats the client never names: no client text has them alone with a number (MOVE1 and SUMMON_SPEED are only in
/// the server's custom tokusei). Everything else is named by the game (stat-labels-game.tsv).
fn no_game_text(id: &str) -> Option<&'static str> {
    match id {
        "MOVE1" => Some("Movement Speed"),
        "SUMMON_SPEED" => Some("Summon Speed"),
        // the server's own tokusei (Mitama and the like) lower or raise it; the client words it only as "Support Magic damage"
        "RATE_SUPPORT_TAKEN" => Some("Support Magic damage Taken"),
        _ => None,
    }
}

/// The name to show for a stat id: the game's wording, else the few names above, else the id itself.
/// `typ` tells null / reflect / drain apart for NRA_* (3 / 4 / 5).
pub fn name(id: &str, typ: i32) -> String {
    name_opt(Some(id), typ).unwrap_or_default()
}

/// `Name(id, type)` for an id that may be null (then the result is null too).
pub fn name_opt(id: Option<&str>, typ: i32) -> Option<String> {
    let id = id?;
    let nra = if id.starts_with("NRA_") { of(&format!("{id}:{typ}")) } else { None };
    Some(nra.or_else(|| of(id)).or_else(|| no_game_text(id)).unwrap_or(id).to_string())
}

pub fn has(id: &str) -> bool {
    of(id).is_some() || no_game_text(id).is_some()
}

/// An element as the game names it (SLASH -> Slash, STRIKE -> Blunt, CURSE -> Death, LNGR -> Handgun), from
/// its affinity's wording ("Blunt Affinity"); `None` when the game has no text for it.
pub fn element(element: &str) -> Option<String> {
    of(&format!("BOOST_{element}")).and_then(|l| l.strip_suffix(" Affinity")).map(str::to_string)
}

/// The elements in the game's affinity order (CorrectTbl RES_* from index 28, the skill affinity numbers
/// from 2, AFFINITY_CAP_MAX's Value): 2 SLASH ... 22 SUICIDE. 0 and 1 are not single elements.
pub static ELEMENT_IDS: [Option<&str>; 23] = [
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

/// An element by its affinity number (see [`ELEMENT_IDS`]; 1 = the weapon's own) as the game names it.
pub fn element_by_number(n: i32) -> Option<String> {
    if n == 1 {
        of("BOOST_WEAPON").and_then(|w| w.strip_suffix(" Affinity")).map(str::to_string)
    } else if n >= 0 && (n as usize) < ELEMENT_IDS.len() {
        ELEMENT_IDS[n as usize].and_then(element)
    } else {
        None
    }
}

// "Slash Affinity +5%", "HP Regen + 100%", "Close-Range -50"
static TRAILING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<label>[A-Za-z][A-Za-z0-9 '’()./&-]*?)\s*(?P<sign>[+-])\s*(?P<num>\d+(?:\.\d+)?)(?P<pct>%?)$").unwrap()
});
// "100% Chance to Null Knockback", "10% Chance of nulling skill interruption when receiving damage"
// "0.5% chance of surviving a lethal blow"
static CHANCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<num>\d+(?:\.\d+)?)%\s+(?P<label>[Cc]hance (?:to|of) [A-Za-z][A-Za-z ,/'-]*)$").unwrap());
static COND_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(When|If)\b.*,$").unwrap());

/// `Regex.Replace(s, @"^[^\x00-\x7F]+(?=[A-Za-z])", "")`: drops the Japanese word a translation left in front of a name.
fn strip_leading_non_ascii(s: &str) -> &str {
    let run = s.char_indices().find(|(_, c)| c.is_ascii()).map(|(i, _)| i).unwrap_or(s.len());
    if run > 0 && s[run..].chars().next().is_some_and(|c| c.is_ascii_alphabetic()) { &s[run..] } else { s }
}

/// The value as the text shows it: Precision 1 = tenths (CLENCH_CHANCE 5 = 0.5%).
fn shown(v: &TokValue) -> f64 {
    v.value / 10f64.powi(v.precision)
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.001
}

/// Builds reference\stat-labels-game.tsv from the client text and the server tokusei data (written next to
/// reference\tokusei-client-text.tsv, so that file must be a real file on disk). Returns the number of stats and the path written.
pub fn generate(data: &GameData) -> Result<(usize, std::path::PathBuf)> {
    let text_path = reference_file_on_disk("tokusei-client-text.tsv")
        .ok_or_else(|| anyhow::Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "reference\\tokusei-client-text.tsv not found")))?;
    let text = super::json::read_all_text(&text_path)?;
    // stat -> label -> (count, example tokusei)
    let mut votes: IndexMap<String, IndexMap<String, (i32, i32)>> = IndexMap::new();
    for line in super::json::lines(&text) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = line.split('\t').collect();
        if r.len() < 3 {
            continue;
        }
        let Some(id) = crate::fmt::parse_i32(r[0]) else { continue };
        if r[2].is_empty() {
            continue;
        }
        let Some(def) = data.tokusei_full(id)? else { continue };
        if def.from_client_text || def.target != "SELF" || !def.skill_conditions.is_empty() || def.target_conditions {
            continue;
        }
        let values: Vec<&TokValue> = def.correct.iter().chain(def.aspects.iter()).collect();
        if values.is_empty() || values.iter().any(|v| v.mult != "NONE") {
            continue;
        }
        // One stat per text line; notes ("※ ...") and blank lines are not stats. A leading Japanese word the
        // translation left ("通常Movement Speed + 20%": normal movement) is not part of the name.
        let mut lines: Vec<String> = r[2]
            .split("\\n")
            .map(|s| s.trim_matches(&[' ', '\u{3000}', '\t'][..]))
            .filter(|s| !s.is_empty() && !s.starts_with('※'))
            .map(|s| strip_leading_non_ascii(s).to_string())
            .collect();
        // A condition line ("When HP is 2 or higher,") goes with the tokusei's conditions; other conditions are
        // not worded apart, so those tokusei are skipped
        let cond_line = !lines.is_empty() && COND_LINE.is_match(&lines[0]);
        if cond_line {
            lines.remove(0);
        }
        if (!def.conditions.is_empty() && !cond_line) || lines.len() != values.len() {
            continue;
        }
        for (i, line) in lines.iter().enumerate() {
            let v = values[i];
            let mut label: Option<String> = None;
            if let Some(m) = TRAILING.captures(line) {
                let n = crate::fmt::parse_f64(&m["num"]).unwrap_or(0.0) * if &m["sign"] == "-" { -1.0 } else { 1.0 };
                if same(n, shown(v)) {
                    label = Some(m["label"].trim().to_string());
                }
            }
            if label.is_none() {
                if let Some(c) = CHANCE.captures(line) {
                    if same(crate::fmt::parse_f64(&c["num"]).unwrap_or(0.0), shown(v)) {
                        let l = &c["label"];
                        let mut ch = l.chars();
                        let first = ch.next().map(|f| f.to_ascii_uppercase()).unwrap_or(' ');
                        label = Some(format!("{first}{}", ch.as_str()));
                    }
                }
            }
            // AFFINITY_CAP_MAX names its element in Value (ItemEffects.AffinityCapLabel); NRA_* names null, reflect
            // or drain in Type (type 3/4/5), so those are kept per type ("NRA_FIRE:4")
            let (Some(label), Some(vid)) = (label.filter(|l| !l.is_empty()), v.id.as_deref()) else { continue };
            if vid == "AFFINITY_CAP_MAX" {
                continue;
            }
            let key = if vid.starts_with("NRA_") { format!("{vid}:{}", v.r#type) } else { vid.to_string() };
            let by_label = votes.entry(key).or_default();
            by_label.entry(label).and_modify(|e| e.0 += 1).or_insert((1, id));
        }
    }
    let out_path = text_path.parent().map(|p| p.join(FILE)).unwrap_or_else(|| FILE.into());
    let mut w = String::new();
    w.push_str("#stat id\tgame label (most used wording)\ttokusei texts with it\texample tokusei\tother wordings (count)\r\n");
    // OrderByDescending(count).ThenBy(label length).ThenBy(label, Ordinal) for each stat
    let ranked_by: IndexMap<&String, Vec<(&String, (i32, i32))>> = votes
        .iter()
        .map(|(k, v)| {
            let mut l: Vec<(&String, (i32, i32))> = v.iter().map(|(a, b)| (a, *b)).collect();
            l.sort_by(|x, y| y.1.0.cmp(&x.1.0).then(x.0.chars().count().cmp(&y.0.chars().count())).then(x.0.cmp(y.0)));
            (k, l)
        })
        .collect();
    // Two stats must not share a name: a reduction of damage taken is worded "Close-range Damage -5%" too, so the
    // stat with fewer texts for that wording takes its next one that no other stat uses ("Close-range Damage Taken").
    let mut chosen: IndexMap<&String, &String> = ranked_by.iter().map(|(k, v)| (*k, v[0].0)).collect();
    for (stat, ranked) in &ranked_by {
        let label = chosen[*stat];
        let rival = ranked_by
            .iter()
            .filter(|(o, _)| *o != stat && chosen[**o].to_uppercase() == label.to_uppercase())
            .map(|(_, v)| v[0].1.0)
            .max()
            .unwrap_or(-1);
        if rival < ranked[0].1.0 {
            continue;
        }
        let alt = ranked.iter().skip(1).find(|x| !chosen.values().any(|c| c.to_uppercase() == x.0.to_uppercase())).map(|x| x.0);
        if let Some(alt) = alt {
            chosen.insert(stat, alt);
        }
    }
    let mut stats: Vec<&&String> = ranked_by.keys().collect();
    stats.sort();
    for stat in stats {
        let ranked = &ranked_by[*stat];
        let best = ranked.iter().find(|x| x.0 == chosen[*stat]).expect("the chosen label is one of the ranked ones");
        let others = ranked.iter().filter(|x| x.0 != best.0).map(|k| format!("{} ({})", k.0, (k.1).0)).collect::<Vec<_>>().join("; ");
        w.push_str(&format!("{}\t{}\t{}\t{}\t{}\r\n", stat, best.0, (best.1).0, (best.1).1, others));
    }
    std::fs::write(&out_path, w)?;
    Ok((votes.len(), out_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_japanese_word() {
        assert_eq!(strip_leading_non_ascii("通常Movement Speed + 20%"), "Movement Speed + 20%");
        assert_eq!(strip_leading_non_ascii("通常 Movement"), "通常 Movement");
        assert_eq!(strip_leading_non_ascii("Movement"), "Movement");
    }
}

/// The client's stat labels as a [`crate::calc::bonuses::StatLabelSource`] (C# passed `StatLabels.Of` around).
#[derive(Debug, Clone, Copy, Default)]
pub struct GameStatLabels;

impl crate::calc::bonuses::StatLabelSource for GameStatLabels {
    fn of(&self, id: &str) -> Option<String> {
        of(id).map(str::to_string)
    }
}
