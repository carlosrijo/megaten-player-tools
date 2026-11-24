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
        let mut records = parse(&json::read_all_text(path)?);
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
        std::fs::write(path, sb)?;
        Ok(())
    }
}

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
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
        let dir = std::env::temp_dir().join(format!("newmoon-csv-test-{}", std::process::id()));
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
}
