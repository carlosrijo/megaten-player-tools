//! Port of `src/Build/Csv.cs`: the snapshot spreadsheet: one row per snapshot, every field quoted (same format as before).

use std::path::Path;

use indexmap::IndexMap;

use crate::Result;
use crate::data::json;

/// C# `CsvTable`. A row is a column name -> text map (insertion ordered, so what a caller iterates matches C#).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CsvTable {
    pub columns: Vec<String>,
    pub rows: Vec<IndexMap<String, String>>,
}

impl CsvTable {
    /// `CsvTable.Load`: an empty table when the file does not exist.
    pub fn load(path: impl AsRef<Path>) -> Result<CsvTable> {
        let path = path.as_ref();
        let mut t = CsvTable::default();
        if !path.exists() {
            return Ok(t);
        }
        let mut records: Vec<Vec<String>> = parse(&json::read_all_text(path)?).into_iter().map(|r| r.into_iter().map(unmark).collect()).collect();
        if records.is_empty() {
            return Ok(t);
        }
        t.columns = records.remove(0);
        for r in records {
            if r.len() == 1 && r[0].is_empty() {
                continue;
            }
            let mut d = IndexMap::new();
            for (i, c) in t.columns.iter().enumerate() {
                d.insert(c.clone(), r.get(i).cloned().unwrap_or_default());
            }
            t.rows.push(d);
        }
        Ok(t)
    }

    /// `Get(row, col)`: "" when the row has no such column.
    pub fn get(&self, row: &IndexMap<String, String>, col: &str) -> String {
        row.get(col).cloned().unwrap_or_default()
    }

    /// `Save`: CRLF line ends, UTF-8 without BOM, the folder created.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let mut sb = String::new();
        sb.push_str(&self.columns.iter().map(|c| q(c)).collect::<Vec<_>>().join(","));
        sb.push_str("\r\n");
        for r in &self.rows {
            sb.push_str(&self.columns.iter().map(|c| q(&self.get(r, c))).collect::<Vec<_>>().join(","));
            sb.push_str("\r\n");
        }
        json::create_parent(path)?;
        json::write_atomic(path, sb)?;
        Ok(())
    }
}

fn q(s: &str) -> String {
    let s = if needs_text_mark(s) { format!("'{s}") } else { s.to_string() };
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A cell a spreadsheet would read as a formula (it starts with = + - @, a tab or a line break, and is not a number):
/// saved with a ' in front, which spreadsheets show as plain text. Any ' it already has in front is looked past, so
/// such a cell gets one more and reads back as it was: [`q`] adds the ', [`unmark`] takes it off when the file is read.
fn needs_text_mark(s: &str) -> bool {
    let s = s.trim_start_matches('\'');
    let Some(first) = s.chars().next() else { return false };
    match first {
        '=' | '@' | '\t' | '\r' | '\n' => true,
        '+' | '-' => !(s.parse::<f64>().is_ok() && s[1..].chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))),
        _ => false,
    }
}

/// A cell as saved, without the ' [`q`] put in front of it.
fn unmark(s: String) -> String {
    match s.strip_prefix('\'') {
        Some(rest) if needs_text_mark(rest) => rest.to_string(),
        _ => s,
    }
}

fn parse(text: &str) -> Vec<Vec<String>> {
    let chars: Vec<char> = text.chars().collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut f = String::new();
    let mut in_q = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_q {
            if c == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    f.push('"');
                    i += 1;
                } else {
                    in_q = false;
                }
            } else {
                f.push(c);
            }
        } else if c == '"' {
            in_q = true;
        } else if c == ',' {
            row.push(std::mem::take(&mut f));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && i + 1 < chars.len() && chars[i + 1] == '\n' {
                i += 1;
            }
            row.push(std::mem::take(&mut f));
            rows.push(std::mem::take(&mut row));
        } else {
            f.push(c);
        }
        i += 1;
    }
    if !f.is_empty() || !row.is_empty() {
        row.push(f);
        rows.push(row);
    }
    if let Some(first) = rows.first_mut().and_then(|r| r.first_mut()) {
        if let Some(rest) = first.strip_prefix('\u{feff}') {
            *first = rest.to_string();
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_quirks() {
        let dir = std::env::temp_dir().join(format!("imagine-csv-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("sub").join("t.csv");
        let mut t = CsvTable { columns: vec!["a".into(), "b \"q\"".into(), "c".into()], rows: vec![] };
        let mut r = IndexMap::new();
        r.insert("a".to_string(), "1,2".to_string());
        r.insert("b \"q\"".to_string(), "line\r\nbreak \"x\"".to_string());
        t.rows.push(r);
        t.save(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text, "\"a\",\"b \"\"q\"\"\",\"c\"\r\n\"1,2\",\"line\r\nbreak \"\"x\"\"\",\"\"\r\n");
        let back = CsvTable::load(&p).unwrap();
        assert_eq!(back.columns, t.columns);
        assert_eq!(back.rows.len(), 1);
        assert_eq!(back.get(&back.rows[0], "b \"q\""), "line\r\nbreak \"x\"");
        assert_eq!(back.get(&back.rows[0], "nope"), "");
        // a BOM, short rows, blank lines and a missing final newline
        std::fs::write(&p, "\u{feff}x,y\n1\n\n2,3,4").unwrap();
        let t2 = CsvTable::load(&p).unwrap();
        assert_eq!(t2.columns, vec!["x", "y"]);
        assert_eq!(t2.rows.len(), 2);
        assert_eq!(t2.get(&t2.rows[0], "y"), "");
        assert_eq!(t2.get(&t2.rows[1], "y"), "3");
        assert!(CsvTable::load(dir.join("none.csv")).unwrap().columns.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_that_looks_like_a_formula_is_saved_as_text_and_read_back_as_it_was() {
        let dir = std::env::temp_dir().join(format!("imagine-csv-mark-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("t.csv");
        let cells = ["=1+2", "@SUM(A1)", "+x", "-x", "\tx", "\r\nx", "'=1", "''=1", "-5", "+5", "-0.25", "-1e3", "1-2", "'plain", "plain", "", "-", "+"];
        let columns: Vec<String> = (0..cells.len()).map(|i| format!("c{i}")).collect();
        let mut row = IndexMap::new();
        for (c, v) in columns.iter().zip(cells) {
            row.insert(c.clone(), v.to_string());
        }
        let t = CsvTable { columns: columns.clone(), rows: vec![row] };
        t.save(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let line = text.split_once("\r\n").unwrap().1; // the row (one of its cells has a line break in it)
        // marked: the formula-like text (and a ' already in front of it); numbers and plain text are as they were
        assert!(line.starts_with(r#""'=1+2","'@SUM(A1)","'+x","'-x","'	x","'"#), "{line}");
        assert!(line.contains(r#""''=1","'''=1","-5","+5","-0.25","-1e3","1-2","'plain","plain","","'-","'+""#), "{line}");
        let back = CsvTable::load(&p).unwrap();
        for (c, v) in columns.iter().zip(cells) {
            assert_eq!(back.get(&back.rows[0], c), v, "{c}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
