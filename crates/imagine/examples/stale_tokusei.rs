//! Lists the skills whose listed tokusei the game's own skill text contradicts (see `build::skill_description`).
//!
//! Usage: stale_tokusei [website content folder] [tokusei xml folder]   (defaults: the data folder's config.json is not read; no
//! arguments = the bundled copies, which are the public server's tokusei)
//! Also lists the stat ids of every tokusei in the folder that have no game wording (they would be shown as raw codes).

use imagine::build::skill_description::stale_effects;
use imagine::data::game_data::GameData;
use imagine::data::json::{self, At};
use imagine::data::stat_labels;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let gd = GameData::with_paths(args.first().cloned(), args.get(1).cloned());
    // every skill an expertise rank gives
    let mut ids: Vec<i32> = Vec::new();
    for l in gd.rank_skills_client().unwrap().values() {
        for (_, s) in l {
            for x in s {
                if !ids.contains(x) {
                    ids.push(*x);
                }
            }
        }
    }
    ids.sort();
    let passives = gd.passives().unwrap();
    let names = gd.skill_names().unwrap();
    let (mut skills, mut with_tokusei, mut with_text, mut stale_skills, mut stale_effects_n) = (0, 0, 0, 0, 0);
    for id in &ids {
        let Some(p) = passives.get(&id.to_string()).filter(|v| !v.is_null()) else { continue };
        skills += 1;
        let toks: Vec<i32> = json::arr(p.at("tokuseiIds")).map(|t| json::int(Some(t))).filter(|t| *t != 0).collect();
        if toks.is_empty() {
            continue;
        }
        with_tokusei += 1;
        let Some(text) = gd.skill_text(*id).unwrap() else { continue };
        with_text += 1;
        let mut found: Vec<String> = Vec::new();
        for t in toks {
            if let Some(def) = gd.tokusei_full(t).unwrap() {
                for s in stale_effects(&def, &text) {
                    found.push(format!("tokusei {t} {} {}: {}", s.id, s.value, s.why));
                }
            }
        }
        if !found.is_empty() {
            stale_skills += 1;
            stale_effects_n += found.len();
            println!("{} {}", id, names.get(id).cloned().flatten().unwrap_or_default());
            for f in found {
                println!("    {f}");
            }
        }
    }
    println!("\nexpertise skills: {skills}, with a tokusei: {with_tokusei}, with a tokusei and a description: {with_text}, contradicted: {stale_skills} skills / {stale_effects_n} effects");

    // the same over every passive and switch skill the server data has (learned skills, Digitalization Levels, gear traits...)
    let (mut all, mut all_stale) = (0, Vec::new());
    for (k, p) in passives.iter().filter(|(_, p)| !p.is_null()) {
        let id: i32 = k.parse().unwrap_or(0);
        let Some(text) = gd.skill_text(id).unwrap() else { continue };
        let toks: Vec<i32> = json::arr(p.at("tokuseiIds")).map(|t| json::int(Some(t))).filter(|t| *t != 0).collect();
        if toks.is_empty() {
            continue;
        }
        all += 1;
        let why: Vec<String> = toks.iter().filter_map(|t| gd.tokusei_full(*t).unwrap()).flat_map(|d| stale_effects(&d, &text)).map(|s| s.why).collect();
        if !why.is_empty() && !ids.contains(&id) {
            all_stale.push(format!("{id} {}", names.get(&id).cloned().flatten().unwrap_or_default()));
            if std::env::var_os("SHOW_ALL").is_some() {
                println!("  {id} {} :: {}\n      TEXT {}", names.get(&id).cloned().flatten().unwrap_or_default(), why.join(" | "), text.replace('\n', " / "));
            }
        }
    }
    println!("\nall passive/switch skills with a tokusei and a description: {all}; contradicted outside the expertise ranks: {} {}", all_stale.len(), if std::env::var_os("SHOW_ALL").is_some() { String::new() } else { all_stale.join(", ") });

    // stat ids with no wording, over every skill's correct table and every tokusei the folder holds
    let mut missing: std::collections::BTreeMap<String, usize> = Default::default();
    for p in passives.values() {
        for c in json::arr(p.at("correctTbl")) {
            if let Some(id) = json::str(c.at("id")).filter(|i| !stat_labels::has(i)) {
                *missing.entry(format!("skill table {id}")).or_default() += 1;
            }
        }
    }
    if let Some(dir) = args.get(1) {
        let mut seen = std::collections::HashSet::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            for m in regex::Regex::new(r#"<member name="ID">(\d+)</member>"#).unwrap().captures_iter(&text) {
                let t: i32 = m[1].parse().unwrap();
                if !seen.insert(t) {
                    continue;
                }
                if let Some(def) = gd.tokusei_full(t).unwrap() {
                    for v in def.correct.iter().chain(def.aspects.iter()) {
                        if let Some(id) = v.id.as_deref().filter(|i| !stat_labels::has(i)) {
                            *missing.entry(format!("tokusei {id}")).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    println!("\nids with no game wording:");
    for (k, v) in missing {
        println!("  {k}: {v}");
    }
}
