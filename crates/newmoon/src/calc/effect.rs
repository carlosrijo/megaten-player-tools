//! Port of `src/Calc/Effect.cs`.
//!
//! One bonus line ([`Effect`]) and the case-insensitive regex helpers every calc file uses ([`rx`]).
//!
//! .NET regex notes for patterns ported as they are: `$` matches before a final `\n` in .NET but only at the very end
//! here; every text these patterns see is a single line without a line break, so it makes no difference.

/// One bonus line. Cond: "always" | "digitalized" (both counted) | "situational" (depends on the fight/partner: shown, not counted)
/// | "inactive" (a requirement the character doesn't meet) | "unparsed" (looked like a bonus but wasn't understood)
/// | "fusion-set" (tooltip text of a fusion-set bonus; the fusion-set table decides it instead).
/// Source: item | characteristic | tarot | soul | mod | set | appearance | fusion set.
///
/// The C# strings that can be null (Label, Source, Item) are empty strings here: nothing in the C# tells the two apart.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub label: String,
    pub value: Option<f64>,
    pub pct: bool,
    pub cond: String,
    pub cond_note: String,
    pub target: String,
    pub source: String,
    pub item: String,
    pub tokusei: i32,
    /// The skill this comes from, when it is a digitalize demon's skill or trait (for its icon); else 0.
    pub skill_id: i32,
    /// Set when the server data for this part is missing (the reader then uses the tooltip's lines for it).
    pub missing_data: bool,
}

impl Default for Effect {
    fn default() -> Self {
        Effect {
            label: String::new(),
            value: None,
            pct: false,
            cond: "always".to_string(),
            cond_note: String::new(),
            target: "self".to_string(),
            source: String::new(),
            item: String::new(),
            tokusei: 0,
            skill_id: 0,
            missing_data: false,
        }
    }
}

impl Effect {
    /// "always" and "digitalized" effects are counted in the totals.
    pub fn counted(&self) -> bool {
        self.cond == "always" || self.cond == "digitalized"
    }
}

/// Case-insensitive regex helpers (the old scripts used PowerShell's -match, which ignores case).
pub mod rx {
    use regex::{Captures, Regex, RegexBuilder};
    use std::collections::HashMap;
    use std::sync::{LazyLock, RwLock};

    static CACHE: LazyLock<RwLock<HashMap<String, Regex>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

    /// C# `Rx.R`: the compiled, case-insensitive regex of a pattern (compiled once).
    pub fn r(p: &str) -> Regex {
        if let Some(re) = CACHE.read().expect("regex cache").get(p) {
            return re.clone();
        }
        let re = RegexBuilder::new(p).case_insensitive(true).build().unwrap_or_else(|e| panic!("bad pattern {p:?}: {e}"));
        CACHE.write().expect("regex cache").insert(p.to_string(), re.clone());
        re
    }

    /// C# `Rx.M`: the first match of the pattern in `s`.
    pub fn m<'t>(s: &'t str, p: &str) -> Option<Captures<'t>> {
        r(p).captures(s)
    }

    /// C# `Rx.Is`.
    pub fn is(s: &str, p: &str) -> bool {
        r(p).is_match(s)
    }

    /// C# `Rx.Rep`: every match replaced (`$1` in `repl` is a group, as in .NET).
    pub fn rep(s: &str, p: &str, repl: &str) -> String {
        r(p).replace_all(s, repl).into_owned()
    }

    /// C# `Rx.Split` (patterns without capture groups).
    pub fn split(s: &str, p: &str) -> Vec<String> {
        r(p).split(s).map(str::to_string).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_counted() {
        let mut e = Effect::default();
        assert_eq!((e.cond.as_str(), e.target.as_str(), e.cond_note.as_str()), ("always", "self", ""));
        assert!(e.counted());
        e.cond = "digitalized".into();
        assert!(e.counted());
        e.cond = "situational".into();
        assert!(!e.counted());
    }

    #[test]
    fn rx_ignores_case() {
        assert!(rx::is("CRITICAL", "^Critical$"));
        assert_eq!(rx::rep("a  b   c", r"\s+", " "), "a b c");
        assert_eq!(rx::split("a, b,c", r",\s*"), vec!["a", "b", "c"]);
        assert_eq!(rx::rep("+ 5", r"([+\-])\s+(\d)", "$1$2"), "+5");
    }
}
