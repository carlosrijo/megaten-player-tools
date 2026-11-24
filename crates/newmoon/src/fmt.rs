//! Number formatting and rounding that match .NET (InvariantCulture) exactly.
//!
//! The C# reader wrote numbers into the CSV, tooltips, page data and console output with .NET's rules, which differ
//! from Rust's in a few places. Use these helpers wherever the C# code formatted or rounded a double:
//!
//! | C#                                   | Rust                     |
//! |--------------------------------------|--------------------------|
//! | `x.ToString()`, `$"{x}"`, `"R"`, JSON | [`g`]                    |
//! | `x.ToString("0.##")`, `$"{x:0.#}"`   | [`custom`]`(x, "0.##")`  |
//! | `x.ToString("F2")`                   | [`fixed`]`(x, 2)`        |
//! | `x.ToString("N0")`                   | [`n0`]                   |
//! | `Math.Round(x, 2)` (to even)         | [`round`]`(x, 2)`        |
//! | `Math.Round(x, 2, AwayFromZero)`     | [`round_away`]`(x, 2)`   |
//! | `double.Parse(s, Invariant)`         | [`parse_f64`]            |
//!
//! Checked against vectors produced by .NET 10 (`tests/data/dotnet-format-vectors.tsv`).

/// Decimal digits (no leading zeros; empty for zero) and scale: |x| = 0.d1d2d3... × 10^scale.
fn decompose(sci: &str) -> (Vec<u8>, i32) {
    // sci is Rust's `{:e}` output of a non-negative finite number, e.g. "1.2345e-5" or "0e0"
    let (mant, exp) = sci.split_once('e').expect("scientific form");
    let exp: i32 = exp.parse().expect("exponent");
    let digits: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).map(|b| b - b'0').collect();
    if digits.iter().all(|&d| d == 0) {
        return (Vec::new(), 0);
    }
    let mut digits = digits;
    while digits.last() == Some(&0) {
        digits.pop();
    }
    (digits, exp + 1)
}

/// .NET's default `double.ToString()` (also `"R"`, string interpolation and System.Text.Json): the shortest digits
/// that read back as the same double; scientific (`1E+17`, `1E-05`) when the exponent is >= 17 or < -4.
pub fn g(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let neg = x.is_sign_negative();
    let (digits, scale) = decompose(&format!("{:e}", x.abs()));
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    if digits.is_empty() {
        s.push('0');
        return s;
    }
    let exp = scale - 1;
    if exp >= 17 || exp < -4 {
        s.push((b'0' + digits[0]) as char);
        if digits.len() > 1 {
            s.push('.');
            for d in &digits[1..] {
                s.push((b'0' + d) as char);
            }
        }
        s.push('E');
        s.push(if exp >= 0 { '+' } else { '-' });
        s.push_str(&format!("{:02}", exp.abs()));
        return s;
    }
    push_plain(&mut s, &digits, scale, 0);
    s
}

/// Writes digits (value 0.d1d2... × 10^scale) as plain decimal text: at least `min_int` integer digits (at least one),
/// and all the fraction digits present (the caller has already cut/rounded them).
fn push_plain(s: &mut String, digits: &[u8], scale: i32, min_int: usize) {
    let int_len = scale.max(0) as usize;
    let mut int_part = String::new();
    for i in 0..int_len {
        int_part.push((b'0' + digits.get(i).copied().unwrap_or(0)) as char);
    }
    let min_int = min_int.max(1);
    while int_part.len() < min_int {
        int_part.insert(0, '0');
    }
    s.push_str(&int_part);
    let frac: String = if scale >= digits.len() as i32 {
        String::new()
    } else {
        let mut f = String::new();
        for _ in 0..(-scale).max(0) {
            f.push('0');
        }
        for d in &digits[int_len..] {
            f.push((b'0' + d) as char);
        }
        f
    };
    if !frac.is_empty() {
        s.push('.');
        s.push_str(&frac);
    }
}

/// .NET custom numeric formats of the shapes the reader uses: `"0"`, `"000"`, `"0.#"`, `"0.##"`, `"0.###"`,
/// `"0.#####"`, `"0.00"` and the like (zeros, then optionally a point followed by zeros and/or `#`).
/// .NET first takes 15 significant digits, then rounds half away from zero at the last shown place; a negative
/// number that rounds to zero keeps its sign (`"-0"`), as .NET Core 3.0+ does.
pub fn custom(x: f64, pattern: &str) -> String {
    let (int_pat, dec_pat) = pattern.split_once('.').unwrap_or((pattern, ""));
    assert!(
        int_pat.chars().all(|c| c == '0' || c == '#') && dec_pat.chars().all(|c| c == '0' || c == '#'),
        "fmt::custom: unsupported pattern {pattern:?}"
    );
    let min_int = int_pat.chars().filter(|&c| c == '0').count();
    let dec_zero = dec_pat.chars().filter(|&c| c == '0').count();
    let dec_max = dec_pat.len();
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let neg = x.is_sign_negative();
    let (mut digits, mut scale) = decompose(&format!("{:.14e}", x.abs()));
    // round half away from zero at `dec_max` decimals
    let cut = scale + dec_max as i32;
    if cut < 0 {
        digits.clear();
    } else if (cut as usize) < digits.len() {
        let round_up = digits[cut as usize] >= 5;
        digits.truncate(cut as usize);
        if round_up {
            let mut i = digits.len();
            loop {
                if i == 0 {
                    digits.insert(0, 1);
                    scale += 1;
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
        while digits.last() == Some(&0) {
            digits.pop();
        }
    }
    if digits.is_empty() {
        scale = 0;
    }
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    // integer part
    let int_len = scale.max(0) as usize;
    let mut int_part = String::new();
    for i in 0..int_len {
        int_part.push((b'0' + digits.get(i).copied().unwrap_or(0)) as char);
    }
    while int_part.len() < min_int {
        int_part.insert(0, '0');
    }
    s.push_str(&int_part);
    // fraction: every digit up to dec_max, then trailing zeros trimmed down to dec_zero
    let mut frac = String::new();
    for p in 0..dec_max as i32 {
        let idx = scale + p; // index into digits of the p-th decimal
        let d = if idx < 0 { 0 } else { digits.get(idx as usize).copied().unwrap_or(0) };
        frac.push((b'0' + d) as char);
    }
    while frac.len() > dec_zero && frac.ends_with('0') {
        frac.pop();
    }
    if !frac.is_empty() {
        s.push('.');
        s.push_str(&frac);
    }
    s
}

/// .NET `"F<n>"`: exact value rounded half to even (the same as Rust's `{:.n}`).
pub fn fixed(x: f64, decimals: usize) -> String {
    format!("{:.*}", decimals, x)
}

/// .NET `"N0"`: rounded like [`fixed`] with `,` between thousands.
pub fn n0(x: f64) -> String {
    let s = format!("{:.0}", x);
    let (sign, digits) = match s.strip_prefix('-') {
        Some(d) => ("-", d),
        None => ("", s.as_str()),
    };
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

/// .NET `Math.Round(x, digits)` (midpoints to even).
pub fn round(x: f64, digits: i32) -> f64 {
    if x.abs() >= 1e16 || !x.is_finite() {
        return x;
    }
    let p = 10f64.powi(digits);
    (x * p).round_ties_even() / p
}

/// .NET `Math.Round(x, digits, MidpointRounding.AwayFromZero)`.
pub fn round_away(x: f64, digits: i32) -> f64 {
    if x.abs() >= 1e16 || !x.is_finite() {
        return x;
    }
    let p = 10f64.powi(digits);
    (x * p).round() / p
}

/// .NET `NumberStyles` white space: only ASCII 0x09-0x0D and space (Rust's `trim` would also strip NBSP etc.).
fn trim_net(s: &str) -> &str {
    s.trim_matches(|c: char| c == ' ' || ('\u{9}'..='\u{d}').contains(&c))
}

/// `double.Parse` / `TryParse` with InvariantCulture and NumberStyles.Float: surrounding ASCII white space allowed,
/// "NaN", "Infinity" and "-Infinity" (any case) accepted, too-large numbers give ±Infinity (.NET Core 3.0+).
pub fn parse_f64(s: &str) -> Option<f64> {
    let t = trim_net(s);
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "nan" | "+nan" | "-nan" => return Some(f64::NAN),
        "infinity" | "+infinity" | "∞" | "+∞" => return Some(f64::INFINITY),
        "-infinity" | "-∞" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    // Rust also reads "inf"/"nan" spellings .NET doesn't; anything with letters other than an exponent is rejected
    if lower.chars().any(|c| c.is_ascii_alphabetic() && c != 'e') {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `int.Parse` / `TryParse` with InvariantCulture (NumberStyles.Integer: ASCII white space and a leading sign allowed).
pub fn parse_i32(s: &str) -> Option<i32> {
    trim_net(s).parse::<i32>().ok()
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Columns: bits, ToString(), 0.##, 0.#, 0.###, 0.#####, 0.00, 000, 00000, N0, F2, R,
    /// Round(2), Round(2, Away), Round(3), Round(0), JSON
    #[test]
    fn matches_dotnet_vectors() {
        let text = include_str!("../tests/data/dotnet-format-vectors.tsv");
        let mut failures = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let c: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            let x = f64::from_bits(c[0].parse::<i64>().unwrap() as u64);
            let got = [
                g(x),
                custom(x, "0.##"),
                custom(x, "0.#"),
                custom(x, "0.###"),
                custom(x, "0.#####"),
                custom(x, "0.00"),
                custom(x, "000"),
                custom(x, "00000"),
                n0(x),
                fixed(x, 2),
                g(x),
                g(round(x, 2)),
                g(round_away(x, 2)),
                g(round(x, 3)),
                g(round(x, 0)),
                g(x),
            ];
            for (i, want) in c[1..].iter().enumerate() {
                if got[i] != *want {
                    failures.push(format!("{x:e} col {}: want {want:?} got {:?}", i + 1, got[i]));
                }
            }
        }
        assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
    }
}
