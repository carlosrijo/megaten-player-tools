//! Pins the data layer (`data::*`, `build::csv`) to what the C# code produced. The expected values below were taken from a run
//! of the C# sources (a harness that dumped them) and the Rust dump (`examples/data_dump.rs`) matched it line for line, over the
//! reference tables, the website/server data, the fixture copy of a real data folder and the CSV files.
//! Tests that need the website or server folders (outside this repo) are in the example; these only use the repo's own
//! reference tables and temp folders.

use std::path::PathBuf;

use indexmap::IndexMap;
use imagine::build::csv::CsvTable;
use imagine::data::game_data::{GameData, reference_file};
use imagine::data::stat_labels;
use imagine::data::store::{CachedTip, Store};
use imagine::game::{SetRecord, WornItem};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("imagine-data-layer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn no_ts(s: &str) -> String {
    let re = regex::Regex::new(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}").unwrap();
    re.replace_all(s, "<ts>").into_owned()
}

#[test]
fn game_data_is_thread_safe() {
    fn needs<T: Send + Sync>() {}
    needs::<GameData>();
    needs::<imagine::data::demon_tables::DemonTables>();
}

#[test]
fn reference_tables_are_available_with_or_without_the_checkout() {
    // the embedded copy answers even when nothing is found on disk
    for f in ["devils-game.tsv", "items-game.tsv", "stat-labels-game.tsv", "tokusei-missing.tsv"] {
        let r = reference_file(f).unwrap_or_else(|| panic!("{f} missing"));
        assert!(!r.lines().unwrap().is_empty());
    }
    assert!(reference_file("nope.tsv").is_none());
}

#[test]
fn demon_tables_like_csharp() {
    let gd = GameData::with_paths(None, None);
    let t = gd.demons().unwrap();
    assert!(t.loaded());
    assert_eq!(
        (t.devils.len(), t.growth.len(), t.growth_names.len(), t.reunion_bonus.len(), t.mitama_sets.len(), t.union_bonus.len()),
        (5411, 256, 256, 127, 48, 64)
    );
    assert_eq!((t.assists.len(), t.skills.len(), t.levels.len(), t.force_stack_tokusei.len(), t.equipment_skill.len()), (648, 7159, 43, 2924, 302));
    let satan = &t.devils[&1];
    assert_eq!((satan.name.as_str(), satan.race.as_str(), satan.race_id, satan.base_level), ("Satan", "GOD_SPIRIT", 14, 95));
    assert_eq!(satan.correct.len(), 126);
    assert_eq!(&satan.correct[..8], &[5, 5, 5, 5, 5, 5, 232, 30]);
    // the race id is DevilData's race (what the server matches assists against), also for demons the website's
    // devils.json has no raceId for (NewMoon-only ones): Gold Demon Goldfish is a Fairy (54) with Fairy's 18 assists
    let gold = &t.devils[&12069];
    assert_eq!((gold.name.as_str(), gold.race.as_str(), gold.race_id, t.assists_of_race(gold.race_id).len()), ("Gold Demon Goldfish", "FAIRY", 54, 18));
    assert_eq!((t.devils[&758].race_id, t.devils[&9910949].race_id), (102, 105)); // Mezuki BRUTE, Belphegor FALLEN_ANGEL
    let no_race: Vec<i32> = t.devils.values().filter(|d| d.race != "NONE" && d.race_id == 0).map(|d| d.id).collect();
    assert_eq!(no_race, Vec::<i32>::new());
    let m = &t.mitama_sets[0];
    assert_eq!((m.requirements.clone(), m.bonus.clone(), m.description.as_str()), (vec![4, 0, 0, 0], vec![0, 20, 2, 10, 0, 0, 0, 0], "Strength+20 Vitality+10"));
    assert_eq!(t.digitalize_races(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 51, 52, 53, 54, 55, 56, 58, 59, 60, 61, 62, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113]);
    assert_eq!((t.race_name(1), t.race_name(14), t.race_name(16), t.race_name(101)), ("SERAPHIM".into(), "GOD_SPIRIT".into(), "race 16".into(), "FOUL".into()));
    assert_eq!(t.levels[&1][1].next_xp, 379500);
    let levels: IndexMap<i32, i32> = [(1, 7), (2, 15), (3, 3), (99999, 4)].into_iter().collect();
    let points: IndexMap<i32, i32> = [(1, 1234), (3, 5)].into_iter().collect();
    let rl = t.race_levels(Some(&levels), Some(&points));
    assert_eq!((rl[0].race.as_str(), rl[0].level, rl[0].extend_skills.clone()), ("SERAPHIM", 7, vec![60136, 60137, 60138]));
    assert_eq!(rl[0].percent_to_next, Some(0.0021679969219362987));
    assert_eq!((rl[1].level, rl[1].percent_to_next, rl[1].extend_skills.clone()), (15, None, vec![60112, 60113, 60114, 60115]));
    assert_eq!(rl[2].percent_to_next, Some(0.00018823883743694));
}

#[test]
fn digitalize_unlocks_specials_and_race_names() {
    let gd = GameData::with_paths(None, None);
    let t = gd.demons().unwrap();
    // GuardianUnlockData: 25 races start locked; Raptor needs Wild Bird 4 and Evil Dragon 4, Tyrant Vile 7, Reaper 8, Foul 8
    assert_eq!(t.unlocks.len(), 25);
    assert_eq!(t.unlocks[&8], vec![(11, 4), (109, 4)]);
    assert_eq!(t.unlocks[&112], vec![(4, 7), (51, 8), (101, 8)]);
    assert!(!t.unlocks.contains_key(&2) && !t.unlocks.contains_key(&56)); // Entity and Fiend: open from the start
    // GuardianSpecialData: David only needs Fiend 1 (or Brute 6); Gabriel Seraphim 6 or Avian 8
    assert_eq!(t.specials.len(), 73);
    assert_eq!(t.specials[&615], vec![(56, 1), (102, 6)]);
    assert_eq!(t.specials[&24], vec![(1, 6), (5, 8)]);
    // the client's names (CGuardianAssistData): MAJIN is "Fiend" in game; one Divine line is Japanese and loses the vote
    assert_eq!(t.game_race_name(56).as_deref(), Some("Fiend"));
    assert_eq!(t.game_race_name(9).as_deref(), Some("Divine"));
    assert_eq!(t.game_race_name(11).as_deref(), Some("Wild Bird"));
    assert_eq!(t.assists[&1].effect_text(), "Duration time +1 minute");
}

#[test]
fn digitalize_planner_data() {
    use imagine::build::digi_planner;
    let gd = GameData::with_paths(None, None);
    let v = digi_planner::planner_json(&gd).unwrap();
    let races = v["races"].as_array().unwrap();
    assert_eq!(races.len(), 36); // the 37 digitalize races but Gaian, which has no points to earn
    let race = |id: i64| races.iter().find(|r| r["id"] == id).unwrap();
    // tiers: the points from level 1 to 10 are 1x to 5x the cheapest race's (70,200,100)
    assert_eq!((race(9)["tier"].as_i64(), race(8)["tier"].as_i64(), race(2)["tier"].as_i64(), race(112)["tier"].as_i64()), (Some(1), Some(2), Some(4), Some(5)));
    assert_eq!(race(9)["xp"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).sum::<i64>(), 70_200_100);
    assert_eq!((race(56)["name"].as_str(), race(56)["side"].as_str()), (Some("Fiend"), Some("NEUTRAL")));
    // Fiend DLv 2: Intelligence +55 for you; DLv 5: an affinity cap for the demon ("DE【...】" name unwrapped)
    assert_eq!(race(56)["extend"][0], serde_json::json!([2, 60142, "Base Stat Boost", "Intelligence +55", "you"]));
    assert_eq!(race(56)["extend"][2][2], "Affinity Upper Limit Raise");
    assert_eq!(race(56)["extend"][2][4], "demon");
    assert_eq!(race(56)["assists"][0][1][0][1], "Duration time +1 minute");
    assert_eq!(race(1)["demons"][0], serde_json::json!([26, "Uriel", 1, 80]));
    assert!(v["special"].as_array().unwrap().iter().any(|s| s[0] == 615 && s[2] == 56));
}

#[test]
fn expertise_tables_like_csharp() {
    let gd = GameData::with_paths(None, None);
    let e = gd.expertises_client().unwrap();
    assert_eq!(e.len(), 59);
    assert_eq!((e[&0].name.as_str(), e[&0].max_class, e[&0].is_chain), ("Attack", 10, false));
    assert_eq!((e[&1].name.as_str(), e[&1].max_class), ("Spin", 7));
    let r = gd.rank_skills_client().unwrap();
    assert_eq!(r.len(), 40);
    assert_eq!(r[&1], vec![(0, vec![5404]), (10, vec![5405, 5406]), (25, vec![5850]), (30, vec![5427]), (45, vec![5851]), (50, vec![5424]), (60, vec![5425]), (70, vec![5426])]);
}

#[test]
fn stat_labels_like_csharp() {
    assert_eq!(stat_labels::of("BOOST_SLASH"), Some("Slash Affinity"));
    assert_eq!(stat_labels::name("RATE_CLSR", 0), "Close-Range Damage");
    assert_eq!(stat_labels::name("RATE_CLSR_TAKEN", 0), "Close-range Damage Taken");
    assert_eq!(stat_labels::name("RATE_SUPPORT", 0), "Support Magic damage");
    assert_eq!(stat_labels::name("RATE_SUPPORT_TAKEN", 0), "Support Magic damage Taken");
    assert_eq!(stat_labels::name("MOVE1", 0), "Movement Speed");
    assert_eq!(stat_labels::name("UNKNOWN_STAT", 0), "UNKNOWN_STAT");
    assert_eq!(stat_labels::name_opt(None, 0), None);
    assert!(stat_labels::has("SUMMON_SPEED") && !stat_labels::has("UNKNOWN_STAT"));
    assert_eq!(stat_labels::element("SLASH").as_deref(), Some("Slash"));
    assert_eq!(stat_labels::element_by_number(4).as_deref(), Some("Blunt"));
    assert_eq!(stat_labels::element_by_number(-1), None);
    assert_eq!(stat_labels::element_by_number(23), None);
    assert_eq!(stat_labels::ELEMENT_IDS.len(), 23);
}

#[test]
fn crystal_keys_like_csharp() {
    assert_eq!(GameData::crystal_key(Some("Cerberus' Crystal")), "cerberus");
    assert_eq!(GameData::crystal_key(Some("Lucifuge Crystal")), "lucifuge");
    assert_eq!(GameData::crystal_key(Some("Soul Stone: Procyon")), "soul stone: procyon");
    assert_eq!(GameData::crystal_key(Some("Odin's   Crystal ")), "odin");
    assert_eq!(GameData::crystal_key(Some("  ")), "");
    assert_eq!(GameData::crystal_key(None), "");
}

#[test]
fn item_records_pack_and_parse_like_csharp() {
    let good = "1:2:3:4:5:1,2,3,4,5:1,2,3:4:5";
    let w = Store::parse_item(Some(good)).unwrap();
    assert_eq!(w, WornItem { id: 1, tarot: 2, soul: 3, basic: 4, special: 5, mods: vec![1, 2, 3, 4, 5], fuse: vec![1, 2, 3], durability: 4, max_durability: 5 });
    assert_eq!(Store::pack_item(Some(&w)), good);
    assert_eq!(Store::pack_item(None), "-");
    for bad in [Some("1:2"), Some("1:2:3:4:5:1,2,3,4:1,2,3:4:5"), Some("a:2:3:4:5:1,2,3,4,5:1,2,3:4:5"), Some("1:2:3:4:5:1,2,3,4,5::4:5"), None, Some("")] {
        assert!(Store::parse_item(bad).is_none(), "{bad:?}");
    }
    assert_eq!(Store::pack_item(Store::parse_item(Some("+1:-2:3:4:5:1,2,3,4,5:1,2,3:4:5")).as_ref()), "1:-2:3:4:5:1,2,3,4,5:1,2,3:4:5");
    assert_eq!(Store::pack_item(Store::parse_item(Some("1: 2 :3:4:5:1,2,3,4,5:1,2,3:4:-1")).as_ref()), "1:2:3:4:5:1,2,3,4,5:1,2,3:4:-1");
    let fifteen = format!("{} -", vec![good; 14].join(" "));
    let items = Store::parse_items(Some(&fifteen)).unwrap();
    assert_eq!(items.len(), 15);
    assert!(items[14].is_none());
    assert_eq!(Store::pack_items(Some(&items)), fifteen);
    assert!(Store::parse_items(Some(&format!("{good} {good}"))).is_none());
    assert!(Store::parse_items(Some(&fifteen.replacen(good, "x", 1))).is_none());
    assert_eq!(Store::parse_items(Some(&format!("  {} ", vec!["-"; 15].join("  ")))).map(|v| v.len()), Some(15));
    assert_eq!(Store::pack_items(None), "");
}

#[test]
fn store_files_like_csharp() {
    let dir = temp("store");
    let st = Store::new(&dir);
    assert_eq!(st.alignment().unwrap(), "NEUTRAL");
    assert!(st.active_toggles().unwrap().is_empty());
    assert_eq!(st.counts().unwrap(), (None, None, None));
    assert!(st.load_set_table().unwrap().is_none());
    assert_eq!(st.tooltips_level().unwrap(), 0);

    let note = "Updated <ts>. alignment: for 'If Player is LAW/NEUTRAL/CHAOS' gear conditions; toggles: switch skills that are on; quest_count / compendium_count (from the game once its Quest Completion Bonuses / Demonic Compendium window has been opened, else set by hand) and compendium_v2 (from the game's valuables, else by hand): for the quest and compendium bonuses.";
    st.save_counts(Some(3), Some(4), Some(false), false, false).unwrap();
    let text = no_ts(&std::fs::read_to_string(st.profile_path()).unwrap());
    assert_eq!(
        text,
        format!("{{\r\n  \"quest_count_source\": \"set by hand\",\r\n  \"compendium_count_source\": \"set by hand\",\r\n  \"quest_count\": 3,\r\n  \"compendium_count\": 4,\r\n  \"compendium_v2\": false,\r\n  \"note\": \"{note}\"\r\n}}")
    );
    // the note moves to the end of the other keys it is written after; existing keys keep their place
    st.save_toggles([1, 2, 1]).unwrap();
    let text = no_ts(&std::fs::read_to_string(st.profile_path()).unwrap());
    assert!(text.ends_with(&format!("\"note\": \"{note}\",\r\n  \"alignment\": \"NEUTRAL\",\r\n  \"toggles\": [\r\n    1,\r\n    2\r\n  ]\r\n}}")));
    assert_eq!(st.active_toggles().unwrap(), vec![1, 2]);
    assert_eq!(st.counts().unwrap(), (Some(3), Some(4), Some(false)));
    assert_eq!(st.quest_count_source().unwrap().as_deref(), Some("set by hand"));
    st.save_alignment(Some("lawful")).unwrap();
    assert_eq!(st.alignment().unwrap(), "LAWFUL");

    let dir2 = temp("store2");
    let s3 = Store::new(&dir2);
    s3.save_toggles([3]).unwrap();
    assert_eq!(
        no_ts(&std::fs::read_to_string(s3.profile_path()).unwrap()),
        format!("{{\r\n  \"alignment\": null,\r\n  \"toggles\": [\r\n    3\r\n  ],\r\n  \"note\": \"{note}\"\r\n}}")
    );

    let mut tips: IndexMap<i32, CachedTip> = IndexMap::new();
    tips.insert(30, CachedTip { text: Some("line1\nline2 あ <&>".into()), seen_at: Some("2026-11-22T19:18:17".into()) });
    tips.insert(7, CachedTip { text: None, seen_at: None });
    tips.insert(100000, CachedTip { text: Some(String::new()), seen_at: Some("x\"y".into()) });
    st.save_tooltip_cache(&tips).unwrap();
    assert_eq!(
        std::fs::read_to_string(st.tooltip_cache_path()).unwrap(),
        "{\r\n  \"30\": {\r\n    \"text\": \"line1\\nline2 あ <&>\",\r\n    \"seen_at\": \"2026-11-22T19:18:17\"\r\n  },\r\n  \"7\": {\r\n    \"text\": null,\r\n    \"seen_at\": null\r\n  },\r\n  \"100000\": {\r\n    \"text\": \"\",\r\n    \"seen_at\": \"x\\\"y\"\r\n  }\r\n}"
    );
    assert_eq!(st.load_tooltip_cache().unwrap(), tips);

    let sets = vec![
        SetRecord { id: 1, pieces: vec![0, 0, 5, 6], tokusei: vec![10, 20] },
        SetRecord { id: 2, pieces: vec![], tokusei: vec![] },
        SetRecord { id: 3, pieces: vec![1], tokusei: vec![0, 0] },
    ];
    st.save_set_table(&sets).unwrap();
    assert_eq!(
        std::fs::read_to_string(st.set_table_path()).unwrap(),
        "[\r\n  {\r\n    \"id\": 1,\r\n    \"pieces\": [\r\n      0,\r\n      0,\r\n      5,\r\n      6\r\n    ],\r\n    \"tok\": [\r\n      10,\r\n      20\r\n    ]\r\n  },\r\n  {\r\n    \"id\": 2,\r\n    \"pieces\": [],\r\n    \"tok\": []\r\n  },\r\n  {\r\n    \"id\": 3,\r\n    \"pieces\": [\r\n      1\r\n    ],\r\n    \"tok\": [\r\n      0,\r\n      0\r\n    ]\r\n  }\r\n]"
    );
    let back = st.load_set_table().unwrap().unwrap();
    assert_eq!(back[2].tokusei, Vec::<i32>::new()); // zero tokusei are dropped on reading
    assert_eq!(back[0], sets[0]);

    // tooltips.txt as an older/odd writer left it
    std::fs::write(st.tooltips_path(), "level 7x\r\n===== record 0 (id 5)\r\nabc  \r\n\r\n===== record 1 (id -2) (tarot 3 soul 4)\r\nx\r\n===== record 2 (id 9) (item 1:2:3:4:5:1,2,3,4,5:1,2,3:4:5)\r\nlast").unwrap();
    assert_eq!(st.tooltips_level().unwrap(), 0);
    let tips = st.load_tooltips().unwrap();
    assert_eq!(tips.iter().map(|t| (t.index, t.id, t.tarot, t.soul, t.text.as_str())).collect::<Vec<_>>(), vec![(0, 5, None, None, "abc"), (1, -2, Some(3), Some(4), "x"), (2, 9, None, None, "last")]);
    assert_eq!(tips[2].detail.as_ref().map(|d| d.id), Some(1));
    std::fs::write(st.tooltips_path(), "level 12\n===== record 0 (id 5)\nLF only\n").unwrap();
    assert_eq!(st.tooltips_level().unwrap(), 12);
    assert_eq!(st.load_tooltips().unwrap()[0].text, "LF only");

    // written and read back
    let texts: Vec<String> = (0..15).map(|i| if i % 2 == 0 { format!("item {i}\nline 2") } else { String::new() }).collect();
    let ids: Vec<i32> = (0..15).map(|i| 100 + i).collect();
    st.save_tooltips(&ids, &texts, None, None, None, 95).unwrap();
    let text = std::fs::read_to_string(st.tooltips_path()).unwrap();
    assert!(text.starts_with("level 95\r\n===== record 0 (id 100)\r\nitem 0\r\nline 2\r\n===== record 1 (id 101)\r\n===== record 2 (id 102)\r\n"));
    let recs = st.load_tooltips().unwrap();
    assert_eq!(recs.len(), 15);
    assert_eq!(st.tooltips_level().unwrap(), 95);
    let (t2, i2, tarot, soul) = Store::unpack(&recs);
    assert_eq!((t2[0].as_str(), i2[14]), ("item 0\nline 2".replace('\n', "\r\n").as_str(), 114));
    assert!(tarot.is_none() && soul.is_none());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn snapshot_keeps_item_records_while_the_same_items_are_worn() {
    let dir = temp("keepdet");
    let st = Store::new(&dir);
    let ids: Vec<i32> = (0..15).map(|i| if i == 3 { 0 } else { 100 + i }).collect();
    let tarot: Vec<i32> = (0..15).map(|i| i * 2).collect();
    let soul: Vec<i32> = (0..15).map(|i| i * 3).collect();
    let det: Vec<Option<WornItem>> = (0..15)
        .map(|k| (ids[k] > 0).then(|| WornItem { id: ids[k], tarot: tarot[k], soul: soul[k], basic: 7, special: 8, mods: vec![1, 2, 3, 4, 5], fuse: vec![0, 0, 0], durability: 9, max_durability: 10 }))
        .collect();
    let texts = vec![String::new(); 15];
    st.save_tooltips(&ids, &texts, Some(&tarot), Some(&soul), Some(&det), 95).unwrap();
    let prev = st.load_tooltips().unwrap();

    // same items: kept, with or without the enchants read now
    assert_eq!(Store::keep_details(&prev, &ids, Some(&tarot), Some(&soul)), Some(det.clone()));
    assert_eq!(Store::keep_details(&prev, &ids, None, None), Some(det.clone()));
    // another item in a slot, or a re-enchanted one: not kept
    let mut other = ids.clone();
    other[5] = 999;
    assert_eq!(Store::keep_details(&prev, &other, None, None), None);
    let mut t2 = tarot.clone();
    t2[0] = 77;
    assert_eq!(Store::keep_details(&prev, &ids, Some(&t2), Some(&soul)), None);
    // an earlier snapshot without records has nothing to keep
    st.save_tooltips(&ids, &texts, None, None, None, 95).unwrap();
    assert_eq!(Store::keep_details(&st.load_tooltips().unwrap(), &ids, None, None), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expertise_files_like_csharp() {
    let dir = temp("exp");
    let st = Store::new(&dir);
    std::fs::write(
        st.expertise_ranks_path(),
        "{\"ranks\":[{\"id\":4,\"name\":\"Rapid\",\"class\":10,\"rank\":3},{\"id\":5.7,\"name\":\"rapid\",\"class\":2,\"rank\":1},{\"name\":\"\"},{\"id\":\"9\",\"name\":\"Z\"},null]}",
    )
    .unwrap();
    // names are case-insensitive keys (upper-cased): the later "rapid" takes the first one's place with its own class
    assert_eq!(st.expertise_classes().unwrap().into_iter().collect::<Vec<_>>(), vec![("RAPID".to_string(), 2), ("Z".to_string(), 0)]);
    assert_eq!(st.expertise_ranks().unwrap().into_iter().collect::<Vec<_>>(), vec![(4, 103), (5, 21), (9, 0)]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Some expertise ranks list several skills and let the character learn only some of them (Pursuit class 1: Strength,
/// Magic, Intelligence or Speed; class 7: CLS-R, LNG-R or Spell). Only the learned ones count.
#[test]
fn expertise_pick_one_ranks_count_only_the_pick() {
    let gd = GameData::with_paths(None, None);
    // the table says how many may be learned: one of four at Pursuit 1-0 and one of three at 7-0; all four of the
    // Pursuit 3-0 Mantras and the single skill of 2-0 have no choice
    assert_eq!(gd.rank_pick_count(35, 10).unwrap(), Some(1));
    assert_eq!(gd.rank_pick_count(35, 70).unwrap(), Some(1));
    assert_eq!(gd.rank_pick_count(35, 30).unwrap(), None);
    assert_eq!(gd.rank_pick_count(35, 20).unwrap(), None);

    let pursuit = |dir: &std::path::Path, learned: Option<&str>| {
        let st = Store::new(dir);
        std::fs::write(
            st.expertise_ranks_path(),
            format!("{{\"ranks\":[{{\"id\":35,\"name\":\"Pursuit\",\"class\":7,\"rank\":0}}]{}}}", learned.map(|l| format!(",\"learned_skills\":[{l}]")).unwrap_or_default()),
        )
        .unwrap();
        let ctx = std::sync::Arc::new(imagine::context::Context::new(Store::new(dir), GameData::with_paths(None, None)));
        let exp = imagine::build::expertise::Expertise::new(ctx);
        let ranks = vec![serde_json::json!({ "id": 35, "name": "Pursuit", "class": 7, "rank": 0 })];
        let rows = exp.rows(None, &ranks, false).unwrap();
        let counted: Vec<i32> = rows.iter().filter(|r| r.get("counted").and_then(|c| c.as_bool()).unwrap_or(true) && r.get("skill_id").is_some()).map(|r| r["skill_id"].as_i64().unwrap() as i32).collect();
        let prompt = rows.iter().filter(|r| r["row"].as_str().is_some_and(|id| id.contains("-pick-"))).filter_map(|r| r.get("effects").and_then(|e| e.as_str()).map(String::from)).collect::<Vec<_>>();
        (counted, prompt)
    };
    let dir = temp("pick");
    // the player's choice: Speed (class 1) and LNG-R (class 7); everything else of Pursuit is learned outright
    let (counted, prompt) = pursuit(&dir, Some("5060,5062"));
    assert!(counted.contains(&5060) && counted.contains(&5062), "{counted:?}");
    for other in [5057, 5058, 5059, 5061, 5063] {
        assert!(!counted.contains(&other), "{other} counted: {counted:?}");
    }
    assert!(counted.contains(&5073) && counted.contains(&5064), "ranks with no choice count as before: {counted:?}");
    assert!(prompt.is_empty(), "{prompt:?}");
    // nothing learned at a pick rank: nothing counts there, and the page says to learn one
    let (counted, prompt) = pursuit(&dir, Some("5073"));
    assert!(![5057, 5058, 5059, 5060, 5061, 5062, 5063].iter().any(|s| counted.contains(s)), "{counted:?}");
    assert_eq!(prompt.len(), 2, "{prompt:?}");
    assert!(prompt.iter().all(|p| p.starts_with("Nothing learned here yet.")), "{prompt:?}");
    // never read the learned skills: no guess either, and it says which skills it needs read
    let (counted, prompt) = pursuit(&dir, None);
    assert!(![5057, 5058, 5059, 5060, 5061, 5062, 5063].iter().any(|s| counted.contains(s)), "{counted:?}");
    assert_eq!(prompt.len(), 2, "{prompt:?}");
    assert!(prompt.iter().all(|p| p.contains("isn't known yet")), "{prompt:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn csv_like_csharp() {
    let dir = temp("csv");
    let p = dir.join("edge.csv");
    let cases: [(&str, &str, &str, usize, &str); 6] = [
        ("plain", "a,b\r\n1,2\r\n", "a|b", 1, "\"a\",\"b\"\r\n\"1\",\"2\"\r\n"),
        ("quotes", "a,b\r\n\"x,\"\"y\"\"\",z\r\n\r\n1\r\n", "a|b", 2, "\"a\",\"b\"\r\n\"x,\"\"y\"\"\",\"z\"\r\n\"1\",\"\"\r\n"),
        ("lone-cr", "a,b\r1,2\r3,4", "a|b", 2, "\"a\",\"b\"\r\n\"1\",\"2\"\r\n\"3\",\"4\"\r\n"),
        ("unterminated", "a,b\n\"1,2\n3", "a|b", 1, "\"a\",\"b\"\r\n\"1,2\n3\",\"\"\r\n"),
        ("empty", "", "", 0, "\r\n"),
        ("dup-col", "a,a\n1,2\n", "a|a", 1, "\"a\",\"a\"\r\n\"2\",\"2\"\r\n"),
    ];
    for (label, text, cols, rows, saved) in cases {
        std::fs::write(&p, text).unwrap();
        let t = CsvTable::load(&p).unwrap();
        assert_eq!(t.columns.join("|"), cols, "{label}");
        assert_eq!(t.rows.len(), rows, "{label}");
        let out = dir.join("out.csv");
        t.save(&out).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), saved, "{label}");
    }
    std::fs::write(&p, "\u{feff}\"a\",\"b\"\n\"1\",\"2\"").unwrap();
    assert_eq!(CsvTable::load(&p).unwrap().columns, vec!["a", "b"]);
    assert!(CsvTable::load(dir.join("none.csv")).unwrap().columns.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toggles_source_tells_when_they_were_read() {
    let dir = temp("toggles-source");
    let st = Store::new(&dir);
    assert_eq!(st.toggles_source().unwrap(), None); // no profile: never read
    st.save_toggles_from_game([5, 6]).unwrap();
    let from_game = st.toggles_source().unwrap().unwrap();
    assert!(from_game.starts_with("game, ") && from_game.len() == "game, ".len() + 16, "{from_game}");
    assert_eq!(st.active_toggles().unwrap(), vec![5, 6]);
    // an alignment change is not a reading of the toggles
    st.save_alignment(Some("law")).unwrap();
    assert_eq!(st.toggles_source().unwrap().as_deref(), Some(from_game.as_str()));
    // toggles set by hand drop the stamp; the profile's own save time stands in (no newer than that)
    st.save_toggles([7]).unwrap();
    let saved = st.toggles_source().unwrap().unwrap();
    assert!(saved.starts_with("saved, ") && saved.len() == "saved, ".len() + 16, "{saved}");
    // a profile from before the stamp: its note's time
    let old = Store::new(temp("toggles-source-old"));
    std::fs::create_dir_all(old.profile_path().parent().unwrap()).unwrap();
    std::fs::write(old.profile_path(), "{\"note\": \"Updated 2026-11-24 14:00. x\", \"alignment\": \"NEUTRAL\", \"toggles\": [1]}").unwrap();
    assert_eq!(old.toggles_source().unwrap().as_deref(), Some("saved, 2026-11-24 14:00"));
}

/// The game words these stats only for the summoner, so the label generator never saw them: they must not reach the page as codes.
#[test]
fn stat_codes_the_client_words_for_the_summoner_are_not_shown_raw() {
    assert_eq!(stat_labels::name("RATE_HEAL_TAKEN", 0), "Curative Received"); // "Summoner Curative Received +20%", tokusei 130138
    assert_eq!(stat_labels::name("RES_STATUS", 0), "Ailment Resistance"); // "Summoner Ailment Resistance +10", tokusei 130123
    assert_eq!(stat_labels::name("RATE_HEAL", 0), "Curative Boost");
    // "Ailment Resistance +10" is RES_STATUS -10 in the table
    assert_eq!(stat_labels::displayed("RES_STATUS", 0, -50.0), ("Ailment Resistance".to_string(), 50.0));
    assert_eq!(stat_labels::displayed("RATE_CLSR", 1, 15.0), ("Close-Range Damage".to_string(), 15.0));
    // an EQUIPPED_WEAPON_TYPE value is an item class, which the client names
    use imagine::data::game_data::item_class_name;
    assert_eq!(
        [23, 24, 25, 26].map(item_class_name),
        [Some("Martial-Arts"), Some("One-Handed Sword"), Some("Stabbing Weapon"), Some("Primitive Weapon")]
    );
    assert_eq!(item_class_name(9999), None);
}

/// Lord of Victory (5549): the game's text says "Close-Range Damage +15%, FCC +5%, Healing received -25%". The skill's table has the
/// first and last, and the tokusei it lists (520061) is the public server's old "+25% Close-Range Damage, Critical +100 with a
/// melee weapon", which the page must not show. Run through both ways a tokusei reaches the page: the website's tokusei-effects.json
/// (the reader's usual) and the server XML alone (the portable app's bundled copy).
#[test]
fn lord_of_victory_shows_what_the_game_says() {
    let rows_for = |content: Option<String>, name: &str, toggle_on: bool| {
        let dir = temp(name);
        let st = Store::new(&dir);
        std::fs::write(st.expertise_ranks_path(), "{\"ranks\":[{\"id\":57,\"name\":\"Lord\",\"class\":6,\"rank\":0}]}").unwrap();
        if toggle_on {
            std::fs::create_dir_all(st.profile_path().parent().unwrap()).unwrap();
            st.save_toggles([5549]).unwrap();
        }
        let ctx = std::sync::Arc::new(imagine::context::Context::new(Store::new(&dir), GameData::with_paths(content, None)));
        let exp = imagine::build::expertise::Expertise::new(ctx);
        let ranks = vec![serde_json::json!({ "id": 57, "name": "Lord", "class": 6, "rank": 0 })];
        let rows = exp.rows(None, &ranks, false).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        rows.into_iter().find(|r| r.get("skill_id").and_then(|s| s.as_i64()) == Some(5549)).expect("Lord of Victory is a row")
    };
    // the website's file, with 520061 as comp_hack has it (conditions on four weapon classes)
    let site = temp("lov-site");
    // a website folder must hold every file the reader asks for; empty ones stand in for those this test does not use
    for (dir, file, empty) in [
        ("wiki", "items.json", r#"{"items":[]}"#),
        ("wiki", "skills.json", r#"{"skills":[]}"#),
        ("wiki", "enchants.json", r#"{"enchants":{}}"#),
        ("armory", "expertises.json", r#"{"expertises":{}}"#),
        ("armory", "expertise-rank-skills.json", r#"{"expertises":{}}"#),
        ("armory", "passive-skills.json", r#"{"skills":{}}"#),
        ("armory", "enchant-sets.json", r#"{"sets":{}}"#),
        ("armory", "item-subcategories.json", r#"{"items":{}}"#),
        ("armory", "sitem-tokusei.json", r#"{"items":{}}"#),
        ("armory", "mod-effects.json", "{}"),
        ("armory", "devils.json", "{}"),
    ] {
        std::fs::create_dir_all(site.join(dir)).unwrap();
        std::fs::write(site.join(dir).join(file), empty).unwrap();
    }
    std::fs::write(
        site.join("armory").join("tokusei-effects.json"),
        r#"{"tokusei":{"520061":{"conditions":[{"type":"EQUIPPED_WEAPON_TYPE","value":23,"group":1},{"type":"EQUIPPED_WEAPON_TYPE","value":24,"group":1}],"correct":[{"id":"RATE_CLSR","type":1,"value":25},{"id":"CRITICAL","type":0,"value":100}]}}}"#,
    )
    .unwrap();
    let want = "Toggle skill, switched on: Close-Range Damage +15%; Curative Received -25%; Final Crit +5% (from the game's description)";
    for (label, content) in [("website", Some(site.to_string_lossy().into_owned())), ("xml", None)] {
        let row = rows_for(content, &format!("lov-{label}"), true);
        assert_eq!(row["effects"].as_str().unwrap(), want, "{label}");
        let num = |k: &str| row[k].as_f64().unwrap();
        assert_eq!((num("dmg_close"), num("final_crit"), num("crit")), (15.0, 5.0, 0.0), "{label}: the old +25% and Critical +100 are not counted");
        // the structured lines the page sums carry the same three, none of them marked as a code
        let fx: Vec<(String, f64)> = row["fx"].as_array().unwrap().iter().map(|f| (f["label"].as_str().unwrap().to_string(), f["value"].as_f64().unwrap())).collect();
        assert_eq!(fx, vec![("Close-Range Damage".to_string(), 15.0), ("Curative Received".to_string(), -25.0), ("Final Crit".to_string(), 5.0)], "{label}");
        // switched off: listed, nothing counted
        let off = rows_for(None, &format!("lov-{label}-off"), false);
        assert!(off["effects"].as_str().unwrap().starts_with("Toggle skill, switched off (not counted): "), "{label}");
        assert_eq!(off["dmg_close"].as_f64().unwrap() + off["final_crit"].as_f64().unwrap(), 0.0, "{label}");
    }
    let _ = std::fs::remove_dir_all(&site);
}

/// A tokusei the text does not contradict is left alone.
#[test]
fn a_tokusei_the_text_backs_is_not_flagged() {
    // Criminal Bullet's tokusei 510082 is
    // "+50% Long-Range Damage" against one demon family, and the text lists the same +50%
    let gd = GameData::with_paths(None, None);
    let def = gd.tokusei_full(510082).unwrap().expect("510082 is in the bundled server data");
    let text = gd.skill_text(5450).unwrap().expect("Criminal Bullet has a description");
    assert!(imagine::build::skill_description::stale_effects(&def, &text).is_empty(), "the text agrees with it");
}

/// Cataclysmic Mind (160611, Essence of Magic 9-3) says "When Radiant Hands is active, Spell Damage +500%". Radiant Hands (5485)
/// is a switch skill, so that line follows the switch the game reports: counted while it is on, listed and not counted while it
/// is off, and Radiant Hands' own row says the line needs it. Run on the public data (no sealed tokusei): the line is read from
/// the skill's description, as for any skill the server tables lack (GitHub issue 5).
#[test]
fn a_skill_that_needs_a_switch_follows_its_state() {
    let rows_for = |name: &str, switch_on: bool| {
        let dir = temp(name);
        let st = Store::new(&dir);
        std::fs::write(
            st.expertise_ranks_path(),
            "{\"ranks\":[{\"id\":43,\"name\":\"Essence of Magic\",\"class\":9,\"rank\":3},{\"id\":53,\"name\":\"Enhancement\",\"class\":7,\"rank\":5}],\"learned_skills\":[160611,5485]}",
        )
        .unwrap();
        if switch_on {
            std::fs::create_dir_all(st.profile_path().parent().unwrap()).unwrap();
            st.save_toggles([5485]).unwrap();
        }
        let ctx = std::sync::Arc::new(imagine::context::Context::new(Store::new(&dir), GameData::with_paths(None, None).without_sealed_data()));
        let exp = imagine::build::expertise::Expertise::new(ctx);
        let ranks = vec![
            serde_json::json!({ "id": 43, "name": "Essence of Magic", "class": 9, "rank": 3 }),
            serde_json::json!({ "id": 53, "name": "Enhancement", "class": 7, "rank": 5 }),
        ];
        let rows = exp.rows(None, &ranks, false).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let get = |id: i64| rows.iter().find(|r| r.get("skill_id").and_then(|s| s.as_i64()) == Some(id)).cloned().unwrap_or_else(|| panic!("skill {id} is a row"));
        (get(160611), get(5485))
    };
    let magic = |row: &imagine::data::json::Row| -> Vec<(f64, String, bool)> {
        row["fx"].as_array().unwrap().iter().filter(|f| f["label"] == "Magic Damage").map(|f| (f["value"].as_f64().unwrap(), f["cond"].as_str().unwrap().to_string(), f["toggle"].as_bool().unwrap_or(false))).collect()
    };
    // Radiant Hands on: Cataclysmic Mind's +500% counts (it cancels Radiant Hands' -500%) and is marked as the switch's
    let (cata, radiant) = rows_for("switch-on", true);
    assert!(cata["effects"].as_str().unwrap().contains("Magic Damage +500% (while Radiant Hands is on, from the game's description)"), "{}", cata["effects"]);
    assert_eq!(magic(&cata), vec![(500.0, "always".to_string(), true)]);
    assert_eq!(cata["dmg_spell"].as_f64().unwrap(), 500.0);
    assert_eq!(radiant["dmg_spell"].as_f64().unwrap(), -500.0);
    assert!(radiant["effects"].as_str().unwrap().ends_with("; Cataclysmic Mind: Magic Damage +500% needs this on"), "{}", radiant["effects"]);
    // Radiant Hands off: listed with the reason, nothing counted, and Radiant Hands still says what depends on it
    let (cata, radiant) = rows_for("switch-off", false);
    assert!(cata["effects"].as_str().unwrap().contains("Magic Damage +500% (only while Radiant Hands is on, and it is off now: not counted"), "{}", cata["effects"]);
    assert!(magic(&cata).is_empty());
    assert_eq!(cata["dmg_spell"].as_f64().unwrap(), 0.0);
    assert_eq!(radiant["dmg_spell"].as_f64().unwrap(), 0.0);
    assert!(radiant["effects"].as_str().unwrap().contains("Cataclysmic Mind: Magic Damage +500% needs this on"));
    assert!(radiant["effects"].as_str().unwrap().starts_with("Toggle skill, switched off (not counted): "));
}

#[test]
fn a_passive_with_no_stat_effect_is_still_passive() {
    // "Mitama Demon Enhancement - Iron Man" (5133, MITAMA_SET_BOOST): passive in SkillData, but its effect is a server
    // rule, so the passive effect table has no entry for it (the page called it an "Active skill").
    let gd = GameData::with_paths(None, None);
    assert!(gd.skill_is_passive(5133).unwrap());
    assert!(gd.passives().unwrap().get("5133").is_none_or(|p| p.is_null()));
    assert!(gd.skill_is_passive(5821).unwrap()); // Speed UP1: a passive with an effect
    assert!(!gd.skill_is_passive(7).unwrap()); // category 1: an active skill
}

/// The compendium mask's bits (shiftValues) become DCM item ids (entry ids) through reference\devil-book.tsv: Pixie's two
/// bits (570 and 749, seen on Rubito 2026-11-28) are one entry, 10570; Tajikarao's bit 148 is entry 11236 (not 10148).
#[test]
fn compendium_bits_to_dcm_ids() {
    use imagine::calc::character_extras as extras;
    let shifts: indexmap::IndexSet<i32> = [570, 749, 148, 1815].into_iter().collect();
    let mut got: Vec<i32> = extras::compendium_entries(&shifts).unwrap().unwrap().into_iter().collect();
    got.sort();
    assert_eq!(got, vec![10570, 11231, 11236]);
    let all = extras::all_compendium_entries().unwrap().unwrap();
    assert!(all.contains(&10001) && all.contains(&11231) && !all.contains(&570));
}
