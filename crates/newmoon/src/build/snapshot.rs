//! Port of `src/Build/Snapshot.cs`.
//!
//! One snapshot of a character's stats and gear, read-only, from the running client.
//!  * Information window open  -> everything live; refreshes the cache (last stats, tooltip text per item id).
//!  * Information window closed -> gear from the gear grid, level from the nameplate, the 12 stats and HP/MP from the
//!                                 last window snapshot, gear bonuses from cached tooltips.

use std::sync::Arc;

use chrono::{Local, NaiveDateTime};
use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::Result;
use crate::calc::bonuses::{self, CombatRow};
use crate::calc::digitalize::DigitalizeResult;
use crate::calc::gear_effects::GearResult;
use crate::calc::tooltip_parser::TooltipParser;
use crate::context::Context;
use crate::data::json::{self, At};
use crate::data::store::{CachedTip, Store};
use crate::diag::log;
use crate::fmt;
use crate::game::{WornItem, client_reader, demon_reader};
use crate::memory::GameMemory;
use crate::user_error;

use super::csv::CsvTable;
use super::dashboard::parse_datetime;
use super::digitalize_demon::{DigitalizeSave, iso_s};

pub const DEMON_COLUMN: &str = "Digitalize demon";
pub const DEMON_RECORD_COLUMN: &str = "Digitalize demon record";
pub const GEAR_IDS_COLUMN: &str = "Gear ids";
pub const GEAR_RECORDS_COLUMN: &str = "Gear records";

/// `DemonCol(stat)`: the CSV column of what the digitalize demon adds to a stat.
pub fn demon_col(stat: &str) -> String {
    format!("{stat} (demon, digitalized)")
}

/// `Snapshot.Result`.
pub struct SnapshotResult {
    pub source: String,
    pub stats_note: String,
    pub missing: String,
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub pairs: Vec<i32>,
    pub items: Vec<i32>,
    pub texts: Vec<String>,
    pub gear: GearResult,
    pub combat: Vec<CombatRow>,
    pub csv_row: IndexMap<String, String>,
    /// The digitalize demon and what it adds per tracked combat stat (None / empty without one).
    pub demon: Option<DigitalizeSave>,
    pub digitalize_result: Option<DigitalizeResult>,
    pub demon_combat: IndexMap<String, f64>,
    /// Taken while digitalized: the window's stats were skipped.
    pub while_digitalized: bool,
}

/// `Inv(v)`: `v.ToString(InvariantCulture)`.
fn inv(v: f64) -> String {
    fmt::g(v)
}

/// The demon's name and record, and what it adds per combat stat, as CSV columns.
fn put_demon(snap: &mut IndexMap<String, String>, d: Option<&DigitalizeSave>, combat: &IndexMap<String, f64>) {
    snap.insert(DEMON_COLUMN.into(), d.and_then(|d| d.demon.as_ref()).map(|d| d.name.clone()).unwrap_or_default());
    for (stat, _) in bonuses::STAT_MAP {
        snap.insert(demon_col(stat), inv(combat.get(*stat).copied().unwrap_or(0.0)));
    }
    snap.insert(DEMON_RECORD_COLUMN.into(), d.map(|d| d.pack()).unwrap_or_default());
}

/// The CSV's name for slot k ("Gear: Weapon" ... "Gear: Extra"), the same as the item list's slot names.
fn slot_label(k: usize) -> String {
    if client_reader::SLOT_NAMES[k] == "COMP" { "Comp".to_string() } else { client_reader::SLOT_NAMES[k].to_string() }
}

/// `File.GetLastWriteTime` as a local time.
fn last_write_time(path: &std::path::Path) -> Option<NaiveDateTime> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(chrono::DateTime::<Local>::from(t).naive_local())
}

/// `new Snapshot(store, data, ctx)`: the store and the data are the context's.
pub struct Snapshot {
    ctx: Arc<Context>,
}

impl Snapshot {
    pub fn new(ctx: Arc<Context>) -> Snapshot {
        Snapshot { ctx }
    }

    /// Make d the digitalize demon of the latest CSV row (the row the page's Digitalized view adds the demon to), with
    /// what it adds per combat stat worked out at that row's level. Older rows keep the demon they were taken with.
    /// False when the CSV has no rows.
    pub fn put_demon_on_latest_row(&self, csv_path: &str, d: &DigitalizeSave) -> Result<bool> {
        if !std::path::Path::new(csv_path).exists() {
            return Ok(false);
        }
        let mut t = CsvTable::load(csv_path)?;
        if t.rows.is_empty() {
            return Ok(false);
        }
        let last = t.rows.len() - 1;
        let lvl = fmt::parse_i32(&t.get(&t.rows[last], "Level")).unwrap_or(0);
        let combat = bonuses::demon_summary(&d.compute(&self.ctx, if lvl > 0 { lvl } else { 99 })?.effects);
        let name = d.demon.as_ref().map(|d| d.name.clone()).ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
        let mut sets: Vec<(String, String)> = vec![(DEMON_COLUMN.into(), name)];
        for (stat, _) in bonuses::STAT_MAP {
            sets.push((demon_col(stat), inv(combat.get(*stat).copied().unwrap_or(0.0))));
        }
        sets.push((DEMON_RECORD_COLUMN.into(), d.pack()));
        for (col, v) in sets {
            if !t.columns.contains(&col) {
                t.columns.push(col.clone());
            }
            t.rows[last].insert(col, v);
        }
        t.save(csv_path)?;
        Ok(true)
    }

    pub fn take(&self, name: &str, csv_path: &str, ignore_window: bool) -> Result<SnapshotResult> {
        let store: &Store = &self.ctx.store;
        let data = &*self.ctx.data;
        let proc = client_reader::find_client(name)?;
        let mem = GameMemory::new(proc.id())?;
        let started = std::time::Instant::now();
        crate::log_info!("snapshot: reading client pid {} ({}), game memory opened read-only", proc.id(), proc.folder);
        std::fs::create_dir_all(store.cache_dir())?;
        let now = Local::now().naive_local();

        let known_items = data.items()?;
        let mut win = log::timed("snapshot: Information window", || client_reader::read_info_window(&mem, name, |id| known_items.contains_key(&id)), |w| if w.is_some() { "open".into() } else { "not open".into() });
        // A closed window's block can linger; only trust it if its items match the live gear grid.
        let grid = log::timed("snapshot: worn gear", || client_reader::live_gear(&mem), |g| match g {
            Some(g) => format!("found, {} of 15 slots worn, item records {}", g.ids.iter().filter(|&&i| i > 0).count(), if g.details.is_some() { "read" } else { "not read" }),
            None => "not found".into(),
        });
        let live: Option<Vec<i32>> = grid.as_ref().map(|g| g.ids.clone());
        if let (Some(w), Some(l)) = (&win, &live) {
            if w.items != *l {
                win = None;
            }
        }
        if ignore_window {
            win = None;
        }

        // Game's equipment-set table (static game data): re-read once per client start.
        if !store.set_table_path().exists() || last_write_time(&store.set_table_path()).is_none_or(|t| t < proc.started) {
            let sets = log::timed("snapshot: equipment-set table", || client_reader::read_set_table(&mem), |t| match t {
                Some(t) => format!("found, {} sets", t.len()),
                None => "not found".into(),
            });
            if let Some(table) = sets {
                store.save_set_table(&table)?;
            }
        }

        // The digitalize demon (the one in use now, else the saved one). While digitalized, the window's stats include
        // the demon, so they're not taken as the character's own: the saved ones are used instead.
        let digi = log::timed("snapshot: digitalize demon", || DigitalizeSave::read(&mem, store, data, now), |r| match r {
            Ok(Some(d)) => format!("found, {}", if d.digitalized_when_taken { "digitalized now" } else { "not digitalized" }),
            Ok(None) => "no demon to go by".into(),
            Err(_) => "failed".into(),
        })?;
        if let Some(d) = &digi {
            d.save(store)?;
            if let Some(dm) = &d.demon {
                DigitalizeSave::save_icon(store, data, proc.id(), dm.r#type)?;
            }
        }
        let mut tips = store.load_tooltip_cache()?;
        // Digitalized or not is the game's own flag (the progress object's digitalized demon), not whether that demon's
        // record was found: when the record isn't loaded, the saved demon stands in and its object id doesn't match,
        // and the demon-inflated stats would be saved as the character's own.
        let digitalized_when_taken =
            digi.as_ref().is_some_and(|d| d.digitalized_when_taken) || demon_reader::read_progress(&mem).is_some_and(|p| p.digitalized());
        let demons = data.demons()?;
        // The toggles switched on when the stats are read: their percent Max HP/MP is part of those numbers, so the saved
        // stats keep them (CharacterHpMp calibration).
        let toggles_on = || -> Value {
            match demon_reader::read_progress(&mem).and_then(|p| p.switched_on) {
                Some(on) => {
                    let mut v: Vec<i32> = on.into_iter().filter(|id| demons.skills.get(id).is_some_and(|s| s.0 == 2)).collect();
                    v.sort();
                    json!(v)
                }
                None => Value::Null,
            }
        };
        let seen_at = iso_s(now);
        let put_tip = |tips: &mut IndexMap<i32, CachedTip>, items: &[i32], texts: &[String]| {
            for k in 0..15 {
                if items[k] > 0 && !texts[k].is_empty() {
                    tips.insert(items[k], CachedTip { text: Some(texts[k].clone()), seen_at: Some(seen_at.clone()) });
                }
            }
        };

        let mut r = SnapshotResult {
            source: String::new(),
            stats_note: String::new(),
            missing: String::new(),
            level: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            pairs: Vec::new(),
            items: Vec::new(),
            texts: Vec::new(),
            gear: GearResult::default(),
            combat: Vec::new(),
            csv_row: IndexMap::new(),
            demon: digi.clone(),
            digitalize_result: None,
            demon_combat: IndexMap::new(),
            while_digitalized: false,
        };
        let stats_from: NaiveDateTime;
        if win.is_some() && digitalized_when_taken {
            // Item tooltips are still good; only the 12 stats and HP/MP are left alone. With no saved stats yet, the
            // cache branch below refuses ("take the first snapshot when not digitalized") rather than saving the
            // window's digitalized stats as the character's own.
            let w = win.as_ref().expect("window");
            put_tip(&mut tips, &w.items, &w.texts);
            store.save_tooltip_cache(&tips)?;
            win = None;
            r.while_digitalized = true;
        }
        let own_status = if win.is_none() && !digitalized_when_taken && live.is_some() {
            log::timed("snapshot: own stats", || client_reader::read_own_status(&mem, false), |s| if s.is_some() { "found".into() } else { "not found".into() })
        } else {
            None
        };
        if let Some(win) = &win {
            r.source = "window".into();
            stats_from = now;
            r.level = win.level;
            r.hp = win.hp;
            r.max_hp = win.max_hp;
            r.mp = win.mp;
            r.max_mp = win.max_mp;
            r.pairs = win.pairs.clone();
            r.items = win.items.clone();
            json::save(
                store.last_stats_path(),
                &json!({
                    "taken_at": iso_s(now), "level": win.level, "hp": win.hp, "max_hp": win.max_hp, "mp": win.mp, "max_mp": win.max_mp,
                    "pairs": win.pairs, "toggles_on": toggles_on(),
                }),
            )?;
            put_tip(&mut tips, &win.items, &win.texts);
            store.save_tooltip_cache(&tips)?;
            r.texts = win.texts.clone();
        } else if let (Some(os), Some(live)) = (&own_status, &live) {
            // Window closed: the same numbers from the character's status data (always in memory). Not while
            // digitalized: those include the demon.
            r.source = "status".into();
            stats_from = now;
            r.level = os.level;
            r.hp = os.hp;
            r.max_hp = os.max_hp;
            r.mp = os.mp;
            r.max_mp = os.max_mp;
            r.pairs = os.pairs.clone();
            r.items = live.clone();
            json::save(
                store.last_stats_path(),
                &json!({
                    "taken_at": iso_s(now), "level": os.level, "hp": os.hp, "max_hp": os.max_hp, "mp": os.mp, "max_mp": os.max_mp,
                    "pairs": os.pairs, "toggles_on": toggles_on(),
                }),
            )?;
            r.texts = r.items.iter().map(|id| tip_text(&tips, *id)).collect();
        } else {
            r.source = "cache".into();
            if !store.last_stats_path().exists() {
                return Err(user_error("No saved stats yet: the stats couldn't be read now (while digitalized they include the demon). Take the first snapshot when not digitalized."));
            }
            let Some(live) = &live else {
                return Err(user_error("Couldn't find the equipped gear in the client's memory (is the character logged in?)."));
            };
            let last = json::load(store.last_stats_path())?;
            r.items = live.clone();
            let np = log::timed("snapshot: nameplate", || client_reader::read_nameplate(&mem, name), |n| if n.is_some() { "found".into() } else { "not found".into() });
            r.level = np.map(|n| n.level).unwrap_or_else(|| json::int(last.at("level")));
            stats_from = parse_datetime(&json::str(last.at("taken_at")).ok_or_else(|| anyhow::anyhow!("Value cannot be null. (Parameter 's')"))?)?;
            r.hp = json::int(last.at("hp"));
            r.max_hp = json::int(last.at("max_hp"));
            r.mp = json::int(last.at("mp"));
            r.max_mp = json::int(last.at("max_mp"));
            r.pairs = json::arr(last.at("pairs")).map(|p| json::int(Some(p))).collect();
            r.texts = r.items.iter().map(|id| tip_text(&tips, *id)).collect();
        }

        // Tarot/soul enchant ids come from the live gear grid (same items as the window when the window is trusted).
        // Each worn item's record (when read): its bonuses then come from ids and server data, not its tooltip.
        let mut tarot: Option<Vec<i32>> = None;
        let mut soul: Option<Vec<i32>> = None;
        let mut details: Option<Vec<Option<WornItem>>> = None;
        if let Some(g) = &grid {
            if g.ids == r.items {
                if !g.tarot.is_empty() {
                    tarot = Some(g.tarot.clone());
                    soul = Some(g.soul.clone());
                }
                details = g.details.clone();
            }
        }
        // Not read this time: keep the last snapshot's records while the same items (and enchants) are worn.
        if details.is_none()
            && store.tooltips_path().exists()
            && let Ok(prev) = store.load_tooltips()
        {
            details = Store::keep_details(&prev, &r.items, tarot.as_deref(), soul.as_deref());
            if details.is_some() {
                log::write(log::Level::Info, "snapshot: item records not read; kept the last snapshot's (same items worn)");
            }
        }
        store.save_tooltips(&r.items, &r.texts, tarot.as_deref(), soul.as_deref(), details.as_deref(), r.level)?;
        // Missing = no way to know the item's bonuses: neither its record nor its tooltip.
        let missing: Vec<&str> =
            (0..15).filter(|&k| r.items[k] > 0 && r.texts[k].is_empty() && details.as_ref().is_none_or(|d| d[k].is_none())).map(|k| client_reader::SLOT_NAMES[k]).collect();
        r.missing = missing.join(", ");
        r.gear = self.ctx.gear()?.compute(&r.texts, Some(&r.items), tarot.as_deref(), soul.as_deref(), details.as_deref(), r.level, None)?;
        r.combat = bonuses::combat_summary(&r.gear.effects);
        if let Some(d) = &digi {
            let res = d.compute(&self.ctx, r.level)?;
            r.demon_combat = bonuses::demon_summary(&res.effects);
            r.digitalize_result = Some(res);
        }
        r.stats_note = match r.source.as_str() {
            "window" => "Information window".to_string(),
            "status" => "Character data".to_string(),
            _ => format!(
                "Saved {}{}",
                stats_from.format("%Y-%m-%d %H:%M"),
                if r.while_digitalized { " (digitalized: the window showed digitalized stats)" } else { "" }
            ),
        };

        let mut snap: IndexMap<String, String> = IndexMap::new();
        let mut put = |k: &str, v: String| {
            snap.insert(k.to_string(), v);
        };
        put("Timestamp", now.format("%Y-%m-%d %H:%M:%S").to_string());
        put("Character", name.to_string());
        put("Level", r.level.to_string());
        put("HP", r.hp.to_string());
        put("MaxHP", r.max_hp.to_string());
        put("MP", r.mp.to_string());
        put("MaxMP", r.max_mp.to_string());
        put("Stats source", r.stats_note.clone());
        put("Stats from", stats_from.format("%Y-%m-%d %H:%M:%S").to_string());
        put("Gear bonuses missing", r.missing.clone());
        for i in 0..12 {
            let b = r.pairs[i * 2];
            let bonus = r.pairs[i * 2 + 1];
            let s = client_reader::STAT_NAMES[i];
            put(s, b.wrapping_add(bonus).to_string());
            put(&format!("{s} base"), b.to_string());
            put(&format!("{s} bonus"), bonus.to_string());
        }
        for c in &r.combat {
            put(&format!("{} (gear)", c.stat), inv(c.always));
            put(&format!("{} (gear, digitalized)", c.stat), inv(c.digi));
        }
        for k in 0..15 {
            let id = r.items[k];
            let slot = slot_label(k); // one fixed column per slot, whether it is filled, empty or an item the data doesn't know
            if id <= 0 {
                put(&format!("Gear: {slot}"), "(empty)".into());
                continue;
            }
            let it = known_items.get(&id);
            // An item missing from the item list (NewMoon-only) is named as its tooltip shows it.
            let shown = if r.texts[k].is_empty() { None } else { Some(TooltipParser::info(&r.texts[k]).base_name) };
            let value = match it {
                Some(it) => json::str(it.at("name")).unwrap_or_default(),
                None => match shown {
                    Some(s) if !s.is_empty() => s,
                    _ => format!("Item {id}"),
                },
            };
            put(&format!("Gear: {slot}"), value);
        }
        // The ids themselves, so a later recalculation never depends on recognising a name.
        put(GEAR_IDS_COLUMN, r.items.iter().map(i32::to_string).collect::<Vec<_>>().join(" "));
        // The items' records, so a later recalculation works from ids too (Store.PackItems; "" when not read).
        put(GEAR_RECORDS_COLUMN, Store::pack_items(details.as_deref()));
        put_demon(&mut snap, digi.as_ref(), &r.demon_combat);
        r.csv_row = snap.clone();

        if !csv_path.is_empty() {
            let mut t = CsvTable::load(csv_path)?;
            let mut cols: Vec<String> = snap.keys().cloned().collect();
            for c in &t.columns {
                if !cols.contains(c) {
                    cols.push(c.clone());
                }
            }
            t.columns = cols;
            t.rows.push(snap);
            t.save(csv_path)?;
        }
        crate::log_info!("snapshot: done in {} ms: stats from {}, level {}, gear with no saved bonuses: {}", started.elapsed().as_millis(), r.source, r.level, if r.missing.is_empty() { 0 } else { r.missing.split(", ").count() });
        Ok(r)
    }

    /// Recompute every CSV row's gear-bonus columns from the cached tooltips (after a calculation fix).
    /// Item ids are recovered from the item names in the row's "Gear: *" columns.
    pub fn recalc_csv(&self, csv_path: &str) -> Result<Vec<String>> {
        let store: &Store = &self.ctx.store;
        let data = &*self.ctx.data;
        let mut log = Vec::new();
        let mut t = CsvTable::load(csv_path)?;
        if t.rows.is_empty() {
            return Ok(log);
        }
        let tips = store.load_tooltip_cache()?;
        let items = data.items()?;
        let by_name = item_ids_by_name(items);
        let gear_cols: Vec<String> = t.columns.iter().filter(|c| c.starts_with("Gear: ")).cloned().collect();
        // What the latest snapshot read, slot by slot: the only place an unlisted item's id survives for rows without ids.
        let mut last_read: Option<Vec<i32>> = None;
        if store.tooltips_path().exists() {
            let mut recs = store.load_tooltips()?;
            recs.sort_by_key(|r| r.index);
            if recs.len() == 15 {
                last_read = Some(recs.iter().map(|r| r.id).collect());
            }
        }
        // One gear calculator for every row: building it reads the set table (client-sets.json, ~720 KB) and the
        // expertise files, none of which change during the loop.
        let gear = self.ctx.gear()?;
        for ri in 0..t.rows.len() {
            let mut changes: Vec<String> = Vec::new();
            // Newer rows carry the item ids; older ones only the names, which are looked up (an unlisted item is lost there).
            let ids_text = t.get(&t.rows[ri], GEAR_IDS_COLUMN);
            let saved: Vec<&str> = ids_text.split(' ').filter(|s| !s.is_empty()).collect();
            let has_ids = saved.len() == 15 && saved.iter().all(|s| fmt::parse_i32(s).is_some());
            let mut ids: Vec<i32> = if has_ids {
                saved.iter().map(|s| fmt::parse_i32(s).unwrap_or(0)).collect()
            } else {
                gear_cols
                    .iter()
                    .map(|c| match by_name.get(&t.get(&t.rows[ri], c)) {
                        Some(cand) => cand.iter().copied().find(|id| tips.contains_key(id)).unwrap_or(cand[0]),
                        None => 0,
                    })
                    .collect()
            };
            // A row without ids that wears exactly what was last read, except for items the item list doesn't have
            // (their names can't be looked up): those items are the last-read ones. Give the row its ids back.
            if let (false, Some(last_read), true) = (has_ids, &last_read, gear_cols.len() == 15) {
                let unlisted = |ids: &[i32], t: &CsvTable, k: usize| ids[k] == 0 && last_read[k] > 0 && !items.contains_key(&last_read[k]) && t.get(&t.rows[ri], &gear_cols[k]) != "(empty)";
                if (0..15).all(|k| ids[k] == last_read[k] || unlisted(&ids, &t, k)) && (0..15).any(|k| unlisted(&ids, &t, k)) {
                    for k in 0..15 {
                        if !unlisted(&ids, &t, k) {
                            continue;
                        }
                        let shown = tips.get(&last_read[k]).and_then(|tip| tip.text.as_deref()).map(|text| TooltipParser::info(text).base_name);
                        let named = match shown {
                            Some(s) if !s.is_empty() => s,
                            _ => format!("Item {}", last_read[k]),
                        };
                        changes.push(format!("{}: {} -> {} (id {})", gear_cols[k], t.get(&t.rows[ri], &gear_cols[k]), named, last_read[k]));
                        t.rows[ri].insert(gear_cols[k].clone(), named);
                    }
                    ids = last_read.clone();
                    if !t.columns.iter().any(|c| c == GEAR_IDS_COLUMN) {
                        t.columns.push(GEAR_IDS_COLUMN.into());
                    }
                    t.rows[ri].insert(GEAR_IDS_COLUMN.into(), ids.iter().map(i32::to_string).collect::<Vec<_>>().join(" "));
                }
            }
            let texts: Vec<String> = ids.iter().map(|id| if *id > 0 { tip_text(&tips, *id) } else { String::new() }).collect();
            // Rows that saved the items' records are worked out from ids; older rows from the tooltips.
            let mut details = Store::parse_items(Some(&t.get(&t.rows[ri], GEAR_RECORDS_COLUMN)));
            if let Some(d) = &details {
                if !d.iter().map(|w| w.as_ref().map_or(0, |w| w.id)).eq(ids.iter().copied()) {
                    details = None;
                }
            }
            let row_level = fmt::parse_i32(&t.get(&t.rows[ri], "Level")).unwrap_or(0);
            let combat = bonuses::combat_summary(&gear.compute(&texts, Some(&ids), None, None, details.as_deref(), row_level, None)?.effects);
            for c in &combat {
                for (col, v) in [(format!("{} (gear)", c.stat), c.always), (format!("{} (gear, digitalized)", c.stat), c.digi)] {
                    if !t.columns.contains(&col) {
                        continue;
                    }
                    let old = t.get(&t.rows[ri], &col);
                    let ov = fmt::parse_f64(&old).unwrap_or(0.0);
                    if ov != v {
                        changes.push(format!("{col}: {old} -> {}", inv(v)));
                    }
                    t.rows[ri].insert(col, inv(v));
                }
            }
            // The digitalize demon saved with the row, recomputed the same way
            if let Some(dsave) = DigitalizeSave::unpack(&t.get(&t.rows[ri], DEMON_RECORD_COLUMN)).filter(|d| d.demon.is_some()) {
                let dc = bonuses::demon_summary(&dsave.compute(&self.ctx, row_level)?.effects);
                for (stat, _) in bonuses::STAT_MAP {
                    let col = demon_col(stat);
                    if !t.columns.contains(&col) {
                        continue;
                    }
                    let old = t.get(&t.rows[ri], &col);
                    let ov = fmt::parse_f64(&old).unwrap_or(0.0);
                    let v = dc.get(*stat).copied().unwrap_or(0.0);
                    if ov != v {
                        changes.push(format!("{col}: {old} -> {}", inv(v)));
                    }
                    t.rows[ri].insert(col, inv(v));
                }
            }
            log.push(format!("{}: {}", t.get(&t.rows[ri], "Timestamp"), if changes.is_empty() { "no change".to_string() } else { changes.join("; ") }));
        }
        t.save(csv_path)?;
        Ok(log)
    }

    /// Rename the gear in every CSV row ("Gear: <slot>" columns) to the item list's name, the game's own (rows saved
    /// before 2026-11-24 hold the website's names, e.g. "Levi Blaster" for Leviathan Blaster). Only names change, no
    /// number. Items come from the row's "Gear ids", else from its names (the website's included); a slot whose item
    /// isn't in the item list keeps its name. The CSV is copied first (`<name>.backup-before-item-names-<time>.csv`),
    /// and only when a name changes. Returns one line per changed row, and the backup.
    pub fn rename_gear_csv(&self, csv_path: &str) -> Result<(Vec<String>, Option<std::path::PathBuf>)> {
        let data = &*self.ctx.data;
        let mut t = CsvTable::load(csv_path)?;
        let tips = self.ctx.store.load_tooltip_cache()?;
        let items = data.items()?;
        let by_name = item_ids_by_name(items);
        let gear_cols: Vec<String> = t.columns.iter().filter(|c| c.starts_with("Gear: ")).cloned().collect();
        let mut log = Vec::new();
        for ri in 0..t.rows.len() {
            let ids_text = t.get(&t.rows[ri], GEAR_IDS_COLUMN);
            let saved: Vec<i32> = ids_text.split(' ').filter(|s| !s.is_empty()).filter_map(fmt::parse_i32).collect();
            let has_ids = saved.len() == 15 && saved.len() == gear_cols.len();
            let mut changes = Vec::new();
            for (k, col) in gear_cols.iter().enumerate() {
                let old = t.get(&t.rows[ri], col);
                if old.is_empty() || old == "(empty)" {
                    continue;
                }
                let id = if has_ids {
                    saved[k]
                } else {
                    match by_name.get(&old) {
                        Some(cand) => cand.iter().copied().find(|id| tips.contains_key(id)).unwrap_or(cand[0]),
                        None => 0,
                    }
                };
                if id <= 0 {
                    continue;
                }
                let Some(name) = data.item_name(id)?.filter(|n| !n.is_empty()) else { continue };
                if name != old {
                    changes.push(format!("{col}: {old} -> {name}"));
                    t.rows[ri].insert(col.clone(), name);
                }
            }
            if !changes.is_empty() {
                log.push(format!("{}: {}", t.get(&t.rows[ri], "Timestamp"), changes.join("; ")));
            }
        }
        if log.is_empty() {
            return Ok((log, None));
        }
        let path = std::path::Path::new(csv_path);
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_else(|| std::path::PathBuf::from("."));
        let backup = dir.join(format!("{stem}.backup-before-item-names-{}.csv", Local::now().format("%Y%m%d-%H%M%S")));
        if backup.exists() {
            return Err(anyhow::anyhow!("The file '{}' already exists.", backup.display()));
        }
        std::fs::copy(csv_path, &backup)?;
        t.save(csv_path)?;
        Ok((log, Some(backup)))
    }
}

/// Item name -> ids, by the item list's names (the game's) and the website's ("webName"): rows saved before 2026-11-24
/// named their gear the website's way, which can differ from the game's.
fn item_ids_by_name(items: &IndexMap<i32, Value>) -> IndexMap<String, Vec<i32>> {
    let mut by_name: IndexMap<String, Vec<i32>> = IndexMap::new();
    for (id, v) in items {
        by_name.entry(json::str(v.at("name")).unwrap_or_default()).or_default().push(*id);
    }
    for (id, v) in items {
        if let Some(web) = json::str(v.at("webName")) {
            let l = by_name.entry(web).or_default();
            if !l.contains(id) {
                l.push(*id);
            }
        }
    }
    by_name
}

/// The cached tooltip text of an item id ("" when none).
fn tip_text(tips: &IndexMap<i32, CachedTip>, id: i32) -> String {
    if id > 0 { tips.get(&id).and_then(|t| t.text.clone()).unwrap_or_default() } else { String::new() }
}

