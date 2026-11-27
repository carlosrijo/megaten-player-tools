//! Port of `src/Build/Simulator.cs`.
//!
//! The Simulator tool: "what would my stats be with this gear and these expertise ranks". Nothing is read from the
//! game and nothing is written (item icons aside, which are cached like everywhere else).
//!
//! A build is the 15 worn pieces as the game holds them (objects::Item: the item itself, whose id decides its set and
//! appearance bonus; the item its basic features come from; the item its characteristics come from; tarot; soul; the
//! fusion great-success bonuses; mods) plus the points of every expertise. The start is the character as last read
//! (tooltips.txt records, expertise-ranks.json, cache\last-stats.json).
//!
//! Bonuses are worked out by the same code as the page data: [`GearEffects`] for the gear, [`Expertise::rows`] for the
//! passives the ranks unlock, [`Dashboard::bonus_rows`] for the list. So an unchanged build gives exactly
//! dash\bonuses.json.
//!
//! The 12 stats and Max HP / MP start from the game's own numbers and move by what the change adds, following the
//! server's order (comp_hack CharacterState::RecalculateStats, ActiveEntityState::AdjustStats,
//! CharacterManager::CalculateDependentStats):
//!   STR..LUCK   = (base + digitalize demon's) with percent, + flat
//!   CLS-R etc.  = (floor(source x factor + level x 0.1) + fusion bonus + demon's) with percent, + flat
//!                 CLS-R from STR, LNG-R from SPD, Spell from MAG, Support from INT (x 0.5); P.Def from VIT, M.Def from INT (x 0.1)
//!   Max HP / MP = CharacterHpMp's formula on the new STR/MAG/VIT/INT, then percent, flat (calibrated on the reading)
//! Whatever the reader's bonus list does not explain of a reading (a buff, a source it does not know) is kept as it
//! was: each stat is "the reading + (model of the new build - model of the build as read)".
//! Checked on Rubito (2026-11-23): the build as read gives back the 12 readings and 702 / 291; digitalized (Specter)
//! it gives the same Max HP / MP as the Digitalize page.
//!
//! Differences from C#, forced by the language: the C# `Simulator` is built from `(store, data, dash, expertise, ...)`;
//! here it is built from the pipeline's [`Context`] (store + data) and makes its own `Dashboard` and `Expertise` from it
//! (both are thin wrappers around the context, so they are the same as the pipeline's). The worn pieces are `Arc`s so
//! that "the build's piece is the very object the base holds" (`ReferenceEquals` in `TipOk`) is `Arc::ptr_eq`. The
//! simulator is `Send + Sync`: every public call takes the one lock, like the C# `lock (_lock)`.

#![allow(clippy::collapsible_if, clippy::redundant_guards, clippy::needless_range_loop)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::UNIX_EPOCH;

use indexmap::IndexMap;
use regex::Regex;
use serde_json::{Value, json};

use crate::calc::character_hp_mp::{self, Calibration};
use crate::calc::digitalize::{Digitalize, DigitalizeResult};
use crate::calc::effect::Effect;
use crate::calc::gear_effects::{GearEffects, GearResult};
use crate::calc::item_effects::ItemEffects;
use crate::calc::tooltip_parser::TooltipParser;
use crate::context::Context;
use crate::data::game_data::GameData;
use crate::data::json::{self, At, Escape, Row};
use crate::data::store::Store;
use crate::game::client_reader::{self, SLOT_NAMES};
use crate::game::{DemonRecord, SetRecord, WornItem};
use crate::text::{eq_ic, ic_key};
use crate::{Result, user_error};

use super::dashboard::{Dashboard, Part, fuse_stat_label};
use super::digitalize_demon::DigitalizeSave;
use super::expertise::Expertise;
use super::gear_db;
use super::item_icons;
use super::pipeline::{self, BlankSets};
use super::share_code::{self, Piece, Shared};

/// `Simulator.ToJson(o)`: compact JSON, relaxed escaping.
pub fn to_json(o: &Value) -> String {
    json::serialize(o, Escape::Relaxed, false)
}

fn rows_value(rows: Vec<Row>) -> Value {
    Value::Array(rows.into_iter().map(Value::Object).collect())
}

fn obj(v: Value) -> Row {
    match v {
        Value::Object(m) => m,
        _ => Row::new(),
    }
}

/// The page's keys for the 12 stats, in the reading's order (ClientReader.StatNames).
const STAT_KEYS: [&str; 12] = ["str", "mag", "vit", "int", "spd", "luck", "clsr", "lngr", "spell", "support", "pdef", "mdef"];
/// The names the bonus list gives those 12 (Bonuses.Canonical), then Max HP and Max MP.
const STAT_NAMES: [&str; 14] = ["STR", "MAG", "VIT", "INT", "SPD", "LUCK", "CLS-R", "LNG-R", "Spell", "Support", "P.Def", "M.Def", "Max HP", "Max MP"];
/// Stats 6..11: the core stat each grows from, and by how much per point (CalculateDependentStats).
const DEP: [(usize, f64); 6] = [(0, 0.5), (4, 0.5), (1, 0.5), (3, 0.5), (2, 0.1), (3, 0.1)];
/// The CorrectTbl index of each of the 12 (Digitalize.Result.Stats).
const TBL: [i32; 12] = [0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 19, 20];
/// Game equip type (0 Head ... 14 Bullets) -> the item table's name for it.
const EQUIP_TYPE_NAMES: [&str; 15] = [
    "EQUIP_TYPE_HEAD",
    "EQUIP_TYPE_FACE",
    "EQUIP_TYPE_NECK",
    "EQUIP_TYPE_TOP",
    "EQUIP_TYPE_ARMS",
    "EQUIP_TYPE_BOTTOM",
    "EQUIP_TYPE_FEET",
    "EQUIP_TYPE_COMP",
    "EQUIP_TYPE_RING",
    "EQUIP_TYPE_EARRING",
    "EQUIP_TYPE_EXTRA",
    "EQUIP_TYPE_BACK",
    "EQUIP_TYPE_TALISMAN",
    "EQUIP_TYPE_WEAPON",
    "EQUIP_TYPE_BULLETS",
];
/// The top of every fusion bonus table (ItemEffects.FuseBonus): the game's "MAX".
const MAX_FUSE: i32 = 50;

static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("spaces pattern"));
static RE_SPECIAL_CONDITION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"special condition \d+").expect("special condition pattern"));
static RE_SEX_MARKS: LazyLock<Regex> = LazyLock::new(|| Regex::new("[♂♀]").expect("sex marks pattern"));

// ---------- the character as last read ----------

/// Per stat (the 12, then Max HP and Max MP): flat, percent and fusion bonus.
#[derive(Debug, Clone, Default)]
struct Sums {
    flat: [f64; 14],
    pct: [f64; 14],
    fuse: [f64; 14],
}

/// The worn pieces: one per slot, `None` where nothing is worn.
type Items = Vec<Option<Arc<WornItem>>>;

struct Base {
    /// No character: nothing worn, no expertise, every reading 0 (see `load_blank`).
    blank: bool,
    items: Items,
    /// The snapshot had no item records (some clients): the pieces are the worn items alone, with the tarot and soul
    /// read some other way when there were any, and no mods, source items or fusion bonuses. What those add is in the
    /// reading and stays in `resid`, so the totals still equal the game's; only changes to those parts aren't exact.
    gear_partial: bool,
    texts: Vec<String>,
    level: i32,
    stat_level: i32,
    base_core: [i32; 6],
    totals: [i32; 12],
    taken_at: Option<String>,
    cal: Option<Calibration>,
    alignment: String,
    set_table: Vec<SetRecord>,
    /// Which sets (indexes into `set_table`, in the table's order) each piece is in.
    sets_of: HashMap<i32, Vec<usize>>,
    /// Points of the regular expertises (10000 per rank), and what the saved chain ranks have over the chain formula on
    /// those (points below one rank are not saved, but count toward chains).
    points: IndexMap<i32, i32>,
    chain_extra: IndexMap<i32, i32>,
    /// Entries the reader can't place (no id): kept as they are.
    other_ranks: Vec<Value>,
    cap: Option<i64>,
    demon: Option<DigitalizeSave>,
    /// The character's COMP demons as last seen (the demon picker's list, by "type:name:level"), and the digitalize
    /// boosts last read: a build can be digitalized with any of them, not only the saved one.
    seen: IndexMap<String, DemonRecord>,
    assists: Option<Vec<i32>>,
    boost: bool,
    normal0: Sums,
    resid: [i32; 12],
    catalog: Mutex<HashMap<i32, String>>,
}

impl Base {
    fn empty() -> Base {
        Base {
            blank: false,
            items: Vec::new(),
            gear_partial: false,
            texts: Vec::new(),
            level: 0,
            stat_level: 0,
            base_core: [0; 6],
            totals: [0; 12],
            taken_at: None,
            cal: None,
            alignment: String::new(),
            set_table: Vec::new(),
            sets_of: HashMap::new(),
            points: IndexMap::new(),
            chain_extra: IndexMap::new(),
            other_ranks: Vec::new(),
            cap: None,
            demon: None,
            seen: IndexMap::new(),
            assists: None,
            boost: false,
            normal0: Sums::default(),
            resid: [0; 12],
            catalog: Mutex::new(HashMap::new()),
        }
    }

    /// The saved digitalize demon's key in the demon list: the entry that is the same demon (two Specter Lv 90s in one
    /// COMP have "...:90" and "...:90#2"), else its kind and level.
    fn saved_demon_key(&self) -> Option<String> {
        let d = self.demon.as_ref().and_then(|d| d.demon.as_ref())?;
        Some(pipeline::find_key(self.seen.iter(), d).unwrap_or_else(|| pipeline::key_of(d)))
    }
}

#[derive(Default)]
struct State {
    base: Option<Arc<Base>>,
    stamp: Vec<u128>,
    blanks: HashMap<String, Arc<Base>>,
}

/// A build: the worn pieces and the expertise points.
struct Build {
    items: Items,
    points: IndexMap<i32, i32>,
    /// The build's digitalize demon: None = the one saved for the character, "" = none, else the "type:name:level" of a COMP demon seen.
    demon: Option<String>,
}

struct Computed {
    gear: GearResult,
    parts: Vec<Part>,
    rows: Vec<Row>,
    demon: Option<DigitalizeResult>,
    ranks: IndexMap<i32, i32>,
    names: Vec<Option<String>>,
}

struct RankSet {
    entries: Vec<Value>,
    ranks: IndexMap<i32, i32>,
    classes: IndexMap<String, i32>,
}

/// `new Simulator(store, data, dash, expertise, blankSets, icons)`.
///
/// `blank_sets`: given = this simulator has NO character behind it ("no base"): a build starts empty and the result is
/// only what the build itself gives. The function supplies the game's set table (read from the game at any character's
/// snapshot). The store is then a folder that need not exist. `icons`: where item icons are cached (a character's
/// store), when not this simulator's own store.
pub struct Simulator {
    ctx: Arc<Context>,
    dash: Dashboard,
    exp: Expertise,
    blank_sets: Option<BlankSets>,
    icons: Option<Store>,
    state: Mutex<State>,
}

/// The worn pieces of a snapshot without item records (`recs` in slot order, 15 of them): each worn item as itself,
/// with the tarot / soul the snapshot saved (0 = none or not read), no mods, source items or fusion bonuses.
fn pieces_without_records(recs: &[crate::data::store::SlotTip]) -> Vec<Option<WornItem>> {
    recs.iter()
        .map(|r| {
            (r.id > 0).then(|| WornItem {
                id: r.id,
                tarot: r.tarot.unwrap_or(0),
                soul: r.soul.unwrap_or(0),
                basic: 0,
                special: 0,
                mods: vec![0; 5],
                fuse: vec![0; 3],
                durability: -1,
                max_durability: -1,
            })
        })
        .collect()
}

fn basic_of(w: &WornItem) -> i32 {
    if w.basic > 0 { w.basic } else { w.id }
}

fn special_of(w: &WornItem) -> i32 {
    if w.special > 0 { w.special } else { w.id }
}

fn same(a: &WornItem, b: &WornItem) -> bool {
    a.id == b.id && basic_of(a) == basic_of(b) && special_of(a) == special_of(b) && a.tarot == b.tarot && a.soul == b.soul && a.fuse == b.fuse && a.mods == b.mods
}

fn ints(n: Option<&Value>, count: usize) -> Vec<i32> {
    let mut a: Vec<i32> = json::arr(n).map(|x| json::int(Some(x))).take(count).collect();
    while a.len() < count {
        a.push(0);
    }
    a
}

/// `string.Equals(a, b, OrdinalIgnoreCase)`-style ordering: by the upper-cased UTF-16 units.
fn ic_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn up(s: &str) -> impl Iterator<Item = u16> + '_ {
        s.chars()
            .flat_map(|c| {
                let mut u = c.to_uppercase();
                match (u.next(), u.next()) {
                    (Some(x), None) => x,
                    _ => c, // a character .NET's simple mapping leaves alone
                }
                .encode_utf16(&mut [0; 2])
                .to_vec()
            })
    }
    up(a).cmp(up(b))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn wide(i: usize) -> i32 {
    i as i32
}

impl Simulator {
    pub fn new(ctx: Arc<Context>, blank_sets: Option<BlankSets>, icons: Option<Store>) -> Simulator {
        Simulator { dash: Dashboard::new(Arc::clone(&ctx)), exp: Expertise::new(Arc::clone(&ctx)), ctx, blank_sets, icons, state: Mutex::new(State::default()) }
    }

    fn store(&self) -> &Store {
        &self.ctx.store
    }

    fn data(&self) -> &Arc<GameData> {
        &self.ctx.data
    }

    // ---------- the character as last read ----------

    /// The base without a character, for a level and an alignment (the page's "level" and "alignment": they decide
    /// level-scaled effects and "If you are LAW" lines). Nothing worn, no expertise, no learned skills, no demon. Every
    /// reading is 0 and the stat level is 0, so the 12 stats come out as what the build adds and nothing else
    /// (STR is its flat bonuses; CLS-R is floor(that x 0.5) + its own). Max HP / MP are not given: there is no body.
    fn load_blank(&self, st: &mut State, opts: Option<&Value>) -> Result<Arc<Base>> {
        let level = json::int_or(opts.at("level"), 99).clamp(1, 99);
        let mut align = json::str(opts.at("alignment")).unwrap_or_default().to_uppercase();
        if !matches!(align.as_str(), "LAW" | "NEUTRAL" | "CHAOS") {
            align = "NEUTRAL".into();
        }
        let key = format!("{level}|{align}");
        if let Some(hit) = st.blanks.get(&key) {
            return Ok(Arc::clone(hit));
        }
        let blank_sets = self.blank_sets.as_ref().ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
        let sets = blank_sets()?;
        let sets = match sets {
            Some(s) if !s.is_empty() => s,
            _ => return Err(user_error("A build without a base still needs the game's list of sets, which is read from the game. Take one snapshot of any character first.")),
        };
        let mut b = Base::empty();
        b.blank = true;
        b.items = vec![None; 15];
        b.texts = vec![String::new(); 15];
        b.level = level;
        b.stat_level = 0;
        b.alignment = align;
        b.set_table = sets;
        self.anchor(&mut b)?;
        let b = Arc::new(b);
        st.blanks.insert(key, Arc::clone(&b));
        Ok(b)
    }

    /// Finishes a base: which sets each piece is in, and what the readings hold beyond the model of the build as read.
    fn anchor(&self, b: &mut Base) -> Result<()> {
        let mut sets_of: HashMap<i32, Vec<usize>> = HashMap::new();
        for (i, s) in b.set_table.iter().enumerate() {
            let mut done: Vec<i32> = Vec::new();
            for &p in s.pieces.iter().filter(|&&p| p != 0) {
                if done.contains(&p) {
                    continue; // Distinct()
                }
                done.push(p);
                sets_of.entry(p).or_default().push(i);
            }
        }
        b.sets_of = sets_of;
        // What the reading holds beyond the model of the build as read (kept as it is in every simulation)
        let parts0 = self.compute(b, &Build { items: b.items.clone(), points: b.points.clone(), demon: None }, &b.totals)?.parts;
        let normal0 = sum_stats(&parts0, false);
        let zero = [0; 12];
        let core = model(b, &normal0, None, &zero, None);
        let dep = model(b, &normal0, None, &zero, Some(&b.totals));
        let mut resid = [0; 12];
        for i in 0..12 {
            resid[i] = b.totals[i].wrapping_sub(if i < 6 { core[i] } else { dep[i] });
        }
        b.normal0 = normal0;
        b.resid = resid;
        Ok(())
    }

    /// `opts`: the page's request: only read without a character (its "level" and "alignment").
    fn load(&self, st: &mut State, opts: Option<&Value>) -> Result<Arc<Base>> {
        if self.blank_sets.is_some() {
            return self.load_blank(st, opts);
        }
        let store = self.store();
        let data = self.data();
        let files = [
            store.tooltips_path(),
            store.last_stats_path(),
            store.expertise_ranks_path(),
            store.profile_path(),
            store.set_table_path(),
            store.digitalize_path(),
            store.comp_demons_path(),
            store.digitalize_progress_path(),
        ];
        let stamp: Vec<u128> = files
            .iter()
            .map(|f| {
                std::fs::metadata(f)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_nanos() + 1)
            })
            .collect();
        if let Some(b) = &st.base {
            if stamp == st.stamp {
                return Ok(Arc::clone(b));
            }
        }

        if !store.tooltips_path().exists() || !store.last_stats_path().exists() {
            return Err(user_error("The Build Planner starts from your character as the game last showed it. Take a snapshot first."));
        }
        let mut recs = store.load_tooltips()?;
        recs.sort_by_key(|r| r.index);
        let sets = if store.set_table_path().exists() { store.load_set_table()? } else { None };
        let Some(sets) = sets.filter(|s| !s.is_empty()) else {
            return Err(user_error("The Build Planner needs the game's equipment sets, which the saved snapshot doesn't have. Take a new snapshot with the game running."));
        };
        if recs.len() != 15 {
            return Err(user_error("The saved worn gear is incomplete. Take a new snapshot with the game running."));
        }
        let (items, gear_partial) = match Store::unpack_details(&recs) {
            Some(i) => (i, false),
            None => (pieces_without_records(&recs), true),
        };

        let last = json::load(store.last_stats_path())?;
        let pairs: Vec<i32> = json::arr(last.at("pairs")).map(|p| json::int(Some(p))).collect();
        if pairs.len() < 24 {
            return Err(user_error("The saved stats are incomplete. Take a new snapshot."));
        }
        let mut b = Base::empty();
        b.items = items.into_iter().map(|w| w.map(Arc::new)).collect();
        b.gear_partial = gear_partial;
        b.texts = recs.iter().map(|r| r.text.clone()).collect();
        b.stat_level = json::int(last.at("level"));
        b.taken_at = json::str(last.at("taken_at"));
        for i in 0..6 {
            b.base_core[i] = pairs[2 * i];
        }
        for i in 0..12 {
            b.totals[i] = pairs[2 * i].wrapping_add(pairs[2 * i + 1]);
        }
        b.alignment = store.alignment()?;
        b.set_table = sets;
        b.demon = DigitalizeSave::load(store)?;
        let lv = store.tooltips_level()?;
        b.level = if lv > 0 { lv } else { b.stat_level };
        let toggles = store.active_toggles()?;
        b.cal = character_hp_mp::from_saved_stats(&store.last_stats_path(), data.passives()?, Some(&toggles))?.0;
        if store.comp_demons_path().exists() {
            for (key, seen) in pipeline::load_seen(store)? {
                b.seen.insert(key, seen.demon); // catch (JsonException) { }: load_seen gives an empty list for an unreadable one
            }
        }
        let progress = DigitalizeSave::load_progress(store); // as switching demon in the app does: the boosts last read, else the saved demon's
        b.assists = match &progress {
            Some(p) if p.assists.is_some() => p.assists.clone(),
            _ => b.demon.as_ref().and_then(|d| d.assists.clone()),
        };
        b.boost = match &progress {
            Some(p) => p.mitama_boost,
            None => b.demon.as_ref().is_some_and(|d| d.mitama_boost),
        };

        if store.expertise_ranks_path().exists() {
            let doc = json::load(store.expertise_ranks_path())?;
            let mut chains: IndexMap<i32, i32> = IndexMap::new();
            for r in json::arr(doc.at("ranks")) {
                if r.at("id").is_none() || json::truthy(r.at("unknown")) {
                    b.other_ranks.push(r.clone());
                    continue;
                }
                let id = json::int(r.at("id"));
                let dflt = json::int(r.at("class")).wrapping_mul(10).wrapping_add(json::int(r.at("rank"))).wrapping_mul(10000);
                let pts = json::int_or(r.at("points"), dflt);
                if json::truthy(r.at("chain")) {
                    chains.insert(id, pts);
                } else {
                    b.points.insert(id, pts);
                }
            }
            let computed: HashMap<i32, i32> = Expertise::chain_ranks(data, &b.points)?.iter().map(|r| (json::int(r.get("id")), json::int(r.get("points")))).collect();
            for (id, pts) in &chains {
                b.chain_extra.insert(*id, pts.wrapping_sub(computed.get(id).copied().unwrap_or(0)));
            }
            if let Some(cap) = json::num(doc.at("points").at("cap")) {
                b.cap = Some(cap as i64);
            }
        }

        self.anchor(&mut b)?;
        let b = Arc::new(b);
        st.base = Some(Arc::clone(&b));
        st.stamp = stamp;
        Ok(b)
    }

    // ---------- a build ----------

    /// The page's build: { slots: [null | {id, basic, special, tarot, soul, fuse[3], mods[5]}] x 15, points: {expertise id: points} }.
    fn parse(&self, b: &Base, spec: Option<&Value>) -> Result<Build> {
        let mut s = Build { items: b.items.clone(), points: b.points.clone(), demon: None };
        if let Some(Value::Array(slots)) = spec.at("slots") {
            for k in 0..15 {
                let n = slots.get(k).filter(|v| !v.is_null());
                let id = json::int(n.at("id"));
                let Some(n) = n.filter(|_| id > 0) else {
                    s.items[k] = None;
                    continue;
                };
                if !self.data().items()?.contains_key(&id) {
                    return Err(user_error("One of the pieces isn't in the game's item list."));
                }
                let w = WornItem {
                    id,
                    tarot: json::int(n.at("tarot")).max(0),
                    soul: json::int(n.at("soul")).max(0),
                    basic: json::int(n.at("basic")),
                    special: json::int(n.at("special")),
                    mods: ints(n.at("mods"), 5),
                    fuse: ints(n.at("fuse"), 3),
                    durability: 1,
                    max_durability: 1,
                };
                // The piece as read keeps its own record (durability included); a changed one is taken as whole.
                s.items[k] = match &b.items[k] {
                    Some(old) if same(&w, old) => Some(Arc::clone(old)),
                    _ => Some(Arc::new(self.clean(w, k)?)),
                };
            }
        }
        if let Some(Value::Object(pts)) = spec.at("points") {
            for (key, v) in pts {
                if let Some(id) = crate::fmt::parse_i32(key) {
                    s.points.insert(id, json::int(Some(v)).max(0));
                }
            }
        }
        if let Some(d) = spec.at("demon") {
            s.demon = Some(json::str(Some(d)).unwrap_or_default());
        }
        Ok(s)
    }

    /// Mods belong to the piece: on another item they only stay where the mod tables have that effect for it.
    fn clean(&self, w: WornItem, slot: usize) -> Result<WornItem> {
        let data = self.data();
        let equip_type = client_reader::equip_type_of_slot(wide(slot));
        let weapon = equip_type == 13;
        let mut mods = Vec::with_capacity(w.mods.len());
        for &m in &w.mods {
            let keep = m == 0 || m == 255 || data.mod_tokusei(weapon, data.item_sub_category(w.id)?, equip_type, m)? != 0;
            mods.push(if keep { m } else { 0 });
        }
        Ok(WornItem { mods, ..w })
    }

    fn spec_of(items: &Items) -> Value {
        Value::Array(
            items
                .iter()
                .map(|w| match w {
                    None => Value::Null,
                    Some(w) => json!({
                        "id": w.id, "basic": basic_of(w), "special": special_of(w), "tarot": w.tarot, "soul": w.soul,
                        "fuse": w.fuse, "mods": w.mods,
                    }),
                })
                .collect(),
        )
    }

    // ---------- expertise ----------

    /// The regular expertises' points as rank entries (expertise-ranks.json "ranks"), with the chains they give.
    fn ranks(&self, b: &Base, points: &IndexMap<i32, i32>) -> Result<RankSet> {
        let data = self.data();
        let name_of = |id: i32| -> Result<String> {
            Ok(match data.expertises_client()?.get(&id) {
                Some(d) => d.name.clone(),
                None => json::str(data.expertises()?.at(&id.to_string()).at("name")).unwrap_or_default(),
            })
        };
        let mut rows: Vec<Row> = Vec::new();
        let mut sorted: Vec<(i32, i32)> = points.iter().map(|(k, v)| (*k, *v)).collect();
        sorted.sort_by_key(|kv| kv.0);
        for &(id, pts) in &sorted {
            if pts < 10000 {
                continue;
            }
            let ri = pts / 10000;
            rows.push(obj(json!({ "id": id, "name": name_of(id)?, "class": ri / 10, "rank": ri % 10, "points": pts })));
        }
        // Chains: the formula on these points, plus what the saved ranks had over it (see Base.chain_extra)
        let mut chains = Expertise::chain_ranks(data, points)?;
        let chain_ids: Vec<i32> = chains.iter().map(|c| json::int(c.get("id"))).collect();
        for mut c in std::mem::take(&mut chains) {
            let id = json::int(c.get("id"));
            let pts = json::int(c.get("points")).wrapping_add(b.chain_extra.get(&id).copied().unwrap_or(0));
            let ri = pts / 10000;
            if pts < 10000 {
                continue;
            }
            c.insert("points".into(), json!(pts));
            c.insert("class".into(), json!(ri / 10));
            c.insert("rank".into(), json!(ri % 10));
            rows.push(c);
        }
        // A saved chain the formula no longer reaches on unchanged points (never seen; kept for safety)
        let mut mine: Vec<(i32, i32)> = points.iter().map(|(k, v)| (*k, *v)).collect();
        mine.sort_by_key(|kv| kv.0);
        let mut base_pts: Vec<(i32, i32)> = b.points.iter().map(|(k, v)| (*k, *v)).collect();
        base_pts.sort_by_key(|kv| kv.0);
        for (&id, &extra) in &b.chain_extra {
            if extra >= 10000 && !chain_ids.contains(&id) && mine == base_pts {
                rows.push(obj(json!({ "id": id, "name": name_of(id)?, "class": extra / 100000, "rank": extra / 10000 % 10, "points": extra, "chain": true })));
            }
        }
        let mut ranks: IndexMap<i32, i32> = IndexMap::new();
        for r in &rows {
            ranks.insert(json::int(r.get("id")), json::int(r.get("class")).wrapping_mul(10).wrapping_add(json::int(r.get("rank"))));
        }
        let mut classes: IndexMap<String, i32> = IndexMap::new();
        for r in &rows {
            if let Some(nm) = r.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()) {
                classes.insert(ic_key(nm), json::int(r.get("class")));
            }
        }
        let mut entries: Vec<Value> = rows.into_iter().map(Value::Object).collect();
        entries.extend(b.other_ranks.iter().cloned());
        for o in &b.other_ranks {
            if let Some(nm) = json::str(o.at("name")).filter(|n| !n.is_empty()) {
                classes.insert(ic_key(&nm), json::int(o.at("class")));
            }
        }
        Ok(RankSet { entries, ranks, classes })
    }

    // ---------- the bonuses of a build ----------

    fn plain(text: &str) -> Option<String> {
        if text.is_empty() { None } else { Some(TooltipParser::info(text).base_name) }
    }

    /// Everything a build gives, with the given stats behind stat-scaled effects (totals, the reading's order).
    fn compute(&self, b: &Base, s: &Build, stats: &[i32; 12]) -> Result<Computed> {
        let data = self.data();
        let RankSet { entries, ranks, classes } = self.ranks(b, &s.points)?;
        let mut texts = vec![String::new(); 15];
        for k in 0..15 {
            // a tooltip only describes the piece it was read from: another item in the slot has none
            if let (Some(w), Some(w0)) = (&s.items[k], &b.items[k]) {
                if w.id == w0.id {
                    texts[k] = b.texts[k].clone();
                }
            }
        }
        // ... and of that piece, only the parts that are still what they were
        let tip_ok = |k: i32, source: &str| -> bool {
            let k = k as usize;
            let (Some(w), Some(w0)) = (&s.items[k], &b.items[k]) else { return false };
            if w.id != w0.id {
                return false;
            }
            if Arc::ptr_eq(w, w0) {
                return true;
            }
            match source {
                "item" => basic_of(w) == basic_of(w0) && w.fuse == w0.fuse,
                "characteristic" => special_of(w) == special_of(w0),
                "tarot" => w.tarot == w0.tarot,
                "soul" => w.soul == w0.soul,
                _ => true,
            }
        };
        let ids: Vec<i32> = s.items.iter().map(|w| w.as_ref().map_or(0, |w| w.id)).collect();
        let details: Vec<Option<WornItem>> = s.items.iter().map(|w| w.as_ref().map(|w| (**w).clone())).collect();
        let gear = GearEffects::new(&b.alignment, &classes, Arc::clone(data), Some(b.set_table.clone()), Some(ranks.clone()))
            .compute(&texts, Some(&ids), None, None, Some(&details), b.level, Some(&tip_ok))?;

        let mut names: Vec<Option<String>> = vec![None; 15];
        let mut short_name: IndexMap<String, String> = IndexMap::new();
        for k in 0..15 {
            if ids[k] <= 0 {
                continue;
            }
            let name = match Self::plain(&texts[k]) {
                Some(n) => n,
                None => data.item_name(ids[k])?.unwrap_or_else(|| format!("item {}", ids[k])),
            };
            short_name.insert(ic_key(&name), data.item_name(ids[k])?.unwrap_or_else(|| name.clone()));
            names[k] = Some(name);
        }
        // The build's digitalize demon: the one saved for the character (with the boosts saved with it: what the page
        // data uses), any other COMP demon seen (with the boosts last read, as switching demon in the app does), or none.
        let mut demon: Option<DigitalizeResult> = None;
        let saved_key = b.saved_demon_key();
        let saved = match &s.demon {
            None => true,
            Some(d) => saved_key.as_deref() == Some(d.as_str()),
        };
        let record: Option<&DemonRecord> = if saved {
            b.demon.as_ref().and_then(|d| d.demon.as_ref())
        } else {
            let want = s.demon.as_deref().unwrap_or("");
            if want.is_empty() {
                None
            } else {
                // (a key from before the level was part of it)
                b.seen.get(want).or_else(|| b.seen.iter().find(|(k, _)| k.starts_with(&format!("{want}:"))).map(|(_, v)| v))
            }
        };
        if let Some(record) = record.filter(|_| !b.blank) {
            let (assists, boost) = if saved {
                let d = b.demon.as_ref().ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
                (d.assists.as_deref(), d.mitama_boost)
            } else {
                (b.assists.as_deref(), b.boost)
            };
            demon = Some(Digitalize::new(Arc::clone(data), ItemEffects::new(Arc::clone(data), &b.alignment, b.level, ranks.clone())).compute(record, assists, boost)?);
        }
        let mut snap = Row::new();
        for i in 0..12 {
            snap.insert(STAT_KEYS[i].into(), json!(stats[i]));
        }
        let expertise_rows = self.exp.rows(Some(&Value::Object(snap)), &entries, false)?;
        let mut parts = Dashboard::parts(&gear, &short_name, demon.as_ref(), &expertise_rows);
        // What the build's VIT, INT and SPD take off cooldown and incantation time (as in bonuses.json)
        if !b.blank {
            let normal = [stats[2], stats[3], stats[4]];
            parts.extend(Dashboard::stat_time_parts(normal, demon.as_ref().map(|d| Dashboard::digitalized_time_stats(normal, d, &gear.effects))));
        }
        // The partner summoned beside the character gives it what its tokusei aim at the summoner (as in bonuses.json)
        if let Some(partner) = Dashboard::summoned_demon(b.seen.values()).filter(|_| !b.blank) {
            let calc = Digitalize::new(Arc::clone(data), ItemEffects::new(Arc::clone(data), &b.alignment, b.level, ranks.clone()));
            parts.extend(Dashboard::summoner_parts_of(&calc.summoner_effects(partner, b.boost)?));
        }
        // BonusRows labels the parts it is given (the C# parts are shared with the caller)
        Dashboard::label_parts(&mut parts);
        let rows = self.dash.bonus_rows(parts.clone(), None)?;
        Ok(Computed { gear, parts, rows, demon, ranks, names })
    }

    // ---------- stats ----------

    fn stats_row(b: &Base, v: &[i32; 12], s: &Sums, demon: Option<&DigitalizeResult>) -> Row {
        let mut row = Row::new();
        for i in 0..12 {
            // without a character these are what the build adds, which can be below 0
            row.insert(STAT_KEYS[i].into(), json!(if b.blank { v[i] } else { v[i].max(0) }));
        }
        let Some(cal) = &b.cal else { return row };
        let digi = demon.is_some();
        let (hp, mp) = character_hp_mp::formula(b.stat_level, v[0], v[1], v[2], v[3]);
        // percent: the toggles of the reading (normal: the reading itself must come back), of now while digitalized
        // (as the Digitalize page does), plus what the build changed
        let pct_hp = (if digi { cal.now_pct_hp.unwrap_or(cal.pct_hp) } else { cal.pct_hp }) + s.pct[12] - b.normal0.pct[12];
        let pct_mp = (if digi { cal.now_pct_mp.unwrap_or(cal.pct_mp) } else { cal.pct_mp }) + s.pct[13] - b.normal0.pct[13];
        let (d_hp, d_mp) = demon.map_or((0, 0), |d| (d.hp(), d.mp()));
        row.insert("max_hp".into(), json!(character_hp_mp::adjust(hp.wrapping_add(d_hp), cal.flat_hp() as f64 + s.flat[12] - b.normal0.flat[12], pct_hp, 0.0)));
        row.insert("max_mp".into(), json!(character_hp_mp::adjust(mp.wrapping_add(d_mp), cal.flat_mp() as f64 + s.flat[13] - b.normal0.flat[13], pct_mp, 0.0)));
        row
    }

    // ---------- wording ----------

    /// One effect as a line for the page: t (text), c (always | digitalized | situational | inactive | unknown; left out
    /// when always), n (the note; left out when empty). None for lines that are not shown (fusion-set text the set
    /// table decides, raw table ids the reader has no words for).
    fn line(&self, e: &Effect) -> Result<Option<Row>> {
        if e.cond == "fusion-set" {
            return Ok(None);
        }
        let mut cond = if e.cond == "unparsed" { "unknown".to_string() } else { e.cond.clone() };
        let text;
        if e.missing_data || e.label.starts_with("Unknown effect") {
            // never the internal number: the game's own description, or a plain sentence
            let own = if e.tokusei > 0 {
                let t = self.data().client_tokusei_text(e.tokusei)?.unwrap_or_default().replace("\\n", " ");
                RE_SPACES.replace_all(&t, " ").trim().to_string()
            } else {
                String::new()
            };
            text = if !own.is_empty() { own } else { "An effect the game data has no numbers for".to_string() };
            cond = "unknown".into();
        } else {
            if gear_db::no_words(&e.label) {
                return Ok(None);
            }
            let v = match e.value {
                Some(d) => format!(" {}{}{}", if d > 0.0 { "+" } else if d < 0.0 { "−" } else { "" }, crate::fmt::custom(d.abs(), "0.##"), if e.pct { "%" } else { "" }),
                None => String::new(),
            };
            text = format!("{}{}{}", if matches!(e.target.as_str(), "self" | "") { "" } else { "Partner: " }, e.label, v);
        }
        let note = RE_SPECIAL_CONDITION
            .replace_all(&e.cond_note, "under a condition the game data does not explain")
            .replace("from the tooltip: no server data", "as the game's tooltip says it")
            .replace("from the client's text (no server data)", "as the game's text says it");
        let mut row = Row::new();
        row.insert("t".into(), json!(text));
        if cond != "always" {
            row.insert("c".into(), json!(cond));
        }
        if !note.is_empty() {
            row.insert("n".into(), json!(note));
        }
        Ok(Some(row))
    }

    fn lines<'a>(&self, fx: impl IntoIterator<Item = &'a Effect>) -> Result<Vec<Row>> {
        let mut res = Vec::new();
        for e in fx {
            if let Some(l) = self.line(e)? {
                res.push(l);
            }
        }
        Ok(res)
    }

    fn item_name(&self, id: i32) -> Result<String> {
        Ok(match self.data().item_name(id)? {
            Some(n) if !n.is_empty() => n,
            _ => "A piece the game data has no name for".to_string(),
        })
    }

    fn crystal_name(&self, id: i32, tarot: bool) -> Result<String> {
        Ok(self.data().crystal_name(id, Some(tarot))?.unwrap_or_else(|| "A crystal the game data has no name for".to_string()))
    }

    fn icon(&self, id: i32) -> Result<Option<String>> {
        let store = self.icons.as_ref().unwrap_or_else(|| self.store());
        match item_icons::data_uri(store, self.data(), id, 0) {
            Ok(v) => Ok(v),
            Err(e) if e.downcast_ref::<std::io::Error>().is_some() => Ok(None), // catch (IOException)
            Err(e) => Err(e),
        }
    }

    /// The picture of a crystal fused in as tarot or soul (the crystal's own item), or None.
    fn crystal_icon(&self, id: i32, tarot: bool) -> Result<Option<String>> {
        if id <= 0 {
            return Ok(None);
        }
        match self.data().enchant_sides()?.get(&(id, tarot)).map(|side| side.crystal_item).filter(|&item| item > 0) {
            Some(item) => self.icon(item),
            None => Ok(None),
        }
    }

    /// Who can wear a piece: "Male" or "Female" (the item list's gender, else the mark in its name), None for anyone or not known.
    fn sex_of(&self, id: i32) -> Result<Option<&'static str>> {
        let Some(n) = self.data().items()?.get(&id) else { return Ok(None) };
        let name = json::str(n.at("name")).unwrap_or_default();
        Ok(match json::num(n.at("gender")) {
            Some(g) if g == 0.0 => Some("Male"),
            Some(g) if g == 1.0 => Some("Female"),
            Some(g) if g == 2.0 => None,
            _ => {
                if name.contains('♂') {
                    Some("Male")
                } else if name.contains('♀') {
                    Some("Female")
                } else {
                    None
                }
            }
        })
    }

    /// What the fusion bonus tables give this item at MAX, per stat (empty: this kind of piece has none).
    fn fuse_at_max(fx: &ItemEffects, w: &WornItem) -> Result<Vec<Row>> {
        let at_max = WornItem { fuse: vec![MAX_FUSE; 3], ..w.clone() };
        Ok(fx.fuse_bonus(&at_max)?.into_iter().map(|(stat, boost, _)| obj(json!({ "stat": fuse_stat_label(stat), "bonus": boost }))).collect())
    }

    // ---------- commands ----------

    /// The build's result for the page: stats (normal; digitalized when a demon is saved), the bonus list, each slot
    /// with what its parts give, the sets, and the expertise ranks with the points spent.
    fn result(&self, b: &Base, s: &Build) -> Result<Row> {
        let data = self.data();
        let mut c = self.compute(b, s, &b.totals)?;
        let mut normal = sum_stats(&c.parts, false);
        let mut v = model(b, &normal, None, &b.resid, None);
        if v != b.totals {
            // stat-scaled effects (a rate that grows with LNG-R, say) follow the new stats
            c = self.compute(b, s, &v)?;
            normal = sum_stats(&c.parts, false);
            v = model(b, &normal, None, &b.resid, None);
        }
        let mut digi_row: Option<Row> = None;
        if let Some(d) = c.demon.as_ref().filter(|d| d.devil.is_some()) {
            let mut add = [0i32; 12];
            for i in 0..12 {
                add[i] = d.stats.get(&TBL[i]).copied().unwrap_or(0);
            }
            let ds = sum_stats(&c.parts, true);
            digi_row = Some(Self::stats_row(b, &model(b, &ds, Some(&add), &b.resid, None), &ds, Some(d)));
        }

        let fx = ItemEffects::new(Arc::clone(data), &b.alignment, b.level, c.ranks.clone());
        let mut slots: Vec<Row> = Vec::new();
        for k in 0..15 {
            let mut o = Row::new();
            o.insert("slot".into(), json!(SLOT_NAMES[k]));
            let Some(w) = &s.items[k] else {
                o.insert("empty".into(), json!(true));
                slots.push(o);
                continue;
            };
            let name = c.names[k].clone().unwrap_or_default();
            let mine: Vec<&Effect> = c.gear.effects.iter().filter(|e| eq_ic(&e.item, &name)).collect();
            o.insert("id".into(), json!(w.id));
            o.insert("name".into(), json!(self.item_name(w.id)?));
            o.insert("icon".into(), json!(self.icon(w.id)?));
            o.insert("basic".into(), if basic_of(w) == w.id { Value::Null } else { json!(self.item_name(basic_of(w))?) });
            o.insert("special".into(), if special_of(w) == w.id { Value::Null } else { json!(self.item_name(special_of(w))?) });
            o.insert("tarot".into(), if w.tarot > 0 { json!(self.crystal_name(w.tarot, true)?) } else { Value::Null });
            o.insert("soul".into(), if w.soul > 0 { json!(self.crystal_name(w.soul, false)?) } else { Value::Null });
            // the pictures of what each part comes from (the page's piece tooltip): the pieces lending their basic
            // features and characteristics (none when it is the piece itself: "icon" above), and the two crystals
            o.insert("basic_icon".into(), if basic_of(w) == w.id { Value::Null } else { json!(self.icon(basic_of(w))?) });
            o.insert("special_icon".into(), if special_of(w) == w.id { Value::Null } else { json!(self.icon(special_of(w))?) });
            o.insert("tarot_icon".into(), json!(self.crystal_icon(w.tarot, true)?));
            o.insert("soul_icon".into(), json!(self.crystal_icon(w.soul, false)?));
            let max = Self::fuse_at_max(&fx, w)?;
            let now = fx.fuse_bonus(w)?;
            let limit = !max.is_empty() && now.len() == max.len() && now.iter().all(|f| f.2);
            o.insert("fuse_max".into(), rows_value(max));
            o.insert(
                "fuse".into(),
                rows_value(now.iter().map(|&(stat, boost, mx)| obj(json!({ "stat": fuse_stat_label(stat), "bonus": boost, "max": mx }))).collect()),
            );
            o.insert("limit".into(), json!(limit));
            let mut lines = Row::new();
            for src in ["item", "characteristic", "tarot", "soul", "mod", "appearance"] {
                lines.insert(src.into(), rows_value(self.lines(mine.iter().copied().filter(|e| e.source == src))?));
            }
            o.insert("lines".into(), Value::Object(lines));
            slots.push(o);
        }

        let mut worn_sets: Vec<&crate::calc::gear_effects::SetReport> = c.gear.sets.iter().filter(|x| !x.appearance).collect();
        worn_sets.sort_by_key(|x| !x.active); // OrderByDescending(x => x.Active), stable
        let mut sets: Vec<Row> = Vec::new();
        for x in worn_sets {
            sets.push(obj(json!({
                "kind": if x.fusion { "fusion" } else { "set" }, "name": RE_SEX_MARKS.replace_all(&x.pieces, ""), "worn": x.worn, "active": x.active,
                "lines": rows_value(self.lines(&x.effects)?),
                // a gear set's pieces as pictures, in the set's order (a fusion set is made of crystals: none here)
                "icons": if x.fusion { Vec::new() } else { x.piece_ids.iter().filter(|&&p| p > 0).map(|&p| self.icon(p)).collect::<Result<Vec<_>>>()? },
            })));
        }

        let rows: Vec<Row> = c
            .rows
            .iter()
            .map(|r| {
                let mut o = Row::new();
                for (k, val) in r {
                    if k.as_str() != "from" && k.as_str() != "parts" {
                        o.insert(k.clone(), val.clone());
                    }
                }
                let parts: Vec<Row> = match r.get("parts") {
                    Some(Value::Array(a)) => a
                        .iter()
                        .map(|p| {
                            let g = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
                            obj(json!({ "source": g("source"), "kind": g("kind"), "value": g("value"), "digi": g("digi"), "note": g("note") }))
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                o.insert("parts".into(), rows_value(parts));
                o
            })
            .collect();

        let ranks: Row = c.ranks.iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
        let total: i64 = s.points.iter().filter(|(k, _)| **k <= 37).map(|(_, v)| *v as i64).sum();
        Ok(obj(json!({
            "stats": Self::stats_row(b, &v, &normal, None),
            "digi": digi_row,
            "demon": c.demon.as_ref().filter(|d| d.devil.is_some()).map(|d| d.demon.name.clone()),
            "bonuses": rows_value(rows),
            "slots": rows_value(slots),
            "sets": rows_value(sets),
            "expertise": { "ranks": ranks, "total": total, "cap": b.cap },
        })))
    }

    /// The character as last read (the build to start from), the expertises that can be changed, and its result.
    pub fn init(&self, args: Option<&Value>) -> Result<String> {
        let mut st = lock(&self.state);
        let b = self.load(&mut st, args)?;
        let data = self.data();
        let build = Build { items: b.items.clone(), points: b.points.clone(), demon: None };
        let expertise_defs = data.expertises_client()?;
        let mut defs_src: Vec<&crate::data::game_data::ExpertiseDef> = expertise_defs.values().collect();
        defs_src.sort_by_key(|d| d.id);
        let mut defs: Vec<Row> = Vec::new();
        for d in defs_src {
            let pts = b.points.get(&d.id).copied().unwrap_or(0);
            if !(d.max_class > 0 || pts >= 10000 || d.is_chain && !d.chain.is_empty()) {
                continue;
            }
            let needs: Vec<Row> = d
                .chain
                .iter()
                .map(|&(source, min_rank, share)| obj(json!({ "id": source, "name": expertise_defs.get(&source).map_or("", |sd| sd.name.as_str()), "rank": min_rank, "share": share })))
                .collect();
            let mut by_rank: Vec<&(i32, Vec<i32>)> = data.rank_skills_client()?.get(&d.id).map(|l| l.iter().collect()).unwrap_or_default();
            by_rank.sort_by_key(|x| x.0);
            let mut unlocks: Vec<Row> = Vec::new();
            for (rank_idx, skills) in by_rank.into_iter().map(|x| (x.0, &x.1)) {
                let mut sk: Vec<Row> = Vec::new();
                for sid in skills {
                    let name = data.skill_names()?.get(sid).cloned().flatten().unwrap_or_else(|| "A skill the game data has no name for".to_string());
                    let passive = data.passives()?.get(&sid.to_string()).is_some_and(|p| !p.is_null());
                    sk.push(obj(json!({ "name": name, "passive": passive })));
                }
                // a rank that lets the character learn only some of its skills says how many
                unlocks.push(obj(json!({ "rank": rank_idx, "skills": rows_value(sk), "pick": data.rank_pick_count(d.id, rank_idx)? })));
            }
            defs.push(obj(json!({
                "id": d.id, "name": d.name, "chain": d.is_chain,
                // the top rank: the class the game data gives, or where the character already is
                "max": (d.max_class * 10).max(pts / 10000),
                "points": if d.is_chain { Value::Null } else { json!(pts) },
                "counts": !d.is_chain && d.id <= 37, // toward the expertise point cap (Expertise.Read)
                "needs": rows_value(needs),
                "unlocks": rows_value(unlocks),
            })));
        }
        let mut tarot = Vec::new();
        let mut soul = Vec::new();
        for k in 0..15 {
            let bit = 1i32.wrapping_shl(client_reader::equip_type_of_slot(wide(k)) as u32);
            tarot.push(data.enchant_sides()?.iter().any(|(key, side)| key.1 && side.mask & bit != 0));
            soul.push(data.enchant_sides()?.iter().any(|(key, side)| !key.1 && side.mask & bit != 0));
        }
        let mut sex: Option<&str> = None;
        for w in b.items.iter().flatten() {
            if let Some(x) = self.sex_of(w.id)? {
                sex = Some(x);
                break;
            }
        }
        let mut build_points = Row::new();
        for (k, v) in &b.points {
            build_points.insert(k.to_string(), json!(v));
        }
        let result = self.result(&b, &build)?;
        let out = obj(json!({
            "meta": {
                "level": b.level, "alignment": b.alignment, "stats_at": b.taken_at, "demon": b.demon.as_ref().and_then(|d| d.demon.as_ref()).map(|d| d.name.clone()),
                "blank": b.blank, // no character behind the build: the numbers are what the build gives by itself
                // the worn pieces' own records weren't read: no mods, source items or fusion bonuses on them
                "gear_partial": b.gear_partial,
                // the saved digitalize demon, as the demon list names it
                "demon_key": b.saved_demon_key(),
                "slots": SLOT_NAMES, "hp_known": b.cal.is_some(),
                // the character's sex, as far as the worn gear says it (pieces made for the other one can't be worn)
                "sex": sex,
                // which slots take a tarot / a soul fusion at all (the crystals' equip-type masks)
                "tarot": tarot,
                "soul": soul,
            },
            "build": { "slots": Self::spec_of(&b.items), "points": build_points },
            "expertise": rows_value(defs),
            "result": result,
        }));
        Ok(to_json(&Value::Object(out)))
    }

    /// The result of a build the page sends (see `parse`).
    pub fn run(&self, spec: Option<&Value>) -> Result<String> {
        let mut st = lock(&self.state);
        let b = self.load(&mut st, spec)?;
        let build = self.parse(&b, spec)?;
        Ok(to_json(&Value::Object(self.result(&b, &build)?)))
    }

    // ---------- sharing a build (share_code.rs) ----------

    /// The build the page sends, as a code holds it: whole (every slot, every expertise with points), numbers only.
    fn shared_of(spec: Option<&Value>, level: i32, alignment: &str) -> Shared {
        let mut slots: [Option<Piece>; share_code::SLOTS] = Default::default();
        let given = match spec.at("slots") {
            Some(Value::Array(a)) => Some(a),
            _ => None,
        };
        for (k, slot) in slots.iter_mut().enumerate() {
            let n = given.and_then(|a| a.get(k)).filter(|v| !v.is_null());
            let id = json::int(n.at("id"));
            if id <= 0 {
                continue;
            }
            let own = |v: i32| if v > 0 { v } else { id };
            let (fuse, mods) = (ints(n.at("fuse"), 3), ints(n.at("mods"), 5));
            *slot = Some(Piece {
                id,
                basic: own(json::int(n.at("basic"))),
                special: own(json::int(n.at("special"))),
                tarot: json::int(n.at("tarot")).max(0),
                soul: json::int(n.at("soul")).max(0),
                fuse: [fuse[0], fuse[1], fuse[2]],
                mods: [mods[0], mods[1], mods[2], mods[3], mods[4]],
            });
        }
        let mut points: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
        if let Some(Value::Object(pts)) = spec.at("points") {
            for (key, v) in pts {
                if let Some(id) = crate::fmt::parse_i32(key) {
                    points.insert(id, json::int(Some(v)));
                }
            }
        }
        let cap = json::num(spec.at("cap")).filter(|c| *c >= 0.0).map(|c| if c > f64::from(i32::MAX) { i32::MAX } else { c as i32 });
        Shared {
            level: level.clamp(1, share_code::MAX_LEVEL),
            alignment: share_code::ALIGNMENTS.iter().position(|a| a.eq_ignore_ascii_case(alignment)).unwrap_or(0),
            cap,
            slots,
            points: points.into_iter().filter(|&(_, pts)| pts > 0).collect(),
        }
    }

    /// Whether every number of a build names something the game data has, where it can go: pieces of the slot's kind,
    /// crystals that kind of piece takes on that side, fusion bonuses up to the game's MAX, expertises a character can
    /// raise, no further than their top rank. `Some(what is wrong)` when not. A code read from a stranger is only ever
    /// used after this.
    fn vet(&self, s: &Shared) -> Result<Option<String>> {
        let data = self.data();
        let items = data.items()?;
        let sides = data.enchant_sides()?;
        for (k, p) in s.slots.iter().enumerate() {
            let Some(p) = p else { continue };
            let slot = SLOT_NAMES[k];
            let equip_type = client_reader::equip_type_of_slot(wide(k));
            let Some(kind) = usize::try_from(equip_type).ok().and_then(|t| EQUIP_TYPE_NAMES.get(t)) else {
                return Ok(Some(format!("the {slot} slot takes no piece")));
            };
            let fits = |id: i32| items.get(&id).is_some_and(|node| json::str(node.at("equipType")).as_deref() == Some(*kind));
            if !fits(p.id) {
                return Ok(Some(format!("the piece in the {slot} slot is not a {slot} piece in the game data")));
            }
            if !fits(p.basic) || !fits(p.special) {
                return Ok(Some(format!("the {slot} piece carries the features of something that is not a {slot} piece in the game data")));
            }
            let bit = 1i32.wrapping_shl(equip_type as u32);
            let takes = |id: i32, tarot: bool| id == 0 || sides.get(&(id, tarot)).is_some_and(|side| side.mask & bit != 0);
            if !takes(p.tarot, true) || !takes(p.soul, false) {
                return Ok(Some(format!("the {slot} piece has a tarot or soul fusion the game data has no crystal for")));
            }
            if p.fuse.iter().any(|v| !(0..=MAX_FUSE).contains(v)) {
                return Ok(Some(format!("the {slot} piece has a fusion bonus above the game’s MAX")));
            }
            if p.mods.iter().any(|v| !(0..=0xFFFF).contains(v)) {
                return Ok(Some(format!("the {slot} piece has a mod the game has no number for")));
            }
        }
        let defs = data.expertises_client()?;
        for &(id, pts) in &s.points {
            match defs.get(&id) {
                Some(d) if !d.is_chain && d.max_class > 0 => {
                    if pts > d.max_class.saturating_mul(100_000) {
                        return Ok(Some(format!("{} is above its top rank", d.name)));
                    }
                }
                _ => return Ok(Some("it has points in an expertise the game data doesn’t let a character raise".to_string())),
            }
        }
        Ok(None)
    }

    /// The build with each piece's mods as the piece can have them (see `clean`). After `vet`: the pieces are real.
    fn tidied(&self, mut s: Shared) -> Result<Shared> {
        for (k, slot) in s.slots.iter_mut().enumerate() {
            if let Some(p) = slot {
                let w = WornItem {
                    id: p.id,
                    tarot: p.tarot,
                    soul: p.soul,
                    basic: p.basic,
                    special: p.special,
                    mods: p.mods.to_vec(),
                    fuse: p.fuse.to_vec(),
                    durability: 1,
                    max_durability: 1,
                };
                let mods = self.clean(w, k)?.mods;
                for (i, m) in p.mods.iter_mut().enumerate() {
                    *m = mods.get(i).copied().unwrap_or(0);
                }
            }
        }
        Ok(s)
    }

    /// "Share": the build the page sends (as `save_build` takes it, on its base) as a code for another player:
    /// { code }. The base's level and alignment go with it; the base's name, the build's name and its demon do not.
    pub fn share(&self, spec: Option<&Value>) -> Result<String> {
        let shared = {
            let mut st = lock(&self.state);
            let b = self.load(&mut st, spec)?;
            Self::shared_of(spec, b.level, &b.alignment)
        };
        if let Some(why) = self.vet(&shared)? {
            return Err(user_error(format!("This build can’t be shared: {why}.")));
        }
        let code = share_code::encode(&self.tidied(shared)?)?;
        Ok(to_json(&json!({ "code": code })))
    }

    /// "Import build": a code another player gave, as a build for the page: { build: { slots, points, cap, level,
    /// alignment } }, in the page's own format, or a user error saying in plain words why not. Nothing is saved here and
    /// nothing of the code reaches the page but numbers that passed `share_code::decode` and `vet`. The points list
    /// every expertise a character can raise (0 where the code has none): a build is whole.
    pub fn import(&self, args: Option<&Value>) -> Result<String> {
        let code = match args.at("code") {
            Some(Value::String(s)) => s.as_str(),
            _ => "",
        };
        let shared = share_code::decode(code)?;
        if self.vet(&shared)?.is_some() {
            return Err(user_error(
                "That build code names a piece, a crystal or an expertise rank this version of BABEL doesn’t have. It may have been made with a newer version.",
            ));
        }
        let shared = self.tidied(shared)?;
        let slots: Vec<Value> = shared
            .slots
            .iter()
            .map(|p| match p {
                None => Value::Null,
                Some(p) => json!({ "id": p.id, "basic": p.basic, "special": p.special, "tarot": p.tarot, "soul": p.soul, "fuse": p.fuse, "mods": p.mods }),
            })
            .collect();
        let mut ids: Vec<i32> = self.data().expertises_client()?.values().filter(|d| !d.is_chain && d.max_class > 0).map(|d| d.id).collect();
        ids.sort_unstable();
        let mut points = Row::new();
        for id in ids {
            let pts = shared.points.iter().find(|p| p.0 == id).map_or(0, |p| p.1);
            points.insert(id.to_string(), json!(pts));
        }
        Ok(to_json(&json!({ "build": {
            "slots": slots, "points": points, "cap": shared.cap,
            "level": shared.level, "alignment": share_code::ALIGNMENTS[shared.alignment],
        } })))
    }

    /// What can go in a slot (0..14, ClientReader.SlotNames), with what each choice gives: every item of that kind (its
    /// basic features, its characteristics, its appearance bonus and the sets it belongs to), and the crystals that
    /// kind of piece takes as tarot and as soul fusion (with the fusion sets they belong to).
    pub fn catalog(&self, slot: i32, args: Option<&Value>) -> Result<String> {
        let mut st = lock(&self.state);
        let b = self.load(&mut st, args)?;
        if !(0..=14).contains(&slot) {
            return Err(user_error("No such slot."));
        }
        if let Some(done) = lock(&b.catalog).get(&slot) {
            return Ok(done.clone());
        }
        let data = self.data();
        let equip_type = client_reader::equip_type_of_slot(slot);
        let fx = ItemEffects::new(Arc::clone(data), &b.alignment, b.level, self.ranks(&b, &b.points)?.ranks);

        let mut items: Vec<Row> = Vec::new();
        for (&id, node) in data.items()? {
            if json::str(node.at("equipType")).as_deref() != Some(EQUIP_TYPE_NAMES[equip_type as usize]) {
                continue;
            }
            let name = json::str(node.at("name"));
            let Some(name) = name.filter(|n| !n.trim().is_empty()) else { continue };
            let mut o = Row::new();
            o.insert("id".into(), json!(id));
            o.insert("name".into(), json!(name));
            if let Some(sex) = self.sex_of(id)? {
                o.insert("sex".into(), json!(sex));
            }
            if equip_type == 13 {
                if let Some(wt) = data.item_weapon_type(id)? {
                    o.insert("tag".into(), if wt == "CLOSE_RANGE" { json!("Close range") } else if wt == "LONG_RANGE" { json!("Long range") } else { Value::Null });
                }
            }
            let basic: Vec<Effect> =
                data.item_correct(id)?.unwrap_or_default().iter().map(|c| fx.correct(c.id.as_deref().unwrap_or(""), c.r#type, c.value, "always", "item", &name)).collect();
            o.insert("basic".into(), rows_value(self.lines(&basic)?));
            let mut special: Vec<Effect> = Vec::new();
            for t in data.sitem_tokusei(id)? {
                special.extend(fx.tokusei(t, "characteristic", &name)?);
            }
            o.insert("special".into(), rows_value(self.lines(&special)?));
            let mut look: Vec<Row> = Vec::new();
            let mut sets: Vec<Row> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            for &si in b.sets_of.get(&id).map(Vec::as_slice).unwrap_or(&[]) {
                let s = &b.set_table[si];
                let pieces: Vec<i32> = s.pieces.iter().copied().filter(|&p| p != 0).collect();
                let mut fxs: Vec<Effect> = Vec::new();
                for &t in &s.tokusei {
                    fxs.extend(fx.tokusei(t, if pieces.len() == 1 { "appearance" } else { "set" }, &name)?);
                }
                let lines = self.lines(&fxs)?;
                if pieces.len() == 1 {
                    look.extend(lines);
                    continue;
                }
                let mut with: Vec<String> = Vec::new();
                for &p in pieces.iter().filter(|&&p| p != id) {
                    with.push(self.item_name(p)?);
                }
                let lines_value = rows_value(lines);
                if seen.insert(format!("{}#{}", with.join("|"), to_json(&lines_value))) {
                    sets.push(obj(json!({ "with": with, "lines": lines_value })));
                }
            }
            o.insert("look".into(), rows_value(look));
            o.insert("sets".into(), rows_value(sets));
            items.push(o);
        }
        items.sort_by(|a, b| ic_cmp(a.get("name").and_then(Value::as_str).unwrap_or(""), b.get("name").and_then(Value::as_str).unwrap_or("")));

        let crystals = |tarot: bool| -> Result<Vec<Row>> {
            let mut res: Vec<Row> = Vec::new();
            for (key, side) in data.enchant_sides()? {
                if key.1 != tarot || side.mask & (1i32.wrapping_shl(equip_type as u32)) == 0 {
                    continue;
                }
                let kid = key.0;
                let mut csets: Vec<Row> = Vec::new();
                for es in data.enchant_sets()?.iter().filter(|es| es.effects.contains(&kid)) {
                    let mut with: Vec<String> = Vec::new();
                    for &x in es.effects.iter().filter(|&&x| x != kid) {
                        with.push(self.crystal_name(x, data.enchant_sides()?.contains_key(&(x, true)))?);
                    }
                    let mut fxs: Vec<Effect> = Vec::new();
                    let tokusei = es.tokusei.iter().copied().chain(es.conditions.iter().filter(|cd| cd.0 == 0).flat_map(|cd| cd.1.iter().copied()));
                    for t in tokusei {
                        fxs.extend(fx.tokusei(t, "fusion set", "")?);
                    }
                    csets.push(obj(json!({ "with": with, "lines": rows_value(self.lines(&fxs)?) })));
                }
                res.push(obj(json!({
                    "id": kid, "name": self.crystal_name(kid, tarot)?, "lines": rows_value(self.lines(&fx.enchant(kid, tarot, "")?)?), "sets": rows_value(csets),
                })));
            }
            res.sort_by(|a, b| ic_cmp(a.get("name").and_then(Value::as_str).unwrap_or(""), b.get("name").and_then(Value::as_str).unwrap_or("")));
            Ok(res)
        };

        let text = to_json(&Value::Object(obj(json!({ "slot": slot, "items": rows_value(items), "tarot": rows_value(crystals(true)?), "soul": rows_value(crystals(false)?) }))));
        lock(&b.catalog).insert(slot, text.clone());
        Ok(text)
    }

    /// For tests (`imagine-reader simulate`): the unchanged build against the saved page data. Lines that differ; empty
    /// when the simulator gives back dash\bonuses.json row for row, the 12 readings, and the Digitalize page's HP / MP.
    pub fn self_check(&self) -> Result<Vec<String>> {
        let mut st = lock(&self.state);
        let b = self.load(&mut st, None)?;
        let res = Value::Object(self.result(&b, &Build { items: b.items.clone(), points: b.points.clone(), demon: None })?);
        let mut diffs: Vec<String> = Vec::new();
        for i in 0..12 {
            let got = json::int(res.at("stats").at(STAT_KEYS[i]));
            if got != b.totals[i] {
                diffs.push(format!("{}: read {}, simulated {}", STAT_KEYS[i], b.totals[i], got));
            }
        }
        if let Some(cal) = &b.cal {
            if json::int(res.at("stats").at("max_hp")) != cal.max_hp || json::int(res.at("stats").at("max_mp")) != cal.max_mp {
                let t = |k: &str| json::node_to_string(res.at("stats").at(k)).unwrap_or_default();
                diffs.push(format!("Max HP/MP: read {}/{}, simulated {}/{}", cal.max_hp, cal.max_mp, t("max_hp"), t("max_mp")));
            }
        }
        let saved = self.store().dash_dir().join("bonuses.json");
        if saved.exists() {
            let key = |r: &Value| format!("{}|{}", json::str(r.at("row")).unwrap_or_default(), json::str(r.at("unit")).unwrap_or_default());
            let doc = json::load(&saved)?;
            let old: IndexMap<String, &Value> = json::arr(Some(&doc)).map(|r| (key(r), r)).collect();
            let now: IndexMap<String, &Value> = json::arr(res.at("bonuses")).map(|r| (key(r), r)).collect();
            let mut keys: Vec<&String> = old.keys().collect();
            for k in now.keys() {
                if !old.contains_key(k) {
                    keys.push(k);
                }
            }
            for k in keys {
                let Some(o) = old.get(k) else {
                    diffs.push(format!("bonus {k}: only in the simulator"));
                    continue;
                };
                let Some(n) = now.get(k) else {
                    diffs.push(format!("bonus {k}: only in bonuses.json"));
                    continue;
                };
                for f in ["gear", "gear_digi", "expertise", "demon", "total", "total_digi"] {
                    if json::num(o.at(f)) != json::num(n.at(f)) {
                        let show = |v: &&Value| v.at(f).map_or("null".to_string(), |x| json::serialize(x, Escape::Default, false));
                        diffs.push(format!("bonus {k} {f}: page {}, simulator {}", show(o), show(n)));
                    }
                }
            }
        }
        let digi = self.store().dash_dir().join("digitalize.json");
        if digi.exists() {
            let d = json::load(&digi)?;
            if let (Some(hm @ Value::Object(_)), Some(dg @ Value::Object(_))) = (d.at("hp_mp"), res.at("digi")) {
                if json::int(hm.at("hp")) != json::int(dg.at("max_hp")) || json::int(hm.at("mp")) != json::int(dg.at("max_mp")) {
                    let t = |v: &Value, k: &str| json::node_to_string(v.at(k)).unwrap_or_default();
                    diffs.push(format!("Digitalized Max HP/MP: page {}/{}, simulator {}/{}", t(hm, "hp"), t(hm, "mp"), t(dg, "max_hp"), t(dg, "max_mp")));
                }
            }
        }
        Ok(diffs)
    }
}

// ---------- stats ----------

/// The counted stat lines of a build, per stat: flat, percent and fusion bonus (which the server adds before the
/// percent). `digi` adds what only works while digitalized (gear lines and the demon's effects; the demon's own stats
/// are not here, they join the base).
fn sum_stats(parts: &[Part], digi: bool) -> Sums {
    let mut s = Sums::default();
    for p in parts {
        if p.target != "self" || p.group != "Stats" || p.demon_stat {
            continue;
        }
        // the summoned partner is away while digitalized
        let on = if p.kind == "summoned" { !digi && p.cond == "always" } else if p.kind == "demon" { digi && p.cond == "digitalized" } else { p.cond == "always" || (digi && p.kind == "gear" && p.cond == "digitalized") };
        if !on {
            continue;
        }
        let v = p.value.trunc(); // the server works in whole numbers
        let mut add = |i: usize| {
            if p.pct {
                s.pct[i] += v;
            } else if p.fuse {
                s.fuse[i] += v;
            } else {
                s.flat[i] += v;
            }
        };
        if p.bonus == "All stats" {
            for i in 0..6 {
                add(i);
            }
            continue;
        }
        if let Some(k) = STAT_NAMES.iter().position(|n| *n == p.bonus) {
            add(k);
        }
    }
    s
}

/// AdjustStats' percent step: the value plus that percent of it, cut to a whole number; -100% or less gives 0.
fn with_pct(value: i32, pct: f64) -> i32 {
    if pct == 0.0 {
        value
    } else if pct <= -100.0 {
        0
    } else {
        value.wrapping_add((value as f64 * (pct * 0.01)) as i32)
    }
}

/// The 12 stats of a build by the server's order. `demon`: what the digitalize demon adds to each (None = not
/// digitalized). `resid`: what each reading holds beyond the model. `core_is`: use these STR..LUCK for the dependent
/// stats instead of the model's own (to measure the dependent stats' residual against the read core stats).
fn model(b: &Base, s: &Sums, demon: Option<&[i32; 12]>, resid: &[i32; 12], core_is: Option<&[i32; 12]>) -> [i32; 12] {
    let mut v = [0i32; 12];
    for i in 0..6 {
        v[i] = with_pct(b.base_core[i].wrapping_add(demon.map_or(0, |d| d[i])), s.pct[i]).wrapping_add(s.flat[i] as i32).wrapping_add(resid[i]);
    }
    for j in 6..12 {
        let (src, k) = DEP[j - 6];
        let core = core_is.map_or(v[src], |c| c[src]);
        let own = ((core as f64 * k + b.stat_level as f64 * 0.1).floor() as i32).wrapping_add(s.fuse[j] as i32).wrapping_add(demon.map_or(0, |d| d[j]));
        v[j] = with_pct(own, s.pct[j]).wrapping_add(s.flat[j] as i32).wrapping_add(resid[j]);
    }
    v
}

// ---------- saved builds ----------
// The builds a player keeps: ONE list for all characters (sim-builds.json in the data folder), since a build names
// its own base: [{ id, name, created_at, saved_at, base (a character's name; "" = no base, then also level and
// alignment), slots (15, as the page's build), points, cap }]. A build is whole, not "what differs from the gear
// worn": it is a plan, and stays the same plan when the worn gear changes. The page shows it against its base as
// read at that time. A build saved before bases existed has no "base": the page takes the character it is showing.

static BUILDS_LOCK: Mutex<()> = Mutex::new(());
/// Expertise ids are small numbers (the game has under a hundred); a saved build keeps no other key.
const MAX_SAVED_EXPERTISE_ID: i32 = 999;

/// `owner`: whose builds the ones without a "base" are: saved before builds had bases, when this file was the list of
/// the character whose folder is the data folder itself. They get that character as their base.
fn load_builds(path: &std::path::Path, owner: Option<&str>) -> Result<Vec<Value>> {
    let mut list: Vec<Value> = if path.exists() {
        // catch (JsonException) { list = new JsonArray(); }
        match serde_json::from_str::<Value>(json::read_all_text(path)?.trim_start_matches('\u{feff}')) {
            Ok(Value::Array(a)) => a,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    if let Some(owner) = owner {
        for b in list.iter_mut() {
            if let Value::Object(m) = b {
                if m.get("base").is_none_or(Value::is_null) {
                    m.insert("base".into(), json!(owner));
                }
            }
        }
    }
    Ok(list)
}

/// The saved builds, in the order they were made.
pub fn builds(path: &std::path::Path, owner: Option<&str>) -> Result<String> {
    let _guard = lock(&BUILDS_LOCK);
    Ok(to_json(&Value::Array(load_builds(path, owner)?)))
}

/// `DateTime.UtcNow.Ticks` (100 ns since 0001-01-01).
fn utc_ticks() -> u128 {
    let since_unix = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() / 100);
    since_unix + 621_355_968_000_000_000u128
}

/// Saves a build: { id (none, or one that is not there: a new build), name, base, level, alignment, slots, points,
/// cap }. Only what a build is made of is kept, as numbers. Returns { id, builds }.
pub fn save_build(path: &std::path::Path, b: Option<&Value>, owner: Option<&str>) -> Result<String> {
    let _guard = lock(&BUILDS_LOCK);
    let name = RE_SPACES.replace_all(&json::str(b.at("name")).unwrap_or_default(), " ").trim().to_string();
    if name.is_empty() {
        return Err(user_error("Give the build a name."));
    }
    let units: Vec<u16> = name.encode_utf16().collect();
    let name = if units.len() > 60 { String::from_utf16_lossy(&units[..60]) } else { name };
    let b = b.ok_or_else(|| anyhow::anyhow!("Object reference not set to an instance of an object."))?;
    let mut slots: Vec<Value> = Vec::new();
    let in_slots = match b.at("slots") {
        Some(Value::Array(a)) => Some(a),
        _ => None,
    };
    // Every number is held to what a build can have at all (the same limits as a shared code, share_code.rs), so
    // nothing the page sends, an imported build included, puts a number in the file that the game has no use for.
    let held = |v: i32| v.clamp(0, share_code::MAX_NUMBER);
    let held_list = |n: Option<&Value>, count: usize| -> Vec<i32> { ints(n, count).into_iter().map(held).collect() };
    for k in 0..15 {
        let n = in_slots.and_then(|a| a.get(k)).filter(|v| !v.is_null());
        let id = json::int(n.at("id"));
        slots.push(match n.filter(|_| id > 0 && id <= share_code::MAX_NUMBER) {
            None => Value::Null,
            Some(n) => json!({
                "id": id, "basic": held(json::int_or(n.at("basic"), id)), "special": held(json::int_or(n.at("special"), id)),
                "tarot": held(json::int(n.at("tarot"))), "soul": held(json::int(n.at("soul"))),
                "fuse": held_list(n.at("fuse"), 3), "mods": held_list(n.at("mods"), 5),
            }),
        });
    }
    let mut points = Row::new();
    if let Some(Value::Object(pts)) = b.at("points") {
        for (key, v) in pts {
            if let Some(eid) = crate::fmt::parse_i32(key).filter(|id| (0..=MAX_SAVED_EXPERTISE_ID).contains(id)) {
                points.insert(eid.to_string(), json!(json::int(Some(v)).clamp(0, share_code::MAX_POINTS)));
            }
        }
    }
    let mut list = load_builds(path, owner)?;
    let want = json::str(b.at("id"));
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let found = list.iter().position(|x| x.is_object() && want.is_some() && json::str(x.at("id")) == want);
    let idx = match found {
        Some(i) => i,
        None => {
            list.push(json!({ "id": format!("b{:x}", utc_ticks()), "created_at": now }));
            list.len() - 1
        }
    };
    let Value::Object(entry) = &mut list[idx] else { unreachable!("the entry is an object") };
    entry.insert("name".into(), json!(name));
    entry.insert("saved_at".into(), json!(now));
    entry.insert("slots".into(), Value::Array(slots));
    entry.insert("points".into(), Value::Object(points));
    entry.insert("cap".into(), match json::num(b.at("cap")) {
        Some(cap) if cap >= 0.0 => json!((cap as i64).min(i64::from(share_code::MAX_CAP))),
        _ => Value::Null,
    });
    let who = json::str(b.at("base")).unwrap_or_default().trim().to_string();
    entry.insert("base".into(), json!(who));
    entry.shift_remove("demon");
    if b.at("demon").is_some() && !who.is_empty() {
        entry.insert("demon".into(), json!(json::str(b.at("demon")).unwrap_or_default())); // "" = not digitalized; none saved = the page decides (older builds)
    }
    entry.shift_remove("level");
    entry.shift_remove("alignment");
    if who.is_empty() {
        entry.insert("level".into(), json!(json::int_or(b.at("level"), 99).clamp(1, 99)));
        let align = json::str(b.at("alignment")).unwrap_or_default().to_uppercase();
        entry.insert("alignment".into(), json!(if align == "LAW" || align == "CHAOS" { align } else { "NEUTRAL".to_string() }));
    }
    let id = json::str(list[idx].at("id"));
    let list = Value::Array(list);
    json::save(path, &list)?;
    Ok(to_json(&json!({ "id": id, "builds": list })))
}

/// Removes a saved build. Returns { builds }.
pub fn delete_build(path: &std::path::Path, id: Option<&str>, owner: Option<&str>) -> Result<String> {
    let _guard = lock(&BUILDS_LOCK);
    let mut list = load_builds(path, owner)?;
    let id = id.map(str::to_string);
    list.retain(|x| !(x.is_object() && json::str(x.at("id")) == id));
    let list = Value::Array(list);
    json::save(path, &list)?;
    Ok(to_json(&json!({ "builds": list })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_without_records_are_the_items_themselves() {
        use crate::data::store::SlotTip;
        let tip = |index: i32, id: i32, ench: Option<(i32, i32)>| SlotTip { index, id, text: String::new(), tarot: ench.map(|e| e.0), soul: ench.map(|e| e.1), detail: None };
        let mut recs: Vec<SlotTip> = (0..15).map(|k| tip(k, 0, None)).collect();
        recs[0] = tip(0, 2625, Some((64, 3105)));
        recs[2] = tip(2, 22160, None);
        let p = pieces_without_records(&recs);
        assert_eq!(p.len(), 15);
        let w = p[0].as_ref().unwrap();
        assert_eq!((w.id, w.tarot, w.soul, w.basic, w.special), (2625, 64, 3105, 0, 0));
        assert_eq!((w.mods.clone(), w.fuse.clone()), (vec![0; 5], vec![0; 3]));
        assert_eq!(basic_of(w), 2625); // its own stats
        assert_eq!(p[2].as_ref().map(|w| (w.tarot, w.soul)), Some((0, 0))); // enchants not read: none
        assert!(p[1].is_none());
    }

    #[test]
    fn the_simulator_is_shareable_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Simulator>();
        // the app keeps pipelines (with their simulators) and calls them from several threads
        assert_send_sync::<super::super::pipeline::Pipeline>();
        assert_send_sync::<std::sync::Arc<super::super::pipeline::Pipeline>>();
    }

    #[test]
    fn percent_step_cuts_to_a_whole_number() {
        assert_eq!(with_pct(100, 0.0), 100);
        assert_eq!(with_pct(100, 10.0), 110);
        assert_eq!(with_pct(99, 10.0), 108); // 9.9 cut to 9
        assert_eq!(with_pct(100, -100.0), 0);
        assert_eq!(with_pct(100, -150.0), 0);
        assert_eq!(with_pct(-50, 10.0), -55);
    }

    #[test]
    fn ordinal_ignore_case_order() {
        use std::cmp::Ordering::*;
        assert_eq!(ic_cmp("abc", "ABD"), Less);
        assert_eq!(ic_cmp("Abc", "aBC"), Equal);
        assert_eq!(ic_cmp("a", "B"), Less);
        // upper-cased ordinal: "_" (0x5F) sorts after letters' upper case, before lower case's
        assert_eq!(ic_cmp("_x", "Zed"), Greater);
    }

    #[test]
    fn saved_builds_round_trip() {
        let dir = std::env::temp_dir().join(format!("imagine-sim-test-{}", std::process::id()));
        let path = dir.join("sim-builds.json");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(builds(&path, None).unwrap(), "[]");
        let e = save_build(&path, Some(&json!({ "name": "  " })), None).unwrap_err();
        assert_eq!(e.downcast_ref::<crate::UserError>().unwrap().0, "Give the build a name.");
        let spec = json!({ "name": " My   build ", "base": "Rubito", "demon": "1:A:2", "cap": 5.9, "points": { "3": 20000, "x": 1, "4": -5 },
            "slots": [{ "id": 5, "tarot": -1, "fuse": [1, 2] }, null, { "id": 0 }] });
        let out: Value = serde_json::from_str(&save_build(&path, Some(&spec), None).unwrap()).unwrap();
        let list = out.at("builds").unwrap();
        let first = &list[0];
        assert_eq!(first.at("name").unwrap(), "My build");
        assert_eq!(first.at("base").unwrap(), "Rubito");
        assert_eq!(first.at("demon").unwrap(), "1:A:2");
        assert_eq!(first.at("cap").unwrap(), 5);
        assert_eq!(first.at("points").unwrap(), &json!({ "3": 20000, "4": 0 }));
        assert_eq!(first.at("slots").idx(0).unwrap(), &json!({ "id": 5, "basic": 5, "special": 5, "tarot": 0, "soul": 0, "fuse": [1, 2, 0], "mods": [0, 0, 0, 0, 0] }));
        assert_eq!(first.at("slots").unwrap().as_array().unwrap().len(), 15);
        assert!(first.at("level").is_none());
        let id = out.at("id").unwrap().as_str().unwrap().to_string();
        // saving it again by id changes it in place; with no base it takes level and alignment, and drops the demon
        let again = json!({ "id": id, "name": "Plan", "level": 150, "alignment": "law", "demon": "x" });
        let out2: Value = serde_json::from_str(&save_build(&path, Some(&again), None).unwrap()).unwrap();
        let list2 = out2.at("builds").unwrap().as_array().unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].at("level").unwrap(), 99);
        assert_eq!(list2[0].at("alignment").unwrap(), "LAW");
        assert!(list2[0].at("demon").is_none());
        let keys: Vec<&String> = list2[0].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["id", "created_at", "name", "saved_at", "slots", "points", "cap", "base", "level", "alignment"]);
        // delete
        let gone: Value = serde_json::from_str(&delete_build(&path, Some(&id), None).unwrap()).unwrap();
        assert_eq!(gone.at("builds").unwrap().as_array().unwrap().len(), 0);
        // builds saved before bases existed get the owner
        std::fs::write(&path, r#"[{"id":"old","name":"x"}]"#).unwrap();
        assert_eq!(builds(&path, Some("Rubito")).unwrap(), r#"[{"id":"old","name":"x","base":"Rubito"}]"#);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
