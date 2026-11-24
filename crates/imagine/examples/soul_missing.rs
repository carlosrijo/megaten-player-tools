//! Lists the soul fusion (and tarot) crystals whose effect data the reader cannot resolve: a tokusei the server data, the
//! website file and the client's own text all fail to give (the reader would show "Unknown effect" and drop the line).
//!
//! Usage: soul_missing [website content folder] [tokusei xml folder]
//! (no arguments = the bundled copies, the public server's tokusei; pass the data folder's paths for NewMoon's)

use imagine::data::game_data::GameData;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let gd = GameData::with_paths(args.first().cloned(), args.get(1).cloned());
    for tarot in [false, true] {
        let (mut total, mut missing) = (0, Vec::new());
        let ids: Vec<i32> = {
            let mut v = Vec::new();
            for l in imagine::data::game_data::reference_lines("enchants-game.tsv").unwrap().unwrap() {
                let f: Vec<&str> = l.split('\t').collect();
                if f.len() > 3 && f[3] == if tarot { "tarot" } else { "soul" } {
                    if let Ok(id) = f[0].parse::<i32>() {
                        v.push(id);
                    }
                }
            }
            v
        };
        for id in ids {
            let Some(side) = gd.enchant_effect(id, tarot).unwrap() else { continue };
            let mut all = side.tokusei.clone();
            for c in &side.conditions {
                all.extend(c.3.iter().copied());
            }
            if all.is_empty() {
                continue;
            }
            total += 1;
            let bad: Vec<i32> = all.iter().copied().filter(|t| gd.tokusei_full(*t).unwrap().is_none()).collect();
            if !bad.is_empty() {
                missing.push(format!("{id} {} crystal item {} tokusei {:?} unresolved {:?}", side.name.clone().unwrap_or_default(), side.crystal_item, all, bad));
            }
        }
        println!("{}: {} with effects, {} not resolved", if tarot { "tarot" } else { "soul" }, total, missing.len());
        for m in &missing {
            println!("  {m}");
        }
    }
}
