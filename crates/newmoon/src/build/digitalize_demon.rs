//! Port of `src/Build/DigitalizeDemon.cs`.
//!
//! The demon a character digitalizes with, as saved with each snapshot (cache\digitalize.json and the CSV's
//! "Digitalize demon record" column): the demon's own data, the learned assists and whether Mitama sets are boosted.
//! Enough to recompute what digitalizing gives without the game.
//!
//! The C# saved these records with `JsonSerializer` (default options + `IncludeFields`): PascalCase member names in
//! declaration order, the get-only `Summoned` and `Loaded` of `DemonRecord` last, a `byte[]` as base64, compact text,
//! non-ASCII and `<>&'+` escaped as `\uXXXX`. The helpers here write and read exactly that (the C# and the Rust program
//! read each other's files).

use std::path::Path;

use chrono::NaiveDateTime;
use serde_json::{Value, json};

use crate::Result;
use crate::calc::digitalize::DigitalizeResult;
use crate::context::Context;
use crate::data::game_data::GameData;
use crate::data::json::{self, At, Escape};
use crate::data::store::Store;
use crate::data::tga;
use crate::diag::log;
use crate::game::DemonRecord;
use crate::game::demon_reader::{self, Progress};
use crate::memory::GameMemory;

use super::demon_model::base64;
use super::item_icons;
use super::pipeline;

/// `DateTime.ToString("s")`.
pub fn iso_s(t: NaiveDateTime) -> String {
    t.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// The demon a character digitalizes with (C# `record DigitalizeSave(TakenAt, DigitalizedWhenTaken, Demon, Assists, MitamaBoost)`).
/// `demon` is None only for a record read from a file that has none (the C# let `Demon` be null there).
#[derive(Debug, Clone, PartialEq)]
pub struct DigitalizeSave {
    pub taken_at: Option<String>,
    pub digitalized_when_taken: bool,
    pub demon: Option<DemonRecord>,
    pub assists: Option<Vec<i32>>,
    pub mitama_boost: bool,
}

// ---------- the JSON of a DemonRecord ----------

fn ints(v: &[i32]) -> Value {
    Value::Array(v.iter().map(|&i| json!(i)).collect())
}

fn opt_ints(v: &Option<Vec<i32>>) -> Value {
    match v {
        Some(v) => ints(v),
        None => Value::Null,
    }
}

/// The record as `JsonSerializer.Serialize` wrote it (members in declaration order, then `Summoned` and `Loaded`).
pub fn demon_to_value(d: &DemonRecord) -> Value {
    json!({
        "Address": d.address,
        "Flags": d.flags,
        "Name": d.name,
        "Type": d.r#type,
        "Level": d.level,
        "ObjectId": d.object_id,
        "Hp": d.hp,
        "MaxHp": d.max_hp,
        "Mp": d.mp,
        "MaxMp": d.max_mp,
        "LiveValue": ints(&d.live_value),
        "LiveBonus": ints(&d.live_bonus),
        "GrowthType": d.growth_type,
        "Reunion": ints(&d.reunion),
        "MitamaType": d.mitama_type,
        "MitamaRank": d.mitama_rank,
        "MitamaReunion": base64(&d.mitama_reunion),
        "MitamaSetsShown": Value::Array(d.mitama_sets_shown.iter().map(|&b| Value::Bool(b)).collect()),
        "Skills": ints(&d.skills),
        "Acquired": ints(&d.acquired),
        "SoulPoints": d.soul_points,
        "ForceValues": opt_ints(&d.force_values),
        "ForceStack": opt_ints(&d.force_stack),
        "Equipment": opt_ints(&d.equipment),
        "Summoned": d.summoned(),
        "Loaded": d.loaded(),
    })
}

fn int_list(v: Option<&Value>) -> Vec<i32> {
    json::arr(v).map(|x| json::int(Some(x))).collect()
}

fn opt_int_list(v: Option<&Value>) -> Option<Vec<i32>> {
    v.filter(|x| x.is_array()).map(|x| int_list(Some(x)))
}

/// `Convert.FromBase64String`: None when the text is not valid base64.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let b = s.as_bytes();
    if b.len() % 4 != 0 {
        return None;
    }
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    for chunk in b.chunks(4) {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 {
            return None;
        }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n <<= 6;
            if i < 4 - pad {
                n |= val(c)?;
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// A record read back from its JSON (`Summoned` and `Loaded` are computed, so they are ignored). None unless the node is an object.
pub fn demon_from_value(v: &Value) -> Option<DemonRecord> {
    if !v.is_object() {
        return None;
    }
    Some(DemonRecord {
        address: json::num(v.at("Address")).map(|d| d as i64).unwrap_or(0),
        flags: json::int(v.at("Flags")),
        name: json::str(v.at("Name")).unwrap_or_default(),
        r#type: json::int(v.at("Type")),
        level: json::int(v.at("Level")),
        object_id: json::num(v.at("ObjectId")).map(|d| d as u32).unwrap_or(0),
        hp: json::int(v.at("Hp")),
        max_hp: json::int(v.at("MaxHp")),
        mp: json::int(v.at("Mp")),
        max_mp: json::int(v.at("MaxMp")),
        live_value: int_list(v.at("LiveValue")),
        live_bonus: int_list(v.at("LiveBonus")),
        growth_type: json::int(v.at("GrowthType")),
        reunion: int_list(v.at("Reunion")),
        mitama_type: json::int(v.at("MitamaType")),
        mitama_rank: json::int(v.at("MitamaRank")),
        mitama_reunion: match v.at("MitamaReunion") {
            Some(Value::String(s)) => base64_decode(s)?,
            _ => Vec::new(),
        },
        mitama_sets_shown: json::arr(v.at("MitamaSetsShown")).map(|x| x.as_bool().unwrap_or(false)).collect(),
        skills: int_list(v.at("Skills")),
        acquired: int_list(v.at("Acquired")),
        soul_points: json::int(v.at("SoulPoints")),
        force_values: opt_int_list(v.at("ForceValues")),
        force_stack: opt_int_list(v.at("ForceStack")),
        equipment: opt_int_list(v.at("Equipment")),
    })
}

/// `JsonSerializer.Serialize(value, opts)` with the default options: compact, default encoder.
pub fn compact(v: &Value) -> String {
    json::serialize(v, Escape::Default, false)
}

impl DigitalizeSave {
    pub fn new(taken_at: Option<String>, digitalized_when_taken: bool, demon: DemonRecord, assists: Option<Vec<i32>>, mitama_boost: bool) -> DigitalizeSave {
        DigitalizeSave { taken_at, digitalized_when_taken, demon: Some(demon), assists, mitama_boost }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "TakenAt": self.taken_at,
            "DigitalizedWhenTaken": self.digitalized_when_taken,
            "Demon": self.demon.as_ref().map(demon_to_value),
            "Assists": opt_ints(&self.assists),
            "MitamaBoost": self.mitama_boost,
        })
    }

    pub fn pack(&self) -> String {
        compact(&self.to_value())
    }

    /// None for an empty or unreadable text (the C# caught `JsonException`).
    pub fn unpack(text: &str) -> Option<DigitalizeSave> {
        if text.chars().all(char::is_whitespace) {
            return None;
        }
        let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
        if !v.is_object() {
            return None;
        }
        Some(DigitalizeSave {
            taken_at: json::str(v.at("TakenAt")),
            digitalized_when_taken: json::truthy(v.at("DigitalizedWhenTaken")),
            demon: v.at("Demon").and_then(demon_from_value),
            assists: opt_int_list(v.at("Assists")),
            mitama_boost: json::truthy(v.at("MitamaBoost")),
        })
    }

    pub fn load(store: &Store) -> Result<Option<DigitalizeSave>> {
        let path = store.digitalize_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(DigitalizeSave::unpack(&json::read_all_text(&path)?))
    }

    pub fn save(&self, store: &Store) -> Result<()> {
        std::fs::create_dir_all(store.cache_dir())?;
        std::fs::write(store.digitalize_path(), self.pack())?;
        Ok(())
    }

    /// Which demon counts: the one the character is digitalized with right now; otherwise the saved one, refreshed from
    /// the COMP when it's still there (matched by object id, or by type and name after a restart: object ids change per
    /// session). Assists and the Mitama boost are read again each time. None when there's no demon to go by.
    pub fn read(mem: &GameMemory, store: &Store, data: &GameData, now: NaiveDateTime) -> Result<Option<DigitalizeSave>> {
        let saved = DigitalizeSave::load(store)?;
        let progress = log::timed("snapshot: progress", || demon_reader::read_progress(mem), |p| match p { Some(p) => format!("found, {} entries in the learned-skill list", p.skills.as_ref().map_or(0, |s| s.len())), None => "not found".into() });
        let demons = log::timed("snapshot: demons", || demon_reader::read_demons(mem, false), |d| format!("{} demon records with details", d.len()));
        let mut demon: Option<DemonRecord> = None;
        if let Some(p) = progress.as_ref().filter(|p| p.digitalized()) {
            demon = demons.iter().find(|d| d.object_id == p.digitalized_demon).cloned();
        }
        if demon.is_none() {
            if let Some(old) = saved.as_ref().and_then(|s| s.demon.as_ref()) {
                demon = demons.iter().find(|d| d.object_id == old.object_id && d.r#type == old.r#type).cloned();
                let same: Vec<&DemonRecord> = demons.iter().filter(|d| d.r#type == old.r#type && d.name == old.name).collect();
                if demon.is_none() {
                    demon = Some(if same.len() == 1 { same[0].clone() } else { old.clone() });
                }
            }
        }
        // the demons seen, and which one is summoned now (the Overview shows it); of two demons of one kind and level, the
        // list keeps this one's record
        pipeline::note_demons(store, &demons, demon.as_ref().map(|d| d.object_id), now)?;
        let Some(demon) = demon else { return Ok(None) };
        let active = progress.as_ref().is_some_and(|p| p.digitalized() && demon.object_id == p.digitalized_demon);
        let assists = match &progress {
            Some(p) => Some(p.assists.clone()),
            None => saved.as_ref().and_then(|s| s.assists.clone()),
        };
        let boost = match progress.as_ref().filter(|p| p.skills.is_some()) {
            Some(p) => p.has_skill(data.mitama_set_boost_skill()?),
            None => saved.as_ref().is_some_and(|s| s.mitama_boost),
        };
        DigitalizeSave::save_progress(store, data, progress.as_ref(), now)?;
        Ok(Some(DigitalizeSave { taken_at: Some(iso_s(now)), digitalized_when_taken: active, demon: Some(demon), assists, mitama_boost: boost }))
    }

    /// The learned assists and the Mitama set boost as last read from the game (cache\digitalize-progress.json). They
    /// belong to the character, not to a demon, so switching the digitalize demon can use them without reading the game.
    /// Written by every read that has them: a snapshot, and the picker's "Load demons from the game".
    pub fn save_progress(store: &Store, data: &GameData, progress: Option<&Progress>, now: NaiveDateTime) -> Result<()> {
        let Some(progress) = progress else { return Ok(()) }; // not readable now: keep what was read before
        let boost = if progress.skills.is_some() {
            progress.has_skill(data.mitama_set_boost_skill()?)
        } else if let Some(p) = DigitalizeSave::load_progress(store) {
            p.mitama_boost
        } else {
            DigitalizeSave::load(store)?.is_some_and(|s| s.mitama_boost)
        };
        let run = || -> std::io::Result<()> {
            std::fs::create_dir_all(store.cache_dir())?;
            let v = json!({ "At": iso_s(now), "Assists": ints(&progress.assists), "MitamaBoost": boost });
            std::fs::write(store.digitalize_progress_path(), compact(&v))
        };
        let _ = run(); // catch (IOException or UnauthorizedAccessException) { }
        Ok(())
    }

    pub fn load_progress(store: &Store) -> Option<SeenProgress> {
        let path = store.digitalize_progress_path();
        if !path.exists() {
            return None;
        }
        let text = json::read_all_text(&path).ok()?; // catch (JsonException or IOException) { return null; }
        let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
        if !v.is_object() {
            return None;
        }
        Some(SeenProgress { at: json::str(v.at("At")), assists: opt_int_list(v.at("Assists")), mitama_boost: json::truthy(v.at("MitamaBoost")) })
    }

    /// `Compute(ctx, level)`: what digitalizing with this demon gives.
    pub fn compute(&self, ctx: &Context, level: i32) -> Result<DigitalizeResult> {
        let demon = self.demon.as_ref().ok_or_else(|| anyhow::anyhow!("Value cannot be null. (Parameter 'd')"))?;
        ctx.digitalize(level)?.compute(demon, self.assists.as_deref(), self.mitama_boost)
    }

    /// The demon's icon for the page (dash\demon.png): the client's own 32x32 icon (Interface\tga\<icon>.tga, the icon
    /// name and folder from GameData.DevilIconFile), converted to PNG. Kept as it was when the client or icon can't be read.
    pub fn save_icon(store: &Store, data: &GameData, pid: u32, r#type: i32) -> Result<()> {
        let Some(png) = DigitalizeSave::icon(store, data, pid, r#type)? else { return Ok(()) };
        let run = || -> std::io::Result<()> {
            std::fs::create_dir_all(store.dash_dir())?;
            std::fs::write(store.demon_picture_path(), &png)
        };
        let _ = run();
        Ok(())
    }

    /// A demon type's 32x32 icon as PNG: converted from the client's Interface\tga when the client is known (pid > 0) and
    /// kept in cache\demon-icons, else that kept copy. None when neither exists.
    pub fn icon(store: &Store, data: &GameData, pid: u32, r#type: i32) -> Result<Option<Vec<u8>>> {
        let cached = store.demon_icon_path(r#type);
        let run = || -> std::io::Result<Option<Vec<u8>>> {
            if cached.is_file() {
                return Ok(Some(std::fs::read(&cached)?));
            }
            let Some((name, folder)) = data.devil_icon_file(r#type).map_err(std::io::Error::other)? else { return Ok(None) };
            // the client's folder: the running game's, else the one remembered from before (ItemIcons.ClientDir)
            let dir = item_icons::client_dir(store, pid);
            let tga_path = dir.unwrap_or_default().join("Interface").join(folder).join(format!("{name}.tga"));
            if !tga_path.is_file() {
                return Ok(None);
            }
            let Some(png) = tga::to_png(&std::fs::read(&tga_path)?) else { return Ok(None) };
            if let Some(parent) = Path::new(&cached).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&cached, &png)?;
            Ok(Some(png))
        };
        // catch (IOException or UnauthorizedAccessException or ...) { return null; }
        Ok(run().unwrap_or(None))
    }
}

/// The learned assists and the Mitama set boost as last read (C# `SeenProgress(At, Assists, MitamaBoost)`).
#[derive(Debug, Clone, PartialEq)]
pub struct SeenProgress {
    pub at: Option<String>,
    pub assists: Option<Vec<i32>>,
    pub mitama_boost: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demon() -> DemonRecord {
        DemonRecord {
            address: 1058211668,
            flags: 49,
            name: "Cait Sith".into(),
            r#type: 10549,
            level: 1,
            object_id: 2181028,
            hp: 218,
            max_hp: 218,
            mp: 72,
            max_mp: 72,
            live_value: vec![36, 25, 27, 15, 35, 29, 0, 0, 0, 0, 0, 0, 70, 55, 48, 38, 0, 0, 0, 25, 12],
            live_bonus: vec![14; 21],
            growth_type: 86,
            reunion: vec![0; 12],
            mitama_type: 3,
            mitama_rank: 0,
            mitama_reunion: vec![0, 1, 2, 3, 250],
            mitama_sets_shown: vec![false, true],
            skills: vec![7, 109, 251, 0, 0, 0, 0, 0],
            acquired: vec![2, 6, 51],
            soul_points: 0,
            force_values: Some(vec![0; 20]),
            force_stack: None,
            equipment: Some(vec![0; 4]),
        }
    }

    #[test]
    fn pack_is_the_csharp_text() {
        let s = DigitalizeSave::new(Some("2026-11-23T22:34:36".into()), false, demon(), Some(vec![5, 8]), true);
        let p = s.pack();
        assert!(p.starts_with(r#"{"TakenAt":"2026-11-23T22:34:36","DigitalizedWhenTaken":false,"Demon":{"Address":1058211668,"Flags":49,"Name":"Cait Sith","Type":10549,"#));
        assert!(p.contains(r#""MitamaReunion":"AAECA/o=","MitamaSetsShown":[false,true],"#));
        assert!(p.contains(r#""ForceStack":null,"Equipment":[0,0,0,0],"Summoned":false,"Loaded":true},"Assists":[5,8],"MitamaBoost":true}"#));
        assert_eq!(DigitalizeSave::unpack(&p), Some(s));
    }

    #[test]
    fn unpack_handles_bad_text() {
        assert_eq!(DigitalizeSave::unpack(""), None);
        assert_eq!(DigitalizeSave::unpack("  "), None);
        assert_eq!(DigitalizeSave::unpack("{not json"), None);
    }

    #[test]
    fn base64_roundtrip() {
        for n in 0..8 {
            let v: Vec<u8> = (0..n).map(|i| (i * 37 + 5) as u8).collect();
            assert_eq!(base64_decode(&base64(&v)).unwrap(), v);
        }
        assert!(base64_decode("abc").is_none());
    }
}
