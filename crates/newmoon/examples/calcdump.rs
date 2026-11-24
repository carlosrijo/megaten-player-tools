//! Throwaway checker for the calc ports: the same dump as the C# harness (`harness calc dump`), to be diffed.
#![allow(dead_code)]
//!   calcdump <corpus.txt> <calc-in dir> <stat-labels-game.tsv> <out file>
use newmoon::calc::bonuses::{self, calc_fmt, StatLabelSource};
use newmoon::calc::character_hp_mp as hp;
use newmoon::calc::effect::Effect;
use newmoon::calc::tooltip_parser::{TooltipInfo, TooltipParser};
use newmoon::fmt::g;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::path::PathBuf;

const SEP: &str = "=====TIP=====";

struct Labels(HashMap<String, String>);

impl StatLabelSource for Labels {
    fn of(&self, id: &str) -> Option<String> {
        self.0.get(id).cloned()
    }
}

fn s(x: &str) -> &str {
    x
}

fn so(x: &Option<String>) -> String {
    x.clone().unwrap_or_else(|| "~".to_string())
}

fn fx_line(e: &Effect) -> String {
    format!(
        "label={} value={} pct={} cond={} note={} target={} source={} item={}",
        e.label,
        e.value.map(g).unwrap_or_else(|| "~".into()),
        if e.pct { 1 } else { 0 },
        e.cond,
        e.cond_note,
        e.target,
        e.source,
        e.item
    )
}

fn eff(label: &str, value: Option<f64>, cond: &str, item: &str, source: &str) -> Effect {
    Effect { label: label.into(), value, cond: cond.into(), item: item.into(), source: source.into(), ..Default::default() }
}

/// The whole dump (also run by tests/calc_golden.rs against the C# output).
pub fn dump(corpus_path: &std::path::Path, in_dir: &std::path::Path, labels_path: &std::path::Path) -> String {
    let corpus = std::fs::read_to_string(corpus_path).unwrap();
    let in_dir = in_dir.to_path_buf();
    let mut labels = Labels(HashMap::new());
    for line in std::fs::read_to_string(labels_path).unwrap().lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let r: Vec<&str> = line.split('\t').collect();
        if r.len() > 1 && !r[1].is_empty() {
            labels.0.insert(r[0].to_string(), r[1].to_string());
        }
    }
    let mut out = String::new();
    macro_rules! w {
        ($($t:tt)*) => {{ let _ = writeln!(out, $($t)*); }};
    }
    let sep = format!("\n{SEP}\n");
    let texts: Vec<&str> = corpus.split(sep.as_str()).collect();
    let infos: Vec<TooltipInfo> = texts.iter().map(|t| TooltipParser::info(t)).collect();
    w!("corpus {}", texts.len());
    type Ctx = (&'static str, Vec<(String, i32)>, bool);
    let contexts: Vec<Ctx> = vec![
        ("law", vec![("Rapid".into(), 4), ("Shot".into(), 2)], false),
        ("Chaos", vec![("rapid".into(), 5), ("SPIN".into(), 6)], false),
        ("NEUTRAL", vec![], true),
    ];
    let mut all: Vec<Effect> = Vec::new();
    let mut label_seen: Vec<String> = Vec::new();
    for (i, info) in infos.iter().enumerate() {
        w!("T{i} name={} base={} sex={} tarot={} soul={} lines={}", info.name, info.base_name, so(&info.sex), so(&info.tarot), so(&info.soul), info.lines.len());
        for (k, c) in contexts.iter().enumerate() {
            let crystal: Option<Box<dyn Fn(&str) -> bool + Send + Sync>> = if c.2 { Some(Box::new(|who: &str| who.contains("Crystal"))) } else { None };
            let parser = TooltipParser::new(c.0, c.1.clone(), crystal);
            let (fx, sets) = parser.effects(info, &infos);
            w!("  C{k} fx={} sets={}", fx.len(), sets.len());
            for e in &fx {
                w!("    E {}", fx_line(e));
            }
            for st in &sets {
                w!("    S pieces=[{}] fx={}", st.pieces.join("|"), st.effects.len());
                for e in &st.effects {
                    w!("      E {}", fx_line(e));
                }
            }
            if k == 0 {
                all.extend(fx.iter().cloned());
            }
            for e in &fx {
                if !label_seen.contains(&e.label) {
                    label_seen.push(e.label.clone());
                }
            }
        }
    }
    // static helpers
    let mut bodies: Vec<String> = Vec::new();
    for t in &texts {
        for l in t.split('\n') {
            let l = l.trim().to_string();
            if l.contains(',') && !bodies.contains(&l) {
                bodies.push(l);
            }
        }
    }
    for b in ["Close-range, Long-range, Spell damage suffered -10%", "Fire, Ice, Elec Affinity +5%", "A, B, C", "STR +5, MAG", "x, y 3, z +4", "  ,  , ", "a+5, b, c, d, e +3.5%"] {
        {
            bodies.push(b.to_string());
        }
    }
    for b in &bodies {
        w!("SP [{b}] -> {}", TooltipParser::split_parts(b).join(" ;; "));
    }
    let mut pieces: Vec<String> = ["Test Set Item♂", "Other Set Piece♀", "Spiral Code License♂", "Spiral Code License", "Crystal Thing", "crystal thing", "Class Ring♀", "Class Ring♂", ""].iter().map(|x| x.to_string()).collect();
    for t in &infos {
        pieces.push(t.base_name.clone());
    }
    let mut seen: Vec<String> = Vec::new();
    for p in pieces {
        if seen.contains(&p) {
            continue;
        }
        w!("PW [{p}] -> {}", if TooltipParser::piece_worn(&p, &infos) { "True" } else { "False" });
        seen.push(p);
    }
    for who in ["Hecatonchires's Crystal", "Hecatonchires'", "Ogami's Crystal", "The Fool", "Orthrus's Crystal", "Nothing Here", "Some Soul", "crystal", ""] {
        w!("WW [{who}] -> {}", if TooltipParser::worn_with(who, &infos) { "True" } else { "False" });
    }

    // Bonuses
    let mut label_list: Vec<String> = label_seen.clone();
    for l in [
        "Critical", "critical", "Final Crit", "Final Critical Rate Correction", "LB Chance", "Limit Break Power", "Pursuit Damage", "TA Rate", "Tech Attack Power",
        "Close-Range Damage", "Long-Range Damage Dealt", "Magic Damage", "Close-Range Damage Taken", "Spell Damage Suffered", "Spin Skill Boost", "Attack Damage",
        "Rush Power", "Rapid Skill Power", "Shot Boost", "Cooldown", "Movement Speed", "Limit Break Cap", "LB Damage Cap", "LB Upper Limit",
        "Fire Affinity", "Slash Boost", "Gun Affinity", "Gun resist", "Handgun Affinity Cap", "Strike affinity", "Strike-affinity", "Blunt Resistance", "Elec resist", "Electric Affinity",
        "Words of Power Boost", "Word affinity", "Heal affinity", "Healing Boost", "Magic Force affinity", "MagicForce affinity", "Curse Boost", "Death Affinity", "Mystic affinity", "Nothing affinity",
        "STR", "Strength", "magic", "VIT", "Vitality", "INT", "SPD", "LUCK", "CLS-R", "Close-Range", "LNGR", "Long-Range", "Spell", "Support", "P.Def", "Physical Defense", "M.Def", "Magic Defense",
        "Max HP", "HP Max", "Max MP", "All parameters", "EXP Earned", "XP Gain", "Experience Points Earned", "Expertise earned", "Expertise Points earned", "Incantation", "Cast time", "CHANT_TIME",
        "HP regen", "HP Regeneration", "MP regen", "MP Regeneration", "HP/MP Regeneration", "Critical defense", "  padded label  ", "über", "ßtest", "", "x",
    ] {
        label_list.push(l.to_string());
    }
    let mut seen: Vec<String> = Vec::new();
    for l in &label_list {
        if seen.contains(l) {
            continue;
        }
        seen.push(l.clone());
        let (n, gr) = bonuses::canonical(l, &labels);
        w!("CAN [{l}] -> {n} | {gr}");
    }
    for (stat, _) in bonuses::STAT_MAP {
        w!("PAT {stat} {}", bonuses::pattern(stat));
    }
    let mut synth = all.clone();
    synth.push(eff("Critical", Some(2.5), "digitalized", "Ring", "item"));
    synth.push(eff("Fire Affinity", Some(3.0), "always", "Cape", "item"));
    synth.push(eff("fire-affinity", Some(4.0), "digitalized", "Boots", "item"));
    synth.push(Effect { pct: true, ..eff("Ice Boost", Some(1.25), "always", "Hat", "item") });
    synth.push(Effect { pct: true, ..eff("Cooldown", Some(-3.333), "always", "Gem", "mod") });
    synth.push(Effect { pct: true, target: "partner".into(), ..eff("Cooldown", Some(5.0), "always", "Partner gem", "mod") });
    synth.push(eff("Slash   Affinity", Some(7.0), "always", "", "soul"));
    synth.push(eff("Cooldown", None, "always", "Null value", ""));
    synth.push(eff("Cooldown", Some(9.0), "inactive", "Inactive", ""));
    for (k, v) in bonuses::demon_summary(&synth) {
        w!("DS {k} = {}", g(v));
    }
    for r in bonuses::combat_summary(&synth) {
        w!("CS {} | {} | {} | {}", r.stat, g(r.always), g(r.digi), r.sources);
    }
    for v in [0.0, 1.0, -1.0, 0.5, -0.456, 12.345, -0.004, 1e-6, 1234567.891, f64::NAN, -0.0] {
        w!("FMT {} -> {} {}", g(v), calc_fmt::n(v), calc_fmt::signed(v));
    }

    // CharacterHpMp
    for lv in [0, 1, 50, 95, 100, 999] {
        for st in [0, 1, 33, 96, 235, 1000] {
            let (h, m) = hp::formula(lv, st, st + 5, 100 - st / 3, st * 2);
            w!("F lv{lv} st{st} -> {h} {m}");
        }
    }
    for flat in [-5000.0, -1.5, 0.0, 326.7, 1e12, -1e12] {
        for p1 in [0.0, -10.0, -50.0, 12.5, -100.0, -150.0, 33.3] {
            for p2 in [0.0, -10.0, 7.7, -100.0] {
                w!("ADJ {} {} {} -> {} {} {} {}", g(flat), g(p1), g(p2), hp::adjust(1000, flat, p1, p2), hp::adjust(0, flat, p1, p2), hp::adjust(i32::MAX, flat, p1, p2), hp::adjust(-5, flat, p1, p2));
            }
        }
    }
    for lv in [1, 95] {
        for p in [0.0, -50.0] {
            let (h, m) = hp::compute(lv, 96, 71, 64, 154, 326.0, 139.0, p, p, 1630, 371);
            w!("COMPUTE lv{lv} p{} -> {h} {m}", g(p));
        }
    }
    let cal = hp::calibrate(95, 96, 71, 64, 154, 702, 291, 0.0, 0.0);
    w!("CAL {} {} {} {}", cal.flat_hp(), cal.flat_mp(), cal.formula_hp, cal.formula_mp);
    let mut cal2 = hp::calibrate(95, 96, 71, 64, 154, 702, 291, 0.0, -50.0);
    cal2.now_pct_mp = Some(-20.0);
    let opt = |o: Option<f64>| o.map(g).unwrap_or_default();
    w!("CAL2 {} {} {} {}", cal2.flat_hp(), cal2.flat_mp(), opt(cal2.now_pct_hp), opt(cal2.now_pct_mp));
    for c in [&cal, &cal2] {
        let (h, m) = hp::digitalized(c, 139, 124, 101, 112, 1630, 371, 0, 0);
        let (h2, m2) = hp::digitalized(c, 0, 0, 0, 0, 0, 0, 100, -50);
        w!("DIGI {h} {m} {h2} {m2}");
    }
    let mut dfx = synth.clone();
    dfx.push(eff("Strength", Some(12.9), "digitalized", "", ""));
    dfx.push(eff("STR", Some(-2.5), "always", "", ""));
    dfx.push(Effect { cond_note: "stats of demon".into(), ..eff("MAG", Some(3.0), "always", "", "") });
    dfx.push(Effect { pct: true, ..eff("Vitality", Some(4.0), "always", "", "") });
    dfx.push(eff("Intelligence", Some(1.0), "always", "", ""));
    dfx.push(eff("INT", Some(1.0), "situational", "", ""));
    dfx.push(eff("Max HP", Some(100.9), "always", "", ""));
    dfx.push(Effect { cond_note: "x".into(), ..eff("Max HP", Some(50.0), "digitalized", "", "") });
    dfx.push(eff("Max MP", Some(25.5), "always", "", ""));
    dfx.push(Effect { cond_note: "statsfoo".into(), ..eff("Max MP", Some(-3.5), "always", "", "") });
    dfx.push(eff("Max HP", None, "always", "", ""));
    let only = hp::digitalize_only_stats(&dfx);
    let mut ks: Vec<_> = only.iter().collect();
    ks.sort();
    w!("ONLYSTATS {}", ks.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(","));
    let (fh, fm) = hp::digitalize_only_flat(&dfx);
    w!("ONLYFLAT {fh} {fm}");
    let mut stats: BTreeMap<i32, i32> = BTreeMap::new();
    for (i, v) in [139, 124, 101, 112].into_iter().enumerate() {
        stats.insert(i as i32, v);
    }
    stats.insert(6, 1630);
    stats.insert(7, 371);
    let (rh, rm) = hp::digitalized_with_result(&cal, &stats, &dfx, stats[&6], stats[&7], 5, -5);
    w!("DIGIRES {rh} {rm} hp={} mp={}", stats[&6], stats[&7]);

    // JSON-reading parts
    let passives: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(in_dir.join("passives.json")).unwrap()).unwrap();
    let passives = passives.as_object().unwrap();
    let toggle_sets: Vec<Vec<i32>> = vec![vec![], vec![1], vec![1, 1, 2], vec![2, 3], vec![4, 5, 6, 7], vec![6, 6, 6, 6, 6]];
    for t in &toggle_sets {
        for stat in ["HP_MAX", "MP_MAX", "NOPE"] {
            w!("TOG [{}] {stat} -> {}", t.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","), g(hp::toggle_pct(passives, t, stat)));
        }
    }
    w!("TOGNULL {}", g(hp::toggle_pct(passives, &[], "MP_MAX")));
    let pairs = [96, 0, 71, 0, 64, 0, 154, 0, 10, 2, 5, 5];
    let cases: Vec<(Vec<i32>, Vec<i32>)> = vec![(vec![1], vec![6]), (vec![], vec![]), (vec![1, 6], vec![])];
    for (then, now) in &cases {
        let c = hp::calibrate_from_reading(passives, 95, &pairs, 702, 291, then, now);
        match c {
            None => w!("CALTOG null"),
            Some(c) => w!("CALTOG {} {} {} {} flat {} {}", c.pct_hp, c.pct_mp, opt(c.now_pct_hp), opt(c.now_pct_mp), c.flat_hp(), c.flat_mp()),
        }
    }
    w!("CALSHORT {}", if hp::calibrate_from_reading(passives, 95, &[1, 2, 3], 702, 291, &[], &[]).is_none() { "True" } else { "False" });
    w!("CALNOHP {}", if hp::calibrate_from_reading(passives, 95, &pairs, 0, 291, &[], &[]).is_none() { "True" } else { "False" });
    w!("CALNULLPAIRS {}", if hp::calibrate_from_reading(passives, 95, &[], 5, 291, &[], &[]).is_none() { "True" } else { "False" });
    let tmp = std::env::temp_dir().join(format!("w1-assets-rs-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let last = tmp.join("last-stats.json");
    w!("SAVED none {}", if hp::from_saved_stats(&last, passives, Some(&[1])).unwrap().0.is_none() { "True" } else { "False" });
    for f in ["last1.json", "last2.json", "last3.json", "last4.json"] {
        for now in [Some(vec![1]), None] {
            std::fs::copy(in_dir.join(f), &last).unwrap();
            let (c, at) = hp::from_saved_stats(&last, passives, now.as_deref()).unwrap();
            let now_s = match &now {
                None => "null".to_string(),
                Some(n) => n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
            };
            let body = match c {
                None => "null".to_string(),
                Some(c) => format!(
                    "{} {} {} {} {} {} {} {} {} {} {} {} {} flat {} {}",
                    c.level, c.str_, c.mag, c.vit, c.int, c.max_hp, c.max_mp, c.formula_hp, c.formula_mp, g(c.pct_hp), g(c.pct_mp), opt(c.now_pct_hp), opt(c.now_pct_mp), c.flat_hp(), c.flat_mp()
                ),
            };
            w!("SAVED {f} now={now_s} -> {body} at={}", at.unwrap_or_else(|| "~".into()));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = s("");
    out
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let out = dump(&PathBuf::from(&a[0]), &PathBuf::from(&a[1]), &PathBuf::from(&a[2]));
    std::fs::write(&a[3], out.as_bytes()).unwrap();
    println!("{} chars", out.len());
}
