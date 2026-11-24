//! Reads a crystal's own description (the client's text) into effects, for the tarot and soul crystals whose tokusei the
//! server data, the website file and the client's per-tokusei text cannot give.
//!
//! The client has no tokusei values, only display text, and for a crystal that text is one block shared by all its tokusei
//! (there is no way to tell which id carries which line). So the whole block is read once, clause by clause, and every clause
//! stands on its own: the ones understood become effects, the ones not become text-only notes (`value` None, `cond`
//! "unparsed", "from the game's text"), which the totals never count. One clause the reader cannot read never drops the others.
//!
//! The wording, as the game's own crystals use it (the server's soul 287 "Avoiding the Light" has the same text as data):
//!   "While Digitalized"               counted only while digitalized
//!   "Player/Demon's X"                the character and the partner / digitalize demon (a self and a partner effect)
//!   "Player's X"                      the character only;  "Demon's X", "Partner's X", "Demon Partner's X"   the partner only
//!   "For Player and Demon Partner"    a heading: the lines after it (up to a blank line) are for both
//!   "Damage to Neutral enemies +20%"  situational (depends on the enemy)
//!   "Against LAW Enemies,", "When using X skills:", "When your Partner uses X:"   situational, the condition kept as the note
//!   "If Player is LAW or NEUTRAL:"    decided from the character's alignment
//!   "If Player has Pursuit Class 9:", "... Sharpshooter Class 8 Rank 4,"   decided from the expertise ranks when they are
//!                                     known, otherwise situational with the requirement as the note
//! A line ending in "," or "and", or one that starts in lower case, continues the line before it; a line ending in ":" (a
//! condition on its own) is a heading for the lines after it, up to a blank line.

use super::effect::{rx, Effect};

/// The note every effect and note read from the text carries.
pub const FROM_TEXT: &str = "from the game's text";

/// What the text needs to know about the character.
pub struct Rules<'a> {
    /// The alignment in upper case (LAW, NEUTRAL, CHAOS).
    pub alignment: &'a str,
    /// The rank of the expertise with this name (class x 10 + rank, 90 = Class 9), `None` when the ranks are not known (or no
    /// such expertise): the requirement is then kept as a note, not decided.
    pub rank_of: &'a dyn Fn(&str) -> Option<i32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Who {
    Own,
    Partner,
    Both,
}

#[derive(Clone, Default)]
struct Ctx {
    who: Option<Who>,
    digitalized: bool,
    inactive: bool,
    situational: bool,
    notes: Vec<String>,
}

impl Ctx {
    fn note(&mut self, n: String) {
        if !n.is_empty() && !self.notes.contains(&n) {
            self.notes.push(n);
        }
    }
}

fn clean(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{3000}').to_string()
}

/// The text's lines as the game shows them (breaks are `\n` or ` / `), wrapped lines joined, blank lines kept (they end a heading).
fn logical_lines(text: &str) -> Vec<String> {
    let raw: Vec<String> = text.split('\n').flat_map(|l| l.split(" / ")).map(clean).collect();
    let mut out: Vec<String> = Vec::new();
    for l in raw {
        if l.is_empty() {
            out.push(String::new());
            continue;
        }
        let join = match out.last() {
            Some(p) if !p.is_empty() => p.ends_with(',') || p.ends_with(" and") || l.starts_with(|c: char| c.is_lowercase()) || l.starts_with('(') || l.starts_with("+("),
            _ => false,
        };
        if join {
            let last = out.last_mut().expect("a previous line");
            last.push(' ');
            last.push_str(&l);
        } else {
            out.push(l);
        }
    }
    out
}

/// Takes the leading conditions and "who" words off a clause into `c`; returns what is left.
fn peel(line: &str, rules: &Rules, c: &mut Ctx) -> String {
    let mut s = clean(line);
    loop {
        let before = s.clone();
        if let Some(m) = rx::m(&s, r"^While Digitalized\s*[:,]?\s*(.*)$") {
            c.digitalized = true;
            s = m[1].to_string();
        } else if let Some(m) = rx::m(&s, r"^If [Pp]layer is ((?:LAW|NEUTRAL|CHAOS)(?:\s*(?:,|or)\s*(?:LAW|NEUTRAL|CHAOS))*)\s*[:,]\s*(.*)$") {
            let want: Vec<String> = rx::split(&m[1], r"\s*(?:,|\bor\b)\s*").iter().map(|w| w.trim().to_uppercase()).filter(|w| !w.is_empty()).collect();
            if !want.iter().any(|w| *w == rules.alignment.to_uppercase()) {
                c.inactive = true;
            }
            c.note(format!("if you are {}", want.join(" or ")));
            s = m[2].to_string();
        } else if let Some(m) = rx::m(&s, r"^If [Pp]layer has ([A-Za-z][A-Za-z' .\-]*?) Class (\d+)(?:\s+Rank (\d+))?(?: or more)?\s*[:,]\s*(.*)$") {
            let (name, class, rank) = (m[1].to_string(), m[2].parse::<i32>().unwrap_or(0), m.get(3).and_then(|r| r.as_str().parse::<i32>().ok()).unwrap_or(0));
            requirement(c, rules, &name, class, rank);
            s = m[4].to_string();
        } else if let Some(m) = rx::m(&s, r"^If [Pp]layer has Class (\d+) (.+?)(?:\s+Rank (\d+))? or more\s*[:,]\s*(.*)$") {
            let (name, class, rank) = (m[2].to_string(), m[1].parse::<i32>().unwrap_or(0), m.get(3).and_then(|r| r.as_str().parse::<i32>().ok()).unwrap_or(0));
            requirement(c, rules, &name, class, rank);
            s = m[4].to_string();
        } else if let Some(m) = rx::m(&s, r"^When your Partner uses (.+?)\s*:\s*(.*)$") {
            c.situational = true;
            c.note(format!("when your partner uses {}", clean(&m[1])));
            s = m[2].to_string();
        } else if let Some(m) = rx::m(&s, r"^When using (.+?)\s*[:,]\s*(.*)$") {
            c.situational = true;
            c.note(format!("when using {}", clean(&m[1])));
            s = m[2].to_string();
        } else if let Some(m) = rx::m(&s, r"^Against (LAW|NEUTRAL|CHAOS) [Ee]nem(?:y|ies)\s*[:,]?\s*(.*)$") {
            c.situational = true;
            c.note(format!("vs {} enemies", m[1].to_uppercase()));
            s = m[2].to_string();
        } else if let Some(m) = rx::m(&s, r"^(?:For|To) (?:the )?Player and (?:the )?(?:Demon )?Partner\s*[:,]?\s*(.*)$") {
            c.who = Some(Who::Both);
            s = m[1].to_string();
        } else if let Some(m) = rx::m(&s, r"^(?:For|To) (?:the )?(?:Demon )?Partner\s*[:,]\s*(.*)$") {
            c.who = Some(Who::Partner);
            s = m[1].to_string();
        } else if let Some(m) = rx::m(&s, r"^Player/Demon's\s+(.*)$") {
            c.who = Some(Who::Both);
            s = m[1].to_string();
        } else if let Some(m) = rx::m(&s, r"^Player's\s+(.*)$") {
            c.who = Some(Who::Own);
            s = m[1].to_string();
        } else if let Some(m) = rx::m(&s, r"^(?:Demon Partner's|Demon's|Partner's)\s+(.*)$") {
            c.who = Some(Who::Partner);
            s = m[1].to_string();
        }
        s = clean(&s);
        if s == before {
            return s;
        }
    }
}

/// "If Player has <expertise> Class N [Rank R]": decided from the ranks when known, else a situational note.
fn requirement(c: &mut Ctx, rules: &Rules, name: &str, class: i32, rank: i32) {
    let need = class * 10 + rank;
    let what = if rank > 0 { format!("Class {class} Rank {rank} {name}") } else { format!("Class {class} {name}") };
    match (rules.rank_of)(name.trim()) {
        Some(have) => {
            if have < need {
                c.inactive = true;
            }
            c.note(format!("needs {what} (you have {}-{})", have / 10, have % 10));
        }
        None => {
            c.situational = true;
            c.note(format!("needs {what}"));
        }
    }
}

/// "A, B and C damage suffered -10%" into "A damage suffered -10%", "B ...", "C ..." (a shared ending is given to the parts that lack one).
fn split_parts(body: &str) -> Vec<String> {
    let body = rx::rep(body, r"\s+and\s+", ", ");
    let mut parts: Vec<String> = rx::split(&body, r",\s*").iter().map(|p| clean(p)).filter(|p| !p.is_empty()).collect();
    if parts.len() > 1 {
        let has_val = |p: &str| rx::is(p, r"[+\-]\s?\d");
        let last = parts[parts.len() - 1].clone();
        if has_val(&last) && parts[..parts.len() - 1].iter().all(|p| !has_val(p) && p.split_whitespace().count() <= 3) {
            if let Some(m) = rx::m(&last, r"^(.*?\S\s+)(\b(?:damage|power|chance|boost|resist(?:ance)?|cap|rate)\b.*)$") {
                let suffix = m[2].to_string();
                let n = parts.len();
                for p in parts.iter_mut().take(n - 1) {
                    *p = format!("{p} {suffix}");
                }
            }
        }
    }
    parts
}

/// The game's wording of a label the text writes another way ("Close Range damage done" is "Close-Range Damage Dealt").
fn canon_label(label: &str) -> String {
    let l = rx::rep(label, r"\s+", " ");
    let l = rx::rep(&l, r"\b(Close|Long)[ -]Range\b", "$1-Range");
    let l = rx::rep(&l, r"\b[Dd]amage done\b", "Damage Dealt");
    clean(&l)
}

/// One part of a clause: "label +N[%]" or "+N[%] label" or "Damage to X enemies +N%".
struct Part {
    label: String,
    value: f64,
    pct: bool,
    situational: Option<String>,
}

fn read_part(p: &str) -> Option<Part> {
    let p = rx::rep(p, r"([+\-])\s+(\d)", "${1}${2}");
    let num = |s: &str| -> Option<f64> {
        // "10.000" is ten thousand written the Japanese-English way, not 10: leave it to the note
        if rx::is(s, r"^[+\-]?\d{1,3}\.\d{3}$") {
            return None;
        }
        crate::fmt::parse_f64(s)
    };
    if let Some(m) = rx::m(&p, r"^Damage to ([A-Za-z]+) enemies\s+([+\-]\d+(?:\.\d+)?)\s*(%?)$") {
        return Some(Part { label: "Damage".into(), value: num(&m[2])?, pct: &m[3] == "%", situational: Some(format!("vs {} enemies", m[1].to_uppercase())) });
    }
    if let Some(m) = rx::m(&p, r"^(?<label>[A-Za-z][A-Za-z0-9'\.\-\s/&\(\)\[\]]*?)\s*:?\s*(?<val>[+\-]\d+(?:\.\d+)?)\s*(?<pct>%?)$") {
        return Some(Part { label: canon_label(&m["label"]), value: num(&m["val"])?, pct: &m["pct"] == "%", situational: None });
    }
    if let Some(m) = rx::m(&p, r"^(?<val>[+\-]\d+(?:\.\d+)?)\s*(?<pct>%?)\s+(?<label>[A-Za-z][A-Za-z0-9'\-\s]*)$") {
        return Some(Part { label: canon_label(&m["label"]), value: num(&m["val"])?, pct: &m["pct"] == "%", situational: None });
    }
    None
}

/// The effects and notes the crystal's text says. `source` is "tarot" or "soul"; `item` is what the page names as the source.
pub fn effects(text: &str, source: &str, item: &str, rules: &Rules) -> Vec<Effect> {
    let mut out = Vec::new();
    let mut block = Ctx::default();
    for line in logical_lines(text) {
        if line.is_empty() {
            block = Ctx::default();
            continue;
        }
        if line.starts_with('※') || rx::is(&line, r"^-\s*[A-Za-z ]+-$") {
            continue;
        }
        let mut c = block.clone();
        let rest = peel(&line, rules, &mut c);
        let heading = rest.trim_end().ends_with(':');
        let rest = rest.trim_matches(|ch: char| ch.is_whitespace() || ch == '.' || ch == ',' || ch == ':').to_string();
        if rest.is_empty() {
            block = c; // a condition or "who" on a line of its own: it holds for the lines after it
            continue;
        }
        // A condition the reader does not know ("Against a target enemy, if their HP is lower than 25%:", "If Bulwark is active,"):
        // what it holds for must not be counted as unconditional, so the clause stays a note, and a heading makes the lines under
        // it situational.
        if rx::is(&rest, r"^(If|When|Whenever|Against|While|During|After|With)\b") || rx::is(&rest, r"\b(if|when|while|unless|during)\b") {
            out.push(note_effect(&rest, &c, source, item));
            if heading {
                c.situational = true;
                c.note(rest.clone());
                block = c;
            }
            continue;
        }
        let mut read_any = false;
        let mut unread: Vec<String> = Vec::new();
        for part in split_parts(&rest) {
            // a part may name its own side: "Partner's Spin Damage +5%"
            let mut who = c.who;
            let mut text = part.clone();
            if let Some(m) = rx::m(&part, r"^(Player/Demon's|Player's|Demon Partner's|Demon's|Partner's|Demon)\s+(.*)$") {
                who = Some(match m[1].to_lowercase().as_str() {
                    "player/demon's" => Who::Both,
                    "player's" => Who::Own,
                    _ => Who::Partner,
                });
                text = m[2].to_string();
            }
            // "Physical Defense +3 to all Party members": the character and the partner
            if let Some(m) = rx::m(&text, r"^(.*?)\s+to all [Pp]arty [Mm]embers$") {
                who = Some(Who::Both);
                text = m[1].to_string();
            }
            // a label that is a sentence ("... is increased by") or names the party is not a stat the reader can place
            let Some(p) = read_part(&text)
                .filter(|p| !rx::is(&p.label, r"^(Party|all party)") && !rx::is(&p.label, r"\b(is|are|will|can|increased|decreased|further|no longer|learnt|active)\b"))
            else {
                unread.push(part);
                continue;
            };
            read_any = true;
            let cond = if c.inactive {
                "inactive"
            } else if c.situational || p.situational.is_some() {
                "situational"
            } else if c.digitalized {
                "digitalized"
            } else {
                "always"
            };
            let mut notes = c.notes.clone();
            if let Some(n) = p.situational {
                notes.push(n);
            }
            notes.push(FROM_TEXT.to_string());
            let sides: &[&str] = match who.unwrap_or(Who::Own) {
                Who::Own => &["self"],
                Who::Partner => &["partner"],
                Who::Both => &["self", "partner"],
            };
            for target in sides {
                out.push(Effect {
                    label: p.label.clone(),
                    value: Some(p.value),
                    pct: p.pct,
                    cond: cond.into(),
                    cond_note: notes.join("; "),
                    target: (*target).into(),
                    source: source.into(),
                    item: item.into(),
                    ..Default::default()
                });
            }
        }
        // what could not be read stays as the game's words: the whole clause when nothing in it was understood
        let notes: Vec<String> = if read_any { unread } else { vec![rest.clone()] };
        for n in notes {
            out.push(note_effect(&n, &c, source, item));
        }
    }
    out
}

/// What could not be read, kept as the game's words: no value, never counted.
fn note_effect(text: &str, c: &Ctx, source: &str, item: &str) -> Effect {
    Effect {
        label: text.to_string(),
        value: None,
        cond: "unparsed".into(),
        cond_note: FROM_TEXT.into(),
        target: match c.who.unwrap_or(Who::Own) {
            Who::Partner => "partner",
            _ => "self",
        }
        .into(),
        source: source.into(),
        item: item.into(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, alignment: &str, ranks: &[(&str, i32)]) -> Vec<Effect> {
        let table: Vec<(String, i32)> = ranks.iter().map(|(n, r)| (n.to_string(), *r)).collect();
        let rank_of = move |name: &str| -> Option<i32> {
            if table.is_empty() {
                return None;
            }
            Some(table.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map_or(0, |(_, r)| *r))
        };
        effects(text, "soul", "Weapon", &Rules { alignment, rank_of: &rank_of })
    }

    fn row(e: &Effect) -> (String, Option<f64>, bool, String, String) {
        (e.label.clone(), e.value, e.pct, e.cond.clone(), e.target.clone())
    }

    /// Ogami's Crystal (soul 3105), as the enchant table has it.
    const OGAMI_SOUL: &str = "- Weapon - / While Digitalized, Player/Demon's Final Critical rate correction +5%, / Limit Break chance +5%, Limit Break power +5% /  / Player's Damage to Neutral enemies +20%.";

    #[test]
    fn ogamis_soul_is_digitalized_for_both_and_the_damage_is_situational() {
        let got: Vec<_> = run(OGAMI_SOUL, "NEUTRAL", &[]).iter().map(row).collect();
        let d = |l: &str, t: &str| (l.to_string(), Some(5.0), true, "digitalized".to_string(), t.to_string());
        assert_eq!(
            got,
            vec![
                d("Final Critical rate correction", "self"),
                d("Final Critical rate correction", "partner"),
                d("Limit Break chance", "self"),
                d("Limit Break chance", "partner"),
                d("Limit Break power", "self"),
                d("Limit Break power", "partner"),
                ("Damage".to_string(), Some(20.0), true, "situational".to_string(), "self".to_string()),
            ]
        );
        let damage = &run(OGAMI_SOUL, "NEUTRAL", &[])[6];
        assert_eq!(damage.cond_note, "vs NEUTRAL enemies; from the game's text");
    }

    #[test]
    fn a_tarot_with_a_class_condition_is_decided_when_the_ranks_are_known() {
        let tarot = "If Player has Pursuit Class 9:  / Digitalize Duration +2% / Limit Break Cap +25 / MAX HP +25";
        // ranks not known: situational, the requirement in the note
        let unknown = run(tarot, "NEUTRAL", &[]);
        assert_eq!(unknown.len(), 3);
        assert!(unknown.iter().all(|e| e.cond == "situational" && e.cond_note == "needs Class 9 Pursuit; from the game's text"), "{unknown:#?}");
        // known and short of it
        let short = run(tarot, "NEUTRAL", &[("Pursuit", 84)]);
        assert!(short.iter().all(|e| e.cond == "inactive" && e.cond_note.contains("you have 8-4")), "{short:#?}");
        // known and met: counted, the label is the game's
        let met = run(tarot, "NEUTRAL", &[("Pursuit", 90)]);
        assert_eq!(met.iter().map(row).collect::<Vec<_>>()[2], ("MAX HP".to_string(), Some(25.0), false, "always".to_string(), "self".to_string()));
        assert_eq!((met[0].label.as_str(), met[1].label.as_str()), ("Digitalize Duration", "Limit Break Cap"));
    }

    #[test]
    fn a_clause_that_cannot_be_read_does_not_drop_the_others() {
        let text = "- Bottom - / STR +7, Blunt-based Skill effects +10%, / For all Death-based skills,  /  Death Boost +10% and  /  Limit Break Cap + (STR x 1) / ";
        let got = run(text, "NEUTRAL", &[]);
        let values: Vec<_> = got.iter().filter(|e| e.value.is_some()).map(|e| (e.label.as_str(), e.value.unwrap())).collect();
        assert_eq!(values, vec![("STR", 7.0), ("Blunt-based Skill effects", 10.0), ("Death Boost", 10.0)]);
        let notes: Vec<_> = got.iter().filter(|e| e.value.is_none()).collect();
        assert_eq!(notes.len(), 2, "{got:#?}");
        assert!(notes.iter().any(|n| n.label.contains("Limit Break Cap")));
        assert!(notes.iter().all(|n| n.cond == "unparsed" && n.cond_note == FROM_TEXT));
    }

    #[test]
    fn player_demon_and_partner_wording() {
        let g = |t: &str| run(t, "NEUTRAL", &[]).iter().map(|e| e.target.clone()).collect::<Vec<_>>();
        assert_eq!(g("Player's Critical +5"), vec!["self"]);
        assert_eq!(g("Demon Partner's STR +6"), vec!["partner"]);
        assert_eq!(g("Partner's Move Speed +15%"), vec!["partner"]);
        assert_eq!(g("Demon Support damage + 2%"), vec!["partner"]);
        assert_eq!(g("Player/Demon's Critical +5"), vec!["self", "partner"]);
        // a heading puts the lines under it on both sides, up to the blank line
        assert_eq!(g("For Player and Demon Partner / Almighty Damage +6% /  / STR +1"), vec!["self", "partner", "self"]);
    }

    #[test]
    fn alignment_and_enemy_conditions() {
        let text = "If Player is CHAOS, Magic and Close Range damage suffered -50%. / Against LAW Enemies, Close Range, Long Range / and Magic damage done +30%.";
        let chaos = run(text, "CHAOS", &[]);
        assert_eq!(
            chaos.iter().map(|e| (e.label.as_str(), e.value.unwrap(), e.cond.as_str())).collect::<Vec<_>>(),
            vec![
                ("Magic damage suffered", -50.0, "always"),
                ("Close-Range damage suffered", -50.0, "always"),
                ("Close-Range Damage Dealt", 30.0, "situational"),
                ("Long-Range Damage Dealt", 30.0, "situational"),
                ("Magic Damage Dealt", 30.0, "situational"),
            ]
        );
        let law = run(text, "LAW", &[]);
        assert!(law[..2].iter().all(|e| e.cond == "inactive" && e.cond_note.starts_with("if you are CHAOS")));
    }

    #[test]
    fn thousands_written_with_a_dot_are_not_read_as_small_numbers() {
        let got = run("When using Spin skills, +10.000 Smirk Damage Cap.", "NEUTRAL", &[]);
        assert_eq!(got.len(), 1);
        assert!(got[0].value.is_none());
    }
}
