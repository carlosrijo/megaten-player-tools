//! Dumps what `calc::{item_effects, gear_effects, digitalize, character_extras}` and `context` produce, as text lines. A throwaway
//! C# harness (compiled from the real sources, `Main.cs` in the porting scratch folder) printed the same lines for the same
//! inputs; the two files must be identical. The inputs are the fixture copy of the user's data plus generated variants.
//!
//! Usage (run with the repo root as the current folder): w2dump <items|gear|digi|extras> <fixture folder (data\ and smt-stats\)> <out file>
#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use newmoon::Result;
use newmoon::build::csv::CsvTable;
use newmoon::calc::character_extras as extras;
use newmoon::calc::digitalize::{self, Digitalize, DigitalizeResult};
use newmoon::calc::effect::Effect;
use newmoon::calc::gear_effects::{self, GearEffects, GearResult, SetReport};
use newmoon::calc::item_effects::{self, ItemEffects};
use newmoon::context::Context;
use newmoon::data::config;
use newmoon::data::game_data::GameData;
use newmoon::data::json::{self, At};
use newmoon::data::store::Store;
use newmoon::fmt::g;
use newmoon::game::progress_reader::ProgressState;
use newmoon::game::{DemonRecord, WornItem};
use serde_json::Value;

struct H {
    sb: String,
    s: PathBuf,
    data: Arc<GameData>,
}

fn q(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\r', "\\r").replace('\n', "\\n")
}
fn gv(v: Option<f64>) -> String {
    v.map(g).unwrap_or_else(|| "~".into())
}
fn b(x: bool) -> &'static str {
    if x { "1" } else { "0" }
}
fn so(s: &Option<String>) -> String {
    s.as_ref().map(|s| q(s)).unwrap_or_else(|| "~".into())
}
fn ints(a: &[i32]) -> String {
    a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}
pub fn fx(e: &Effect) -> String {
    format!(
        "  fx label={}|value={}|pct={}|cond={}|note={}|target={}|source={}|item={}|tok={}|skill={}|missing={}",
        q(&e.label),
        gv(e.value),
        b(e.pct),
        q(&e.cond),
        q(&e.cond_note),
        q(&e.target),
        q(&e.source),
        q(&e.item),
        e.tokusei,
        e.skill_id,
        b(e.missing_data)
    )
}
fn lcg(s: &mut u32) -> i32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    (*s >> 8) as i32
}

impl H {
    fn p(&mut self, s: &str) {
        self.sb.push_str(s);
        self.sb.push('\n');
    }
    fn fxs(&mut self, l: &[Effect]) {
        let _ = writeln!(self.sb, " effects n={}", l.len());
        for e in l {
            let line = fx(e);
            self.p(&line);
        }
    }
    fn try_(&mut self, head: &str, f: impl FnOnce(&mut H) -> Result<()>) {
        self.p(&format!(">> {head}"));
        if let Err(e) = f(self) {
            let _ = e; self.p("!! error");
        }
    }
    fn dirs(&self) -> [PathBuf; 2] {
        [self.s.join("data"), self.s.join("data").join("characters").join("Rokujuu")]
    }
    fn dir_tail(&self, d: &Path) -> String {
        d.to_string_lossy()[self.s.to_string_lossy().len()..].to_string()
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args[1].clone();
    let mut h = H { sb: String::new(), s: PathBuf::from(&args[2]), data: Arc::new(GameData::new()) };
    match mode.as_str() {
        "items" => items(&mut h)?,
        "gear" => gear(&mut h)?,
        "digi" => digi(&mut h)?,
        "extras" => extras_mode(&mut h)?,
        _ => {}
    }
    std::fs::write(&args[3], &h.sb)?;
    println!("{mode}: {} chars", h.sb.chars().count());
    Ok(())
}

fn pack(w: &WornItem) -> String {
    Store::pack_item(Some(w))
}

// ---------------------------------------------------------------- ItemEffects
fn items(h: &mut H) -> Result<()> {
    let data = Arc::clone(&h.data);
    let mut pool: Vec<(i32, WornItem)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut add = |pool: &mut Vec<(i32, WornItem)>, k: i32, w: &WornItem| {
        if seen.insert(format!("{k}|{}", pack(w))) {
            pool.push((k, w.clone()));
        }
    };
    let mut csvs: Vec<String> = std::fs::read_dir(h.s.join("smt-stats"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().into_owned())
        .filter(|p| p.ends_with(".csv"))
        .collect();
    csvs.sort();
    for csv in &csvs {
        let t = CsvTable::load(csv)?;
        for row in &t.rows {
            let cell = t.get(row, "Gear records");
            if let Some(d) = Store::parse_items(Some(cell.as_str())) {
                for (k, w) in d.iter().enumerate() {
                    if let Some(w) = w {
                        add(&mut pool, k as i32, w);
                    }
                }
            }
        }
    }
    for dir in h.dirs() {
        for r in Store::new(&dir).load_tooltips()? {
            if let Some(d) = &r.detail {
                add(&mut pool, r.index, d);
            }
        }
    }
    h.p(&format!("# pool {}", pool.len()));
    let fixture_ranks = Store::new(&h.dirs()[0]).expertise_ranks()?;
    let rank_sets: Vec<(&str, IndexMap<i32, i32>)> = vec![("fixture", fixture_ranks.clone()), ("none", IndexMap::new()), ("high", (0..=160).map(|i| (i, 100)).collect())];
    let aligns = ["LAW", "NEUTRAL", "CHAOS"];
    let name_of = |w: &WornItem| data.item_name(w.id).ok().flatten().unwrap_or_else(|| "unnamed".into());
    let mk = |al: &str, lv: i32, ranks: &IndexMap<i32, i32>| ItemEffects::new(Arc::clone(&data), al, lv, ranks.clone());

    for (slot, w) in &pool {
        for al in aligns {
            for (rn, ranks) in rank_sets.iter().take(2) {
                for lv in [95, 30] {
                    h.try_(&format!("item slot={slot} rec={} align={al} ranks={rn} level={lv}", pack(w)), |h| {
                        let r = mk(al, lv, ranks).compute(Some(w), *slot, &name_of(w), None)?;
                        h.fxs(&r);
                        Ok(())
                    });
                }
            }
        }
    }

    // variants of each distinct item id
    let mut firsts: Vec<(i32, WornItem)> = Vec::new();
    let mut ids = HashSet::new();
    for p in &pool {
        if ids.insert(p.1.id) {
            firsts.push(p.clone());
        }
    }
    for i in 0..firsts.len() {
        let (slot, w) = &firsts[i];
        let other = firsts[(i + 1) % firsts.len()].1.id;
        let prev = firsts[(i + firsts.len() - 1) % firsts.len()].1.id;
        let with = |f: &dyn Fn(&mut WornItem)| {
            let mut v = w.clone();
            f(&mut v);
            v
        };
        let variants: Vec<(&str, i32, WornItem)> = vec![
            ("fuseA", *slot, with(&|v| v.fuse = vec![7, 25, 50])),
            ("fuseB", *slot, with(&|v| v.fuse = vec![1, 22, 60])),
            ("broken", *slot, with(&|v| {
                v.durability = 0;
                v.max_durability = 10;
            })),
            ("bullets", 1, with(&|v| {
                v.durability = 0;
                v.max_durability = 10;
            })),
            ("basic", *slot, with(&|v| {
                v.basic = other;
                v.special = prev;
            })),
            ("modsA", *slot, with(&|v| v.mods = vec![1, 2, 3, 4, 5])),
            ("modsB", *slot, with(&|v| v.mods = vec![255, 10, 0, 20, 30])),
        ];
        for (tag, sl, v) in &variants {
            for al in aligns {
                h.try_(&format!("variant {tag} slot={sl} rec={} align={al}", pack(v)), |h| {
                    let r = mk(al, 95, &fixture_ranks).compute(Some(v), *sl, &name_of(v), None)?;
                    h.fxs(&r);
                    Ok(())
                });
            }
        }
    }
    let rec = |id, tarot, soul, basic, special, mods: [i32; 5]| WornItem { id, tarot, soul, basic, special, mods: mods.to_vec(), fuse: vec![0, 0, 0], durability: -1, max_durability: 0 };
    let z5 = [0; 5];
    let p0 = pool[0].1.id;
    let unknowns = vec![
        rec(999999, 0, 0, 0, 0, z5),
        rec(p0, 0, 0, 888888, 0, z5),
        rec(p0, 0, 0, 0, 777777, z5),
        rec(p0, 99999, 88888, 0, 0, [250, 251, 252, 253, 254]),
        rec(0, 0, 0, 0, 0, z5),
        rec(-5, 0, 0, 0, 0, z5),
    ];
    for w in &unknowns {
        for sl in [3, 13] {
            h.try_(&format!("unknown slot={sl} rec={}", pack(w)), |h| {
                let r = mk("NEUTRAL", 95, &fixture_ranks).compute(Some(w), sl, "Unknown", None)?;
                h.fxs(&r);
                Ok(())
            });
        }
    }
    h.try_("null record", |h| {
        let r = mk("NEUTRAL", 95, &fixture_ranks).compute(None, 3, "Null", None)?;
        h.fxs(&r);
        Ok(())
    });

    // tooltip replacement
    let tip = vec![
        Effect { label: "tip item".into(), value: Some(5.0), source: "item".into(), item: "tip".into(), ..Default::default() },
        Effect { label: "tip tarot".into(), value: Some(6.0), source: "tarot".into(), item: "tip".into(), cond_note: "n".into(), ..Default::default() },
        Effect { label: "tip mod".into(), source: "mod".into(), ..Default::default() },
    ];
    for w in unknowns.iter().take(4) {
        h.try_(&format!("tooltip {}", pack(w)), |h| {
            let r = mk("NEUTRAL", 95, &fixture_ranks).compute(Some(w), 3, "T", Some(&tip))?;
            h.fxs(&r);
            Ok(())
        });
    }
    h.try_("tooltip real", |h| {
        let r = mk("NEUTRAL", 95, &fixture_ranks).compute(Some(&pool[0].1), pool[0].0, "T", Some(&tip))?;
        h.fxs(&r);
        Ok(())
    });

    // enchants
    let mut sides: Vec<(i32, bool)> = data.enchant_sides()?.keys().copied().collect();
    sides.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    for (id, tarot) in sides {
        for al in aligns {
            for (rn, ranks) in &rank_sets {
                for lv in [95, 1] {
                    h.try_(&format!("enchant id={id} tarot={} align={al} ranks={rn} level={lv}", b(tarot)), |h| {
                        let r = mk(al, lv, ranks).enchant(id, tarot, "it")?;
                        h.fxs(&r);
                        Ok(())
                    });
                }
            }
        }
    }
    h.try_("enchant missing", |h| {
        let r = mk("NEUTRAL", 95, &fixture_ranks).enchant(31337, true, "it")?;
        h.fxs(&r);
        Ok(())
    });

    // tokusei
    let mut tids: BTreeSet<i32> = [0, 1, 999999999, 20000000, 20000001, 20010018].into_iter().collect();
    for s in data.enchant_sides()?.values() {
        tids.extend(s.tokusei.iter().copied());
        for c in &s.conditions {
            tids.extend(c.3.iter().copied());
        }
    }
    for id in data.items()?.keys() {
        tids.extend(data.sitem_tokusei(*id)?);
    }
    for dir in h.dirs() {
        for s in Store::new(&dir).load_set_table()?.unwrap_or_default() {
            tids.extend(s.tokusei.iter().copied());
        }
    }
    for s in data.enchant_sets()? {
        tids.extend(s.tokusei.iter().copied());
        for c in &s.conditions {
            tids.extend(c.1.iter().copied());
        }
    }
    h.p(&format!("# tokusei ids {}", tids.len()));
    for t in &tids {
        for al in aligns {
            h.try_(&format!("tokusei {t} align={al}"), |h| {
                let r = mk(al, 95, &fixture_ranks).tokusei(*t, "src", "itm")?;
                h.fxs(&r);
                Ok(())
            });
        }
    }
    for t in tids.iter().take(300) {
        h.try_(&format!("tokusei {t} level=7 noranks"), |h| {
            let r = mk("CHAOS", 7, &IndexMap::new()).tokusei(*t, "characteristic", "itm")?;
            h.fxs(&r);
            Ok(())
        });
    }

    // correct table lines
    let mut stat_ids: Vec<String> = Vec::new();
    let labels = config::repo_root().expect("repo root").join("reference").join("stat-labels-game.tsv");
    for l in json::read_all_text(&labels)?.lines() {
        if !l.is_empty() && !l.starts_with('#') {
            stat_ids.push(l.split('\t').next().unwrap_or("").split(':').next().unwrap_or("").to_string());
        }
    }
    stat_ids.extend(["NRA_FIRE", "NRA_PHYS", "NRA_SLASH", "BOOST_FIRE", "RES_ICE", "NOT_A_STAT", "FINAL_CRIT_CHANCE", "RATE_XP", "LNGR", ""].map(String::from));
    let fx0 = mk("NEUTRAL", 95, &IndexMap::new());
    let mut seen_ids = HashSet::new();
    for id in &stat_ids {
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        for ty in [0, 1, 3, 4, 5, 101, 103] {
            for v in [12.5, -3.0, 0.0] {
                h.try_(&format!("correct {id} {ty} {}", g(v)), |h| {
                    let e = fx0.correct(id, ty, v, "always", "src", "itm");
                    h.p(&fx(&e));
                    Ok(())
                });
            }
        }
    }

    // fusion bonus
    let mut per_key: HashMap<String, i32> = HashMap::new();
    let mut chosen = Vec::new();
    let mut all_ids: Vec<i32> = data.items()?.keys().copied().collect();
    all_ids.sort();
    for id in all_ids {
        let key = format!("{}/{}", data.item_equip_type(id)?.unwrap_or_default(), data.item_weapon_type(id)?.unwrap_or_default());
        let n = per_key.entry(key).or_insert(0);
        if *n >= 2 {
            continue;
        }
        *n += 1;
        chosen.push(id);
    }
    let fv = [0, 1, 6, 7, 22, 50, 60];
    for id in &chosen {
        for a in fv {
            for bb in fv {
                for c in fv {
                    h.try_(&format!("fuse id={id} {a},{bb},{c}"), |h| {
                        let w = WornItem { id: *id, tarot: 0, soul: 0, basic: 0, special: 0, mods: vec![0; 5], fuse: vec![a, bb, c], durability: -1, max_durability: 0 };
                        let r = fx0.fuse_bonus(&w)?;
                        h.p(&format!(" {}", r.iter().map(|x| format!("{},{},{}", x.0, x.1, b(x.2))).collect::<Vec<_>>().join(";")));
                        Ok(())
                    });
                }
            }
        }
    }

    for i in -1..=30 {
        let n = item_effects::affinity_name(i);
        h.p(&format!("affinity {i} {} | {}", so(&n), item_effects::affinity_cap_label(i)));
    }
    Ok(())
}

// ---------------------------------------------------------------- GearEffects
fn sets_dump(h: &mut H, r: &GearResult) {
    h.p(&format!(" src set={} item={}", r.set_source, r.item_source));
    h.p(&format!(" tooltips n={}", r.items.len()));
    for i in &r.items {
        h.p(&format!(
            "  tip name={}|base={}|sex={}|tarot={}|soul={}|lines={}|{}",
            q(&i.name),
            q(&i.base_name),
            so(&i.sex),
            so(&i.tarot),
            so(&i.soul),
            i.lines.len(),
            q(&i.lines.join("\n"))
        ));
    }
    h.fxs(&r.effects);
    set_reports(h, &r.sets);
}

fn set_reports(h: &mut H, sets: &[SetReport]) {
    h.p(&format!(" sets n={}", sets.len()));
    for s in sets {
        h.p(&format!(
            "  set id={}|item={}|pieces={}|worn={}|tok={}|active={}|app={}|fusion={}|pids={}",
            s.set_id,
            s.item_id,
            q(&s.pieces),
            s.worn,
            s.tokusei,
            b(s.active),
            b(s.appearance),
            b(s.fusion),
            ints(&s.piece_ids)
        ));
        for e in &s.effects {
            let line = format!("  {}", fx(e));
            h.p(&line);
        }
    }
}

fn gear(h: &mut H) -> Result<()> {
    let data = Arc::clone(&h.data);
    let aligns = ["LAW", "NEUTRAL", "CHAOS"];
    let dirs = h.dirs();
    for (who, dir) in [("Rubito", &dirs[0]), ("Rokujuu", &dirs[1])] {
        let store = Store::new(dir);
        let ctx = Context::new(Store::new(dir), Arc::clone(&data));
        let recs = store.load_tooltips()?;
        let (texts, ids, tarot, soul) = Store::unpack(&recs);
        let details = Store::unpack_details(&recs);
        let level = store.tooltips_level()?;
        let table = store.load_set_table()?;
        h.p(&format!(
            "# {who}: align={} level={level} details={} tarot={} setTable={}",
            store.alignment()?,
            if details.is_some() { "True" } else { "False" },
            if tarot.is_some() { "True" } else { "False" },
            table.as_ref().map(|t| t.len().to_string()).unwrap_or_default()
        ));
        let (t_, s_) = (tarot.as_deref(), soul.as_deref());
        let d_ = details.as_deref();
        let idv = Some(&ids[..]);
        h.try_(&format!("{who} S1 full"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, d_, level, None)?)));
        h.try_(&format!("{who} S2 memory enchants from details"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, None, None, d_, level, None)?)));
        h.try_(&format!("{who} S3 no details"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, None, level, None)?)));
        h.try_(&format!("{who} S4 no details no enchants"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, None, None, None, level, None)?)));
        h.try_(&format!("{who} S5 tips only"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, None, None, None, None, 0, None)?)));
        let tip_ok1 = |slot: i32, src: &str| !(slot % 2 == 0 && src == "tarot") && src != "appearance";
        h.try_(&format!("{who} S6 tipOk"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, d_, level, Some(&tip_ok1))?)));
        let tip_ok2 = |_: i32, _: &str| false;
        h.try_(&format!("{who} S6b tipOk none"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, d_, level, Some(&tip_ok2))?)));
        h.try_(&format!("{who} S7 level 0"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, d_, 0, None)?)));
        for al in aligns {
            let classes = store.expertise_classes()?;
            let ranks = store.expertise_ranks()?;
            let mk = |table: Option<Vec<newmoon::game::SetRecord>>, classes: &IndexMap<String, i32>, ranks: Option<IndexMap<i32, i32>>| GearEffects::new(al, classes, Arc::clone(&data), table, ranks);
            h.try_(&format!("{who} A {al} full"), |h| Ok(sets_dump(h, &mk(table.clone(), &classes, Some(ranks.clone())).compute(&texts, idv, t_, s_, d_, level, None)?)));
            h.try_(&format!("{who} A {al} no table"), |h| Ok(sets_dump(h, &mk(None, &classes, Some(ranks.clone())).compute(&texts, idv, t_, s_, d_, level, None)?)));
            h.try_(&format!("{who} A {al} no table no ranks"), |h| Ok(sets_dump(h, &mk(None, &IndexMap::new(), None).compute(&texts, idv, None, None, None, 0, None)?)));
            h.try_(&format!("{who} A {al} no ranks"), |h| Ok(sets_dump(h, &mk(table.clone(), &IndexMap::new(), None).compute(&texts, idv, t_, s_, None, 50, None)?)));
        }
        if let Some(details) = &details {
            let part: Vec<Option<WornItem>> = details.iter().enumerate().map(|(k, d)| if k % 3 == 0 { None } else { d.clone() }).collect();
            h.try_(&format!("{who} S8 partial details"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&texts, idv, t_, s_, Some(&part), level, None)?)));
            let short_t: Vec<String> = texts.iter().take(7).cloned().collect();
            h.try_(&format!("{who} S9 short tooltips"), |h| Ok(sets_dump(h, &ctx.gear()?.compute(&short_t, idv, t_, s_, d_, level, None)?)));
        }
    }
    // synthetic: every set row, every fusion set
    let store0 = Store::new(&dirs[0]);
    let ctx0 = Context::new(Store::new(&dirs[0]), Arc::clone(&data));
    let all_sets = store0.load_set_table()?.unwrap_or_default();
    h.p(&format!("# set rows {}", all_sets.len()));
    let gear = ctx0.gear()?;
    let no_names: HashMap<i32, String> = HashMap::new();
    let first_nz = |s: &newmoon::game::SetRecord| s.pieces.iter().copied().find(|&p| p != 0).unwrap_or(0);
    let all_pieces: Vec<i32> = all_sets.iter().flat_map(|s| s.pieces.clone()).collect();
    h.try_("setbonuses all pieces", |h| Ok(set_reports(h, &gear.set_bonuses(&all_pieces, &no_names, &no_names)?)));
    let firsts: Vec<i32> = all_sets.iter().take(400).map(first_nz).collect();
    h.try_("setbonuses first pieces", |h| Ok(set_reports(h, &gear.set_bonuses(&firsts, &no_names, &no_names)?)));
    let halves: Vec<i32> = all_sets
        .iter()
        .take(400)
        .flat_map(|s| {
            let nz: Vec<i32> = s.pieces.iter().copied().filter(|&p| p != 0).collect();
            let n = 1.max(nz.len() / 2);
            nz.into_iter().take(n).collect::<Vec<_>>()
        })
        .collect();
    h.try_("setbonuses half pieces", |h| Ok(set_reports(h, &gear.set_bonuses(&halves, &no_names, &no_names)?)));
    let mut named = HashMap::new();
    let mut bases = HashMap::new();
    for s in all_sets.iter().take(50) {
        for &p in &s.pieces {
            named.insert(p, format!("Name {p}♂"));
            bases.insert(p, if p % 2 == 0 { String::new() } else { format!("Base {p}") });
        }
    }
    let named_ids: Vec<i32> = all_sets.iter().take(50).flat_map(|s| s.pieces.clone()).collect();
    h.try_("setbonuses named", |h| Ok(set_reports(h, &gear.set_bonuses(&named_ids, &named, &bases)?)));
    let esets = data.enchant_sets()?;
    let all_e: HashSet<i32> = esets.iter().flat_map(|s| s.effects.clone()).collect();
    h.try_("fusion all", |h| Ok(set_reports(h, &gear.fusion_sets(&all_e)?)));
    let first_e: HashSet<i32> = esets.iter().map(|s| s.effects.first().copied().unwrap_or(0)).collect();
    h.try_("fusion first of each", |h| Ok(set_reports(h, &gear.fusion_sets(&first_e)?)));
    h.try_("fusion none", |h| Ok(set_reports(h, &gear.fusion_sets(&HashSet::new())?)));
    for tid in [1, 2, 3, 999999999] {
        h.try_(&format!("tokusei effects {tid}"), |h| {
            let r = gear.tokusei_effects(tid, "item x", "set")?;
            h.fxs(&r);
            Ok(())
        });
    }
    h.try_("tokusei effects appearance", |h| {
        let r = gear.tokusei_effects(all_sets.first().map_or(1, |s| s.tokusei[0]), "item y", "appearance")?;
        h.fxs(&r);
        Ok(())
    });
    h.p(&format!(
        "# labels {}|{}|{}|{}{}",
        gear_effects::correct_label_of("STR"),
        gear_effects::correct_label_of("NRA_FIRE"),
        gear_effects::correct_label_of("ZZZ"),
        b(gear_effects::is_pct_correct("RATE_XP")),
        b(gear_effects::is_pct_correct("STR"))
    ));
    Ok(())
}

// ---------------------------------------------------------------- Digitalize
fn b64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        if let Some(v) = val(c) {
            acc = (acc << 6) | v as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
    }
    out
}

fn int_arr(v: Option<&Value>) -> Option<Vec<i32>> {
    v?.as_array().map(|a| a.iter().map(|x| json::int(Some(x))).collect())
}

pub fn demon_from_json(d: &Value) -> DemonRecord {
    let ia = |k: &str| int_arr(d.at(k)).unwrap_or_default();
    DemonRecord {
        address: d.at("Address").and_then(Value::as_i64).unwrap_or(0),
        flags: json::int(d.at("Flags")),
        name: json::str(d.at("Name")).unwrap_or_default(),
        r#type: json::int(d.at("Type")),
        level: json::int(d.at("Level")),
        object_id: d.at("ObjectId").and_then(Value::as_u64).unwrap_or(0) as u32,
        hp: json::int(d.at("Hp")),
        max_hp: json::int(d.at("MaxHp")),
        mp: json::int(d.at("Mp")),
        max_mp: json::int(d.at("MaxMp")),
        live_value: ia("LiveValue"),
        live_bonus: ia("LiveBonus"),
        growth_type: json::int(d.at("GrowthType")),
        reunion: ia("Reunion"),
        mitama_type: json::int(d.at("MitamaType")),
        mitama_rank: json::int(d.at("MitamaRank")),
        mitama_reunion: b64(&json::str(d.at("MitamaReunion")).unwrap_or_default()),
        mitama_sets_shown: d.at("MitamaSetsShown").and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_bool().unwrap_or(false)).collect()).unwrap_or_default(),
        skills: ia("Skills"),
        acquired: ia("Acquired"),
        soul_points: json::int(d.at("SoulPoints")),
        force_values: int_arr(d.at("ForceValues")),
        force_stack: int_arr(d.at("ForceStack")),
        equipment: int_arr(d.at("Equipment")),
    }
}

fn digi_result(h: &mut H, r: &DigitalizeResult) {
    h.p(&format!(
        " demon={} type={} level={} devil={}",
        q(&r.demon.name),
        r.demon.r#type,
        r.demon.level,
        r.devil.as_ref().map_or("~".to_string(), |d| format!("{}:{}", d.id, d.name))
    ));
    h.p(&format!(" growth={} baseRate={} rate={} maxRate={} hp={} mp={}", r.growth, r.base_rate, r.rate, r.max_rate, r.hp(), r.mp()));
    h.p(&format!(" base={}", if r.base.is_empty() { "~".to_string() } else { ints(&r.base) }));
    h.p(&format!(" stats={}", r.stats.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",")));
    h.p(&format!(
        " active={} sets={} assists={}",
        ints(&r.active_skills),
        ints(&r.sets.iter().map(|s| s.id).collect::<Vec<_>>()),
        r.assists.as_ref().map_or("~".to_string(), |a| a.iter().cloned().collect::<Vec<_>>().join(","))
    ));
    h.p(&format!(" extend={} reduce={} pre={}/{} dep={}/{}", r.extend_time, r.reduce_wait, r.pre_hp, r.pre_mp, r.dependent_hp, r.dependent_mp));
    for n in &r.notes {
        h.p(&format!(" note {}", q(n)));
    }
    h.fxs(&r.effects);
}

fn digi(h: &mut H) -> Result<()> {
    let data = Arc::clone(&h.data);
    let t = data.demons()?;
    h.p(&format!("# tables loaded={} devils={} assists={}", if t.loaded() { "True" } else { "False" }, t.devils.len(), t.assists.len()));
    let aligns = ["LAW", "NEUTRAL", "CHAOS"];
    let dirs = h.dirs();
    for (who, dir) in [("Rubito", &dirs[0]), ("Rokujuu", &dirs[1])] {
        let store = Store::new(dir);
        let ctx = Context::new(Store::new(dir), Arc::clone(&data));
        let mut demons: Vec<DemonRecord> = Vec::new();
        let mut seen_keys: HashSet<String> = HashSet::new();
        let mut add_demon = |d: DemonRecord| {
            if seen_keys.insert(format!("{}|{}|{}|{}", d.r#type, d.name, d.level, d.object_id)) {
                demons.push(d);
            }
        };
        if store.comp_demons_path().is_file() {
            let root = json::load(store.comp_demons_path())?;
            for s in json::arr(Some(&root)) {
                if let Some(d) = s.at("Demon") {
                    add_demon(demon_from_json(d));
                }
            }
        }
        let save = if store.digitalize_path().is_file() { Some(json::load(store.digitalize_path())?) } else { None };
        if let Some(d) = save.as_ref().and_then(|s| s.at("Demon")) {
            add_demon(demon_from_json(d));
        }
        let prog = if store.digitalize_progress_path().is_file() { Some(json::load(store.digitalize_progress_path())?) } else { None };
        let learned = int_arr(prog.as_ref().and_then(|p| p.at("Assists"))).or_else(|| int_arr(save.as_ref().and_then(|p| p.at("Assists"))));
        let learned_boost = prog
            .as_ref()
            .and_then(|p| p.at("MitamaBoost"))
            .and_then(Value::as_bool)
            .or_else(|| save.as_ref().and_then(|s| s.at("MitamaBoost")).and_then(Value::as_bool))
            .unwrap_or(false);
        let level = store.tooltips_level()?;
        h.p(&format!(
            "# {who}: demons={} learned={} boost={} level={level}",
            demons.len(),
            learned.as_ref().map(|l| l.len().to_string()).unwrap_or_default(),
            if learned_boost { "True" } else { "False" }
        ));
        let ranks = store.expertise_ranks()?;

        for d in &demons {
            let devil = t.devils.get(&d.r#type);
            let race: Vec<newmoon::data::demon_tables::Assist> = devil.map(|dv| t.assists_of_race(dv.race_id).into_iter().cloned().collect()).unwrap_or_default();
            let race_ids: Vec<i32> = race.iter().map(|a| a.id).collect();
            let mut scenarios: Vec<(String, Option<Vec<i32>>, bool)> = vec![
                ("unknown".into(), None, false),
                ("learned".into(), learned.clone(), learned_boost),
                ("learned+boost".into(), learned.clone(), true),
                ("learned-boost".into(), learned.clone(), false),
                ("none".into(), Some(vec![]), false),
                ("all".into(), Some(race_ids.clone()), false),
                ("all+boost".into(), Some(race_ids.clone()), true),
                ("dups".into(), Some(race_ids.iter().take(3).chain(race_ids.iter().take(3)).copied().chain([999999]).collect()), false),
            ];
            let mut types: IndexSet<String> = IndexSet::new();
            for a in &race {
                types.insert(a.r#type.clone());
            }
            for ty in &types {
                scenarios.push((format!("type {ty}"), Some(race.iter().filter(|a| &a.r#type == ty).map(|a| a.id).collect()), false));
            }
            for (name, assists, boost) in &scenarios {
                h.try_(&format!("{who} demon={}/{}/{} scenario={name} boost={} level={level}", d.name, d.r#type, d.level, b(*boost)), |h| {
                    let r = ctx.digitalize(level)?.compute(d, assists.as_deref(), *boost)?;
                    digi_result(h, &r);
                    Ok(())
                });
            }
            h.try_(&format!("{who} demon={} level=1", d.name), |h| {
                let r = ctx.digitalize(1)?.compute(d, learned.as_deref(), true)?;
                digi_result(h, &r);
                Ok(())
            });
            for al in aligns {
                h.try_(&format!("{who} demon={} align={al} all", d.name), |h| {
                    let r = Digitalize::new(Arc::clone(&data), ItemEffects::new(Arc::clone(&data), al, 99, ranks.clone())).compute(d, Some(&race_ids), true)?;
                    digi_result(h, &r);
                    Ok(())
                });
            }
        }

        // synthetic variants of the first three demons
        let equip_keys: Vec<i32> = t.equipment_skill.keys().copied().collect();
        let stack_keys: Vec<i32> = t.force_stack_tokusei.keys().copied().collect();
        let reunion_keys: Vec<i32> = t.reunion_bonus.keys().copied().filter(|&k| k > 0 && k < 256).collect();
        let skill_keys: Vec<i32> = t.skills.keys().copied().collect();
        for d in demons.iter().take(3) {
            let all: Vec<i32> = t.devils.get(&d.r#type).map(|dv| t.assists_of_race(dv.race_id).iter().map(|a| a.id).collect()).unwrap_or_default();
            let mut variants: Vec<(String, DemonRecord)> = Vec::new();
            let with = |f: &dyn Fn(&mut DemonRecord)| {
                let mut v = d.clone();
                f(&mut v);
                v
            };
            let eq0 = equip_keys.first().copied().unwrap_or(0);
            let eq1 = equip_keys.get(1).copied().unwrap_or(0);
            variants.push(("equip".into(), with(&|v| v.equipment = Some(vec![eq0, 0, 999, eq1]))));
            variants.push(("equip4".into(), with(&|v| v.equipment = Some(equip_keys.iter().take(4).copied().collect()))));
            variants.push(("equip-null".into(), with(&|v| v.equipment = None)));
            variants.push(("stack".into(), with(&|v| v.force_stack = Some(stack_keys.iter().take(7).copied().chain([99999]).collect()))));
            variants.push(("stack-null".into(), with(&|v| v.force_stack = None)));
            let mut i = 0;
            while i < stack_keys.len() && i < 96 {
                variants.push((format!("stack{i}"), with(&|v| v.force_stack = Some(stack_keys.iter().skip(i).take(8).copied().collect()))));
                i += 8;
            }
            let mut seed: u32 = 12345;
            for mt in 1..=4 {
                for rep in 0..3 {
                    let mut bytes = vec![0u8; 96];
                    for byte in bytes.iter_mut() {
                        let r1 = lcg(&mut seed) % 4;
                        let r2 = lcg(&mut seed);
                        *byte = if r1 == 0 { 0 } else { reunion_keys[(r2 as usize) % reunion_keys.len()] as u8 };
                    }
                    let rank = lcg(&mut seed) % 16;
                    variants.push((format!("mitama{mt}-{rep}"), with(&|v| {
                        v.mitama_type = mt;
                        v.mitama_rank = rank;
                        v.mitama_reunion = bytes.clone();
                    })));
                }
            }
            let fill: Vec<u8> = (0..96).map(|i| (((i % 4) * 32) + 1 + (i % 7)) as u8).collect();
            variants.push(("mitama-filled".into(), with(&|v| {
                v.mitama_type = 3;
                v.mitama_rank = 5;
                v.mitama_reunion = fill.clone();
            })));
            for lv in [0, 1, 4, 5, 24, 25, 28, 29, 48, 49, 60, 61, 80, 81, 99, 120] {
                variants.push((format!("lvl{lv}"), with(&|v| {
                    v.level = lv;
                    v.growth_type = 0;
                    v.reunion = vec![3, 2, 5, 1, 4, 0, 7, 2, 3, 1, 4, 6];
                })));
            }
            variants.push(("growth-bad".into(), with(&|v| v.growth_type = 99999)));
            variants.push(("type-bad".into(), with(&|v| v.r#type = 99999999)));
            variants.push(("skills".into(), with(&|v| v.skills = skill_keys.iter().take(7).copied().chain([99999]).collect())));
            let mut i = 0;
            while i < skill_keys.len() && i < 96 {
                variants.push((format!("skills{i}"), with(&|v| v.skills = skill_keys.iter().skip(i).take(8).copied().collect())));
                i += 8;
            }
            for (tag, v) in &variants {
                let assist_sets: [(&str, Option<Vec<i32>>, bool); 4] = [("all", Some(all.clone()), false), ("all+boost", Some(all.clone()), true), ("none", Some(vec![]), false), ("unknown", None, false)];
                for (aname, assists, boost) in &assist_sets {
                    h.try_(&format!("{who} variant {} {tag} assists={aname}", d.name), |h| {
                        let r = ctx.digitalize(95)?.compute(v, assists.as_deref(), *boost)?;
                        digi_result(h, &r);
                        Ok(())
                    });
                }
            }
        }
        h.p(&format!(
            "# set name {} {} {} {} {} | {} {} {}",
            digitalize::set_name(0),
            digitalize::set_name(5),
            digitalize::set_name(47),
            digitalize::set_name(48),
            digitalize::set_name(100),
            digitalize::correct_id(0),
            digitalize::correct_id(21),
            digitalize::correct_id(22)
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- CharacterExtras
fn extras_mode(h: &mut H) -> Result<()> {
    let data = Arc::clone(&h.data);
    let dirs = h.dirs();
    let ranks = Store::new(&dirs[0]).expertise_ranks()?;
    let fxc = ItemEffects::new(Arc::clone(&data), "NEUTRAL", 95, ranks);
    let opt_i = |v: Option<i32>| v.map(|x| x.to_string()).unwrap_or_default();
    let opt_s = |v: Option<i32>| v.map(|x| x.to_string()).unwrap_or_else(|| "~".into());
    for dir in &dirs {
        let store = Store::new(dir);
        let (qc, cc, v2) = store.counts()?;
        h.p(&format!(
            "# counts {} q={} c={} v2={} sources={}|{}",
            h.dir_tail(dir),
            opt_i(qc),
            opt_i(cc),
            v2.map(|x| if x { "True" } else { "False" }).unwrap_or(""),
            store.quest_count_source()?.unwrap_or_default(),
            store.compendium_count_source()?.unwrap_or_default()
        ));
        let st = ProgressState { quest_bonus_count: qc, compendium_count: cc, has_compendium_v2: v2, ..Default::default() };
        for inc in [false, true] {
            let tail = h.dir_tail(dir);
            h.try_(&format!("fixture {tail} clan={}", b(inc)), |h| {
                let r = extras::compute(&data, Some(&fxc), Some(&st), inc)?;
                h.fxs(&r);
                let rep = extras::report_state(&st, &data, &fxc, inc)?.replace("\r\n", "\n");
                h.p(&rep);
                Ok(())
            });
        }
    }
    let qs: [Option<i32>; 15] = [None, Some(0), Some(1), Some(5), Some(10), Some(31), Some(32), Some(33), Some(64), Some(100), Some(150), Some(200), Some(300), Some(500), Some(1000)];
    let cs: [Option<i32>; 10] = [None, Some(0), Some(1), Some(10), Some(50), Some(100), Some(300), Some(500), Some(1000), Some(2000)];
    let vs: [Option<bool>; 3] = [None, Some(true), Some(false)];
    for qv in qs {
        for cv in cs {
            for v in vs {
                h.try_(&format!("grid q={} c={} v2={}", opt_i(qv), opt_i(cv), v.map(|x| if x { "True" } else { "False" }).unwrap_or("")), |h| {
                    let st = ProgressState { quest_bonus_count: qv, compendium_count: cv, has_compendium_v2: v, ..Default::default() };
                    let r = extras::compute(&data, Some(&fxc), Some(&st), false)?;
                    h.fxs(&r);
                    Ok(())
                });
            }
        }
    }
    h.try_("null fx", |h| {
        let st = ProgressState { quest_bonus_count: Some(40), compendium_count: Some(70), ..Default::default() };
        let r = extras::compute(&data, None, Some(&st), false)?;
        h.fxs(&r);
        Ok(())
    });
    h.try_("null fx clan", |h| {
        let st = ProgressState { learned_skills: Some((7001..7013).collect()), quest_bonus_count: Some(40), compendium_count: Some(70), ..Default::default() };
        let r = extras::compute(&data, None, Some(&st), true)?;
        h.fxs(&r);
        Ok(())
    });
    h.try_("null state", |h| {
        let r = extras::compute(&data, Some(&fxc), None, true)?;
        h.fxs(&r);
        Ok(())
    });
    // clan skills: every level complete up to k
    let all_clan: IndexSet<i32> = (7000..=7040).collect();
    let mut learned_sets: Vec<(String, Option<IndexSet<i32>>)> =
        vec![("null".into(), None), ("empty".into(), Some(IndexSet::new())), ("all7000-7040".into(), Some(all_clan)), ("only7001".into(), Some([7001].into_iter().collect()))];
    let mut k = 1;
    while k <= 24 {
        learned_sets.push((format!("upto{}", 7000 + k), Some((7001..7001 + k).collect())));
        k += 3;
    }
    for (name, ls) in &learned_sets {
        for inc in [false, true] {
            let st = ProgressState {
                clan_name: if name == "empty" { None } else { Some(format!("Clan {name}")) },
                learned_skills: ls.clone(),
                quest_bonus_count: Some(32),
                compendium_count: Some(100),
                has_compendium_v2: None,
            };
            h.try_(&format!("clan {name} include={}", b(inc)), |h| {
                let lvl = extras::clan_level(&data, Some(&st))?;
                let skills = extras::clan_skills(&data, Some(&st))?;
                h.p(&format!(" level={} skills={}", lvl.map_or("~".to_string(), |l| l.to_string()), skills.iter().map(|c| format!("{}:{}:{}", c.0, c.1, c.2)).collect::<Vec<_>>().join(",")));
                let r = extras::compute(&data, Some(&fxc), Some(&st), inc)?;
                h.fxs(&r);
                let rep = extras::report_state(&st, &data, &fxc, inc)?.replace("\r\n", "\n");
                h.p(&rep);
                Ok(())
            });
        }
    }
    h.p(&format!("# clan level null state: {} skills={}", extras::clan_level(&data, None)?.map_or("~".to_string(), |l| l.to_string()), extras::clan_skills(&data, None)?.len()));
    for i in 6998..=7032 {
        h.p(&format!("isclan {i} {}", b(extras::is_clan_skill(&data, i)?)));
    }
    h.p(&format!("isclan 0 {}", b(extras::is_clan_skill(&data, 0)?)));

    // counts from quest ids and masks
    let mut qtypes: Vec<(i32, String)> = Vec::new();
    let qpath = config::repo_root().expect("repo root").join("reference").join("quest-types.tsv");
    for l in json::read_all_text(&qpath)?.lines() {
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = l.split('\t').collect();
        if r.len() > 1 {
            if let Ok(id) = r[0].parse::<i32>() {
                qtypes.push((id, r[1].to_string()));
            }
        }
    }
    let mut kinds: IndexSet<String> = IndexSet::new();
    for x in &qtypes {
        kinds.insert(x.1.clone());
    }
    h.p(&format!("# quest types {}: {}", qtypes.len(), kinds.iter().cloned().collect::<Vec<_>>().join(",")));
    h.p(&format!("questcount empty {}", opt_i(extras::quest_bonus_count(Some(Vec::<i32>::new()))?)));
    h.p(&format!("questcount null {}", opt_s(extras::quest_bonus_count(None::<Vec<i32>>)?)));
    h.p(&format!("questcount 0..200 {}", opt_i(extras::quest_bonus_count(Some(0..200))?)));
    h.p(&format!("questcount step3 {}", opt_i(extras::quest_bonus_count(Some((0..100).map(|i| 1 + i * 3)))?)));
    h.p(&format!("questcount dups {}", opt_i(extras::quest_bonus_count(Some(qtypes.iter().take(20).map(|x| x.0).chain(qtypes.iter().take(20).map(|x| x.0))))?)));
    for ty in &kinds {
        h.p(&format!("questcount type {ty} {}", opt_i(extras::quest_bonus_count(Some(qtypes.iter().filter(|x| &x.1 == ty).map(|x| x.0)))?)));
    }
    let mut seed: u32 = 777;
    let mut mask = |len: usize, mode: i32| -> Vec<u8> {
        (0..len)
            .map(|_| match mode {
                0 => 0u8,
                1 => 0xFF,
                _ => lcg(&mut seed) as u8,
            })
            .collect()
    };
    for mode in [0, 1, 2, 2, 2] {
        let m = mask(128, mode);
        h.p(&format!("questmask mode={mode} {}", opt_i(extras::quest_bonus_count_from_mask(Some(&m))?)));
        let m2 = mask(512, mode);
        h.p(&format!("compmask mode={mode} {}", opt_i(extras::compendium_count(Some(&m2))?)));
        let m3 = mask(100, mode);
        h.p(&format!("compmask short mode={mode} {}", opt_i(extras::compendium_count(Some(&m3))?)));
    }
    h.p(&format!("questmask null {}", opt_s(extras::quest_bonus_count_from_mask(None)?)));
    h.p(&format!("compmask null {}", opt_s(extras::compendium_count(None)?)));
    h.p(&format!("compmask empty {}", opt_i(extras::compendium_count(Some(&[] as &[u8]))?)));
    Ok(())
}
