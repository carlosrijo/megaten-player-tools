//! Port of `src/Calc/CharacterExtras.cs`.
//!
//! Bonuses from the title, the clan, completed quests ("quest bonus") and the Demonic Compendium, as the server applies
//! them (comp_hack TokuseiManager::GetDirectTokusei, CharacterState::UpdateQuestState / GetAllSkills,
//! DemonState::UpdateSharedState, TokuseiManager::CalculateAttributeValue):
//!   title       nothing: a title is an id and a text (MiTitleData), the server gives it no effect.
//!   clan        the CLAN_LEVEL_SKILLS skills of every level up to the clan's. The client lists them among the learned
//!               skills (progress +412), so the learned-passive pass already counts them; `compute` adds them only when
//!               include_clan_skills is set.
//!   quest       the QUEST_BONUS tokusei of every threshold <= the count of completed NORMAL/REPEATABLE quests; values
//!               with QUEST_BONUS_DIVIDE are value x floor(count / divisor).
//!   compendium  with Demonic Compendium v2 (valuable 251): DEMON_BOOK_SUMMONER_BONUS (for the character) and
//!               DEMON_BOOK_BONUS (for the summoned demon, Target "partner") of every threshold <= the unique entry
//!               count; DEMON_BOOK_DIVIDE values are value x floor(count / divisor). Family/race-divided values depend
//!               on the summoned demon and are "situational".
//! Server data: constants.xml (QUEST_BONUS, DEMON_BOOK_BONUS, DEMON_BOOK_SUMMONER_BONUS, CLAN_LEVEL_SKILLS) and the
//! tokusei XML (tokusei_custom.xml holds 20000000-20010018). The game's own tables say the same (reference\
//! quest-bonus-game.tsv = server QuestBonusData, reference\compendium-bonus-game.tsv = DevilBookBonusData + client text),
//! and their descriptions name the bonuses here.

// The checks keep the C# shape (nested ifs, index loops) so each line can be compared with the original.
#![allow(clippy::collapsible_if, clippy::needless_range_loop, clippy::type_complexity, clippy::too_many_arguments, clippy::manual_range_contains)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use indexmap::{IndexMap, IndexSet};

use super::effect::Effect;
use super::gear_effects;
use super::item_effects::ItemEffects;
use crate::Result;
use crate::data::game_data::{GameData, double_parse, int_parse, member_objects, member_text, parse_xml, reference_file, server_constants_text, xml_children, xml_value};
use crate::data::json::{self, At};
use crate::data::stat_labels;
use crate::fmt;
use crate::game::progress_reader::{self, ProgressState};
use crate::memory::GameMemory;

/// Is this one of the clan level skills (the server's CLAN_LEVEL_SKILLS)?
pub fn is_clan_skill(data: &GameData, skill_id: i32) -> Result<bool> {
    Ok(Rules::load(data)?.clan_level_skills.iter().any(|l| l.contains(&skill_id)))
}

/// All four kinds of extra bonus for the state. fx is used for the clan skills (include_clan_skills).
pub fn compute(data: &GameData, fx: Option<&ItemEffects>, state: Option<&ProgressState>, include_clan_skills: bool) -> Result<Vec<Effect>> {
    let mut res = Vec::new();
    let Some(state) = state else { return Ok(res) };
    let rules = Rules::load(data)?;

    // Clan: skills by clan level
    if include_clan_skills {
        if let Some(lvl) = clan_level(data, Some(state))?.filter(|&l| l > 0) {
            let mut i = 0;
            while i < lvl as usize && i < rules.clan_level_skills.len() {
                for &sid in &rules.clan_level_skills[i] {
                    res.extend(skill_effects(data, fx, sid, &format!("{} Lv{}: {}", state.clan_name.as_deref().unwrap_or("Clan"), i + 1, skill_name(data, sid)))?);
                }
                i += 1;
            }
        }
    }

    // Quest bonus
    if let Some(qc) = state.quest_bonus_count {
        for (key, tids) in &rules.quest_bonus {
            if *key <= qc {
                for &tid in tids {
                    let text = rules.quest_text.get(key).cloned().unwrap_or_else(|| format!("tokusei {tid}"));
                    res.extend(tokusei_effects(&rules, tid, "quest", &format!("Quest bonus {key}: {text}"), "self", qc, 0));
                }
            }
        }
    } else {
        res.push(Effect {
            label: "Quest bonus not counted: the completed-quest count isn't known".into(),
            cond: "unparsed".into(),
            source: "quest".into(),
            item: "Quest bonus".into(),
            missing_data: true,
            ..Default::default()
        });
    }

    // Demonic Compendium
    if let Some(cc) = state.compendium_count {
        if state.has_compendium_v2 == Some(false) {
            res.push(Effect {
                label: "No compendium bonus: Demonic Compendium v2 not owned".into(),
                cond: "inactive".into(),
                source: "compendium".into(),
                item: "Demonic Compendium".into(),
                ..Default::default()
            });
        } else if cc > 0 {
            for (key, tids) in &rules.book_summoner {
                if *key <= cc {
                    for &tid in tids.iter().filter(|&&t| t != 0) {
                        let text = rules.book_text.get(&(*key, true)).cloned().unwrap_or_else(|| format!("tokusei {tid}"));
                        res.extend(tokusei_effects(&rules, tid, "compendium", &format!("Compendium {key}: {text}"), "self", 0, cc));
                    }
                }
            }
            for (key, tids) in &rules.book_demon {
                if *key <= cc {
                    for (i, &tid) in tids.iter().enumerate() {
                        let text = if *key == 0 { rules.book_zero.get(i).cloned() } else { rules.book_text.get(&(*key, false)).cloned() };
                        let text = text.unwrap_or_else(|| format!("tokusei {tid}"));
                        res.extend(tokusei_effects(&rules, tid, "compendium", &format!("Compendium {key} (summoned demon): {text}"), "partner", 0, cc));
                    }
                }
            }
        }
    } else {
        res.push(Effect {
            label: "Compendium bonus not counted: the compendium entry count isn't known".into(),
            cond: "unparsed".into(),
            source: "compendium".into(),
            item: "Demonic Compendium".into(),
            missing_data: true,
            ..Default::default()
        });
    }

    Ok(res)
}

/// The clan level: the highest level whose CLAN_LEVEL_SKILLS are all learned (None when skills or the rule are missing).
pub fn clan_level(data: &GameData, state: Option<&ProgressState>) -> Result<Option<i32>> {
    let rules = Rules::load(data)?;
    let Some(learned) = state.and_then(|s| s.learned_skills.as_ref()) else { return Ok(None) };
    if rules.clan_level_skills.is_empty() {
        return Ok(None);
    }
    let mut lvl = 0usize;
    while lvl < rules.clan_level_skills.len() && rules.clan_level_skills[lvl].iter().all(|s| learned.contains(s)) {
        lvl += 1;
    }
    Ok(Some(lvl as i32))
}

/// The clan level skills learned, in level order (clan level, skill id, name).
pub fn clan_skills(data: &GameData, state: Option<&ProgressState>) -> Result<Vec<(i32, i32, String)>> {
    let rules = Rules::load(data)?;
    let mut res = Vec::new();
    for (i, skills) in rules.clan_level_skills.iter().enumerate() {
        for &s in skills {
            if state.and_then(|st| st.learned_skills.as_ref()).is_some_and(|l| l.contains(&s)) {
                res.push((i as i32 + 1, s, skill_name(data, s)));
            }
        }
    }
    Ok(res)
}

/// The server's quest-bonus count from a CharacterProgress.CompletedQuests mask (bit id; 128 bytes): completed quests
/// whose QuestData type is NORMAL or REPEATABLE (reference\quest-types.tsv). None when the table is missing.
pub fn quest_bonus_count_from_mask(completed_quests: Option<&[u8]>) -> Result<Option<i32>> {
    let Some(mask) = completed_quests else { return Ok(None) };
    let mut ids = Vec::new();
    for id in 0..mask.len() * 8 {
        if (mask[id / 8] & (1 << (id % 8))) != 0 {
            ids.push(id as i32);
        }
    }
    quest_bonus_count(Some(ids))
}

/// The quest-bonus count from completed quest ids (the client's completed-quest map, DemonReader.ReadQuests): those
/// whose QuestData type is NORMAL or REPEATABLE. Checked on Rubito: 28 + 4 = 32, as the Quest Completion Bonuses window
/// shows (its text says repeatable quests don't count, but the number only matches with them). None when the table
/// is missing.
pub fn quest_bonus_count(completed: Option<impl IntoIterator<Item = i32>>) -> Result<Option<i32>> {
    let Some(completed) = completed else { return Ok(None) };
    let Some(file) = reference_file("quest-types.tsv") else { return Ok(None) };
    let mut bonus: IndexSet<i32> = IndexSet::new();
    for line in file.lines()? {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = line.split('\t').collect();
        if r.len() > 1 {
            if let Some(id) = fmt::parse_i32(r[0]) {
                if r[1] == "NORMAL" || r[1] == "REPEATABLE" {
                    bonus.insert(id);
                }
            }
        }
    }
    Ok(Some(completed.into_iter().filter(|c| bonus.contains(c)).count() as i32))
}

/// The server's compendium count from an AccountWorldData.DevilBook mask (bit = DevilBookData shiftValue; 512 bytes):
/// unique entry ids among the set bits' records with a group id (reference\devil-book.tsv). None when the table is missing.
pub fn compendium_count(devil_book: Option<&[u8]>) -> Result<Option<i32>> {
    let Some(book) = devil_book else { return Ok(None) };
    let Some(file) = reference_file("devil-book.tsv") else { return Ok(None) };
    let mut entries: IndexSet<i32> = IndexSet::new();
    for line in file.lines()? {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = line.split('\t').collect();
        if r.len() < 6 {
            continue;
        }
        let Some(shift) = fmt::parse_i32(r[1]) else { continue };
        if r[4] == "0" || shift < 0 || (shift / 8) as usize >= book.len() {
            continue;
        }
        if (book[(shift / 8) as usize] & (1 << (shift % 8))) != 0 {
            if let Some(entry) = fmt::parse_i32(r[5]) {
                entries.insert(entry);
            }
        }
    }
    Ok(Some(entries.len() as i32))
}

// ---------- effects ----------

/// `v.ToString("0.##", InvariantCulture)`.
fn num(v: f64) -> String {
    fmt::custom(v, "0.##")
}

fn tokusei_effects(rules: &Rules, tid: i32, source: &str, item: &str, target: &str, quest_count: i32, book_count: i32) -> Vec<Effect> {
    let mut res = Vec::new();
    let Some(vals) = rules.tokusei.get(&tid) else {
        res.push(Effect {
            label: format!("Unknown effect (tokusei {tid})"),
            cond: "unparsed".into(),
            source: source.into(),
            item: item.into(),
            tokusei: tid,
            target: target.into(),
            missing_data: true,
            ..Default::default()
        });
        return res;
    };
    for v in vals {
        let mut value = v.value;
        let mut cond = "always";
        let mut note = String::new();
        match v.mult.as_str() {
            "" | "NONE" => {}
            "QUEST_BONUS_DIVIDE" => {
                value *= (quest_count as f64 / v.div as f64).floor();
                note = format!("{quest_count} quests / {}", v.div);
            }
            "DEMON_BOOK_DIVIDE" => {
                value *= (book_count as f64 / v.div as f64).floor();
                note = format!("{book_count} entries / {}", v.div);
            }
            "DEMON_BOOK_FAMILY_DIVIDE" => {
                cond = "situational";
                value = v.value;
                note = format!("x floor(entries of the summoned demon's family / {})", v.div);
            }
            "DEMON_BOOK_RACE_DIVIDE" => {
                cond = "situational";
                value = v.value;
                note = format!("x floor(entries of the summoned demon's race / {})", v.div);
            }
            m => {
                cond = "unparsed";
                note = format!("x {m} / {}", v.div);
            }
        }
        let mut e = Effect { value: Some(value), cond: cond.into(), cond_note: note, target: target.into(), source: source.into(), item: item.into(), tokusei: tid, ..Default::default() };
        if v.aspect {
            match v.id.as_str() {
                "SUMMON_SYNC_EXTEND" => {
                    e.label = "Summon Sync Duration".into();
                    e.pct = true;
                }
                id => {
                    e.label = format!("{id} {}", num(v.value));
                    e.cond = "unparsed".into();
                    e.value = None;
                }
            }
        } else if v.id == "RES_STATUS" {
            // the game: "Ailment Resistance +20" for RES_STATUS -20 (the same name and sign gear and skills use)
            let (label, shown) = stat_labels::displayed("RES_STATUS", 0, value);
            e.label = label;
            e.value = Some(shown);
        } else if v.id.starts_with("BOOST_") {
            e.label = stat_labels::name(&v.id, 0);
            e.pct = true;
        } else {
            e.label = gear_effects::correct_label_of(&v.id);
            e.pct = v.r#type == 1 || gear_effects::is_pct_correct(&v.id);
        }
        res.push(e);
    }
    res
}

fn skill_effects(data: &GameData, fx: Option<&ItemEffects>, sid: i32, item: &str) -> Result<Vec<Effect>> {
    let mut res = Vec::new();
    let Some(s) = data.passives()?.get(&sid.to_string()).filter(|v| !v.is_null()) else { return Ok(res) };
    // a null calculator (C#) fails at its first use
    let need = || fx.ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."));
    for c in json::arr(s.at("correctTbl")) {
        res.push(need()?.correct(&json::str(c.at("id")).unwrap_or_default(), json::int(c.at("type")), json::num(c.at("value")).unwrap_or(0.0), "always", "clan", item));
    }
    for t in json::arr(s.at("tokuseiIds")) {
        res.extend(need()?.tokusei(json::int(Some(t)), "clan", item)?);
    }
    Ok(res)
}

fn skill_name(data: &GameData, sid: i32) -> String {
    match data.skill_names().ok().and_then(|n| n.get(&sid)) {
        Some(n) => n.clone().unwrap_or_default(),
        None => format!("skill {sid}"),
    }
}

// ---------- server rules ----------

struct TokVal {
    id: String,
    r#type: i32,
    value: f64,
    mult: String,
    div: i32,
    aspect: bool,
}

#[derive(Default)]
struct Rules {
    quest_bonus: Vec<(i32, Vec<i32>)>,
    book_demon: Vec<(i32, Vec<i32>)>,
    book_summoner: Vec<(i32, Vec<i32>)>,
    clan_level_skills: Vec<Vec<i32>>,
    tokusei: HashMap<i32, Vec<TokVal>>,
    quest_text: HashMap<i32, String>,
    book_text: HashMap<(i32, bool), String>,
    book_zero: Vec<String>,
}

/// `static Rules _cache` (`??=`): built once per process, a failed build is tried again next time.
static RULES_CACHE: Mutex<Option<Arc<Rules>>> = Mutex::new(None);

impl Rules {
    fn load(data: &GameData) -> Result<Arc<Rules>> {
        let mut cache = RULES_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = cache.as_ref() {
            return Ok(Arc::clone(r));
        }
        let r = Arc::new(Rules::build(data)?);
        *cache = Some(Arc::clone(&r));
        Ok(r)
    }

    fn build(data: &GameData) -> Result<Rules> {
        let mut r = Rules::default();
        {
            if let Some(text) = server_constants_text()? {
                let doc = parse_xml(&text)?;
                // ToDictionary: a name twice is an error
                let mut consts: HashMap<String, roxmltree::Node> = HashMap::new();
                for c in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "constant") {
                    let name = c.attribute("name").unwrap_or("").to_string();
                    if consts.insert(name.clone(), c).is_some() {
                        return Err(anyhow::anyhow!("An item with the same key has already been added. Key: {name}"));
                    }
                }
                let pairs = |name: &str| -> Result<Vec<(i32, Vec<i32>)>> {
                    let mut v = Vec::new();
                    if let Some(c) = consts.get(name) {
                        for p in xml_children(*c, "pair") {
                            let key = xml_children(p, "key").next().map(xml_value).unwrap_or_default();
                            let val = xml_children(p, "value").next().map(xml_value).unwrap_or_default();
                            v.push((int_parse(key.trim())?, ids(&val)?));
                        }
                    }
                    v.sort_by_key(|p| p.0); // OrderBy: stable
                    Ok(v)
                };
                r.quest_bonus = pairs("QUEST_BONUS")?;
                r.book_demon = pairs("DEMON_BOOK_BONUS")?;
                r.book_summoner = pairs("DEMON_BOOK_SUMMONER_BONUS")?;
                if let Some(cl) = consts.get("CLAN_LEVEL_SKILLS") {
                    for e in xml_children(*cl, "element") {
                        r.clan_level_skills.push(ids(&xml_value(e))?);
                    }
                }
            }
        }

        // Tokusei: the custom file holds the quest/compendium ids; the others are looked up by their range file.
        let want: IndexSet<i32> = r.quest_bonus.iter().chain(&r.book_demon).chain(&r.book_summoner).flat_map(|p| p.1.iter().copied()).filter(|&t| t != 0).collect();
        let mut files: IndexSet<String> = want.iter().map(|t| format!("tokusei_{}.xml", d8(t.wrapping_div(10000).wrapping_mul(10000)))).collect();
        files.insert("tokusei_custom.xml".to_string());
        files.insert(crate::data::game_data::SEALED_TOKUSEI.to_string());
        for file in &files {
            let Some(text) = data.tokusei_xml_text(file)? else { continue };
            let doc = parse_xml(&text)?;
            for o in doc.descendants().filter(|x| x.is_element() && x.tag_name().name() == "object" && x.attribute("name") == Some("Tokusei")) {
                let Some(tid) = member_text(o, "ID").and_then(|s| fmt::parse_i32(&s)) else { continue };
                if !want.contains(&tid) || r.tokusei.contains_key(&tid) {
                    continue;
                }
                let mut vals = Vec::new();
                for (list, aspect) in [("CorrectValues", false), ("TokuseiCorrectValues", false), ("Aspects", true)] {
                    for v in member_objects(o, list) {
                        let attr = xml_children(v, "member").find(|m| m.attribute("name") == Some("Attributes")).and_then(|m| xml_children(m, "object").next());
                        vals.push(TokVal {
                            id: member_text(v, if aspect { "Type" } else { "ID" }).unwrap_or_default(),
                            r#type: if aspect { 0 } else { member_text(v, "Type").and_then(|s| fmt::parse_i32(&s)).unwrap_or(0) },
                            value: double_parse(&member_text(v, "Value").unwrap_or_else(|| "0".into()))?,
                            mult: attr.and_then(|a| member_text(a, "MultiplierType")).unwrap_or_else(|| "NONE".into()),
                            div: attr.and_then(|a| member_text(a, "MultiplierValue")).and_then(|s| fmt::parse_i32(&s)).filter(|&d| d > 0).unwrap_or(1),
                            aspect,
                        });
                    }
                }
                r.tokusei.insert(tid, vals);
            }
        }

        // The game's descriptions (server QuestBonusData, client CDevilBookBonusData)
        for row in tsv("quest-bonus-game.tsv")? {
            if row.len() > 3 {
                if let Some(k) = fmt::parse_i32(&row[1]) {
                    r.quest_text.insert(k, row[3].clone());
                }
            }
        }
        for row in tsv("compendium-bonus-game.tsv")? {
            if row.len() > 5 {
                if let Some(k) = fmt::parse_i32(&row[1]) {
                    if k == 0 {
                        r.book_zero.push(row[5].clone());
                    } else {
                        r.book_text.insert((k, row[3] == "1"), row[5].clone());
                    }
                }
            }
        }
        Ok(r)
    }
}

/// `{n:D8}`
fn d8(n: i32) -> String {
    if n < 0 { format!("-{:08}", -(n as i64)) } else { format!("{n:08}") }
}

fn tsv(name: &str) -> Result<Vec<Vec<String>>> {
    Ok(match reference_file(name) {
        Some(f) => f.lines()?.into_iter().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split('\t').map(str::to_string).collect()).collect(),
        None => Vec::new(),
    })
}

/// "7004-7006" / "7002,7003" / "20000000,20000001" -> ids.
fn ids(s: &str) -> Result<Vec<i32>> {
    let mut res = Vec::new();
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let d: Vec<&str> = part.split('-').collect();
        if d.len() == 2 {
            let (a, b) = (int_parse(d[0])?, int_parse(d[1])?);
            let count = b as i64 - a as i64 + 1;
            if count < 0 || a as i64 + count - 1 > i32::MAX as i64 {
                return Err(anyhow::anyhow!("Specified argument was out of the range of valid values. (Parameter 'count')"));
            }
            res.extend((0..count as i32).map(|k| a + k));
        } else {
            res.push(int_parse(part)?);
        }
    }
    Ok(res)
}

// ---------- test path ----------

/// A text report for the own character of the given memory (no Main needed): state, then every effect and the counted
/// totals per label. quest_bonus_count / compendium_count: the counts when known (the client's memory doesn't hold them).
pub fn report(mem: &GameMemory, data: &GameData, fx: &ItemEffects, quest_bonus_count: Option<i32>, compendium_count: Option<i32>, include_clan_skills: bool) -> Result<String> {
    let Some(st) = progress_reader::read(mem, quest_bonus_count, compendium_count, None) else { return Ok("own character not found".to_string()) };
    report_state(&st, data, fx, include_clan_skills)
}

/// The body of [`report`] for a state already read.
pub fn report_state(st: &ProgressState, data: &GameData, fx: &ItemEffects, include_clan_skills: bool) -> Result<String> {
    let mut sb = String::new();
    let clan_skills_learned = clan_skills(data, Some(st))?;
    sb.push_str(&format!(
        "clan: {}, level {} ({} clan skills learned: {})\r\n",
        st.clan_name.as_deref().unwrap_or("(none)"),
        clan_level(data, Some(st))?.map_or("?".to_string(), |l| l.to_string()),
        clan_skills_learned.len(),
        clan_skills_learned.iter().map(|c| c.1.to_string()).collect::<Vec<_>>().join(", ")
    ));
    sb.push_str(&format!(
        "quest bonus count: {}; compendium entries: {}\r\n",
        st.quest_bonus_count.map_or("unknown".to_string(), |c| c.to_string()),
        st.compendium_count.map_or("unknown".to_string(), |c| c.to_string())
    ));
    let fxs = compute(data, Some(fx), Some(st), include_clan_skills)?;
    for e in &fxs {
        let value = match e.value {
            Some(v) => format!("{}{}{}", if v >= 0.0 { "+" } else { "" }, num(v), if e.pct { "%" } else { "" }),
            None => String::new(),
        };
        sb.push_str(&format!(
            "  [{}] {} | {} {} | {}{}{}\r\n",
            e.source,
            e.item,
            e.label,
            value,
            e.cond,
            if e.target != "self" { format!(" {}", e.target) } else { String::new() },
            if !e.cond_note.is_empty() { format!(" ({})", e.cond_note) } else { String::new() }
        ));
    }
    for (title, target) in [("totals (counted, self):", "self"), ("totals (counted, summoned demon):", "partner")] {
        sb.push_str(title);
        sb.push_str("\r\n");
        // GroupBy (Source, Label, Pct): first-appearance order
        let mut groups: IndexMap<(String, String, bool), f64> = IndexMap::new();
        for e in fxs.iter().filter(|e| e.counted() && e.target == target && e.value.is_some()) {
            let sum = groups.entry((e.source.clone(), e.label.clone(), e.pct)).or_insert(0.0);
            *sum += e.value.unwrap_or(0.0);
        }
        for ((source, label, pct), sum) in groups {
            sb.push_str(&format!("  {source:<10} {label} +{}{}\r\n", num(sum), if pct { "%" } else { "" }));
        }
    }
    Ok(sb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_ranges() {
        assert_eq!(ids("7002,7003").unwrap(), vec![7002, 7003]);
        assert_eq!(ids("7004-7006").unwrap(), vec![7004, 7005, 7006]);
        assert_eq!(ids(" 5 , ,6 ").unwrap(), vec![5, 6]);
        assert!(ids("9-5").is_err());
        assert!(ids("-5").is_err());
    }

    #[test]
    fn eight_digit_ids() {
        assert_eq!(d8(20000000), "20000000");
        assert_eq!(d8(10000), "00010000");
    }

    #[test]
    fn divided_values() {
        let mut rules = Rules::default();
        rules.tokusei.insert(1, vec![TokVal { id: "LUCK".into(), r#type: 0, value: 2.0, mult: "QUEST_BONUS_DIVIDE".into(), div: 10, aspect: false }]);
        let fx = tokusei_effects(&rules, 1, "quest", "Quest bonus 10", "self", 35, 0);
        assert_eq!(fx.len(), 1);
        assert_eq!(fx[0].value, Some(6.0));
        assert_eq!(fx[0].cond_note, "35 quests / 10");
        let missing = tokusei_effects(&rules, 2, "quest", "x", "self", 0, 0);
        assert_eq!(missing[0].label, "Unknown effect (tokusei 2)");
    }
}
