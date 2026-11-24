//! Port of `src/Build/Pipeline.cs`.
//!
//! The whole "read the game, save, rebuild the page data" sequence in one place, shared by the command line
//! (the `newmoon-reader` crate) and the desktop app.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Local, NaiveDateTime};
use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::calc::digitalize::MITAMA_NAMES;
use crate::calc::item_effects::title;
use crate::context::Context;
use crate::data::characters::path_combine;
use crate::data::config;
use crate::data::demon_tables::Devil;
use crate::data::game_data::GameData;
use crate::data::json::{self, At};
use crate::data::store::Store;
use crate::diag::log;
use crate::error::is_user_error;
use crate::game::{DemonRecord, SetRecord, client_reader, demon_reader};
use crate::memory::{self, GameMemory};
use crate::{Result, user_error};

use super::csv::CsvTable;
use super::dashboard::{Dashboard, DemonOffer};
use super::demon_model::base64;
use super::digitalize_demon::{DigitalizeSave, compact, demon_from_value, demon_to_value, iso_s};
use super::expertise::Expertise;
use super::simulator::Simulator;
use super::snapshot::{Snapshot, SnapshotResult};

// ---------- the statics ----------

/// `Path.Combine(null, ...)` on a setting data\config.json does not have.
fn null_path() -> anyhow::Error {
    anyhow::anyhow!("Value cannot be null. (Parameter 'path1')")
}

/// `Pipeline.FindDataDir()`: the "data" folder next to the project file (found by walking up from the program's
/// folder), else beside the exe itself (the portable release).
pub fn find_data_dir() -> PathBuf {
    config::home_dir().join("data")
}

/// `Pipeline.DefaultName`: who the data is for when nothing else says (the defaultCharacter setting in data/config.json).
pub fn default_name() -> String {
    config::current().default_character.clone().unwrap_or_else(|| "Character".to_string())
}

/// `Pipeline.DefaultCsv`: the default character's CSV, in the stats folder (data\config.json "statsFolder").
pub fn default_csv() -> Result<String> {
    let stats = config::current().stats_folder.clone().ok_or_else(null_path)?;
    Ok(path_combine(&stats, &format!("{}-stats.csv", default_name().to_lowercase())))
}

/// `Func<List<SetRecord>> blankSets`: the set table a "no base" pipeline (the Simulator's) uses.
pub type BlankSets = Box<dyn Fn() -> Result<Option<Vec<SetRecord>>> + Send + Sync>;

/// `Pipeline.Counts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub snapshots: usize,
    pub gear: usize,
    pub expertise: usize,
    pub bonuses: usize,
}

/// `Pipeline.AlignmentResult`.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentResult {
    pub changed: bool,
    pub rows: usize,
    pub changed_rows: usize,
    pub backup: Option<String>,
    pub log: Vec<String>,
}

/// Where a demon stands in the game's Digitalization Level table (GuardianLevelData demonIDs, matched by the demon's
/// name within its race: a COMP demon can be a Mitama-fused type of the listed one). `listed` false: no level of its
/// race names it. `level`: the level that unlocks it; `race_level`: the character's level in that race as last read
/// (0 = none, None = levels not read yet).
#[derive(Debug, Clone, PartialEq)]
pub struct DemonUnlock {
    pub listed: bool,
    pub level: i32,
    pub race_level: Option<i32>,
    pub race: String,
}

impl DemonUnlock {
    pub fn reached(&self) -> bool {
        self.listed && self.race_level.is_some_and(|l| l >= self.level)
    }

    /// The page's JSON (camelCase, the record's members then `reached`).
    pub fn to_value(&self) -> Value {
        json!({ "listed": self.listed, "level": self.level, "raceLevel": self.race_level, "race": self.race, "reached": self.reached() })
    }
}

/// One COMP demon for the picker. Key = "type:name:level" (object ids change every session).
/// `offer`: what digitalizing with it would give (stat rate, stats, key bonuses, skills), for the Digitalize page's grid.
#[derive(Debug, Clone, PartialEq)]
pub struct CompDemon {
    pub key: String,
    pub name: String,
    pub r#type: i32,
    pub level: i32,
    pub race: String,
    pub mitama: String,
    pub current: bool,
    pub in_game: bool,
    pub seen_at: Option<String>,
    pub force_stack: usize,
    pub equipment: bool,
    pub icon: Option<String>,
    pub offer: Option<DemonOffer>,
    pub unlock: Option<DemonUnlock>,
    pub loaded: bool,
    pub summoned: bool,
    pub in_comp: Option<bool>,
}

impl CompDemon {
    /// The page's JSON for it (`JsonSerializer` with camelCase names: the record's members in order).
    pub fn to_value(&self) -> Value {
        json!({
            "key": self.key, "name": self.name, "type": self.r#type, "level": self.level, "race": self.race, "mitama": self.mitama,
            "current": self.current, "inGame": self.in_game, "seenAt": self.seen_at, "forceStack": self.force_stack, "equipment": self.equipment,
            "icon": self.icon, "offer": self.offer.as_ref().map(DemonOffer::to_value), "unlock": self.unlock.as_ref().map(DemonUnlock::to_value),
            "loaded": self.loaded, "summoned": self.summoned, "inComp": self.in_comp,
        })
    }
}

/// `Pipeline.DemonSwitch`.
#[derive(Debug, Clone, PartialEq)]
pub struct DemonSwitch {
    pub name: String,
    pub changed: bool,
    pub row_updated: bool,
    pub backup: Option<String>,
}

/// A demon seen in the game before (cache\comp-demons.json). `in_comp`: whether it was in the COMP at the last read of
/// the game's COMP slots (None = never checked: lists saved before 2026-11-23, or a demon read while the game held no
/// slot list).
#[derive(Debug, Clone, PartialEq)]
pub struct SeenDemon {
    pub seen_at: Option<String>,
    pub demon: DemonRecord,
    pub in_comp: Option<bool>,
}

impl SeenDemon {
    fn to_value(&self) -> Value {
        json!({ "SeenAt": self.seen_at, "Demon": demon_to_value(&self.demon), "InComp": self.in_comp })
    }

    fn from_value(v: &Value) -> Option<SeenDemon> {
        let demon = demon_from_value(v.at("Demon")?)?;
        Some(SeenDemon { seen_at: json::str(v.at("SeenAt")), demon, in_comp: v.at("InComp").and_then(Value::as_bool) })
    }
}

/// The key of a demon in the list: type, name and level. The level is part of it because two demons of one kind are
/// different demons (a level 90 Specter in storage and a level 1 one in the COMP); object ids can't be used, they
/// change every session. A demon that levels up becomes a new entry; the old one stays as "not in your COMP".
pub fn key_of(d: &DemonRecord) -> String {
    format!("{}:{}:{}", d.r#type, d.name, d.level)
}

/// The same details: skill slots, learned skills, growth and Mitama (what tells two demons of one kind and level apart
/// when object ids can't, across sessions).
fn same_details(a: &DemonRecord, b: &DemonRecord) -> bool {
    a.skills == b.skills && a.acquired == b.acquired && a.growth_type == b.growth_type && a.mitama_type == b.mitama_type
}

/// Whether two records of one kind and level are the same demon, not two demons of one kind and level (Rubito's COMP
/// holds two Specter Lv 90s, 2026-11-24) or a stale copy the client still holds. The same object id within a session;
/// across sessions (object ids change) the same details ([`same_details`]).
pub fn same_demon(a: &DemonRecord, b: &DemonRecord) -> bool {
    key_of(a) == key_of(b) && (a.object_id == b.object_id || same_details(a, b))
}

/// The list key a demon was stored under, without the "#2" a second demon of one kind and level gets (and without the
/// "|saved" mark): what [`key_of`] gives for it.
pub fn base_key(k: &str) -> &str {
    let k = k.strip_suffix("|saved").unwrap_or(k);
    match k.rsplit_once('#') {
        Some((b, n)) if !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) => b,
        _ => k,
    }
}

/// A key for one more demon of this kind and level: [`key_of`], or "<key>#2", "#3"... when the list already has one
/// (two Specter Lv 90s in one COMP are two demons; the list must keep both).
fn free_key(seen: &IndexMap<String, SeenDemon>, base: &str) -> String {
    if !seen.contains_key(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base}#{n}")).find(|k| !seen.contains_key(k)).unwrap_or_default()
}

/// The key a demon has in a list (`entries`: key and record): of the ones of its kind and level, the same object (this
/// session), else the first with the same details. None when it isn't in the list.
pub fn find_key<'a>(entries: impl Iterator<Item = (&'a String, &'a DemonRecord)>, d: &DemonRecord) -> Option<String> {
    let kind: Vec<(&String, &DemonRecord)> = entries.filter(|(_, x)| key_of(x) == key_of(d)).collect();
    kind.iter().find(|(_, x)| x.object_id == d.object_id).or_else(|| kind.iter().find(|(_, x)| same_details(x, d))).map(|(k, _)| (*k).clone())
}

/// The seen list as saved (cache\comp-demons.json), by the key each was saved under ("Key"; lists saved before
/// 2026-11-24 have none: [`key_of`], one per kind and level, the last of each). An unreadable file is an empty list.
pub fn load_seen(store: &Store) -> Result<IndexMap<String, SeenDemon>> {
    let mut res: IndexMap<String, SeenDemon> = IndexMap::new();
    let path = store.comp_demons_path();
    if path.exists() {
        let text = json::read_all_text(&path)?;
        // catch (JsonException) { }
        if let Ok(Value::Array(a)) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
            for e in &a {
                if let Some(s) = SeenDemon::from_value(e) {
                    // a saved key only counts when it is this demon's kind and level
                    let k = json::str(e.at("Key")).filter(|k| base_key(k) == key_of(&s.demon)).unwrap_or_else(|| key_of(&s.demon));
                    res.insert(k, s);
                }
            }
        }
    }
    Ok(res)
}

fn write_seen(store: &Store, seen: &IndexMap<String, SeenDemon>) -> Result<()> {
    std::fs::create_dir_all(store.cache_dir())?;
    let list = seen.iter().map(|(k, s)| {
        let mut v = s.to_value();
        if let Value::Object(m) = &mut v {
            m.insert("Key".into(), json!(k));
        }
        v
    });
    crate::data::json::write_atomic(store.comp_demons_path(), compact(&Value::Array(list.collect())))?;
    Ok(())
}

/// Add the demons just read from the game to the ones seen before (cache\comp-demons.json). Called by a snapshot, which
/// reads the demons too; the picker's "Load demons from the game" (`comp_demons`) does the same merge ([`merge_read`]).
pub fn note_demons(store: &Store, all: &[DemonRecord], comp: Option<&[DemonRecord]>, now: NaiveDateTime) -> Result<()> {
    let mut seen = load_seen(store)?;
    merge_read(&mut seen, all, comp, &iso_s(now));
    write_seen(store, &seen)
}

/// Merge one read of the game into the seen list. `all`: every demon record in memory (with or without details);
/// `comp`: the game's COMP slots ([`demon_reader::read_comp`]) when found. Returns the keys of the demons in this read.
///
/// With the COMP's slots the list is the COMP as it is now, one entry per slot: two demons of one kind and level are two
/// entries ("#2", [`free_key`]). Memory also holds stale copies and stored demons (on 2026-11-24 seven Specter records
/// for the two Specter Lv 90s in Rubito's COMP: copies with older object ids, and two other Specter Lv 90s that are not
/// in it); only the slots count. Most slots come without details (the game only lists them: DemonRecord.Loaded); for
/// those the details read earlier are kept when it is still the same demon (same object, else same kind and level, in
/// slot order). A demon read in full before that is not in the COMP now stays, marked as not in it (the user asked not
/// to lose what was read: it was moved to storage, and its numbers are still worth seeing), unless the COMP has one of
/// its kind and level.
///
/// Without them (the game holds only the summoned demon and stale copies) the demons with details update the entry that
/// is the same demon; one that matches none is added only when the list has none of its kind and level (or it is the
/// summoned one): with two of a kind listed, an unknown record can't be told from a stale copy.
///
/// Either way "summoned" is the game's mark at this read: a demon seen before that is not summoned in this read loses
/// it (the summoned demon is always in memory, with its details).
pub fn merge_read(seen: &mut IndexMap<String, SeenDemon>, all: &[DemonRecord], comp: Option<&[DemonRecord]>, now: &str) -> HashSet<String> {
    let mut live: HashSet<String> = HashSet::new();
    let fresh = |d: &DemonRecord, in_comp: Option<bool>| SeenDemon { seen_at: Some(now.to_string()), demon: d.clone(), in_comp };
    if let Some(comp) = comp {
        let before = std::mem::take(seen);
        let mut used: HashSet<String> = HashSet::new();
        for d in comp {
            let base = key_of(d);
            // the entry seen before for this slot: the same object, else the same details, else the next of its kind and level
            let old = before
                .iter()
                .filter(|(k, s)| !used.contains(*k) && key_of(&s.demon) == base)
                .min_by_key(|(_, s)| if s.demon.object_id == d.object_id { 0 } else if same_details(&s.demon, d) { 1 } else { 2 });
            let keep = !d.loaded() && old.is_some_and(|(_, s)| s.demon.loaded());
            let v = match old.filter(|_| keep) {
                Some((_, s)) => SeenDemon { in_comp: Some(true), ..s.clone() },
                None => fresh(d, Some(true)),
            };
            if let Some((k, _)) = old {
                used.insert(k.clone());
            }
            let k = free_key(seen, &base);
            seen.insert(k.clone(), v);
            live.insert(k);
        }
        let in_comp_now: HashSet<String> = comp.iter().map(key_of).collect();
        for (k, v) in &before {
            if !used.contains(k) && v.demon.loaded() && !in_comp_now.contains(&key_of(&v.demon)) {
                let nk = free_key(seen, &key_of(&v.demon));
                seen.insert(nk, SeenDemon { in_comp: Some(false), ..v.clone() });
            }
        }
    } else {
        let read: Vec<&DemonRecord> = all.iter().filter(|d| d.loaded()).collect();
        for d in &read {
            let base = key_of(d);
            let kind: Vec<String> = seen.keys().filter(|k| key_of(&seen[*k].demon) == base).cloned().collect();
            // demons of this kind and level in this read with different details (stale copies of one demon are one)
            let mut distinct: Vec<&DemonRecord> = Vec::new();
            for x in read.iter().filter(|x| key_of(x) == base) {
                if !distinct.iter().any(|y| same_details(x, y)) {
                    distinct.push(x);
                }
            }
            let hit = find_key(kind.iter().map(|k| (k, &seen[k].demon)), d)
                // one of its kind listed and one read: the same demon, its skills changed since
                .or_else(|| (kind.len() == 1 && distinct.len() == 1).then(|| kind[0].clone()));
            let k = match hit {
                Some(k) if live.contains(&k) && !d.summoned() => continue, // a stale copy of a demon already updated
                Some(k) => k,
                None if kind.is_empty() || d.summoned() => free_key(seen, &base),
                None => continue,
            };
            let in_comp = if d.summoned() { Some(true) } else { seen.get(&k).and_then(|s| s.in_comp) };
            seen.insert(k.clone(), fresh(d, in_comp));
            live.insert(k);
        }
    }
    let read: Vec<&DemonRecord> = all.iter().filter(|d| d.loaded()).collect();
    only_summoned(seen, &read);
    live
}

/// Take the summoned mark off every demon seen that this read of the game doesn't have as summoned.
fn only_summoned(seen: &mut IndexMap<String, SeenDemon>, read: &[&DemonRecord]) {
    for s in seen.values_mut() {
        if s.demon.summoned() && !read.iter().any(|d| d.loaded() && d.summoned() && same_demon(d, &s.demon)) {
            s.demon.flags &= !0x02;
        }
    }
}

/// `new Pipeline(dataDir, data, blankSets, blankIcons)`.
pub struct Pipeline {
    /// The store (`ctx.store`) and the reference data (`ctx.data`) of the character this pipeline works on.
    pub ctx: Arc<Context>,
    dash: Dashboard,
    exp: Expertise,
    /// The Simulator tool: what another build would give (nothing is read from the game or written). Shared between
    /// threads (it takes its own lock, one call at a time).
    pub sim: Simulator,
}

impl Pipeline {
    /// `data_dir`: one character's folder (see data\characters.rs). `data`: reference data to share between characters;
    /// loaded here when not given. `blank_sets`: given = a pipeline with no character behind it, for the Simulator's "no
    /// base" builds (data_dir is then a folder that need not exist; `blank_icons` is a character's store whose icon
    /// cache is used).
    pub fn new(data_dir: impl Into<PathBuf>, data: Option<Arc<GameData>>, blank_sets: Option<BlankSets>, blank_icons: Option<Store>) -> Pipeline {
        let data = data.unwrap_or_else(|| Arc::new(GameData::new()));
        let ctx = Arc::new(Context::new(Store::new(data_dir), data));
        let sim = Simulator::new(Arc::clone(&ctx), blank_sets, blank_icons);
        Pipeline { dash: Dashboard::new(Arc::clone(&ctx)), exp: Expertise::new(Arc::clone(&ctx)), ctx, sim }
    }

    pub fn store(&self) -> &Store {
        &self.ctx.store
    }

    pub fn data(&self) -> &Arc<GameData> {
        &self.ctx.data
    }

    /// The dashboard builder (the Simulator uses its `parts` and `bonus_rows`).
    pub fn dash(&self) -> &Dashboard {
        &self.dash
    }

    /// The expertise builder (the Simulator uses its `rows` and `chain_ranks`).
    pub fn exp(&self) -> &Expertise {
        &self.exp
    }

    /// Read-only snapshot of the running client: appends a CSV row and refreshes the expertise ranks.
    pub fn take_snapshot(&self, name: &str, csv: &str, ignore_window: bool) -> Result<(SnapshotResult, usize)> {
        // The CSV is written last; check it first, so a CSV open in Excel stops the snapshot before anything is saved
        // (else the caches and the gear page would move on to a snapshot that has no CSV row).
        if !csv.is_empty() {
            json::ensure_writable(csv)?;
        }
        // Expertise first: gear lines like "If player has Class 4 Rapid or more" are decided from the ranks, so the
        // snapshot must see this moment's ranks (a new character has none saved yet), not the previous snapshot's.
        // Alignment from the game (LAW/NEUTRAL/CHAOS conditions on gear); a change recalculates the saved rows
        // All the searches below walk one copy of the game's memory (GameMemory.ShareSweeps): about a dozen otherwise.
        let _shared = memory::share_sweeps();
        let align = log::timed("snapshot: alignment", || Pipeline::read_alignment(name), |a| match a { Ok(Some(_)) => "read".into(), Ok(None) => "not read".into(), Err(_) => "failed".into() })?;
        if let Some(align) = align {
            if align != self.store().alignment()? || !self.store().profile_path().exists() {
                self.set_alignment(csv, &align)?;
            }
        }
        let ranks = log::timed("snapshot: expertise ranks", || self.exp.read(name), |r| match r { Ok(r) => format!("found, {} ranked", r.len()), Err(_) => "failed".into() })?;
        let r = Snapshot::new(Arc::clone(&self.ctx)).take(name, csv, ignore_window)?;
        Ok((r, ranks.len()))
    }

    /// The character's alignment as the game has it right now (`client_reader::read_lnc`), or None when it can't be read.
    pub fn read_alignment(name: &str) -> Result<Option<String>> {
        match client_reader::find_client(name) {
            Ok(p) => {
                let mem = GameMemory::new(p.id())?;
                Ok(client_reader::read_lnc(&mem).map(|v| client_reader::lnc_name(v).to_string()))
            }
            Err(e) if is_user_error(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn recalc(&self, csv: &str) -> Result<Vec<String>> {
        Snapshot::new(Arc::clone(&self.ctx)).recalc_csv(csv)
    }

    /// Set the character's alignment (LAW / NEUTRAL / CHAOS), which decides gear lines like "If Player is LAW ...".
    /// Snapshots call it with the game's own value (`read_alignment`); a person can also set it by hand.
    /// When it changes and there are saved snapshots, every CSV row's gear columns are recomputed and the page data is
    /// rebuilt; the CSV is copied first (kept only if a number actually changed).
    pub fn set_alignment(&self, csv: &str, alignment: &str) -> Result<AlignmentResult> {
        let alignment = alignment.trim().to_uppercase();
        if !matches!(alignment.as_str(), "LAW" | "NEUTRAL" | "CHAOS") {
            return Err(user_error("Alignment must be LAW, NEUTRAL or CHAOS."));
        }
        let store = self.store();
        let was_saved = store.profile_path().exists();
        let same = store.alignment()? == alignment;
        let none = |changed: bool| AlignmentResult { changed, rows: 0, changed_rows: 0, backup: None, log: Vec::new() };
        if was_saved && same {
            return Ok(none(false));
        }
        store.save_alignment(Some(&alignment))?;
        if same {
            return Ok(none(false)); // it was already being treated as this one
        }
        if !Path::new(csv).exists() || CsvTable::load(csv)?.rows.is_empty() {
            return Ok(none(true));
        }
        let stem = Path::new(csv).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = Path::new(csv).parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let backup = dir.join(format!("{stem}.backup-before-alignment-{}.csv", Local::now().format("%Y%m%d-%H%M%S")));
        if backup.exists() {
            return Err(anyhow::anyhow!("The file '{}' already exists.", backup.display()));
        }
        std::fs::copy(csv, &backup)?;
        let log = self.recalc(csv)?;
        self.rebuild(csv)?;
        let changed = log.iter().filter(|l| !l.ends_with(": no change")).count();
        let backup = if changed == 0 {
            std::fs::remove_file(&backup)?;
            None
        } else {
            Some(backup.to_string_lossy().into_owned())
        };
        Ok(AlignmentResult { changed: true, rows: log.len(), changed_rows: changed, backup, log })
    }

    // ---------- choosing the digitalize demon ----------

    /// The same for a demon type, with the race levels last read (for the command line's demon list).
    pub fn unlock_for(&self, r#type: i32) -> Result<Option<DemonUnlock>> {
        let saved = self.saved_race_levels()?;
        self.unlock_of(self.data().demons()?.devils.get(&r#type), saved.as_ref())
    }

    fn unlock_of(&self, devil: Option<&Devil>, race_levels: Option<&HashMap<i32, i32>>) -> Result<Option<DemonUnlock>> {
        let Some(devil) = devil else { return Ok(None) };
        let t = self.data().demons()?;
        let mine = race_levels.map(|m| m.get(&devil.race_id).copied().unwrap_or(0));
        let at = match t.levels.get(&devil.race_id) {
            Some(lv) => lv
                .iter()
                .filter(|l| l.level >= 1 && l.demon_ids.iter().any(|id| t.devils.get(id).is_some_and(|d| d.name == devil.name)))
                .map(|l| l.level)
                .min()
                .unwrap_or(0),
            None => 0,
        };
        Ok(Some(DemonUnlock { listed: at > 0, level: at, race_level: mine, race: title(&devil.race) }))
    }

    /// The character's Digitalization Level per race as last read (expertise-ranks.json), None when never read.
    fn saved_race_levels(&self) -> Result<Option<HashMap<i32, i32>>> {
        let path = self.store().expertise_ranks_path();
        if !path.exists() {
            return Ok(None);
        }
        // catch (JsonException or IOException or InvalidOperationException or FormatException or ArgumentException) { return null; }
        let Ok(text) = json::read_all_text(&path) else { return Ok(None) };
        let Ok(doc) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else { return Ok(None) };
        let Some(arr) = doc.at("digitalize_levels").and_then(Value::as_array) else { return Ok(None) };
        let mut res = HashMap::new();
        for e in arr.iter().filter(|e| !e.is_null()) {
            let (Some(r), Some(l)) = (json::num(e.at("race_id")), json::num(e.at("level"))) else { return Ok(None) };
            if res.insert(r as i32, l as i32).is_some() {
                return Ok(None); // ToDictionary on a repeated key
            }
        }
        Ok(Some(res))
    }

    /// The COMP demons to pick the digitalize demon from: every one the game has in memory now (all of them while the COMP
    /// or Demon List is open, otherwise only the summoned one), plus every one seen before (cache\comp-demons.json, kept
    /// up to date here). Works without the game from what was seen. The current digitalize demon is always included.
    /// With `scan` false the game isn't looked at: only the demons seen before, at once (the scan of the game's memory
    /// takes a second or two), so a list can be shown while the scan runs.
    pub fn comp_demons(&self, name: &str, scan: bool) -> Result<Vec<CompDemon>> {
        let list = self.list_demons(Some(name), scan, false)?;
        if scan {
            self.save_summoned(list.iter().find(|c| c.summoned))?; // the read may have changed who is summoned
        }
        Ok(list)
    }

    /// The summoned demon as the page shows it on the Overview (dash\summoned.json: one CompDemon, or null), so the card
    /// is there with the rest of the page data instead of after a request for every demon. Written whenever it can
    /// change: a rebuild (a snapshot ends with one) and a read of the demons for the picker.
    fn save_summoned(&self, d: Option<&CompDemon>) -> Result<()> {
        let store = self.store();
        std::fs::create_dir_all(store.dash_dir())?;
        let v = d.map(CompDemon::to_value).unwrap_or(Value::Null);
        crate::data::json::write_atomic(store.dash_dir().join("summoned.json"), compact(&v))?;
        Ok(())
    }

    fn list_demons(&self, name: Option<&str>, scan: bool, only_summoned_list: bool) -> Result<Vec<CompDemon>> {
        let store = self.store();
        let data = self.data();
        let mut seen = load_seen(store)?;
        let mut live: HashSet<String> = HashSet::new();
        let mut pid: u32 = 0;
        let now = iso_s(Local::now().naive_local());
        if scan {
            let proc = match client_reader::find_client(name.unwrap_or("")) {
                Ok(p) => Some(p),
                Err(e) if is_user_error(&e) => None, // the game isn't running: what was seen before
                Err(e) => return Err(e),
            };
            if let Some(proc) = proc {
                pid = proc.id();
                let _shared = memory::share_sweeps(); // demons, COMP slots and progress from one copy of memory
                let mem = GameMemory::new(proc.id())?;
                let all = demon_reader::read_demons(&mem, true);
                // the COMP as the game lists its slots when that list is in memory, else the demons with details (merge_read)
                let comp = demon_reader::read_comp(&mem, &all);
                live = merge_read(&mut seen, &all, comp.as_deref(), &now);
                // the learned assists and the Mitama boost too, so switching demons afterwards needs nothing from the game
                DigitalizeSave::save_progress(store, data, demon_reader::read_progress(&mem).as_ref(), Local::now().naive_local())?;
            }
        }
        let current: Option<DemonRecord> = DigitalizeSave::load(store)?.and_then(|s| s.demon);
        if scan {
            write_seen(store, &seen)?;
        }
        // The saved digitalize demon is the entry that is the same demon (find_key: same object, else same details), not
        // any of its kind and level: with two Specter Lv 90s in the COMP the other one is not in use. When none is (never
        // read into the list): listed, not saved with the rest.
        const SAVED_KEY: &str = "|saved";
        let current_key = current.as_ref().and_then(|c| find_key(seen.iter().map(|(k, s)| (k, &s.demon)), c));
        if let Some(c) = current.as_ref().filter(|_| current_key.is_none()) {
            // not read into the list: not in the COMP. Others of its kind listed: the list says nothing about this one.
            let in_comp = if seen.values().any(|s| key_of(&s.demon) == key_of(c)) { None } else { Some(false) };
            seen.insert(format!("{}{SAVED_KEY}", key_of(c)), SeenDemon { seen_at: None, demon: c.clone(), in_comp });
        }

        let t = data.demons()?;
        // what each would give: with the assists and the Mitama boost last read (they belong to the character, not the demon)
        let saved_now = DigitalizeSave::load(store)?;
        let prog = DigitalizeSave::load_progress(store);
        let assists: Option<Vec<i32>> = prog.as_ref().and_then(|p| p.assists.clone()).or_else(|| saved_now.as_ref().and_then(|s| s.assists.clone()));
        let boost = match &prog {
            Some(p) => p.mitama_boost,
            None => saved_now.as_ref().is_some_and(|s| s.mitama_boost),
        };
        let race_levels = self.saved_race_levels()?;
        let mut list: Vec<CompDemon> = Vec::new();
        for (k, s) in seen.iter().filter(|(k, s)| !only_summoned_list || (s.demon.summoned() && !k.ends_with(SAVED_KEY))) {
            let d = &s.demon;
            let devil = t.devils.get(&d.r#type);
            let icon = DigitalizeSave::icon(store, data, pid, d.r#type)?;
            list.push(CompDemon {
                key: k.clone(),
                name: d.name.clone(),
                r#type: d.r#type,
                level: d.level,
                race: devil.map(|v| title(&v.race)).unwrap_or_default(),
                mitama: if d.mitama_type > 0 && (d.mitama_type as usize) < MITAMA_NAMES.len() { MITAMA_NAMES[d.mitama_type as usize].to_string() } else { String::new() },
                current: k.ends_with(SAVED_KEY) || current_key.as_ref() == Some(k),
                in_game: live.contains(k),
                seen_at: s.seen_at.clone(),
                force_stack: d.force_stack.as_ref().map_or(0, |f| f.iter().filter(|&&x| x > 0).count()),
                equipment: d.equipment.as_ref().is_some_and(|e| e.iter().any(|&x| x > 0)),
                icon: icon.map(|p| format!("data:image/png;base64,{}", base64(&p))),
                // nothing to work it out from when the game only lists the demon
                offer: if d.loaded() { self.dash.offer(d, assists.as_deref(), boost)? } else { None },
                unlock: self.unlock_of(devil, race_levels.as_ref())?,
                loaded: d.loaded(),
                summoned: !k.ends_with(SAVED_KEY) && d.summoned(),
                in_comp: s.in_comp,
            });
        }
        // OrderByDescending(Current).ThenByDescending(InComp != false).ThenByDescending(Level).ThenBy(Name): stable
        list.sort_by(|a, b| {
            b.current
                .cmp(&a.current)
                .then((b.in_comp != Some(false)).cmp(&(a.in_comp != Some(false))))
                .then(b.level.cmp(&a.level))
                .then_with(|| crate::text::cmp_invariant(&a.name, &b.name))
        });
        Ok(list)
    }

    /// Make a COMP demon (key from `comp_demons`) the digitalize demon: the saved demon (cache\digitalize.json), the latest
    /// CSV row's demon columns (the CSV is copied first) and the page data. Assists and the Mitama boost are read from the
    /// game when it runs, else kept from the saved demon. A snapshot taken while digitalized switches to the demon in use.
    /// With `scan` false nothing is read from the game (reading the demons and the progress takes about a second, the
    /// rest a tenth of that): the demon is the one in the seen list, the assists and the Mitama boost the ones last read
    /// (cache\digitalize-progress.json, written by snapshots and by `comp_demons` with a scan).
    pub fn use_demon(&self, name: &str, csv: &str, key: Option<&str>, scan: bool) -> Result<DemonSwitch> {
        let store = self.store();
        let data = self.data();
        let seen = load_seen(store)?;
        let Some(s) = key.and_then(|k| seen.get(k)) else {
            return Err(user_error("That demon isn't in the list. Open the COMP in game and look again."));
        };
        let key = key.unwrap_or_default();
        let saved = DigitalizeSave::load(store)?;
        let mut demon = s.demon.clone();
        let mut assists: Option<Vec<i32>> = saved.as_ref().and_then(|s| s.assists.clone());
        let mut boost = saved.as_ref().is_some_and(|s| s.mitama_boost);
        let mut active = false;
        let mut pid: u32 = 0;
        if !scan {
            if let Some(p) = DigitalizeSave::load_progress(store).filter(|p| p.assists.is_some()) {
                assists = p.assists;
                boost = p.mitama_boost;
            }
        } else {
            match client_reader::find_client(name) {
                Ok(proc) => {
                    pid = proc.id();
                    let _shared = memory::share_sweeps(); // demons and progress from one copy of memory
                    let mem = GameMemory::new(proc.id())?;
                    let matches: Vec<DemonRecord> = demon_reader::read_demons(&mem, false).into_iter().filter(|d| key_of(d) == base_key(key)).collect();
                    // the same object, else the same details; any other of its kind and level only when it is the only one
                    // (two Specter Lv 90s in one COMP are two demons, and memory holds stale copies)
                    let pick = matches
                        .iter()
                        .find(|d| d.object_id == s.demon.object_id)
                        .or_else(|| matches.iter().find(|d| same_details(d, &s.demon)))
                        .or_else(|| (matches.len() == 1).then(|| &matches[0]));
                    if let Some(d) = pick {
                        demon = d.clone();
                    }
                    let progress = demon_reader::read_progress(&mem);
                    if let Some(p) = &progress {
                        assists = Some(p.assists.clone());
                        if p.skills.is_some() {
                            boost = p.has_skill(data.mitama_set_boost_skill()?);
                        }
                    }
                    active = progress.as_ref().is_some_and(|p| p.digitalized() && p.digitalized_demon == demon.object_id);
                }
                Err(e) if is_user_error(&e) => {}
                Err(e) => return Err(e),
            }
        }
        // a demon the game has only listed has no stats, skills, growth or Mitama to work from
        if !demon.loaded() {
            return Err(user_error("That demon hasn't been read yet: the game only loads a demon's stats and skills when you look at it. Click it in the COMP in game, or open the fusion window at the Cathedral of Shadows (it loads the whole COMP), then load the demons again."));
        }
        let changed = saved.as_ref().and_then(|s| s.demon.as_ref()).is_none_or(|d| !same_demon(d, &demon));
        let save = DigitalizeSave::new(Some(iso_s(Local::now().naive_local())), active, demon.clone(), assists, boost);
        save.save(store)?;
        if let Some(png) = DigitalizeSave::icon(store, data, pid, demon.r#type)? {
            std::fs::create_dir_all(store.dash_dir())?;
            std::fs::write(store.demon_picture_path(), png)?;
        }

        let mut backup: Option<PathBuf> = None;
        let mut row = false;
        if Path::new(csv).exists() && !CsvTable::load(csv)?.rows.is_empty() {
            let stem = Path::new(csv).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let dir = Path::new(csv).parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
            let b = dir.join(format!("{stem}.backup-before-demon-switch.csv"));
            std::fs::copy(csv, &b)?;
            row = Snapshot::new(Arc::clone(&self.ctx)).put_demon_on_latest_row(csv, &save)?;
            self.rebuild(csv)?;
            backup = Some(b);
        }
        Ok(DemonSwitch {
            name: demon.name.clone(),
            changed,
            row_updated: row,
            backup: backup.map(|b| b.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
        })
    }

    /// Rebuild data\dash\*.json from the CSV and the saved files (no game needed).
    pub fn rebuild(&self, csv: &str) -> Result<Counts> {
        let snaps = self.dash.build_snapshots(csv)?;
        let gear = self.dash.build_gear(csv)?;
        // snaps.OrderBy(taken_at, Ordinal).Last(): the largest, the last of equals
        let mut latest: Option<&json::Row> = None;
        for s in &snaps {
            let key = |r: &json::Row| r.get("taken_at").and_then(Value::as_str).unwrap_or("").to_string();
            if latest.is_none_or(|l| key(s) >= key(l)) {
                latest = Some(s);
            }
        }
        let latest = latest.ok_or_else(|| anyhow::anyhow!("Sequence contains no elements"))?;
        let latest = Value::Object(latest.clone());
        let exp_rows = self.exp.build(Some(&latest))?;
        let bonuses = self.dash.build_bonuses(&exp_rows)?;
        self.dash.build_unlocks()?;
        let summoned = self.list_demons(None, false, true)?;
        self.save_summoned(summoned.first())?;
        Ok(Counts { snapshots: snaps.len(), gear: gear.len(), expertise: exp_rows.len(), bonuses: bonuses.len() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demon(name: &str, flags: i32) -> DemonRecord {
        DemonRecord { name: name.into(), r#type: 12, level: 5, flags, live_value: vec![1; 21], acquired: vec![1], ..Default::default() }
    }

    #[test]
    fn keys_and_unlocks() {
        assert_eq!(key_of(&demon("Pixie", 0)), "12:Pixie:5");
        let u = DemonUnlock { listed: true, level: 3, race_level: Some(3), race: "Fairy".into() };
        assert!(u.reached());
        assert!(!DemonUnlock { race_level: Some(2), ..u.clone() }.reached());
        assert!(!DemonUnlock { race_level: None, ..u.clone() }.reached());
        assert!(!DemonUnlock { listed: false, ..u.clone() }.reached());
        assert_eq!(u.to_value().to_string(), r#"{"listed":true,"level":3,"raceLevel":3,"race":"Fairy","reached":true}"#);
    }

    #[test]
    fn only_summoned_clears_the_mark() {
        let mut seen: IndexMap<String, SeenDemon> = IndexMap::new();
        let a = demon("A", 0x33);
        let b = demon("B", 0x33);
        seen.insert(key_of(&a), SeenDemon { seen_at: None, demon: a.clone(), in_comp: None });
        seen.insert(key_of(&b), SeenDemon { seen_at: None, demon: b.clone(), in_comp: None });
        only_summoned(&mut seen, &[&a]);
        assert_eq!(seen[&key_of(&a)].demon.flags, 0x33);
        assert_eq!(seen[&key_of(&b)].demon.flags, 0x31);
    }

    #[test]
    fn seen_list_round_trips() {
        let dir = std::env::temp_dir().join(format!("newmoon-seen-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::new(&dir);
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 24).unwrap().and_hms_opt(1, 2, 3).unwrap();
        note_demons(&store, &[demon("Pixie", 0x33), DemonRecord { name: "Not loaded".into(), ..Default::default() }], None, now).unwrap();
        let text = std::fs::read_to_string(store.comp_demons_path()).unwrap();
        assert!(text.starts_with(r#"[{"SeenAt":"2026-11-24T01:02:03","Demon":{"Address":0,"Flags":51,"Name":"Pixie""#), "{text}");
        let seen = load_seen(&store).unwrap();
        assert_eq!(seen.len(), 1); // a demon the game only lists is not kept
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn rec(obj: u32, skills: &[i32], flags: i32) -> DemonRecord {
        DemonRecord { object_id: obj, skills: skills.to_vec(), ..demon("Specter", flags) }
    }

    fn only_listed(d: &DemonRecord) -> DemonRecord {
        DemonRecord { live_value: vec![0; 21], acquired: vec![], skills: vec![], ..d.clone() }
    }

    /// Rubito's COMP on 2026-11-24: two Specter Lv 90s (2181060 with Shot Booster, 2181061 with Secret of
    /// Shock・Deathblow), a stale copy of 2181060 (2181056) and another Specter Lv 90 not in the COMP (2181058).
    #[test]
    fn two_of_a_kind_in_the_comp_are_two_entries() {
        let shot = rec(2181060, &[2, 3, 4, 585], 0x31);
        let shock = rec(2181061, &[2, 3, 4, 740], 0x31);
        let copy = rec(2181056, &[2, 3, 4, 585], 0x31);
        let other = rec(2181058, &[48, 49, 586], 0x31);
        let all = vec![copy.clone(), other.clone(), shot.clone(), shock.clone()];
        let mut seen: IndexMap<String, SeenDemon> = IndexMap::new();
        let live = merge_read(&mut seen, &all, Some(&[shot.clone(), shock.clone()]), "now");
        assert_eq!(seen.keys().cloned().collect::<Vec<_>>(), ["12:Specter:5", "12:Specter:5#2"]);
        assert_eq!(live.len(), 2);
        assert_eq!(seen["12:Specter:5"].demon.object_id, 2181060);
        assert_eq!(seen["12:Specter:5#2"].demon.object_id, 2181061);
        assert_eq!(base_key("12:Specter:5#2"), "12:Specter:5");
        assert_eq!(base_key("12:Specter:5#2|saved"), "12:Specter:5");
        // the saved demon is found by what it is, not by its kind and level
        let entries = || seen.iter().map(|(k, s)| (k, &s.demon));
        assert_eq!(find_key(entries(), &shock).as_deref(), Some("12:Specter:5#2"));
        assert_eq!(find_key(entries(), &DemonRecord { object_id: 7, ..shock.clone() }).as_deref(), Some("12:Specter:5#2")); // next session
        assert_eq!(find_key(entries(), &other), None);

        // the COMP listed again without details: each slot keeps its own details
        let live = merge_read(&mut seen, &all, Some(&[only_listed(&shot), only_listed(&shock)]), "later");
        assert_eq!(live.len(), 2);
        assert_eq!(seen["12:Specter:5"].demon.skills, shot.skills);
        assert_eq!(seen["12:Specter:5#2"].demon.skills, shock.skills);

        // no slot list (only stale copies and the summoned one in memory): nothing is added or mixed up
        merge_read(&mut seen, &[copy.clone(), other.clone()], None, "after");
        assert_eq!(seen.len(), 2);
        assert_eq!(seen["12:Specter:5"].demon.skills, shot.skills);
        assert_eq!(seen["12:Specter:5#2"].demon.skills, shock.skills);
    }

    #[test]
    fn keys_survive_the_file() {
        let dir = std::env::temp_dir().join(format!("newmoon-seen-keys-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::new(&dir);
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 24).unwrap().and_hms_opt(4, 0, 9).unwrap();
        let (a, b) = (rec(1, &[1], 0x31), rec(2, &[2], 0x31));
        note_demons(&store, &[a.clone(), b.clone()], Some(&[a.clone(), b.clone()]), now).unwrap();
        let seen = load_seen(&store).unwrap();
        assert_eq!(seen["12:Specter:5"].demon.object_id, 1);
        assert_eq!(seen["12:Specter:5#2"].demon.object_id, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
