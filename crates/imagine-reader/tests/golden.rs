//! Compares the database builders with the C# reader's output for the same inputs (needs the decoded game tables and
//! the C# outputs, so it only runs on request: `cargo test -p imagine-reader --test golden -- --ignored`).
//!
//! Set these environment variables:
//!   IMAGINE_GOLDEN_CS        folder with the C# output: demondb\demon-db.json, skilldb\skill-db.json, dungeons\dungeons.json
//!   IMAGINE_DECODED          smt-decoded\server (DevilData.xml, ZoneData.xml, SkillData.xml, CHelpData.xml, ...)
//!   IMAGINE_DUNGEONS_INPUT   folder with zone.xml, DevilData.xml, citem.tsv, CEventMessageData.xml, CItemData.xml
//!   IMAGINE_STORE            the server's datastore folder
//!   IMAGINE_CLIENT           the game folder (for the pictures)
//! The JSON must match byte for byte, except the generation date. The Demon Database is the exception: the Rust builder
//! lists the variants the C# one never had, so it is checked demon by demon (`demons_match_csharp_and_add_variants`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn var(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

fn mask_date(text: &str) -> String {
    match text.find("\"generatedAt\":\"") {
        Some(i) => {
            let from = i + "\"generatedAt\":\"".len();
            format!("{}<date>{}", &text[..from], &text[from + 10..])
        }
        None => text.to_string(),
    }
}

fn run(args: &[&std::ffi::OsStr]) {
    let out = Command::new(env!("CARGO_BIN_EXE_imagine-reader")).args(args).output().expect("run imagine-reader");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

/// The C# reader listed only the compendium's demons. The Rust one lists those unchanged (same stats, skills, features,
/// tarot and soul; where they can be contracted may move to a variant that spawns as its own type, and a demon can gain a
/// Mitama form from another id of the same demon) and adds the variants players can own, each marked `variant`.
fn demons_match_csharp_and_add_variants(golden: &Path, made: &Path) {
    let cs: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(golden).expect("C# output")).expect("C# json");
    let rs: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(made).expect("Rust output")).expect("Rust json");
    let rust: std::collections::HashMap<i64, &serde_json::Value> = rs["demons"].as_array().unwrap().iter().map(|d| (d["id"].as_i64().unwrap(), d)).collect();
    let moved = ["contract", "enemy", "mt", "get", "variant", "variantOf"];
    for c in cs["demons"].as_array().unwrap() {
        let r = rust.get(&c["id"].as_i64().unwrap()).unwrap_or_else(|| panic!("{} is missing from the Rust output", c["name"]));
        for (k, v) in c.as_object().unwrap() {
            if !moved.contains(&k.as_str()) {
                assert!(&r[k] == v, "{} differs in {k}", c["name"]);
            }
        }
    }
    let in_cs: std::collections::HashSet<i64> = cs["demons"].as_array().unwrap().iter().map(|d| d["id"].as_i64().unwrap()).collect();
    for (id, d) in &rust {
        assert!(in_cs.contains(id) || d["variant"] == true, "{} is new but not a variant", d["name"]);
    }
    assert!(rust.values().any(|d| d["name"] == "Open-minded Ishtar"), "Open-minded Ishtar is listed");
    for (k, v) in cs["skills"].as_object().unwrap() {
        assert!(rs["skills"][k] == *v, "skill {k} differs");
    }
}

fn same(golden: &Path, made: &Path) {
    let a = std::fs::read_to_string(golden).expect("C# output");
    let b = std::fs::read_to_string(made).expect("Rust output");
    assert!(mask_date(&a) == mask_date(&b), "{} differs from {}", made.display(), golden.display());
}

#[test]
#[ignore = "needs the game's decoded tables and the C# outputs"]
fn databases_match_the_csharp_output() {
    let golden = var("IMAGINE_GOLDEN_CS");
    let tmp = std::env::temp_dir().join(format!("imagine-golden-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let (decoded, store, client, dun) = (var("IMAGINE_DECODED"), var("IMAGINE_STORE"), var("IMAGINE_CLIENT"), var("IMAGINE_DUNGEONS_INPUT"));
    let o = |name: &str| tmp.join(name);
    run(&["skilldb".as_ref(), "--decoded".as_ref(), decoded.as_os_str(), "--client".as_ref(), client.as_os_str(), "--out".as_ref(), o("skill-db.json").as_os_str()]);
    same(&golden.join("skilldb").join("skill-db.json"), &o("skill-db.json"));
    run(&["demondb".as_ref(), "--decoded".as_ref(), decoded.as_os_str(), "--store".as_ref(), store.as_os_str(), "--client".as_ref(), client.as_os_str(), "--out".as_ref(), o("demon-db.json").as_os_str()]);
    demons_match_csharp_and_add_variants(&golden.join("demondb").join("demon-db.json"), &o("demon-db.json"));
    run(&["dungeons".as_ref(), "--decoded".as_ref(), dun.as_os_str(), "--datastore".as_ref(), store.as_os_str(), "--client".as_ref(), client.as_os_str(), "--out".as_ref(), o("dungeons.json").as_os_str()]);
    same(&golden.join("dungeons").join("dungeons.json"), &o("dungeons.json"));
    let _ = std::fs::remove_dir_all(&tmp);
}
