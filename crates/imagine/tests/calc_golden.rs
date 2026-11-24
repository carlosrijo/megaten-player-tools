//! The calc ports (Effect, TooltipParser, Bonuses, CharacterHpMp) against what the C# code printed for the same inputs.
//! `golden/calc-expected.txt` was written by the C# reader's own classes (a throwaway harness compiled from the real
//! sources) over `golden/corpus.txt` (invented tooltip texts that exercise every branch of the parser), the JSON inputs
//! in `golden/`, and `reference/stat-labels-game.tsv`; the Rust dump (`examples/calcdump.rs`) must match it line by line.
//! The same dump over the real game tooltips (53 texts, ~3000 lines) also matched; those texts are not kept here.
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../examples/calcdump.rs"]
mod calcdump;

#[test]
fn calc_matches_the_csharp_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden = root.join("tests").join("golden");
    let labels = root.join("..").join("..").join("reference").join("stat-labels-game.tsv");
    let got = calcdump::dump(&golden.join("corpus.txt"), &golden, &labels);
    let want = std::fs::read_to_string(golden.join("calc-expected.txt")).expect("golden file").replace("\r\n", "\n");
    let got = got.replace("\r\n", "\n");
    if got != want {
        for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
            assert_eq!(g, w, "first difference at line {}", i + 1);
        }
        assert_eq!(got.lines().count(), want.lines().count(), "line counts differ");
    }
}
