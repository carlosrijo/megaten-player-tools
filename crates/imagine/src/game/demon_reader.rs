//! Port of `src/Game/DemonReader.cs`.
//!
//! The demons in client memory (read-only). The client has a record for every demon it has been sent (COMP and stored,
//! plus stale copies), but the details of one only after its status window was opened or it was summoned
//! (`DemonRecord::loaded`). Which of them are in the COMP: [`read_comp`]. The record type itself is
//! `game::types::DemonRecord`.
//! Layouts (client build 1.670), found on Rubito's Specter and Alilat and checked against the demon window:
//!   demon record (vtable 0x16CB8F4, the same class as characters): +8 flags (0x31 in COMP, 0x33 summoned),
//!       +12 status object, +28 skill object, +32 data object.
//!   status object (vtable 0x16CB904): +8 name (ASCII), +0x84 level (byte), +0x88 HP, +0x8C max HP, +0x90 MP (u16),
//!       +0x92 max MP (u16); from +0xB0 one 4-byte entry per CorrectTbl index (STR=0 ... MDEF=20):
//!       +2 the shown value (int16), +4 its bonus part (int16, only for STR..LUCK).
//!   skill object (vtable 0x16CB924): +12 demon type; std::map fields {head, size}: +40/+44 skill slot (0-7) -> skill
//!       id (node +12 slot, +16 id); +68/+72 learned skill id -> per-skill state (node +12 id).
//!   data object (vtable 0x16CB8E4): +16 object id, +30 growth type, +31 12 rebirth ranks, +43 Mitama type,
//!       +44 Mitama rank, +45 96 Mitama slot bonus ids, +141 slots per Mitama kind (4), +145 48 active-set flags,
//!       +248 soul points (u16), +256 20 Demon Force values (s32, 100000 = +1), +336 8 force stack ids (u16).
//!       Found on Alilat (force STR 600248 = +6 ... ; stack 10144, 10269, 1298, ... all in DevilBoostExtraData); 0 on
//!       Specter and Pixie. The growth type was matched against the shown stats (Specter: 211 "Wyrd of Atziluth"
//!       reproduces MAG/VIT/SPD exactly; the default 77 of its type doesn't).
//!       +200 4 equipment item object ids (s64, -1 = empty), +232 their 4 item types (u32, -1 = empty), one per trait
//!       slot. Found on Seraph: slot 4 = 36244 (DevilEquipmentItemData skill 23156, Movement Speed +20%).
//! Familiarity: data object +24 (u32); matched the COMP window's numbers for seven demons. Not read into the record yet.

// The checks keep the C# shape (`x < 1 || x > 99`, index loops, nested ifs) so each line can be compared with the original.
#![allow(clippy::manual_range_contains, clippy::needless_range_loop, clippy::collapsible_if)]

use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};

use super::types::DemonRecord;
use crate::memory::{GameMemory, find_u32, i16_at, i32_at, i64_at, u16_at, u32_at};

const RECORD_VTABLE: u32 = 0x16CB8F4;
const STATUS_VTABLE: u32 = 0x16CB904;
const SKILL_VTABLE: u32 = 0x16CB924;
const DATA_VTABLE: u32 = 0x16CB8E4;

/// `DemonRecord.Loaded` as the C# tested it before building the record: no base stats and no learned skills.
fn is_unloaded(d: &DemonRecord) -> bool {
    d.live_value.iter().take(6).all(|&v| v == 0) && d.acquired.is_empty()
}

/// Every demon record in memory with its details (when `every_record` is false, the ones without details are left
/// out: another player's demon in view has a record too, but no base stats or skills).
pub fn read_demons(mem: &GameMemory, every_record: bool) -> Vec<DemonRecord> {
    let mut list = Vec::new();
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue; // the game image: no heap objects
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, RECORD_VTABLE, p) {
            p = pp + 4;
            if pp + 36 > buf.len() {
                break;
            }
            let status = u32_at(buf, pp + 12);
            let skills = u32_at(buf, pp + 28);
            let data_obj = u32_at(buf, pp + 32);
            if status < 0x10000 || skills < 0x10000 || data_obj < 0x10000 {
                continue;
            }
            let s = mem.read(status as i64, 0x108);
            let k = mem.read(skills as i64, 76);
            let d = mem.read(data_obj as i64, 352);
            let (Some(s), Some(k), Some(d)) = (s, k, d) else { continue };
            if u32_at(&s, 0) != STATUS_VTABLE || u32_at(&k, 0) != SKILL_VTABLE || u32_at(&d, 0) != DATA_VTABLE {
                continue;
            }
            let obj = u32_at(&d, 16);
            if !seen.insert(obj) {
                continue;
            }
            let demon = parse(mem, r.base + pp as i64, i32_at(buf, pp + 8), &s, &k, &d);
            // Another player's demon in view has a record too, but no base stats or skills.
            let Some(demon) = demon else { continue };
            if !every_record && is_unloaded(&demon) {
                continue;
            }
            list.push(demon);
        }
    }
    list
}

/// The demons in the COMP, in slot order, picked out of every demon record in memory (`all`, from
/// [`read_demons`] with every_record); None when the game's slot list isn't found.
/// Why: with the COMP open the client holds a record for every demon it has been sent, the COMP's and the stored
/// ones alike, and stale copies with older object ids (53 records for a 7-demon COMP on 2026-11-23), nearly all
/// without details (see `DemonRecord::loaded`). The COMP itself is a separate list of slots:
///   10 slots of 32 bytes: +0 demon type (u32, -1 = empty), +8 object id (s64, -1 = empty), the rest display state.
/// Found by searching for the seven object ids of Rubito's COMP at one stride (memdbg stride): one hit at stride 32
/// with the seven in the COMP window's order and three empty slots after them, each slot's type matching its
/// record. (A second hit at stride 160 is the COMP window's row list, with names and HP: it is not used.)
/// There is no known pointer to the list, so it is recognised by its shape: a run of slots that are each empty or
/// hold the object id and type of a record, with at least two demons (one when that one is the summoned demon).
pub fn read_comp(mem: &GameMemory, all: &[DemonRecord]) -> Option<Vec<DemonRecord>> {
    // object id -> index into `all` of the first record with that id (GroupBy(...).First())
    let mut by_id: HashMap<u32, usize> = HashMap::new();
    for (i, d) in all.iter().enumerate() {
        if d.object_id != 0 {
            by_id.entry(d.object_id).or_insert(i);
        }
    }
    if by_id.is_empty() {
        return None;
    }
    let mut best: Option<Vec<usize>> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        // 1 = a demon of ours (its index in `all`), 0 = empty, -1 = not a slot
        let slot = |q: i64| -> (i32, usize) {
            if q < 0 || q as usize + 16 > buf.len() {
                return (-1, 0);
            }
            let q = q as usize;
            let ty = u32_at(buf, q);
            let obj = i64_at(buf, q + 8);
            if ty == u32::MAX && obj == -1 {
                return (0, 0);
            }
            if obj > 0 && obj <= u32::MAX as i64 {
                if let Some(&i) = by_id.get(&(obj as u32)) {
                    if all[i].r#type as u32 == ty {
                        return (1, i);
                    }
                }
            }
            (-1, 0)
        };
        let mut p = 0usize;
        while p + 16 <= buf.len() {
            let pp = p as i64;
            p += 4;
            if slot(pp).0 != 1 || slot(pp - 32).0 == 1 {
                continue; // the first demon of a run
            }
            let mut start = pp;
            while start - pp > -9 * 32 && slot(start - 32).0 == 0 {
                start -= 32; // empty slots before it
            }
            let mut run: Vec<usize> = Vec::new();
            let mut q = start;
            for _ in 0..10 {
                let (kind, d) = slot(q);
                q += 32;
                if kind < 0 {
                    break;
                }
                if kind == 1 && !run.contains(&d) {
                    run.push(d);
                }
            }
            if run.len() < 2 && !(run.len() == 1 && all[run[0]].summoned()) {
                continue;
            }
            if best.as_ref().is_none_or(|b| run.len() > b.len()) {
                best = Some(run);
            }
        }
    }
    best.map(|b| b.into_iter().map(|i| all[i].clone()).collect())
}

fn parse(mem: &GameMemory, addr: i64, flags: i32, s: &[u8], k: &[u8], d: &[u8]) -> Option<DemonRecord> {
    // Array.IndexOf(s, 0, 8, 40) - 8
    let name_len = s[8..48].iter().position(|&b| b == 0)?;
    if name_len == 0 {
        return None;
    }
    // Encoding.ASCII.GetString: bytes above 0x7F do not decode
    let name: String = s[8..8 + name_len].iter().map(|&b| if b < 0x80 { b as char } else { '?' }).collect();
    let value: Vec<i32> = (0..21).map(|i| i16_at(s, 0xB2 + i * 4) as i32).collect();
    let bonus: Vec<i32> = (0..21).map(|i| if i <= 5 { i16_at(s, 0xB4 + i * 4) as i32 } else { 0 }).collect();

    let mut slots = vec![0i32; 8];
    for (key, val) in map_entries(mem, u32_at(k, 40) as i64, i32_at(k, 44), 512) {
        if (0..8).contains(&key) {
            slots[key as usize] = val;
        }
    }
    let acquired: Vec<i32> = map_entries(mem, u32_at(k, 68) as i64, i32_at(k, 72), 512).into_iter().map(|e| e.0).collect();

    Some(DemonRecord {
        address: addr,
        flags,
        name,
        r#type: i32_at(k, 12),
        level: s[0x84] as i32,
        object_id: u32_at(d, 16),
        hp: i32_at(s, 0x88),
        max_hp: i32_at(s, 0x8C),
        mp: u16_at(s, 0x90) as i32,
        max_mp: u16_at(s, 0x92) as i32,
        live_value: value,
        live_bonus: bonus,
        growth_type: d[30] as i32,
        reunion: (31..43).map(|i| d[i] as i8 as i32).collect(),
        mitama_type: d[43] as i8 as i32,
        mitama_rank: d[44] as i32,
        mitama_reunion: d[45..141].to_vec(),
        mitama_sets_shown: (145..193).map(|i| d[i] != 0).collect(),
        skills: slots,
        acquired,
        soul_points: u16_at(d, 248) as i32,
        force_values: Some((0..20).map(|i| i32_at(d, 256 + i * 4)).collect()),
        force_stack: Some((0..8).map(|i| u16_at(d, 336 + i * 2) as i32).collect()),
        equipment: Some(
            (0..4)
                .map(|i| {
                    let e = i32_at(d, 232 + i * 4);
                    if e > 0 { e } else { 0 }
                })
                .collect(),
        ),
    })
}

const PROGRESS_VTABLE: u32 = 0x16CB964;

/// The digitalize assists the client's own character has learned (CharacterProgress.DigitalizeAssists: bit id of a
/// 256-byte mask), or None when it can't be read. Path: own character record (flags 0x100) +24 -> progress object
/// (vtable 0x16CB964) +0x89 (unaligned) -> the mask. Found from Rubito's Foul choices (9 picks, L2-L10, exactly what
/// the Digitalization Levels window shows). Checked on read: ids stop at 648 (36 races x 18) and no level has both
/// of its two choices.
pub fn read_assists(mem: &GameMemory) -> Option<Vec<i32>> {
    read_progress(mem).map(|p| p.assists)
}

/// The own character's learned assists and skills, from its progress object (C# `DemonReader.Progress`).
///  * `digitalized_demon`: the object id of the demon the character is digitalized with right now, 0 when not
///    digitalized (progress +96 = 1 while digitalized, +104 the demon's object id or -1, +112 its race).
///  * `levels`: race id -> Digitalization Level (0 = locked), None when unreadable. See [`read_levels`].
///  * `points`: race id -> digitalize points collected towards the next level (reset on each level-up).
///  * `switched_on`: learned skills whose "on" flag is set: the toggle (switch) skills that are switched on.
///  * `completed_quests`: completed quest ids (see [`read_quests`]); None while the client hasn't loaded the list.
///  * `level`: the character's level (status object +132), None when unreadable.
///  * `expertise_extension`: Character.ExpertiseExtension (progress +456, int32): each one raises the expertise
///    points cap by 100,000 (CharacterManager::GetMaxExpertisePoints); -1 = no cap. Found on Rubito: +456 = 34, and
///    1,700,000 + 9 x 100,000 (level 95) + 34 x 100,000 = 6,000,000, the "60000.00pt" his Expertise window shows
///    (52000.00pt = his 5,200,000 points). One subject so far. None when outside -1..127.
///  * `valuables`: the character's valuables (the Valuables window, key items and the server's progress flags), see
///    [`valuables_of`]. None when the mask is empty (not received yet).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Progress {
    pub assists: Vec<i32>,
    pub skills: Option<IndexSet<i32>>,
    pub digitalized_demon: u32,
    pub levels: Option<IndexMap<i32, i32>>,
    pub points: Option<IndexMap<i32, i32>>,
    pub switched_on: Option<IndexSet<i32>>,
    pub completed_quests: Option<IndexSet<i32>>,
    pub level: Option<i32>,
    pub expertise_extension: Option<i32>,
    pub valuables: Option<IndexSet<i32>>,
}

impl Progress {
    /// Whether the character holds valuable `id`; None when the valuables couldn't be read.
    pub fn has_valuable(&self, id: i32) -> Option<bool> {
        self.valuables.as_ref().map(|v| v.contains(&id))
    }

    pub fn digitalized(&self) -> bool {
        self.digitalized_demon != 0
    }

    /// MITAMA_SET_BOOST (constants.xml): "Mitama Demon Enhancement - Iron Man" upgrades Mitama set bonuses.
    pub fn has_skill(&self, id: i32) -> bool {
        id > 0 && self.skills.as_ref().is_some_and(|s| s.contains(&id))
    }
}

/// Assists (see [`read_assists`]) and the learned skills: a std::map {head, size} at progress +412 keyed by skill id.
/// Checked on Rubito: it holds the expertise skills of reached ranks (76) and not those of unreached ranks (46 of
/// 63), and skill 5133, which his digitalized stats confirmed (boosted Mitama set bonuses). A second map of the
/// same size sits at +424. `skills` is None when the map can't be read. Levels/Points: see [`read_levels`].
pub fn read_progress(mem: &GameMemory) -> Option<Progress> {
    mem.remember("progress", || search_progress(mem)) // a snapshot asks three times
}

fn search_progress(mem: &GameMemory) -> Option<Progress> {
    let mut result: Option<Vec<i32>> = None;
    let mut skills: Option<IndexSet<i32>> = None;
    let mut on: Option<IndexSet<i32>> = None;
    let mut digi: u32 = 0;
    let mut levels: Option<IndexMap<i32, i32>> = None;
    let mut points: Option<IndexMap<i32, i32>> = None;
    let mut quests: Option<IndexSet<i32>> = None;
    let mut level: Option<i32> = None;
    let mut ext: Option<i32> = None;
    let mut valuables: Option<IndexSet<i32>> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, RECORD_VTABLE, p) {
            p = pp + 4;
            if pp + 28 > buf.len() {
                break;
            }
            if (u32_at(buf, pp + 8) & 0x100) == 0 {
                continue;
            }
            let prog = u32_at(buf, pp + 24);
            if prog < 0x10000 {
                continue;
            }
            let Some(b) = mem.read(prog as i64, 0x630) else { continue };
            if u32_at(&b, 0) != PROGRESS_VTABLE {
                continue;
            }
            let Some(ids) = assist_ids(&b[0x89..0x89 + 256]) else { continue };
            if result.as_ref().is_some_and(|r| *r != ids) {
                return None; // two characters disagree: don't guess
            }
            result = Some(ids);
            let sk = read_skill_map(mem, u32_at(&b, 412) as i64, i32_at(&b, 416));
            skills = sk.as_ref().map(|m| m.keys().copied().collect());
            on = sk.as_ref().map(|m| m.iter().filter(|(_, v)| **v).map(|(k, _)| *k).collect());
            digi = digitalized_demon_of(&b);
            (levels, points) = read_levels(mem, u32_at(&b, 128) as i64, i32_at(&b, 132));
            quests = read_quests(mem, u32_at(&b, 0x624) as i64, i32_at(&b, 0x628));
            let e = b[456] as i8 as i32; // one signed byte (-1 = no cap): Rokujuu had 0x00190016 here for 22
            ext = if (-1..=127).contains(&e) { Some(e) } else { None };
            valuables = valuables_of(&b[VALUABLES_AT..VALUABLES_AT + 128]);
            let st = mem.read(u32_at(buf, pp + 12) as i64, 136);
            level = st.and_then(|st| {
                let lv = st[132] as i32;
                if (1..=99).contains(&lv) { Some(lv) } else { None }
            });
        }
    }
    result.map(|assists| Progress {
        assists,
        skills,
        digitalized_demon: digi,
        levels,
        points,
        switched_on: on,
        completed_quests: quests,
        level,
        expertise_extension: ext,
        valuables,
    })
}

/// Where the valuables mask sits in the progress object.
const VALUABLES_AT: usize = 500;

/// The valuables: a 128-byte bit mask at progress +500, valuable id n = byte n / 8, bit n % 8 (the server's
/// CharacterProgress.Valuables and CharacterManager::ConvertIDToMaskValues; it arrives with the login). Found on
/// 2026-11-25 with a fresh login on Rubito and Rokujuu, Valuables window open: the only 128 bytes of the object
/// holding Demonic Compendium Ver. 2.0 (251), Digitalize System Novice (686) and Shiho's expertise releases
/// (659/660), different on the two characters (102 and 108 valuables). Checked: the expertise-cap valuables plus the
/// cap quests add up exactly to each one's ExpertiseExtension (Rubito 34, Rokujuu 22; see calc::expertise_cap).
/// None when no bit is set (not received yet).
pub(crate) fn valuables_of(mask: &[u8]) -> Option<IndexSet<i32>> {
    let mut res = IndexSet::new();
    for (i, b) in mask.iter().enumerate() {
        for s in 0..8 {
            if b & (1 << s) != 0 {
                res.insert((i * 8 + s) as i32);
            }
        }
    }
    if res.is_empty() { None } else { Some(res) }
}

/// The object id of the demon the character is digitalized with, from the progress object's bytes (0 = not digitalized):
/// +96 is a flag, +104 the demon's object id (0xFFFFFFFF when none). The flag is ONE byte with leftovers above it
/// (Rubito's client, not digitalized: +96 = 0x55555500, +112 = 0x55555500 for the race, +104 = 0xFFFFFFFF), like the
/// level and the expertise extension, so it is read at its real width: an int32 compare with 1 would miss a
/// digitalized 0x55555501.
fn digitalized_demon_of(b: &[u8]) -> u32 {
    let obj = u32_at(b, 104);
    if b[96] == 1 && obj != 0 && obj != u32::MAX { obj } else { 0 }
}

/// Completed quests: a std::map {head, size} at progress +0x624, keyed by quest id (node +12 u16; the 2 bytes after
/// it are garbage; +16 holds the id again with 0xFFFF above it). Found with the Quest Completion Bonuses window open
/// on Rubito: 60 entries (Acts 0-20 and XX, Side a/b, 28 NORMAL, 4 REPEATABLE); the NORMAL + REPEATABLE count, 32, is
/// the window's "Quests Completed" (the server's quest-bonus count, CharacterState::UpdateQuestState). The client
/// seems to fill it only once the quest list has been asked for (not seen before the window was opened). None when
/// empty or unreadable.
fn read_quests(mem: &GameMemory, head: i64, size: i32) -> Option<IndexSet<i32>> {
    if head < 0x10000 || size <= 0 || size > 20000 {
        return None;
    }
    let hb = mem.read(head, 8)?;
    let mut res: IndexSet<i32> = IndexSet::new();
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n < 0x10000 || n == head || !seen.insert(n) || seen.len() as i64 > size as i64 + 1 {
            continue;
        }
        let nb = mem.read(n, 16)?;
        res.insert(u16_at(&nb, 12) as i32);
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if res.len() as i32 == size { Some(res) } else { None }
}

/// The learned-skill map (progress +412/+416): skill id -> switched on. Node +12 skill id, +16 the id again, then the
/// skill's state; byte +33 is 1 while a toggle (switch) skill is on. Found by switching Extended Scope off (its +33
/// went 1 -> 0, nothing else near it changed) and checked against the user's toggles: Gunslinger, Firestarter,
/// Frontier Justice, Guardian Angel, Lord of Armor, Master of Guns, Criminal Bullet, Blaspheme on = 1; passives 0.
/// None when the map can't be read whole.
fn read_skill_map(mem: &GameMemory, head: i64, size: i32) -> Option<IndexMap<i32, bool>> {
    if head < 0x10000 || size <= 0 || size > 20000 {
        return None;
    }
    let hb = mem.read(head, 8)?;
    let mut res: IndexMap<i32, bool> = IndexMap::new();
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n < 0x10000 || n == head || !seen.insert(n) || seen.len() as i64 > size as i64 + 1 {
            continue;
        }
        let Some(nb) = mem.read(n, 34) else { continue };
        res.insert(i32_at(&nb, 12), nb[33] == 1);
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if res.len() as i32 == size { Some(res) } else { None }
}

/// The Digitalization Level and points of every race (CharacterProgress.DigitalizeLevels / DigitalizePoints, as the
/// server's PACKET_DIGITALIZE_POINTS sends them): a std::map {head, size} at progress +128, keyed by race id.
/// Node: +12 race id (u8; the 3 bytes after it are garbage), +16 level (s8), +20 points (s32) towards the next
/// level; the level's GuardianLevelData nextXP is the points it takes (points restart at 0 on a level-up, 0 at
/// the cap). It holds 44 entries on Rubito: the 43 races of GuardianLevelData plus a race-0 entry (level 0).
/// Found and checked on Rubito with the Digitalization Levels window open: every Law race (Avian 8 with 33906000
/// of 56918900 points = the window's 59.56%, truncated; Entity 1 and Raptor 5 at 0.00%; the others 0), the Chaos
/// races shown (Foul, Brute, Femme, Earth Mother 10, Fallen Angel 5, Nation Ruler 0) and the learned assists (no
/// race has an assist above its level). Not yet checked: whether the client fills the map before the window has
/// been opened once in a session (the server sends it when the client asks). Both are None when unreadable.
#[allow(clippy::type_complexity)]
fn read_levels(mem: &GameMemory, head: i64, size: i32) -> (Option<IndexMap<i32, i32>>, Option<IndexMap<i32, i32>>) {
    if head < 0x10000 || size <= 0 || size > 256 {
        return (None, None);
    }
    let Some(hb) = mem.read(head, 8) else { return (None, None) };
    let mut levels: IndexMap<i32, i32> = IndexMap::new();
    let mut points: IndexMap<i32, i32> = IndexMap::new();
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n < 0x10000 || n == head || !seen.insert(n) || seen.len() as i64 > size as i64 + 1 {
            continue;
        }
        let Some(nb) = mem.read(n, 24) else { return (None, None) };
        let race = nb[12] as i32;
        let lvl = nb[16] as i8 as i32;
        let pts = i32_at(&nb, 20);
        if lvl < 0 || lvl > 15 || pts < 0 {
            return (None, None); // not this map: don't guess
        }
        if race != 0 {
            levels.insert(race, lvl);
            points.insert(race, pts);
        }
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if seen.len() as i32 == size { (Some(levels), Some(points)) } else { (None, None) }
}

fn assist_ids(mask: &[u8]) -> Option<Vec<i32>> {
    let bit = |id: usize| (mask[id / 8] & (1 << (id % 8))) != 0;
    if bit(0) || mask.iter().skip(82).any(|&x| x != 0) || (mask[81] & 0xFE) != 0 {
        return None;
    }
    // Assist id = race slot (1-36) + 36 * j; level L offers j = 2(L-2) and 2(L-2)+1, one of which can be picked.
    for s in 1..=36usize {
        for k in 0..9usize {
            if bit(s + 72 * k) && bit(s + 72 * k + 36) {
                return None;
            }
        }
    }
    Some((1..=648usize).filter(|&i| bit(i)).map(|i| i as i32).collect())
}

/// The (key, first value int) pairs of an old-MSVC std::map: node +0 left, +4 parent, +8 right, +12 key, +16 value.
/// Sorted by key; empty when the map can't be read whole.
fn map_entries(mem: &GameMemory, head: i64, size: i32, max_size: i32) -> Vec<(i32, i32)> {
    let mut res: Vec<(i32, i32)> = Vec::new();
    if head < 0x10000 || size <= 0 || size > max_size {
        return res;
    }
    let Some(hb) = mem.read(head, 8) else { return res };
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n < 0x10000 || n == head || !seen.insert(n) || seen.len() as i64 > size as i64 + 1 {
            continue;
        }
        let Some(nb) = mem.read(n, 20) else { continue };
        res.push((i32_at(&nb, 12), i32_at(&nb, 16)));
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if res.len() as i32 == size {
        res.sort_by_key(|e| e.0); // OrderBy is stable
        res
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A progress object's first 112 bytes with the digitalize fields set the way Rubito's client holds them.
    fn progress_head(flag_word: u32, demon: u32) -> Vec<u8> {
        let mut b = vec![0u8; 0x630];
        b[96..100].copy_from_slice(&flag_word.to_le_bytes());
        b[104..108].copy_from_slice(&demon.to_le_bytes());
        b
    }

    /// Rubito, 2026-11-24, not digitalized: +96 = 0x55555500 (a zero flag byte, filler above), +104 = 0xFFFFFFFF.
    /// A digitalized character has the flag byte 1 and the demon's object id; the filler above the flag must not matter.
    #[test]
    fn digitalized_flag_is_one_byte() {
        assert_eq!(digitalized_demon_of(&progress_head(0x5555_5500, u32::MAX)), 0);
        assert_eq!(digitalized_demon_of(&progress_head(0x5555_5500, 2181063)), 0, "flag byte 0: not digitalized");
        assert_eq!(digitalized_demon_of(&progress_head(0x5555_5501, 2181063)), 2181063, "filler above the flag byte");
        assert_eq!(digitalized_demon_of(&progress_head(1, 2181063)), 2181063, "clean upper bytes");
        assert_eq!(digitalized_demon_of(&progress_head(1, u32::MAX)), 0, "flag set but no demon");
        assert_eq!(digitalized_demon_of(&progress_head(1, 0)), 0);
    }

    fn mask_with(ids: &[usize]) -> Vec<u8> {
        let mut m = vec![0u8; 256];
        for &i in ids {
            m[i / 8] |= 1 << (i % 8);
        }
        m
    }

    #[test]
    fn assist_ids_reads_the_bits() {
        assert_eq!(assist_ids(&mask_with(&[1, 40, 648])), Some(vec![1, 40, 648]));
        assert_eq!(assist_ids(&mask_with(&[])), Some(vec![]));
    }

    #[test]
    fn assist_ids_rejects_what_is_not_a_mask() {
        assert_eq!(assist_ids(&mask_with(&[0])), None); // bit 0 never set
        assert_eq!(assist_ids(&mask_with(&[650])), None); // past 648
        assert_eq!(assist_ids(&mask_with(&[3, 3 + 36])), None); // both choices of one level
        assert_eq!(assist_ids(&mask_with(&[3, 3 + 72])), Some(vec![3, 75])); // different levels are fine
    }
}
