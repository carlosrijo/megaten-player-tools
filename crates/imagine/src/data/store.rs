//! Port of `src/Data/Store.cs`: the tool's own files: caches from earlier snapshots and the page data it builds.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use fancy_regex::Regex;
use indexmap::IndexMap;
use serde_json::{Map, Value, json};

use super::json::{self, At, Row};
use crate::Result;
use crate::game::{SetRecord, WornItem};

/// Tooltip text per item id as last seen (`CachedTip`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedTip {
    pub text: Option<String>,
    pub seen_at: Option<String>,
}

/// One slot: item id, tooltip text, tarot/soul enchant ids from memory (None if not read), and the item's full record
/// (None if not read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotTip {
    pub index: i32,
    pub id: i32,
    pub text: String,
    pub tarot: Option<i32>,
    pub soul: Option<i32>,
    pub detail: Option<WornItem>,
}

pub struct Store {
    pub dir: PathBuf,
}

const PROFILE_NOTE_TAIL: &str = ". alignment: for 'If Player is LAW/NEUTRAL/CHAOS' gear conditions; toggles: switch skills that are on; \
quest_count / compendium_count (from the game once its Quest Completion Bonuses / Demonic Compendium window has been opened, else set by hand) and compendium_v2 (from the game's valuables, else by hand): for the quest and compendium bonuses.";

fn now_minute() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        Store { dir: dir.into() }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.dir.join("cache")
    }
    pub fn dash_dir(&self) -> PathBuf {
        self.dir.join("dash")
    }
    /// The builds this character published to Community Builds: their ids and the secrets that change them
    /// (build::community). Not in the support bundle (it takes a fixed list of files).
    pub fn community_path(&self) -> PathBuf {
        self.dir.join("community.json")
    }
    pub fn last_stats_path(&self) -> PathBuf {
        self.cache_dir().join("last-stats.json")
    }
    pub fn tooltip_cache_path(&self) -> PathBuf {
        self.cache_dir().join("tooltip-cache.json")
    }
    pub fn set_table_path(&self) -> PathBuf {
        self.cache_dir().join("client-sets.json")
    }
    pub fn profile_path(&self) -> PathBuf {
        self.cache_dir().join("profile.json")
    }
    /// The demon the character digitalizes with (Build\DigitalizeDemon.cs).
    pub fn digitalize_path(&self) -> PathBuf {
        self.cache_dir().join("digitalize.json")
    }
    /// That demon's icon for the page.
    pub fn demon_picture_path(&self) -> PathBuf {
        self.dash_dir().join("demon.png")
    }
    /// Every COMP demon seen in the game, for the digitalize demon picker (the game only loads them all while the COMP is open).
    pub fn comp_demons_path(&self) -> PathBuf {
        self.cache_dir().join("comp-demons.json")
    }
    /// The learned digitalize assists and the Mitama set boost as last read from the game, so the picker can switch demons without reading it.
    pub fn digitalize_progress_path(&self) -> PathBuf {
        self.cache_dir().join("digitalize-progress.json")
    }
    /// The client's 32x32 icon of a demon type, as PNG (kept for when the game isn't running).
    pub fn demon_icon_path(&self, r#type: i32) -> PathBuf {
        self.cache_dir().join("demon-icons").join(format!("{type}.png"))
    }
    /// The client's 32x32 icon of an item, as PNG (kept for when the game isn't running).
    pub fn item_icon_path(&self, item_id: i32) -> PathBuf {
        self.cache_dir().join("item-icons").join(format!("{item_id}.png"))
    }
    /// A picture cut from the client's interface sheets (e.g. "compendium"), as PNG.
    pub fn ui_icon_path(&self, name: &str) -> PathBuf {
        self.cache_dir().join("ui-icons").join(format!("{name}.png"))
    }
    /// The skill icons the page shows (Expertise, Digitalize and Bonuses), as PNG, named after the client's icon: dash\skill-icons\<icon>.png.
    pub fn skill_icon_dir(&self) -> PathBuf {
        self.dash_dir().join("skill-icons")
    }
    /// The game client's folder as last seen running, so icons can be read from it when the game is closed.
    pub fn client_dir_path(&self) -> PathBuf {
        self.cache_dir().join("client-dir.txt")
    }
    pub fn tooltips_path(&self) -> PathBuf {
        self.dir.join("tooltips.txt")
    }
    pub fn expertise_ranks_path(&self) -> PathBuf {
        self.dir.join("expertise-ranks.json")
    }

    // ---------- Tooltip text per item id (seen while the Information window was open) ----------

    pub fn load_tooltip_cache(&self) -> Result<IndexMap<i32, CachedTip>> {
        let mut res = IndexMap::new();
        let path = self.tooltip_cache_path();
        if !path.exists() {
            return Ok(res);
        }
        let root = json::load(&path)?;
        for (k, v) in json::as_object(Some(&root))? {
            if let Some(id) = crate::fmt::parse_i32(k) {
                if !v.is_null() {
                    res.insert(id, CachedTip { text: json::str(v.at("text")), seen_at: json::str(v.at("seen_at")) });
                }
            }
        }
        Ok(res)
    }

    pub fn save_tooltip_cache(&self, tips: &IndexMap<i32, CachedTip>) -> Result<()> {
        let mut o = Row::new();
        for (id, t) in tips {
            o.insert(id.to_string(), json!({ "text": t.text, "seen_at": t.seen_at }));
        }
        json::save(self.tooltip_cache_path(), &Value::Object(o))
    }

    // ---------- Equipment-set table ----------

    /// The client's equipment sets saved by the last read; `None` when there is no file.
    pub fn load_set_table(&self) -> Result<Option<Vec<SetRecord>>> {
        let path = self.set_table_path();
        if !path.exists() {
            return Ok(None);
        }
        let root = json::load(&path)?;
        Ok(Some(
            json::arr(Some(&root))
                .map(|s| SetRecord {
                    id: json::int(s.at("id")),
                    pieces: json::arr(s.at("pieces")).map(|p| json::int(Some(p))).collect(),
                    tokusei: json::arr(s.at("tok")).map(|p| json::int(Some(p))).filter(|&t| t != 0).collect(),
                })
                .collect(),
        ))
    }

    pub fn save_set_table(&self, sets: &[SetRecord]) -> Result<()> {
        let rows: Vec<Value> = sets.iter().map(|s| json!({ "id": s.id, "pieces": s.pieces, "tok": s.tokusei })).collect();
        json::save(self.set_table_path(), &Value::Array(rows))
    }

    // ---------- Profile (alignment) and expertise classes, for tooltip requirements ----------

    pub fn alignment(&self) -> Result<String> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok("NEUTRAL".into());
        }
        let root = json::load(&path)?;
        Ok(match json::str(root.at("alignment")) {
            Some(a) if !a.is_empty() => a.to_uppercase(),
            _ => "NEUTRAL".into(),
        })
    }

    pub fn save_alignment(&self, alignment: Option<&str>) -> Result<()> {
        let toggles = self.active_toggles()?;
        self.save_profile(alignment, &toggles)
    }

    /// The toggle (switch) skills that are switched on (skill ids), saved from the game's learned-skill map at each
    /// snapshot (or set by hand with `toggles`); expertise counts these toggles and Max HP/MP uses their percentages.
    pub fn active_toggles(&self) -> Result<Vec<i32>> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let root = json::load(&path)?;
        Ok(json::arr(root.at("toggles")).map(|n| json::int(Some(n))).filter(|&i| i > 0).collect())
    }

    /// Toggles set by hand (`toggles "Name, ..."`): the "read from the game" stamp no longer describes them, so it goes.
    pub fn save_toggles(&self, ids: impl IntoIterator<Item = i32>) -> Result<()> {
        self.save_toggles_as(ids, false)
    }

    /// Toggles just read from the game's learned-skill map: saved with the time of the reading (`toggles_source`).
    pub fn save_toggles_from_game(&self, ids: impl IntoIterator<Item = i32>) -> Result<()> {
        self.save_toggles_as(ids, true)
    }

    fn save_toggles_as(&self, ids: impl IntoIterator<Item = i32>, from_game: bool) -> Result<()> {
        let alignment = if self.profile_path().exists() { Some(self.alignment()?) } else { None };
        let mut distinct: Vec<i32> = Vec::new();
        for i in ids {
            if !distinct.contains(&i) {
                distinct.push(i);
            }
        }
        self.update_profile(|o| {
            o.insert("alignment".into(), json!(alignment));
            o.insert("toggles".into(), json!(distinct));
            if from_game {
                o.insert("toggles_source".into(), json!(format!("game, {}", now_minute())));
            } else {
                o.remove("toggles_source");
            }
        })
    }

    /// When the saved toggle states were read, for the page's footnote: "game, <date time>" (read from the game), or
    /// "saved, <date time>" for a profile that predates the stamp (the time of its last save: the toggles are no newer
    /// than that, and may be older) or toggles set by hand. `None` when no toggles were ever saved.
    pub fn toggles_source(&self) -> Result<Option<String>> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok(None);
        }
        let root = json::load(&path)?;
        if let Some(s) = json::str(root.at("toggles_source")).filter(|s| !s.is_empty()) {
            return Ok(Some(s));
        }
        if root.at("toggles").is_none() {
            return Ok(None);
        }
        let saved = json::str(root.at("note")).and_then(|n| n.strip_prefix("Updated ").map(|r| r.chars().take(16).collect::<String>()));
        Ok(saved.filter(|t| t.len() == 16).map(|t| format!("saved, {t}")))
    }

    fn save_profile(&self, alignment: Option<&str>, toggles: &[i32]) -> Result<()> {
        self.update_profile(|o| {
            o.insert("alignment".into(), json!(alignment));
            o.insert("toggles".into(), json!(toggles));
        })
    }

    /// Completed-quest count, Demonic Compendium entry count and whether Compendium v2 is owned, for the quest and
    /// compendium bonuses (Calc\CharacterExtras). The quest count is saved from the game's completed-quest map when the game
    /// has loaded it (Expertise.Read, `extras`); the compendium ones are set by hand (`extras`). `None` = not known.
    pub fn counts(&self) -> Result<(Option<i32>, Option<i32>, Option<bool>)> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok((None, None, None));
        }
        let p = json::load(&path)?;
        let i = |k: &str| json::num(p.at(k)).map(|d| d as i32);
        Ok((i("quest_count"), i("compendium_count"), p.at("compendium_v2").map(|v| json::truthy(Some(v)))))
    }

    /// `quests_from_game`: the quest count was read from the game (the completed-quest map), not typed.
    /// `compendium_from_game`: the compendium count was read from the game (the Demonic Compendium window's text).
    pub fn save_counts(
        &self,
        quests: Option<i32>,
        compendium: Option<i32>,
        compendium_v2: Option<bool>,
        quests_from_game: bool,
        compendium_from_game: bool,
    ) -> Result<()> {
        self.update_profile(|o| {
            let src = |v: Option<i32>, game: bool| -> Value {
                match v {
                    None => Value::Null,
                    Some(_) if game => json!(format!("game, {}", now_minute())),
                    Some(_) => json!("set by hand"),
                }
            };
            let changed = |key: &str, new: Option<i32>| json::node_to_string(o.get(key)) != new.map(|v| v.to_string());
            let (quests_changed, compendium_changed) = (changed("quest_count", quests), changed("compendium_count", compendium));
            if quests_from_game || quests_changed {
                let s = src(quests, quests_from_game);
                o.insert("quest_count_source".into(), s);
            }
            if compendium_from_game || compendium_changed {
                let s = src(compendium, compendium_from_game);
                o.insert("compendium_count_source".into(), s);
            }
            o.insert("quest_count".into(), json!(quests));
            o.insert("compendium_count".into(), json!(compendium));
            o.insert("compendium_v2".into(), json!(compendium_v2));
        })
    }

    /// Where the saved quest count came from ("game, date" or "set by hand"), `None` when unset.
    pub fn quest_count_source(&self) -> Result<Option<String>> {
        self.profile_string("quest_count_source")
    }

    /// Where the saved compendium count came from ("game, date" or "set by hand"), `None` when unset.
    pub fn compendium_count_source(&self) -> Result<Option<String>> {
        self.profile_string("compendium_count_source")
    }

    fn profile_string(&self, key: &str) -> Result<Option<String>> {
        let path = self.profile_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(json::str(json::load(&path)?.at(key)))
    }

    /// Changes some profile keys and keeps the rest.
    fn update_profile(&self, set: impl FnOnce(&mut Map<String, Value>)) -> Result<()> {
        std::fs::create_dir_all(self.cache_dir())?;
        let path = self.profile_path();
        let mut o = match if path.exists() { json::load(&path)? } else { Value::Null } {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        set(&mut o);
        o.insert("note".into(), json!(format!("Updated {}{}", now_minute(), PROFILE_NOTE_TAIL)));
        json::save(&path, &Value::Object(o))
    }

    /// Expertise class by expertise name (case-insensitive: the keys are `crate::text::ic_key(name)`, upper-cased).
    pub fn expertise_classes(&self) -> Result<IndexMap<String, i32>> {
        let mut res = IndexMap::new();
        let path = self.expertise_ranks_path();
        if !path.exists() {
            return Ok(res);
        }
        let root = json::load(&path)?;
        for r in json::arr(root.at("ranks")) {
            if let Some(n) = json::str(r.at("name")).filter(|n| !n.is_empty()) {
                res.insert(crate::text::ic_key(&n), json::int(r.at("class")));
            }
        }
        Ok(res)
    }

    /// Expertise rank by expertise id, as the server counts it for requirements: class * 10 + rank.
    pub fn expertise_ranks(&self) -> Result<IndexMap<i32, i32>> {
        let mut res = IndexMap::new();
        let path = self.expertise_ranks_path();
        if !path.exists() {
            return Ok(res);
        }
        let root = json::load(&path)?;
        for r in json::arr(root.at("ranks")) {
            if let Some(id) = json::num(r.at("id")) {
                res.insert(id as i32, json::int(r.at("class")).wrapping_mul(10).wrapping_add(json::int(r.at("rank"))));
            }
        }
        Ok(res)
    }

    // ---------- Item records (ClientReader.WornItem) as text ----------

    /// "id:tarot:soul:basic:special:m1,m2,m3,m4,m5:f1,f2,f3:durability:max", "-" for an empty slot.
    pub fn pack_item(w: Option<&WornItem>) -> String {
        match w {
            None => "-".into(),
            Some(w) => {
                let join = |v: &[i32]| v.iter().map(i32::to_string).collect::<Vec<_>>().join(",");
                format!("{}:{}:{}:{}:{}:{}:{}:{}:{}", w.id, w.tarot, w.soul, w.basic, w.special, join(&w.mods), join(&w.fuse), w.durability, w.max_durability)
            }
        }
    }

    /// The record `PackItem` wrote, `None` when the text is not one (wrong field count, not numbers).
    pub fn parse_item(s: Option<&str>) -> Option<WornItem> {
        let f: Vec<&str> = s.unwrap_or("").split(':').collect();
        if f.len() != 9 {
            return None;
        }
        let ints = |t: &str| -> Option<Vec<i32>> { t.split(',').map(crate::fmt::parse_i32).collect() };
        let mods = ints(f[5])?;
        let fuse = ints(f[6])?;
        if mods.len() != 5 || fuse.len() != 3 {
            return None;
        }
        let i = |t: &str| crate::fmt::parse_i32(t);
        Some(WornItem { id: i(f[0])?, tarot: i(f[1])?, soul: i(f[2])?, basic: i(f[3])?, special: i(f[4])?, mods, fuse, durability: i(f[7])?, max_durability: i(f[8])? })
    }

    /// The 15 slots' records for a CSV cell (space separated), or "" when not read.
    pub fn pack_items(d: Option<&[Option<WornItem>]>) -> String {
        match d {
            None => String::new(),
            Some(d) => d.iter().map(|w| Store::pack_item(w.as_ref())).collect::<Vec<_>>().join(" "),
        }
    }

    /// Records from a CSV cell: `None` unless all 15 slots are there.
    pub fn parse_items(s: Option<&str>) -> Option<Vec<Option<WornItem>>> {
        let parts: Vec<&str> = s.unwrap_or("").split(' ').filter(|p| !p.is_empty()).collect();
        if parts.len() != 15 {
            return None;
        }
        let res: Vec<Option<WornItem>> = parts.iter().map(|p| if *p == "-" { None } else { Store::parse_item(Some(p)) }).collect();
        if parts.iter().enumerate().any(|(k, p)| *p != "-" && res[k].is_none()) {
            return None;
        }
        Some(res)
    }

    // ---------- tooltips.txt: the 15 slot texts of the latest snapshot ----------

    /// Writes tooltips.txt. `details` and `level` are optional; a "level N" line goes first when known.
    pub fn save_tooltips(
        &self,
        items: &[i32],
        texts: &[String],
        tarot: Option<&[i32]>,
        soul: Option<&[i32]>,
        details: Option<&[Option<WornItem>]>,
        level: i32,
    ) -> Result<()> {
        let mut sb = String::new();
        if level > 0 {
            sb.push_str(&format!("level {level}\r\n"));
        }
        for k in 0..15 {
            let ench = match (tarot, soul) {
                (Some(t), Some(s)) => format!(" (tarot {} soul {})", t[k], s[k]),
                _ => String::new(),
            };
            let rec = match details.and_then(|d| d.get(k)).and_then(|w| w.as_ref()) {
                Some(w) => format!(" (item {})", Store::pack_item(Some(w))),
                None => String::new(),
            };
            sb.push_str(&format!("===== record {k} (id {}){ench}{rec}\r\n", items[k]));
            if !texts[k].is_empty() {
                sb.push_str(&texts[k].replace("\r\n", "\n").replace('\n', "\r\n"));
                sb.push_str("\r\n");
            }
        }
        std::fs::create_dir_all(&self.dir)?;
        crate::data::json::write_atomic(self.tooltips_path(), sb)?;
        Ok(())
    }

    pub fn load_tooltips(&self) -> Result<Vec<SlotTip>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?ms)^===== record (\d+) \(id (-?\d+)\)(?: \(tarot (\d+) soul (\d+)\))?(?: \(item ([-\d:,]+)\))?\r?\n(.*?)(?=^===== record |\z)").unwrap()
        });
        let txt = json::read_all_text(&self.tooltips_path())?;
        let mut res = Vec::new();
        for m in RE.captures_iter(txt.as_str()) {
            let m = m?;
            let opt = |i: usize| m.get(i).map(|g| crate::fmt::parse_i32(g.as_str()).ok_or_else(|| anyhow::anyhow!("bad number in tooltips.txt"))).transpose();
            res.push(SlotTip {
                index: opt(1)?.unwrap_or(0),
                id: opt(2)?.unwrap_or(0),
                text: m[6].trim_end().to_string(),
                tarot: opt(3)?,
                soul: opt(4)?,
                detail: m.get(5).and_then(|g| Store::parse_item(Some(g.as_str()))),
            });
        }
        Ok(res)
    }

    /// The character level saved with tooltips.txt (0 when not saved).
    pub fn tooltips_level(&self) -> Result<i32> {
        let path = self.tooltips_path();
        if !path.exists() {
            return Ok(0);
        }
        let text = json::read_all_text(&path)?;
        let first = json::lines(&text).first().copied().unwrap_or("");
        Ok(match first.strip_prefix("level ") {
            Some(rest) => crate::fmt::parse_i32(rest).unwrap_or(0),
            None => 0,
        })
    }

    /// Item records from saved slots: `None` unless every worn slot has one.
    pub fn unpack_details(recs: &[SlotTip]) -> Option<Vec<Option<WornItem>>> {
        if recs.len() != 15 || recs.iter().any(|r| r.id > 0 && r.detail.is_none()) {
            return None;
        }
        let mut v: Vec<&SlotTip> = recs.iter().collect();
        v.sort_by_key(|r| r.index);
        Some(v.into_iter().map(|r| if r.id > 0 { r.detail.clone() } else { None }).collect())
    }

    /// The item records of an earlier snapshot (`prev`, as [`Store::load_tooltips`] gives them), for a snapshot that
    /// couldn't read them, when they still describe what is worn: the same item id in every slot and, where the
    /// tarot/soul enchants were read now, the same enchants. Otherwise None. Without this, one snapshot that misses the
    /// records wrote tooltips.txt without them, and the simulator refused to import until the next good snapshot.
    /// Mods and fuse can't be checked against anything; only durability is known to go stale.
    pub fn keep_details(prev: &[SlotTip], items: &[i32], tarot: Option<&[i32]>, soul: Option<&[i32]>) -> Option<Vec<Option<WornItem>>> {
        let det = Store::unpack_details(prev)?;
        if items.len() != 15 {
            return None;
        }
        let mut ids: Vec<(i32, i32)> = prev.iter().map(|r| (r.index, r.id)).collect();
        ids.sort();
        if ids.iter().map(|p| p.1).ne(items.iter().copied()) {
            return None;
        }
        for (k, d) in det.iter().enumerate() {
            let Some(d) = d else { continue };
            if d.id != items[k] || tarot.is_some_and(|t| t[k] != d.tarot) || soul.is_some_and(|s| s[k] != d.soul) {
                return None;
            }
        }
        Some(det)
    }

    /// Compute arguments from saved slots (texts, ids, tarot, soul); enchant ids only when every slot has them.
    #[allow(clippy::type_complexity)]
    pub fn unpack(recs: &[SlotTip]) -> (Vec<String>, Vec<i32>, Option<Vec<i32>>, Option<Vec<i32>>) {
        let ench = recs.iter().all(|r| r.tarot.is_some() && r.soul.is_some());
        (
            recs.iter().map(|r| r.text.clone()).collect(),
            recs.iter().map(|r| r.id).collect(),
            ench.then(|| recs.iter().map(|r| r.tarot.unwrap_or(0)).collect()),
            ench.then(|| recs.iter().map(|r| r.soul.unwrap_or(0)).collect()),
        )
    }

    /// `File.Exists(path) ? Json.Load(path) : null`.
    pub fn load_json(&self, path: impl AsRef<Path>) -> Result<Option<Value>> {
        let path = path.as_ref();
        if path.exists() { Ok(Some(json::load(path)?)) } else { Ok(None) }
    }
}
