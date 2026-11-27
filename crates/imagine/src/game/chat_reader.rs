//! The chat window's line lists: the damage log (every hit the own character and its partner land) and the XP lines.
//! Read-only, like every reader.
//!
//! Layout (found 2026-11-27 from a lead in the CLEAR-B Overlay, github.com/LacrimaGames/CLEAR-B-Overlay `dps\log.py`,
//! and proved on Rubito; LIVE-DATA.md):
//!
//!   chat window   `MiUI_Window_Chat`, vtable 0x0170A80C, one object per client, in a private VirtualAlloc block
//!                 (so a heap census misses it; a vtable sweep finds it in ~0.7 s)
//!     +0x3FC      list head: own XP lines, channel 14 ("You have gained 485 experience points!")
//!     +0x408      list head: partner XP lines, channel 15 ("Alilat has gained 335 experience points!")
//!     +0x414      list head: the damage log, channel 16
//!                 (+0x3E4 and +0x3F0 are two more heads, empty in every read so far)
//!   list head     a node of its own: +0 first line, +4 last line (both point back at the head when empty)
//!   line          +0 next, +4 prev, +8 channel (u32), +12 running number (u32, one count shared by every channel),
//!                 +16 the text, NUL-terminated (ASCII on the English client; read as Shift-JIS)
//!
//! Checked: after one fight the damage list grew from #820 to #885 with Rubito's hits ("Rubito dealt 37816 damage to
//! Angel", "Also dealt 37816 damage with Technical Attack and 37816 damage with Pursuit") and Alilat's ("Alilat dealt
//! 586 damage to Angel"); the XP lines took the numbers in between. About 100 damage lines are kept. The log holds
//! only the own character's and its partner's damage, never other players'. The numbers match newmoon-kb's caps and
//! follow-up rules (kb\damage.md 7 and 10: 9,999 normal hits, LB hits above 30,000, TA and pursuit copying the hit).

use std::sync::LazyLock;

use regex::Regex;

use super::client_reader::sjis;
use crate::memory::{GameMemory, find_u32, u32_at};

pub const CHAT_WINDOW_VTABLE: u32 = 0x0170_A80C;
/// List heads in the chat window.
pub const OWN_XP_AT: i64 = 0x3FC;
pub const PARTNER_XP_AT: i64 = 0x408;
pub const DAMAGE_LOG_AT: i64 = 0x414;
/// A line's channel.
pub const OWN_XP_CHANNEL: u32 = 14;
pub const PARTNER_XP_CHANNEL: u32 = 15;
pub const DAMAGE_CHANNEL: u32 = 16;

const LINE_HEADER: usize = 16;
const TEXT_MAX: usize = 200;
/// A walk never goes further back than this (the game keeps about 100 damage lines).
const MOST_LINES: usize = 400;

/// One line of a chat list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLine {
    /// The running number (shared by every channel: a list's numbers have gaps).
    pub number: u32,
    pub channel: u32,
    pub text: String,
}

/// The chat window's address, when exactly one object has its vtable (None: not found, or more than one).
pub fn find_chat_window(mem: &GameMemory) -> Option<i64> {
    let mut found: Option<i64> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x0200_0000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, CHAT_WINDOW_VTABLE, p) {
            p = pp + 4;
            let at = r.base + pp as i64;
            if found.is_some_and(|f| f != at) {
                return None;
            }
            found = Some(at);
        }
    }
    found
}

/// The lines of the list whose head is at `window + list_at`, oldest first, only those numbered after `after`
/// (all of them for None). None when `window` is no longer the chat window (the client closed or logged out: find it
/// again) or the list can't be read.
pub fn read_list(mem: &GameMemory, window: i64, list_at: i64, after: Option<u32>) -> Option<Vec<ChatLine>> {
    let w = mem.read(window, 4)?;
    if u32_at(&w, 0) != CHAT_WINDOW_VTABLE {
        return None;
    }
    let head = u32_at(&mem.read(window + list_at, 4)?, 0) as i64;
    if head < 0x10000 {
        return None;
    }
    let h = mem.read(head, 8)?;
    let (first, last) = (u32_at(&h, 0) as i64, u32_at(&h, 4) as i64);
    let mut lines: Vec<ChatLine> = Vec::new();
    let mut node = last;
    for _ in 0..MOST_LINES {
        if node < 0x10000 || node == head {
            break;
        }
        let b = mem.read(node, LINE_HEADER + TEXT_MAX).or_else(|| mem.read(node, LINE_HEADER))?;
        let (prev, channel, number) = (u32_at(&b, 4) as i64, u32_at(&b, 8), u32_at(&b, 12));
        if after.is_some_and(|a| number <= a) {
            break;
        }
        let text = &b[LINE_HEADER..];
        let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
        lines.push(ChatLine { number, channel, text: sjis(&text[..end]) });
        if node == first {
            break;
        }
        node = prev;
    }
    lines.reverse();
    Some(lines)
}

/// Whose hit a damage line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    Own,
    Partner,
}

/// The part of a hit that isn't a follow-up.
pub const MAIN_HIT: &str = "Hit";

/// One amount of damage from the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The line's running number (a main hit and its "Also dealt" follow-ups have consecutive numbers).
    pub number: u32,
    /// The name the line gives (the character's, or the partner's).
    pub who: String,
    pub source: Source,
    /// [`MAIN_HIT`], or the follow-up's name as the game writes it ("Technical Attack", "Pursuit").
    pub part: String,
    pub amount: i64,
    pub target: String,
}

static MAIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?) dealt (\d+) damage to (.+?)\s*$").unwrap());
static ALSO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Also dealt (.+?)\s*$").unwrap());
static EXTRA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+) damage with (.+?)\s*$").unwrap());

/// Damage lines into [`Hit`]s. An "Also dealt" line belongs to the main line before it, so the parser remembers the
/// last one across calls (a poll can end between the two).
#[derive(Debug, Clone)]
pub struct DamageParser {
    own_name: String,
    last: Option<(String, Source, String)>,
}

impl DamageParser {
    pub fn new(own_name: &str) -> DamageParser {
        DamageParser { own_name: own_name.to_string(), last: None }
    }

    /// The hits of one line; empty for a line of another channel or a shape not known yet.
    pub fn line(&mut self, line: &ChatLine) -> Vec<Hit> {
        if line.channel != DAMAGE_CHANNEL {
            return Vec::new();
        }
        if let Some(m) = MAIN.captures(&line.text)
            && !line.text.starts_with("Also dealt ")
        {
            let who = m[1].to_string();
            let source = if who == self.own_name { Source::Own } else { Source::Partner };
            let target = m[3].to_string();
            let amount = m[2].parse::<i64>().unwrap_or(0);
            self.last = Some((who.clone(), source, target.clone()));
            return vec![Hit { number: line.number, who, source, part: MAIN_HIT.to_string(), amount, target }];
        }
        let (Some(m), Some((who, source, target))) = (ALSO.captures(&line.text), self.last.as_ref()) else { return Vec::new() };
        m[1].split(" and ")
            .filter_map(|part| EXTRA.captures(part.trim()))
            .map(|e| Hit {
                number: line.number,
                who: who.clone(),
                source: *source,
                part: e[2].to_string(),
                amount: e[1].parse::<i64>().unwrap_or(0),
                target: target.clone(),
            })
            .collect()
    }
}

/// Following the damage log poll after poll: keeps the chat window's address (finding it again when it goes away)
/// and the newest line number already handed out, so each hit comes out once.
#[derive(Debug, Clone)]
pub struct DamageLog {
    window: Option<i64>,
    seen: Option<u32>,
    parser: DamageParser,
}

/// What one poll found.
#[derive(Debug, Clone, Default)]
pub struct DamagePoll {
    pub hits: Vec<Hit>,
    /// Damage lines whose shape the parser doesn't know (kept as they are, so a new shape can be looked at).
    pub unknown: Vec<ChatLine>,
    /// The chat window couldn't be found or read this time.
    pub lost: bool,
}

impl DamageLog {
    /// `own_name`: the logged-in character, to tell its lines from the partner's.
    pub fn new(own_name: &str) -> DamageLog {
        DamageLog { window: None, seen: None, parser: DamageParser::new(own_name) }
    }

    /// The hits written since the last poll, oldest first. With `skip_existing`, the first poll only notes where the
    /// log is and hands out nothing (what was there before isn't counted).
    pub fn poll(&mut self, mem: &GameMemory, skip_existing: bool) -> DamagePoll {
        if self.window.is_none() {
            self.window = find_chat_window(mem);
        }
        let Some(window) = self.window else { return DamagePoll { lost: true, ..Default::default() } };
        let Some(mut lines) = read_list(mem, window, DAMAGE_LOG_AT, self.seen) else {
            self.window = None;
            return DamagePoll { lost: true, ..Default::default() };
        };
        // The game counting from the start again (a new session): its newest line is below the one seen.
        if self.seen.is_some() && lines.is_empty() {
            if let Some(all) = read_list(mem, window, DAMAGE_LOG_AT, None)
                && all.last().is_some_and(|l| Some(l.number) < self.seen)
            {
                lines = all;
            }
        }
        let first = self.seen.is_none();
        if let Some(l) = lines.last() {
            self.seen = Some(l.number);
        }
        if first && skip_existing {
            if self.seen.is_none() {
                self.seen = Some(0);
            }
            return DamagePoll::default();
        }
        let mut out = DamagePoll::default();
        for l in &lines {
            let hits = self.parser.line(l);
            if hits.is_empty() && l.channel == DAMAGE_CHANNEL {
                out.unknown.push(l.clone());
            }
            out.hits.extend(hits);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(number: u32, text: &str) -> ChatLine {
        ChatLine { number, channel: DAMAGE_CHANNEL, text: text.to_string() }
    }

    #[test]
    fn parses_hits_and_follow_ups() {
        let mut p = DamageParser::new("Rubito");
        let h = p.line(&line(848, "Rubito dealt 37816 damage to Angel"));
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].source, h[0].part.as_str(), h[0].amount, h[0].target.as_str()), (Source::Own, MAIN_HIT, 37816, "Angel"));
        let h = p.line(&line(849, "Also dealt 37816 damage with Technical Attack and 37816 damage with Pursuit"));
        let parts: Vec<(&str, i64, &str)> = h.iter().map(|x| (x.part.as_str(), x.amount, x.target.as_str())).collect();
        assert_eq!(parts, vec![("Technical Attack", 37816, "Angel"), ("Pursuit", 37816, "Angel")]);
        assert!(h.iter().all(|x| x.source == Source::Own && x.number == 849));
        // a single follow-up, with the trailing space the game writes
        let h = p.line(&line(815, "Also dealt 9999 damage with Pursuit "));
        assert_eq!((h.len(), h[0].part.as_str(), h[0].amount), (1, "Pursuit", 9999));
    }

    #[test]
    fn partner_lines_and_targets_with_spaces() {
        let mut p = DamageParser::new("Rubito");
        let h = p.line(&line(877, "Alilat dealt 586 damage to Lucky Angel"));
        assert_eq!((h[0].source, h[0].who.as_str(), h[0].target.as_str()), (Source::Partner, "Alilat", "Lucky Angel"));
        let h = p.line(&line(878, "Also dealt 200 damage with Pursuit"));
        assert_eq!((h[0].source, h[0].target.as_str()), (Source::Partner, "Lucky Angel"));
    }

    #[test]
    fn unknown_and_orphan_lines_give_nothing() {
        let mut p = DamageParser::new("Rubito");
        assert!(p.line(&line(1, "Also dealt 9999 damage with Pursuit")).is_empty(), "no main line before it");
        assert!(p.line(&line(2, "Something else happened")).is_empty());
        let xp = ChatLine { number: 3, channel: OWN_XP_CHANNEL, text: "You have gained 485 experience points!".into() };
        assert!(p.line(&xp).is_empty());
    }
}
