//! Pins the wave-2 calc ports (`ItemEffects`, `GearEffects`, `Digitalize`, `CharacterExtras`, `GearDb`) to what the C# code
//! produced. The expected values come from the C# sources run by a throwaway harness (compiled from the real `src\` files) over
//! the same inputs; the full comparison (about 100 MB of lines, over a copy of a real data folder) is `examples/w2dump.rs`
//! against the harness, and `examples/geardb.rs` against the C# CLI's `geardb` (byte-identical files).
//! Tests that read the website content or the server's tokusei XML (folders outside this repo, named in data\config.json) are
//! skipped on a machine that does not have them; the ones on the repo's own reference tables always run.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use indexmap::IndexMap;
use newmoon::calc::character_extras as extras;
use newmoon::calc::digitalize::Digitalize;
use newmoon::calc::gear_effects::GearEffects;
use newmoon::calc::item_effects::ItemEffects;
use newmoon::data::config;
use newmoon::data::game_data::GameData;
use newmoon::game::progress_reader::ProgressState;
use newmoon::game::{SetRecord, WornItem};
use serde_json::Value;

#[allow(dead_code)]
#[path = "../examples/w2dump.rs"]
mod w2dump;

/// Whether the website content, the server's tokusei XML and constants.xml of this machine are there.
fn external_data() -> bool {
    let c = config::current();
    let dir = |p: &Option<String>| p.as_deref().is_some_and(|p| Path::new(p).is_dir());
    let file = |p: &Option<String>| p.as_deref().is_some_and(|p| Path::new(p).is_file());
    let ok = dir(&c.website_content) && dir(&c.tokusei_xml) && file(&c.server_constants);
    if !ok {
        eprintln!("skipped: the website / server data folders of data\\config.json are not on this machine");
    }
    ok
}

fn data() -> Arc<GameData> {
    Arc::new(GameData::new())
}

#[test]
fn counts_from_masks_and_quest_ids() {
    // reference\quest-types.tsv and devil-book.tsv are compiled into the library
    assert_eq!(extras::quest_bonus_count(Some(Vec::<i32>::new())).unwrap(), Some(0));
    assert_eq!(extras::quest_bonus_count(None::<Vec<i32>>).unwrap(), None);
    assert_eq!(extras::quest_bonus_count(Some(0..200)).unwrap(), Some(81));
    assert_eq!(extras::quest_bonus_count(Some((0..100).map(|i| 1 + i * 3))).unwrap(), Some(40));
    assert_eq!(extras::quest_bonus_count_from_mask(Some(&[0xFF; 128])).unwrap(), Some(367));
    assert_eq!(extras::quest_bonus_count_from_mask(Some(&[0; 128])).unwrap(), Some(0));
    assert_eq!(extras::quest_bonus_count_from_mask(None).unwrap(), None);
    assert_eq!(extras::compendium_count(Some(&[0xFF; 512])).unwrap(), Some(341));
    assert_eq!(extras::compendium_count(Some(&[0xFF; 100])).unwrap(), Some(222)); // a short mask: only the entries it covers
    assert_eq!(extras::compendium_count(Some(&[0; 512])).unwrap(), Some(0));
    assert_eq!(extras::compendium_count(None).unwrap(), None);
}

#[test]
fn item_effects_of_a_fused_weapon() {
    if !external_data() {
        return;
    }
    // Hecatonchires's Leviathan Blaster of the Warmonger: tarot 64, soul 3105, three fusion bonuses at the top of the table
    let w = WornItem { id: 2625, tarot: 64, soul: 3105, basic: 2625, special: 2616, mods: vec![20, 15, 200, 319, 321], fuse: vec![50, 50, 50], durability: 41, max_durability: 49 };
    let fx = ItemEffects::new(data(), "LAW", 95, IndexMap::new());
    let got: Vec<String> = fx.compute(Some(&w), 0, "Levi Blaster", None).unwrap().iter().map(w2dump::fx).collect();
    let want = [
        "  fx label=Long-Range|value=82|pct=0|cond=always|note=|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Spell|value=82|pct=0|cond=always|note=|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Support|value=45|pct=0|cond=always|note=|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Almighty Resist|value=50|pct=1|cond=always|note=|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Critical|value=80|pct=0|cond=always|note=|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Long-Range|value=45|pct=0|cond=always|note=fusion great success|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Spell|value=45|pct=0|cond=always|note=fusion great success|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Support|value=45|pct=0|cond=always|note=fusion great success|target=self|source=item|item=Levi Blaster|tok=0|skill=0|missing=0",
        "  fx label=Shot Damage|value=42.75|pct=1|cond=always|note=LvL × 0.45|target=self|source=characteristic|item=Levi Blaster|tok=560077|skill=0|missing=0",
        "  fx label=Rapid Damage|value=42.75|pct=1|cond=always|note=LvL × 0.45|target=self|source=characteristic|item=Levi Blaster|tok=560078|skill=0|missing=0",
        "  fx label=Critical|value=30|pct=0|cond=always|note=|target=self|source=characteristic|item=Levi Blaster|tok=363050|skill=0|missing=0",
        "  fx label=Long-Range|value=20|pct=0|cond=always|note=|target=self|source=mod|item=Levi Blaster|tok=360214|skill=0|missing=0",
        "  fx label=Long-Range|value=6|pct=0|cond=always|note=|target=self|source=mod|item=Levi Blaster|tok=360200|skill=0|missing=0",
        "  fx label=Almighty Affinity|value=35|pct=1|cond=always|note=|target=self|source=mod|item=Levi Blaster|tok=362039|skill=0|missing=0",
        "  fx label=Rapid Damage|value=20|pct=1|cond=always|note=|target=self|source=mod|item=Levi Blaster|tok=30008|skill=0|missing=0",
        "  fx label=Rapid Damage|value=50|pct=1|cond=always|note=|target=self|source=mod|item=Levi Blaster|tok=30089|skill=0|missing=0",
        "  fx label=Rapid Damage|value=4|pct=1|cond=always|note=|target=self|source=tarot|item=Levi Blaster|tok=30057|skill=0|missing=0",
        "  fx label=Unknown effect (tokusei 2000192)|value=~|pct=0|cond=unparsed|note=|target=self|source=soul|item=Levi Blaster|tok=2000192|skill=0|missing=1",
        "  fx label=Unknown effect (tokusei 2000193)|value=~|pct=0|cond=unparsed|note=|target=self|source=soul|item=Levi Blaster|tok=2000193|skill=0|missing=1",
        "  fx label=Unknown effect (tokusei 2000194)|value=~|pct=0|cond=unparsed|note=|target=self|source=soul|item=Levi Blaster|tok=2000194|skill=0|missing=1",
    ];
    assert_eq!(got, want);

    // a broken item gives nothing but a note; bullets (slot 1) keep working at durability 0
    let broken = WornItem { durability: 0, max_durability: 10, ..w.clone() };
    let r = fx.compute(Some(&broken), 0, "Levi Blaster", None).unwrap();
    assert_eq!((r.len(), r[0].label.as_str(), r[0].cond.as_str()), (1, "Broken (durability 0): no bonuses", "inactive"));
    assert!(fx.compute(Some(&broken), 1, "Levi Blaster", None).unwrap().len() > 1);

    // fusion great-success growth tables (CharacterState::AdjustFuseBonus), the Machete: close range, bonus 7 / 22 / 50
    let machete = WornItem { id: 1201, fuse: vec![7, 22, 50], ..w.clone() };
    assert_eq!(fx.fuse_bonus(&machete).unwrap(), vec![("CLSR", 4, false), ("SPELL", 15, false), ("SUPPORT", 45, true)]);
    let machete = WornItem { id: 1201, fuse: vec![1, 6, 22], ..w.clone() };
    assert_eq!(fx.fuse_bonus(&machete).unwrap(), vec![("CLSR", 1, false), ("SPELL", 4, false), ("SUPPORT", 15, false)]);
    assert!(fx.fuse_bonus(&WornItem { fuse: vec![0, 0, 0], ..w }).unwrap().is_empty());

    // a tokusei the data does not have, and the affinity names
    let r = fx.tokusei(999999999, "src", "itm").unwrap();
    assert_eq!((r[0].label.as_str(), r[0].cond.as_str(), r[0].missing_data), ("Unknown effect (tokusei 999999999)", "unparsed", true));
    assert_eq!(newmoon::calc::item_effects::affinity_cap_label(9), "Ice Affinity Cap");
}

#[test]
fn set_bonuses_and_appearance() {
    if !external_data() {
        return;
    }
    let table = vec![
        SetRecord { id: 2, pieces: vec![6290, 6490], tokusei: vec![300092] },
        SetRecord { id: 278, pieces: vec![4274], tokusei: vec![60350] },
        SetRecord { id: 4, pieces: vec![6527, 6292], tokusei: vec![300093] },
    ];
    let gear = GearEffects::new("NEUTRAL", &IndexMap::new(), data(), Some(table), None);
    let none: HashMap<i32, String> = HashMap::new();
    let r = gear.set_bonuses(&[6290, 6490, 4274, 6527], &none, &none).unwrap();
    let line = |s: &newmoon::calc::gear_effects::SetReport| format!("{}|{}|{}|{}|{}|{}", s.set_id, s.pieces, s.worn, s.active, s.appearance, s.piece_ids.len());
    assert_eq!(
        r.iter().map(line).collect::<Vec<_>>(),
        vec!["2|Moderator's Outfit, Moderator's Sneakers|2/2|true|false|2", "278|Mr. Ghost|1/1|true|true|0", "4|Isolator's Cap, Isolator's Outfit|1/2|false|false|2"]
    );
    assert_eq!(
        w2dump::fx(&r[0].effects[0]),
        "  fx label=Adds skill Feast of the Earth Mother|value=~|pct=0|cond=always|note=|target=self|source=set|item=Set: Moderator's Outfit (set 2)|tok=300092|skill=0|missing=0"
    );
    assert_eq!(
        w2dump::fx(&r[1].effects[0]),
        "  fx label=0% Chance to Null All Physical|value=~|pct=0|cond=always|note=LvL × 0.03|target=self|source=appearance|item=Mr. Ghost|tok=60350|skill=0|missing=0"
    );
    // without a set table, nothing to report
    let bare = GearEffects::new("NEUTRAL", &IndexMap::new(), data(), None, None);
    assert!(bare.set_bonuses(&[6290, 6490], &none, &none).unwrap().is_empty());
    // fusion sets: no enchant worn -> none
    assert!(gear.fusion_sets(&HashSet::new()).unwrap().is_empty());
}

#[test]
fn digitalizing_with_a_mitama_demon() {
    if !external_data() {
        return;
    }
    // Rubito's Alilat (Lv93, Mitama Nigi), digitalized with every assist of her race, no Mitama set boost
    let demon = w2dump::demon_from_json(
        &serde_json::from_str::<Value>(
            r#"{"Address":468195892,"Flags":51,"Name":"Alilat","Type":10041,"Level":93,"ObjectId":2181027,"Hp":4874,"MaxHp":4874,"Mp":1779,"MaxMp":1779,"LiveValue":[143,198,117,142,124,97,0,0,0,0,0,0,178,160,219,225,0,0,0,88,53],"LiveBonus":[66,97,65,94,64,25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"GrowthType":212,"Reunion":[4,1,4,4,4,0,4,1,4,4,0,7],"MitamaType":2,"MitamaRank":0,"MitamaReunion":"Jy4kKwAAAAAAAAAAAAAAADAiIiQAAAAAZGxvcAAAAAAMEBBGAAAAAAAAAAAAAAAAbW9oQwAAAAAAAAAAAAAAAAgODSYAAAAAJiEhQQAAAAAAAAAAAAAAAAAAAAAAAAAA","MitamaSetsShown":[false,true,false,false,false,false,false,false,false,true,false,false,false,false,false,true,false,false,true,false,false,false,false,false,false,false,false,true,false,false,false,false,true,false,false,false,false,true,false,false,false,false,false,false,false,false,false,false],"Skills":[5306,820,4,962,961,960,958,956],"Acquired":[305,507,508,5026,5027,5112,5114,5206,5307],"SoulPoints":2000,"ForceValues":[750684,330651,600248,575099,485786,444578,944528,203342,69288,25043,25236,10348,8604,12092,3991,71127,46417,4298,632363,10772],"ForceStack":[10144,10269,1298,10271,10283,10272,10118,10270],"Equipment":[0,0,0,0]}"#,
        )
        .unwrap(),
    );
    let d = data();
    let t = d.demons().unwrap();
    let race = t.devils[&demon.r#type].race_id;
    let all: Vec<i32> = t.assists_of_race(race).iter().map(|a| a.id).collect();
    let calc = Digitalize::new(Arc::clone(&d), ItemEffects::new(Arc::clone(&d), "NEUTRAL", 95, IndexMap::new()));
    let r = calc.compute(&demon, Some(&all), false).unwrap();
    assert_eq!((r.growth, r.base_rate, r.rate, r.max_rate, r.hp(), r.mp()), (212, 30, 100, 100, 2781, 1198));
    assert_eq!(&r.base[..16], &[143, 198, 117, 142, 124, 97, 2781, 1198, 7, 7, 15, 36, 64, 56, 62, 82]);
    assert_eq!(r.stats.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(","), "0:143,1:198,2:117,3:142,4:124,5:97,6:2781,7:1198,12:64,13:56,14:62,15:82");
    assert_eq!(r.active_skills, vec![5306, 820]);
    assert_eq!(r.sets.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 9, 15, 18, 27, 32, 37]);
    assert_eq!(
        r.assists.as_ref().unwrap().iter().cloned().collect::<Vec<_>>().join(","),
        "EXTEND_TIME,REDUCE_WAIT,STAT_RATE,AFFINITIES,FORCE_STACK,PASSIVES,TRAITS,ACTIVES,MITAMA_SET"
    );
    assert_eq!((r.extend_time, r.reduce_wait, r.pre_hp, r.pre_mp, r.dependent_hp, r.dependent_mp), (360, 5400, 540, 210, 2781, 1194));
    assert_eq!(r.effects.len(), 63);
    assert_eq!(
        w2dump::fx(&r.effects[0]),
        "  fx label=Strength|value=143|pct=0|cond=digitalized|note=stats|target=self|source=digitalize|item=Alilat (100% of its stats)|tok=0|skill=0|missing=0"
    );
    assert_eq!(
        w2dump::fx(&r.effects[12]),
        "  fx label=Max HP|value=40|pct=1|cond=digitalized|note=passive|target=self|source=digitalize|item=Alilat passive Life Surge|tok=0|skill=4|missing=0"
    );
    // without the assists known, the parts that need one are listed but not counted
    let r = calc.compute(&demon, None, false).unwrap();
    assert_eq!((r.rate, r.assists.is_none()), (30, true));
    assert_eq!(
        w2dump::fx(&r.effects[12]),
        "  fx label=Max HP|value=40|pct=1|cond=inactive|note=passive; needs the PASSIVES assist|target=self|source=digitalize|item=Alilat passive Life Surge|tok=0|skill=4|missing=0"
    );
    // a demon type the table does not have: a note and nothing else
    let mut bad = demon.clone();
    bad.r#type = 99999999;
    let r = calc.compute(&bad, Some(&all), false).unwrap();
    assert_eq!((r.notes.clone(), r.effects.len(), r.devil.is_none()), (vec!["Demon type 99999999 isn't in the demon table.".to_string()], 0, true));
}

#[test]
fn quest_and_compendium_bonuses() {
    if !external_data() {
        return;
    }
    let d = data();
    let fx = ItemEffects::new(Arc::clone(&d), "NEUTRAL", 95, IndexMap::new());
    let st = ProgressState { quest_bonus_count: Some(32), compendium_count: Some(61), has_compendium_v2: Some(true), ..Default::default() };
    let r = extras::compute(&d, Some(&fx), Some(&st), false).unwrap();
    let rep = extras::report_state(&st, &d, &fx, false).unwrap();
    assert!(rep.starts_with("clan: (none), level ? (0 clan skills learned: )\r\nquest bonus count: 32; compendium entries: 61\r\n"));
    assert!(r.iter().any(|e| e.source == "quest") && r.iter().any(|e| e.source == "compendium" && e.target == "partner"));
    // the counts must be known, and the compendium needs v2
    let unknown = extras::compute(&d, None, Some(&ProgressState::default()), false).unwrap();
    assert_eq!(
        unknown.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(),
        vec!["Quest bonus not counted: the completed-quest count isn't known", "Compendium bonus not counted: the compendium entry count isn't known"]
    );
    let no_v2 = ProgressState { quest_bonus_count: Some(0), compendium_count: Some(5), has_compendium_v2: Some(false), ..Default::default() };
    assert!(extras::compute(&d, None, Some(&no_v2), false).unwrap().iter().any(|e| e.label == "No compendium bonus: Demonic Compendium v2 not owned"));
}

/// 64-bit FNV-1a, to pin a big output file without keeping it.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))
}

#[test]
fn gear_database_matches_the_csharp_file() {
    let decoded = Path::new(r"%USERPROFILE%\Documents\smt-decoded\server");
    let sheets = Path::new(r"%USERPROFILE%\Documents\repos\smt-gear-analysis\data\sheet_gear.tsv");
    if !external_data() || !decoded.join("ItemData.xml").is_file() || !sheets.is_file() {
        eprintln!("skipped: the decoded client tables or the gear sheet are not on this machine");
        return;
    }
    let out = std::env::temp_dir().join(format!("newmoon-geardb-{}.json", std::process::id()));
    newmoon::build::gear_db::build(data(), decoded.to_str().unwrap(), sheets.to_str(), out.to_str().unwrap(), None, None, None).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let text = String::from_utf8(bytes).unwrap();
    // the C# CLI wrote 3736521 bytes for the same inputs; since 2026-11-24 every CorrectTbl line is in "stats" (the Gear
    // page tooltip's Basic Features) instead of only the base stats, which moves lines and adds 6,028 commas. The file
    // starts with the day it ran. Since 2026-11-24 night, the 872 pieces with mod slots carry "mods" (read from the
    // record's use restrictions; the C# looked on the record itself and never found them): +7,848 bytes, nothing else.
    assert!(text.starts_with(r#"{"generatedAt":""#));
    let body = &text[r#"{"generatedAt":"2026-11-24""#.len()..];
    assert_eq!((text.len(), fnv(body.as_bytes())), (3750397, GEARDB_FNV));
}

/// FNV-1a of the `geardb` output (the C# one, with all CorrectTbl lines in "stats", plus "mods") after its
/// `{"generatedAt":"yyyy-mm-dd"` start.
const GEARDB_FNV: u64 = 16974497132224511461;
