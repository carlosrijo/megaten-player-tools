//! Does a skill's tokusei still say what the game's own text for the skill says?
//!
//! A skill's effects come from two places: its correct table (the client's SkillData, current) and the tokusei it lists
//! (comp_hack's / the website's tokusei files, which are the public server's). NewMoon reworks some skills and leaves the
//! old tokusei id in the skill's list: the skill's description and correct table are new, the tokusei is the old one.
//! Lord of Victory (5549) is the example: the game says "Close-Range Damage +15%, FCC +5%, healing received -25%", the
//! table has +15% and -25%, and the listed tokusei 520061 is the old "+25% Close-Range Damage, Critical +100 with a
//! melee weapon". Nothing in the client or the server files corrects it, so the text the player reads in game wins.
//!
//! A tokusei effect is stale when the description lists its stat with another number, or lists effects with numbers and
//! never mentions the stat. This only judges what it can read: a description without "<stat> +N" lines (prose, formulas)
//! says nothing about numbers, and stats this module has no wording for are left alone.

use std::sync::LazyLock;

use regex::Regex;

use crate::data::game_data::TokuseiDef;

/// How the game's text words each stat it can be told apart by (lowercase, after [`normalize`]); the longest match
/// wins, so "close-range damage taken" is not "close-range damage" and "final crit" is not "crit".
const FAMILIES: &[(&str, &[&str])] = &[
    ("RATE_CLSR", &["close-range damage dealt", "close-range damage", "melee damage"]),
    ("RATE_LNGR", &["long-range damage dealt", "long-range damage"]),
    ("RATE_SPELL", &["spell damage dealt", "spell damage"]),
    ("RATE_SUPPORT", &["support magic damage", "support damage"]),
    ("RATE_CLSR_TAKEN", &["close-range damage taken", "close-range damage suffered", "close-range damage received"]),
    ("RATE_LNGR_TAKEN", &["long-range damage taken", "long-range damage suffered", "long-range damage received"]),
    ("RATE_SPELL_TAKEN", &["spell damage taken", "spell damage suffered", "spell damage received"]),
    ("RATE_HEAL_TAKEN", &["curative received", "healing received", "healing taken", "recovery received"]),
    ("CLSR", &["close-range"]),
    ("LNGR", &["long-range"]),
    ("CRITICAL", &["critical", "crit"]),
    ("CRIT_DEF", &["critical defense", "crit defense", "crit def"]),
    ("FINAL_CRIT_CHANCE", &["final critical rate correction", "final crit correction", "final critical correction", "final critical", "final crit", "fcc"]),
    ("LB_CHANCE", &["limit break chance", "lb chance"]),
    ("LB_DAMAGE", &["limit break power", "limit break damage", "lb power", "lb damage"]),
    ("LIMIT_BREAK_MAX", &["limit break damage upper limit", "limit break upper limit", "limit break damage cap", "lb upper limit increase", "lb upper limit", "limit break cap", "lb cap"]),
    ("PURSUIT_RATE", &["pursuit chance", "pursuit rate"]),
    ("PURSUIT_POWER", &["pursuit power", "pursuit damage"]),
    ("TECH_ATTACK_RATE", &["technical attack chance", "tech attack chance", "ta chance"]),
    ("TECH_ATTACK_POWER", &["technical attack power", "tech attack power", "ta power"]),
    ("CHANT_TIME", &["incantation time", "incantation", "cast time"]),
    ("COOLDOWN_TIME", &["cooldown", "cool time"]),
    ("MP_COST_ADJUST", &["mp costs", "mp cost"]),
    ("HP_COST_ADJUST", &["hp costs", "hp cost"]),
    // all damage received, whichever kind (no tokusei stat of its own): named so that a line about it counts as understood
    ("DAMAGE_TAKEN_ALL", &["damage received", "damage taken", "damage suffered"]),
    ("HP_MAX", &["max hp"]),
    ("MP_MAX", &["max mp"]),
    ("PDEF", &["physical defense", "p.def"]),
    ("MDEF", &["magic defense", "m.def"]),
    // the power of one kind of skill (EFFECT_POWER with one ACTION_TYPE condition: 0 Attack, 1 Spin, 2 Rush ...)
    ("EFFECT_POWER:0", &["attack skill boost", "attack boost", "attack power"]),
    ("EFFECT_POWER:1", &["spin skill boost", "spin boost", "spin power"]),
    ("EFFECT_POWER:2", &["rush skill boost", "rush boost", "rush power"]),
    ("EFFECT_POWER:3", &["shot skill boost", "shot boost", "shot power"]),
    ("EFFECT_POWER:4", &["rapid skill boost", "rapid boost", "rapid power"]),
    ("EFFECT_POWER:5", &["guard skill boost", "guard boost", "guard power"]),
    ("EFFECT_POWER:6", &["counter skill boost", "counter boost", "counter power"]),
    ("EFFECT_POWER:7", &["dodge skill boost", "dodge boost", "dodge power"]),
];

/// A line that is only the name of a skill kind ("Spin +20%") is that kind's power.
const ACTION_NAMES: [&str; 8] = ["attack", "spin", "rush", "shot", "rapid", "guard", "counter", "dodge"];

/// Lowercase, with the client's spellings of the same stat made one ("CLS-R", "Close range", "Melee" -> "close-range").
fn normalize(s: &str) -> String {
    static RULES: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
        [
            (r"\bcls[- ]?r\b|\bclose[- ]range\b", "close-range"),
            (r"\blng[- ]?r\b|\blong[- ]range\b", "long-range"),
            (r"\bmagic damage\b", "spell damage"),
            (r"[\u{3000}\s]+", " "),
        ]
        .into_iter()
        .map(|(p, r)| (Regex::new(p).expect("fixed pattern"), r))
        .collect()
    });
    // "CLS-R, LNG-R and Magic Damage suffered -50%" says it for each of them
    static LIST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?P<list>(?:(?:close-range|long-range|spell|support)(?:\s*,\s*(?:and\s+)?|\s+and\s+))+)(?P<last>close-range|long-range|spell|support) damage(?P<tail> dealt| taken| suffered| received)?")
            .expect("fixed pattern")
    });
    let mut t = s.to_lowercase();
    for (re, to) in RULES.iter() {
        t = re.replace_all(&t, *to).into_owned();
    }
    LIST.replace_all(&t, |c: &regex::Captures| {
        let tail = c.name("tail").map_or("", |m| m.as_str());
        let mut stats: Vec<&str> = c["list"].split(|ch: char| ch == ',' || ch.is_whitespace()).filter(|w| !w.is_empty() && *w != "and").collect();
        stats.push(&c["last"]);
        stats.iter().map(|st| format!("{st} damage{tail}")).collect::<Vec<_>>().join(", ")
    })
    .into_owned()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-'
}

/// The first place `phrase` stands in `text` as whole words.
fn find_words(text: &str, phrase: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = text[from..].find(phrase) {
        let at = from + i;
        let before = text[..at].chars().next_back();
        let after = text[at + phrase.len()..].chars().next();
        if !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char) {
            return Some(at);
        }
        from = at + phrase.len();
    }
    None
}

/// Every stat a piece of text names, in the order it names them ("Melee Damage and Final Crit Correction" is
/// RATE_CLSR then FINAL_CRIT_CHANCE). A stretch of text is claimed by its longest phrase only.
pub fn stats_named(text: &str) -> Vec<&'static str> {
    scan(text).0
}

/// [`stats_named`], and the words of the text no stat phrase claimed ("magic" in "Magic incantation time").
fn scan(text: &str) -> (Vec<&'static str>, String) {
    let mut t = normalize(text);
    let mut phrases: Vec<(&str, &'static str)> = FAMILIES.iter().flat_map(|(id, ps)| ps.iter().map(move |p| (*p, *id))).collect();
    phrases.sort_by_key(|p| std::cmp::Reverse(p.0.len()));
    let mut found: Vec<(usize, &'static str)> = Vec::new();
    for (phrase, id) in phrases {
        while let Some(at) = find_words(&t, phrase) {
            found.push((at, id));
            t.replace_range(at..at + phrase.len(), &" ".repeat(phrase.len()));
        }
    }
    found.sort_by_key(|f| f.0);
    let mut ids: Vec<&'static str> = Vec::new();
    for (_, id) in found {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    (ids, t)
}

/// One "<stat> +N" line of a description.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    /// the line's words without the number
    pub label: String,
    /// the number with its sign
    pub value: f64,
    pub pct: bool,
    /// "<stat> to <family> enemies: +N", or a line under "When using any Spin Skills:": it only applies in some case
    pub conditional: bool,
    /// the case the heading above the line names ("When using any Spin Skills"), when there is one
    pub when: Option<String>,
    /// the line starts with a bullet (". ", "・"): the NewMoon-style list of effects, not a running text or a "Name: value" trait line
    pub bulleted: bool,
    /// the stat the label names, when it is one this module knows
    pub stat: Option<&'static str>,
    /// what else the label says about it ("magic" for "Magic incantation time"), or what follows the number ("for Rush
    /// skills"): it is not the stat for every skill
    pub extra: String,
    /// the words after the number ("for Rush skills"), with their leading space
    pub suffix: String,
}

/// The lines of a description that are "<stat> +N" / "-N" (with an optional %): its listed effects.
pub fn effect_lines(text: &str) -> Vec<TextLine> {
    scan_lines(text).0
}

/// [`effect_lines`], and whether the description also has a bulleted line with a number in it that is not one of them
/// (a number this module cannot read: "HP Cost -30% for Rush skills", "LB Upper Limit Increase +500"). Then the list
/// is not fully understood, and a stat missing from it proves nothing.
fn scan_lines(text: &str) -> (Vec<TextLine>, bool) {
    static LINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?P<label>.*?)\s*(?P<sign>[+-])\s*(?P<num>\d+(?:\.\d+)?)\s*(?P<pct>%)?(?P<suffix>\s+(?:for|to|on|against|vs\.?)\b.*?)?\s*[.\u{3002}]?$").expect("fixed pattern"));
    static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\s\u{3000}.\u{30fb}*\u{2022}\u{b7}\u{fffd}]+").expect("fixed pattern"));
    static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(when|if|while|under|during|at|for|to)\b.*[:,]$").expect("fixed pattern"));
    static BULLET_GLYPH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\s\u{3000}]*[.\u{30fb}*\u{2022}\u{b7}\u{fffd}]").expect("fixed pattern"));
    let mut out = Vec::new();
    let mut incomplete = false;
    let mut when: Option<String> = None; // the heading of the block the line is in
    for raw in text.replace("\\n", "\n").lines() {
        let bulleted = BULLET_GLYPH.is_match(raw);
        let line = BULLET.replace(raw, "");
        let line = line.trim();
        if line.is_empty() || (line.starts_with('[') && line.ends_with(']')) {
            when = None;
            continue;
        }
        if line.starts_with('\u{203b}') {
            continue;
        }
        if HEADING.is_match(line) {
            when = Some(line.trim_end_matches(|c| c == ':' || c == ',').trim().to_string());
            continue;
        }
        let Some(c) = LINE.captures(line) else {
            incomplete |= bulleted && line.chars().any(|ch| ch.is_ascii_digit());
            continue;
        };
        let label = c["label"].trim().trim_end_matches(':').trim().to_string();
        if label.is_empty() {
            continue;
        }
        let suffix = c.name("suffix").map_or(String::new(), |m| m.as_str().trim_end().to_string());
        let Some(num) = crate::fmt::parse_f64(&c["num"]) else { continue };
        let value = if &c["sign"] == "-" { -num } else { num };
        let (ids, left) = scan(&label);
        let bare = ACTION_NAMES.iter().position(|a| normalize(&label) == *a);
        let extra: Vec<&str> = left.split(|ch: char| !ch.is_alphanumeric() && ch != '-').filter(|w| !w.is_empty() && !["all", "skill", "skills", "damage", "dealt", "the", "your", "of", "for", "and"].contains(w)).collect();
        let stat = ids.first().copied().or(bare.map(|i| FAMILIES.iter().map(|f| f.0).find(|f| *f == format!("EFFECT_POWER:{i}")).expect("a family per action")));
        incomplete |= bulleted && stat.is_none();
        out.push(TextLine {
            bulleted,
            conditional: when.is_some() || c["label"].contains(':'),
            when: when.clone(),
            stat,
            extra: if ids.is_empty() { String::new() } else { format!("{}{}", extra.join(" "), suffix) }.trim().to_string(),
            suffix,
            pct: c.name("pct").is_some(),
            label,
            value,
        });
    }
    (out, incomplete)
}

/// One effect of a tokusei the skill's text does not back.
#[derive(Debug, Clone, PartialEq)]
pub struct Stale {
    /// the stat or aspect id
    pub id: String,
    /// the tokusei's own number (the TokValue's Value, before its precision)
    pub value: f64,
    pub why: String,
}

/// The effects of `def` the skill's description (`text`) contradicts. Only a plain tokusei on the character itself is
/// judged (no multiplier, not read from the client's text, not a partner's).
pub fn stale_effects(def: &TokuseiDef, text: &str) -> Vec<Stale> {
    if def.from_client_text || def.target != "SELF" {
        return Vec::new();
    }
    let (all, incomplete) = scan_lines(text);
    let lines: Vec<TextLine> = all.into_iter().filter(|l| l.bulleted).collect();
    if lines.is_empty() {
        return Vec::new(); // the text lists no numbers: nothing to hold the tokusei against
    }
    // every stat the text names anywhere, prose included ("increases Melee Damage and Final Crit Correction")
    let named: Vec<&str> = text.replace("\\n", "\n").lines().flat_map(stats_named).collect();
    let mut out = Vec::new();
    for v in def.correct.iter().chain(def.aspects.iter()) {
        if !(v.mult.is_empty() || v.mult == "NONE") {
            continue;
        }
        // the power of one kind of skill is a family of its own; one that spans several kinds is not judged
        let action = match def.skill_conditions.iter().filter(|s| s.0.as_deref() == Some("ACTION_TYPE")).collect::<Vec<_>>().as_slice() {
            [one] if (0..ACTION_NAMES.len() as i32).contains(&one.1) => Some(one.1),
            _ => None,
        };
        let family = match (v.id.as_deref(), action) {
            (Some("EFFECT_POWER"), Some(a)) => FAMILIES.iter().map(|f| f.0).find(|f| *f == format!("EFFECT_POWER:{a}")),
            (Some(id), _) => FAMILIES.iter().map(|f| f.0).find(|f| *f == id),
            _ => None,
        };
        let Some(id) = family else { continue };
        let shown = if v.precision > 0 { v.value / 10f64.powi(v.precision) } else { v.value };
        let own: Vec<&TextLine> = lines.iter().filter(|l| l.stat == Some(id)).collect();
        if !own.is_empty() {
            if !own.iter().any(|l| (l.value - shown).abs() < 1e-9) {
                let says: Vec<String> = own.iter().map(|l| crate::fmt::g(l.value)).collect();
                out.push(Stale { id: v.id.clone().unwrap_or_default(), value: v.value, why: format!("{id}: the text says {}, the tokusei {}", says.join(" / "), crate::fmt::g(shown)) });
            }
        } else if !incomplete && !named.contains(&id) {
            out.push(Stale { id: v.id.clone().unwrap_or_default(), value: v.value, why: format!("{id}: the text never mentions it (the tokusei has {})", crate::fmt::g(shown)) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::game_data::TokValue;

    fn def(correct: Vec<TokValue>) -> TokuseiDef {
        TokuseiDef { id: 1, target: "SELF".into(), conditions: Vec::new(), skill_conditions: Vec::new(), target_conditions: false, correct, aspects: Vec::new(), from_client_text: false }
    }

    const LORD_OF_VICTORY: &str = "A switch skill that increases \n\
        Melee Damage and Final Crit Correction.\n\
        However, it also reduces healing received.\n\n\
        [Plus Effect]\n. Close-Range Damage +15%\n. FCC +5%\n\n[Minus Effect]\n. Healing received -25% \n";

    #[test]
    fn stats_are_named_the_way_the_game_words_them() {
        assert_eq!(stats_named("Close-Range Damage +15%"), vec!["RATE_CLSR"]);
        assert_eq!(stats_named("CLS-R Damage taken"), vec!["RATE_CLSR_TAKEN"]);
        assert_eq!(stats_named("Melee Damage and Final Crit Correction"), vec!["RATE_CLSR", "FINAL_CRIT_CHANCE"]);
        assert_eq!(stats_named("FCC"), vec!["FINAL_CRIT_CHANCE"]);
        assert_eq!(stats_named("Critical +100"), vec!["CRITICAL"]);
        assert_eq!(stats_named("Critical Defense -50"), vec!["CRIT_DEF"]);
        assert_eq!(stats_named("Healing received -25%"), vec!["RATE_HEAL_TAKEN"]);
        assert_eq!(stats_named("Fire Skill Incantation Time -20%"), vec!["CHANT_TIME"]);
        assert!(stats_named("Null Knockback").is_empty());
        // one damage word for a whole list (Lord of Armor)
        assert_eq!(stats_named("CLS-R, LNG-R and Magic Damage suffered -50%"), vec!["RATE_CLSR_TAKEN", "RATE_LNGR_TAKEN", "RATE_SPELL_TAKEN"]);
        assert_eq!(stats_named("CLS-R, LNG-R and Magic Damage Dealt -100%"), vec!["RATE_CLSR", "RATE_LNGR", "RATE_SPELL"]);
    }

    #[test]
    fn effect_lines_read_the_listed_numbers() {
        let l = effect_lines(LORD_OF_VICTORY);
        let got: Vec<(Option<&str>, f64, bool)> = l.iter().map(|x| (x.stat, x.value, x.pct)).collect();
        assert_eq!(got, vec![(Some("RATE_CLSR"), 15.0, true), (Some("FINAL_CRIT_CHANCE"), 5.0, true), (Some("RATE_HEAL_TAKEN"), -25.0, true)]);
        // a conditional line is marked, a note and a formula are not lines
        let c = effect_lines("Passive skill.\n\n・Long-Range Damage Dealt to Human family: +50%\n・Pursuit chance + Strength * 0.1%\n　※Caps at 100%");
        assert_eq!(c.len(), 1);
        assert!(c[0].conditional && c[0].stat == Some("RATE_LNGR"));
        // under a "When ...:" heading the lines only apply then, and the heading is kept to say so (Lord of Soldiers)
        let s = effect_lines("Passive Skill\n\nWhen using any Spin Skills:\n・Spin boost +10%\n・Crit +40");
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|l| l.conditional && l.when.as_deref() == Some("When using any Spin Skills")));
        assert_eq!((s[1].stat, s[1].value), (Some("CRITICAL"), 40.0));
        // a block ends at the next blank line
        let e = effect_lines("When X:\n・Crit +1\n\n・Crit +2");
        assert_eq!((e[0].conditional, e[1].conditional), (true, false));
    }

    #[test]
    fn lines_that_say_for_which_skills_and_lists_that_are_not_understood() {
        // "HP Cost -30% for Rush skills" is a line, and says it is not for every skill
        let l = effect_lines(". Rush Boost +30%\n. HP Cost -30% for Rush skills");
        assert_eq!((l[1].stat, l[1].value, l[1].suffix.as_str(), l[1].extra.as_str()), (Some("HP_COST_ADJUST"), -30.0, " for Rush skills", "for Rush skills"));
        assert_eq!((l[0].stat, l[0].extra.as_str()), (Some("EFFECT_POWER:2"), ""));
        // a bulleted number the module cannot read (LB Upper Limit Increase is named; "Spin Boost" typos are not) means a
        // stat missing from the list proves nothing: Murderous Secret's text lists only what its tokusei gives
        let lb = def(vec![TokValue::new(Some("LIMIT_BREAK_MAX"), 0, 500.0, "NONE", 0)]);
        assert!(stale_effects(&lb, "Passive skill.\n\n・LB Upper Limit Increase +500").is_empty());
        let unreadable = def(vec![TokValue::new(Some("CRITICAL"), 0, 100.0, "NONE", 0)]);
        assert!(stale_effects(&unreadable, ". Close-Range Damage +15%\n. Attck Boost +10%").is_empty());
        assert_eq!(stale_effects(&unreadable, ". Close-Range Damage +15%").len(), 1);
        // a "Name: value" line (demon traits) is not a bulleted list, so it is never held against a tokusei
        let trait_line = def(vec![TokValue::new(Some("MP_COST_ADJUST"), 0, -30.0, "NONE", 0)]);
        assert!(stale_effects(&trait_line, "A goddess.\n\nAll Skills' HP/MP Costs: -30%\nAll Skills' Cooldown Time: -30%").is_empty());
    }

    #[test]
    fn lord_of_victory_old_tokusei_is_stale() {
        // 520061 as the public server has it: +25% Close-Range Damage and Critical +100
        let old = def(vec![TokValue::new(Some("RATE_CLSR"), 1, 25.0, "NONE", 0), TokValue::new(Some("CRITICAL"), 0, 100.0, "NONE", 0)]);
        let stale = stale_effects(&old, LORD_OF_VICTORY);
        let ids: Vec<(&str, f64)> = stale.iter().map(|s| (s.id.as_str(), s.value)).collect();
        assert_eq!(ids, vec![("RATE_CLSR", 25.0), ("CRITICAL", 100.0)], "{stale:?}");
        assert!(stale[0].why.contains("15"), "{}", stale[0].why);
        assert!(stale[1].why.contains("never mentions"), "{}", stale[1].why);
    }

    #[test]
    fn a_tokusei_the_text_agrees_with_is_left_alone() {
        let ok = def(vec![TokValue::new(Some("RATE_CLSR"), 1, 15.0, "NONE", 0)]);
        assert!(stale_effects(&ok, LORD_OF_VICTORY).is_empty());
        // a description without numbers judges nothing; neither does one for a partner's or client-text tokusei
        let old = def(vec![TokValue::new(Some("RATE_CLSR"), 1, 25.0, "NONE", 0)]);
        assert!(stale_effects(&old, "Increases your damage a lot.").is_empty());
        let mut partner = old.clone();
        partner.target = "PARTNER".into();
        assert!(stale_effects(&partner, LORD_OF_VICTORY).is_empty());
        let mut from_text = old.clone();
        from_text.from_client_text = true;
        assert!(stale_effects(&from_text, LORD_OF_VICTORY).is_empty());
        // scaled values (LV * 0.5) and stats without wording are not judged
        let scaled = def(vec![TokValue::new(Some("CRITICAL"), 0, 50.0, "LEVEL", 2), TokValue::new(Some("SOMETHING_ELSE"), 0, 9.0, "NONE", 0)]);
        assert!(stale_effects(&scaled, LORD_OF_VICTORY).is_empty());
    }

    #[test]
    fn advanced_sorcery_circulation_text_says_15_not_20() {
        // 5533: the text says "Magic incantation time -15%", the tokusei still has -20
        let t = "Passive skill. \n\n・All skill MP cost -25%\n・Magic incantation time -15%";
        let d = def(vec![TokValue::new(Some("CHANT_TIME"), 1, -20.0, "NONE", 0), TokValue::new(Some("MP_COST_ADJUST"), 0, -25.0, "NONE", 0)]);
        let stale = stale_effects(&d, t);
        assert_eq!(stale.len(), 1, "{stale:?}");
        assert_eq!((stale[0].id.as_str(), stale[0].value), ("CHANT_TIME", -20.0));
    }
}
