//! Port of `src/Data/DemonTables.cs`.
//!
//! The server's demon tables, decoded from BinaryData\Shield (see reference\README.md): demon base data, growth
//! (level-up) tables, Mitama bonus tables, digitalize assists, the Digitalization Level tables (GuardianLevelData:
//! points per level and extend skills) and the skills' digitalize restrictions.

use std::collections::HashMap;
use std::sync::Mutex;

use indexmap::{IndexMap, IndexSet};

use super::game_data::{col, reference_lines};
use crate::Result;

/// One demon type (MiDevilData). `correct`: its 126 base CorrectTbl values (STR=0 ... CHANT_TIME=125).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Devil {
    pub id: i32,
    pub name: String,
    pub race: String,
    pub race_id: i32,
    pub growth_type: i32,
    pub fusion_options: i32,
    pub base_level: i32,
    pub traits: Vec<i32>,
    pub correct: Vec<i32>,
}

impl Devil {
    /// Made by Mitama fusion (FUSION_OPTION_MITAMA): digitalizes at the higher base rate.
    pub fn is_mitama(&self) -> bool {
        (self.fusion_options & 0x08) != 0
    }
}

/// A Mitama set: active when each Mitama kind (Ara, Nigi, Kusi, Saki) fills at least `requirements[k]` slots of its rune.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MitamaSet {
    pub id: i32,
    pub requirements: Vec<i32>,
    pub bonus: Vec<i32>,
    pub bonus_ex: Vec<i32>,
    pub description: String,
    pub ex_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assist {
    pub id: i32,
    pub race_id: i32,
    pub r#type: String,
    pub value: i32,
    /// The client's line for it (CGuardianAssistData), e.g. "Wild Bird DA【Duration time +1 minute】"; empty if unknown.
    pub text: String,
}

impl Assist {
    /// What the client says it does: the part in 【】 ("Duration time +1 minute"), else the whole line.
    pub fn effect_text(&self) -> &str {
        match (self.text.find('【'), self.text.rfind('】')) {
            (Some(a), Some(b)) if b > a => &self.text[a + '【'.len_utf8()..b],
            _ => &self.text,
        }
    }
}

/// One Digitalization Level of a race (MiGuardianLevelData). `next_xp`: the points it takes from this level to the next
/// (0 = the cap; level 0's is 1 when the race can be digitalized at all). `assists`: the two choices offered at this
/// level. `demon_ids`: demons unlocked. `extend_skill_id`: the passive skill learned on reaching it (0 = none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardianLevel {
    pub race_id: i32,
    pub level: i32,
    pub next_xp: i64,
    pub assists: Vec<i32>,
    pub demon_ids: Vec<i32>,
    pub extend_skill_id: i32,
}

/// A race's level as the character has it: the % to the next level (None at the cap) and the extend skills
/// of every level up to it (the server learns all of them when the level is reached).
#[derive(Debug, Clone, PartialEq)]
pub struct RaceLevel {
    pub race_id: i32,
    pub race: String,
    pub level: i32,
    pub points: i32,
    pub percent_to_next: Option<f64>,
    pub extend_skills: Vec<i32>,
}

pub struct DemonTables {
    pub devils: IndexMap<i32, Devil>,
    /// Growth type -> 4 stages (A-D) of STR, MAG, VIT, INT, SPD, LUCK per boost level.
    pub growth: IndexMap<i32, Vec<Vec<i32>>>,
    pub growth_names: IndexMap<i32, String>,
    /// Mitama slot bonus id -> (CorrectTbl type, value).
    pub reunion_bonus: IndexMap<i32, (i32, i32)>,
    pub mitama_sets: Vec<MitamaSet>,
    /// (mitamaType - 1) * 16 + rank -> (type, base value) pairs.
    pub union_bonus: IndexMap<i32, Vec<i32>>,
    pub assists: IndexMap<i32, Assist>,
    /// Skill id -> main category (0 passive, 1 active, 2 switch) and whether digitalizing excludes it.
    pub skills: IndexMap<i32, (i32, bool)>,
    /// Race id -> its 16 GuardianLevelData entries (index = level).
    pub levels: IndexMap<i32, Vec<GuardianLevel>>,
    /// Demon Force stack id -> its tokusei (DevilBoostExtraData; reference\devil-boost-extra.tsv).
    pub force_stack_tokusei: IndexMap<i32, Vec<i32>>,
    /// Demon equipment item id -> the skill that replaces the trait of its slot (DevilEquipmentItemData).
    pub equipment_skill: IndexMap<i32, i32>,
    /// Race id -> the race:level pairs that enable it (GuardianUnlockData; all of them). A race not here starts enabled.
    pub unlocks: IndexMap<i32, Vec<(i32, i32)>>,
    /// Demon id -> race:level pairs that let it be digitalized when no level of its race names it (GuardianSpecialData;
    /// any one of them).
    pub specials: IndexMap<i32, Vec<(i32, i32)>>,
    race_names: Mutex<HashMap<i32, String>>,
}

/// `I(s)`: `int.TryParse` or 0.
fn i(s: &str) -> i32 {
    crate::fmt::parse_i32(s).unwrap_or(0)
}

/// The race id of a DevilData race name (MiDCategoryData `race`, comp_hack libcomp schema binarydata/devildata.xml).
/// This is the id the server goes by (CharacterState::Digitalize matches the assists' raceID against it, and
/// GuardianLevelData is keyed by it). devils-game.tsv's `raceId` column comes from the website's devils.json, which
/// has none for 119 demons (NewMoon-only ones such as Gold Demon Goldfish 12069, Hospitable Pyro Jack 11772, the
/// Traditional and Deformed demons) and a different one for two (Mezuki 758: WILDER 59, Belphegor 9910949: TYRANT
/// 112, while DevilData says BRUTE and FALLEN_ANGEL). With race id 0, a demon digitalized from that list showed no
/// Digitalize boosts at all even though the character had picked them.
pub fn race_id_of(race: &str) -> Option<i32> {
    const RACES: [(&str, i32); 43] = [
        ("NONE", 0), ("SERAPHIM", 1), ("ENTITY", 2), ("DEMON_GOD", 3), ("VILE", 4), ("AVIAN", 5), ("GODDESS", 6),
        ("HEAVENLY_GOD", 7), ("RAPTOR", 8), ("DIVINE", 9), ("EVIL_DEMON", 10), ("WILD_BIRD", 11), ("YOMA", 12),
        ("EARTH_ELEMENT", 13), ("GOD_SPIRIT", 14), ("OTHER", 15), ("REAPER", 51), ("HOLY_BEAST", 52), ("BEAST", 53),
        ("FAIRY", 54), ("ELEMENTAL", 55), ("MAJIN", 56), ("MITAMA", 57), ("DEMIGOD", 58), ("WILDER", 59),
        ("DRAGON_KING", 60), ("NOCTURNE", 61), ("GODLY_BEAST", 62), ("INNOCENT", 63), ("HUMAN", 64), ("FOUL", 101),
        ("BRUTE", 102), ("HAUNT", 103), ("DRAGON", 104), ("FALLEN_ANGEL", 105), ("FEMME", 106), ("NATION_RULER", 107),
        ("EARTH_MOTHER", 108), ("EVIL_DRAGON", 109), ("GUARDIAN", 110), ("DESTROYER", 111), ("TYRANT", 112),
        ("GAIAN", 113),
    ];
    RACES.iter().find(|(n, _)| *n == race).map(|(_, id)| *id)
}

/// `Ints(s)`: comma separated numbers, an empty text gives none (an empty part inside gives 0).
fn ints(s: &str) -> Vec<i32> {
    if s.is_empty() { Vec::new() } else { s.split(',').map(i).collect() }
}

/// `Rows(file)`: the data lines of a reference file split on tabs (none when the file is missing).
fn rows(file: &str) -> Result<Vec<Vec<String>>> {
    Ok(reference_lines(file)?
        .unwrap_or_default()
        .into_iter()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect())
}

fn refs(r: &[String]) -> Vec<&str> {
    r.iter().map(String::as_str).collect()
}

/// "11:4,109:4" -> [(11, 4), (109, 4)] (race:level pairs; a malformed part is left out).
fn race_levels_of(s: &str) -> Vec<(i32, i32)> {
    s.split(',').filter_map(|p| p.split_once(':')).map(|(r, l)| (i(r), i(l))).filter(|(r, _)| *r > 0).collect()
}

impl DemonTables {
    pub fn loaded(&self) -> bool {
        !self.devils.is_empty()
    }

    /// `new DemonTables()`: reads the reference tables.
    pub fn new() -> Result<DemonTables> {
        let mut t = DemonTables {
            devils: IndexMap::new(),
            growth: IndexMap::new(),
            growth_names: IndexMap::new(),
            reunion_bonus: IndexMap::new(),
            mitama_sets: Vec::new(),
            union_bonus: IndexMap::new(),
            assists: IndexMap::new(),
            skills: IndexMap::new(),
            levels: IndexMap::new(),
            force_stack_tokusei: IndexMap::new(),
            equipment_skill: IndexMap::new(),
            unlocks: IndexMap::new(),
            specials: IndexMap::new(),
            race_names: Mutex::new(HashMap::new()),
        };
        for r in rows("devils-game.tsv")? {
            let r = refs(&r);
            let id = i(col(&r, 0)?);
            t.devils.insert(
                id,
                Devil {
                    id,
                    name: col(&r, 1)?.to_string(),
                    race: col(&r, 2)?.to_string(),
                    // the server's race (DevilData's race name), else the table's column
                    race_id: race_id_of(col(&r, 2)?).unwrap_or_else(|| i(col(&r, 3).unwrap_or(""))),
                    growth_type: i(col(&r, 4)?),
                    fusion_options: i(col(&r, 5)?),
                    base_level: i(col(&r, 6)?),
                    traits: ints(col(&r, 7)?),
                    correct: ints(col(&r, 8)?),
                },
            );
        }
        for r in rows("devil-growth.tsv")? {
            let r = refs(&r);
            let id = i(col(&r, 0)?);
            t.growth.insert(id, r.iter().skip(2).take(4).map(|s| ints(s)).collect());
            t.growth_names.insert(id, col(&r, 1)?.to_string());
        }
        for r in rows("mitama-reunion-bonus.tsv")? {
            let r = refs(&r);
            t.reunion_bonus.insert(i(col(&r, 0)?), (i(col(&r, 1)?), i(col(&r, 2)?)));
        }
        for r in rows("mitama-set-bonus.tsv")? {
            let r = refs(&r);
            t.mitama_sets.push(MitamaSet {
                id: i(col(&r, 0)?),
                requirements: ints(col(&r, 1)?),
                bonus: ints(col(&r, 2)?),
                bonus_ex: ints(col(&r, 3)?),
                description: col(&r, 4)?.to_string(),
                ex_description: col(&r, 5)?.to_string(),
            });
        }
        for r in rows("mitama-union-bonus.tsv")? {
            let r = refs(&r);
            t.union_bonus.insert(i(col(&r, 0)?), ints(col(&r, 1)?));
        }
        for r in rows("guardian-assist.tsv")? {
            let r = refs(&r);
            let id = i(col(&r, 0)?);
            let text = r.get(4).map(|s| s.to_string()).unwrap_or_default();
            t.assists.insert(id, Assist { id, race_id: i(col(&r, 1)?), r#type: col(&r, 2)?.to_string(), value: i(col(&r, 3)?), text });
        }
        for r in rows("guardian-unlock.tsv")? {
            let r = refs(&r);
            t.unlocks.insert(i(col(&r, 0)?), race_levels_of(r.get(1).copied().unwrap_or("")));
        }
        for r in rows("guardian-special.tsv")? {
            let r = refs(&r);
            t.specials.insert(i(col(&r, 0)?), race_levels_of(r.get(1).copied().unwrap_or("")));
        }
        for r in rows("skills-digitize.tsv")? {
            let r = refs(&r);
            t.skills.insert(i(col(&r, 0)?), (i(col(&r, 1)?), col(&r, 2)? == "1"));
        }
        let mut levels: Vec<GuardianLevel> = Vec::new();
        for r in rows("guardian-levels.tsv")? {
            let r = refs(&r);
            levels.push(GuardianLevel {
                race_id: i(col(&r, 0)?),
                level: i(col(&r, 1)?),
                next_xp: col(&r, 2)?.trim().parse::<i64>().unwrap_or(0),
                assists: ints(col(&r, 3)?),
                demon_ids: ints(col(&r, 4)?),
                extend_skill_id: i(col(&r, 5)?),
            });
        }
        // GroupBy(race) keeps first-appearance order; OrderBy(level) is stable
        for g in levels {
            t.levels.entry(g.race_id).or_default().push(g);
        }
        for v in t.levels.values_mut() {
            v.sort_by_key(|l| l.level);
        }
        for r in rows("devil-boost-extra.tsv")? {
            let r = refs(&r);
            t.force_stack_tokusei.insert(i(col(&r, 0)?), if r.len() > 3 { ints(r[3]) } else { Vec::new() });
        }
        for r in rows("devil-equipment-items.tsv")? {
            let r = refs(&r);
            t.equipment_skill.insert(i(col(&r, 0)?), i(col(&r, 1)?));
        }
        Ok(t)
    }

    /// A race's 18 digitalize assists.
    pub fn assists_of_race(&self, race_id: i32) -> Vec<&Assist> {
        let mut v: Vec<&Assist> = self.assists.values().filter(|a| a.race_id == race_id).collect();
        v.sort_by_key(|a| a.id);
        v
    }

    /// The races that can be digitalized (level 0's nextXP > 0, as the server's GetGuardianRaceIDs).
    pub fn digitalize_races(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.levels.iter().filter(|(_, l)| !l.is_empty() && l[0].next_xp > 0).map(|(k, _)| *k).collect();
        v.sort();
        v
    }

    /// The race's name (DevilData's race enum, e.g. FOUL), the one most of its demons have.
    pub fn race_name(&self, race_id: i32) -> String {
        let mut cache = self.race_names.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(race_id)
            .or_insert_with(|| {
                // GroupBy(race) in first-appearance order, OrderByDescending(count) is stable: the first of the biggest groups
                let mut groups: IndexMap<&str, usize> = IndexMap::new();
                for d in self.devils.values().filter(|d| d.race_id == race_id) {
                    *groups.entry(d.race.as_str()).or_insert(0) += 1;
                }
                let mut best: Option<(&str, usize)> = None;
                for (k, c) in groups {
                    if best.is_none_or(|(_, bc)| c > bc) {
                        best = Some((k, c));
                    }
                }
                best.map(|(k, _)| k.to_string()).unwrap_or_else(|| format!("race {race_id}"))
            })
            .clone()
    }

    /// The race's name as the game writes it: the "<name> DA【...】" lines of its assists in the client (CGuardianAssistData),
    /// the name most of them give (one Divine line is still in Japanese). None when no assist line names it.
    pub fn game_race_name(&self, race_id: i32) -> Option<String> {
        let mut counts: IndexMap<&str, usize> = IndexMap::new();
        for a in self.assists_of_race(race_id) {
            if let Some((name, _)) = a.text.split_once(" DA") {
                let name = name.trim();
                if !name.is_empty() {
                    *counts.entry(name).or_insert(0) += 1;
                }
            }
        }
        let mut best: Option<(&str, usize)> = None;
        for (k, c) in counts {
            if best.is_none_or(|(_, bc)| c > bc) {
                best = Some((k, c));
            }
        }
        best.map(|(k, _)| k.to_string())
    }

    /// The extend skills a character at this level of the race has: those of levels 1..level.
    pub fn extend_skills(&self, race_id: i32, level: i32) -> Vec<i32> {
        match self.levels.get(&race_id) {
            Some(l) => l.iter().filter(|x| x.level >= 1 && x.level <= level && x.extend_skill_id > 0).map(|x| x.extend_skill_id).collect(),
            None => Vec::new(),
        }
    }

    /// % of the way to the next level (the window shows it truncated to 2 decimals); None at the cap or when unknown.
    pub fn percent_to_next(&self, race_id: i32, level: i32, points: i32) -> Option<f64> {
        let l = self.levels.get(&race_id)?;
        if level >= 0 && (level as usize) < l.len() && l[level as usize].next_xp > 0 {
            Some(points as f64 * 100.0 / l[level as usize].next_xp as f64)
        } else {
            None
        }
    }

    /// Per race (from the levels and points read from the game, DemonReader.Progress.Levels/Points): level, % to next and
    /// extend skills. Races missing from the game's map count as level 0.
    pub fn race_levels(&self, levels: Option<&IndexMap<i32, i32>>, points: Option<&IndexMap<i32, i32>>) -> Vec<RaceLevel> {
        self.digitalize_races()
            .into_iter()
            .map(|id| {
                let lvl = levels.and_then(|m| m.get(&id)).copied().unwrap_or(0);
                let pts = points.and_then(|m| m.get(&id)).copied().unwrap_or(0);
                RaceLevel {
                    race_id: id,
                    race: self.race_name(id),
                    level: lvl,
                    points: pts,
                    percent_to_next: self.percent_to_next(id, lvl, pts),
                    extend_skills: self.extend_skills(id, lvl),
                }
            })
            .collect()
    }

    /// Every extend skill the character has from its Digitalization Levels.
    pub fn all_extend_skills(&self, levels: Option<&IndexMap<i32, i32>>) -> IndexSet<i32> {
        levels.into_iter().flatten().flat_map(|(k, v)| self.extend_skills(*k, *v)).collect()
    }
}
