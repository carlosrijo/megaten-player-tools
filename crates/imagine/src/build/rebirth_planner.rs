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
//!   mitama-pg.tsv           NPCBarterData: the Summon PG: <kind> Mitama trades (two single-PG barters per kind, 10 and 8
//!                           of each gem, and a ten-pack at 100; the data does not say which NPC offers which)
//!   SACRED_SOUL_* below      NewMoon's Saien barter for "Summon PG: Sacred Soul" (read in game; not in NPCBarterData)
//!   items-game.tsv / the website's items.json   item names
//!   devil-equipment-items.tsv, devil-equipment.tsv, devil-equipment-groups.tsv   demon equipment: item -> skill, which
//!                           features it can't cover and what can't be worn together (DevilEquipmentItemData,
//!                           DevilEquipmentData, CDevilEquipmentExclusiveData)
//!   devil-boost-items.tsv, devil-boost-extra.tsv, devil-force-slots.tsv   Demon Force: the items that put an effect in a
//!                           stack slot, each effect's tokusei, and how many slots each demon type opens
//!                           (DevilBoostItemData + DevilBoostData, DevilBoostExtraData, DevilData growth.forceStack)
//!   tokusei-client-text.tsv the client's text of each tokusei (what a Demon Force effect does)
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
use crate::data::json::{self, At, Escape};
use crate::data::stat_labels;

/// Conversion points a rank is worth when extracted (comp_hack FusionTables.cpp REUNION_RANK_POINTS: rank 0..9).
const RANK_POINTS: [i32; 10] = [0, 0, 1, 4, 10, 20, 40, 105, 320, 0];
/// "Summon PG: Sacred Soul": NewMoon's PG for a Mitama of any kind, and what Saien asks for one (Fighting Spirit, item 12658).
pub const SACRED_SOUL_PG: i32 = 21934;
pub const FIGHTING_SPIRIT: i32 = 12658;
const SACRED_SOUL_SPIRITS: i32 = 6;
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
    // NewMoon's own way to a Mitama of any kind: Saien (Shinjuku Docks B1F) barters "Summon PG: Sacred Soul" (item 21934,
    // the client's text: summons Ara / Nigi / Kusi / Saki Mitama) for 6 Fighting Spirit, or 10 of them for 60. Read in
    // game 2026-11-27 (the user's barter window); the server's NPCBarterData has no such trade, so the numbers live here.
    let spirit = json!({ "item": SACRED_SOUL_PG, "name": item_name(SACRED_SOUL_PG)?, "each": SACRED_SOUL_SPIRITS, "pack": [10, SACRED_SOUL_SPIRITS * 10], "currency": item_name(FIGHTING_SPIRIT)?, "where": "Saien, Shinjuku Docks B1F" });
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
    let equipment = equipment_json(data, &item_name)?;

    Ok(json!({
        "source": "The server's growth, rebirth and Mitama tables (DevilLVUpRateData, MitamaUnionBonusData, MitamaReunionBonusData, MitamaReunionSetBonusData, NPCBarterData), the client's texts, and comp_hack's rebirth rules",
        "runes": runes, "growth": growth, "growthNames": growth_names, "xp": xp, "rankPoints": RANK_POINTS,
        "apple": { "id": APPLE_ITEM, "name": item_name(APPLE_ITEM)?, "xp": APPLE_XP },
        "mitama": { "kinds": kinds, "sets": sets, "boostSkill": boost_skill, "boostName": boost_name, "spirit": spirit },
        "stats": stats_used.iter().map(|(id, n)| (id.to_string(), json!(n))).collect::<IndexMap<String, Value>>(),
        "equipment": equipment,
        "force": force_json(data, &item_name)?,
    }))
}

/// Demon equipment (the Epitaph Parts items), as the server lets a demon wear it (packets\game\DemonEquip.cpp,
/// CharacterManager::GetTraitSkills): four slots, one per feature (trait) slot; an item worn in a slot replaces that
/// slot's feature with the item's skill (DevilEquipmentItemData); a feature marked `fixed` can't be covered; and the new
/// item may share no exclusion group with what the other three slots hold (their features, or the items worn there).
///   items:      [item id, its skill, the item's name, the skill's name, the skill's game text]
///   rules:      skill -> [fixed, exclusion groups], for every skill DevilEquipmentData lists (features and items' skills)
///   groups:     exclusion group -> the client's name for it (CDevilEquipmentExclusiveData)
///   traitSlots: demon type -> its 4 feature slots, for the demons whose features leave a gap (demon-db.json lists the
///               features without the empty slots, so slot 3's feature of "0,0,26025,0" would look like slot 1's)
fn equipment_json(data: &GameData, item_name: &dyn Fn(i32) -> Result<String>) -> Result<Value> {
    let t = data.demons()?;
    let names = data.skill_names()?;
    let skill_name = |id: i32| names.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"));
    let mut items: Vec<Value> = Vec::new();
    for r in rows("devil-equipment-items.tsv")? {
        let (item, skill) = (int(&r[0]), int(r.get(1).map_or("", String::as_str)));
        let name = item_name(item)?;
        if name.starts_with("Debug ") {
            continue; // "Debug Epitaph Parts 1-3": the developers' test items, which nothing in the game hands out
        }
        items.push(json!([item, skill, name, skill_name(skill), data.skill_text(skill)?.unwrap_or_default()]));
    }
    let mut rules: IndexMap<String, Value> = IndexMap::new();
    for r in rows("devil-equipment.tsv")? {
        let groups: Vec<i32> = r.get(2).map_or("", String::as_str).split(',').filter(|s| !s.trim().is_empty()).map(int).collect();
        rules.insert(r[0].trim().to_string(), json!([int(r.get(1).map_or("", String::as_str)), groups]));
    }
    // the client leaves six group names in Japanese: plain translations of them (the parts in each group say the same)
    let english = |id: &str, name: &str| -> String {
        match id {
            "520" => "All Physical Boost",
            "521" => "All Magic Boost",
            "522" => "EX Affinity Boost",
            "609" => "In-Game Time Boost",
            "610" => "Moon Phase Boost",
            "611" => "Magic-Sealing Miasma Immunity",
            _ => name,
        }
        .to_string()
    };
    let groups: IndexMap<String, Value> = rows("devil-equipment-groups.tsv")?.iter().filter(|r| r.len() > 1).map(|r| (r[0].trim().to_string(), json!(english(r[0].trim(), r[1].trim())))).collect();
    let mut trait_slots: IndexMap<String, Value> = IndexMap::new();
    for (id, d) in &t.devils {
        let listed: Vec<usize> = (0..d.traits.len().min(4)).filter(|&i| d.traits[i] > 0).collect();
        if listed.iter().enumerate().any(|(k, &i)| k != i) {
            let mut four = d.traits.clone();
            four.resize(4, 0);
            trait_slots.insert(id.to_string(), json!(four[..4]));
        }
    }
    Ok(json!({ "items": items, "rules": rules, "groups": groups, "traitSlots": trait_slots }))
}

/// What every passive skill, feature and demon equipment skill does, for the planner's stats pane (rbp-effects.json):
/// skill id -> { fx, adj }.
///   fx:  the effects as the other pages word them: [label, value (null: none or unknown), percent (0/1), cond
///        ("always", "situational", "inactive", "unparsed"), note, target (the tokusei's TargetType: SELF, SUMMONER,
///        PARTY...; "SELF" for a skill's own correct table), per level (1: the value is per demon level), in the stats
///        table (1: an unconditional change to the demon's own stat that `adj` carries)]
///   adj: the changes the server makes to the demon's own stats (ActiveEntityState::AdjustStats), unconditional only:
///        [CorrectTbl id, type (0 flat, 1 percent layer 1, 2 percent layer 2), value, per level (0/1)]
/// A demon's skills reach its stats like this (ActiveEntityState::ApplySkillCorrectTbls, GetAdditionalCorrectTbls):
/// the correct tables of its passives (and switches that are on: not planned here) and of its traits (a worn part's
/// skill in place of its slot's trait), and its SELF tokusei's correct values. The effects come from the same code the
/// Overview uses for a summoned demon (ItemEffects, level 1 so a per-level value is the factor). Switch and active
/// skills are left out; a tokusei that needs the owner digitalized is left out (the digitalize path, not the demon's).
/// `data` decides what tokusei data is used: the repo's file is made without the sealed data, the app serves one made
/// with it (`effects_bytes`).
pub fn effects_json(data: std::sync::Arc<GameData>) -> Result<Value> {
    use crate::calc::item_effects::ItemEffects;
    let fx = ItemEffects::new(std::sync::Arc::clone(&data), "", 1, IndexMap::new());
    let t = data.demons()?;
    let index: std::collections::HashMap<String, i32> = rows("correct-table.tsv")?.iter().filter(|r| r.len() > 1).map(|r| (r[1].trim().to_string(), int(&r[0]))).collect();
    // the 14 stats of the page's table (RBP_STAT_IDX): an unconditional change to one of them is applied there
    const TABLE: [&str; 14] = ["STR", "MAGIC", "VIT", "INT", "SPEED", "LUCK", "HP_MAX", "MP_MAX", "CLSR", "LNGR", "SPELL", "SUPPORT", "PDEF", "MDEF"];
    let table_labels: std::collections::HashSet<String> = TABLE.iter().flat_map(|n| (0..=2).map(move |k| stat_labels::name(n, k))).collect();
    let mut ids: indexmap::IndexSet<i32> = t.skills.iter().filter(|(_, (cat, _))| *cat == 0).map(|(id, _)| *id).collect();
    for r in rows("devil-equipment.tsv")? {
        ids.insert(int(&r[0]));
    }
    for r in rows("devil-equipment-items.tsv")? {
        ids.insert(int(r.get(1).map_or("", String::as_str)));
    }
    let passives = data.passives()?;
    let line = |e: &crate::calc::effect::Effect, target: &str, own: bool| -> Value {
        let lv = e.cond_note.contains("LvL × ");
        let table = own && e.cond == "always" && e.value.is_some() && table_labels.contains(&e.label);
        json!([e.label, e.value, i32::from(e.pct), e.cond, e.cond_note, target, i32::from(lv), i32::from(table)])
    };
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for id in ids {
        let Some(p) = passives.get(&id.to_string()).filter(|v| !v.is_null()) else { continue };
        let (mut lines, mut adj) = (Vec::new(), Vec::new());
        for c in json::arr(p.at("correctTbl")) {
            let name = json::str(c.at("id")).unwrap_or_default();
            let (typ, v) = (json::int(c.at("type")), json::num(c.at("value")).unwrap_or(0.0));
            let typ = if typ >= 100 { typ - 100 } else { typ };
            lines.push(line(&fx.correct(&name, typ, v, "always", "planner", ""), "SELF", true));
            if let Some(&k) = index.get(&name) {
                adj.push(json!([k, typ, v, 0]));
            }
        }
        for tn in json::arr(p.at("tokuseiIds")) {
            let tid = json::int(Some(tn));
            if tid == 0 {
                continue;
            }
            let def = data.tokusei_full(tid)?;
            let target = def.as_ref().map_or_else(String::new, |d| d.target.clone());
            // on a demon a PARTNER-aimed tokusei is its own (TokuseiManager: a PARTNER_DEMON keeps it); SUMMONER goes to the character
            let own = target.is_empty() || target == "SELF" || target == "PARTNER";
            let plain = def.as_ref().is_some_and(|d| d.conditions.is_empty() && d.skill_conditions.is_empty() && !d.target_conditions);
            for e in fx.tokusei(tid, "planner", "")? {
                if e.cond != "digitalized" {
                    lines.push(line(&e, &target, own && plain));
                }
            }
            if let (true, true, Some(d)) = (own, plain, def.as_ref()) {
                for c in &d.correct {
                    let per_level = c.mult == "LEVEL";
                    if !(per_level || c.mult == "NONE" || c.mult.is_empty()) {
                        continue;
                    }
                    let typ = if c.r#type >= 100 { c.r#type - 100 } else { c.r#type };
                    let v = if c.precision > 0 { c.value / 10f64.powi(c.precision) } else { c.value };
                    if let Some(&k) = c.id.as_deref().and_then(|n| index.get(n)) {
                        adj.push(json!([k, typ, v, i32::from(per_level)]));
                    }
                }
            }
        }
        if !lines.is_empty() {
            out.insert(id.to_string(), json!({ "fx": lines, "adj": adj }));
        }
    }
    Ok(json!({ "source": "Each passive skill's correct table and tokusei, as the server applies them to a demon (comp_hack ActiveEntityState::AdjustStats)", "skills": out }))
}

/// rbp-effects.json as the app serves it: made once with the sealed data (the repo's file is made without it).
pub fn effects_bytes() -> Option<&'static [u8]> {
    static BYTES: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    BYTES
        .get_or_init(|| {
            if !crate::data::pack::available() {
                return None;
            }
            let v = effects_json(std::sync::Arc::new(GameData::new())).ok()?;
            Some(json::serialize(&v, Escape::Relaxed, false).into_bytes())
        })
        .as_deref()
}

/// Demon Force: the force stack, the 8 slots whose effects a demon carries (comp_hack demon.xml ForceStack[8]). Using
/// a Demon Force item on the summoned demon can put the item's effect in a slot the player picks, over what is there
/// (packets\game\DemonForce.cpp: the item's DevilBoostData `extraID`); an empty slot past the demon type's
/// `forceStack` count can't be filled. None of the 378 such boosts has a level, rune rank, alignment or familiarity
/// rule, and the server refuses no repeat. The other way into a slot is the random effect the benefit gauge offers
/// (DevilBoostLotData), which can't be planned.
///   items: [item id, stack id, the item's name, [the client's text of each of the effect's tokusei]]
///   slots: demon type -> its slot count, for the types that open fewer than `max`
///   random: stack id -> the client's text, for the effects no item places (a COMP demon may hold one)
fn force_json(data: &GameData, item_name: &dyn Fn(i32) -> Result<String>) -> Result<Value> {
    const MAX: i32 = 8;
    let extra = rows("devil-boost-extra.tsv")?;
    let tokusei: IndexMap<i32, Vec<i32>> = extra.iter().map(|r| (int(&r[0]), r.get(3).map_or("", String::as_str).split(',').map(int).filter(|&x| x > 0).collect())).collect();
    let texts_of = |stack: i32| -> Result<Vec<String>> {
        let mut out = Vec::new();
        for &tid in tokusei.get(&stack).map_or(&[][..], Vec::as_slice) {
            if let Some(t) = data.tokusei_text(tid)? {
                out.push(t.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        }
        Ok(out)
    };
    let mut items: Vec<Value> = Vec::new();
    let mut placed: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for r in rows("devil-boost-items.tsv")? {
        let (item, stack) = (int(&r[0]), int(r.get(2).map_or("", String::as_str)));
        placed.insert(stack);
        items.push(json!([item, stack, item_name(item)?, texts_of(stack)?]));
    }
    // the effects no item places (the benefit gauge's random ones): their text, for a COMP demon that holds one
    let mut random: IndexMap<String, Value> = IndexMap::new();
    for r in &extra {
        let stack = int(&r[0]);
        if !placed.contains(&stack) {
            let t = texts_of(stack)?;
            if !t.is_empty() {
                random.insert(stack.to_string(), json!(t));
            }
        }
    }
    let mut slots: IndexMap<String, Value> = IndexMap::new();
    for r in rows("devil-force-slots.tsv")? {
        let n = int(r.get(1).map_or("", String::as_str));
        if n < MAX {
            slots.insert(r[0].trim().to_string(), json!(n));
        }
    }
    Ok(json!({ "max": MAX, "items": items, "slots": slots, "random": random }))
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
    // beside it, rbp-effects.json: the skills' effects, made WITHOUT the sealed data (this file is committed; the app
    // serves its own made with it, `effects_bytes`)
    let fx_file = out_file.with_file_name("rbp-effects.json");
    let fx = effects_json(std::sync::Arc::new(GameData::new().without_sealed_data()))?;
    std::fs::write(&fx_file, json::serialize(&fx, Escape::Relaxed, false))?;
    crate::console::write_line(&format!("Wrote {} ({} skills).", fx_file.display(), fx["skills"].as_object().map_or(0, |a| a.len())));
    Ok(())
}
