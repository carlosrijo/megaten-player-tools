//! The files behind https://newmoon.app/ (the page) and https://data.newmoon.app/ (the data folder).
//!
//! The C# app mapped two virtual hosts to folders (`SetVirtualHostNameToFolderMapping`). Here the page is inside the
//! exe (rust-embed, compressed) and the answers are made by the WebResourceRequested handler (see host.rs), with the
//! same origins, so what the page stored (localStorage, IndexedDB) under https://newmoon.app keeps working.
//!
//! Where a page file comes from, in this order:
//!   1. `--dev`: the repo's app\ui folder on disk (page edits need no rebuild);
//!   2. the copy embedded in the exe;
//!   3. a file that is not embedded (the pictures made locally from the game client: app\ui\*-icons, which are the
//!      game's own art and never shipped) from disk: <exe folder>\ui\..., then <repo root>\app\ui\...
//! The data host serves only the data folder.

use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub const HOST: &str = "newmoon.app";
pub const DATA_HOST: &str = "data.newmoon.app";

/// The page (app\ui), compressed into the exe. The folders of game art made locally (*-icons) are never embedded.
#[derive(RustEmbed)]
#[folder = "../../app/ui"]
#[exclude = "*-icons/*"]
#[exclude = "*-icons"]
pub struct UiFiles;

/// The pictures CharacterPicture compares the game window with (app\assets).
#[derive(RustEmbed)]
#[folder = "../../app/assets"]
#[include = "*.png"]
pub struct AssetFiles;

/// app\app.ico: the window's and the taskbar's icon (the exe's own icon resource is the same file).
pub static APP_ICO: &[u8] = include_bytes!("../../../app/app.ico");

pub struct Site {
    /// `--dev`: app\ui on disk, looked at first
    dev_ui: Option<PathBuf>,
    /// where a file that is not embedded may live
    disk_ui: Vec<PathBuf>,
    data_dir: PathBuf,
}

/// An answer for the page.
pub struct Served {
    pub body: Cow<'static, [u8]>,
    pub mime: &'static str,
}

impl Site {
    pub fn new(dev: bool, data_dir: &Path) -> Site {
        let repo = crate::backend::repo_root();
        // C# UiDir(): `Path.GetDirectoryName(_dataDir)\app\ui` in dev mode; the repo root found by walking up is the same place
        let dev_ui = if dev {
            let mut cands = Vec::new();
            if let Some(r) = &repo {
                cands.push(r.join("app").join("ui"));
            }
            if let Some(p) = data_dir.parent() {
                cands.push(p.join("app").join("ui"));
            }
            cands.into_iter().find(|d| d.is_dir())
        } else {
            None
        };
        let mut disk_ui = vec![crate::backend::program_dir().join("ui")];
        if let Some(r) = &repo {
            disk_ui.push(r.join("app").join("ui"));
        }
        Site { dev_ui, disk_ui, data_dir: data_dir.to_path_buf() }
    }

    /// The answer for `https://<host>/<path>` (path without the query), or None for 404.
    pub fn serve(&self, host: &str, url_path: &str) -> Option<Served> {
        let rel = safe_relative(url_path)?;
        if host.eq_ignore_ascii_case(DATA_HOST) {
            return read_file(&self.data_dir, &rel);
        }
        if !host.eq_ignore_ascii_case(HOST) {
            return None;
        }
        let rel = if rel.is_empty() { vec!["index.html".to_string()] } else { rel };
        if let Some(dev) = &self.dev_ui {
            if let Some(s) = read_file(dev, &rel) {
                return Some(s);
            }
        }
        let key = rel.join("/");
        if let Some(f) = UiFiles::get(&key) {
            return Some(Served { body: f.data, mime: mime_of(&key) });
        }
        for d in &self.disk_ui {
            if let Some(s) = read_file(d, &rel) {
                return Some(s);
            }
        }
        None
    }
}

/// The segments of a URL path, percent-decoded; None when it could leave the folder (`..`, a drive, a backslash,
/// an alternate data stream, a NUL).
pub fn safe_relative(url_path: &str) -> Option<Vec<String>> {
    let path = url_path.split(['?', '#']).next().unwrap_or("");
    let decoded = crate::jsonx::unescape(path)?;
    let mut out = Vec::new();
    for seg in decoded.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.contains(['\\', ':', '\0']) || seg.ends_with('.') || seg.ends_with(' ') {
            return None;
        }
        out.push(seg.to_string());
    }
    Some(out)
}

fn read_file(root: &Path, rel: &[String]) -> Option<Served> {
    if rel.is_empty() {
        return None;
    }
    let mut p = root.to_path_buf();
    for s in rel {
        p.push(s);
    }
    // only plain components below the root (safe_relative already refused anything else)
    if !p.components().skip(root.components().count()).all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let body = std::fs::read(&p).ok()?;
    Some(Served { body: Cow::Owned(body), mime: mime_of(rel.last()?) })
}

pub fn mime_of(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "txt" | "tsv" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "text/xml; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cannot_leave_the_folder() {
        assert_eq!(safe_relative("/a/b.png").unwrap(), vec!["a", "b.png"]);
        assert_eq!(safe_relative("/").unwrap(), Vec::<String>::new());
        assert!(safe_relative("/../x").is_none());
        assert!(safe_relative("/a/%2e%2e/x").is_none());
        assert!(safe_relative("/a/..%5Cx").is_none());
        assert!(safe_relative("/a\\b").is_none());
        assert!(safe_relative("/C:/x").is_none());
        assert!(safe_relative("/a.txt::$DATA").is_none());
        assert_eq!(safe_relative("/a%20b.png?v=1").unwrap(), vec!["a b.png"]);
    }

    #[test]
    fn the_page_is_embedded_but_not_the_game_art() {
        assert!(UiFiles::get("index.html").is_some());
        assert!(UiFiles::get("app.js").is_some());
        assert!(UiFiles::iter().all(|n| !n.contains("-icons")));
        assert!(AssetFiles::get("info-window-labels-1.png").is_some());
    }
}
