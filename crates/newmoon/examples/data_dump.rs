//! Dumps what the data layer (`newmoon::data::*`, `newmoon::build::csv`) produces, as tab separated lines. The C# data layer
//! was dumped by a throwaway harness in the same format; the two files must be identical (that is how the port was checked).
//!
//! Usage: data_dump dump <out.txt> <work folder> <fixture folder (with data\ and smt-stats\)> <input folder (json-cases.json, json-write.json)>
//!        data_dump config-create <folder>      (folder must hold crates\newmoon\Cargo.toml or NewMoonReader.csproj; run with it as current folder)
//!        data_dump gen-statlabels              (rewrites reference\stat-labels-game.tsv next to the file it finds)

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use newmoon::Result;
use newmoon::build::csv::CsvTable;
use newmoon::data::characters::{CharacterEntry, Characters};
use newmoon::data::config;
use newmoon::data::game_data::{self, GameData, TokValue, reference_file};
use newmoon::data::json::{self, At, Escape, Row};
use newmoon::data::stat_labels;
use newmoon::data::store::{CachedTip, Store};
use newmoon::fmt::g;
use newmoon::game::{SetRecord, WornItem};
use serde_json::{Value, json};

struct Dumper {
    out: String,
    work: String,
    repo: String,
}

trait F {
    fn f(&self) -> String;
}
fn q(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\r', "\\r").replace('\n', "\\n")
}
impl F for str {
    fn f(&self) -> String {
        q(self)
    }
}
impl F for String {
    fn f(&self) -> String {
        q(self)
    }
}
impl<T: F + ?Sized> F for &T {
    fn f(&self) -> String {
        (**self).f()
    }
}
impl F for i32 {
    fn f(&self) -> String {
        self.to_string()
    }
}
impl F for i64 {
    fn f(&self) -> String {
        self.to_string()
    }
}
impl F for u64 {
    fn f(&self) -> String {
        self.to_string()
    }
}
impl F for usize {
    fn f(&self) -> String {
        self.to_string()
    }
}
impl F for bool {
    fn f(&self) -> String {
        self.to_string()
    }
}
impl F for f64 {
    fn f(&self) -> String {
        g(*self)
    }
}
impl F for Vec<i32> {
    fn f(&self) -> String {
        format!("[{}]", self.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
    }
}
impl F for [i32] {
    fn f(&self) -> String {
        format!("[{}]", self.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
    }
}
impl<T: F> F for Option<T> {
    fn f(&self) -> String {
        match self {
            Some(v) => v.f(),
            None => "<null>".into(),
        }
    }
}

macro_rules! l {
    ($d:expr, $($x:expr),+ $(,)?) => {{
        let parts: Vec<String> = vec![$(F::f(&$x)),+];
        $d.out.push_str(&parts.join("\t"));
        $d.out.push('\n');
    }};
}

fn join_i(v: &[i32]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let t = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &t)?;
        } else {
            std::fs::copy(e.path(), t)?;
        }
    }
    Ok(())
}

fn ts(s: &str) -> String {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2})?").unwrap());
    RE.replace_all(s, "<ts>").into_owned()
}

fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for x in b {
        h ^= *x as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn nn(v: &Value) -> Option<&Value> {
    if v.is_null() { None } else { Some(v) }
}

impl Dumper {
    fn p(&self, s: &str) -> String {
        s.replace(&self.work, "<W>").replace(&self.repo, "<R>")
    }
    fn pp(&self, s: &Path) -> String {
        self.p(&s.to_string_lossy())
    }
    fn sec(&mut self, name: &str, f: impl FnOnce(&mut Dumper) -> Result<()>) {
        self.out.push_str(&format!("== {name}\n"));
        if let Err(e) = f(self) {
            eprintln!("{name}: {e:#}");
            self.out.push_str("EXC\n");
        }
    }
}

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a[0].as_str() {
        "dump" => dump(&a[1], &a[2], &a[3], &a[4]),
        "config-create" => {
            let c = config::current();
            println!("{}", config::file_path().display());
            println!(
                "{}|{}|{}|{}|{}",
                c.website_content.as_deref().unwrap_or(""),
                c.tokusei_xml.as_deref().unwrap_or(""),
                c.server_constants.as_deref().unwrap_or(""),
                c.stats_folder.as_deref().unwrap_or(""),
                c.default_character.as_deref().unwrap_or("")
            );
            Ok(())
        }
        "gen-statlabels" => {
            let (n, p) = stat_labels::generate(&GameData::new())?;
            println!("({n}, {})", p.display());
            Ok(())
        }
        _ => std::process::exit(1),
    }
}

fn dump(out_file: &str, work: &str, fixture: &str, in_dir: &str) -> Result<()> {
    let repo = config::repo_root().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    if Path::new(work).exists() {
        std::fs::remove_dir_all(work)?;
    }
    std::fs::create_dir_all(work)?;
    let mut d = Dumper { out: String::new(), work: work.to_string(), repo };
    let gd = GameData::new();
    d.sec("config", config_t);
    d.sec("json", |d| json_t(d, in_dir));
    d.sec("characters", |d| chars_t(d, fixture));
    d.sec("demon-tables", |d| demon_t(d, &gd));
    d.sec("stat-labels", labels_t);
    d.sec("game-misc", |d| game_misc(d, &gd));
    d.sec("game-items", |d| game_items(d, &gd));
    d.sec("game-skills", |d| game_skills(d, &gd));
    d.sec("game-enchants", |d| game_enchants(d, &gd));
    d.sec("game-devils", |d| game_devils(d, &gd));
    d.sec("game-mods", |d| game_mods(d, &gd));
    d.sec("game-tokusei", |d| game_tokusei(d, &gd));
    d.sec("store", |d| store_t(d, fixture));
    d.sec("csv", |d| csv_t(d, fixture));
    std::fs::write(out_file, d.out)?;
    Ok(())
}

// ---------------------------------------------------------------- config
fn config_t(d: &mut Dumper) -> Result<()> {
    let c = config::current();
    l!(d, "websiteContent", c.website_content);
    l!(d, "tokuseiXml", c.tokusei_xml);
    l!(d, "serverConstants", c.server_constants);
    l!(d, "statsFolder", c.stats_folder);
    l!(d, "defaultCharacter", c.default_character);
    let (r, h, f) = (config::repo_root(), config::home_dir(), config::file_path());
    l!(d, "RepoRoot", r.map(|p| d.pp(&p)));
    l!(d, "HomeDir", d.pp(&h));
    l!(d, "FilePath", d.pp(&f));
    Ok(())
}

// ---------------------------------------------------------------- json helpers
fn json_t(d: &mut Dumper, in_dir: &str) -> Result<()> {
    let cases = json::load(Path::new(in_dir).join("json-cases.json"))?;
    for (i, v) in cases.as_array().unwrap().iter().enumerate() {
        let n = nn(v);
        l!(d, "case", i as i32, json::num(n), json::int(n), json::int_or(n, -7), json::str(n), json::truthy(n), json::arr(n).count());
    }
    let nested = json::parse("{\"a\":{\"b\":[1,2,{\"c\":\"d\"}]},\"n\":null,\"s\":\"x\"}")?;
    l!(d, "nested", json::str(nested.at("a").at("b").idx(2).at("c")), json::str(nested.at("n")), json::str(nested.at("zz").at("y")), json::int(nested.at("a").at("b").idx(0)));
    let doc = json::load(Path::new(in_dir).join("json-write.json"))?;
    let p = Path::new(&d.work).join("json").join("save.json");
    json::save(&p, &doc)?;
    l!(d, "save", std::fs::read_to_string(&p)?, std::fs::metadata(&p)?.len());
    let t1 = std::fs::metadata(&p)?.modified()?;
    std::thread::sleep(std::time::Duration::from_millis(60));
    json::save_if_changed(&p, &doc)?;
    l!(d, "saveIfChanged-same-untouched", std::fs::metadata(&p)?.modified()? == t1);
    json::save_if_changed(&p, &json::parse("{\"x\":1}")?)?;
    l!(d, "saveIfChanged-new", std::fs::read_to_string(&p)?);
    let row: Row = match json!({"id": 5, "name": "あ<&>", "list": [1, 2, 3], "empty": [], "null": null, "d": 2.5, "whole": 3.0, "b": true, "sub": {"k": "v"}}) {
        Value::Object(m) => m,
        _ => unreachable!(),
    };
    let p2 = Path::new(&d.work).join("json").join("row.json");
    json::save(&p2, &Value::Object(row))?;
    l!(d, "row", std::fs::read_to_string(&p2)?);
    let missing = Path::new(&d.work).join("json").join("none.json");
    match json::load(&missing) {
        Ok(_) => l!(d, "load-missing", "no-throw"),
        Err(e) if e.downcast_ref::<std::io::Error>().is_some() => l!(d, "load-missing", "IOException"),
        Err(_) => l!(d, "load-missing", "other"),
    }
    let bad = Path::new(&d.work).join("json").join("bad.json");
    std::fs::write(&bad, "{bad")?;
    match json::load(&bad) {
        Ok(_) => l!(d, "load-bad", "no-throw"),
        Err(e) if e.downcast_ref::<serde_json::Error>().is_some() => l!(d, "load-bad", "JsonException"),
        Err(_) => l!(d, "load-bad", "other"),
    }
    let bom = Path::new(&d.work).join("json").join("bom.json");
    std::fs::write(&bom, b"\xEF\xBB\xBF{\"a\":1}")?;
    l!(d, "load-bom", json::serialize(&json::load(&bom)?, Escape::Default, false));
    Ok(())
}

// ---------------------------------------------------------------- characters
fn chars_t(d: &mut Dumper, fixture: &str) -> Result<()> {
    let wd = Path::new(&d.work).join("chars");
    std::fs::create_dir_all(&wd)?;
    std::fs::copy(Path::new(fixture).join("data").join("characters.json"), wd.join("characters.json"))?;
    let def_csv = Path::new(&d.work).join("stats").join("default-stats.csv").to_string_lossy().into_owned();
    let mut ch = Characters::new(&wd, "Rubito", &def_csv)?;
    for e in &ch.all {
        l!(d, "char", e.name, e.dir, e.csv.as_deref().map(|c| d.p(c)), e.auto_picture);
    }
    l!(d, "find", ch.find("rubito").and_then(|e| e.name.clone()), ch.find("ROKUJUU").and_then(|e| e.name.clone()), ch.find("nobody").and_then(|e| e.name.clone()));
    for e in &ch.all {
        l!(d, "datadir", d.pp(&ch.data_dir(e)));
    }
    let mk = |dir: Option<&str>| CharacterEntry { name: Some("x".into()), dir: dir.map(String::from), csv: None, auto_picture: None };
    l!(d, "datadir-null", d.pp(&ch.data_dir(&mk(None))), d.pp(&ch.data_dir(&mk(Some("")))), d.pp(&ch.data_dir(&mk(Some("sub\\d")))));
    for name in ["New Char", "Roku.juu: ねこ", "A/B\\C?*\"<>|x", "  ", "Ünï Çode", "Hi\u{1}There", "..", "ROKUJUU", "new char"] {
        let e = ch.get_or_add(name)?.clone();
        l!(d, "add", e.name, e.dir, e.csv.as_deref().map(|c| d.p(c)), e.auto_picture);
    }
    ch.all[0].auto_picture = Some(false);
    ch.save()?;
    l!(d, "saved", std::fs::read_to_string(wd.join("characters.json"))?);
    let ch2 = Characters::new(&wd, "x", "y")?;
    l!(d, "reload", ch2.all.len(), ch2.all.iter().map(|e| e.name.clone().unwrap_or_default()).collect::<Vec<_>>().join("|"));

    let wd2 = Path::new(&d.work).join("chars2");
    std::fs::create_dir_all(wd2.join("dash"))?;
    std::fs::create_dir_all(wd2.join("cache"))?;
    std::fs::write(wd2.join("dash").join("snapshots.json"), "[{\"character\":\"First\"},{\"character\":\"Zed\u{3042}\"}]")?;
    let fb = Path::new(&d.work).join("stats").join("fb-stats.csv").to_string_lossy().into_owned();
    let ch3 = Characters::new(&wd2, "Fallback", &fb)?;
    l!(d, "pre", ch3.all.len(), std::fs::read_to_string(wd2.join("characters.json"))?);
    let wd3 = Path::new(&d.work).join("chars3");
    std::fs::create_dir_all(wd3.join("cache"))?;
    let ch4 = Characters::new(&wd3, "Fallback", &fb)?;
    l!(d, "pre-no-snapshots", ch4.all.len(), ch4.all[0].name, ch4.all[0].csv.as_deref().map(|c| d.p(c)));
    Ok(())
}

// ---------------------------------------------------------------- demon tables
fn demon_t(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let t = gd.demons()?;
    l!(d, "loaded", t.loaded(), t.devils.len(), t.growth.len(), t.growth_names.len(), t.reunion_bonus.len(), t.mitama_sets.len(), t.union_bonus.len(), t.assists.len(), t.skills.len(), t.levels.len(), t.force_stack_tokusei.len(), t.equipment_skill.len());
    for (k, v) in &t.devils {
        l!(d, "devil", *k, v.id, v.name, v.race, v.race_id, v.growth_type, v.fusion_options, v.base_level, v.traits.clone(), v.correct.clone(), v.is_mitama());
    }
    for (k, gr) in &t.growth {
        let s = gr.iter().map(|s| format!("[{}]", join_i(s))).collect::<Vec<_>>().join(";");
        l!(d, "growth", *k, t.growth_names[k], s);
    }
    for (k, v) in &t.reunion_bonus {
        l!(d, "reunion", *k, v.0, v.1);
    }
    for s in &t.mitama_sets {
        l!(d, "mitamaset", s.id, s.requirements.clone(), s.bonus.clone(), s.bonus_ex.clone(), s.description, s.ex_description);
    }
    for (k, v) in &t.union_bonus {
        l!(d, "union", *k, v.clone());
    }
    for (k, a) in &t.assists {
        l!(d, "assist", *k, a.id, a.race_id, a.r#type, a.value);
    }
    for (k, v) in &t.skills {
        l!(d, "skill", *k, v.0, v.1);
    }
    for (k, ls) in &t.levels {
        for x in ls {
            l!(d, "level", *k, x.race_id, x.level, x.next_xp, x.assists.clone(), x.demon_ids.clone(), x.extend_skill_id);
        }
    }
    for (k, v) in &t.force_stack_tokusei {
        l!(d, "forcestack", *k, v.clone());
    }
    for (k, v) in &t.equipment_skill {
        l!(d, "equipskill", *k, *v);
    }
    l!(d, "digitalizeRaces", t.digitalize_races());
    let mut races: Vec<i32> = Vec::new();
    for r in t.levels.keys().copied().chain(t.assists.values().map(|a| a.race_id)).chain([99999, -1]) {
        if !races.contains(&r) {
            races.push(r);
        }
    }
    for &r in &races {
        l!(d, "assistsOfRace", r, t.assists_of_race(r).iter().map(|a| a.id).collect::<Vec<_>>());
        l!(d, "raceName", r, t.race_name(r));
        let ex = (-1..17).map(|lv| format!("[{}]", join_i(&t.extend_skills(r, lv)))).collect::<Vec<_>>().join(";");
        l!(d, "extend", r, ex);
        let pct = [(0, 0), (0, 1), (1, 5000), (3, 123456), (15, 10), (16, 1), (-1, 1)]
            .iter()
            .map(|x| t.percent_to_next(r, x.0, x.1).map(g).unwrap_or_else(|| "null".into()))
            .collect::<Vec<_>>()
            .join(";");
        l!(d, "pct", r, pct);
    }
    let dr = t.digitalize_races();
    let lv: IndexMap<i32, i32> = [(dr[0], 7), (dr[1], 15), (dr[2], 3), (99999, 4)].into_iter().collect();
    let pts: IndexMap<i32, i32> = [(dr[0], 1234), (dr[2], 5)].into_iter().collect();
    for x in t.race_levels(Some(&lv), Some(&pts)) {
        l!(d, "racelevel", x.race_id, x.race, x.level, x.points, x.percent_to_next, x.extend_skills);
    }
    for x in t.race_levels(None, None).into_iter().take(3) {
        l!(d, "racelevel0", x.race_id, x.race, x.level, x.points, x.percent_to_next, x.extend_skills);
    }
    l!(d, "allExtend", t.all_extend_skills(Some(&lv)).into_iter().collect::<Vec<_>>());
    l!(d, "allExtendNull", t.all_extend_skills(None).len());
    Ok(())
}

// ---------------------------------------------------------------- stat labels
fn labels_t(d: &mut Dumper) -> Result<()> {
    let lines = reference_file("stat-labels-game.tsv").unwrap().lines()?;
    let keys: Vec<String> = lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split('\t').next().unwrap().to_string()).collect();
    l!(d, "count", keys.len());
    for k in &keys {
        l!(d, "label", k, stat_labels::of(k), stat_labels::name(k, 0), stat_labels::name(k, 4), stat_labels::has(k));
    }
    for k in ["MOVE1", "SUMMON_SPEED", "UNKNOWN_STAT", "", "NRA_FIRE", "NRA_SLASH", "NRA_UNKNOWN", "BOOST_SLASH", "BOOST_WEAPON", "AFFINITY_CAP_MAX"] {
        l!(d, "probe", k, stat_labels::of(k), stat_labels::name(k, 3), stat_labels::name(k, 4), stat_labels::name(k, 5), stat_labels::has(k));
    }
    l!(d, "null", Option::<&str>::None, stat_labels::name_opt(None, 0), false);
    for e in stat_labels::ELEMENT_IDS.iter() {
        l!(d, "elementid", *e, e.and_then(stat_labels::element));
    }
    l!(d, "elementIdsLen", stat_labels::ELEMENT_IDS.len());
    for n in -2..=25 {
        l!(d, "elementNo", n, stat_labels::element_by_number(n));
    }
    l!(d, "element-unknown", stat_labels::element("NOPE"));
    Ok(())
}

// ---------------------------------------------------------------- game data
fn game_misc(d: &mut Dumper, gd: &GameData) -> Result<()> {
    l!(d, "content", gd.content);
    l!(d, "tokuseiXmlDir", gd.tokusei_xml_dir);
    l!(d, "mitamaSetBoost", gd.mitama_set_boost_skill()?);
    for n in ["DIGITALIZE_STAT_RATES", "MITAMA_SET_BOOST", "NOPE", ""] {
        l!(d, "constant", n, gd.server_constant(n)?);
    }
    let mut ids: Vec<i32> = vec![-1, 0, 999999];
    for line in reference_file("traits-unresolved.tsv").unwrap().lines()? {
        if !line.is_empty() && !line.starts_with('#') {
            if let Some(id) = newmoon::fmt::parse_i32(line.split('\t').next().unwrap()) {
                ids.push(id);
            }
        }
    }
    let mut seen = Vec::new();
    for id in ids {
        if !seen.contains(&id) {
            seen.push(id);
            l!(d, "trait", id, gd.trait_description(id)?);
        }
    }
    l!(d, "enchantTable", game_data::game_enchant_table().and_then(|f| f.disk_path().map(|p| d.pp(p))));
    l!(d, "refFile", reference_file("devils-game.tsv").and_then(|f| f.disk_path().map(|p| d.pp(p))), reference_file("nope.tsv").and_then(|f| f.disk_path().map(|p| d.pp(p))));
    let tok = gd.tokusei()?.as_object().unwrap();
    l!(d, "tokusei-count", tok.len());
    l!(d, "tokusei-first", tok.keys().next().cloned());
    l!(d, "tokusei-last", tok.keys().last().cloned());
    for (name, v) in [("expertises", gd.expertises()?), ("rankSkills", gd.rank_skills()?)] {
        let o = v.as_object().unwrap();
        l!(d, name, o.len(), o.keys().cloned().collect::<Vec<_>>().join(","));
    }
    for (id, e) in gd.expertises_nm()? {
        let chain = e.chain.iter().map(|c| format!("{}:{}:{}", c.0, c.1, g(c.2))).collect::<Vec<_>>().join(";");
        l!(d, "expNm", *id, e.id, e.name, e.max_class, e.is_chain, chain);
    }
    for (id, v) in gd.rank_skills_nm()? {
        let s = v.iter().map(|x| format!("{}={}", x.0, join_i(&x.1))).collect::<Vec<_>>().join(";");
        l!(d, "rankNm", *id, s);
    }
    l!(d, "passives", gd.passives()?.len());
    for (k, v) in gd.passives()? {
        l!(d, "passive", k, if v.is_null() { None } else { Some(json::serialize(v, Escape::Default, false)) });
    }
    Ok(())
}

fn game_items(d: &mut Dumper, gd: &GameData) -> Result<()> {
    l!(d, "items", gd.items()?.len());
    for id in gd.items()?.keys().copied().collect::<Vec<_>>() {
        let ic = gd.item_icon(id)?;
        let correct = gd.item_correct(id)?.unwrap_or_default().iter().map(|c| format!("{}:{}:{}", c.id.as_deref().unwrap_or(""), c.r#type, g(c.value))).collect::<Vec<_>>().join(";");
        l!(d, "item", id, gd.item_name(id)?, ic.0, ic.1, gd.item_equip_type(id)?, gd.item_weapon_type(id)?, gd.item_sub_category(id)?, correct, gd.sitem_tokusei(id)?);
    }
    for id in [-5, 0, 1, 99999999] {
        let ic = gd.item_icon(id)?;
        l!(d, "item-missing", id, gd.item_name(id)?, ic.0, ic.1, gd.item_equip_type(id)?, gd.item_weapon_type(id)?, gd.item_sub_category(id)?, gd.item_correct(id)?.is_none(), gd.sitem_tokusei(id)?);
    }
    Ok(())
}

fn game_skills(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let names = gd.skill_names()?;
    l!(d, "skillNames", names.len());
    let mut ids: BTreeSet<i32> = names.keys().copied().collect();
    for (id, n) in names {
        l!(d, "skillName", *id, n);
    }
    for f in ["skill-icons.tsv", "skill-text.tsv"] {
        for line in reference_file(f).unwrap().lines()? {
            if let Some(id) = newmoon::fmt::parse_i32(line.split('\t').next().unwrap()) {
                ids.insert(id);
            }
        }
    }
    ids.extend([-1, 0, 99999999]);
    for id in ids {
        l!(d, "skill", id, gd.skill_text(id)?, gd.skill_icon_asset(id)?);
    }
    Ok(())
}

fn game_enchants(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let en = gd.enchants()?;
    l!(d, "enchants", en.len());
    for (id, e) in en {
        let ids = gd.enchant_ids_by_crystal(e.crystal.as_deref().unwrap_or(""))?;
        l!(d, "enchant", *id, e.id, e.crystal, e.tarot_name, e.soul_name, GameData::crystal_key(e.crystal.as_deref()), gd.enchant_name(*id)?, join_i(&ids), gd.is_crystal(e.crystal.as_deref().unwrap_or(""))?);
    }
    l!(d, "enchantName-missing", gd.enchant_name(-5)?, gd.enchant_name(99999)?);
    for s in gd.enchant_sets()? {
        let c = s.conditions.iter().map(|c| format!("{}={}", c.0, join_i(&c.1))).collect::<Vec<_>>().join(";");
        l!(d, "enchantSet", s.id, s.effects.clone(), s.tokusei.clone(), c);
    }
    let fi = gd.fusion_icon_items()?;
    l!(d, "fusionIcon", fi.0, fi.1);
    for k in [Some("Cerberus' Crystal"), Some("Lucifuge Crystal"), Some("Soul Stone: Procyon"), Some("  "), Some(""), None, Some("Odin's   Crystal "), Some("X'"), Some("ABC crystal"), Some("Tarot Card"), Some("Crystal"), Some("With Orthrus:"), Some("orthrus")] {
        l!(d, "crystalKey", k, GameData::crystal_key(k), join_i(&gd.enchant_ids_by_crystal(k.unwrap_or(""))?), gd.is_crystal(k.unwrap_or(""))?);
    }
    let sides = gd.enchant_sides()?;
    l!(d, "sides", sides.len());
    for ((id, tarot), s) in sides {
        let c = s.conditions.iter().map(|c| format!("{}:{}:{}>{}", c.0, c.1, c.2, join_i(&c.3))).collect::<Vec<_>>().join(";");
        l!(d, "side", *id, *tarot, s.name, s.tokusei.clone(), c, s.mask, s.crystal_item);
    }
    for (id, tarot) in [(64, true), (3105, false), (111, false), (3105, true), (-1, true)] {
        l!(d, "effect", id, tarot, gd.enchant_effect(id, tarot)?.and_then(|e| e.name.clone()));
    }
    Ok(())
}

fn game_devils(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let mut ids: BTreeSet<i32> = gd.demons()?.devils.keys().copied().collect();
    ids.extend([-1, 0, 99999999]);
    for id in ids {
        let f = gd.devil_icon_file(id)?;
        l!(d, "devilIcon", id, f.as_ref().map(|x| x.0.clone()), f.as_ref().map(|x| x.1.clone()), gd.devil_alignment(id)?);
    }
    Ok(())
}

fn game_mods(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let mut n = 0;
    for e in 0..600 {
        let v = gd.mod_tokusei(true, 0, 0, e)?;
        if v != 0 {
            l!(d, "modW", e, v);
            n += 1;
        }
    }
    for sc in -1..80 {
        for eq in 0..14 {
            for e in 0..500 {
                let v = gd.mod_tokusei(false, sc, eq, e)?;
                if v != 0 {
                    l!(d, "modA", sc, eq, e, v);
                    n += 1;
                }
            }
        }
    }
    l!(d, "mods-nonzero", n);
    Ok(())
}

fn tv(v: &TokValue) -> String {
    format!("{}|{}|{}|{}|{}|{}", v.id.as_deref().unwrap_or(""), v.r#type, g(v.value), v.mult, v.precision, v.modifier.map(g).unwrap_or_else(|| "null".into()))
}

fn game_tokusei(d: &mut Dumper, gd: &GameData) -> Result<()> {
    let mut ids: BTreeSet<i32> = [-1, 0, 1, 2, 999999999].into_iter().collect();
    for k in gd.tokusei()?.as_object().unwrap().keys() {
        if let Some(id) = newmoon::fmt::parse_i32(k) {
            ids.insert(id);
        }
    }
    let mut xml_ids: Vec<i32> = Vec::new();
    let re = regex::Regex::new("<member name=\"ID\">(\\d+)</member>")?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(gd.tokusei_xml_dir.as_deref().unwrap())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            n.starts_with("tokusei_") && n.ends_with(".xml")
        })
        .collect();
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    for f in files {
        for m in re.captures_iter(&json::read_all_text(&f)?) {
            let id: i32 = m[1].parse()?;
            ids.insert(id);
            xml_ids.push(id);
        }
    }
    let mut missing: Vec<i32> = Vec::new();
    for line in reference_file("tokusei-missing.tsv").unwrap().lines()? {
        if let Some(id) = newmoon::fmt::parse_i32(line.split('\t').next().unwrap()) {
            ids.insert(id);
            missing.push(id);
        }
    }
    for line in reference_file("tokusei-client-text.tsv").unwrap().lines()? {
        if let Some(id) = newmoon::fmt::parse_i32(line.split('\t').next().unwrap()) {
            ids.insert(id);
        }
    }
    l!(d, "ids", ids.len(), xml_ids.len(), missing.len());
    for &id in &ids {
        let Some(t) = gd.tokusei_full(id)? else {
            l!(d, "tok", id, Option::<&str>::None);
            continue;
        };
        let conds = t.conditions.iter().map(|c| format!("{}|{}|{}|{}", c.r#type.as_deref().unwrap_or(""), c.value, c.cmp, c.group)).collect::<Vec<_>>().join(";");
        let sk = t.skill_conditions.iter().map(|c| format!("{}|{}", c.0.as_deref().unwrap_or(""), c.1)).collect::<Vec<_>>().join(";");
        let cor = t.correct.iter().map(tv).collect::<Vec<_>>().join(";");
        let asp = t.aspects.iter().map(tv).collect::<Vec<_>>().join(";");
        l!(d, "tok", id, t.id, t.target, conds, sk, t.target_conditions, cor, asp, t.from_client_text);
    }
    let mut seen = BTreeSet::new();
    for &id in &missing {
        if !seen.insert(id) {
            continue;
        }
        let dd = gd.client_tokusei(id)?;
        let s = match dd {
            None => "null".to_string(),
            Some(t) => format!(
                "{}/{}/{}/{}",
                t.target,
                t.correct.iter().map(tv).collect::<Vec<_>>().join(";"),
                t.aspects.iter().map(tv).collect::<Vec<_>>().join(";"),
                t.skill_conditions.iter().map(|c| format!("{}|{}", c.0.as_deref().unwrap_or(""), c.1)).collect::<Vec<_>>().join(";")
            ),
        };
        l!(d, "clientTok", id, gd.client_tokusei_text(id)?, s);
    }
    let mut seen = Vec::new();
    for id in xml_ids.iter().copied().filter(|i| i % 7 == 0).chain([-1, 0, 5, 999999999]) {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let r = gd.raw_tokusei_xml(id)?;
        let asp = r.as_ref().map(|r| r.aspects.iter().map(|a| format!("{}={}", a.0, a.1)).collect::<Vec<_>>().join(";"));
        l!(d, "raw", id, r.as_ref().and_then(|r| r.target.clone()), asp, r.as_ref().map(|r| r.digitalized), gd.raw_tokusei_types(id)?);
    }
    Ok(())
}

// ---------------------------------------------------------------- store
fn pi(w: &Option<WornItem>) -> String {
    Store::pack_item(w.as_ref())
}

fn store_t(d: &mut Dumper, fixture: &str) -> Result<()> {
    let sd = Path::new(&d.work).join("store");
    copy_dir(&Path::new(fixture).join("data"), &sd)?;
    let st = Store::new(&sd);
    let ps = [
        st.dir.clone(),
        st.cache_dir(),
        st.dash_dir(),
        st.last_stats_path(),
        st.tooltip_cache_path(),
        st.set_table_path(),
        st.profile_path(),
        st.digitalize_path(),
        st.demon_picture_path(),
        st.comp_demons_path(),
        st.digitalize_progress_path(),
        st.demon_icon_path(5),
        st.item_icon_path(7),
        st.ui_icon_path("compendium"),
        st.skill_icon_dir(),
        st.client_dir_path(),
        st.tooltips_path(),
        st.expertise_ranks_path(),
    ];
    let ps: Vec<String> = ps.iter().map(|p| d.pp(p)).collect();
    d.out.push_str(&format!("paths	{}
", ps.iter().map(|s| q(s)).collect::<Vec<_>>().join("	")));
    l!(d, "alignment", st.alignment()?);
    for (k, v) in st.expertise_classes()? {
        l!(d, "expClass", k, v);
    }
    for (k, v) in st.expertise_ranks()? {
        l!(d, "expRank", k, v);
    }
    l!(d, "toggles", st.active_toggles()?);
    let c = st.counts()?;
    l!(d, "counts", c.0, c.1, c.2, st.quest_count_source()?, st.compendium_count_source()?);
    let sets = st.load_set_table()?;
    l!(d, "sets", sets.as_ref().map(|s| s.len()));
    for s in sets.unwrap_or_default() {
        l!(d, "set", s.id, s.pieces.clone(), s.tokusei.clone());
    }
    let tips = st.load_tooltip_cache()?;
    l!(d, "tipcache", tips.len());
    for (id, t) in &tips {
        l!(d, "tip", *id, t.text, t.seen_at);
    }
    let recs = st.load_tooltips()?;
    l!(d, "tooltips", recs.len(), st.tooltips_level()?);
    for r in &recs {
        l!(d, "slot", r.index, r.id, r.text, r.tarot, r.soul, pi(&r.detail));
    }
    let det = Store::unpack_details(&recs);
    l!(d, "unpackDetails", det.as_ref().map(|x| Store::pack_items(Some(x))));
    let un = Store::unpack(&recs);
    l!(d, "unpack", un.0.len(), un.1.clone(), un.2.clone(), un.3.clone());
    l!(d, "unpackDetails-short", Store::unpack_details(&recs[..14]).is_none());
    let no_detail: Vec<_> = recs.iter().cloned().map(|mut r| { r.detail = None; r }).collect();
    l!(d, "unpackDetails-nodetail", Store::unpack_details(&no_detail).is_none());
    let no_ench: Vec<_> = recs.iter().cloned().enumerate().map(|(i, mut r)| { if i == 3 { r.tarot = None; } r }).collect();
    let un2 = Store::unpack(&no_ench);
    l!(d, "unpack-noench", un2.2.is_none(), un2.3.is_none());
    for s in [Some("-"), Some("1:2:3:4:5:1,2,3,4,5:1,2,3:4:5"), Some(" 1:2:3:4:5:1,2,3,4,5:1,2,3:4:5"), Some("1:2"), Some("1:2:3:4:5:1,2,3,4:1,2,3:4:5"), Some("a:2:3:4:5:1,2,3,4,5:1,2,3:4:5"), Some("1:2:3:4:5:1,2,3,4,5::4:5"), None, Some(""), Some("+1:-2:3:4:5:1,2,3,4,5:1,2,3:4:5"), Some("1:2:3:4:5:1,2,3,4,5:1,2,3:4:5:6"), Some("1: 2 :3:4:5:1,2,3,4,5:1,2,3:4:-1")] {
        let w = Store::parse_item(s);
        l!(d, "parseItem", s, w.as_ref().map(|w| Store::pack_item(Some(w))).unwrap_or_else(|| "null".into()));
    }
    let good = "1:2:3:4:5:1,2,3,4,5:1,2,3:4:5";
    let mut parts: Vec<&str> = vec![good; 14];
    parts.push("-");
    let fifteen = parts.join(" ");
    let pi15 = Store::parse_items(Some(&fifteen));
    l!(d, "parseItems", Store::pack_items(pi15.as_deref()), pi15.as_ref().unwrap()[14].is_none());
    l!(d, "parseItems-bad", Store::parse_items(Some(&fifteen.replacen(good, "x", 1))).is_none());
    l!(d, "parseItems-short", Store::parse_items(Some(&format!("{good} {good}"))).is_none(), Store::parse_items(None).is_none(), Store::parse_items(Some("")).is_none());
    l!(d, "parseItems-spaces", Store::parse_items(Some(&format!("  {} ", vec!["-"; 15].join("  ")))).map(|v| v.len()));
    l!(d, "packItems", Store::pack_items(None), Store::pack_item(None));

    // writes
    l!(d, "profile-before", ts(&std::fs::read_to_string(st.profile_path())?));
    let prof = |st: &Store| -> Result<String> { Ok(ts(&std::fs::read_to_string(st.profile_path())?)) };
    st.save_alignment(Some("CHAOS"))?;
    l!(d, "profile-1", prof(&st)?);
    st.save_toggles([5, 5, 7, 0, -3, 9, 7])?;
    l!(d, "profile-2", prof(&st)?);
    st.save_counts(Some(5), None, Some(true), false, false)?;
    l!(d, "profile-3", prof(&st)?);
    st.save_counts(Some(5), Some(61), None, true, false)?;
    l!(d, "profile-4", prof(&st)?);
    st.save_counts(None, None, None, false, true)?;
    l!(d, "profile-5", prof(&st)?);
    let c = st.counts()?;
    l!(d, "counts-after", c.0, c.1, c.2, st.quest_count_source()?, st.compendium_count_source()?.map(|s| ts(&s)));
    l!(d, "alignment-after", st.alignment()?, st.active_toggles()?);
    st.save_alignment(Some(""))?;
    l!(d, "alignment-empty", st.alignment()?);
    st.save_alignment(Some("lawful"))?;
    l!(d, "alignment-lower", st.alignment()?);
    let mut tc: IndexMap<i32, CachedTip> = IndexMap::new();
    tc.insert(30, CachedTip { text: Some("line1\nline2 あ <&>".into()), seen_at: Some("2026-11-22T19:18:17".into()) });
    tc.insert(7, CachedTip { text: None, seen_at: None });
    tc.insert(100000, CachedTip { text: Some("".into()), seen_at: Some("x\"y".into()) });
    st.save_tooltip_cache(&tc)?;
    l!(d, "tipcache-saved", std::fs::read_to_string(st.tooltip_cache_path())?);
    let back = st.load_tooltip_cache()?;
    l!(d, "tipcache-back", back.len(), back.iter().map(|(k, v)| format!("{}={}@{}", k, v.text.clone().unwrap_or_default(), v.seen_at.clone().unwrap_or_default())).collect::<Vec<_>>().join("|"));
    let sl = vec![
        SetRecord { id: 1, pieces: vec![0, 0, 5, 6], tokusei: vec![10, 20] },
        SetRecord { id: 2, pieces: vec![], tokusei: vec![] },
        SetRecord { id: 3, pieces: vec![1], tokusei: vec![0, 0] },
    ];
    st.save_set_table(&sl)?;
    l!(d, "sets-saved", std::fs::read_to_string(st.set_table_path())?);
    let sb = st.load_set_table()?.unwrap();
    l!(d, "sets-back", sb.iter().map(|s| format!("{}:{}:{}", s.id, join_i(&s.pieces), join_i(&s.tokusei))).collect::<Vec<_>>().join("|"));
    let (texts, ids) = (un.0.clone(), un.1.clone());
    st.save_tooltips(&ids, &texts, un.2.as_deref(), un.3.as_deref(), det.as_deref(), 95)?;
    l!(d, "tooltips-saved", std::fs::read_to_string(st.tooltips_path())?);
    let again = st.load_tooltips()?;
    l!(d, "tooltips-again", again.len(), st.tooltips_level()?, again.iter().map(|r| format!("{}:{}:{}:{}:{}:{}", r.index, r.id, r.tarot.map(|x| x.to_string()).unwrap_or_default(), r.soul.map(|x| x.to_string()).unwrap_or_default(), pi(&r.detail), r.text.encode_utf16().count())).collect::<Vec<_>>().join("|"));
    st.save_tooltips(&ids, &texts, None, None, None, 0)?;
    l!(d, "tooltips-plain", std::fs::read_to_string(st.tooltips_path())?);
    l!(d, "tooltips-plain-level", st.tooltips_level()?, st.load_tooltips()?.len());
    std::fs::write(st.tooltips_path(), "level 7x\r\n===== record 0 (id 5)\r\nabc  \r\n\r\n===== record 1 (id -2) (tarot 3 soul 4)\r\nx\r\n===== record 2 (id 9) (item 1:2:3:4:5:1,2,3,4,5:1,2,3:4:5)\r\nlast")?;
    let odd = st.load_tooltips()?.iter().map(|r| format!("{}:{}:{}:{}:{}:{}", r.index, r.id, r.tarot.map(|x| x.to_string()).unwrap_or_default(), r.soul.map(|x| x.to_string()).unwrap_or_default(), pi(&r.detail), q(&r.text))).collect::<Vec<_>>().join("|");
    l!(d, "tooltips-odd", st.tooltips_level()?, odd);
    std::fs::write(st.tooltips_path(), "level 12\n===== record 0 (id 5)\nLF only\n")?;
    let lf = st.load_tooltips()?.iter().map(|r| format!("{}:{}:{}", r.index, r.id, q(&r.text))).collect::<Vec<_>>().join("|");
    l!(d, "tooltips-lf", st.tooltips_level()?, lf);
    let sj = st.load_json(st.set_table_path())?;
    l!(d, "loadJson", st.load_json(sd.join("nope.json"))?.is_none(), sj.map(|v| json::serialize(&v, Escape::Default, false).len()));

    // an empty folder
    let s2 = Store::new(Path::new(&d.work).join("store-empty"));
    let c2 = s2.counts()?;
    l!(d, "empty", s2.alignment()?, s2.active_toggles()?, c2.0, c2.2, s2.quest_count_source()?, s2.expertise_classes()?.len(), s2.expertise_ranks()?.len(), s2.load_set_table()?.is_none(), s2.load_tooltip_cache()?.len(), s2.tooltips_level()?);
    s2.save_counts(Some(3), Some(4), Some(false), false, false)?;
    l!(d, "empty-profile", ts(&std::fs::read_to_string(s2.profile_path())?));
    s2.save_toggles([1, 2])?;
    l!(d, "empty-profile2", ts(&std::fs::read_to_string(s2.profile_path())?));
    let s3 = Store::new(Path::new(&d.work).join("store-empty2"));
    s3.save_toggles([3])?;
    l!(d, "empty-profile3", ts(&std::fs::read_to_string(s3.profile_path())?));
    std::fs::write(s3.expertise_ranks_path(), "{\"ranks\":[{\"id\":4,\"name\":\"Rapid\",\"class\":10,\"rank\":3},{\"id\":5.7,\"name\":\"rapid\",\"class\":2,\"rank\":1},{\"name\":\"\"},{\"id\":\"9\",\"name\":\"Z\"},null]}")?;
    l!(d, "expOdd", s3.expertise_classes()?.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("|"), s3.expertise_ranks()?.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("|"));
    Ok(())
}

// ---------------------------------------------------------------- csv
fn csv_t(d: &mut Dumper, fixture: &str) -> Result<()> {
    let dir = Path::new(fixture).join("smt-stats");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "csv")).collect();
    files.sort_by(|a, b| a.file_name().unwrap().to_string_lossy().as_bytes().cmp(b.file_name().unwrap().to_string_lossy().as_bytes()));
    for f in files {
        let t = CsvTable::load(&f)?;
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        l!(d, "csv", name, t.columns.len(), t.rows.len(), t.columns.iter().take(4).cloned().collect::<Vec<_>>().join(","), t.columns.last().cloned());
        let mut row_hash = String::new();
        for r in &t.rows {
            let mut h: i32 = 17;
            for c in &t.columns {
                for ch in t.get(r, c).encode_utf16() {
                    h = h.wrapping_mul(31).wrapping_add(ch as i32);
                }
            }
            write!(row_hash, "{h},")?;
        }
        l!(d, "csvhash", name, row_hash);
        let outp = Path::new(&d.work).join("csv").join(&name);
        t.save(&outp)?;
        let (a, b) = (std::fs::read(&f)?, std::fs::read(&outp)?);
        l!(d, "csvresave", name, a.len(), b.len(), fnv(&b), a == b);
        if let Some(last) = t.rows.last() {
            l!(d, "csvlast", name, t.columns.iter().take(6).map(|c| t.get(last, c)).collect::<Vec<_>>().join("|"), t.get(&t.rows[0], "nope-column"));
        }
    }
    let p = Path::new(&d.work).join("csv").join("edge.csv");
    for (label, text) in [
        ("plain", "a,b\r\n1,2\r\n"),
        ("bom", "\u{feff}\"a\",\"b\"\n\"1\",\"2\""),
        ("quotes", "a,b\r\n\"x,\"\"y\"\"\",z\r\n\r\n1\r\n"),
        ("lone-cr", "a,b\r1,2\r3,4"),
        ("unterminated", "a,b\n\"1,2\n3"),
        ("empty", ""),
        ("only-header", "a,b,c"),
        ("blank-lines", "\n\na\n\n1\n"),
        ("long-row", "a\n1,2,3\n"),
        ("dup-col", "a,a\n1,2\n"),
    ] {
        std::fs::write(&p, text)?;
        let t = CsvTable::load(&p)?;
        let o2 = PathBuf::from(format!("{}.out", p.display()));
        t.save(&o2)?;
        let rows = t.rows.iter().map(|r| r.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(",")).collect::<Vec<_>>().join("/");
        l!(d, "edge", label, t.columns.join("|"), t.rows.len(), rows, std::fs::read_to_string(&o2)?);
    }
    l!(d, "missing", CsvTable::load(Path::new(&d.work).join("csv").join("nope.csv"))?.columns.len());
    Ok(())
}
