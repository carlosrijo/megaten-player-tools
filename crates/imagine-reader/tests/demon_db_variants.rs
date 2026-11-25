//! The Demon Database page's data (app\ui\demon-db.json, committed) lists the variants players can own, not only the
//! compendium's base demons. Runs on the committed file: no game, no data folder.

use std::collections::HashSet;

use serde_json::Value;

fn db() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("app").join("ui").join("demon-db.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("app\\ui\\demon-db.json")).expect("demon-db.json parses")
}

fn named<'a>(db: &'a Value, name: &str) -> Vec<&'a Value> {
    db["demons"].as_array().expect("demons").iter().filter(|d| d["name"] == name).collect()
}

#[test]
fn open_minded_ishtar_is_listed_with_its_own_stats_and_how_to_get_it() {
    let db = db();
    let ishtar = named(&db, "Open-minded Ishtar");
    assert_eq!(ishtar.len(), 1, "one entry for Open-minded Ishtar");
    let x = ishtar[0];
    assert_eq!(x["level"], 30);
    assert_eq!(x["race"], "Earth Mother");
    assert_eq!(x["variantOf"], "Ishtar");
    // its own stats, not Ishtar's (Lv 96): Open-minded Ishtar starts at Lv 30 with 358 HP
    assert_eq!(x["stats"][6], 358);
    // it comes out of the Open-Minded summoning program
    let get = x["get"].as_array().expect("how to get it");
    let item = get.iter().find(|g| g["kind"] == "Exchange item").expect("an exchange item");
    assert!(item["text"].as_str().unwrap().contains("Summon PG: Open-Minded (♀)"), "{item}");
    assert!(!item["from"].as_array().expect("from").is_empty(), "the data names where the item is exchanged");
    // the plain demon is still there, once
    assert_eq!(named(&db, "Ishtar").len(), 1);
}

#[test]
fn every_demon_is_listed_once_and_every_variant_says_how_to_get_it() {
    let db = db();
    let demons = db["demons"].as_array().expect("demons");
    let ids: HashSet<i64> = demons.iter().map(|d| d["id"].as_i64().expect("id")).collect();
    assert_eq!(ids.len(), demons.len(), "no demon twice");
    let variants: Vec<&Value> = demons.iter().filter(|d| d["variant"] == true).collect();
    assert!(variants.len() > 500, "the variants are listed ({})", variants.len());
    for v in variants {
        let how = v["get"].as_array().unwrap().len() + v["contract"].as_array().unwrap().len();
        assert!(how > 0, "{} has no way to get it", v["name"]);
    }
    // no internal codes in what the page shows
    let text = serde_json::to_string(&db["demons"]).unwrap();
    assert!(!text.contains("ITEM_SUBCATEGORY") && !text.contains("EXCHANGE_DEMON"));
}

#[test]
fn enemy_copies_stay_folded() {
    let db = db();
    // Ishtar has six boss and dungeon copies under the same name: still one entry
    assert_eq!(named(&db, "Ishtar").len(), 1);
    let demons = db["demons"].as_array().unwrap();
    assert!(demons.len() < 1200, "not every one of the 5,411 DevilData records ({})", demons.len());
}
