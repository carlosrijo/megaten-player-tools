//! T20: a crystal fused into a piece is named by the game's crystal, never by its internal id ("enchant 3105").
//! The names come from the client's enchant table (reference\enchants-game.tsv: each enchant's crystal item) and the client's
//! item names (reference\items-game.tsv), both compiled into the library, so none of these tests needs the website folder.

use std::collections::HashSet;
use std::sync::Arc;

use imagine::build::dashboard::{Dashboard, Part};
use imagine::context::Context;
use imagine::data::game_data::GameData;
use imagine::data::store::Store;
use serde_json::Value;

/// A game data with no website folder: the enchant list of the website (the old name source) is empty, as it is for
/// any crystal the website does not list.
fn bare() -> GameData {
    GameData::with_paths(None, None)
}

/// Ogami's Crystal (item 54015) is enchant 3105: "Ogami's" as a tarot, "of the Warmonger" as a soul fusion. The website's enchant
/// list does not have it, so a piece whose tooltip was never read (Mercedes, a weapon) showed "enchant 3105".
#[test]
fn a_soul_of_ogamis_crystal_is_named_by_its_crystal() {
    let d = bare();
    assert!(d.enchants().unwrap().get(&3105).is_none() || d.enchants().unwrap()[&3105].crystal.is_none(), "this machine's website list names it: the test needs it not to");
    assert_eq!(d.crystal_name(3105, Some(false)).unwrap().as_deref(), Some("Ogami's Crystal"));
    assert_eq!(d.crystal_name(3105, Some(true)).unwrap().as_deref(), Some("Ogami's Crystal"));
    assert_eq!(d.crystal_label(3105, false).unwrap(), "Ogami's Crystal");
    assert_eq!(d.crystal_label(3105, true).unwrap(), "Ogami's Crystal");
    // in a fusion set's list of crystals the trailing "Crystal" goes
    assert_eq!(d.enchant_name(3105).unwrap(), "Ogami's");
    // with the website folder too (this machine's data\config.json), the name is the same
    assert_eq!(GameData::new().crystal_label(3105, false).unwrap(), "Ogami's Crystal");
}

/// Mercedes (item 2057, a Long-Range weapon) carrying soul 3105, as the player's gear row is built when the piece has a
/// record but no saved tooltip: the soul's name comes from the id.
#[test]
fn mercedes_with_the_warmongers_soul_names_the_crystal() {
    let d = GameData::new();
    assert_eq!(d.item_name(2057).unwrap().as_deref(), Some("Mercedes"));
    let soul = 3105;
    let shown = d.crystal_label(soul, false).unwrap();
    assert_eq!(shown, "Ogami's Crystal");
    assert!(!shown.contains("enchant") && !shown.chars().any(|c| c.is_ascii_digit()), "{shown}");
}

/// A crystal no table names reads as plain words, not as its number.
#[test]
fn a_crystal_nobody_names_gets_plain_words() {
    let d = bare();
    assert_eq!(d.crystal_name(987_654, Some(false)).unwrap(), None);
    assert_eq!(d.crystal_label(987_654, false).unwrap(), "a soul fusion");
    assert_eq!(d.crystal_label(987_654, true).unwrap(), "a tarot");
    assert_eq!(d.crystal_label(-5, false).unwrap(), "a soul fusion");
    assert_eq!(d.enchant_name(987_654).unwrap(), "a crystal");
    for s in [d.crystal_label(987_654, false).unwrap(), d.enchant_name(-5).unwrap()] {
        assert!(!s.contains("enchant") && !s.chars().any(|c| c.is_ascii_digit()), "{s}");
    }
}

/// Over every soul and tarot the client's enchant table has: how many would show an id (the old rule: the website's list
/// names it) and how many do now. Every one of them now has a name.
#[test]
fn every_soul_and_tarot_resolves_to_a_crystal_name() {
    let d = bare();
    let web = GameData::new();
    let sides = d.enchant_sides().unwrap();
    assert!(sides.len() > 1500, "the client's enchant table is compiled in: {} sides", sides.len());
    let (mut before, mut after, mut before_ids, mut after_ids) = (0, 0, HashSet::new(), HashSet::new());
    let mut renamed: Vec<String> = Vec::new();
    let have_web = !web.enchants().unwrap().is_empty();
    // BABEL's English for the crystals the client names only in Japanese (reference\item-names-en.tsv, third column): it
    // wins over the website's name by choice, so those are expected to read differently from the website's
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/item-names-en.tsv");
    let translations: HashSet<String> = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(2))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut translated = 0;
    for &(id, tarot) in sides.keys() {
        // before: only the website's list named a crystal (and the gear row used it only when the tooltip was missing)
        let old = web.enchants().unwrap().get(&id).and_then(|e| e.crystal.clone()).filter(|c| !c.trim().is_empty());
        if old.is_none() {
            before += 1;
            before_ids.insert(id);
        }
        let now = d.crystal_name(id, Some(tarot)).unwrap();
        if now.is_none() {
            after += 1;
            after_ids.insert(id);
        }
        // a name the website had is the same crystal now (the client spells a few differently: "Xuan-Wu's" / "Xuan Wu's")
        if let (Some(old), Some(now)) = (&old, &now) {
            let letters = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
            if letters(old) != letters(now) {
                if translations.contains(now) {
                    translated += 1;
                } else {
                    renamed.push(format!("{id}: {old} -> {now}"));
                }
            }
        }
    }
    // the client's name wins; it differs from the website's for a handful only (spelling: "Suiki" / "Suiki's Crystal", "Sword X" / "X of Sword")
    eprintln!("names the client spells differently from the website: {renamed:?}; read from BABEL's English instead: {translated}");
    assert!(renamed.len() <= 10, "the client's crystals differ from the website's: {renamed:#?}");
    eprintln!(
        "enchant sides {} ({} ids): website list on this machine = {have_web}; falling back to an id before: {before} sides / {} ids; after: {after} sides / {} ids",
        sides.len(),
        sides.keys().map(|k| k.0).collect::<HashSet<_>>().len(),
        before_ids.len(),
        after_ids.len()
    );
    // what is left has no name anywhere: its crystal item is not in the client's item list (placeholder tarots that do
    // "Nothing", the "@Transfiguring" souls that only upgrade another weapon), so it reads in plain words
    for &(id, tarot) in sides.keys().filter(|k| after_ids.contains(&k.0)) {
        let item = sides[&(id, tarot)].crystal_item;
        assert!(item == 0 || d.item_name(item).unwrap().is_none(), "enchant {id}: its crystal item {item} has a name but was not used");
    }
    assert!(after_ids.len() <= 20 && after < before, "{after} sides still without a name (before: {before}): {after_ids:?}");
    // the plain-words fallback is never an id
    for &(id, tarot) in sides.keys() {
        let n = d.crystal_label(id, tarot).unwrap();
        assert!(!n.starts_with("enchant "), "{id}: {n}");
        if after_ids.contains(&id) {
            assert!(n == "a tarot" || n == "a soul fusion", "{id}: {n}");
        }
    }
}

/// The bonus list names a set by its name alone: its internal number ("(set 1792)") is only the key that links the bonus to the
/// worn set row on the page.
#[test]
fn a_sets_bonus_part_has_no_set_number_in_its_name() {
    let d = Arc::new(GameData::new());
    let dir = std::env::temp_dir().join(format!("imagine-t20-sets-{}", std::process::id()));
    let ctx = Arc::new(Context::new(Store::new(&dir), Arc::clone(&d)));
    let dash = Dashboard::new(ctx);
    let part = |from: &str| Part {
        kind: "gear".into(),
        label: "Long-Range".into(),
        value: 10.0,
        target: "self".into(),
        cond: "always".into(),
        from: from.into(),
        ..Default::default()
    };
    let rows = dash.bonus_rows(vec![part("Set: Yashamaru Armor (Red) (set 1792)"), part("Fusion set: Ogami's + Satan's (fusion set 4)"), part("Mercedes (soul)")], None).unwrap();
    let row = rows.iter().find(|r| r.get("bonus").and_then(Value::as_str) == Some("LNG-R")).expect("a Long-Range row");
    let parts = row["parts"].as_array().unwrap();
    let by = |kind: &str| parts.iter().find(|p| p["kind"] == kind).unwrap_or_else(|| panic!("{kind}: {parts:#?}"));
    assert_eq!((by("set")["source"].as_str(), by("set")["set_key"].as_str()), (Some("Yashamaru Armor (Red)"), Some("set:1792")));
    assert_eq!((by("fusion set")["source"].as_str(), by("fusion set")["set_key"].as_str()), (Some("Ogami's + Satan's"), Some("fusion set:4")));
    let soul = by("soul");
    assert_eq!(soul["source"], "Mercedes");
    assert!(soul.get("set_key").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
