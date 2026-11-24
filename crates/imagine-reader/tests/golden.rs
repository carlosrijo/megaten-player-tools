//! Compares the database builders with the C# reader's output for the same inputs (needs the decoded game tables and
//! the C# outputs, so it only runs on request: `cargo test -p imagine-reader --test golden -- --ignored`).
//!
//! Set these environment variables:
//!   IMAGINE_GOLDEN_CS        folder with the C# output: demondb\demon-db.json, skilldb\skill-db.json, dungeons\dungeons.json
//!   IMAGINE_DECODED          smt-decoded\server (DevilData.xml, ZoneData.xml, SkillData.xml, CHelpData.xml, ...)
//!   IMAGINE_DUNGEONS_INPUT   folder with zone.xml, DevilData.xml, citem.tsv, CEventMessageData.xml, CItemData.xml
//!   IMAGINE_STORE            the server's datastore folder
//!   IMAGINE_CLIENT           the game folder (for the pictures)
//! The JSON must match byte for byte, except the generation date.

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
    same(&golden.join("demondb").join("demon-db.json"), &o("demon-db.json"));
    run(&["dungeons".as_ref(), "--decoded".as_ref(), dun.as_os_str(), "--datastore".as_ref(), store.as_os_str(), "--client".as_ref(), client.as_os_str(), "--out".as_ref(), o("dungeons.json").as_os_str()]);
    same(&golden.join("dungeons").join("dungeons.json"), &o("dungeons.json"));
    let _ = std::fs::remove_dir_all(&tmp);
}
