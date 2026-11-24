//! The C# `Json` helpers the app uses on the page's messages (src\Data\Json.cs: Str, Int, Num, Truthy), plus a few
//! text helpers shared by the modules.

use serde_json::Value;

/// `Json.Num`: a number, or a string that reads as one.
pub fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => imagine::fmt::parse_f64(s),
        _ => None,
    }
}

/// `Json.Int(n, dflt)`: `(int)` of the number (truncated), else the default.
pub fn int(v: Option<&Value>, dflt: i32) -> i32 {
    match num(v) {
        Some(d) => d as i32,
        None => dflt,
    }
}

/// `Json.Str`: strings as they are, numbers and booleans as text; objects, arrays and null are null.
pub fn str(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(match n.as_f64() {
            Some(d) => imagine::fmt::g(d),
            None => n.to_string(),
        }),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        _ => None,
    }
}

/// `Json.Truthy`
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty() && !s.eq_ignore_ascii_case("false"),
        Some(Value::Number(n)) => n.as_f64().map(|d| d != 0.0).unwrap_or(true),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(_)) => true,
    }
}

/// `Uri.EscapeDataString`: everything except the unreserved characters (A-Z a-z 0-9 - . _ ~) as %XX of its UTF-8 bytes.
pub fn escape_data_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Percent-decoding of a URL path (invalid sequences are kept as they are); None when the bytes are not UTF-8.
pub fn unescape(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = ((b[i + 1] as char).to_digit(16), (b[i + 2] as char).to_digit(16)) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// `JsonSerializer.Serialize(string)`: a JSON string literal (also valid JavaScript).
pub fn quote(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}
