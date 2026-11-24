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
        let first_csv = self.all.first().and_then(|e| e.csv.clone());
        let csv_dir = get_directory_name(first_csv.as_deref().unwrap_or(&self.default_csv))
            .unwrap_or_else(|| self.root.to_string_lossy().into_owned());
        // The folder and file names come from the name once, here; from then on the entry's stored Dir and Csv are used,
        // so the real name never has to be turned into a path again. Two names that turn into the same file name
        // ("a:b" and "a_b") get a number after it, so they never share a folder.
        let base = safe_file_name(name);
        let mut safe = base.clone();
        let mut n = 1;
        loop {
            let dir = path_combine("characters", &safe);
            let csv = path_combine(&csv_dir, &format!("{}-stats.csv", safe.to_lowercase()));
            let taken = self.all.iter().any(|e| {
                e.dir.as_deref().is_some_and(|d| crate::text::eq_ic(&normalize_separators(d), &dir))
                    || e.csv.as_deref().is_some_and(|c| crate::text::eq_ic(&normalize_separators(c), &csv))
            });
            if !taken {
                break;
            }
            n += 1;
            safe = format!("{base} ({n})");
        }
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

/// A character name as a single file or folder name that Windows keeps exactly as written. Letters, digits, spaces and
/// symbols stay; the characters Windows forbids in a file name (`\ / : * ? " < > |` and control characters) and every
/// full stop become `_`, which was always so (a name ending in "." would otherwise lose it on disk: "Chidori Y." is
/// stored as "Chidori Y" and a later lookup of the exact name misses). New: a trailing space also becomes `_` (Windows
/// drops it too), a name Windows reserves for a device (CON, PRN, AUX, NUL, COM1-9, LPT1-9) gets a `_` after it, and an
/// empty result is `_`. A name with none of those is unchanged, so every existing folder keeps its name.
pub fn safe_file_name(name: &str) -> String {
    let mut s: String = name.chars().map(|c| if is_invalid_file_name_char(c) || c == '.' { '_' } else { c }).collect();
    let kept = s.trim_end_matches(' ').len();
    s.replace_range(kept.., &"_".repeat(s.len() - kept));
    if s.is_empty() {
        return "_".to_string();
    }
    let up = s.to_uppercase();
    let reserved = matches!(up.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (up.len() == 4 && (up.starts_with("COM") || up.starts_with("LPT")) && matches!(up.as_bytes()[3], b'1'..=b'9'));
    if reserved { format!("{s}_") } else { s }
}

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
    fn file_names_from_character_names() {
        // names that were fine before keep their folder
        for n in ["Rubito", "Roku_juu", "Accsquad5", "A B", "Chidori Y", " Lead", "O'Neil", "x,y;z", "Ünïcode", "\u{540D}\u{524D}"] {
            assert_eq!(safe_file_name(n), n, "{n}");
        }
        // forbidden characters and every full stop become _ (as before)
        assert_eq!(safe_file_name("Chidori Y."), "Chidori Y_");
        assert_eq!(safe_file_name("Name."), "Name_");
        assert_eq!(safe_file_name("a:b"), "a_b");
        assert_eq!(safe_file_name(r#"a\b/c*d?e"f<g>h|i"#), "a_b_c_d_e_f_g_h_i");
        // a trailing space is dropped by Windows: it becomes _ too; empty is _
        assert_eq!(safe_file_name("Name "), "Name_");
        assert_eq!(safe_file_name("Name  "), "Name__");
        assert_eq!(safe_file_name(""), "_");
        // device names
        for n in ["CON", "con", "Nul", "COM1", "lpt9"] {
            assert_eq!(safe_file_name(n), format!("{n}_"));
        }
        assert_eq!(safe_file_name("COM0"), "COM0");
        assert_eq!(safe_file_name("Console"), "Console");
    }

    /// Unusual names through the whole path: made into an entry, saved, read back, found again, and their folder really
    /// is created and found on disk under the stored name.
    #[test]
    fn unusual_names_get_their_own_folders() {
        let dir = std::env::temp_dir().join(format!("imagine-oddnames-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cache")).unwrap();
        let mut c = Characters::new(&dir, "Rubito", &dir.join("stats").join("rubito-stats.csv").to_string_lossy()).unwrap();
        let names = ["Chidori Y.", "A B", "Name.", "Name ", "\u{DC}n\u{EF}code", "\u{540D}\u{524D}", "a:b", "a_b", "Con", "x.y", "x_y", "a b."];
        for n in names {
            c.get_or_add(n).unwrap();
        }
        let mut seen = std::collections::HashSet::new();
        for n in names {
            let e = c.find(n).unwrap_or_else(|| panic!("{n:?} not found")).clone();
            assert_eq!(e.name.as_deref(), Some(n));
            let d = c.data_dir(&e);
            std::fs::create_dir_all(d.join("cache")).unwrap();
            assert!(d.join("cache").is_dir(), "{n:?}: folder {d:?}");
            // the stored folder name is exactly what is on disk (no trailing dot or space for Windows to strip)
            let last = d.file_name().unwrap().to_string_lossy().into_owned();
            assert!(!last.ends_with('.') && !last.ends_with(' '), "{last:?}");
            assert!(std::fs::read_dir(d.parent().unwrap()).unwrap().any(|f| f.unwrap().file_name().to_string_lossy() == last), "{n:?}");
            assert!(seen.insert(last.to_lowercase()), "{n:?} shares a folder");
            assert!(seen.insert(e.csv.clone().unwrap().to_lowercase()), "{n:?} shares a spreadsheet");
        }
        assert_eq!(c.find("chidori y.").unwrap().dir.as_deref(), Some(r"characters\Chidori Y_"));
        assert_eq!(c.find("a_b").unwrap().dir.as_deref(), Some(r"characters\a_b (2)"));
        assert_eq!(c.find("x_y").unwrap().dir.as_deref(), Some(r"characters\x_y (2)"));
        // saved and read back: the real names come back, with the same folders
        let again = Characters::new(&dir, "x", "y").unwrap();
        assert_eq!(again.all, c.all);
        // asking again for a known name adds nothing
        let n = c.all.len();
        c.get_or_add("CHIDORI Y.").unwrap();
        assert_eq!(c.all.len(), n);
        let _ = std::fs::remove_dir_all(&dir);
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
