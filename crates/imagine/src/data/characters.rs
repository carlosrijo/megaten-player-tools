//! Port of `src/Data/Characters.cs`.
//!
//! The list of known characters (data\characters.json). Every character has its own folder (caches, tooltips,
//! expertise ranks, page data) and its own CSV, so nothing is shared or overwritten between characters.
//! The data that existed before this list (kept directly in the data folder) becomes the first entry, where it is.
//!
//! characters.json is read and written by both the C# and the Rust program: the property names are the C# member names
//! (`Name`, `Dir`, `Csv`, `AutoPicture`), written with .NET's default JSON encoder (non-ASCII as \uXXXX) and \r\n line ends.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use super::json::{self, At, Escape};
use crate::Result;

/// One character the tool keeps data for: where its saved files live and which spreadsheet it writes to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct CharacterEntry {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    /// Folder for this character's files, relative to the data folder ("." = the data folder itself).
    #[serde(rename = "Dir")]
    pub dir: Option<String>,
    #[serde(rename = "Csv")]
    pub csv: Option<String>,
    /// The app only: false = keep the picture the person chose; otherwise a snapshot taken with the Information window
    /// open replaces the picture with that window's character view.
    #[serde(rename = "AutoPicture")]
    pub auto_picture: Option<bool>,
}

impl CharacterEntry {
    fn to_json(&self) -> Value {
        json!({ "Name": self.name, "Dir": self.dir, "Csv": self.csv, "AutoPicture": self.auto_picture })
    }
}

pub struct Characters {
    root: PathBuf,
    default_csv: String,
    pub all: Vec<CharacterEntry>,
}

/// `string.IsNullOrWhiteSpace`.
fn is_blank(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.chars().all(char::is_whitespace))
}

impl Characters {
    /// C# `new Characters(dataRoot, defaultName, defaultCsv)`. Saving the first entry can fail (an IO error), as in C#.
    pub fn new(data_root: impl Into<PathBuf>, default_name: &str, default_csv: &str) -> Result<Characters> {
        let mut c = Characters { root: data_root.into(), default_csv: default_csv.to_string(), all: Vec::new() };
        let file = c.file_path();
        if file.exists() {
            // try { ... } catch { All = new(); }
            c.all = (|| -> Result<Vec<CharacterEntry>> {
                let text = json::read_all_text(&file)?;
                let list: Option<Vec<Option<CharacterEntry>>> = serde_json::from_str(&text)?;
                Ok(list.unwrap_or_default().into_iter().map(|e| e.unwrap_or_default()).collect())
            })()
            .unwrap_or_default();
            c.all.retain(|e| !is_blank(e.name.as_deref()));
        } else if c.root.join("dash").is_dir() || c.root.join("cache").is_dir() {
            let name = c.existing_name().unwrap_or_else(|| default_name.to_string());
            c.all.push(CharacterEntry { name: Some(name), dir: Some(".".into()), csv: Some(default_csv.to_string()), auto_picture: None });
            c.save()?;
        }
        Ok(c)
    }

    fn file_path(&self) -> PathBuf {
        self.root.join("characters.json")
    }

    /// Whose data is already in the data folder: the character named in its latest saved snapshot.
    fn existing_name(&self) -> Option<String> {
        let p = self.root.join("dash").join("snapshots.json");
        if !p.exists() {
            return None;
        }
        let v = json::load(&p).ok()?;
        let n = json::str(json::arr(Some(&v)).last().at("character"))?;
        if n.chars().all(char::is_whitespace) { None } else { Some(n) }
    }

    pub fn find(&self, name: &str) -> Option<&CharacterEntry> {
        self.all.iter().find(|e| e.name.as_deref().is_some_and(|n| crate::text::eq_ic(n, name)))
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut CharacterEntry> {
        self.all.iter_mut().find(|e| e.name.as_deref().is_some_and(|n| crate::text::eq_ic(n, name)))
    }

    /// The entry for a character, created with its own folder and CSV the first time the name is seen.
    pub fn get_or_add(&mut self, name: &str) -> Result<&mut CharacterEntry> {
        if let Some(i) = self.all.iter().position(|e| e.name.as_deref().is_some_and(|n| crate::text::eq_ic(n, name))) {
            return Ok(&mut self.all[i]);
        }
        let safe: String = name.chars().map(|c| if is_invalid_file_name_char(c) || c == '.' { '_' } else { c }).collect();
        let first_csv = self.all.first().and_then(|e| e.csv.clone());
        let csv_dir = get_directory_name(first_csv.as_deref().unwrap_or(&self.default_csv))
            .unwrap_or_else(|| self.root.to_string_lossy().into_owned());
        let e = CharacterEntry {
            name: Some(name.to_string()),
            dir: Some(path_combine("characters", &safe)),
            csv: Some(path_combine(&csv_dir, &format!("{}-stats.csv", safe.to_lowercase()))),
            auto_picture: None,
        };
        self.all.push(e);
        self.save()?;
        let last = self.all.len() - 1;
        Ok(&mut self.all[last])
    }

    /// The data folder of a character ("." or empty = the data folder itself).
    pub fn data_dir(&self, e: &CharacterEntry) -> PathBuf {
        match e.dir.as_deref() {
            None | Some("") | Some(".") => self.root.clone(),
            Some(d) => self.root.join(d),
        }
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let list = Value::Array(self.all.iter().map(CharacterEntry::to_json).collect());
        json::write_atomic(self.file_path(), json::serialize(&list, Escape::Default, true))?;
        Ok(())
    }

    /// The data folder (`dataRoot`).
    pub fn root(&self) -> &Path {
        &self.root
    }
}

// ---------- .NET Path helpers on strings (Windows rules) ----------

/// `Path.GetInvalidFileNameChars().Contains(c)` on Windows.
pub fn is_invalid_file_name_char(c: char) -> bool {
    (c as u32) < 32 || matches!(c, '"' | '<' | '>' | '|' | ':' | '*' | '?' | '\\' | '/')
}

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

/// `PathInternal.GetRootLength` (in chars) for ordinary paths: `C:\`, `C:`, `\\server\share\`, `\`.
fn root_length(p: &[char]) -> usize {
    let n = p.len();
    if n >= 1 && is_sep(p[0]) {
        if n >= 2 && is_sep(p[1]) {
            let mut i = 2;
            while i < n && !is_sep(p[i]) {
                i += 1;
            }
            if i < n {
                i += 1;
            }
            while i < n && !is_sep(p[i]) {
                i += 1;
            }
            if i < n {
                i += 1;
            }
            return i;
        }
        return 1;
    }
    if n >= 2 && p[1] == ':' {
        return if n >= 3 && is_sep(p[2]) { 3 } else { 2 };
    }
    0
}

/// `PathInternal.NormalizeDirectorySeparators`: `/` -> `\`, runs of separators collapsed (a leading `\\` kept).
fn normalize_separators(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut prev_sep = false;
    for (i, &c) in chars.iter().enumerate() {
        if is_sep(c) {
            if prev_sep && !(i == 1 && chars.len() > 2) {
                continue;
            }
            out.push('\\');
            prev_sep = true;
        } else {
            out.push(c);
            prev_sep = false;
        }
    }
    out
}

/// `Path.GetDirectoryName(string)` (Windows): `None` for null/empty paths and roots; "" for a bare file name.
pub fn get_directory_name(path: &str) -> Option<String> {
    let p: Vec<char> = path.chars().collect();
    if p.is_empty() || p.iter().all(|c| c.is_whitespace()) {
        return None;
    }
    let root = root_length(&p);
    let mut end = p.len();
    if end <= root {
        return None;
    }
    while end > root {
        end -= 1;
        if is_sep(p[end]) {
            break;
        }
    }
    while end > root && is_sep(p[end - 1]) {
        end -= 1;
    }
    Some(normalize_separators(&p[..end].iter().collect::<String>()))
}

/// `Path.Combine(a, b)` on strings (Windows).
pub fn path_combine(a: &str, b: &str) -> String {
    if b.is_empty() {
        return a.to_string();
    }
    if a.is_empty() {
        return b.to_string();
    }
    let bc: Vec<char> = b.chars().collect();
    let rooted = is_sep(bc[0]) || (bc.len() >= 2 && bc[1] == ':');
    if rooted {
        return b.to_string();
    }
    let last = a.chars().last().unwrap_or(' ');
    if is_sep(last) { format!("{a}{b}") } else { format!("{a}\\{b}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotnet_path_rules() {
        assert_eq!(get_directory_name(r"C:\a\b\c.csv").as_deref(), Some(r"C:\a\b"));
        assert_eq!(get_directory_name("C:/a/b/c.csv").as_deref(), Some(r"C:\a\b"));
        assert_eq!(get_directory_name(r"C:\c.csv").as_deref(), Some(r"C:\"));
        assert_eq!(get_directory_name("c.csv").as_deref(), Some(""));
        assert_eq!(get_directory_name(r"C:\"), None);
        assert_eq!(get_directory_name(""), None);
        assert_eq!(path_combine(r"C:\a", "b.csv"), r"C:\a\b.csv");
        assert_eq!(path_combine(r"C:\a\", "b.csv"), r"C:\a\b.csv");
        assert_eq!(path_combine("a", r"D:\x"), r"D:\x");
        assert_eq!(path_combine("", "x"), "x");
    }

    #[test]
    fn reads_and_writes_like_csharp() {
        let dir = std::env::temp_dir().join(format!("imagine-chars-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cache")).unwrap();
        // data from before the list: becomes the first entry, where it is
        let mut c = Characters::new(&dir, "Rubito", r"C:\stats\rubito-stats.csv").unwrap();
        assert_eq!(c.all.len(), 1);
        assert_eq!(c.all[0].dir.as_deref(), Some("."));
        assert_eq!(c.data_dir(&c.all[0]), dir);
        let e = c.get_or_add("Roku.juu: ねこ").unwrap().clone();
        assert_eq!(e.dir.as_deref(), Some(r"characters\Roku_juu_ ねこ"));
        assert_eq!(e.csv.as_deref(), Some(r"C:\stats\roku_juu_ ねこ-stats.csv"));
        assert!(c.find("ROKU.JUU: ねこ").is_some());
        let text = std::fs::read_to_string(dir.join("characters.json")).unwrap();
        assert!(text.starts_with("[\r\n  {\r\n    \"Name\": \"Rubito\",\r\n    \"Dir\": \".\",\r\n    \"Csv\": \"C:\\\\stats\\\\rubito-stats.csv\",\r\n    \"AutoPicture\": null\r\n  },"));
        assert!(text.contains("\\u306D\\u3053"));
        // a second load reads it back
        let again = Characters::new(&dir, "x", "y").unwrap();
        assert_eq!(again.all, c.all);
        // blank names are dropped, unknown properties ignored, case matters for property names
        std::fs::write(dir.join("characters.json"), r#"[{"Name":"A","Dir":null,"extra":1,"csv":"x"},{"Name":" "},null,{"Name":"B","AutoPicture":false}]"#).unwrap();
        let c3 = Characters::new(&dir, "x", "y").unwrap();
        assert_eq!(c3.all.len(), 2);
        assert_eq!(c3.all[0].csv, None);
        assert_eq!(c3.all[1].auto_picture, Some(false));
        // a broken file gives an empty list
        std::fs::write(dir.join("characters.json"), "[{\"Name\": 5}]").unwrap();
        assert!(Characters::new(&dir, "x", "y").unwrap().all.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
