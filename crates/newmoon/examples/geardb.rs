//! The `geardb` command of the C# CLI, to compare the Gear Database builder with it:
//!   geardb --decoded <dir> [--sheets sheet_gear.tsv] [--out gear-db.json] [--ids-out file] [--game-sources file]
use std::sync::Arc;

use newmoon::build::gear_db;
use newmoon::data::game_data::GameData;

fn main() -> newmoon::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |name: &str| args.iter().position(|a| a == &format!("--{name}")).and_then(|i| args.get(i + 1)).cloned();
    let decoded = opt("decoded").ok_or_else(|| newmoon::user_error("geardb needs --decoded <folder with the decoded client tables>."))?;
    let out = opt("out").unwrap_or_else(|| r"app\ui\gear-db.json".to_string());
    gear_db::build(Arc::new(GameData::new()), &decoded, opt("sheets").as_deref(), &out, opt("ids-out").as_deref(), opt("game-sources").as_deref())
}
