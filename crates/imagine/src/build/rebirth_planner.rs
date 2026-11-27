//! The Rebirth Planner page's data (app\ui\rebirth-planner.json): the 12 rebirth runes with the growth type of each
//! rank and what a rebirth into it costs, every growth table (a demon's stats follow the growth type it ends on), the XP
//! per level (how many Magical Golden Apples a rank's level takes), and the Mitama tables: the rank bonus of each Mitama
//! kind, the random slot bonuses, the 48 sets and the Summon PG trades. No character data: the page takes a demon from
//! the Demon Database (demon-db.json) or from the COMP (the `compDemons` command, with its ranks and slots).
//!
//! Inputs (reference\, see reference\README.md):
//!   reunion-growth.tsv      DevilLVUpRateData, the rune growth types: rank, name, cost items, the client's rune text
//!   devil-growth.tsv        every growth type's stat gains (DemonTables)
//!   level-xp.tsv            comp_hack LEVEL_XP_REQUIREMENTS
//!   correct-table.tsv       CorrectTbl id -> name, for the stat ids the Mitama tables use
//!   mitama-union-bonus.tsv, mitama-reunion-bonus.tsv, mitama-set-bonus.tsv   the Mitama tables (DemonTables)
//!   mitama-pg.tsv           NPCBarterData: the Summon PG: <kind> Mitama trades
//!   items-game.tsv / the website's items.json   item names
//!
//! The rules the page follows are the server's (comp_hack CharacterManager::ReunionDemon, AdjustDemonBaseStats,
//! MitamaDemon, AdjustMitamaStats, GetMitamaBonuses, packets\game\MitamaReunion.cpp) and the public server data's
//! Vivian event (10,000 XP per apple); NewMoon may differ. Nothing here is checked in game yet.

use std::path::Path;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::Result;
use crate::calc::digitalize::MITAMA_NAMES;
use crate::data::game_data::{GameData, reference_lines};
use crate::data::json::{self, Escape};
use crate::data::stat_labels;

/// Conversion points a rank is worth when extracted (comp_hack FusionTables.cpp REUNION_RANK_POINTS: rank 0..9).
const RANK_POINTS: [i32; 10] = [0, 0, 1, 4, 10, 20, 40, 105, 320, 0];
/// Magical Golden Apple, and the XP one gives (the public datastore's Vivian event N3400: 100,000 XP per 10).
const APPLE_ITEM: i32 = 21941;
const APPLE_XP: i64 = 10000;
/// Rank bonus per rank above 1 (AdjustDemonBaseStats): (CorrectTbl id, amount) pairs per rune; Ingwaz is the
/// magnetite cut instead (5% per rank, capped at 100%).
const RUNE_BONUS: [&[(i32, i32)]; 12] = [
    &[(12, 4)],            // Tiwaz: CLS-R
    &[(14, 4)],            // Peorth: Spell
    &[(13, 4)],            // Fehu (Eoh): LNG-R
    &[(15, 4)],            // Eihwaz: Support
    &[(12, 2), (13, 2)],   // Uruz
    &[(12, 2), (14, 2)],   // Hagalaz
    &[(13, 2), (14, 2)],   // Laguz
    &[(20, 5)],            // Ansuz: M.Def
    &[(19, 5)],            // Nauthiz: P.Def
    &[],                   // Ingwaz: summon magnetite -5%
    &[(0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3)], // Sowilo (Sigel): every stat
    &[(6, 40), (7, 10)],   // Wyrd: Max HP / Max MP, before the HP/MP formula
];

fn rows(name: &str) -> Result<Vec<Vec<String>>> {
    let lines = reference_lines(name)?.ok_or_else(|| anyhow::anyhow!("reference\\{name} is missing"))?;
    Ok(lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split('\t').map(str::to_string).collect()).collect())
}

fn int(s: &str) -> i32 {
    crate::fmt::parse_i32(s.trim()).unwrap_or(0)
}

/// "id:amount;id:amount" -> pairs.
fn pairs(s: &str) -> Vec<(i32, i32)> {
    s.split(';').filter(|p| !p.is_empty()).filter_map(|p| p.split_once(':').map(|(a, b)| (int(a), int(b)))).collect()
}

/// The page's name for a CorrectTbl id: the short stat names the other pages use, else the game's wording for it.
fn stat_name(id: i32, names: &IndexMap<i32, String>) -> String {
    match id {
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
        16 => "Critical".into(),
        19 => "P.Def".into(),
        20 => "M.Def".into(),
        _ => match names.get(&id) {
            Some(n) => stat_labels::name(n, 0),
            None => format!("CorrectTbl {id}"),
        },
    }
}

/// The page data as JSON.
pub fn planner_json(data: &GameData) -> Result<Value> {
    let t = data.demons()?;
    let item_name = |id: i32| -> Result<String> { Ok(data.item_name(id)?.unwrap_or_else(|| format!("item {id}"))) };
    let correct: IndexMap<i32, String> = rows("correct-table.tsv")?.iter().filter(|r| r.len() > 1).map(|r| (int(&r[0]), r[1].clone())).collect();
    let mut stats_used: IndexMap<i32, String> = IndexMap::new();
    let mut name_of = |id: i32| -> String {
        let n = stat_name(id, &correct);
        stats_used.entry(id).or_insert_with(|| n.clone());
        n
    };

    // ---- runes and their growth types ----
    let mut runes: Vec<Value> = Vec::new();
    let mut rune_rows: Vec<Vec<Vec<String>>> = vec![Vec::new(); 12];
    for r in rows("reunion-growth.tsv")? {
        if r.len() < 5 {
            continue;
        }
        let g = int(&r[1]);
        if (1..=12).contains(&g) {
            rune_rows[(g - 1) as usize].push(r);
        }
    }
    for (i, list) in rune_rows.iter().enumerate() {
        let plain = list.iter().find(|r| int(&r[2]) == 1);
        let name = plain.map(|r| r[3].trim().to_string()).unwrap_or_else(|| format!("Rune {}", i + 1));
        let text = plain.and_then(|r| r.get(5)).cloned().unwrap_or_default();
        let mut ranks: Vec<Value> = Vec::new();
        for rank in 1..=9 {
            match list.iter().find(|r| int(&r[2]) == rank) {
                Some(r) => {
                    let mut items = Vec::new();
                    for (id, amount) in pairs(r.get(4).map(String::as_str).unwrap_or("")) {
                        items.push(json!([id, item_name(id)?, amount]));
                    }
                    ranks.push(json!({ "growth": int(&r[0]), "name": r[3].trim(), "items": items }));
                }
                None => ranks.push(Value::Null),
            }
        }
        let own: Vec<i32> = list.iter().filter(|r| int(&r[2]) == 0).map(|r| int(&r[0])).collect();
        let bonus: Vec<Value> = RUNE_BONUS[i].iter().map(|(id, v)| json!([id, name_of(*id), v])).collect();
        runes.push(json!({ "id": i + 1, "name": name, "text": text, "bonus": bonus, "mag": if i == 9 { 5 } else { 0 }, "ranks": ranks, "own": own }));
    }
    let growth: IndexMap<String, Value> = t.growth.iter().map(|(id, g)| (id.to_string(), json!(g))).collect();
    let growth_names: IndexMap<String, Value> = t.growth_names.iter().map(|(id, n)| (id.to_string(), json!(n))).collect();
    let xp: Vec<i64> = rows("level-xp.tsv")?.iter().filter(|r| r.len() > 1).map(|r| r[1].trim().parse::<i64>().unwrap_or(0)).collect();
    if xp.len() != 99 {
        anyhow::bail!("reference\\level-xp.tsv has {} rows, expected 99", xp.len());
    }

    // ---- Mitama ----
    let mitama_demon = |k: usize| -> i32 {
        let key = ["MITAMA_1_ARAMITAMA", "MITAMA_2_NIGIMITAMA", "MITAMA_3_KUSHIMITAMA", "MITAMA_4_SAKIMITAMA"][k];
        data.server_constant(key).ok().flatten().and_then(|v| crate::fmt::parse_i32(&v)).unwrap_or(0)
    };
    let pg_rows = rows("mitama-pg.tsv")?;
    let mut kinds: Vec<Value> = Vec::new();
    for k in 0..4usize {
        let typ = k as i32 + 1;
        let mine: Vec<&Vec<String>> = pg_rows.iter().filter(|r| r.len() > 4 && int(&r[2]) == typ).collect();
        let pg = mine.first().map(|r| int(&r[1])).unwrap_or(0);
        let mut trades = Vec::new();
        for r in &mine {
            let mut gems = Vec::new();
            for (id, amount) in pairs(&r[4]) {
                gems.push(json!([id, item_name(id)?, amount]));
            }
            trades.push(json!({ "pgs": int(&r[3]), "gems": gems }));
        }
        // the rank bonus of this kind: rank 0..12 -> (stat, base value) pairs (the page scales them by level)
        let mut ranks: Vec<Value> = Vec::new();
        for rank in 0..=12 {
            let u = t.union_bonus.get(&(k as i32 * 16 + rank)).cloned().unwrap_or_default();
            let mut ps = Vec::new();
            let mut i = 0;
            while i + 1 < u.len() {
                if u[i + 1] != 0 {
                    ps.push(json!([u[i], name_of(u[i]), u[i + 1]]));
                }
                i += 2;
            }
            ranks.push(json!(ps));
        }
        // the random slot bonuses of this kind (ids kind*32 .. +31 with a value)
        let mut slots: Vec<Value> = Vec::new();
        for (id, (stat, v)) in &t.reunion_bonus {
            if *id / 32 == k as i32 && *v != 0 {
                slots.push(json!([id, stat, name_of(*stat), v]));
            }
        }
        let demon = mitama_demon(k);
        let demon_name = t.devils.get(&demon).map(|d| d.name.clone()).unwrap_or_else(|| format!("{} Mitama", MITAMA_NAMES[k + 1]));
        kinds.push(json!({ "type": typ, "name": MITAMA_NAMES[k + 1], "demon": demon, "demonName": demon_name, "pg": pg, "pgName": if pg > 0 { item_name(pg)? } else { String::new() }, "trades": trades, "ranks": ranks, "slots": slots }));
    }
    let mut sets: Vec<Value> = Vec::new();
    for set in &t.mitama_sets {
        let mut bonus_pairs = |b: &[i32]| -> Vec<Value> {
            let mut out = Vec::new();
            let mut i = 0;
            while i + 1 < b.len() {
                if b[i + 1] != 0 || b[i] < 0 {
                    if b[i] >= 0 && b[i + 1] != 0 {
                        out.push(json!([b[i], name_of(b[i]), b[i + 1]]));
                    } else if b[i] == -2 {
                        out.push(json!([-2, "Summon magnetite", b[i + 1]]));
                    } else if b[i] == -1 && b[i + 1] != 0 {
                        out.push(json!([-1, "effect", b[i + 1]]));
                    }
                }
                i += 2;
            }
            out
        };
        let bonus = bonus_pairs(&set.bonus);
        let bonus_ex = bonus_pairs(&set.bonus_ex);
        sets.push(json!({ "id": set.id, "rune": set.id / 4, "req": set.requirements, "bonus": bonus, "bonusEx": bonus_ex, "text": set.description, "exText": set.ex_description }));
    }
    let boost_skill = data.mitama_set_boost_skill()?;
    let boost_name = data.skill_names()?.get(&boost_skill).cloned().flatten().unwrap_or_else(|| format!("skill {boost_skill}"));

    Ok(json!({
        "source": "The server's growth, rebirth and Mitama tables (DevilLVUpRateData, MitamaUnionBonusData, MitamaReunionBonusData, MitamaReunionSetBonusData, NPCBarterData), the client's texts, and comp_hack's rebirth rules",
        "runes": runes, "growth": growth, "growthNames": growth_names, "xp": xp, "rankPoints": RANK_POINTS,
        "apple": { "id": APPLE_ITEM, "name": item_name(APPLE_ITEM)?, "xp": APPLE_XP },
        "mitama": { "kinds": kinds, "sets": sets, "boostSkill": boost_skill, "boostName": boost_name },
        "stats": stats_used.iter().map(|(id, n)| (id.to_string(), json!(n))).collect::<IndexMap<String, Value>>(),
    }))
}

/// Writes app\ui\rebirth-planner.json.
pub fn build(data: &GameData, out_file: &Path) -> Result<()> {
    let v = planner_json(data)?;
    std::fs::write(out_file, json::serialize(&v, Escape::Relaxed, false))?;
    crate::console::write_line(&format!(
        "Wrote {} ({} runes, {} growth types, {} Mitama sets).",
        out_file.display(),
        v["runes"].as_array().map_or(0, |a| a.len()),
        v["growth"].as_object().map_or(0, |a| a.len()),
        v["mitama"]["sets"].as_array().map_or(0, |a| a.len())
    ));
    Ok(())
}
