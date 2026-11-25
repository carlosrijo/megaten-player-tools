//! What raised the character's expertise point cap, and what is left. The cap is 17,000.00pt + 1,000.00pt per 10
//! levels (+1,000.00pt at 99) + ExpertiseExtension x 1,000.00pt (CharacterManager::GetMaxExpertisePoints); the
//! extension is one number, so which raises make it up comes from what each raise leaves behind.
//!
//! Every raise is an event action (ActionGrantSkills `expertiseMax`) that also sets a valuable or ends a quest (the
//! server's event files: zone_events-230102/230108 for the casino, event_npcshared N491 for Gomory, zone_events-50106
//! for Shiho with its race conditions (bool_digitalizeRaceDLv), zone_events-50101 for Naotura's and St. Germain's
//! quests). NewMoon adds its own (the wiki's "New Moon Expertise Increase" table): Act 7, Act 16 and Act XX, and two
//! Black Frost exchanges in Dark Babel whose mark isn't known yet.
//!
//! Checked 2026-11-25 against both of the user's characters, read from the game: Rubito casino 4 + Gomory 8 + Shiho 15
//! + Naotura 2 + Acts 5 = 34, Rokujuu 4 + 2 + 9 + 2 + 5 = 22: each its ExpertiseExtension exactly.

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::Result;
use crate::build::xml::title;
use crate::data::game_data::{GameData, reference_file};
use crate::fmt;
use crate::game::demon_reader::Progress;

/// What says a raise was taken.
enum Mark {
    /// a valuable the same event sets
    Valuable(i32),
    /// one of these quests completed
    Quest(&'static [i32]),
    /// nothing known in the character's data
    Unknown,
}

struct Raise {
    group: &'static str,
    /// the name when it isn't a quest's (a quest's is the game's quest name)
    name: &'static str,
    /// a short label (the page's chip; empty: the amount says it all); Shiho's is made from its races
    short: &'static str,
    /// how to get it, in game
    how: &'static str,
    /// in cap raises of 1,000.00pt
    amount: i32,
    mark: Mark,
    /// Shiho's: the DLevel and the races (any one of them) it asks for
    dlevel: Option<(i32, &'static [i32])>,
    newmoon: bool,
}

const SHIHO_HOW: &str = "Talk to Shiho (Digitalize) once a demon race of the list has the Digitalization Level";
const GROUP1: &[i32] = &[9, 11, 13, 53, 54, 55, 102, 103, 109];
const GROUP2: &[i32] = &[8, 12, 59, 60, 61, 101, 106];
const GROUP3: &[i32] = &[5, 10, 52, 62, 104, 105];

const fn v(group: &'static str, name: &'static str, short: &'static str, how: &'static str, amount: i32, id: i32) -> Raise {
    Raise { group, name, short, how, amount, mark: Mark::Valuable(id), dlevel: None, newmoon: false }
}
const fn shiho(amount: i32, id: i32, dlevel: i32, races: &'static [i32]) -> Raise {
    Raise { group: "Shiho (Digitalize)", name: "", short: "", how: SHIHO_HOW, amount, mark: Mark::Valuable(id), dlevel: Some((dlevel, races)), newmoon: false }
}
const fn q(group: &'static str, short: &'static str, how: &'static str, amount: i32, ids: &'static [i32], newmoon: bool) -> Raise {
    Raise { group, name: "", short, how, amount, mark: Mark::Quest(ids), dlevel: None, newmoon }
}

const CASINO: &str = "Golden Ark Casino";
const GOMORY: &str = "Gomory (Diaspora)";
// NewMoon's own (the page says NewMoon beside these names)
const NM_STORY: &str = "Main story";
const NM_FROST: &str = "Black Frost (Dark Babel)";

static RAISES: &[Raise] = &[
    v(CASINO, "Expertise cap release (1,000)", "", "Trade casino coins at the Golden Ark Casino", 1, 188),
    v(CASINO, "Expertise cap release (3,000)", "", "Trade casino coins at the Golden Ark Casino", 3, 189),
    v(GOMORY, "Diaspora Suginami: Gomory's expertise release 1", "Suginami 1", "Diaspora Suginami rewards, traded with Gomory", 1, 191),
    v(GOMORY, "Diaspora Suginami: Gomory's expertise release 2", "Suginami 2", "Diaspora Suginami rewards, traded with Gomory", 1, 192),
    v(GOMORY, "Diaspora Suginami: Gomory's expertise release 3", "Suginami 3", "Diaspora Suginami rewards, traded with Gomory", 2, 718),
    v(GOMORY, "Diaspora Suginami: Gomory's expertise release 4", "Suginami 4", "Diaspora Suginami rewards, traded with Gomory", 4, 719),
    v(GOMORY, "Diaspora Shinagawa: Gomory's expertise release 1", "Shinagawa 1", "Diaspora Shinagawa rewards, traded with Gomory", 1, 195),
    v(GOMORY, "Diaspora Shinagawa: Gomory's expertise release 2", "Shinagawa 2", "Diaspora Shinagawa rewards, traded with Gomory", 1, 196),
    v(GOMORY, "Diaspora Shinagawa: Gomory's expertise release 3", "Shinagawa 3", "Diaspora Shinagawa rewards, traded with Gomory", 2, 720),
    v(GOMORY, "Diaspora Shinagawa: Gomory's expertise release 4", "Shinagawa 4", "Diaspora Shinagawa rewards, traded with Gomory", 4, 721),
    shiho(1, 659, 5, GROUP1),
    shiho(2, 660, 10, GROUP1),
    shiho(1, 661, 5, GROUP2),
    shiho(2, 662, 10, GROUP2),
    shiho(1, 670, 5, GROUP3),
    shiho(2, 671, 10, GROUP3),
    shiho(3, 672, 10, &[58]),
    shiho(3, 708, 10, &[4, 6, 7]),
    shiho(3, 709, 10, &[51]),
    shiho(3, 710, 10, &[107, 108, 110]),
    shiho(3, 711, 10, &[2]),
    q("Naotura", "Training 1", "Naotura in Shinjuku Babel", 1, &[770], false),
    q("Naotura", "Training 2", "Naotura in Shinjuku Babel", 1, &[771], false),
    q("Naotura", "Training 3", "Naotura in Shinjuku Babel", 1, &[772], false),
    q("St. Germain", "Level 95", "St. Germain, from level 95", 1, &[823], false),
    q("St. Germain", "Level 97", "St. Germain, from level 97", 1, &[824], false),
    q(NM_STORY, "Act 7", "The main story", 1, &[306], true),
    q(NM_STORY, "Act 16", "The main story", 2, &[316], true),
    q(NM_STORY, "Act XX", "The main story", 2, &[334, 335], true),
    Raise { group: NM_FROST, name: "Black Frost: 250 Devouring Sigils", short: "Devouring Sigils", how: "Exchange with Black Frost in Dark Babel", amount: 1, mark: Mark::Unknown, dlevel: None, newmoon: true },
    Raise { group: NM_FROST, name: "Black Frost: 250 Fragments of Creation", short: "Fragments of Creation", how: "Exchange with Black Frost in Dark Babel", amount: 1, mark: Mark::Unknown, dlevel: None, newmoon: true },
];

/// Quest id -> the game's quest name (reference\quest-types.tsv, client CQuestData).
fn quest_names() -> Result<IndexMap<i32, String>> {
    let mut res = IndexMap::new();
    let Some(file) = reference_file("quest-types.tsv") else { return Ok(res) };
    for line in file.lines()? {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = line.split('\t').collect();
        if let (Some(id), Some(name)) = (fmt::parse_i32(r[0]), r.get(3)) {
            res.insert(id, name.trim().to_string());
        }
    }
    Ok(res)
}

/// The cap raises for the page: every raise with whether the character has it (null when the data that says so wasn't
/// read), Shiho's with whether one is ready to claim (a listed race has the DLevel, not claimed yet). NewMoon's are
/// only listed when `newmoon` is Some(true). `matched` sums the raises taken; the rest of the extension is `other`
/// (null when the valuables or quests weren't read, or there is no cap).
pub fn report(data: &GameData, prog: &Progress, newmoon: Option<bool>, server: Option<&str>) -> Result<Value> {
    let names = quest_names()?;
    let demons = data.demons()?;
    // the client's race names, as the Digitalize planner shows them (DevilData's race enum when the client has none)
    let race = |r: i32| demons.game_race_name(r).unwrap_or_else(|| title(&demons.race_name(r)));
    let done = |ids: &[i32]| prog.completed_quests.as_ref().map(|c| ids.iter().any(|i| c.contains(i)));
    let empty = IndexMap::new();
    let levels = prog.levels.as_ref().unwrap_or(&empty);
    let mut rows = Vec::new();
    let mut matched = 0;
    let mut all_known = true;
    for r in RAISES.iter().filter(|r| !r.newmoon || newmoon == Some(true)) {
        let got = match r.mark {
            Mark::Valuable(id) => prog.has_valuable(id),
            Mark::Quest(ids) => done(ids),
            Mark::Unknown => None,
        };
        if !matches!(r.mark, Mark::Unknown) && got.is_none() {
            all_known = false;
        }
        if got == Some(true) {
            matched += r.amount;
        }
        let mut short = r.short.to_string();
        let name = match (&r.mark, r.dlevel) {
            (_, Some((lv, races))) => {
                let mut seen = IndexSet::new();
                let list: Vec<String> = races.iter().map(|x| race(*x)).filter(|n| seen.insert(n.clone())).collect();
                // one race: its name; several: how many (the full list is in the name)
                short = if list.len() == 1 { format!("{} DLv {lv}", list[0]) } else { format!("{} races DLv {lv}", list.len()) };
                format!("DLevel {lv}: {}", list.join(", "))
            }
            (Mark::Quest(ids), _) => match ids.len() {
                1 => names.get(&ids[0]).cloned().unwrap_or_else(|| r.name.to_string()),
                _ => "Act XX (either ending)".to_string(),
            },
            _ => r.name.to_string(),
        };
        let ready = match (r.dlevel, got) {
            (Some((lv, races)), Some(false)) if prog.levels.is_some() => races.iter().any(|x| levels.get(x).copied().unwrap_or(0) >= lv),
            _ => false,
        };
        rows.push(json!({ "group": r.group, "name": name, "short": short, "how": r.how, "amount": r.amount, "got": got, "ready": ready, "newmoon": r.newmoon }));
    }
    let ext = prog.expertise_extension;
    let other = match ext {
        Some(e) if e >= 0 && all_known => json!(e - matched),
        _ => Value::Null,
    };
    Ok(json!({ "server": server, "newmoon": newmoon, "sources": rows, "matched": matched, "other": other }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two characters it was checked on add up to their ExpertiseExtension.
    #[test]
    fn the_raises_add_up() {
        let sum = |vals: &[i32], quests: &[i32], nm: bool| -> i32 {
            RAISES
                .iter()
                .filter(|r| !r.newmoon || nm)
                .filter(|r| match r.mark {
                    Mark::Valuable(id) => vals.contains(&id),
                    Mark::Quest(ids) => ids.iter().any(|i| quests.contains(i)),
                    Mark::Unknown => false,
                })
                .map(|r| r.amount)
                .sum()
        };
        // the cap-related valuables and quests each held on 2026-11-25
        let rubito = [188, 189, 191, 192, 195, 196, 718, 720, 659, 660, 661, 662, 670, 671, 672, 710];
        let rokujuu = [188, 189, 191, 195, 659, 660, 661, 662, 670, 671];
        assert_eq!(sum(&rubito, &[306, 316, 334, 770, 771], true), 34);
        assert_eq!(sum(&rokujuu, &[306, 316, 335, 770, 771], true), 22);
        // everything: the wiki's 49 general raises (76,000 at level 99) and NewMoon's 7 (83,000)
        let all: i32 = RAISES.iter().filter(|r| !r.newmoon).map(|r| r.amount).sum();
        let nm: i32 = RAISES.iter().filter(|r| r.newmoon).map(|r| r.amount).sum();
        assert_eq!((all, nm), (49, 7));
    }
}
