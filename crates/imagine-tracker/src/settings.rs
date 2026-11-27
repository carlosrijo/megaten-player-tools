//! The app's own preferences, kept in %AppData%\BABEL\settings.json (C# `AppSettings`, app\AppSettings.cs).
//! Same property names and order as the C# serializer wrote, so an existing file loads and is written back compatibly.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::backend;

fn d_character() -> Option<String> {
    Some(backend::default_name())
}
fn d_csv() -> Option<String> {
    Some(backend::default_csv().to_string_lossy().into_owned())
}
fn d_theme() -> Option<String> {
    Some("system".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// The character whose data was shown last. Characters themselves are listed in data\characters.json.
    #[serde(rename = "ActiveCharacter")]
    pub active_character: Option<String>,
    // From before the app kept several characters: only used once, to list the data that already existed.
    #[serde(rename = "Character")]
    pub character: Option<String>,
    #[serde(rename = "Csv")]
    pub csv: Option<String>,
    /// system | light | dark
    #[serde(rename = "Theme")]
    pub theme: Option<String>,
    /// x, y, width, height of the window when not maximized
    #[serde(rename = "Bounds")]
    pub bounds: Option<Vec<i32>>,
    #[serde(rename = "Maximized")]
    pub maximized: bool,
    /// look for a new version when the app starts (update.rs)
    #[serde(rename = "UpdateCheck")]
    pub update_check: bool,
    /// also offer pre-releases (test versions)
    #[serde(rename = "UpdatePre")]
    pub update_pre: bool,
    /// Set for capture and tour runs: nothing they do is written to the settings file.
    #[serde(skip)]
    pub read_only: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            active_character: None,
            character: d_character(),
            csv: d_csv(),
            theme: d_theme(),
            bounds: None,
            maximized: false,
            update_check: true,
            update_pre: false,
            read_only: false,
        }
    }
}

/// The app's own folder under `base` (%AppData% or %LocalAppData%): `base\BABEL`. Versions up to 0.4.0 used the
/// folder name below; the first run of a newer version moves it, so nobody loses their preferences or page storage.
pub fn app_folder(base: &std::path::Path) -> PathBuf {
    const OLD_NAME: &str = "NewMoonTracker";
    let dir = base.join("BABEL");
    let old = base.join(OLD_NAME);
    if !dir.exists() && old.is_dir() {
        let _ = std::fs::rename(&old, &dir);
    }
    dir
}

impl AppSettings {
    /// %AppData%\BABEL
    pub fn dir() -> PathBuf {
        let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        app_folder(&base)
    }

    pub fn file_path() -> PathBuf {
        Self::dir().join("settings.json")
    }

    pub fn load() -> AppSettings {
        // a damaged settings file just means defaults
        if let Ok(text) = std::fs::read_to_string(Self::file_path()) {
            let text = text.trim_start_matches('\u{feff}');
            if let Ok(s) = serde_json::from_str::<AppSettings>(text) {
                return s;
            }
        }
        AppSettings::default()
    }

    pub fn save(&self) {
        if self.read_only {
            return;
        }
        // preferences are not worth an error box
        let _ = std::fs::create_dir_all(Self::dir());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::file_path(), json);
        }
    }
}
