//! The Digitalize Planner page's data (app\ui\digi-planner.json): every race that has Digitalization Levels, with what
//! enables it, the points each level takes, the demons each level lets you digitalize, the passive learned at each level
//! and the two assists offered there; plus the demons that other races' levels let you digitalize. No character data:
//! the page takes the character's levels and points from expertise-ranks.json ("digitalize_levels").
//!
//! Inputs (reference\, all decoded from the server's BinaryData\Shield, see reference\README.md):
//!   guardian-levels.tsv   GuardianLevelData: points to the next level, assists, demons, extend skill per level
//!   guardian-unlock.tsv   GuardianUnlockData: the race:level pairs that enable a race (all of them)
//!   guardian-special.tsv  GuardianSpecialData: demons any one race:level pair lets you digitalize
//!   guardian-assist.tsv   GuardianAssistData + the client's text (CGuardianAssistData): the assists, the race names
//!   skill-text.tsv        the client's names and descriptions of the extend skills
//!   devils-game.tsv       demon names, races and base levels
//!
//! The rules the page follows are the server's (comp_hack CharacterManager::UpdateDigitalizePoints and the digitalize
//! check in SkillManager): points go to the race of the demon you are digitalized with; a race is enabled at level 1 once
//! every pair of its unlock row is reached; levels stop at 10, and at 5 while the demon is not Mitama-fused.

use std::path::Path;

use serde_json::{Value, json};

use crate::Result;
use crate::data::demon_tables::DemonTables;
use crate::data::game_data::GameData;
use crate::data::json::{self, Escape};

/// The highest Digitalization Level (CharacterManager::UpdateDigitalizePoints `levelCap`).
pub const LEVEL_CAP: i32 = 10;
/// The highest level a race reaches while digitalized with a demon that is not Mitama-fused.
pub const PLAIN_CAP: i32 = 5;

/// An extend skill as the page shows it: its name without the "DE[...]" wrapping, what it does (the description's
/// "・" lines, joined) and who gets it ("demon" when the lines start with "Demon's", else "you").
pub fn extend_skill(data: &GameData, id: i32) -> Result<(String, String, &'static str)> {
    let raw = data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("Skill {id}"));
    // "DE[Base Stat Boost]", and a few with the full-width brackets: "DE【Affinity Upper Limit Raise】"
    let name = raw
        .strip_prefix("DE[")
        .and_then(|s| s.strip_suffix(']'))
        .or_else(|| raw.strip_prefix("DE【").and_then(|s| s.strip_suffix('】')))
        .unwrap_or(&raw)
        .trim()
        .to_string();
    let desc = data.skill_text(id)?.unwrap_or_default();
    let lines: Vec<String> = desc
        .split('・')
        .skip(1)
        .map(|p| p.replace(['\n', '\u{3000}'], " ").split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect();
    let mut who = "you";
    let lines: Vec<String> = lines
        .into_iter()
        .map(|l| match l.strip_prefix("Demon's ") {
            Some(rest) => {
                who = "demon";
                rest.to_string()
            }
            None => l,
        })
        .collect();
    Ok((name, lines.join(" · "), who))
}

/// The races the planner covers: those the server lets you level (level 0's points > 0) that have points to earn.
pub fn planner_races(t: &DemonTables) -> Vec<i32> {
    t.digitalize_races().into_iter().filter(|r| t.levels.get(r).is_some_and(|l| l.len() > 1 && l[1].next_xp > 0)).collect()
}

/// The page data as JSON.
pub fn planner_json(data: &GameData) -> Result<Value> {
    let t = data.demons()?;
    let races = planner_races(t);
    // the points from level 1 to 10 of the cheapest race; every race costs a whole multiple of it (tier 1-5)
    let to_cap = |r: i32| -> i64 { t.levels[&r].iter().filter(|l| l.level >= 1 && l.level < LEVEL_CAP).map(|l| l.next_xp).sum() };
    let base = races.iter().map(|r| to_cap(*r)).filter(|v| *v > 0).min().unwrap_or(1);
    let mut out_races = Vec::new();
    for &r in &races {
        let lv = &t.levels[&r];
        let mut xp = Vec::new();
        let mut demons = Vec::new();
        let mut extend = Vec::new();
        let mut assists = Vec::new();
        for l in lv.iter().filter(|l| l.level >= 1 && l.level <= LEVEL_CAP) {
            if l.level < LEVEL_CAP {
                xp.push(json!(l.next_xp));
            }
            for &id in l.demon_ids.iter().filter(|&&id| id > 0) {
                if let Some(d) = t.devils.get(&id) {
                    demons.push(json!([id, d.name, l.level, d.base_level]));
                }
            }
            if l.extend_skill_id > 0 {
                let (name, text, who) = extend_skill(data, l.extend_skill_id)?;
                extend.push(json!([l.level, l.extend_skill_id, name, text, who]));
            }
            let choices: Vec<Value> = l.assists.iter().filter_map(|id| t.assists.get(id)).map(|a| json!([a.id, a.effect_text()])).collect();
            if !choices.is_empty() {
                assists.push(json!([l.level, choices]));
            }
        }
        // the race's side: what most of its demons are (the website's alignment of each demon)
        let mut sides = [0usize; 3];
        for d in t.devils.values().filter(|d| d.race_id == r) {
            match data.devil_alignment(d.id)? {
                Some("LAW") => sides[0] += 1,
                Some("NEUTRAL") => sides[1] += 1,
                Some("CHAOS") => sides[2] += 1,
                _ => {}
            }
        }
        let side = ["LAW", "NEUTRAL", "CHAOS"][(0..3).max_by_key(|&i| (sides[i], 2 - i)).unwrap_or(1)];
        let name = t.game_race_name(r).unwrap_or_else(|| super::xml::title(&t.race_name(r)));
        let unlock: Vec<Value> = t.unlocks.get(&r).map(|v| v.iter().map(|(a, b)| json!([a, b])).collect()).unwrap_or_default();
        out_races.push(json!({
            "id": r, "name": name, "side": side, "tier": ((to_cap(r) as f64) / (base as f64)).round() as i64,
            "unlock": unlock, "xp": xp, "demons": demons, "extend": extend, "assists": assists,
        }));
    }
    let mut special = Vec::new();
    for (&id, opts) in &t.specials {
        let Some(d) = t.devils.get(&id) else { continue };
        special.push(json!([id, d.name, d.race_id, d.base_level, opts.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>()]));
    }
    Ok(json!({
        "source": "The server's Digitalization tables (GuardianLevelData, GuardianUnlockData, GuardianSpecialData, GuardianAssistData) and the client's names and texts",
        "cap": LEVEL_CAP, "plainCap": PLAIN_CAP,
        "races": out_races, "special": special,
    }))
}

/// Writes app\ui\digi-planner.json.
pub fn build(data: &GameData, out_file: &Path) -> Result<()> {
    let v = planner_json(data)?;
    std::fs::write(out_file, json::serialize(&v, Escape::Relaxed, false))?;
    crate::console::write_line(&format!("Wrote {} ({} races, {} special demons).", out_file.display(), v["races"].as_array().map_or(0, |a| a.len()), v["special"].as_array().map_or(0, |a| a.len())));
    Ok(())
}
