//! String comparison as .NET did it.
//!
//! - `StringComparer.Ordinal` / `string ==`: plain Rust `==` / `Ord` on `str`.
//! - `StringComparison.OrdinalIgnoreCase`, `StringComparer.OrdinalIgnoreCase` (dictionary keys, `Equals(..., OrdinalIgnoreCase)`):
//!   [`eq_ic`] and [`ic_key`] (use `ic_key(s)` as the key of a map that was case-insensitive in C#).
//! - Default string ordering (`OrderBy(x => x.Name)`, `List<string>.Sort()`, `string.Compare(a, b)` without a comparer):
//!   culture-aware (invariant culture, ICU) in .NET, NOT ordinal. Use [`cmp_invariant`]. It approximates ICU's root
//!   collation: white space < punctuation and symbols < digits < letters (case-insensitive first, lower case before upper
//!   case on a tie), other characters by code point after those.

use std::cmp::Ordering;

/// `string.Equals(a, b, StringComparison.OrdinalIgnoreCase)`.
pub fn eq_ic(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.chars().zip(b.chars()).all(|(x, y)| x == y || x.to_uppercase().eq(y.to_uppercase()))
        || ic_key(a) == ic_key(b)
}

/// A key that makes a map behave like a C# `Dictionary<string, T>(StringComparer.OrdinalIgnoreCase)`.
pub fn ic_key(s: &str) -> String {
    s.to_uppercase()
}

fn class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_ascii_punctuation() || (!c.is_alphanumeric() && (c as u32) < 0x3000) {
        1
    } else if c.is_numeric() {
        2
    } else if c.is_alphabetic() && (c as u32) < 0x2E80 {
        3
    } else {
        4
    }
}

fn primary(c: char) -> (u8, u32) {
    let lower = c.to_lowercase().next().unwrap_or(c);
    (class(c), lower as u32)
}

/// .NET's default (culture-aware, invariant culture) string order, approximated. See the module comment.
pub fn cmp_invariant(a: &str, b: &str) -> Ordering {
    let pa = a.chars().map(primary);
    let pb = b.chars().map(primary);
    match pa.cmp(pb) {
        Ordering::Equal => {}
        o => return o,
    }
    // same letters ignoring case: lower case first at the first difference
    for (x, y) in a.chars().zip(b.chars()) {
        if x != y {
            let xl = x.is_lowercase();
            let yl = y.is_lowercase();
            if xl != yl {
                return if xl { Ordering::Less } else { Ordering::Greater };
            }
            return x.cmp(&y);
        }
    }
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_order() {
        let mut v = vec!["b", "B", "a", "A", "10", "9", "_x", " z", "Zed", "apple"];
        v.sort_by(|a, b| cmp_invariant(a, b));
        assert_eq!(v, vec![" z", "_x", "10", "9", "a", "A", "apple", "b", "B", "Zed"]);
    }

    #[test]
    fn ignore_case() {
        assert!(eq_ic("Rubito", "RUBITO"));
        assert!(!eq_ic("Rubito", "Rubit"));
        assert_eq!(ic_key("Pixie"), ic_key("PIXIE"));
    }
}
