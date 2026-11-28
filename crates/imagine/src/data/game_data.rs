//! Port of `src/Data/GameData.cs`.
//!
//! Reference data from the local server/website repos: item names, tokusei (effect) numbers, expertise tables.
//! Tokusei numbers live only on the server, so the client's set table gives effect ids and these files give their values.
//!
//! Differences from C#, all forced by the language:
//! - Every lazily loaded table is a `OnceLock` (the app calls in from worker threads); a loader that fails (a missing website
//!   file) returns the error and is tried again on the next call, like C#'s `_x ??= Load()`. So accessors that can fail return
//!   [`Result`].
//! - `reference\*.tsv` are compiled into the library (rust-embed). [`reference_file`] still looks on disk first, the way
//!   `GameData.ReferenceFile` did (up from the exe, then the repo root), so edits in a checkout apply; the embedded copy is the
//!   fallback that lets the exe work anywhere.
//! - A C# `string` that can be `null` is `Option<String>` where the code tells the two apart.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use fancy_regex::Regex as FancyRegex;
use indexmap::IndexMap;
use regex::Regex;
use rust_embed::RustEmbed;
use serde_json::{Map, Value, json};

use super::config;
use super::demon_tables::DemonTables;
use super::json::{self, At};
use crate::Result;

// ---------- reference\*.tsv: on disk first, else the copy compiled into the library ----------

#[derive(RustEmbed)]
#[folder = "../../reference"]
#[compression = "zstd"]
#[include = "*.tsv"]
struct EmbeddedReference;

/// reference\bundled\: copies of the server's data (tokusei XML, constants.xml) and of the website files no game table
/// covers yet, made by tools\refresh-bundled-data.ps1. The folders set in data\config.json win when they exist; these
/// copies are what a player with only the exe gets (without them a snapshot found 88 bonuses instead of 140).
#[derive(RustEmbed)]
#[folder = "../../reference/bundled"]
#[compression = "zstd"]
#[include = "*.xml"]
#[include = "*.json"]
struct EmbeddedBundled;

/// A file of reference\bundled\ by its path there ("tokusei/tokusei_00010000.xml"); `None` if it is not bundled.
pub fn bundled_text(rel: &str) -> Option<String> {
    EmbeddedBundled::get(rel).map(|f| json::decode_text(&f.data))
}

/// The server's constants.xml: the serverConstants file when it exists, else the bundled copy.
pub fn server_constants_text() -> Result<Option<String>> {
    if let Some(path) = config::current().server_constants.as_deref() {
        if Path::new(path).is_file() {
            return Ok(Some(json::read_all_text(Path::new(path))?));
        }
    }
    Ok(bundled_text("constants.xml"))
}

/// A file of the reference folder: found on disk (the repo's `reference\`, next to the program or up from it), or the copy
/// compiled into the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceFile {
    Disk(PathBuf),
    Embedded(String),
}

impl ReferenceFile {
    /// The file's text (`File.ReadAllText`: BOM dropped).
    pub fn read_to_string(&self) -> Result<String> {
        match self {
            ReferenceFile::Disk(p) => json::read_all_text(p),
            ReferenceFile::Embedded(name) => {
                let f = EmbeddedReference::get(name).ok_or_else(|| anyhow::anyhow!("reference\\{name} is not embedded"))?;
                Ok(json::decode_text(&f.data))
            }
        }
    }

    /// `File.ReadLines`: every line of the file.
    pub fn lines(&self) -> Result<Vec<String>> {
        Ok(json::lines(&self.read_to_string()?).into_iter().map(str::to_string).collect())
    }

    /// The file's path when it is a real file on disk (a regenerating tool needs one to write beside it).
    pub fn disk_path(&self) -> Option<&Path> {
        match self {
            ReferenceFile::Disk(p) => Some(p),
            ReferenceFile::Embedded(_) => None,
        }
    }
}

/// `GameData.ReferenceFile`: a file in the repo's reference folder, found next to the program or up from it, else the copy
/// built into the library; `None` if neither has it.
pub fn reference_file(name: &str) -> Option<ReferenceFile> {
    if let Some(p) = reference_file_on_disk(name) {
        return Some(ReferenceFile::Disk(p));
    }
    EmbeddedReference::get(name).map(|_| ReferenceFile::Embedded(name.to_string()))
}

/// The disk half of [`reference_file`] (exactly C# `GameData.ReferenceFile`).
pub fn reference_file_on_disk(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for d in dir.ancestors() {
                let f = d.join("reference").join(name);
                if f.is_file() {
                    return Some(f);
                }
            }
        }
    }
    // Tools run from elsewhere: the repo found from the current folder (Config.RepoRoot)
    let g = config::repo_root()?.join("reference").join(name);
    g.is_file().then_some(g)
}

/// BABEL's English for names the game has only in Japanese (or garbled, like "Izar ﾎｱ" for Izar α): a reference\<file>
/// of "id <tab> the game's name <tab> English" lines (demon-names-en.tsv, growth-names-en.tsv). id -> (the game's name,
/// English). Use a line through [`translated_name`], which takes it only while the game still has that exact name, so a
/// game update that renames or translates it wins. Pages show such a name greyed out, with the game's own in its tooltip.
pub fn name_translations(file: &str) -> Result<HashMap<i32, (String, String)>> {
    let mut res = HashMap::new();
    for line in reference_lines(file)?.unwrap_or_default() {
        let r: Vec<&str> = line.split('\t').collect();
        if line.starts_with('#') || r.len() < 3 {
            continue;
        }
        if let Some(id) = crate::fmt::parse_i32(r[0]) {
            if !r[2].trim().is_empty() {
                res.insert(id, (r[1].trim().to_string(), r[2].trim().to_string()));
            }
        }
    }
    Ok(res)
}

/// The English for the game's `name` of `id` when `map` ([`name_translations`]) has one for exactly that name.
pub fn translated_name(map: &HashMap<i32, (String, String)>, id: i32, name: &str) -> Option<String> {
    // spaces compared loosely: the builders squash the client's runs of spaces (full width too) to one
    let loose = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    map.get(&id).filter(|(game, _)| loose(game) == loose(name)).map(|(_, en)| en.clone())
}

/// BABEL's English for the item names the client has only in Japanese (reference\item-names-en.tsv: id, the client's
/// name, the English), loaded once; empty when the file is missing.
pub fn item_translations() -> &'static HashMap<i32, (String, String)> {
    static T: LazyLock<HashMap<i32, (String, String)>> = LazyLock::new(|| name_translations("item-names-en.tsv").unwrap_or_default());
    &T
}

/// BABEL's English for the client's item name `name` of `id` (see [`item_translations`]); `None` when the name is the
/// game's own English or has no translation.
pub fn item_name_en(id: i32, name: &str) -> Option<String> {
    translated_name(item_translations(), id, name)
}

/// A description as a lookup key: each line's spaces squashed (full width too), blank lines dropped. The builders tidy the
/// client's text in different ways (the Item Database drops blank lines, skill texts keep them), so both meet here.
fn desc_key(s: &str) -> String {
    s.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

/// BABEL's English for the game's descriptions that are only (or partly) in Japanese: reference\desc-en.tsv, two columns,
/// the game's text and the English, line breaks written as \n. Keyed by the game's own text ([`desc_key`]), so a client
/// update that changes a description drops its translation instead of showing a wrong one.
pub fn desc_translations() -> &'static HashMap<String, String> {
    static T: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        let unescape = |s: &str| s.replace("\\n", "\n").replace("\\\\", "\\");
        let mut res = HashMap::new();
        for line in reference_lines("desc-en.tsv").ok().flatten().unwrap_or_default() {
            let p: Vec<&str> = line.split('\t').collect();
            if line.starts_with('#') || p.len() < 2 || p[1].trim().is_empty() {
                continue;
            }
            res.insert(desc_key(&unescape(p[0])), unescape(p[1]).trim().to_string());
        }
        res
    });
    &T
}

/// BABEL's English for a description of the game (see [`desc_translations`]); `None` when it has none.
pub fn desc_en(text: &str) -> Option<String> {
    desc_translations().get(&desc_key(text)).cloned()
}

/// BABEL's English for the status names the client has only in Japanese (reference\status-names-en.tsv: id, the
/// client's name, the English), loaded once.
pub fn status_translations() -> &'static HashMap<i32, (String, String)> {
    static T: LazyLock<HashMap<i32, (String, String)>> = LazyLock::new(|| name_translations("status-names-en.tsv").unwrap_or_default());
    &T
}

/// Whether a text has Japanese in it (kana or kanji).
pub fn has_japanese(s: &str) -> bool {
    s.chars().any(|c| matches!(c as u32, 0x3040..=0x30FA | 0x30FC..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF))
}

/// Japanese pieces inside English text with BABEL's English, longest first: every item name of item-names-en.tsv and
/// status name of status-names-en.tsv ("Immune to 至福"), and the game terms of reference\text-en.tsv when it exists.
fn text_fragments() -> &'static Vec<(String, String)> {
    static T: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
        let mut m: HashMap<String, String> = HashMap::new();
        for (ja, en) in item_translations().values().chain(status_translations().values()) {
            if ja.chars().count() >= 2 {
                m.entry(ja.trim().to_string()).or_insert_with(|| en.clone());
            }
        }
        for line in reference_lines("text-en.tsv").ok().flatten().unwrap_or_default() {
            let p: Vec<&str> = line.split('\t').collect();
            if !line.starts_with('#') && p.len() >= 2 && !p[0].is_empty() && !p[1].trim().is_empty() {
                m.insert(p[0].to_string(), p[1].trim().to_string()); // the terms win over an item of the same name
            }
        }
        let mut v: Vec<(String, String)> = m.into_iter().collect();
        v.sort_by(|a, b| b.0.chars().count().cmp(&a.0.chars().count()).then_with(|| a.0.cmp(&b.0)));
        v
    });
    &T
}

/// A text the pages show, with BABEL's English put in: the whole text when desc-en.tsv has it, else the game terms and
/// item names inside it (text-en.tsv, item-names-en.tsv). `None` when nothing changed (no Japanese, or none known).
pub fn english_text(s: &str) -> Option<String> {
    if !has_japanese(s) {
        return None;
    }
    if let Some(en) = desc_en(s) {
        return Some(en);
    }
    let mut t = s.to_string();
    for (ja, en) in text_fragments() {
        if t.contains(ja.as_str()) {
            t = t.replace(ja.as_str(), en);
            if !has_japanese(&t) {
                break;
            }
        }
    }
    (t != s).then_some(t)
}

/// [`english_text`] over every string of a page's data (object keys too), leaving the game's originals kept for the
/// tooltips alone (`ja`, `jp`, `descJa`, `growthNamesJa`). Returns how many strings it changed.
pub fn translate_json(v: &mut Value) -> usize {
    const KEEP: [&str; 4] = ["ja", "jp", "descJa", "growthNamesJa"];
    match v {
        Value::String(s) => match english_text(s) {
            Some(en) => {
                *s = en;
                1
            }
            None => 0,
        },
        Value::Array(a) => a.iter_mut().map(translate_json).sum(),
        Value::Object(o) => {
            let mut n = 0;
            let keys: Vec<String> = o.keys().cloned().collect();
            for k in keys {
                if KEEP.contains(&k.as_str()) {
                    continue;
                }
                let mut val = o.remove(&k).unwrap_or(Value::Null);
                n += translate_json(&mut val);
                let k2 = english_text(&k).inspect(|_| n += 1).unwrap_or(k);
                o.insert(k2, val);
            }
            n
        }
        _ => 0,
    }
}

/// The client's item table (reference\items-game.tsv) as lines, with BABEL's English in the name column where the
/// client's name is Japanese: for builders that only need a readable name (barter costs, exchange items).
pub fn items_game_lines_en() -> Result<Vec<String>> {
    Ok(reference_lines("items-game.tsv")?
        .unwrap_or_default()
        .into_iter()
        .map(|line| {
            let mut p: Vec<String> = line.split('\t').map(str::to_string).collect();
            if let (Some(id), Some(n)) = (p.first().and_then(|s| crate::fmt::parse_i32(s)), p.get(1)) {
                if let Some(en) = item_name_en(id, n) {
                    p[1] = en;
                }
            }
            p.join("\t")
        })
        .collect())
}

/// The lines of a reference file; `None` when it is missing.
pub fn reference_lines(name: &str) -> Result<Option<Vec<String>>> {
    match reference_file(name) {
        Some(f) => Ok(Some(f.lines()?)),
        None => Ok(None),
    }
}

/// The client's decoded enchant table (reference\enchants-game.tsv), found next to the program or up from it.
pub fn game_enchant_table() -> Option<ReferenceFile> {
    reference_file("enchants-game.tsv")
}

/// The game's name for an item class (reference\item-classes.tsv, the client's CIconData_ItemClass): 23 "Martial-Arts",
/// 24 "One-Handed Sword", 25 "Stabbing Weapon", 26 "Primitive Weapon"... It is the number an item has as its sub-category,
/// so it is also what the server's EQUIPPED_WEAPON_TYPE tokusei condition compares (TokuseiManager: the equipped weapon's
/// sub-category equals the condition's Value). `None` when the client has no name for it.
pub fn item_class_name(id: i32) -> Option<&'static str> {
    static NAMES: OnceLock<HashMap<i32, String>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            let mut m = HashMap::new();
            for line in reference_lines("item-classes.tsv").ok().flatten().unwrap_or_default() {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                if let (Some(id), Some(name)) = (r.first().and_then(|s| crate::fmt::parse_i32(s)), r.get(1).filter(|n| !n.trim().is_empty())) {
                    m.insert(id, name.trim().to_string());
                }
            }
            m
        })
        .get(&id)
        .map(String::as_str)
}

/// The game's name for a status effect (reference\status-names.tsv, the client's CStatusData): 10 "Stun", 32 "Instant
/// Death", 844 "Panic"... It is the number the server's STATUS_NULL and STATUS_INFLICT_ADJUST tokusei have as their
/// Value when it is positive (SkillManager: "nullified by direct ID"). `None` when the client has no name for it.
pub fn status_name(id: i32) -> Option<&'static str> {
    static NAMES: OnceLock<HashMap<i32, String>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            let mut m = HashMap::new();
            for line in reference_lines("status-names.tsv").ok().flatten().unwrap_or_default() {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                if let (Some(id), Some(name)) = (r.first().and_then(|s| crate::fmt::parse_i32(s)), r.get(1).filter(|n| !n.trim().is_empty())) {
                    // BABEL's English where the client names the status only in Japanese (status-names-en.tsv: 至福 Bliss)
                    m.insert(id, translated_name(status_translations(), id, name).unwrap_or_else(|| name.trim().to_string()));
                }
            }
            m
        })
        .get(&id)
        .map(String::as_str)
}

/// What a STATUS_NULL / STATUS_INFLICT_ADJUST tokusei's Value names, in words (never the number): a status by the
/// game's name; -1 the status ailments as a whole (the server reads a negative Value as "every status of main category
/// -Value - 1", and category 0 is the ailments: Stone, Sleep, Charm, Poison...); anything else, said plainly.
pub fn status_target(value: i32) -> String {
    match value {
        -1 => "status ailments".to_string(),
        v if v > 0 => match status_name(v) {
            Some(name) => name.to_string(),
            None => "a status the game data has no name for".to_string(),
        },
        _ => "a group of statuses the game data does not name".to_string(),
    }
}

// ---------- small parsing helpers (C# int.Parse throws, int.TryParse does not) ----------

/// `int.Parse(s)`: an error where C# threw FormatException/OverflowException.
pub(crate) fn int_parse(s: &str) -> Result<i32> {
    crate::fmt::parse_i32(s).ok_or_else(|| anyhow::anyhow!("The input string '{s}' was not in a correct format."))
}

/// `double.Parse(s, InvariantCulture)`.
pub(crate) fn double_parse(s: &str) -> Result<f64> {
    crate::fmt::parse_f64(s).ok_or_else(|| anyhow::anyhow!("The input string '{s}' was not in a correct format."))
}

/// `s.Split(sep, StringSplitOptions.RemoveEmptyEntries)`.
fn split_nonempty(s: &str, sep: char) -> Vec<&str> {
    s.split(sep).filter(|x| !x.is_empty()).collect()
}

/// `row[i]`: an error where C# would throw IndexOutOfRangeException.
pub(crate) fn col<'a>(r: &[&'a str], i: usize) -> Result<&'a str> {
    r.get(i).copied().ok_or_else(|| anyhow::anyhow!("Index was outside the bounds of the array."))
}

/// `{n:D8}`.
fn d8(n: i32) -> String {
    if n < 0 { format!("-{:08}", -(n as i64)) } else { format!("{n:08}") }
}

/// `char.IsAsciiLetterOrDigit` over a string, plus the extra characters allowed.
fn is_file_name_like(a: &str, extra: &[char]) -> bool {
    a.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(&c))
}

fn once<T>(cell: &OnceLock<T>, f: impl FnOnce() -> Result<T>) -> Result<&T> {
    if let Some(v) = cell.get() {
        return Ok(v);
    }
    let v = f()?;
    let _ = cell.set(v);
    Ok(cell.get().expect("just set"))
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// XML text of an element (`XElement.Value`: all descendant text).
pub(crate) fn xml_value(n: roxmltree::Node) -> String {
    n.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect()
}

pub(crate) fn xml_children<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

/// `o.Elements("member").FirstOrDefault(m => (string)m.Attribute("name") == name)?.Value.Trim()`
pub(crate) fn member_text(o: roxmltree::Node, name: &str) -> Option<String> {
    xml_children(o, "member").find(|m| m.attribute("name") == Some(name)).map(|m| xml_value(m).trim().to_string())
}

/// `o.Elements("member").Where(name).Elements("element").Elements("object")`
pub(crate) fn member_objects<'a, 'i>(o: roxmltree::Node<'a, 'i>, name: &str) -> Vec<roxmltree::Node<'a, 'i>> {
    xml_children(o, "member")
        .filter(|m| m.attribute("name") == Some(name))
        .flat_map(|m| xml_children(m, "element"))
        .flat_map(|e| xml_children(e, "object"))
        .collect()
}

pub(crate) fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>> {
    Ok(roxmltree::Document::parse_with_options(text, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })?)
}

// ---------- record types ----------

/// One weapon mod effect at one level (reference\mods-weapon.tsv: ModifiedEffectData, CModifiedEffectData,
/// ModificationData). `slots` are the mod slots (0-4) a mod item puts it in directly.
#[derive(Clone, Debug)]
pub struct WeaponMod {
    pub id: i32,
    pub kind: i32,
    pub level: i32,
    pub tokusei: i32,
    pub name: String,
    pub slots: Vec<i32>,
}

/// One IMAGINE expertise (reference\expertise-imagine.tsv, from the client's ExpertClassData and translation.xml).
/// Chain: (source expertise, rank it needs (points / 10000), share of its points).
#[derive(Debug, Clone, PartialEq)]
pub struct ExpertiseDef {
    pub id: i32,
    pub name: String,
    pub max_class: i32,
    pub is_chain: bool,
    pub chain: Vec<(i32, i32, f64)>,
}

/// One crystal: its enchant id (what the game stores on the item) and names as shown in tooltips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enchant {
    pub id: i32,
    pub crystal: Option<String>,
    pub tarot_name: Option<String>,
    pub soul_name: Option<String>,
}

/// A fusion ("enchant") set: active when every enchant id in Effects is on a worn item, as tarot or soul
/// (CharacterState::RecalcEquipState). Conditions are extra tokusei with a requirement (type 0 = none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantSet {
    pub id: i32,
    pub effects: Vec<i32>,
    pub tokusei: Vec<i32>,
    pub conditions: Vec<(i32, Vec<i32>)>,
}

/// One correct-table entry: stat id (e.g. LNGR, RATE_LNGR, NRA_SLASH), type (0 flat, 1 %, NRA kinds), value.
#[derive(Debug, Clone, PartialEq)]
pub struct Correct {
    pub id: Option<String>,
    pub r#type: i32,
    pub value: f64,
}

/// A tarot or soul side of an enchant: its tokusei, and extra tokusei with a requirement (type 0 = none).
/// `mask`: which gear takes it: bit n = the game's equip type n (0 Head ... 13 Weapon); 0 = none, or not known (website data).
/// `crystal_item`: the crystal's item id (its name is what players call the enchant), 0 when not known.
/// Conditions are (type, p1, p2, tokusei).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantSide {
    pub name: Option<String>,
    pub tokusei: Vec<i32>,
    pub conditions: Vec<(i32, i32, i32, Vec<i32>)>,
    pub mask: i32,
    pub crystal_item: i32,
    /// The client's own description of the crystal's effect on this side (the enchant table's last column; its line
    /// breaks are ` / `), empty when not known (website data). Read when the tokusei above have no data.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokCond {
    pub r#type: Option<String>,
    pub value: i32,
    pub cmp: String,
    pub group: i32,
}

/// A correct value (Id = stat) or an aspect (Id = aspect type), with its multiplier (NONE, LEVEL, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct TokValue {
    pub id: Option<String>,
    pub r#type: i32,
    pub value: f64,
    pub mult: String,
    /// The multiplier's MultiplierValue: the divisor of the *_DIVIDE types, the CorrectTbl index of CORRECT_TABLE(_DIVIDE).
    pub mult_value: i32,
    pub precision: i32,
    pub modifier: Option<f64>,
}

impl TokValue {
    /// `new TokValue(id, type, value, mult, precision)` (no modifier).
    pub fn new(id: Option<&str>, r#type: i32, value: f64, mult: &str, precision: i32) -> TokValue {
        TokValue { id: id.map(str::to_string), r#type, value, mult: mult.to_string(), mult_value: 0, precision, modifier: None }
    }
}

/// The tokusei in the sealed data ([`super::pack`], entry [`super::pack::TOKUSEI`]): server tokusei that comp_hack's files
/// lack. Never in a folder; this name stands for them in [`GameData::tokusei_xml_text`]. Read before comp_hack's files,
/// so they win should an id ever be in both (the server refuses a duplicate id).
pub const SEALED_TOKUSEI: &str = "sealed:tokusei";

/// XML comments taken out: the server files keep old versions of tokusei commented out, which a text search over
/// the file would otherwise find (a disabled copy can come after the real one).
fn strip_xml_comments(text: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
    RE.replace_all(text, "").into_owned()
}

/// One `<object name="Tokusei">` of the server XML as a [`TokuseiDef`].
fn xml_tokusei_def(o: roxmltree::Node, tid: i32) -> Result<TokuseiDef> {
    Ok(TokuseiDef {
        id: tid,
        target: member_text(o, "TargetType").unwrap_or_else(|| "SELF".into()),
        conditions: member_objects(o, "Conditions")
            .into_iter()
            .map(|c| TokCond {
                r#type: member_text(c, "Type"),
                value: member_text(c, "Value").and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0),
                cmp: member_text(c, "Comparator").unwrap_or_else(|| "EQUALS".into()),
                group: member_text(c, "OptionGroupID").and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0),
            })
            .collect(),
        skill_conditions: member_objects(o, "SkillConditions")
            .into_iter()
            .map(|c| (member_text(c, "SkillConditionType"), member_text(c, "Value").and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0)))
            .collect(),
        target_conditions: !member_objects(o, "SkillTargetConditions").is_empty(),
        correct: member_objects(o, "CorrectValues")
            .into_iter()
            .chain(member_objects(o, "TokuseiCorrectValues"))
            .map(|c| xml_tok_value(c, "ID"))
            .collect::<Result<Vec<_>>>()?,
        aspects: member_objects(o, "Aspects").into_iter().map(|c| xml_tok_value(c, "Type")).collect::<Result<Vec<_>>>()?,
        from_client_text: false,
    })
}

/// Everything a tokusei does and needs. `from_client_text`: not in the server data: the values were read from the client's
/// display text for this tokusei (reference\tokusei-missing.tsv).
#[derive(Debug, Clone, PartialEq)]
pub struct TokuseiDef {
    pub id: i32,
    pub target: String,
    pub conditions: Vec<TokCond>,
    pub skill_conditions: Vec<(Option<String>, i32)>,
    pub target_conditions: bool,
    pub correct: Vec<TokValue>,
    pub aspects: Vec<TokValue>,
    pub from_client_text: bool,
}

/// The skill that lists a tokusei, as a linked skill condition ("tokusei N applies") points at it: the tokusei is
/// active while that skill applies, so for a switch (toggle) skill while the switch is on. See [`GameData::tokusei_owner`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokuseiOwner {
    pub skill: i32,
    /// A switch (toggle) skill: its tokusei apply only while it is on.
    pub switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTokusei {
    pub target: Option<String>,
    pub aspects: Vec<(String, i32)>,
    pub digitalized: bool,
}

struct ItemsData {
    items: IndexMap<i32, Value>,
    subcats_game: HashMap<i32, i32>,
    /// Each item's basic flags and gender restriction (`items-game.tsv` columns 8 and 9: ItemData `basic.flags`,
    /// `restriction.gender`), for the soul infusion rules (`item_spirit_fuse`).
    fuse_game: HashMap<i32, (u32, i32)>,
    /// Each item's own number of mod slots (column 10: ItemData `restriction.modSlots`).
    mod_slots_game: HashMap<i32, i32>,
}

struct RankData {
    by_id: IndexMap<i32, Vec<(i32, Vec<i32>)>>,
    client_skill_names: IndexMap<i32, String>,
    /// (expertise id, class*10+rank) -> how many skills the character may learn there, for the pick-one ranks only
    /// (the ones that list more skills than that).
    picks: HashMap<(i32, i32), i32>,
}

enum SItemSource {
    Game(HashMap<i32, Vec<i32>>),
    Website(Map<String, Value>),
}

#[derive(Default)]
struct Defs {
    defs: HashMap<i32, Option<Arc<TokuseiDef>>>,
    files: HashSet<String>,
}

static ACTION_TYPE_NAMES: [&str; 12] = ["ATTACK", "SPIN", "RUSH", "SHOT", "RAPID", "GUARD", "COUNTER", "DODGE", "TALK", "INTIMIDATE", "TAUNT", "SUPPORT"];
static ASPECT_KEYS: [&str; 9] = [
    "LIMIT_BREAK_MAX",
    "TECH_ATTACK_RATE",
    "TECH_ATTACK_POWER",
    "PURSUIT_RATE",
    "PURSUIT_POWER",
    "DIGITALIZE_LENGTH",
    "DIGITALIZE_POINT_RATE",
    "EFFECT_POWER",
    "DAMAGE_DEALT",
];

/// `GearEffects.IsPctCorrect` (the list lives in the calc module, as in C#).
fn is_pct_correct(id: &str) -> bool {
    crate::calc::gear_effects::is_pct_correct(id)
}

static RE_ENCHANT_COND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(-?\d+):(-?\d+)?,?(-?\d+)?,?>([\d,]*)$").unwrap());
static RE_CRYSTAL_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*crystal$").unwrap());
static RE_POSSESSIVE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"('s|')$").unwrap());
static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static RE_CRYSTAL_NAME_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*Crystal$").unwrap());
static RE_SKILLDATA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"SkillData:(\d+):([^;\t]+)").unwrap());

/// A client skill or item name that is English as the game shows it: Latin letters and no Japanese (kana, kanji, full-width
/// forms like "％" or "（"). The game's "・" (Secret of the Flame・Destruction) is allowed. "Frost窶冱 Punch" (108, a
/// UTF-8 apostrophe the client stores garbled) counts as not English, so the website's name is used for it.
pub fn is_english_game_name(n: &str) -> bool {
    n.chars().any(|c| c.is_ascii_alphabetic())
        && !n.chars().any(|c| {
            c != '・' && matches!(c as u32, 0x3000..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
        })
}

#[cfg(test)]
#[test]
fn english_skill_names() {
    assert!(is_english_game_name("Talk to the Fist!"));
    assert!(is_english_game_name("Secret of the Flame・Destruction"));
    assert!(is_english_game_name("If a Demon is Cursed, Dig 4 Graves"));
    assert!(!is_english_game_name("Vileの牙"));
    assert!(!is_english_game_name("四Seraphim召喚"));
    assert!(!is_english_game_name("HP23％減少（敵専用）"));
    assert!(!is_english_game_name("Frost窶冱 Punch"));
    assert!(!is_english_game_name("ヤればデキる"));
    assert!(!is_english_game_name(""));
}

/// The portable release has no website or tokusei folder: every table still loads, from the bundled copies
/// (reference\bundled\) and the client's own tables.
#[cfg(test)]
#[test]
fn loads_without_website_folder() {
    // the bundled copies are compiled in
    assert!(bundled_text("constants.xml").is_some_and(|t| t.contains("DIGITALIZE_STAT_RATES")));
    assert!(bundled_text("tokusei/tokusei_00010000.xml").is_some());
    let d = GameData::with_paths(None, None);
    assert_eq!(d.devil_alignment(1).unwrap(), Some("LAW"), "devils.json Law/Chaos points are bundled");
    assert!(!d.enchant_sets().unwrap().is_empty(), "fusion sets are bundled");
    assert!(d.tokusei_xml_text("tokusei_00010000.xml").unwrap().is_some(), "the server's tokusei XML is bundled");    for content in [None, Some(r"Z:\no\such\folder".to_string())] {
        let d = GameData::with_paths(content, None);
        assert!(d.items().is_ok());
        assert!(d.tokusei().is_ok());
        assert!(d.expertises().is_ok());
        assert!(d.rank_skills().is_ok());
        assert!(d.passives().is_ok());
        assert!(d.skill_names().is_ok());
        assert!(d.skill_icon_asset(1).is_ok());
        assert!(d.enchants().is_ok());
        assert!(d.enchant_sets().is_ok());
        assert!(d.fusion_icon_items().is_ok());
        assert!(d.enchant_sides().is_ok());
        assert!(d.item_sub_category(1).is_ok());
        assert!(d.sitem_tokusei(1).is_ok());
        assert!(d.mod_tokusei(true, 0, 0, 1).is_ok());
        assert!(d.devil_icon_file(1).is_ok());
        assert!(d.devil_alignment(1).is_ok());
        assert!(d.tokusei_full(1).is_ok());
        assert!(d.raw_tokusei_xml(1).is_ok());
        if reference_file("items-game.tsv").is_some() {
            assert!(!d.items().unwrap().is_empty(), "the client's item table fills the list");
        }
    }
}

/// The sealed tokusei come from the sealed data, with or without a tokuseiXml folder, and not from the client's text;
/// a GameData made without it (the repo's Gear Database) falls back to the client's text. No values are checked here.
/// Skipped in a build without the key.
#[cfg(test)]
#[test]
fn tokusei_from_the_sealed_data() {
    if !super::pack::available() {
        return;
    }
    let ids: Vec<i32> = {
        let text = String::from_utf8_lossy(super::pack::entry(super::pack::TOKUSEI).unwrap()).into_owned();
        let doc = parse_xml(&text).unwrap();
        doc.descendants()
            .filter(|x| x.is_element() && x.tag_name().name() == "object" && x.attribute("name") == Some("Tokusei"))
            .filter_map(|o| member_text(o, "ID").and_then(|s| crate::fmt::parse_i32(&s)))
            .collect()
    };
    assert!(ids.len() > 500, "{}", ids.len());
    for dir in [None, Some(r"Z:\no\such\folder".to_string())] {
        let d = GameData::with_paths(None, dir);
        for &id in ids.iter().step_by(37) {
            let t = d.tokusei_full(id).unwrap().unwrap_or_else(|| panic!("{id} in the sealed data"));
            assert!(!t.from_client_text, "{id}");
        }
        // comp_hack's own ids still load
        assert!(d.tokusei_full(10003).unwrap().is_some());
    }
    let public = GameData::with_paths(None, None).without_sealed_data();
    assert!(ids.iter().all(|&id| public.tokusei_full(id).unwrap().is_none_or(|t| t.from_client_text)));
}

static RE_MEMBERS_STAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^members' (strength|magic|vitality|intelligence|speed|luck) \+(\d+),?$").unwrap());
static RE_PARTY_SEG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(all party( members)?|by the power of [a-z ]+)$").unwrap());
static RE_CLIENT_TOK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:(partner|party|summoner):)?([A-Z_]+)(?:\[action=([A-Z]+)\])?=([+-]?\d+(?:\.\d+)?)(%?)$").unwrap());
static RE_RAW_TYPES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)<member name="Type">([A-Z_]+)</member>\s*<member name="Value">(-?\d+)</member>"#).unwrap());
static RE_TARGET_TYPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)<member name="TargetType">([A-Z_]+)</member>"#).unwrap());
static RE_DIGITALIZED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)DIGITALIZED").unwrap());
static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Reference data from the local server/website repos.
pub struct GameData {
    /// data\config.json "websiteContent"
    pub content: Option<String>,
    /// data\config.json "tokuseiXml"
    pub tokusei_xml_dir: Option<String>,
    /// Use the sealed data ([`super::pack`]); off for what is committed (the repo's gear-db.json)
    sealed_data: bool,

    trait_text: OnceLock<HashMap<i32, String>>,
    constants: OnceLock<HashMap<String, String>>,
    items: OnceLock<ItemsData>,
    tokusei: OnceLock<Value>,
    expertises: OnceLock<Value>,
    rank_skills: OnceLock<Value>,
    passives: OnceLock<Map<String, Value>>,
    tokusei_owner: OnceLock<HashMap<i32, TokuseiOwner>>,
    exp_client: OnceLock<IndexMap<i32, ExpertiseDef>>,
    rank_client: OnceLock<RankData>,
    skill_text: OnceLock<HashMap<i32, String>>,
    tokusei_text: OnceLock<HashMap<i32, String>>,
    skill_icons: OnceLock<HashMap<i32, Option<String>>>,
    skill_names: OnceLock<IndexMap<i32, Option<String>>>,
    skill_translations: OnceLock<HashMap<i32, (String, String)>>,
    enchants: OnceLock<IndexMap<i32, Enchant>>,
    enchant_by_crystal: OnceLock<HashMap<String, Vec<i32>>>,
    enchant_sets: OnceLock<Vec<EnchantSet>>,
    fusion_icon_items: OnceLock<(i32, i32)>,
    subcats: OnceLock<HashMap<i32, i32>>,
    sitem: OnceLock<SItemSource>,
    mods: OnceLock<Value>,
    weapon_mods: OnceLock<IndexMap<i32, WeaponMod>>,
    enchant_sides: OnceLock<IndexMap<(i32, bool), EnchantSide>>,
    devil_icon_files: OnceLock<HashMap<i32, (String, String)>>,
    devil_icons: OnceLock<Map<String, Value>>,
    devil_lnc: OnceLock<Map<String, Value>>,
    demons: OnceLock<DemonTables>,
    client_tok_text: OnceLock<HashMap<i32, String>>,
    client_tok: OnceLock<HashMap<i32, Arc<TokuseiDef>>>,
    defs: Mutex<Defs>,
    raw: Mutex<HashMap<i32, Option<Arc<RawTokusei>>>>,
    raw_types: Mutex<HashMap<i32, Option<String>>>,
    xml_files: Mutex<HashMap<String, Option<Arc<String>>>>,
}

impl Default for GameData {
    fn default() -> Self {
        GameData::new()
    }
}

impl GameData {
    /// `new GameData()`: paths from data\config.json (`Config.Current`).
    pub fn new() -> GameData {
        let c = config::current();
        GameData::with_paths(c.website_content.clone(), c.tokusei_xml.clone())
    }

    /// A GameData reading the website content and tokusei XML from these folders.
    pub fn with_paths(content: Option<String>, tokusei_xml_dir: Option<String>) -> GameData {
        GameData {
            content,
            tokusei_xml_dir,
            sealed_data: true,
            trait_text: OnceLock::new(),
            constants: OnceLock::new(),
            items: OnceLock::new(),
            tokusei: OnceLock::new(),
            expertises: OnceLock::new(),
            rank_skills: OnceLock::new(),
            passives: OnceLock::new(),
            tokusei_owner: OnceLock::new(),
            exp_client: OnceLock::new(),
            rank_client: OnceLock::new(),
            skill_text: OnceLock::new(),
            tokusei_text: OnceLock::new(),
            skill_icons: OnceLock::new(),
            skill_names: OnceLock::new(),
            skill_translations: OnceLock::new(),
            enchants: OnceLock::new(),
            enchant_by_crystal: OnceLock::new(),
            enchant_sets: OnceLock::new(),
            fusion_icon_items: OnceLock::new(),
            subcats: OnceLock::new(),
            sitem: OnceLock::new(),
            mods: OnceLock::new(),
            weapon_mods: OnceLock::new(),
            enchant_sides: OnceLock::new(),
            devil_icon_files: OnceLock::new(),
            devil_icons: OnceLock::new(),
            devil_lnc: OnceLock::new(),
            demons: OnceLock::new(),
            client_tok_text: OnceLock::new(),
            client_tok: OnceLock::new(),
            defs: Mutex::new(Defs::default()),
            raw: Mutex::new(HashMap::new()),
            raw_types: Mutex::new(HashMap::new()),
            xml_files: Mutex::new(HashMap::new()),
        }
    }

    /// The website content folder, when one is set and exists. The portable release has none (no server repo beside
    /// it). (The C# threw ArgumentNullException here, and the first port copied that, so a snapshot in the portable app
    /// failed with "Value cannot be null. (Parameter 'path1')".)
    fn content_dir(&self) -> Option<PathBuf> {
        self.content.as_deref().map(PathBuf::from).filter(|p| p.is_dir())
    }

    /// A website JSON file: from the website folder when there is one, else the bundled copy (reference\bundled\website),
    /// else `empty` (the file's shape with nothing in it): the files not bundled are only a base under the client's
    /// own tables (reference\*.tsv).
    fn load_content_json(&self, rel: &[&str], empty: Value) -> Result<Value> {
        if let Some(mut p) = self.content_dir() {
            for r in rel {
                p.push(r);
            }
            return json::load(p);
        }
        match bundled_text(&format!("website/{}", rel.join("/"))) {
            Some(text) => json::parse(&text),
            None => Ok(empty),
        }
    }

    /// The same data without the sealed data: what is committed (the repo's app\ui\gear-db.json).
    pub fn without_sealed_data(mut self) -> GameData {
        self.sealed_data = false;
        self
    }

    /// The text of one of the server's tokusei XML files ("tokusei_00010000.xml"): from the tokuseiXml folder when it has
    /// the file, else the bundled copy; `None` when neither has it. [`SEALED_TOKUSEI`] only ever comes
    /// from the sealed data (`None` without it, or for [`Self::without_sealed_data`]).
    pub fn tokusei_xml_text(&self, file: &str) -> Result<Option<String>> {
        if file == SEALED_TOKUSEI {
            return Ok(if self.sealed_data { super::pack::entry(super::pack::TOKUSEI).map(|b| String::from_utf8_lossy(b).into_owned()) } else { None });
        }
        if let Some(dir) = self.tokusei_xml_dir.as_deref().map(Path::new).filter(|d| d.is_dir()) {
            let p = dir.join(file);
            if p.is_file() {
                return Ok(Some(json::read_all_text(&p)?));
            }
        }
        Ok(bundled_text(&format!("tokusei/{file}")))
    }

    /// `GameData.ReferenceFile` (also available as the free function [`reference_file`]).
    pub fn reference_file(name: &str) -> Option<ReferenceFile> {
        reference_file(name)
    }

    /// `GameData.GameEnchantTable`.
    pub fn game_enchant_table() -> Option<ReferenceFile> {
        game_enchant_table()
    }

    /// MITAMA_SET_BOOST (constants.xml): the skill that upgrades Mitama set bonuses; 0 when unknown.
    pub fn mitama_set_boost_skill(&self) -> Result<i32> {
        Ok(self.server_constant("MITAMA_SET_BOOST")?.and_then(|v| crate::fmt::parse_i32(&v)).unwrap_or(0))
    }

    /// The game's description of a demon trait that has no effect data (reference\traits-unresolved.tsv,
    /// status no-data, e.g. the crafting assists); `None` otherwise.
    pub fn trait_description(&self, id: i32) -> Result<Option<String>> {
        let t = once(&self.trait_text, || {
            let mut m = HashMap::new();
            if let Some(lines) = reference_lines("traits-unresolved.tsv")? {
                for line in &lines {
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() > 4 && r[2] == "no-data" {
                        if let Some(tid) = crate::fmt::parse_i32(r[0]) {
                            m.insert(tid, RE_WS.replace_all(&r[4].replace("\\n", " "), " ").trim().to_string());
                        }
                    }
                }
            }
            Ok(m)
        })?;
        Ok(t.get(&id).cloned())
    }

    /// A constant from the local server's config\constants.xml (data\config.json "serverConstants"), e.g.
    /// DIGITALIZE_STAT_RATES = "10,30", MITAMA_SET_BOOST = "5133" (the bundled copy when that file is missing);
    /// `None` when the constant is missing.
    pub fn server_constant(&self, name: &str) -> Result<Option<String>> {
        let c = once(&self.constants, || {
            let mut m = HashMap::new();
            if let Some(text) = server_constants_text()? {
                let doc = parse_xml(&text)?;
                for c in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "constant") {
                    if let Some(n) = c.attribute("name") {
                        m.insert(n.to_string(), xml_value(c).trim().to_string());
                    }
                }
            }
            Ok(m)
        })?;
        Ok(c.get(name).cloned())
    }

    // ---------- items ----------

    /// Every item. The website's items.json gives the base (icons); reference\items-game.tsv, flattened from the
    /// client's current ItemData / CItemData, then overrides each item's name (when the client's is English; the
    /// website's is kept as "webName"), stats (its correct table), equip type and weapon type, and adds the items the
    /// website file does not have. The website file can lag the client by months: on
    /// 2026-11-23 it had 32 items with old stats and lacked 951 (HANDOVER, section 15).
    pub fn items(&self) -> Result<&IndexMap<i32, Value>> {
        Ok(&self.items_data()?.items)
    }

    fn items_data(&self) -> Result<&ItemsData> {
        once(&self.items, || {
            let root = self.load_content_json(&["wiki", "items.json"], json!({ "items": [] }))?;
            let mut items: IndexMap<i32, Value> = IndexMap::new();
            for i in json::arr(root.at("items")) {
                items.entry(json::int(i.at("id"))).or_insert_with(|| i.clone());
            }
            let mut subcats_game = HashMap::new();
            let mut fuse_game = HashMap::new();
            let mut mod_slots_game = HashMap::new();
            let Some(lines) = reference_lines("items-game.tsv")? else {
                return Ok(ItemsData { items, subcats_game, fuse_game, mod_slots_game });
            };
            for line in &lines {
                if line.starts_with('#') {
                    continue;
                }
                let p: Vec<&str> = line.split('\t').collect();
                if p.len() < 7 {
                    continue;
                }
                let Some(id) = crate::fmt::parse_i32(p[0]) else { continue };
                let mut correct = Vec::new();
                for c in split_nonempty(p[6], ';') {
                    let q: Vec<&str> = c.split(':').collect();
                    if q.len() == 3 {
                        if let Some(v) = crate::fmt::parse_f64(q[2]) {
                            correct.push(json!({ "id": q[0], "type": crate::fmt::parse_i32(q[1]).unwrap_or(0), "value": v }));
                        }
                    }
                }
                if !items.contains_key(&id) {
                    if p[1].is_empty() {
                        continue; // no name in the client: not an item a player has
                    }
                    items.insert(id, json!({ "id": id, "name": p[1], "icon": crate::fmt::parse_i32(p[2]).unwrap_or(0) }));
                }
                if let Some(node) = items.get_mut(&id).and_then(Value::as_object_mut) {
                    // the client's name wins when it is an English one (the game shows it; 1,015 website names differ,
                    // e.g. 38194 "Memory of a Pie Fight" is "Pie Throw Memory" there, 785 "Value Pack: Silver" is
                    // "Random Box: Candy"). The website's name stays as "webName": CSV rows saved before the item ids
                    // were written name their gear that way, and recalc_csv finds the ids from those names.
                    let game = p[1].trim();
                    let web = node.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                    if is_english_game_name(game) && web != game {
                        if !web.is_empty() {
                            node.insert("webName".into(), json!(web));
                        }
                        node.insert("name".into(), json!(game));
                    } else if let Some(en) = item_name_en(id, game) {
                        // only Japanese in the client: BABEL's English (item-names-en.tsv) wins over the website's name,
                        // which stays as webName for recalc_csv; `ja` keeps the client's name for the pages
                        if !web.is_empty() && web != en {
                            node.insert("webName".into(), json!(web));
                        }
                        node.insert("name".into(), json!(en));
                        node.insert("ja".into(), json!(game));
                    }
                    node.insert("basicFeatures".into(), Value::Array(correct)); // the whole correct table; ItemCorrect reads both lists
                    node.insert("characteristics".into(), Value::Array(Vec::new()));
                    node.insert("equipType".into(), json!(p[3]));
                    node.insert("weaponType".into(), if p[4].is_empty() || p[4] == "NONE" { Value::Null } else { json!(p[4]) });
                }
                if let Some(sc) = crate::fmt::parse_i32(p[5]) {
                    subcats_game.insert(id, sc);
                }
                if let (Some(fl), Some(g)) = (p.get(7).and_then(|s| s.trim().parse::<u32>().ok()), p.get(8).and_then(|s| crate::fmt::parse_i32(s.trim()))) {
                    fuse_game.insert(id, (fl, g));
                }
                if let Some(n) = p.get(9).and_then(|s| crate::fmt::parse_i32(s.trim())) {
                    mod_slots_game.insert(id, n);
                }
            }
            Ok(ItemsData { items, subcats_game, fuse_game, mod_slots_game })
        })
    }

    /// Soul infusion (the game's Spirit Fusion) as the server allows it (comp_hack `EquipmentSpiritFuse`): None when the
    /// item list has no flags for the item (an older items-game.tsv), else whether the item may take part at all (not
    /// Bullets, no "no fusion" flag 0x0800; every piece without durability has that flag) and its gender restriction
    /// (2 = anyone). The other rules (same equip type and sub-category, armor of one sex) are the caller's.
    pub fn item_spirit_fuse(&self, id: i32) -> Result<Option<(bool, i32)>> {
        const ITEM_FLAG_FUSE: u32 = 0x0800;
        let data = self.items_data()?;
        let Some(&(flags, gender)) = data.fuse_game.get(&id) else { return Ok(None) };
        let bullets = data.items.get(&id).and_then(|n| json::str(n.at("equipType"))).as_deref() == Some("EQUIP_TYPE_BULLETS");
        Ok(Some((!bullets && flags & ITEM_FLAG_FUSE == 0, gender)))
    }

    /// An item's own number of mod slots (0-5), None when the item list doesn't say (an older items-game.tsv). Only
    /// weapons have any in NewMoon's data; the game can add slots up to 5 (comp_hack `EquipmentModEdit`, add slot).
    pub fn item_mod_slots(&self, id: i32) -> Result<Option<i32>> {
        Ok(self.items_data()?.mod_slots_game.get(&id).copied())
    }

    /// Every weapon mod effect by id (reference\mods-weapon.tsv), empty when the file is missing.
    pub fn weapon_mods(&self) -> Result<&IndexMap<i32, WeaponMod>> {
        once(&self.weapon_mods, || {
            let mut res = IndexMap::new();
            if let Some(lines) = reference_lines("mods-weapon.tsv")? {
                for line in &lines {
                    if line.starts_with('#') {
                        continue;
                    }
                    let p: Vec<&str> = line.split('\t').collect();
                    if p.len() < 5 {
                        continue;
                    }
                    let Some(id) = crate::fmt::parse_i32(p[0]) else { continue };
                    let n = |i: usize| p.get(i).and_then(|s| crate::fmt::parse_i32(s.trim())).unwrap_or(0);
                    let slots = p.get(6).map(|s| split_nonempty(s, ',').into_iter().filter_map(crate::fmt::parse_i32).collect()).unwrap_or_default();
                    res.insert(id, WeaponMod { id, kind: n(1), level: n(2), tokusei: n(3), name: p[4].trim().to_string(), slots });
                }
            }
            Ok(res)
        })
    }

    pub fn item_name(&self, id: i32) -> Result<Option<String>> {
        Ok(self.items()?.get(&id).and_then(|n| json::str(n.at("name"))))
    }

    /// An item's icon number (the key into the client's CIconData_Item) and the website's name for it (used when the client's table lacks the key).
    pub fn item_icon(&self, id: i32) -> Result<(i32, Option<String>)> {
        Ok(match self.items()?.get(&id) {
            Some(n) => (json::int(n.at("icon")), json::str(n.at("iconAsset"))),
            None => (0, None),
        })
    }

    /// The website's tokusei-effects.json "tokusei" object.
    pub fn tokusei(&self) -> Result<&Value> {
        once(&self.tokusei, || {
            let root = self.load_content_json(&["armory", "tokusei-effects.json"], json!({ "tokusei": {} }))?;
            take_object(root, "tokusei")
        })
    }

    pub fn expertises(&self) -> Result<&Value> {
        once(&self.expertises, || {
            let root = self.load_content_json(&["armory", "expertises.json"], json!({ "expertises": {} }))?;
            take_object(root, "expertises")
        })
    }

    pub fn rank_skills(&self) -> Result<&Value> {
        once(&self.rank_skills, || {
            let root = self.load_content_json(&["armory", "expertise-rank-skills.json"], json!({ "expertises": {} }))?;
            take_object(root, "expertises")
        })
    }

    /// Passive and switch skills' effects, id -> {id, mainCategory, correctTbl [{id,type,value}], tokuseiIds}. From the
    /// server's own SkillData (reference\skills-game.tsv, IMAGINE skills included); the website's passive-skills.json
    /// only fills skills that table doesn't have (it lags IMAGINE and differs for some, e.g. Bullet Friend).
    pub fn passives(&self) -> Result<&Map<String, Value>> {
        once(&self.passives, || {
            let mut res = Map::new();
            if let Some(lines) = reference_lines("skills-game.tsv")? {
                for line in &lines {
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let r: Vec<&str> = line.split('\t').collect();
                    let mut ct = Vec::new();
                    for e in split_nonempty(if r.len() > 2 { r[2] } else { "" }, ';') {
                        let p: Vec<&str> = e.split(':').collect();
                        if p.len() == 3 {
                            ct.push(json!({ "id": p[0], "type": int_parse(p[1])?, "value": double_parse(p[2])? }));
                        }
                    }
                    let mut tok = Vec::new();
                    for t in split_nonempty(if r.len() > 3 { r[3] } else { "" }, ',') {
                        tok.push(json!(int_parse(t)?));
                    }
                    let id = int_parse(col(&r, 0)?)?;
                    let main = int_parse(col(&r, 1)?)?;
                    res.insert(r[0].to_string(), json!({ "id": id, "mainCategory": main, "correctTbl": ct, "tokuseiIds": tok, "source": "server" }));
                }
            }
            let root = self.load_content_json(&["armory", "passive-skills.json"], json!({ "skills": {} }))?;
            for (k, v) in json::as_object(root.at("skills"))? {
                if !res.contains_key(k) {
                    res.insert(k.clone(), v.clone());
                }
            }
            Ok(res)
        })
    }

    /// The skill whose tokusei list holds `tid` (reference\skills-game.tsv): what a linked skill condition on `tid`
    /// ([`super::stat_labels::linked_condition`]) depends on. When several skills list it, a switch skill wins (its state
    /// decides), then the lowest skill id. None: no skill lists it, so what makes it apply isn't known.
    pub fn tokusei_owner(&self, tid: i32) -> Result<Option<TokuseiOwner>> {
        let map = once(&self.tokusei_owner, || {
            let mut res: HashMap<i32, TokuseiOwner> = HashMap::new();
            for (key, v) in self.passives()? {
                let Some(skill) = crate::fmt::parse_i32(key) else { continue };
                let switch = json::int(v.at("mainCategory")) == 2;
                for t in json::arr(v.at("tokuseiIds")) {
                    let t = json::int(Some(t));
                    if t == 0 {
                        continue;
                    }
                    let better = match res.get(&t) {
                        None => true,
                        Some(old) => (switch, std::cmp::Reverse(skill)) > (old.switch, std::cmp::Reverse(old.skill)),
                    };
                    if better {
                        res.insert(t, TokuseiOwner { skill, switch });
                    }
                }
            }
            Ok(res)
        })?;
        Ok(map.get(&tid).cloned())
    }

    /// The switch (toggle) skill the game calls `name` (a description says "When Radiant Hands is active"); None when no
    /// switch skill has that name, or two do.
    pub fn switch_skill_named(&self, name: &str) -> Result<Option<i32>> {
        let names = self.skill_names()?;
        let mut found: Option<i32> = None;
        for (key, v) in self.passives()? {
            if json::int(v.at("mainCategory")) != 2 {
                continue;
            }
            let Some(id) = crate::fmt::parse_i32(key) else { continue };
            if names.get(&id).cloned().flatten().is_some_and(|n| crate::text::eq_ic(&n, name.trim())) {
                if found.is_some_and(|f| f != id) {
                    return Ok(None);
                }
                found = Some(id);
            }
        }
        Ok(found)
    }

    /// IMAGINE's expertise names and chain formulas; empty when the reference file is missing.
    pub fn expertises_client(&self) -> Result<&IndexMap<i32, ExpertiseDef>> {
        once(&self.exp_client, || {
            let mut res = IndexMap::new();
            let Some(lines) = reference_lines("expertise-imagine.tsv")? else { return Ok(res) };
            for line in &lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                let mut chain = Vec::new();
                for c in split_nonempty(if r.len() > 5 { r[5] } else { "" }, ';') {
                    let p: Vec<&str> = c.split(':').collect();
                    if p.len() == 3 {
                        chain.push((int_parse(p[0])?, int_parse(p[1])?, double_parse(p[2])?));
                    }
                }
                let id = int_parse(col(&r, 0)?)?;
                res.insert(
                    id,
                    ExpertiseDef {
                        id,
                        name: col(&r, 1)?.to_string(),
                        max_class: crate::fmt::parse_i32(col(&r, 3)?).unwrap_or(0),
                        is_chain: col(&r, 4)? == "true",
                        chain,
                    },
                );
            }
            Ok(res)
        })
    }

    /// Skills each expertise rank unlocks (reference\expertise-rank-skills.tsv, client ExpertClassData): id -> (class*10+rank, skills).
    pub fn rank_skills_client(&self) -> Result<&IndexMap<i32, Vec<(i32, Vec<i32>)>>> {
        Ok(&self.rank_data()?.by_id)
    }

    /// How many of a rank's skills the character may learn, when the rank offers a choice (more skills listed than that):
    /// the server's LearnSkill refuses a skill once that many of the rank's skills are learned, so the character holds
    /// only its picks. None = every skill listed is learned (or the game data doesn't say).
    pub fn rank_pick_count(&self, expertise: i32, rank_idx: i32) -> Result<Option<i32>> {
        Ok(self.rank_data()?.picks.get(&(expertise, rank_idx)).copied())
    }

    fn rank_data(&self) -> Result<&RankData> {
        once(&self.rank_client, || {
            let mut by_id: IndexMap<i32, Vec<(i32, Vec<i32>)>> = IndexMap::new();
            let mut client_skill_names: IndexMap<i32, String> = IndexMap::new();
            let mut picks: HashMap<(i32, i32), i32> = HashMap::new();
            let Some(lines) = reference_lines("expertise-rank-skills.tsv")? else {
                return Ok(RankData { by_id, client_skill_names, picks });
            };
            for line in &lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                let ids = split_nonempty(col(&r, 3)?, ',').into_iter().map(int_parse).collect::<Result<Vec<i32>>>()?;
                let rank = int_parse(col(&r, 1)?)?.wrapping_mul(10).wrapping_add(int_parse(col(&r, 2)?)?);
                let eid = int_parse(col(&r, 0)?)?;
                if let Some(n) = r.get(5).and_then(|c| crate::fmt::parse_i32(c.trim())).filter(|n| *n > 0 && (*n as usize) < ids.len()) {
                    picks.insert((eid, rank), n);
                }
                by_id.entry(eid).or_default().push((rank, ids.clone()));
                let names: Vec<&str> = if r.len() > 4 { r[4].split(" | ").collect() } else { Vec::new() };
                for i in 0..ids.len().min(names.len()) {
                    client_skill_names.entry(ids[i]).or_insert_with(|| names[i].trim().to_string());
                }
            }
            Ok(RankData { by_id, client_skill_names, picks })
        })
    }

    /// The game's own description of a skill (reference\skill-text.tsv, the client's CSkillData), with real line breaks; `None` if unknown.
    pub fn skill_text(&self, skill_id: i32) -> Result<Option<String>> {
        // BABEL's English where the game's text is Japanese (desc-en.tsv); skill_text_ja gives the game's own
        Ok(self.skill_text_game(skill_id)?.map(|t| desc_en(&t).unwrap_or(t)))
    }

    /// The game's own (Japanese) description of a skill when [`skill_text`](Self::skill_text) shows BABEL's translation of
    /// it; `None` when the text shown is the game's.
    pub fn skill_text_ja(&self, skill_id: i32) -> Result<Option<String>> {
        Ok(self.skill_text_game(skill_id)?.filter(|t| desc_en(t).is_some()))
    }

    /// The client's description of a skill as it is (reference\skill-text.tsv), with real line breaks.
    fn skill_text_game(&self, skill_id: i32) -> Result<Option<String>> {
        let t = once(&self.skill_text, || {
            let mut res = HashMap::new();
            if let Some(lines) = reference_lines("skill-text.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 3 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            if !r[2].trim().is_empty() {
                                res.insert(id, r[2].replace("\\n", "\n").trim().to_string());
                            }
                        }
                    }
                }
            }
            Ok(res)
        })?;
        Ok(t.get(&skill_id).cloned())
    }

    /// The game's own text of a tokusei (reference\tokusei-client-text.tsv, the client's CMessageData_Charastic: "Summoner Curative
    /// Received +20%"), with real line breaks; `None` if the client has none.
    pub fn tokusei_text(&self, tokusei_id: i32) -> Result<Option<String>> {
        let t = once(&self.tokusei_text, || {
            let mut res = HashMap::new();
            if let Some(lines) = reference_lines("tokusei-client-text.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 3 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            if !r[2].trim().is_empty() {
                                res.insert(id, r[2].replace("\\n", "\n").trim_end().to_string());
                            }
                        }
                    }
                }
            }
            Ok(res)
        })?;
        Ok(t.get(&tokusei_id).cloned())
    }

    /// The client's icon name for a skill (e.g. "icon_SD01_0002a"); `None` if it has none. From reference\skill-icons.tsv
    /// (IMAGINE's own CSkillData iconID + CIconData_Skill), which wins over the website's skills.json: that one lacks
    /// IMAGINE-only skills (Quickdraw) and has no icon for many others (Sympathy skills, Gun Mastery).
    pub fn skill_icon_asset(&self, skill_id: i32) -> Result<Option<String>> {
        let icons = once(&self.skill_icons, || {
            let root = self.load_content_json(&["wiki", "skills.json"], json!({ "skills": [] }))?;
            let mut res: HashMap<i32, Option<String>> = HashMap::new();
            let mut seen = HashSet::new();
            for s in json::arr(root.at("skills")) {
                let id = json::int(s.at("id"));
                if seen.insert(id) {
                    res.insert(id, json::str(s.at("iconAsset")));
                }
            }
            if let Some(lines) = reference_lines("skill-icons.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 3 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            res.insert(id, Some(r[2].to_string()));
                        }
                    }
                }
            }
            Ok(res)
        })?;
        let a = icons.get(&skill_id).cloned().flatten();
        // it becomes a file name: letters, digits, "_" and "-" (Infernal Hail's is icon_ice-hurricane2)
        Ok(a.filter(|a| !a.is_empty() && is_file_name_like(a, &['_', '-']) && !a.starts_with('-')))
    }

    /// Skill names as the game shows them: the client's CSkillData name (reference\skill-text.tsv, and the expertise
    /// ranks' names) whenever it is an English one; the website's skills.json only where the client's is Japanese
    /// (untranslated or half-translated, like "Vileの牙") or missing. The website's names lag and drift: 334 differ
    /// from the client's, e.g. Talk to the Fist! (814) is "Fistfight Communication" there, and from 5139 on many are
    /// shifted onto other skills (Rush Up shows as "Critical Up 3").
    pub fn skill_names(&self) -> Result<&IndexMap<i32, Option<String>>> {
        once(&self.skill_names, || {
            let root = self.load_content_json(&["wiki", "skills.json"], json!({ "skills": [] }))?;
            let mut res: IndexMap<i32, Option<String>> = IndexMap::new();
            for s in json::arr(root.at("skills")) {
                res.entry(json::int(s.at("id"))).or_insert_with(|| json::str(s.at("name")));
            }
            for (id, n) in &self.rank_data()?.client_skill_names {
                res.insert(*id, Some(n.clone())); // the client's own name wins (the game shows it; the website's can differ)
            }
            // the client's English names win over the website's
            if let Some(lines) = reference_lines("skill-text.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 2 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            // a line break in the name ("Incense of Magic and Experience\n(Demons)", written as \n) is a space
                            let n = RE_SPACES.replace_all(&r[1].replace("\\n", " "), " ").trim().to_string();
                            if is_english_game_name(&n) {
                                res.insert(id, Some(n));
                            }
                        }
                    }
                }
            }
            // BABEL's English where the client's name is Japanese (reference\skill-names-en.tsv); the pages show it greyed out.
            // It wins over the website's name too (bundled skills.json: "Glowing Eye" for 光る眼, and from 5139 on names shifted
            // onto other skills), so every page shows the same English; only the client's own (the expertise ranks') wins over it.
            let rank_names = &self.rank_data()?.client_skill_names;
            for (id, (_, en)) in self.skill_translations()? {
                if !rank_names.contains_key(id) {
                    res.insert(*id, Some(en.clone()));
                }
            }
            // IMAGINE-only skills the website lacks: tokusei-missing.tsv names the SkillData records that use each tokusei ("SkillData:id:name").
            if let Some(lines) = reference_lines("tokusei-missing.tsv")? {
                for line in &lines {
                    for m in RE_SKILLDATA.captures_iter(line) {
                        let id = int_parse(&m[1])?;
                        res.entry(id).or_insert_with(|| Some(m[2].trim().to_string()));
                    }
                }
            }
            // the rest from the client's CSkillData (reference\skill-text.tsv): IMAGINE-only skills like Infernal Hail
            if let Some(lines) = reference_lines("skill-text.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 2 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            if !r[1].trim().is_empty() {
                                res.entry(id).or_insert_with(|| Some(r[1].trim().to_string()));
                            }
                        }
                    }
                }
            }
            Ok(res)
        })
    }

    /// BABEL's own English for skills the client names only in Japanese (or half in Japanese, like "Vileの牙"):
    /// reference\skill-names-en.tsv (id, the client's name, the translation), id -> (Japanese, English). A line counts only
    /// while the client's name in skill-text.tsv is still that Japanese one, so a client update that renames or translates
    /// the skill wins over it.
    pub fn skill_translations(&self) -> Result<&HashMap<i32, (String, String)>> {
        once(&self.skill_translations, || {
            let mut client: HashMap<i32, String> = HashMap::new();
            for line in reference_lines("skill-text.tsv")?.unwrap_or_default() {
                let r: Vec<&str> = line.split('\t').collect();
                if let (Some(id), Some(n)) = (crate::fmt::parse_i32(r[0]), r.get(1)) {
                    client.insert(id, n.trim().to_string());
                }
            }
            let mut res = HashMap::new();
            for line in reference_lines("skill-names-en.tsv")?.unwrap_or_default() {
                let r: Vec<&str> = line.split('\t').collect();
                if line.starts_with('#') || r.len() < 3 {
                    continue;
                }
                if let Some(id) = crate::fmt::parse_i32(r[0]) {
                    let (ja, en) = (r[1].trim(), r[2].trim());
                    if !en.is_empty() && client.get(&id).map(String::as_str) == Some(ja) {
                        res.insert(id, (ja.to_string(), en.to_string()));
                    }
                }
            }
            Ok(res)
        })
    }

    /// The client's Japanese name of a skill whose shown name (`skill_names`) is BABEL's translation; `None` when the
    /// name shown is the game's own. Pages show such a name greyed out, with the Japanese in its tooltip.
    pub fn skill_name_ja(&self, id: i32) -> Result<Option<String>> {
        let Some((ja, en)) = self.skill_translations()?.get(&id) else { return Ok(None) };
        Ok((self.skill_names()?.get(&id).cloned().flatten().as_deref() == Some(en.as_str())).then(|| ja.clone()))
    }

    // ---------- Enchants (tarot/soul crystals) and fusion sets ----------

    pub fn enchants(&self) -> Result<&IndexMap<i32, Enchant>> {
        once(&self.enchants, || {
            let root = self.load_content_json(&["wiki", "enchants.json"], json!({ "enchants": {} }))?;
            let mut res: IndexMap<i32, Enchant> = IndexMap::new();
            for (_, v) in json::as_object(root.at("enchants"))? {
                if v.is_null() {
                    continue;
                }
                let e = Enchant {
                    id: json::int(v.at("id")),
                    crystal: json::str(v.at("sourceName")),
                    tarot_name: json::str(v.at("tarot").at("name")),
                    soul_name: json::str(v.at("soul").at("name")),
                };
                res.entry(e.id).or_insert(e);
            }
            Ok(res)
        })
    }

    pub fn enchant_sets(&self) -> Result<&Vec<EnchantSet>> {
        once(&self.enchant_sets, || {
            let root = self.load_content_json(&["armory", "enchant-sets.json"], json!({ "sets": {} }))?;
            let nonzero = |n: Option<&Value>| -> Vec<i32> { json::arr(n).map(|x| json::int(Some(x))).filter(|&x| x != 0).collect() };
            let mut res = Vec::new();
            for (_, v) in json::as_object(root.at("sets"))? {
                if v.is_null() {
                    continue;
                }
                res.push(EnchantSet {
                    id: json::int(v.at("id")),
                    effects: json::arr(v.at("effects")).map(|x| json::int(Some(x))).collect(),
                    tokusei: nonzero(v.at("tokuseiIds")),
                    conditions: json::arr(v.at("conditions")).map(|c| (json::int(c.at("type")), nonzero(c.at("tokuseiIds")))).collect(),
                });
            }
            Ok(res)
        })
    }

    /// One item each to stand for "a tarot" and "a soul fusion" in pictures: an item carrying the icon that most tarot
    /// sources (enchants.json usage 1 = tarot only, 3 = both) and most soul sources (usage 2 = soul only, 3) use.
    /// Today those are the Tarot Card and the Soul Stone. 0 when unknown.
    pub fn fusion_icon_items(&self) -> Result<(i32, i32)> {
        Ok(*once(&self.fusion_icon_items, || {
            let root = self.load_content_json(&["wiki", "enchants.json"], json!({ "enchants": {} }))?;
            let mut src: Vec<(i32, i32, i32)> = Vec::new(); // (usage, item, icon key)
            for (_, v) in json::as_object(root.at("enchants"))? {
                if v.is_null() {
                    continue;
                }
                let (usage, item) = (json::int(v.at("usage")), json::int(v.at("crystalItemId")));
                if item > 0 {
                    let key = self.item_icon(item)?.0;
                    if key > 0 {
                        src.push((usage, item, key));
                    }
                }
            }
            let top = |bit: i32| -> i32 {
                // GroupBy icon key (first-appearance order) -> OrderByDescending(count).ThenBy(key) -> Min(item) of the first group
                let mut groups: IndexMap<i32, (usize, i32)> = IndexMap::new();
                for &(_, item, key) in src.iter().filter(|e| e.0 & bit != 0) {
                    let g = groups.entry(key).or_insert((0, i32::MAX));
                    g.0 += 1;
                    g.1 = g.1.min(item);
                }
                let mut v: Vec<(i32, (usize, i32))> = groups.into_iter().collect();
                v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
                v.first().map(|g| (g.1).1).unwrap_or(0)
            };
            Ok((top(1), top(2)))
        })?)
    }

    /// Crystal name as a lookup key: "Cerberus' Crystal", "Lucifuge Crystal", "Soul Stone: Procyon" -> "cerberus", "lucifuge", "soul stone: procyon".
    pub fn crystal_key(name: Option<&str>) -> String {
        let Some(name) = name else { return String::new() };
        if name.chars().all(char::is_whitespace) {
            return String::new();
        }
        let lower = name.trim().to_lowercase();
        let k = RE_CRYSTAL_END.replace_all(&lower, "");
        let k = RE_POSSESSIVE.replace_all(&k, "");
        RE_SPACES.replace_all(&k, " ").trim().to_string()
    }

    fn enchant_by_crystal(&self) -> Result<&HashMap<String, Vec<i32>>> {
        once(&self.enchant_by_crystal, || {
            let mut m: HashMap<String, Vec<i32>> = HashMap::new();
            for e in self.enchants()?.values() {
                m.entry(GameData::crystal_key(e.crystal.as_deref())).or_default().push(e.id);
            }
            Ok(m)
        })
    }

    /// Enchant ids whose crystal has this name (as on a tooltip's [Tarot Fusion] / [Soul Fusion] line).
    pub fn enchant_ids_by_crystal(&self, name: &str) -> Result<Vec<i32>> {
        Ok(self.enchant_by_crystal()?.get(&GameData::crystal_key(Some(name))).cloned().unwrap_or_default())
    }

    /// Is this a crystal name ("With Orthrus:" names a crystal, so the line is a fusion-set bonus)?
    pub fn is_crystal(&self, name: &str) -> Result<bool> {
        Ok(self.enchant_by_crystal()?.contains_key(&GameData::crystal_key(Some(name))))
    }

    /// The in-game name of the crystal an enchant id comes from ("Ogami's Crystal", "Soul Stone: Procyon"), or `None` when
    /// no table names it. The client's own enchant table gives the crystal's item (its name is the game's); the website's
    /// list is the fallback for an id the client table lacks. `tarot` picks the side to look at first: a soul and a tarot of
    /// one id come from the same crystal, but only one side may be in a table.
    pub fn crystal_name(&self, id: i32, tarot: Option<bool>) -> Result<Option<String>> {
        let sides = self.enchant_sides()?;
        let mut tries: Vec<(i32, bool)> = Vec::new();
        match tarot {
            Some(t) => tries.extend([(id, t), (id, !t)]),
            None => tries.extend([(id, true), (id, false)]),
        }
        for key in tries {
            if let Some(side) = sides.get(&key) {
                if side.crystal_item > 0 {
                    if let Some(n) = self.item_name(side.crystal_item)?.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
                        return Ok(Some(n));
                    }
                }
            }
        }
        Ok(self.enchants()?.get(&id).and_then(|e| e.crystal.clone()).map(|c| c.trim().to_string()).filter(|c| !c.is_empty()))
    }

    /// What the page shows for the crystal fused into a piece: its name, or when the data has none, plain words (never the id).
    pub fn crystal_label(&self, id: i32, tarot: bool) -> Result<String> {
        Ok(self.crystal_name(id, Some(tarot))?.unwrap_or_else(|| if tarot { "a tarot".to_string() } else { "a soul fusion".to_string() }))
    }

    /// A crystal's name without its trailing "Crystal" ("Ogami's"), as the pieces of a fusion set are listed; "a crystal" when
    /// no table names it (the id is an internal number and never shown).
    pub fn enchant_name(&self, id: i32) -> Result<String> {
        Ok(match self.crystal_name(id, None)? {
            Some(c) => RE_CRYSTAL_NAME_END.replace_all(&c, "").into_owned(),
            None => "a crystal".to_string(),
        })
    }

    // ---------- Item instances: what the server applies for one worn item (CharacterState.cpp) ----------

    /// Every tarot (`true`) and soul side the data has, by enchant id.
    pub fn enchant_sides(&self) -> Result<&IndexMap<(i32, bool), EnchantSide>> {
        self.load_enchant_sides()
    }

    /// An item's full correct table (stats and characteristics): what its id gives as a "basic effect". `None` for an unknown item.
    pub fn item_correct(&self, id: i32) -> Result<Option<Vec<Correct>>> {
        let Some(n) = self.items()?.get(&id) else { return Ok(None) };
        Ok(Some(
            json::arr(n.at("basicFeatures"))
                .chain(json::arr(n.at("characteristics")))
                .map(|c| Correct { id: json::str(c.at("id")), r#type: json::int(c.at("type")), value: json::num(c.at("value")).unwrap_or(0.0) })
                .collect(),
        ))
    }

    pub fn item_weapon_type(&self, id: i32) -> Result<Option<String>> {
        Ok(self.items()?.get(&id).and_then(|n| json::str(n.at("weaponType"))))
    }

    pub fn item_equip_type(&self, id: i32) -> Result<Option<String>> {
        Ok(self.items()?.get(&id).and_then(|n| json::str(n.at("equipType"))))
    }

    pub fn item_sub_category(&self, id: i32) -> Result<i32> {
        let data = self.items_data()?; // loads the client's sub-categories (items-game.tsv); the website's file is the fallback
        if let Some(&c) = data.subcats_game.get(&id) {
            return Ok(c);
        }
        let subcats = once(&self.subcats, || {
            let root = self.load_content_json(&["armory", "item-subcategories.json"], json!({ "items": {} }))?;
            let mut m = HashMap::new();
            for (k, v) in json::as_object(root.at("items"))? {
                m.insert(int_parse(k)?, json::int(Some(v)));
            }
            Ok(m)
        })?;
        Ok(subcats.get(&id).copied().unwrap_or(-1))
    }

    /// Special-effect tokusei of an item id (SItemData): what its id gives as a "special effect".
    pub fn sitem_tokusei(&self, id: i32) -> Result<Vec<i32>> {
        // reference\sitem-game.tsv (the client's current SItemData) is complete: an item not in it has no special effect.
        // Without that file, the website's (older) copy.
        let src = once(&self.sitem, || {
            if let Some(lines) = reference_lines("sitem-game.tsv")? {
                let mut m = HashMap::new();
                for line in &lines {
                    if line.starts_with('#') {
                        continue;
                    }
                    let p: Vec<&str> = line.split('\t').collect();
                    if p.len() >= 2 {
                        if let Some(sid) = crate::fmt::parse_i32(p[0]) {
                            m.insert(
                                sid,
                                split_nonempty(p[1], ',').into_iter().map(|x| crate::fmt::parse_i32(x).unwrap_or(0)).filter(|&v| v != 0).collect(),
                            );
                        }
                    }
                }
                Ok(SItemSource::Game(m))
            } else {
                let root = self.load_content_json(&["armory", "sitem-tokusei.json"], json!({ "items": {} }))?;
                Ok(SItemSource::Website(json::as_object(root.at("items"))?.clone()))
            }
        })?;
        Ok(match src {
            SItemSource::Game(m) => m.get(&id).cloned().unwrap_or_default(),
            SItemSource::Website(o) => json::arr(o.get(&id.to_string())).map(|x| json::int(Some(x))).filter(|&x| x != 0).collect(),
        })
    }

    /// Tokusei of a modification slot effect. Weapons: the modified-effect table by effect id. Other gear: the
    /// extension table by item subcategory, equip type and effect id. 0 when unknown.
    pub fn mod_tokusei(&self, weapon: bool, sub_category: i32, equip_type: i32, effect: i32) -> Result<i32> {
        let mods = once(&self.mods, || {
            let v = self.load_content_json(&["armory", "mod-effects.json"], json!({}))?;
            json::as_object(Some(&v))?;
            Ok(v)
        })?;
        if weapon {
            // the client's current table first (mods-weapon.tsv), the website's file for what it lacks
            if let Some(m) = self.weapon_mods()?.get(&effect).filter(|m| m.tokusei != 0) {
                return Ok(m.tokusei);
            }
        }
        let n = if weapon {
            mods.at("weapon").at(&effect.to_string())
        } else {
            mods.at("armor").at(&sub_category.to_string()).at(&equip_type.to_string()).at(&effect.to_string())
        };
        Ok(json::int(n))
    }

    /// The client's icon for a demon type: its name (e.g. "icon_D3_01_004a") and the folder under the client's Interface
    /// that holds it ("tga" or "tga2"); `None` if unknown. From reference\devil-icons.tsv (the picture named after the
    /// demon's model, checked against the client's files: it has the IMAGINE-only demons, the ones whose website name is
    /// a "Bone" model name, and the ones kept in tga2), else the website's devils.json name in "tga".
    pub fn devil_icon_file(&self, r#type: i32) -> Result<Option<(String, String)>> {
        let files = once(&self.devil_icon_files, || {
            let mut res = HashMap::new();
            if let Some(lines) = reference_lines("devil-icons.tsv")? {
                for line in &lines {
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() >= 3 {
                        if let Some(id) = crate::fmt::parse_i32(r[0]) {
                            if r[2] == "tga" || r[2] == "tga2" {
                                res.insert(id, (r[1].to_string(), r[2].to_string()));
                            }
                        }
                    }
                }
            }
            Ok(res)
        })?;
        let hit = match files.get(&r#type) {
            Some(h) => Some(h.clone()),
            None => {
                let icons = once(&self.devil_icons, || {
                    let root = self.load_content_json(&["armory", "devils.json"], json!({}))?;
                    Ok(match root.at("iconAssets") {
                        Some(Value::Object(m)) => m.clone(),
                        _ => Map::new(),
                    })
                })?;
                json::str(icons.get(&r#type.to_string())).map(|n| (n, "tga".to_string()))
            }
        };
        // it becomes a file name
        Ok(hit.filter(|(name, _)| !name.is_empty() && is_file_name_like(name, &['_'])))
    }

    /// A demon type's alignment: LAW, NEUTRAL or CHAOS from its LNC points in devils.json (server rule
    /// ActiveEntityState::CalculateLNCType: 5000 or more is Chaos, -5000 or less is Law); `None` if unknown.
    pub fn devil_alignment(&self, r#type: i32) -> Result<Option<&'static str>> {
        let lncs = once(&self.devil_lnc, || {
            let root = self.load_content_json(&["armory", "devils.json"], json!({}))?;
            Ok(match root.at("lncs") {
                Some(Value::Object(m)) => m.clone(),
                _ => Map::new(),
            })
        })?;
        let Some(n) = lncs.get(&r#type.to_string()).filter(|v| !v.is_null()) else { return Ok(None) };
        let lnc = json::int(Some(n));
        Ok(Some(if lnc >= 5000 {
            "CHAOS"
        } else if lnc <= -5000 {
            "LAW"
        } else {
            "NEUTRAL"
        }))
    }

    /// The server's demon, growth, Mitama and digitalize-assist tables (reference\*.tsv).
    pub fn demons(&self) -> Result<&DemonTables> {
        once(&self.demons, DemonTables::new)
    }

    /// The tarot (`tarot` = true) or soul side of an enchant id, or `None`. From the client's own enchant table when
    /// available (reference\enchants-game.tsv), otherwise the website's enchants.json.
    pub fn enchant_effect(&self, id: i32, tarot: bool) -> Result<Option<&EnchantSide>> {
        Ok(self.load_enchant_sides()?.get(&(id, tarot)))
    }

    fn load_enchant_sides(&self) -> Result<&IndexMap<(i32, bool), EnchantSide>> {
        once(&self.enchant_sides, || {
            let mut sides: IndexMap<(i32, bool), EnchantSide> = IndexMap::new();
            if let Some(tsv) = game_enchant_table() {
                for line in &tsv.lines()? {
                    let f: Vec<&str> = line.split('\t').collect();
                    if f.len() < 9 {
                        continue;
                    }
                    let Some(eid) = crate::fmt::parse_i32(f[0]) else { continue };
                    if f[3] != "tarot" && f[3] != "soul" {
                        continue;
                    }
                    let toks = split_nonempty(f[7], ',').into_iter().map(int_parse).collect::<Result<Vec<i32>>>()?.into_iter().filter(|&x| x != 0).collect();
                    let mut conds = Vec::new();
                    for c in split_nonempty(f[8], ';') {
                        let Some(m) = RE_ENCHANT_COND.captures(c) else { continue };
                        let g = |i: usize| m.get(i).map(|x| int_parse(x.as_str())).transpose();
                        let tk = split_nonempty(&m[4], ',').into_iter().map(int_parse).collect::<Result<Vec<i32>>>()?.into_iter().filter(|&x| x != 0).collect();
                        conds.push((int_parse(&m[1])?, g(2)?.unwrap_or(0), g(3)?.unwrap_or(0), tk));
                    }
                    sides.insert(
                        (eid, f[3] == "tarot"),
                        EnchantSide {
                            name: Some(f[4].trim().to_string()),
                            tokusei: toks,
                            conditions: conds,
                            mask: crate::fmt::parse_i32(f[6]).unwrap_or(0),
                            crystal_item: crate::fmt::parse_i32(f[2]).unwrap_or(0),
                            text: f.get(9).map(|s| s.to_string()).unwrap_or_default(),
                        },
                    );
                }
            }
            let root = self.load_content_json(&["wiki", "enchants.json"], json!({ "enchants": {} }))?;
            for (_, v) in json::as_object(root.at("enchants"))? {
                if v.is_null() {
                    continue;
                }
                let eid = json::int(v.at("id"));
                for (side, is_tarot) in [("tarot", true), ("soul", false)] {
                    let Some(s) = v.at(side) else { continue };
                    if sides.contains_key(&(eid, is_tarot)) {
                        continue;
                    }
                    let nonzero = |n: Option<&Value>| -> Vec<i32> { json::arr(n).map(|x| json::int(Some(x))).filter(|&x| x != 0).collect() };
                    sides.insert(
                        (eid, is_tarot),
                        EnchantSide {
                            name: json::str(s.at("name")),
                            tokusei: nonzero(s.at("tokuseiIds")),
                            conditions: json::arr(s.at("conditions"))
                                .map(|c| {
                                    (
                                        json::int(c.at("type")),
                                        json::int(json::arr(c.at("params")).next()),
                                        json::int(json::arr(c.at("params")).nth(1)),
                                        nonzero(c.at("tokuseiIds")),
                                    )
                                })
                                .collect(),
                            mask: 0,
                            crystal_item: 0,
                            text: String::new(),
                        },
                    );
                }
            }
            Ok(sides)
        })
    }

    // ---------- Tokusei, complete (server XML first, website JSON as fallback) ----------

    /// The client's text for a tokusei the server data lacks (its own text, or the description of the record using it); `None` if none.
    pub fn client_tokusei_text(&self, id: i32) -> Result<Option<String>> {
        let t = once(&self.client_tok_text, || {
            let mut m = HashMap::new();
            if let Some(lines) = reference_lines("tokusei-missing.tsv")? {
                for line in &lines {
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let r: Vec<&str> = line.split('\t').collect();
                    if r.len() > 3 {
                        if let Some(tid) = crate::fmt::parse_i32(r[0]) {
                            if !r[3].is_empty() {
                                m.insert(tid, r[3].to_string());
                            }
                        }
                    }
                }
            }
            Ok(m)
        })?;
        Ok(t.get(&id).cloned())
    }

    /// A tokusei the server data lacks, from the client's own text for it: only rows the client has text for this exact
    /// id, with no condition and every line understood (reference\tokusei-missing.tsv, fullyParsed = yes). Parsed keys:
    /// CorrectTbl names (KEY=+N or +N%), aspects, EFFECT_POWER[action=X]; a partner:/party:/summoner: prefix is the target.
    pub fn client_tokusei(&self, id: i32) -> Result<Option<Arc<TokuseiDef>>> {
        let t = once(&self.client_tok, || {
            let mut m: HashMap<i32, Arc<TokuseiDef>> = HashMap::new();
            let Some(lines) = reference_lines("tokusei-missing.tsv")? else { return Ok(m) };
            for line in &lines {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let r: Vec<&str> = line.split('\t').collect();
                if r.len() < 9 {
                    continue;
                }
                let Some(tid) = crate::fmt::parse_i32(r[0]) else { continue };
                let mut parsed = r[6].to_string();
                if r[8] != "yes" {
                    // Also usable: rows whose only unparsed parts are "all party (members)" (the whole party: the
                    // character and the partner) and flavour ("By the power of the Moon"), e.g. Blessing of Soma
                    // "all party members' MAX MP +25", which the summoned demons' and Rubito's Max MP confirmed.
                    let mut segs: Vec<String> = r[7].split(" | ").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    // "members' vitality +3," (the stat split from "all party" by a line break), e.g. Blessing of Shani
                    for s in segs.clone() {
                        if let Some(sm) = RE_MEMBERS_STAT.captures(&s) {
                            let stat = match &sm[1] {
                                "strength" => "STR",
                                "magic" => "MAGIC",
                                "vitality" => "VIT",
                                "intelligence" => "INT",
                                "speed" => "SPEED",
                                _ => "LUCK",
                            };
                            parsed = format!("{parsed} {stat}=+{}", &sm[2]).trim().to_string();
                            if let Some(pos) = segs.iter().position(|x| *x == s) {
                                segs.remove(pos);
                            }
                        }
                    }
                    if parsed.is_empty() || parsed.contains(':') || segs.is_empty() || !segs.iter().all(|s| RE_PARTY_SEG.is_match(s)) {
                        continue;
                    }
                    if segs.iter().any(|s| s.starts_with("all party")) {
                        parsed = split_nonempty(&parsed, ' ').into_iter().map(|t| format!("party:{t}")).collect::<Vec<_>>().join(" ");
                    }
                }
                if let Some(def) = parse_client_tokusei(tid, &parsed) {
                    m.insert(tid, Arc::new(def));
                }
            }
            Ok(m)
        })?;
        Ok(t.get(&id).cloned())
    }

    /// The name of the server XML file that holds this tokusei id ("tokusei_00010000.xml"); read it with [`Self::tokusei_xml_text`].
    fn tokusei_file(&self, id: i32) -> String {
        format!("tokusei_{}.xml", d8(id.wrapping_div(10000).wrapping_mul(10000)))
    }

    /// Everything a tokusei does and needs, or `None` when no source has it. Order: the sealed tokusei, comp_hack's
    /// file for the id's range, the website file, the client's text (reference\tokusei-missing.tsv).
    pub fn tokusei_full(&self, id: i32) -> Result<Option<Arc<TokuseiDef>>> {
        let mut st = lock(&self.defs);
        if let Some(d) = st.defs.get(&id) {
            return Ok(d.clone());
        }
        // each file is parsed once (st.files); the sealed one on the first call, so its ids are in before any range file's
        for file in [SEALED_TOKUSEI.to_string(), self.tokusei_file(id)] {
            let Some(text) = (if st.files.insert(file.clone()) { self.tokusei_xml_text(&file)? } else { None }) else { continue };
            let doc = parse_xml(&text)?;
            for o in doc.descendants().filter(|x| x.is_element() && x.tag_name().name() == "object" && x.attribute("name") == Some("Tokusei")) {
                let Some(tid) = member_text(o, "ID").and_then(|s| crate::fmt::parse_i32(&s)) else { continue };
                if st.defs.contains_key(&tid) {
                    continue;
                }
                let def = xml_tokusei_def(o, tid)?;
                st.defs.insert(tid, Some(Arc::new(def)));
            }
        }
        if let Some(d) = st.defs.get(&id) {
            return Ok(d.clone());
        }
        let mut d: Option<Arc<TokuseiDef>> = None;
        if let Some(j) = self.tokusei()?.at(&id.to_string()) {
            let v = |c: &Value, id_field: &str| TokValue {
                id: json::str(c.at(id_field)),
                r#type: json::int(c.at("type")),
                value: json::num(c.at("value")).unwrap_or(0.0),
                mult: json::str(c.at("attr").at("mult")).unwrap_or_else(|| "NONE".into()),
                mult_value: json::int(c.at("attr").at("value")),
                precision: json::int(c.at("attr").at("precision")),
                modifier: None,
            };
            d = Some(Arc::new(TokuseiDef {
                id,
                target: json::str(j.at("target")).unwrap_or_else(|| "SELF".into()),
                conditions: json::arr(j.at("conditions"))
                    .map(|c| TokCond {
                        r#type: json::str(c.at("type")),
                        value: json::int(c.at("value")),
                        cmp: json::str(c.at("cmp")).unwrap_or_else(|| "EQUALS".into()),
                        group: json::int(c.at("group")),
                    })
                    .collect(),
                skill_conditions: Vec::new(),
                target_conditions: json::truthy(j.at("skill")),
                correct: json::arr(j.at("correct")).chain(json::arr(j.at("tokuseiCorrect"))).map(|c| v(c, "id")).collect(),
                aspects: json::arr(j.at("aspects")).map(|c| v(c, "type")).collect(),
                from_client_text: false,
            }));
        }
        let d = match d {
            Some(d) => Some(d),
            None => self.client_tokusei(id)?,
        };
        st.defs.insert(id, d.clone());
        Ok(d)
    }

    /// One server tokusei file's text, comments taken out (for the text searches below), cached.
    fn xml_file(&self, file: String) -> Result<Option<Arc<String>>> {
        let mut files = lock(&self.xml_files);
        if let Some(x) = files.get(&file) {
            return Ok(x.clone());
        }
        let x = self.tokusei_xml_text(&file)?.map(|t| Arc::new(strip_xml_comments(&t)));
        files.insert(file, x.clone());
        Ok(x)
    }

    /// The server files that may hold this tokusei id, in the order [`Self::tokusei_full`] reads them: the sealed one,
    /// then comp_hack's file for the id's range.
    fn xml_for(&self, id: i32) -> Result<Vec<Arc<String>>> {
        Ok([self.xml_file(SEALED_TOKUSEI.to_string())?, self.xml_file(self.tokusei_file(id))?].into_iter().flatten().collect())
    }

    /// A tokusei straight from the server XML (for ids the website file leaves out).
    pub fn raw_tokusei_xml(&self, id: i32) -> Result<Option<Arc<RawTokusei>>> {
        if let Some(r) = lock(&self.raw).get(&id) {
            return Ok(r.clone());
        }
        let mut r = None;
        let re = FancyRegex::new(&format!(r#"(?is)<member name="ID">{id}</member>(.*?)(?=<object name="Tokusei">|\z)"#))?;
        for x in self.xml_for(id)? {
            if r.is_some() {
                break;
            }
            if let Some(m) = re.captures(x.as_str())? {
                let body = &m[1];
                let tt = RE_TARGET_TYPE.captures(body);
                let mut asp = Vec::new();
                for a in RE_RAW_TYPES.captures_iter(body) {
                    asp.push((a[1].to_string(), int_parse(&a[2])?));
                }
                r = Some(Arc::new(RawTokusei {
                    target: tt.map(|t| t[1].to_string()),
                    aspects: asp,
                    digitalized: RE_DIGITALIZED.is_match(body),
                }));
            }
        }
        lock(&self.raw).insert(id, r.clone());
        Ok(r)
    }

    /// "TYPE=value, ..." for one tokusei object in the server XML (expertise passives the website leaves out).
    pub fn raw_tokusei_types(&self, id: i32) -> Result<Option<String>> {
        if let Some(s) = lock(&self.raw_types).get(&id) {
            return Ok(s.clone());
        }
        let mut s = None;
        let re = Regex::new(&format!(r#"(?is)<member name="ID">{id}</member>(.*?)</object>\s*</element>\s*</member>\s*</object>"#))?;
        for x in self.xml_for(id)? {
            if s.is_some() {
                break;
            }
            if let Some(m) = re.captures(&x) {
                s = Some(
                    RE_RAW_TYPES
                        .captures_iter(&m[1])
                        .map(|a| format!("{}={}", &a[1], &a[2]))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
        }
        lock(&self.raw_types).insert(id, s.clone());
        Ok(s)
    }
}

/// `root[key].AsObject()`, taking the object out of the parsed file (an error when it is missing or not an object).
fn take_object(mut root: Value, key: &str) -> Result<Value> {
    match root.get_mut(key).map(Value::take) {
        Some(v @ Value::Object(_)) => Ok(v),
        _ => Err(anyhow::anyhow!("The node must be of type 'JsonObject'.")),
    }
}

/// `Val(o, idField)` of `TokuseiFull`: one correct value / aspect object of the server XML.
fn xml_tok_value(o: roxmltree::Node, id_field: &str) -> Result<TokValue> {
    let attr = xml_children(o, "member").find(|m| m.attribute("name") == Some("Attributes")).and_then(|m| xml_children(m, "object").next());
    Ok(TokValue {
        id: member_text(o, id_field),
        r#type: member_text(o, "Type").and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0),
        value: double_parse(&member_text(o, "Value").unwrap_or_else(|| "0".into()))?,
        mult: attr.and_then(|a| member_text(a, "MultiplierType")).unwrap_or_else(|| "NONE".into()),
        mult_value: attr.and_then(|a| member_text(a, "MultiplierValue")).and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0),
        precision: attr.and_then(|a| member_text(a, "Precision")).and_then(|s| crate::fmt::parse_i32(&s)).unwrap_or(0),
        modifier: member_text(o, "Modifier").and_then(|s| crate::fmt::parse_f64(&s)),
    })
}

fn parse_client_tokusei(id: i32, parsed: &str) -> Option<TokuseiDef> {
    let mut target: Option<String> = None;
    let mut correct = Vec::new();
    let mut aspects = Vec::new();
    let mut skill: Vec<(Option<String>, i32)> = Vec::new();
    for tok in split_nonempty(parsed, ' ') {
        let m = RE_CLIENT_TOK.captures(tok)?;
        let t = m.get(1).map(|g| g.as_str().to_uppercase()).unwrap_or_else(|| "SELF".into());
        if target.as_ref().is_some_and(|x| *x != t) {
            return None; // one target per tokusei
        }
        target = Some(t);
        let key = &m[2];
        let v = crate::fmt::parse_f64(&m[4]).unwrap_or(0.0);
        let pct = &m[5] == "%";
        if let Some(act) = m.get(3) {
            let a = ACTION_TYPE_NAMES.iter().position(|n| *n == act.as_str())? as i32;
            if skill.iter().any(|s| s.1 != a) {
                return None;
            }
            if skill.is_empty() {
                skill.push((Some("ACTION_TYPE".into()), a));
            }
        }
        if ASPECT_KEYS.contains(&key) {
            aspects.push(TokValue::new(Some(key), 0, v, "NONE", 0));
        } else {
            // A % on a stat-like value is the percent type; affinities and rates are already percentages
            let already_pct = key.starts_with("BOOST_") || key.starts_with("RES_") || key.starts_with("RATE_") || is_pct_correct(key);
            correct.push(TokValue::new(Some(key), if pct && !already_pct { 1 } else { 0 }, v, "NONE", 0));
        }
    }
    let target = target?;
    Some(TokuseiDef { id, target, conditions: Vec::new(), skill_conditions: skill, target_conditions: false, correct, aspects, from_client_text: true })
}

/// A linked condition ("tokusei N applies") points at the skill that lists N: Radiant Hands (a switch) lists
/// 2100039, so what depends on 2100039 depends on that switch. A passive that lists one is no switch, and a tokusei no skill
/// lists stays unknown.
#[cfg(test)]
#[test]
fn a_tokusei_finds_the_skill_that_lists_it() {
    let d = GameData::with_paths(None, None);
    assert_eq!(d.tokusei_owner(2100039).unwrap(), Some(TokuseiOwner { skill: 5485, switch: true }));
    assert_eq!(d.tokusei_owner(2000127).unwrap(), Some(TokuseiOwner { skill: 160611, switch: false }), "a passive is found, and is no switch");
    assert_eq!(d.tokusei_owner(0).unwrap(), None);
    assert_eq!(d.tokusei_owner(1).unwrap(), None);
    // the switch is found by the name the game gives it, case aside; a passive's name finds nothing
    assert_eq!(d.switch_skill_named("Radiant Hands").unwrap(), Some(5485));
    assert_eq!(d.switch_skill_named("radiant hands").unwrap(), Some(5485));
    assert_eq!(d.switch_skill_named("Cataclysmic Mind").unwrap(), None);
    assert_eq!(d.switch_skill_named("No Such Skill").unwrap(), None);
}
