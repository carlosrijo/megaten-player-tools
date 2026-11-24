//! Port of `src/Build/ItemIcons.cs`.
//!
//! 32x32 icons for the page, from the client's own files. An item's icon number (items.json "icon") is looked up in the
//! client's BinaryData\Client\CIconData_Item.bin (not encrypted: a u32 count, then 40-byte records of u32 key + 36-byte
//! name), which gives the file name: Interface\tga\icon_<name>.tga, or Interface\tga2 for IMAGINE's own items (their
//! name is just the number, e.g. icon_2625.tga). A skill's file name comes from reference\skill-icons.tsv
//! (GameData.SkillIconAsset; it already starts with "icon_"). Converted to PNG and kept in cache\item-icons\<item id>.png (items) and
//! dash\skill-icons\<icon name>.png (skills), so the page still has them when the game isn't running.
//!
//! The PNG bytes come from the `image` crate (see [`crate::data::tga`]): the same pictures as the C# wrote, not the
//! same bytes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::demon_model::{base64, dds_to_png};
use crate::Result;
use crate::data::characters::is_invalid_file_name_char;
use crate::data::game_data::GameData;
use crate::data::store::Store;
use crate::data::tga;
use crate::game::client_reader;
use crate::text::ic_key;

type IconTable = Arc<HashMap<i32, String>>;

/// C# `Tables`: the CIconData tables read so far, by file path (ignoring case).
static TABLES: Mutex<Option<HashMap<String, IconTable>>> = Mutex::new(None);
/// C# `ClientDirs`: the client folder found per data folder (a missing one is remembered, too).
static CLIENT_DIRS: Mutex<Option<HashMap<String, Option<PathBuf>>>> = Mutex::new(None);

/// PNG bytes of an item's icon; None when it was never cached and the client can't supply it.
pub fn png(store: &Store, data: &GameData, item_id: i32, pid: u32) -> Result<Option<Vec<u8>>> {
    if item_id <= 0 {
        return Ok(None);
    }
    let icon = data.item_icon(item_id)?;
    Ok(load(&store.item_icon_path(item_id), Some("CIconData_Item.bin"), icon, pid, store))
}

/// A skill's icon for the page: the icon's name (reference\skill-icons.tsv via GameData.SkillIconAsset), with the
/// picture saved once as dash\skill-icons\<name>.png, which the page loads by name (meta.skillIcons is the folder).
/// None when the skill has no icon or the picture can't be made.
pub fn skill_asset(store: &Store, data: &GameData, skill_id: i32, pid: u32) -> Result<Option<String>> {
    let asset = if skill_id > 0 { data.skill_icon_asset(skill_id)? } else { None };
    let Some(asset) = asset else { return Ok(None) };
    let cached = store.skill_icon_dir().join(format!("{asset}.png"));
    // Already saved: only the name is needed (reading the picture just to drop it cost ~25% of a rebuild).
    if cached.is_file() {
        return Ok(Some(asset));
    }
    let got = load(&cached, None, (0, Some(asset.clone())), pid, store);
    Ok(got.map(|_| asset))
}

/// The item's icon as a data: URI for the page's JSON, or None.
pub fn data_uri(store: &Store, data: &GameData, item_id: i32, pid: u32) -> Result<Option<String>> {
    Ok(uri(png(store, data, item_id, pid)?.as_deref()))
}

/// The Demonic Compendium's icon: the game's own menu picture for it, cut out of the menu icon sheet
/// (Interface\tga\menu_icon_00.tga, 32x32 cells; the book with a summoning circle is column 1, row 3 from the top
/// left, counting from 0). Kept in cache\ui-icons\compendium.png.
pub fn compendium_data_uri(store: &Store, pid: u32) -> Option<String> {
    menu_icon(store, "compendium", 32, 96, pid, "menu_icon_00.tga", 32)
}

/// The clan's icon: the menu picture of the Communication menu's clan entry (two figures under crossed banners,
/// below party and friends): column 4, row 4 of the same sheet. Kept in cache\ui-icons\clan.png.
pub fn clan_data_uri(store: &Store, pid: u32) -> Option<String> {
    menu_icon(store, "clan", 128, 128, pid, "menu_icon_00.tga", 32)
}

/// The quest bonus's icon: the green speech bubble the game shows on the menu when there are new daily missions,
/// the right half of Interface\tga\menu_alert_devil.tga (64x32). Kept in cache\ui-icons\quest.png.
pub fn quest_data_uri(store: &Store, pid: u32) -> Option<String> {
    menu_icon(store, "quest", 32, 0, pid, "menu_alert_devil.tga", 32)
}

/// The mark the game puts on the top left corner of a Mitama demon's picture (COMP bar, demon lists): the whole of
/// Interface\tga\union_mark_mitama.tga (14x14, white; the game tints it). Kept in cache\ui-icons\mitama.png.
pub fn mitama_mark_data_uri(store: &Store, pid: u32) -> Option<String> {
    menu_icon(store, "mitama", 0, 0, pid, "union_mark_mitama.tga", 14)
}

fn menu_icon(store: &Store, name: &str, x: i32, y: i32, pid: u32, file: &str, size: i32) -> Option<String> {
    let cached = store.ui_icon_path(name);
    // catch (IOException or UnauthorizedAccessException) { return null; }
    let run = || -> std::io::Result<Option<String>> {
        if cached.is_file() {
            return Ok(uri(Some(&std::fs::read(&cached)?)));
        }
        let Some(dir) = client_dir(store, pid) else { return Ok(None) };
        let sheet = dir.join("Interface").join("tga").join(file);
        if !sheet.is_file() {
            return Ok(None);
        }
        let Some(png) = tga::to_png_crop(&std::fs::read(&sheet)?, Some((x, y, size, size))) else { return Ok(None) };
        if let Some(parent) = cached.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&cached, &png)?;
        Ok(uri(Some(&png)))
    };
    run().unwrap_or(None)
}

fn uri(png: Option<&[u8]>) -> Option<String> {
    png.map(|p| format!("data:image/png;base64,{}", base64(p)))
}

fn load(cached: &Path, table: Option<&str>, icon: (i32, Option<String>), pid: u32, store: &Store) -> Option<Vec<u8>> {
    let run = || -> std::io::Result<Option<Vec<u8>>> {
        if cached.is_file() {
            return Ok(Some(std::fs::read(cached)?));
        }
        let Some(dir) = client_dir(store, pid) else { return Ok(None) };
        let from_table = match table {
            Some(t) if icon.0 > 0 => icon_table(&dir, t)?.get(&icon.0).cloned(),
            _ => None,
        };
        let asset = from_table.or(icon.1.clone());
        let Some(asset) = asset.filter(|a| !a.is_empty() && !a.chars().any(is_invalid_file_name_char)) else { return Ok(None) };
        let name = if asset.len() >= 5 && asset[..5].eq_ignore_ascii_case("icon_") { asset } else { format!("icon_{asset}") };
        let Some(png) = client_png(&dir, &name, &[]) else { return Ok(None) };
        if let Some(parent) = cached.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(cached, &png)?;
        Ok(Some(png))
    };
    run().unwrap_or(None)
}

/// A picture of the client's Interface folder as PNG: <name>.tga, else <name>.dds (a few icons are only kept as
/// DDS, e.g. Mjolnir Pulveriser's icon_SD01_0019a), looked for in the given folders (default tga, then tga2, IMAGINE's
/// own). None when there is no such file or it can't be read.
pub fn client_png(client_dir: &Path, name: &str, folders: &[&str]) -> Option<Vec<u8>> {
    if client_dir.as_os_str().is_empty() || name.is_empty() || name.chars().any(is_invalid_file_name_char) {
        return None;
    }
    let default = ["tga", "tga2"];
    for f in if folders.is_empty() { &default[..] } else { folders } {
        let tga_path = client_dir.join("Interface").join(f).join(format!("{name}.tga"));
        if tga_path.is_file() {
            // File.ReadAllBytes failing (IOException) would throw in C#; the picture is simply not there here
            return tga::to_png(&std::fs::read(&tga_path).ok()?);
        }
        let dds = client_dir.join("Interface").join(f).join(format!("{name}.dds"));
        if dds.is_file() {
            return dds_to_png(&std::fs::read(&dds).ok()?);
        }
    }
    None
}

/// The picture name (without extension) of an item icon number, as the client's CItemData "icon" or items.json
/// "icon" gives it: CIconData_Item.bin's name, with the "icon_" the files carry. None when the number is unknown.
pub fn item_icon_file(client_dir: &Path, icon_key: i32) -> Option<String> {
    if client_dir.as_os_str().is_empty() || icon_key <= 0 {
        return None;
    }
    let table = icon_table(client_dir, "CIconData_Item.bin").ok()?;
    let a = table.get(&icon_key).filter(|a| !a.is_empty())?;
    Some(if a.len() >= 5 && a[..5].eq_ignore_ascii_case("icon_") { a.clone() } else { format!("icon_{a}") })
}

/// The game client's folder: that of the given client, or of any running one (they share the same files),
/// remembered in cache\client-dir.txt for when none runs. None when never seen. Looked up once per data folder.
pub fn client_dir(store: &Store, pid: u32) -> Option<PathBuf> {
    let mut guard = CLIENT_DIRS.lock().unwrap_or_else(|e| e.into_inner());
    let known = guard.get_or_insert_with(HashMap::new);
    let key = ic_key(&store.dir.to_string_lossy());
    if let Some(k) = known.get(&key) {
        return k.clone();
    }
    let dir = find_client_dir(store, pid);
    known.insert(key, dir.clone());
    dir
}

fn find_client_dir(store: &Store, pid: u32) -> Option<PathBuf> {
    let run = || -> std::io::Result<Option<PathBuf>> {
        let exes: Vec<Option<PathBuf>> = if pid > 0 {
            vec![client_reader::process_exe_path(pid)]
        } else {
            client_reader::client_processes().into_iter().map(|p| p.exe_path).collect()
        };
        let mut dir: Option<PathBuf> = None;
        for exe in exes {
            if dir.is_none() {
                dir = exe.and_then(|e| e.parent().map(Path::to_path_buf));
            }
        }
        let mut dir = dir.filter(|d| !d.as_os_str().is_empty());
        if let Some(d) = &dir {
            std::fs::create_dir_all(store.cache_dir())?;
            std::fs::write(store.client_dir_path(), d.to_string_lossy().as_bytes())?;
        } else if store.client_dir_path().is_file() {
            let text = crate::data::json::read_all_text(&store.client_dir_path()).map_err(std::io::Error::other)?;
            dir = Some(PathBuf::from(text.trim()));
        }
        Ok(dir.filter(|d| !d.as_os_str().is_empty() && d.is_dir()))
    };
    run().unwrap_or(None)
}

fn icon_table(client_dir: &Path, file: &str) -> std::io::Result<IconTable> {
    let path = client_dir.join("BinaryData").join("Client").join(file);
    let key = ic_key(&path.to_string_lossy());
    let mut guard = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    let tables = guard.get_or_insert_with(HashMap::new);
    if let Some(t) = tables.get(&key) {
        return Ok(t.clone());
    }
    let mut t = HashMap::new();
    if path.is_file() {
        let b = std::fs::read(&path)?;
        const REC: usize = 40;
        let n = if b.len() >= 4 { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64 } else { 0 };
        if 4 + n * REC as u64 <= b.len() as u64 {
            for i in 0..n as usize {
                let o = 4 + i * REC;
                let name_bytes = &b[o + 4..o + REC];
                let len = name_bytes.iter().position(|&c| c == 0).unwrap_or(REC - 4);
                // Encoding.ASCII: anything above 127 reads as '?'
                let name: String = name_bytes[..len].iter().map(|&c| if c < 128 { c as char } else { '?' }).collect();
                if !name.is_empty() {
                    t.insert(i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]), name);
                }
            }
        }
    }
    let t = Arc::new(t);
    tables.insert(key, t.clone());
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("imagine-item-icons-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn tga_1x1() -> Vec<u8> {
        let mut t = vec![0u8; 18];
        t[2] = 2;
        t[12] = 1;
        t[14] = 1;
        t[16] = 32;
        t.extend_from_slice(&[1, 2, 3, 4]);
        t
    }

    #[test]
    fn icon_table_names_and_item_icon_file() {
        let d = temp_dir("table");
        let bin = d.join("BinaryData").join("Client");
        std::fs::create_dir_all(&bin).unwrap();
        let mut b = 3u32.to_le_bytes().to_vec();
        for (key, name) in [(5i32, "ICON_x"), (7, "2625"), (9, "")] {
            b.extend_from_slice(&key.to_le_bytes());
            let mut n = name.as_bytes().to_vec();
            n.resize(36, 0);
            b.extend_from_slice(&n);
        }
        std::fs::write(bin.join("CIconData_Item.bin"), &b).unwrap();
        assert_eq!(item_icon_file(&d, 5).as_deref(), Some("ICON_x"));
        assert_eq!(item_icon_file(&d, 7).as_deref(), Some("icon_2625"));
        assert_eq!(item_icon_file(&d, 9), None);
        assert_eq!(item_icon_file(&d, 0), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn client_png_looks_in_tga_then_tga2() {
        let d = temp_dir("png");
        let tga2 = d.join("Interface").join("tga2");
        std::fs::create_dir_all(&tga2).unwrap();
        std::fs::write(tga2.join("icon_1.tga"), tga_1x1()).unwrap();
        assert!(client_png(&d, "icon_1", &[]).is_some());
        assert!(client_png(&d, "icon_1", &["tga"]).is_none());
        assert!(client_png(&d, "icon_2", &[]).is_none());
        assert!(client_png(&d, "a:b", &[]).is_none());
        let img = image::load_from_memory(&client_png(&d, "icon_1", &["tga2"]).unwrap()).unwrap().to_rgba8();
        assert_eq!(img.into_raw(), vec![3, 2, 1, 4]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
