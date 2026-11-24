//! Port of `src/Calc/TooltipParser.cs`.
//!
//! Turns the client's equipment tooltip texts into effects. Requirements are decided from the context:
//! alignment (for "If Player is LAW or NEUTRAL:") and expertise classes (for "If player has Class 4 Rapid or more, ...").

use super::effect::{rx, Effect};
use crate::fmt;
use crate::text::{eq_ic, ic_key};
use std::collections::HashMap;

/// full-width space used for nested lines
const FW: char = '\u{3000}';
const TRIM_CHARS: &[char] = &[' ', FW, '\t'];
const TRIM_BODY: &[char] = &[' ', FW, '\t', '.'];

/// One item's tooltip, as shown in game. `sex`, `tarot` and `soul` are None when the tooltip has none.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TooltipInfo {
    pub name: String,
    pub base_name: String,
    pub sex: Option<String>,
    pub tarot: Option<String>,
    pub soul: Option<String>,
    pub lines: Vec<String>,
}

/// A "Set Bonus" block inside a tooltip (only used when the set table isn't available).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TipSet {
    pub pieces: Vec<String>,
    pub effects: Vec<Effect>,
}

/// The C# primary-constructor class: `TooltipParser(alignment, expertiseClasses, isCrystal)`.
///
/// `is_crystal`: when set, fusion-set lines ("With Orthrus:", "If all items obtained by using Soul Stone Fusion on ... are
/// equipped:") get Cond "fusion-set": not counted from text, because the fusion-set table decides them from the worn
/// enchant ids.
pub struct TooltipParser {
    /// The alignment in upper case.
    pub alignment: String,
    /// Expertise class by expertise name. The C# dictionary compares names ignoring case (Store.ExpertiseClasses), so the
    /// keys here are [`ic_key`] of the names; look up with `expertise.get(&ic_key(name))`.
    pub expertise: HashMap<String, i32>,
    is_crystal: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
}

fn like(s: Option<&str>, needle: &str) -> bool {
    // s != null && s.Contains(needle, OrdinalIgnoreCase)
    s.is_some_and(|s| ic_key(s).contains(&ic_key(needle)))
}

fn trim_ws(s: &str) -> &str {
    s.trim()
}

impl TooltipParser {
    pub fn new(
        alignment: &str,
        expertise_classes: impl IntoIterator<Item = (String, i32)>,
        is_crystal: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
    ) -> TooltipParser {
        let mut expertise = HashMap::new();
        for (k, v) in expertise_classes {
            expertise.insert(ic_key(&k), v);
        }
        TooltipParser { alignment: alignment.to_uppercase(), expertise, is_crystal }
    }

    pub fn info(text: &str) -> TooltipInfo {
        let lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        let mut info = TooltipInfo { name: trim_ws(&lines[0]).to_string(), lines: lines.clone(), ..Default::default() };
        for i in 0..lines.len() {
            let l = &lines[i];
            if let Some(m) = rx::m(l, r"^\s*Sex:\s*(Male|Female)") {
                info.sex = Some(m[1].to_string());
            }
            if let Some(m) = rx::m(l, r"^\s*Tarot Card:\s*(.+?)\s*$") {
                info.tarot = Some(m[1].to_string());
            }
            if trim_ws(l) == "[Tarot Fusion]" && info.tarot.is_none() && i + 1 < lines.len() {
                info.tarot = Some(trim_ws(&lines[i + 1]).to_string());
            }
            if trim_ws(l) == "[Soul Fusion]" && i + 1 < lines.len() {
                info.soul = Some(trim_ws(&lines[i + 1]).to_string());
            }
        }
        info.base_name = trim_ws(&rx::rep(&info.name, r"\s*[♂♀]\s*$", "")).to_string();
        info
    }

    /// One piece of effect text into parts, each "label value".
    /// "Close-range, Long-range, Spell damage suffered -10%" becomes three parts.
    pub fn split_parts(body: &str) -> Vec<String> {
        let mut parts: Vec<String> = rx::split(body, r",\s*").iter().map(|p| trim_ws(p).to_string()).filter(|p| !p.is_empty()).collect();
        if parts.len() <= 1 {
            return parts;
        }
        let has_val = |p: &str| rx::is(p, r"[+\-]\d");
        let last = parts[parts.len() - 1].clone();
        let valueless: Vec<&String> = parts.iter().filter(|p| !has_val(p)).collect();
        if !valueless.is_empty() && has_val(&last) {
            let short_ones = valueless.iter().filter(|p| rx::split(p, r"\s+").len() <= 3 && !rx::is(p, r"[%\d]")).count();
            if short_ones == valueless.len() {
                if let Some(m) = rx::m(&last, r"^(\S+)\s*(.*?)\s*([+\-]\d+(?:\.\d+)?%?)$") {
                    let suffix = m[2].to_string();
                    let val = m[3].to_string();
                    parts = parts.iter().map(|p| if has_val(p) { p.clone() } else { trim_ws(&rx::rep(&format!("{p} {suffix} {val}"), r"\s+", " ")).to_string() }).collect();
                }
            }
        }
        parts
    }

    /// Is the crystal/item named in "With X:" worn (as an item name, soul or tarot)?
    pub fn worn_with(who: &str, worn: &[TooltipInfo]) -> bool {
        let n = trim_ws(&rx::rep(&rx::rep(who, "'s Crystal$", ""), "'$", "")).to_string();
        worn.iter().any(|w| like(Some(&w.name), &n) || like(w.soul.as_deref(), &n) || like(w.tarot.as_deref(), &n))
    }

    /// Is a set piece like "Spiral Code License♂" worn? Names can carry a tarot prefix and soul suffix.
    pub fn piece_worn(piece: &str, worn: &[TooltipInfo]) -> bool {
        let g = if piece.ends_with('♂') {
            Some("Male")
        } else if piece.ends_with('♀') {
            Some("Female")
        } else {
            None
        };
        let n = trim_ws(&rx::rep(piece, r"[♂♀]\s*$", "")).to_string();
        worn.iter().any(|w| {
            (eq_ic(&w.base_name, &n) || like(Some(&w.base_name), &n)) && (g.is_none() || w.sex.is_none() || eq_ic(w.sex.as_deref().unwrap_or(""), g.unwrap_or("")))
        })
    }

    pub fn effects(&self, info: &TooltipInfo, worn: &[TooltipInfo]) -> (Vec<Effect>, Vec<TipSet>) {
        let mut effects: Vec<Effect> = Vec::new();
        let mut sets: Vec<TipSet> = Vec::new();
        let mut section = "basic".to_string();
        let mut cond = "always".to_string();
        let mut note = String::new();
        let mut target = "self".to_string();
        let mut base_cond = "always".to_string();
        let mut cur_set: Option<usize> = None;
        let mut in_pieces = false;
        let mut skip_first = false;

        // C# local function Add: into the current set while in a set section, else into the item's effects
        fn add(effects: &mut Vec<Effect>, sets: &mut [TipSet], section: &str, cur_set: Option<usize>, e: Effect) {
            match cur_set {
                Some(i) if section == "set" => sets[i].effects.push(e),
                _ => effects.push(e),
            }
        }

        for l in info.lines.iter().skip(1) {
            let trim = l.trim_matches(TRIM_CHARS);
            if trim.is_empty() {
                // A blank line ends a condition block in fusion/mod sections.
                if matches!(section.as_str(), "soul" | "tarot" | "mod" | "basic") {
                    cond = "always".into();
                    note.clear();
                    target = "self".into();
                    base_cond = "always".into();
                }
                continue;
            }
            if rx::is(trim, r"^(Type|Sex|Level|Durability|Affinity):") {
                continue; // item header lines
            }

            macro_rules! reset {
                () => {
                    cond = "always".into();
                    note.clear();
                    target = "self".into();
                };
            }
            if rx::is(trim, r"^\[Characteristics\]$") {
                section = "characteristic".into();
                reset!();
                cur_set = None;
                continue;
            }
            if rx::is(trim, r"^\[Modification Slot\]$") {
                section = "mod".into();
                reset!();
                cur_set = None;
                continue;
            }
            if rx::is(trim, r"^\[Tarot Fusion\]$") {
                section = "tarot".into();
                reset!();
                cur_set = None;
                skip_first = true;
                continue;
            }
            if rx::is(trim, r"^\[Soul Fusion\]$") {
                section = "soul".into();
                reset!();
                cur_set = None;
                skip_first = true;
                continue;
            }
            if rx::is(trim, r"^\[Appearance Bonus\]$") {
                section = "appearance".into();
                reset!();
                cur_set = None;
                continue;
            }
            if rx::is(trim, r"^\[Expiration Date\]") {
                section = "ignore".into();
                continue;
            }
            if rx::is(trim, r"^(Tarot|Soul) Fusion: Possible$") {
                continue;
            }
            if rx::is(trim, r"^Set Bonus \d+$") {
                sets.push(TipSet::default());
                cur_set = Some(sets.len() - 1);
                section = "set".into();
                in_pieces = false;
                continue;
            }
            if rx::is(trim, r"^\[Set Pieces\]$") {
                in_pieces = true;
                continue;
            }
            if rx::is(trim, r"^\[Activation Effects\]$") {
                in_pieces = false;
                reset!();
                continue;
            }
            if section == "ignore" {
                continue;
            }
            if section == "set" && in_pieces {
                if let Some(pm) = rx::m(trim, r"^\*\s*(.+)$") {
                    if let Some(i) = cur_set {
                        sets[i].pieces.push(trim_ws(&pm[1]).to_string());
                    }
                }
                continue;
            }
            if skip_first {
                skip_first = false; // crystal / tarot card name line
                continue;
            }
            if rx::is(trim, r"^-\s*\w+\s*-$") {
                continue; // "- Weapon -"
            }
            if rx::is(trim, r"^Tarot Card:") {
                continue;
            }

            if trim.starts_with('*') {
                reset!();
                base_cond = "always".into();
            }
            let mut body = rx::rep(trim, r"^\*\s*", "");
            body = rx::split(&body, "※").swap_remove(0);
            body = body.trim_matches(TRIM_BODY).to_string();
            body = rx::rep(&body, r"([+\-])\s+(\d)", "${1}${2}");

            // Mod slots: "Slot 4: Rapid Power 2"
            if section == "mod" {
                if let Some(mm) = rx::m(&body, r"^Slot\s*\d+\s*:\s*(.+?)\s+(-?\d+(?:\.\d+)?)\s*(%?)$") {
                    let lab = mm[1].to_string();
                    let pct = &mm[3] == "%" || rx::is(&lab, "power|damage|rate|chance|boost");
                    effects.push(Effect {
                        label: lab,
                        // C# double.Parse throws on a digit it cannot read; such a number is not in any tooltip
                        value: Some(fmt::parse_f64(&mm[2]).unwrap_or(0.0)),
                        pct,
                        source: "mod".into(),
                        item: info.base_name.clone(),
                        ..Default::default()
                    });
                }
                continue;
            }

            // Peel condition prefixes (may be several, and may share the line with effects)
            let mut changed = true;
            while changed && !body.is_empty() {
                changed = false;
                body = rx::rep(&body, r"^(Player/Demon's|Player's)\s+", "");
                if let Some(m) = rx::m(&body, r"^While Digitalized\s*[:,]?\s*(.*)$") {
                    base_cond = "digitalized".into();
                    if cond == "always" {
                        cond = "digitalized".into();
                    }
                    body = m[1].to_string();
                    changed = true;
                    continue;
                }
                // Each requirement line starts fresh from the block's base condition (always / digitalized).
                if let Some(m) = rx::m(&body, r"^If [Pp]layer is ([A-Za-z ,]+?)\s*[:,]\s*(.*)$") {
                    let want: Vec<String> =
                        rx::split(&m[1], r"\s*(?:,|\bor\b)\s*").iter().map(|w| trim_ws(w).to_uppercase()).filter(|w| !w.is_empty()).collect();
                    let rest = m[2].to_string();
                    note = format!("if you are {}", want.join(" or "));
                    cond = if want.iter().any(|w| eq_ic(w, &self.alignment)) { base_cond.clone() } else { "inactive".into() };
                    body = rest;
                    changed = true;
                    continue;
                }
                if let Some(m) = rx::m(&body, r"^If [Pp]layer has Class (\d+) (.+?) or more\s*,\s*(.*)$") {
                    // C# int.Parse throws on an overflowing number; saturate instead
                    let need = m[1].parse::<i32>().unwrap_or(i32::MAX);
                    let exp = trim_ws(&m[2]).to_string();
                    let rest = m[3].to_string();
                    let have = self.expertise.get(&ic_key(&exp)).copied().unwrap_or(0);
                    note = format!("needs Class {need} {exp} (you have {have})");
                    cond = if have >= need { base_cond.clone() } else { "inactive".into() };
                    body = rest;
                    changed = true;
                    continue;
                }
                if let Some(m) = rx::m(&body, r"^With (.+?)\s*:\s*(.*)$") {
                    let who = m[1].to_string();
                    let rest = m[2].to_string();
                    note = format!("with {who}");
                    cond = if self.is_crystal.as_ref().is_some_and(|f| f(&who)) {
                        "fusion-set".into()
                    } else if Self::worn_with(&who, worn) {
                        base_cond.clone()
                    } else {
                        "inactive".into()
                    };
                    body = rest;
                    changed = true;
                    continue;
                }
                if self.is_crystal.is_some() && rx::is(&body, r"^(If all items obtained by using Soul.*)$") {
                    // Spans several lines ("... Soul / Stone Fusion on the Soul Stones of / the Great Winter Triangle are equipped:")
                    cond = "fusion-set".into();
                    note = "soul stone set".into();
                    body = String::new();
                    changed = true;
                    continue;
                }
                if let Some(m) = rx::m(&body, r"^(When Partner's Friendship[^,]*|Against [A-Za-z]+ enemies|If all items .*)\s*[:,]?\s*(.*)$") {
                    cond = "situational".into();
                    note = m[1].to_string();
                    let rest = m[2].to_string();
                    body = rest;
                    changed = true;
                    continue;
                }
            }
            if body.is_empty() {
                continue;
            }
            if rx::is(&body, r"^Partner's$") {
                target = "partner".into();
                continue;
            }

            let src = if section == "basic" { "item".to_string() } else { section.clone() };
            let mut parsed_any = false;
            for p0 in Self::split_parts(&body) {
                let mut part = rx::rep(&p0, r"^(Player/Demon's|Player's)\s+", "");
                let mut tgt = target.clone();
                let mut pcond = cond.clone();
                let mut pnote = note.clone();
                if let Some(dm) = rx::m(&part, r"^(?:Player's )?Damage to ([A-Za-z]+) enemies\s+(.*)$") {
                    pcond = "situational".into();
                    pnote = format!("vs {} enemies", &dm[1]);
                    part = format!("Damage {}", &dm[2]);
                }
                if let Some(em) = rx::m(&part, r"^(?<label>[A-Za-z][A-Za-z0-9'\.\-\s/&\(\)]*?)\s*:?\s*(?<val>[+\-]\d+(\.\d+)?)\s*(?<pct>%?)\s*$") {
                    let mut label = trim_ws(&em["label"]).to_string();
                    let val = fmt::parse_f64(&em["val"]).unwrap_or(0.0);
                    if let Some(pm2) = rx::m(&label, r"^Partner's\s+(.+)$") {
                        tgt = "partner".into();
                        label = pm2[1].to_string();
                    }
                    let e = Effect {
                        label,
                        value: Some(val),
                        pct: &em["pct"] == "%",
                        cond: pcond,
                        cond_note: pnote,
                        target: tgt,
                        source: src.clone(),
                        item: info.base_name.clone(),
                        ..Default::default()
                    };
                    add(&mut effects, &mut sets, &section, cur_set, e);
                    parsed_any = true;
                }
            }
            if !parsed_any && rx::is(&body, r"\b(nulls?|reflect|absorb|resist|drain|cancels)\b") {
                let e = Effect {
                    label: body.clone(),
                    value: None,
                    cond: cond.clone(),
                    cond_note: note.clone(),
                    target: target.clone(),
                    source: src.clone(),
                    item: info.base_name.clone(),
                    ..Default::default()
                };
                add(&mut effects, &mut sets, &section, cur_set, e);
            } else if !parsed_any && rx::is(&body, r"[+\-]\s?\d|\d\s?%|\*\s?0\.\d") {
                // Looks like a bonus but wasn't understood: keep it visible so nothing is silently dropped.
                let e = Effect {
                    label: body.clone(),
                    value: None,
                    cond: "unparsed".into(),
                    cond_note: note.clone(),
                    target: target.clone(),
                    source: src.clone(),
                    item: info.base_name.clone(),
                    ..Default::default()
                };
                add(&mut effects, &mut sets, &section, cur_set, e);
            }
        }
        (effects, sets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> TooltipParser {
        TooltipParser::new("neutral", [("Rapid".to_string(), 4)], None)
    }

    #[test]
    fn info_reads_header_lines() {
        let i = TooltipParser::info("Spiral Code License♂\r\nSex: male\r\n[Tarot Fusion]\r\n The Fool \r\n[Soul Fusion]\r\nSoul X\r\n");
        assert_eq!(i.name, "Spiral Code License♂");
        assert_eq!(i.base_name, "Spiral Code License");
        assert_eq!(i.sex.as_deref(), Some("male"));
        assert_eq!(i.tarot.as_deref(), Some("The Fool"));
        assert_eq!(i.soul.as_deref(), Some("Soul X"));
    }

    #[test]
    fn split_parts_shares_the_value() {
        assert_eq!(
            TooltipParser::split_parts("Close-range, Long-range, Spell damage suffered -10%"),
            vec!["Close-range damage suffered -10%", "Long-range damage suffered -10%", "Spell damage suffered -10%"]
        );
    }

    #[test]
    fn conditions_and_values() {
        let info = TooltipParser::info("Test Ring\n* If Player is LAW or NEUTRAL: STR +5, Critical +2%\n* If player has Class 5 Rapid or more, Rapid Power +10%\n* While Digitalized: Max HP + 100");
        let (fx, sets) = parser().effects(&info, std::slice::from_ref(&info));
        assert!(sets.is_empty());
        let got: Vec<(String, Option<f64>, bool, String, String)> = fx.iter().map(|e| (e.label.clone(), e.value, e.pct, e.cond.clone(), e.cond_note.clone())).collect();
        assert_eq!(got[0], ("STR".into(), Some(5.0), false, "always".into(), "if you are LAW or NEUTRAL".into()));
        assert_eq!(got[1].0, "Critical");
        assert_eq!(got[2], ("Rapid Power".into(), Some(10.0), true, "inactive".into(), "needs Class 5 Rapid (you have 4)".into()));
        assert_eq!(got[3], ("Max HP".into(), Some(100.0), false, "digitalized".into(), String::new()));
    }
}
