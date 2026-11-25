//! Port of `src/Game/ProgressReader.cs`: what the title / clan / quest-bonus / compendium bonuses need (see
//! `calc::character_extras`). Read-only, like every reader.

// The checks keep the C# shape (`x < 1 || x > 99`, index loops, nested ifs) so each line can be compared with the original.
#![allow(clippy::manual_range_contains, clippy::needless_range_loop, clippy::collapsible_if)]

use std::sync::LazyLock;

use indexmap::IndexSet;
use regex::Regex;

use super::client_reader::sjis;
use super::demon_reader;
use crate::memory::{GameMemory, find_bytes, find_u32, u32_at};

/// C# `ProgressState`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressState {
    /// The clan's name as the client shows it under the character's name: own character record +12 -> status
    /// object (vtable 0x16CB904) +48, NUL-terminated Shift-JIS, brackets stripped ("[Sukeban]" -> "Sukeban"). None when
    /// the character has no clan or the text can't be read. (This field is the clan, not the title.)
    pub clan_name: Option<String>,
    /// The character's learned skills (progress object +412, `demon_reader::read_progress`). The clan level skills
    /// (constants.xml CLAN_LEVEL_SKILLS, 7001-7024) are in it, so the clan level is worked out from it.
    pub learned_skills: Option<IndexSet<i32>>,
    /// Completed quests of type NORMAL or REPEATABLE (the server's quest-bonus count). Not found in the client's
    /// memory: None unless the caller sets it (from the user, or CharacterExtras.QuestBonusCount on a completed-quest mask).
    pub quest_bonus_count: Option<i32>,
    /// Unique Demonic Compendium entries (the server's compendium count). Not found in the client's memory: None
    /// unless the caller sets it (from the user, or CharacterExtras.CompendiumCount on the 512-byte devil-book mask).
    pub compendium_count: Option<i32>,
    /// Whether the account has Demonic Compendium v2 (valuable 251): the server gives no compendium bonus without
    /// it. Not read here: the caller passes it (the profile's, saved from the valuables, demon_reader::Progress); None
    /// = assumed yes when a count is given.
    pub has_compendium_v2: Option<bool>,
}

const CHARACTER_VTABLE: u32 = 0x16CB8F4;
const STATUS_VTABLE: u32 = 0x16CB904;

/// The state the extra bonuses need, from the client's own character. Clan name and learned skills come from memory;
/// the counts are taken from the arguments (None = unknown): the quest count from DemonReader's completed-quest map
/// (via CharacterExtras.QuestBonusCount) or the saved profile, the compendium count from the profile (not found in
/// memory yet; see HANDOVER). Returns None when the own character can't be found.
pub fn read(
    mem: &GameMemory,
    quest_bonus_count: Option<i32>,
    compendium_count: Option<i32>,
    has_compendium_v2: Option<bool>,
) -> Option<ProgressState> {
    let mut clan: Option<String> = None;
    let mut found = false;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, CHARACTER_VTABLE, p) {
            p = pp + 4;
            if pp + 16 > buf.len() {
                break;
            }
            if (u32_at(buf, pp + 8) & 0x100) == 0 {
                continue;
            }
            let st = u32_at(buf, pp + 12);
            let s = if st > 0x10000 { mem.read(st as i64, 96) } else { None };
            let Some(s) = s else { continue };
            if u32_at(&s, 0) != STATUS_VTABLE {
                continue;
            }
            let c = clan_text(&s[48..96]);
            if found && c != clan {
                return None; // two own records disagree: don't guess
            }
            clan = c;
            found = true;
        }
    }
    if !found {
        return None;
    }
    Some(ProgressState {
        clan_name: clan,
        learned_skills: demon_reader::read_progress(mem).and_then(|p| p.skills),
        quest_bonus_count,
        compendium_count,
        has_compendium_v2,
    })
}

static COMPENDIUM_HEADER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Demons: (\d{1,4})/(\d{1,4})").expect("static pattern"));

/// The Demonic Compendium's registered count, from the window's own header text "Demons: 60/339" (UTF-16, in the
/// UI's string heap; found on Rubito with the window open). The compendium data itself wasn't found in memory (no
/// DevilBook bit mask with the visible demons' bits, no map keyed by demon/entry id), so this is read only while that
/// text is in memory, i.e. once the window has been opened. Counts only grow, so the largest copy wins. None when
/// there is no such text.
pub fn read_compendium_count(mem: &GameMemory) -> Option<i32> {
    let pat: Vec<u8> = "Demons: ".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut best: Option<i32> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut at = find_bytes(buf, &pat);
        while let Some(p) = at {
            // Encoding.Unicode.GetString(buf, p, Math.Min(64, buf.Length - p) & ~1)
            let n = (64usize.min(buf.len() - p)) & !1;
            let units: Vec<u16> = buf[p..p + n].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let text = String::from_utf16_lossy(&units);
            if let Some(m) = COMPENDIUM_HEADER.captures(&text) {
                // (.NET `\d` also takes non-ASCII digits and then int.Parse throws; text like that is no header.)
                if let (Ok(a), Ok(b)) = (m[1].parse::<i32>(), m[2].parse::<i32>()) {
                    if a <= b {
                        best = Some(best.unwrap_or(0).max(a));
                    }
                }
            }
            at = find_bytes(&buf[p + 2..], &pat).map(|next| p + 2 + next);
        }
    }
    best
}

fn clan_text(b: &[u8]) -> Option<String> {
    let n = b.iter().position(|&x| x == 0)?;
    if n == 0 {
        return None;
    }
    let mut t = sjis(&b[..n]).trim().to_string();
    if t.starts_with('[') && t.ends_with(']') && t.chars().count() >= 2 {
        let inner: String = t.chars().skip(1).take(t.chars().count() - 2).collect();
        t = inner.trim().to_string();
    }
    if t.is_empty() || t.chars().any(|c| c.is_control()) { None } else { Some(t) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clan_text_strips_brackets() {
        let mut b = [0u8; 48];
        b[..9].copy_from_slice(b"[Sukeban]");
        assert_eq!(clan_text(&b).as_deref(), Some("Sukeban"));
        let mut c = [0u8; 48];
        c[..2].copy_from_slice(b"[]");
        assert_eq!(clan_text(&c), None);
        assert_eq!(clan_text(&[0u8; 48]), None);
    }
}
