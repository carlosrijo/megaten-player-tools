//! The pictures of the Tools pages (Gear Database, Dungeon Database, DCM Locator, Skill Database, Demon Database),
//! made on the player's PC from their own game folder.
//!
//! Those pictures are the game's own art, so they are never put in git or the exe. On the developer's PC the page
//! builders write them to app\ui\<folder>-icons\; reference\page-icons.tsv (made by `imagine-reader page-icons`) records,
//! for each one, which file of the client's Interface folder it is. A player's app makes a missing picture from that
//! file the first time the page asks for it, and keeps it in <data>\cache\page-icons\<folder>-icons\.
//!
//! The list is made by comparing pixels, not names: every picture of the client is decoded and each page picture is
//! looked up by its pixels, so renamed ones (dcm-icons\1085.png, demon-db-icons\d725.png) are found too.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::Result;
use crate::build::demon_model::dds_to_png;
use crate::build::item_icons;
use crate::data::game_data::reference_lines;
use crate::data::store::Store;
use crate::data::tga;

/// The page folders whose pictures come from the game (the `*-icons` folders under app\ui).
pub const FOLDERS: [&str; 5] = ["gear-db-icons", "dungeon-icons", "dcm-icons", "skill-db-icons", "demon-db-icons"];

/// The list's file in reference\.
pub const LIST: &str = "page-icons.tsv";

// ---------- on the player's PC ----------

/// reference\page-icons.tsv: "<folder>/<name>.png" -> the client file ("tga2/icon_2625.tga").
static SOURCES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    if let Ok(Some(lines)) = reference_lines(LIST) {
        for l in &lines {
            if l.starts_with('#') {
                continue;
            }
            if let Some((page, src)) = l.split_once('\t') {
                m.insert(page.to_string(), src.to_string());
            }
        }
    }
    m
});

/// The game folder, looked for again at most every 10 seconds while it isn't known (the player may start the game
/// with the page already open).
static CLIENT: Mutex<Option<(Option<PathBuf>, Instant)>> = Mutex::new(None);

fn client_dir(data_dir: &Path) -> Option<PathBuf> {
    let mut g = CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((dir, at)) = g.as_ref() {
        if dir.is_some() || at.elapsed() < Duration::from_secs(10) {
            return dir.clone();
        }
    }
    // the data folder's saved game folder (or a running game), else one a character's folder saved
    let mut dir = item_icons::find_client_dir_uncached(&Store::new(data_dir));
    if dir.is_none() {
        if let Ok(rd) = std::fs::read_dir(data_dir.join("characters")) {
            dir = rd.flatten().find_map(|e| item_icons::find_client_dir_uncached(&Store::new(&e.path())));
        }
    }
    *g = Some((dir.clone(), Instant::now()));
    dir
}

/// Is this page path ("gear-db-icons", "icon_A09_0123a.png") one of the game pictures?
pub fn is_page_icon(rel: &[String]) -> bool {
    rel.len() == 2 && FOLDERS.contains(&rel[0].as_str()) && rel[1].to_ascii_lowercase().ends_with(".png")
}

/// The picture the page asks for (`rel` = folder, file name), as PNG: the copy made earlier, else made now from the
/// player's game folder and kept. None when it isn't a game picture, the list doesn't know it, or the game folder
/// isn't known yet (the page then shows the name without a picture, as before).
pub fn png(data_dir: &Path, rel: &[String]) -> Option<Vec<u8>> {
    if !is_page_icon(rel) {
        return None;
    }
    let cached = data_dir.join("cache").join("page-icons").join(&rel[0]).join(&rel[1]);
    if let Ok(b) = std::fs::read(&cached) {
        return Some(b);
    }
    let src = SOURCES.get(&format!("{}/{}", rel[0], rel[1]))?;
    let client = client_dir(data_dir)?;
    // "a|b": b is the fallback when a player's game folder lacks a
    let png = src.split('|').find_map(|s| client_file_png(&client, s))?;
    if std::fs::create_dir_all(cached.parent()?).is_ok() {
        let _ = std::fs::write(&cached, &png);
    }
    Some(png)
}

/// A client picture ("tga/icon_x.tga", "tga2/icon_y.dds", under Interface) as PNG.
fn client_file_png(client: &Path, src: &str) -> Option<Vec<u8>> {
    if src.split('/').any(|s| s.is_empty() || s == ".." || s.contains(['\\', ':'])) {
        return None;
    }
    let mut p = client.join("Interface");
    for s in src.split('/') {
        p.push(s);
    }
    let bytes = std::fs::read(&p).ok()?;
    if src.to_ascii_lowercase().ends_with(".dds") { dds_to_png(&bytes) } else { tga::to_png(&bytes) }
}

// ---------- on the developer's PC: the list ----------

fn pixel_key(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut s = DefaultHasher::new();
    (w, h).hash(&mut s);
    rgba.hash(&mut s);
    s.finish()
}

fn png_rgba(png: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}

/// A client picture's pixels.
fn client_rgba(path: &Path, dds: bool) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    if dds {
        png_rgba(&dds_to_png(&bytes)?)
    } else {
        let (rgba, w, h) = tga::decode_rgba(&bytes)?;
        Some((w as u32, h as u32, rgba))
    }
}

/// What `build_list` found.
pub struct ListResult {
    pub listed: usize,
    /// page pictures no client picture has the same pixels as
    pub unmatched: Vec<String>,
}

/// reference\page-icons.tsv from the page pictures in `ui_dir` (app\ui) and the client's Interface\tga, tga2.
pub fn build_list(ui_dir: &Path, client: &Path, out: &Path) -> Result<ListResult> {
    // every client picture by its pixels; the same pixels can be in several files, and a file of the page picture's
    // own name wins (so the list reads naturally)
    let mut by_pixels: HashMap<u64, Vec<String>> = HashMap::new();
    for folder in ["tga", "tga2"] {
        let dir = client.join("Interface").join(folder);
        for e in std::fs::read_dir(&dir)?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            let dds = lower.ends_with(".dds");
            if !(dds || lower.ends_with(".tga")) {
                continue;
            }
            if let Some((w, h, rgba)) = client_rgba(&e.path(), dds) {
                by_pixels.entry(pixel_key(w, h, &rgba)).or_default().push(format!("{folder}/{name}"));
            }
        }
    }
    let mut rows = Vec::new();
    let mut unmatched = Vec::new();
    for folder in FOLDERS {
        let dir = ui_dir.join(folder);
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.to_ascii_lowercase().ends_with(".png")).collect();
        files.sort();
        for f in files {
            let page = format!("{folder}/{f}");
            let Some((w, h, rgba)) = std::fs::read(dir.join(&f)).ok().and_then(|b| png_rgba(&b)) else {
                unmatched.push(page);
                continue;
            };
            let stem = f[..f.len() - 4].to_ascii_lowercase();
            let pick = by_pixels.get(&pixel_key(w, h, &rgba)).and_then(|c| {
                // the same name first, then a .tga over a .dds, then tga over tga2
                c.iter().min_by_key(|s| {
                    let file = s.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
                    let same = file.rsplit_once('.').map(|x| x.0) == Some(stem.as_str());
                    (!same, file.ends_with(".dds"), s.starts_with("tga2/"), (*s).clone())
                })
            });
            // checked: the picked file really gives these pixels (not only the same hash)
            match pick {
                Some(src) if client_rgba(&client.join("Interface").join(src.replace('/', "\\")), src.to_ascii_lowercase().ends_with(".dds")).is_some_and(|(cw, ch, c)| (cw, ch) == (w, h) && c == rgba) => {
                    // a file with a second dot ("icon_D1_01_003a.baclup.tga", a leftover copy in this game folder) may
                    // not be in every player's: the plain file of that name follows as the fallback
                    let (dir, file) = src.split_once('/').unwrap_or(("", src.as_str()));
                    let parts: Vec<&str> = file.split('.').collect();
                    let plain = (parts.len() > 2).then(|| format!("{dir}/{}.{}", parts[0], parts[parts.len() - 1]));
                    match plain.filter(|p| client.join("Interface").join(p.replace('/', "\\")).is_file()) {
                        Some(p) => rows.push(format!("{page}\t{src}|{p}")),
                        None => rows.push(format!("{page}\t{src}")),
                    }
                }
                _ => unmatched.push(page),
            }
        }
    }
    let mut text = String::from("# page picture (app\\ui)\tclient picture (Interface\\). Made by imagine-reader page-icons; see build\\page_icons.rs\r\n");
    for r in &rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    std::fs::write(out, text)?;
    Ok(ListResult { listed: rows.len(), unmatched })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_game_picture_folders_are_page_icons() {
        let v = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
        assert!(is_page_icon(&v("gear-db-icons", "icon_A09_0123a.png")));
        assert!(is_page_icon(&v("dcm-icons", "1085.png")));
        assert!(!is_page_icon(&v("gear-db-icons", "x.json")));
        assert!(!is_page_icon(&v("other", "x.png")));
        assert!(!is_page_icon(&["gear-db-icons".to_string()]));
    }

    #[test]
    fn the_list_is_compiled_in() {
        if reference_lines(LIST).ok().flatten().is_some() {
            assert!(SOURCES.len() > 10_000, "{} pictures listed", SOURCES.len());
            assert!(SOURCES.values().flat_map(|s| s.split('|')).all(|s| s.starts_with("tga/") || s.starts_with("tga2/")));
        }
    }

    #[test]
    fn a_client_path_cannot_leave_the_interface_folder() {
        assert!(client_file_png(Path::new("C:\\x"), "../secret.tga").is_none());
        assert!(client_file_png(Path::new("C:\\x"), "tga/..\\a.tga").is_none());
    }
}
