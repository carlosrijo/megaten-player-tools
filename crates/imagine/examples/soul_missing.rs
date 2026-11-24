//! Lists the soul fusion and tarot crystals that have a tokusei the server data, the website file and the client's per-tokusei
//! text cannot give, and what the reader now makes of each from the crystal's own description (`calc::enchant_text`):
//!   full     every clause of the text became an effect (or a counted-nowhere note is absent)
//!   partial  some clauses became effects, others stayed text-only notes ("from the game's text")
//!   none     nothing readable: the "Unknown effect" rows stay (a saved tooltip may still stand in for them)
//!
//! Usage: soul_missing [website content folder] [tokusei xml folder] [--ranks]
//! (no folders = the bundled copies, the public server's tokusei; pass the data folder's paths for NewMoon's.
//! The character is NEUTRAL, level 95, and its expertise ranks unknown unless --ranks gives it every expertise at Class 9)

use std::sync::Arc;

use imagine::calc::effect::Effect;
use imagine::calc::item_effects::ItemEffects;
use imagine::data::game_data::{GameData, reference_lines};
use indexmap::IndexMap;

fn show(e: &Effect) -> String {
    let v = match e.value {
        Some(v) => format!("{}{}", if v > 0.0 { "+" } else { "" }, v) + if e.pct { "%" } else { "" },
        None => "(note)".to_string(),
    };
    format!("{} {v} [{}, {}] {}", e.label, e.cond, e.target, e.cond_note)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ranks_known = args.iter().any(|a| a == "--ranks");
    let paths: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    let data = Arc::new(GameData::with_paths(paths.first().cloned(), paths.get(1).cloned()));
    let mut ranks = IndexMap::new();
    if ranks_known {
        for id in data.expertises_client().unwrap().keys() {
            ranks.insert(*id, 90);
        }
    }
    let fx = ItemEffects::new(Arc::clone(&data), "NEUTRAL", 95, ranks);
    for tarot in [false, true] {
        let side_name = if tarot { "tarot" } else { "soul" };
        let mut ids = Vec::new();
        for l in reference_lines("enchants-game.tsv").unwrap().unwrap() {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() > 3 && f[3] == side_name {
                if let Ok(id) = f[0].parse::<i32>() {
                    ids.push(id);
                }
            }
        }
        let (mut with_effects, mut gaps, mut full, mut partial, mut none) = (0, 0, Vec::new(), Vec::new(), Vec::new());
        for id in ids {
            let Some(side) = data.enchant_effect(id, tarot).unwrap() else { continue };
            let mut all = side.tokusei.clone();
            for c in &side.conditions {
                all.extend(c.3.iter().copied());
            }
            if all.is_empty() {
                continue;
            }
            with_effects += 1;
            let missing: Vec<i32> = all.iter().copied().filter(|t| data.tokusei_full(*t).unwrap().is_none()).collect();
            if missing.is_empty() {
                continue;
            }
            gaps += 1;
            let list = fx.enchant(id, tarot, &side.name.clone().unwrap_or_default()).unwrap();
            let values = list.iter().filter(|e| e.value.is_some()).count();
            let notes = list.iter().filter(|e| e.value.is_none() && !e.missing_data).count();
            let unknown = list.iter().filter(|e| e.missing_data).count();
            let entry = (format!("{id} {}{}  (tokusei without data: {missing:?})", side.name.clone().unwrap_or_default().trim(), if tarot { "" } else { " (soul)" }), list);
            if values == 0 {
                none.push(entry);
            } else if notes == 0 && unknown == 0 {
                full.push(entry);
            } else {
                partial.push(entry);
            }
        }
        println!("{side_name}: {with_effects} crystals with effects, {gaps} with a tokusei that has no data: {} full, {} partial, {} none", full.len(), partial.len(), none.len());
        for (name, set) in [("full", &full), ("partial", &partial), ("none", &none)] {
            for (n, list) in set.iter() {
                println!("  [{name}] {n}");
                for e in list {
                    println!("      {}", show(e));
                }
            }
        }
    }
}
