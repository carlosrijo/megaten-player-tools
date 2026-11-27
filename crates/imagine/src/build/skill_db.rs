//! Port of `src/Build/SkillDb.cs`.
//!
//! The Skill Database page's data (app\ui\skill-db.json) and its pictures (app\ui\skill-db-icons\): every skill of the
//! game grouped by its action type, with its element, cost, cast time, cooldown, reach, power and who can learn it.
//!
//! Inputs (all decoded from the IMAGINE client, smt-decoded\server):
//!   SkillData.xml   the skills: category (active / passive / switch), action type, family, element, costs, timings,
//!                   target and the damage modifier
//!   CHelpData.xml   the game's Help: the names of the 12 action types ("Action types include the following") in the
//!                   order of the action type numbers (comp_hack skilldata.xml: ATTACK 0 ... SUPPORT 11), and the page
//!                   that explains each one
//!   reference       skill-text.tsv (names, descriptions), skill-icons.tsv, expertise-rank-skills.tsv, expertise-imagine.tsv
//!   NPCBarterData.xml, NPCBarterGroupData.xml   the NPC exchanges that teach a skill, and the lists they are on
//! With `--store <server datastore>`: which NPC opens each list, and where (`npc_places`: the events of zones, partials
//! and events traced back to the ServerNPC that starts them), written as each barter's `at`. NPCBarterConditionData.xml:
//! what an exchange asks of the demon (`lv`, `needs`).
//!
//! Left out: item uses (family ITEM: what an item does when used), the demons' own item uses (DEMON_SOLO), DEBUG skills,
//! skills without a name and skills whose name the client never translated (only Japanese: test or unused records) that
//! referenceskill-names-en.tsv has no English for; the ones it has are kept with that English as `name` and the
//! client's Japanese as `ja`.
//! Which demons learn a skill is not written here: the page works it out from demon-db.json, which already holds every
//! compendium demon's skills and features.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use anyhow::anyhow;
use regex::Regex;
use serde_json::{Map, Value, json};

use super::{item_icons, npc_places};
use super::xml::{self, elements, element, m, mem, obj, title};
use crate::Result;
use crate::console;
use crate::data::game_data::{GameData, reference_file};
use crate::data::json::{self, Escape};
use crate::data::stat_labels;
use crate::fmt;
use crate::text::cmp_invariant;

/// `Int`: `int.TryParse` of the trimmed text, else 0.
fn int(s: Option<&str>) -> i32 {
    s.and_then(fmt::parse_i32).unwrap_or(0)
}

fn int_of(o: Option<roxmltree::Node>, name: &str) -> i32 {
    int(m(o, name).as_deref())
}

/// The action type numbers in order (comp_hack libcomp\schema\binarydata\skilldata.xml, MiSkillBasicData actionType).
const ACTION_IDS: [&str; 12] = ["ATTACK", "SPIN", "RUSH", "SHOT", "RAPID", "GUARD", "COUNTER", "DODGE", "TALK", "INTIMIDATE", "TAUNT", "SUPPORT"];

fn stat_of(k: &str) -> Option<&'static str> {
    Some(match k {
        "CLSR" => "Close-range",
        "LNGR" => "Long-range",
        "SPELL" => "Spell",
        "SUPPORT" => "Support",
        "WEAPON" => "Weapon",
        _ => return None,
    })
}

fn target_of(k: &str) -> Option<&'static str> {
    Some(match k {
        "ENEMY" => "Enemy",
        "ALLY" => "Ally",
        "PARTY" => "Party",
        "SOURCE" => "Self",
        "DEAD_ALLY" => "Fallen ally",
        "DEAD_PARTY" => "Fallen party member",
        "PARTNER" => "Partner demon",
        "OTHER_PLAYER" => "Another player",
        "ALLY_PLAYER" => "Allied player",
        "PLAYER" => "Player",
        "OBJECT" => "Object",
        _ => return None,
    })
}

/// `Sec`: milliseconds as seconds, "1.5 s".
fn sec(ms: i32) -> String {
    format!("{} s", fmt::custom(ms as f64 / 1000.0, "0.##"))
}

struct HelpPage {
    id: i32,
    prev: i32,
    topic: String,
    text: String,
}

fn ms(sw: &Instant) -> u128 {
    sw.elapsed().as_millis()
}

pub fn build(data: &GameData, decoded: &Path, store: Option<&Path>, client_dir: Option<&Path>, out_file: &Path) -> Result<()> {
    let sw = Instant::now();
    let reference = |f: &str| reference_file(f).ok_or_else(|| anyhow!("reference\\{f} is missing."));

    // ---- names and descriptions (client CSkillData via reference\skill-text.tsv) ----
    let mut names: HashMap<i32, String> = HashMap::new();
    for line in reference("skill-text.tsv")?.lines()? {
        if line.starts_with('#') {
            continue;
        }
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() >= 2 {
            if let Some(id) = fmt::parse_i32(p[0]) {
                if !p[1].trim().is_empty() {
                    names.insert(id, p[1].trim().to_string());
                }
            }
        }
    }

    // BABEL's English for the names the client has only (or half) in Japanese (referenceskill-names-en.tsv); the
    // Japanese is kept as `ja`, and the page shows such a name greyed out
    let mut name_ja: HashMap<i32, String> = HashMap::new();
    for (id, (ja, en)) in data.skill_translations()? {
        if names.get(id).is_some_and(|n| n == ja) {
            names.insert(*id, en.clone());
            name_ja.insert(*id, ja.clone());
        }
    }

    // ---- the action types, as the game's Help names and explains them ----
    let help_text = xml::read_text(&decoded.join("CHelpData.xml"))?;
    let help_doc = xml::parse(&help_text)?;
    let help: Vec<HelpPage> = elements(help_doc.root_element(), "object")
        .map(|o| HelpPage {
            id: int_of(Some(o), "ID"),
            prev: int_of(Some(o), "previousID"),
            topic: m(Some(o), "topic").unwrap_or_default(),
            text: m(Some(o), "text").unwrap_or_default().replace('\r', "").trim().to_string(),
        })
        .collect();
    // the text of the first page under a topic
    let under = |topic_pattern: &str| -> Option<String> {
        let re = Regex::new(&format!("(?i){topic_pattern}")).expect("topic pattern");
        let t = help.iter().find(|x| re.is_match(&x.topic))?;
        help.iter().find(|x| x.prev == t.id && !x.text.is_empty()).map(|x| x.text.clone())
    };
    let list = help
        .iter()
        .find(|x| x.text.contains("Action types include the following"))
        .map(|x| x.text.clone())
        .ok_or_else(|| anyhow!("CHelpData.xml has no list of action types."))?;
    let game_names: Vec<String> = list
        .split('\n')
        .filter(|l| l.trim_start().starts_with('・'))
        .map(|l| l.trim().trim_start_matches('・').trim().to_string())
        .collect();
    if game_names.len() != ACTION_IDS.len() {
        return Err(anyhow!("The Help lists {} action types, the skill data has {}.", game_names.len(), ACTION_IDS.len()));
    }
    let ws = Regex::new(r"\s+").expect("whitespace");
    let intro = ws.replace_all(list.split("Action types include").next().unwrap_or("").trim(), " ").into_owned();
    let negotiation = under("^Negotiation Methods$");
    let skills_help = help.iter().find(|x| x.text.starts_with("Skills are split into")).map(|x| x.text.clone()).unwrap_or_default();
    let phrase = |what: &str| -> Option<String> {
        let re = Regex::new(&format!("{what} Skills that ([^,.]+)")).expect("phrase pattern");
        re.captures(&skills_help).map(|c| c[1].trim().to_string())
    };
    let mut types: Vec<Map<String, Value>> = Vec::new();
    for i in 0..ACTION_IDS.len() {
        let n = &game_names[i];
        let text = if matches!(ACTION_IDS[i], "TALK" | "INTIMIDATE" | "TAUNT") { negotiation.clone() } else { under(&format!("- {}( Magic)?$", regex::escape(n))) };
        let mut t = Map::new();
        t.insert("key".into(), json!(ACTION_IDS[i].to_lowercase()));
        t.insert("name".into(), json!(n));
        t.insert("help".into(), json!(text));
        types.push(t);
    }
    let passive_text = phrase("Passive");
    let switch_text = phrase("Switch");
    for (key, name, text) in [("passive", "Passive", passive_text), ("switch", "Switch", switch_text)] {
        let mut t = Map::new();
        t.insert("key".into(), json!(key));
        t.insert("name".into(), json!(name));
        t.insert("help".into(), match text {
            Some(x) => json!(format!("{name} skills {x}.")),
            None => Value::Null,
        });
        types.push(t);
    }
    // Destruction magic (Agi, Zio...) is the Shot type in the data; say so under Shot with the game's own words for it
    if let Some(destruction) = under("- Destruction$") {
        types[3].insert("also".into(), json!(destruction));
    }
    if let Some(support) = under("- Support Magic$") {
        types[11].insert("also".into(), json!(support));
    }
    if let Some(curse) = under("- Curse Magic$") {
        types[11].insert("also2".into(), json!(curse));
    }

    // ---- expertise ranks that unlock each skill ----
    let mut exp_of: HashMap<i32, Vec<String>> = HashMap::new();
    let expertises = data.expertises_client()?;
    for (eid, ranks) in data.rank_skills_client()? {
        let Some(en) = expertises.get(eid).map(|d| d.name.as_str()) else { continue };
        for (rank_idx, unlocked) in ranks {
            for s in unlocked {
                exp_of.entry(*s).or_default().push(format!("{en} · Class {} Rank {}", rank_idx / 10, rank_idx % 10));
            }
        }
    }

    // ---- icons ----
    let out_dir = std::path::absolute(out_file)?.parent().map(Path::to_path_buf).unwrap_or_default();
    let icon_dir = out_dir.join("skill-db-icons");
    std::fs::create_dir_all(&icon_dir)?;
    let mut icon_done: HashMap<String, Option<String>> = HashMap::new();
    let mut icon = |id: i32| -> Result<Option<String>> {
        let Some(client) = client_dir else { return Ok(None) };
        let Some(a) = data.skill_icon_asset(id)? else { return Ok(None) };
        if let Some(have) = icon_done.get(&a) {
            return Ok(have.clone());
        }
        let dest = icon_dir.join(format!("{a}.png"));
        let mut res = None;
        if dest.is_file() {
            res = Some(format!("skill-db-icons/{a}.png"));
        } else if let Some(png) = item_icons::client_png(client, &a, &[]) {
            std::fs::write(&dest, png)?;
            res = Some(format!("skill-db-icons/{a}.png"));
        }
        icon_done.insert(a, res.clone());
        Ok(res)
    };

    // ---- NPC exchanges (barter) that teach a skill ----
    // NPCBarterData (client, so IMAGINE's own): resultItems SKILL_DEMON (the barter window's "Skill: Demon": your summoned
    // demon learns it) or SKILL_CHARACTER (you learn it), for tradeItems. Only exchanges an NPC list offers count
    // (NPCBarterGroupData entries). Which NPC opens a list, and where, is in the server datastore's events (`store`,
    // npc_places): a list no event there opens is the client's only, and its `at` stays empty.
    // NPCBarterConditionData (by barter id) gates an exchange, checked by the server on the summoned demon for a SKILL_DEMON
    // result (comp_hack packets\game\Barter.cpp BarterConditionValid): DEMON_LEVEL (`lv`), CHARACTER_NO_SKILL (the target
    // must not know the skill, learned or inherited: the wiki's "incompatible" skills), DEMON_SKILL, DEMON_SKILL_INHERITANCE
    // (the demon's growth allows the skill's inheritance type), DEMON_BASE_TYPE (a demon-specific orb), CHARACTER_LEVEL.
    // per skill: (who it is for, the cost as text, the items asked [id, name, amount], the Soul Points asked, where, the
    // demon level asked, the other conditions as text)
    let mut barter_of: HashMap<i32, Vec<(String, String, Vec<(i32, String, i32)>, i32, String, i32, String)>> = HashMap::new();
    let mut barter_items: std::collections::BTreeMap<i32, String> = std::collections::BTreeMap::new();   // every item a demon-skill barter asks, for the dashboard's icons
    let (barter_file, group_file) = (decoded.join("NPCBarterData.xml"), decoded.join("NPCBarterGroupData.xml"));
    if barter_file.is_file() && group_file.is_file() {
        let places = match store {
            Some(s) => {
                let p = npc_places::NpcPlaces::load(decoded, s)?;
                console::write_line(&format!("{} exchange lists are opened by an NPC the server data places ({} ms)", p.lists_opened, ms(&sw)));
                Some(p)
            }
            None => None,
        };
        let group_text = xml::read_text(&group_file)?;
        let group_doc = xml::parse(&group_text)?;
        let mut groups_of: HashMap<i32, Vec<i32>> = HashMap::new();   // exchange -> the lists it is on
        for g in xml::records(&group_doc).into_iter().filter(|o| o.attribute("name") == Some("MiNPCBarterGroupData")) {
            let gid = int_of(Some(g), "ID");
            if let Some(es) = mem(Some(g), "entries") {
                for e in elements(es, "element").filter_map(|e| element(e, "object")) {
                    let b = int_of(Some(e), "barterID");
                    if b != 0 {
                        groups_of.entry(b).or_default().push(gid);
                    }
                }
            }
        }
        // cost items by the client's own names (items-game.tsv; the website's can differ: 38194 is "Memory of a Pie Fight"
        // in game, "Pie Throw Memory" there)
        let mut item_names: HashMap<i32, String> = HashMap::new();
        for line in reference("items-game.tsv")?.lines()? {
            let p: Vec<&str> = line.split('\t').collect();
            if let (Some(id), Some(n)) = (p.first().and_then(|s| fmt::parse_i32(s)), p.get(1)) {
                if !n.trim().is_empty() {
                    item_names.insert(id, n.trim().to_string());
                }
            }
        }
        // the conditions by barter id: (type, value1, value2)
        let mut cond_of: HashMap<i32, Vec<(String, i32, i32)>> = HashMap::new();
        let cond_file = decoded.join("NPCBarterConditionData.xml");
        if cond_file.is_file() {
            let cond_text = xml::read_text(&cond_file)?;
            let cond_doc = xml::parse(&cond_text)?;
            for c in xml::records(&cond_doc).into_iter().filter(|o| o.attribute("name") == Some("MiNPCBarterConditionData")) {
                let list = cond_of.entry(int_of(Some(c), "ID")).or_default();
                for e in mem(Some(c), "conditions").into_iter().flat_map(|cs| elements(cs, "element")).filter_map(|e| element(e, "object")) {
                    let t = m(Some(e), "type").unwrap_or_default();
                    if !t.is_empty() && t != "NONE" {
                        list.push((t, int_of(Some(e), "value1"), int_of(Some(e), "value2")));
                    }
                }
            }
        }
        // demon names, for a demon-specific orb (DEMON_BASE_TYPE)
        let mut demon_names: HashMap<i32, String> = HashMap::new();
        for line in reference("devils-game.tsv")?.lines()? {
            let p: Vec<&str> = line.split('\t').collect();
            if let (Some(id), Some(n)) = (p.first().and_then(|s| fmt::parse_i32(s)), p.get(1)) {
                if !n.trim().is_empty() {
                    demon_names.insert(id, n.trim().to_string());
                }
            }
        }
        let skill_name = |id: i32| names.get(&id).cloned().unwrap_or_else(|| format!("skill {id}"));
        let barter_text = xml::read_text(&barter_file)?;
        let barter_doc = xml::parse(&barter_text)?;
        let parts = |o: roxmltree::Node, member: &str| -> Vec<(String, i32, i32)> {
            mem(Some(o), member)
                .map(|ms| {
                    elements(ms, "element")
                        .filter_map(|e| element(e, "object"))
                        .map(|p| (m(Some(p), "type").unwrap_or_default(), int_of(Some(p), "subtype"), int_of(Some(p), "amount")))
                        .filter(|(t, _, _)| !t.is_empty() && t != "NONE")
                        .collect()
                })
                .unwrap_or_default()
        };
        for b in xml::records(&barter_doc).into_iter().filter(|o| o.attribute("name") == Some("MiNPCBarterData")) {
            let barter_id = int_of(Some(b), "ID");
            let Some(groups) = groups_of.get(&barter_id) else { continue };
            let at = places.as_ref().map(|p| npc_places::say(&p.of_groups(groups))).unwrap_or_default();
            // the conditions: the demon level apart (a number), the rest as text ("not knowing Satisfaction 1, Overkiller 1")
            let (mut lv, mut not, mut has, mut inherit, mut demon, mut you_lv) = (0, Vec::new(), Vec::new(), Vec::new(), Vec::new(), 0);
            for (t, v1, _) in cond_of.get(&barter_id).into_iter().flatten() {
                match t.as_str() {
                    "DEMON_LEVEL" => lv = lv.max(*v1),
                    "CHARACTER_NO_SKILL" => not.push(skill_name(*v1)),
                    "DEMON_SKILL" | "CHARACTER_SKILL" => has.push(skill_name(*v1)),
                    "DEMON_SKILL_INHERITANCE" => inherit.push(skill_name(*v1)),
                    "DEMON_BASE_TYPE" | "DEMON_TYPE" => demon.push(demon_names.get(v1).cloned().unwrap_or_else(|| format!("demon {v1}"))),
                    "CHARACTER_LEVEL" => you_lv = you_lv.max(*v1),
                    _ => {}
                }
            }
            let mut needs: Vec<String> = Vec::new();
            if !demon.is_empty() {
                needs.push(format!("{} (any form)", demon.join(" or ")));
            }
            if !has.is_empty() {
                needs.push(format!("knowing {}", has.join(", ")));
            }
            if !inherit.is_empty() {
                needs.push(format!("able to inherit {}", inherit.join(", ")));
            }
            if !not.is_empty() {
                needs.push(format!("not knowing {}", not.join(", ")));
            }
            if you_lv > 0 {
                needs.push(format!("you at Lv {you_lv}"));
            }
            let needs = needs.join("; ");
            let taught: Vec<(String, i32)> =
                parts(b, "resultItems").into_iter().filter(|(t, _, _)| t == "SKILL_DEMON" || t == "SKILL_CHARACTER").map(|(t, s, _)| (t, s)).collect();
            if taught.is_empty() {
                continue;
            }
            let mut cost: Vec<String> = Vec::new();
            let mut items: Vec<(i32, String, i32)> = Vec::new();
            let mut soul = 0;
            let for_demon = taught.iter().any(|(t, _)| t == "SKILL_DEMON");
            for (t, sub, n) in parts(b, "tradeItems") {
                cost.push(match t.as_str() {
                    "ITEM" => {
                        let name = match item_names.get(&sub) {
                            Some(n) => n.clone(),
                            None => data.item_name(sub)?.unwrap_or_else(|| "an item the data does not name".to_string()),
                        };
                        items.push((sub, name.clone(), n));
                        if for_demon {
                            barter_items.entry(sub).or_insert_with(|| name.clone());
                        }
                        if n > 1 { format!("{} × {name}", fmt::n0(n as f64)) } else { name }
                    }
                    // a Soul Point price sits in `subtype` (amount is 0 in every record; user, 2026-11-27: Maragidyne read as 0)
                    "SOUL_POINT" => { soul = sub; format!("{} Soul Points", fmt::n0(sub as f64)) }
                    "COIN" => format!("{} Casino Coins", fmt::n0(n as f64)),
                    "BETHEL" => format!("{} Bethel", fmt::n0(n as f64)),
                    _ => continue,   // event counters, cooldowns: conditions, not a price
                });
            }
            let cost = if cost.is_empty() { "nothing the data names".to_string() } else { cost.join(" + ") };
            for (t, sid) in taught {
                let who = if t == "SKILL_DEMON" { "demon" } else { "you" }.to_string();
                let l = barter_of.entry(sid).or_default();
                if !l.iter().any(|(w, c, _, _, _, l2, n)| *w == who && *c == cost && *l2 == lv && *n == needs) {
                    l.push((who, cost.clone(), items.clone(), soul, at.clone(), lv, needs.clone()));
                }
            }
        }
    }

    // ---- the skills ----
    let mut skills: Vec<Map<String, Value>> = Vec::new();
    let (mut untranslated, mut unnamed, mut items) = (0, 0, 0);
    // full-width letters count: "ＤＯＯＲ－01", "Ｔｒｉｃｋ", "Ｃ4Ｕ！" are English names the client writes full width
    let latin = Regex::new("[A-Za-zＡ-Ｚａ-ｚ]").expect("latin");
    let skill_text = xml::read_text(&decoded.join("SkillData.xml"))?;
    let skill_doc = xml::parse(&skill_text)?;
    for o in xml::records(&skill_doc) {
        let common = obj(Some(o), "common");
        let basic = obj(Some(o), "basic");
        let cond = obj(Some(o), "condition");
        let cast = obj(Some(o), "cast");
        let id = int_of(common, "id");
        let cat = int_of(obj(common, "category"), "mainCategory");
        let family = m(basic, "family").unwrap_or_default();
        if matches!(family.as_str(), "ITEM" | "DEMON_SOLO" | "DEBUG") {
            items += 1;
            continue;
        }
        // "-" is no name either (50325-50334, the curative limit passives)
        let Some(name) = names.get(&id).filter(|n| n.as_str() != "-") else {
            unnamed += 1;
            continue;
        };
        if !latin.is_match(name) {
            untranslated += 1;
            continue;
        }
        let action = m(basic, "actionType").unwrap_or_else(|| "SUPPORT".to_string()).to_lowercase();
        let ty = if cat == 0 { "passive".to_string() } else if cat == 2 { "switch".to_string() } else { action };

        let mut s = Map::new();
        s.insert("id".into(), json!(id));
        s.insert("name".into(), json!(name));
        if let Some(ja) = name_ja.get(&id) {
            s.insert("ja".into(), json!(ja));   // the name is BABEL's translation of the client's Japanese one
        }
        s.insert("type".into(), json!(ty));
        if let Some(desc) = data.skill_text(id)? {
            s.insert("desc".into(), json!(desc));
        }
        if let Some(ic) = icon(id)? {
            s.insert("icon".into(), json!(ic));
        }
        // why there is no picture: the client gives the skill none (fusion skills, most features: CSkillData
        // iconID 0), or it names a picture its Interface folders do not have (Machine Gun's icon_SD01_0023a)
        else if client_dir.is_some() {
            s.insert("noIcon".into(), json!(if data.skill_icon_asset(id)?.is_none() { "none" } else { "missing" }));
        }
        if let Some(el) = stat_labels::element_by_number(int_of(common, "affinity")) {
            if cat == 1 {
                s.insert("el".into(), json!(el));
            }
        }
        if family == "FUSION" {
            s.insert("fusion".into(), json!(true));
        }
        if let Some(ex) = exp_of.get(&id) {
            s.insert("exp".into(), json!(ex));
        }
        if let Some(bs) = barter_of.get(&id) {
            s.insert("barter".into(), Value::Array(bs.iter().map(|(who, cost, items, soul, at, lv, needs)| {
                let mut b = json!({ "for": who, "cost": cost });
                if !items.is_empty() {
                    b["items"] = Value::Array(items.iter().map(|(id, name, n)| json!([id, name, n])).collect());
                }
                if *soul > 0 {
                    b["soul"] = json!(soul);
                }
                if !at.is_empty() {
                    b["at"] = json!(at);   // "Incubus in Home II": who opens the list, and where (npc_places)
                }
                if *lv > 0 {
                    b["lv"] = json!(lv);   // the demon (or you) must be this level (NPCBarterConditionData)
                }
                if !needs.is_empty() {
                    b["needs"] = json!(needs);   // "not knowing Satisfaction 1, Overkiller 1": the other conditions
                }
                b
            }).collect()));
        }

        if cat == 1 {
            // cost: MP, HP, bullets or an item, a number or a % of the maximum
            let mut costs: Vec<String> = Vec::new();
            if let Some(cs) = mem(cond, "costs") {
                for c in elements(cs, "element").map(|e| element(e, "object")) {
                    let n = int_of(c, "cost");
                    let ct = m(c, "type");
                    let pct = m(c, "numType").as_deref() == Some("PERCENT");
                    if n <= 0 {
                        continue;
                    }
                    costs.push(match ct.as_deref() {
                        Some(t @ ("MP" | "HP")) => format!("{n}{} {t}", if pct { "%" } else { "" }),
                        Some("BULLET") => if n == 1 { "1 bullet".to_string() } else { format!("{n} bullets") },
                        Some("ITEM") => format!("{} × {}", fmt::n0(n as f64), data.item_name(int_of(c, "item"))?.unwrap_or_else(|| "an item".to_string())),
                        t => format!("{n} {}", title(t.unwrap_or(""))),
                    });
                }
            }
            if !costs.is_empty() {
                s.insert("cost".into(), json!(costs.join(", ")));
            }
            let drain = int_of(cond, "activeMPDrain");
            if drain > 0 {
                s.insert("drain".into(), json!(drain));
            }
            let charge = int_of(obj(cast, "basic"), "chargeTime");
            let cd = int_of(cond, "cooldownTime");
            if charge > 0 {
                s.insert("cast".into(), json!(sec(charge)));
            }
            if cd > 0 {
                s.insert("cd".into(), json!(sec(cd)));
            }
            let target = obj(Some(o), "target");
            let range = obj(Some(o), "range");
            let reach = int_of(target, "range");
            if reach > 0 {
                s.insert("reach".into(), json!(reach));
            }
            let valid = m(range, "validType");
            let area = m(range, "areaType");
            let aoe = int_of(range, "aoeRange");
            let who = target_of(valid.as_deref().unwrap_or("")).map(str::to_string).or_else(|| match valid.as_deref() {
                None | Some("NONE") => None,
                Some(v) => Some(title(v)),
            });
            if let Some(who) = who {
                s.insert("target".into(), json!(match area.as_deref() {
                    Some("SOURCE_RADIUS") if aoe > 0 => format!("{who}, all around you"),
                    Some("TARGET_RADIUS") if aoe > 0 => format!("{who}, all around the target"),
                    Some("STRAIGHT_LINE") => format!("{who}, in a line"),
                    _ => who,
                }));
            }
            // stiffness: how long the hit target stays stunned (the "Stiffness:" in the game's description)
            let stiff = int_of(obj(Some(o), "discharge"), "stiffness");
            if stiff > 0 {
                s.insert("stiff".into(), json!(stiff));
            }
            let dmg = obj(obj(Some(o), "damage"), "battleDamage");
            let formula = m(dmg, "formula");
            let power = int_of(dmg, "modifier1");
            match formula.as_deref() {
                Some("DMG_NORMAL") if power > 0 => {
                    s.insert("power".into(), json!(power));
                }
                Some("HEAL_NORMAL" | "HEAL_STATIC") if power > 0 => {
                    s.insert("heal".into(), json!(power));
                }
                Some("HEAL_MAX_PERCENT") if power > 0 => {
                    s.insert("heal".into(), json!(format!("{power}%")));
                }
                _ => {}
            }
            let dep = m(basic, "dependencyType");
            if let Some(dep) = dep.as_deref().filter(|d| *d != "NONE") {
                let parts: Vec<String> = dep.split('_').map(|d| stat_of(d).map(str::to_string).unwrap_or_else(|| title(d))).collect();
                s.insert("stat".into(), json!(parts.join(", ")));
            }
            match m(obj(cond, "restriction"), "weaponType").as_deref() {
                Some("CLOSE_RANGE") => {
                    s.insert("weapon".into(), json!("Close-range weapon"));
                }
                Some("LONG_RANGE") => {
                    s.insert("weapon".into(), json!("Long-range weapon"));
                }
                _ => {}
            }
        }
        skills.push(s);
    }
    console::write_line(&format!(
        "{} skills kept ({items} item and debug uses, {unnamed} without a name, {untranslated} with only a Japanese name left out; {} ms)",
        skills.len(),
        ms(&sw)
    ));

    let mut result = Map::new();
    result.insert("generatedAt".into(), json!(chrono::Utc::now().format("%Y-%m-%d").to_string()));
    result.insert("intro".into(), json!(intro));
    result.insert("types".into(), Value::Array(types.iter().cloned().map(Value::Object).collect()));
    result.insert("skipped".into(), json!({ "untranslated": untranslated }));
    result.insert("skills".into(), Value::Array(skills.iter().cloned().map(Value::Object).collect()));
    let text = json::serialize(&Value::Object(result), Escape::Relaxed, false);
    std::fs::write(out_file, &text)?;
    // reference\skill-barter-items.tsv: every item a demon-skill barter asks (id, name), so the dashboard exports their
    // icons (icons.json item<id>) for the Rebirth Planner's cost card; written when the repo's reference folder is there
    if let Some(reference) = out_dir.parent().and_then(Path::parent).map(|r| r.join("reference")).filter(|r| r.is_dir()) {
        let mut tsv = String::from("#item id\tname (every item an NPC asks for a demon skill; NPCBarterData, made by imagine-reader skilldb)\n");
        for (id, name) in &barter_items {
            tsv.push_str(&format!("{id}\t{name}\n"));
        }
        std::fs::write(reference.join("skill-barter-items.tsv"), tsv)?;
    }

    // per type, in the order of the type keys (the default string order of .NET)
    let mut keys: Vec<&str> = Vec::new();
    for s in &skills {
        let k = s["type"].as_str().unwrap_or("");
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    keys.sort_by(|a, b| cmp_invariant(a, b));
    for k in keys {
        let count = skills.iter().filter(|s| s["type"].as_str() == Some(k)).count();
        let name = types.iter().find(|t| t["key"].as_str() == Some(k)).and_then(|t| t["name"].as_str()).unwrap_or("");
        console::write_line(&format!("  {name:<10} {count:>5}"));
    }
    let size = std::fs::metadata(out_file)?.len();
    console::write_line(&format!(
        "{} with a picture, {} from expertise. Written to {} ({} KB, {} ms)",
        skills.iter().filter(|x| x.contains_key("icon")).count(),
        skills.iter().filter(|x| x.contains_key("exp")).count(),
        out_file.display(),
        size / 1024,
        ms(&sw)
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_text() {
        assert_eq!(sec(1500), "1.5 s");
        assert_eq!(sec(2000), "2 s");
        assert_eq!(sec(250), "0.25 s");
        assert_eq!(sec(1234), "1.23 s");
    }

    #[test]
    fn integers_are_read_like_int_tryparse() {
        assert_eq!(int(Some(" 7 ")), 7);
        assert_eq!(int(Some("7.5")), 0);
        assert_eq!(int(None), 0);
    }
}
