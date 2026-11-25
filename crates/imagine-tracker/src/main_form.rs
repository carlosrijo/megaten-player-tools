//! What the page asks for and what the window keeps (C# `MainForm`, app\MainForm.cs, without the window itself:
//! that is main.rs / host.rs / winutil.rs).
//!
//! The page asks for things with chrome.webview.postMessage({ id, cmd, args }) and gets { id, ok, result | error }
//! back. The game is only ever touched through the reader's read-only snapshot (backend.rs).
//!
//! Threads: a message from the page is handled on its own worker thread (the C# `await Task.Run`), so the window
//! stays responsive. Shared state sits behind one short-lived lock; reader work is serialized by `calc`.

use anyhow::Result;
use imagine::user_error;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use crate::backend::{self, Backend, CharacterEntry, Characters, ClientInfo, EntryExt, SimRequest, Store};
use crate::character_picture::{self as pic, PicError};
use crate::jsonx;
use crate::options::AppOptions;
use crate::settings::AppSettings;
use crate::site::DATA_HOST;
use crate::ui::Ui;
use crate::{dialogs, support, tour, winutil};

/// Everything that changes while the app runs.
struct State {
    set: AppSettings,
    /// every character with its own folder and CSV
    chars: Characters,
    /// the character whose data is shown
    active: Option<String>,
    /// what the last look at the running clients found
    clients: Vec<ClientInfo>,
    scanned: bool,
    busy: bool,
    captured: bool,
}

/// One look at the running clients, shared by whoever asks while it is under way (C# `_scan`).
struct ScanJob {
    done: Mutex<Option<Result<Vec<ClientInfo>, (String, bool)>>>,
    cv: Condvar,
}

pub struct MainForm {
    pub opt: AppOptions,
    pub data_dir: PathBuf,
    pub ui: Ui,
    /// the window, for the file dialogs (0 until it exists)
    pub hwnd: AtomicIsize,
    st: Mutex<State>,
    backend: Backend,
    /// One piece of reader work at a time: the reference data fills its tables on first use, which two threads must not do at once.
    calc: Mutex<()>,
    scan: Mutex<Option<Arc<ScanJob>>>,
    /// the page's own JavaScript errors, as far as the log takes them (support.rs)
    page_log: support::PageLog,
    /// where the last support bundle was saved, for "Show in folder" (never a path from the page)
    last_bundle: Mutex<Option<PathBuf>>,
    /// the log level of this run, by name
    log_level: String,
    /// where the active character's progress object was found (client pid, address, when), for questProgress
    quest_at: Mutex<Option<(u32, i64, std::time::Instant)>>,
}

impl MainForm {
    /// C# `new MainForm(opt)`; an error (the characters list could not be saved) is what the C# constructor threw.
    pub fn new(opt: AppOptions, ui: Ui) -> Result<MainForm> {
        let data_dir = Self::data_dir_for(&opt);
        // With its own data folder (--data), that folder's config.json replaces the program's own settings before anything
        // reads them, so no spreadsheet path ever comes from the real data folder.
        if opt.data.is_some() && !imagine::data::config::use_data_dir(&data_dir) {
            return Err(anyhow::anyhow!("--data could not be applied: the settings were already loaded."));
        }
        let level_name = imagine::diag::log::Level::parse(opt.log_level.as_deref().unwrap_or("info")).map(|l| l.name()).unwrap_or("info").to_string();
        let mut set = AppSettings::load();
        set.read_only = opt.capture.is_some(); // a capture or tour run never changes the saved preferences
        // With its own data folder (--data), a run keeps its spreadsheets where that folder's config.json says (else in its
        // own stats folder), never in the real place; the saved preference of the real data is not used.
        let first_csv = if opt.data.is_some() {
            backend::default_csv().to_string_lossy().into_owned()
        } else {
            set.csv.clone().unwrap_or_else(|| backend::default_csv().to_string_lossy().into_owned())
        };
        let chars = Characters::new(&data_dir, &set.character.clone().unwrap_or_else(backend::default_name), &first_csv)?;
        let wanted = opt.character.clone().or_else(|| set.active_character.clone()).unwrap_or_default();
        let active = chars.find(&wanted).or_else(|| chars.all.first()).and_then(|e| e.name.clone());
        Ok(MainForm {
            opt,
            data_dir,
            ui,
            hwnd: AtomicIsize::new(0),
            st: Mutex::new(State { set, chars, active, clients: Vec::new(), scanned: false, busy: false, captured: false }),
            backend: Backend::new(),
            calc: Mutex::new(()),
            scan: Mutex::new(None),
            page_log: support::PageLog::new(),
            last_bundle: Mutex::new(None),
            quest_at: Mutex::new(None),
            log_level: level_name,
        })
    }

    /// The data folder of a run: `--data <folder>`, else `data` beside the exe (or the repo).
    pub fn data_dir_for(opt: &AppOptions) -> PathBuf {
        match &opt.data {
            Some(d) => std::path::absolute(d).unwrap_or_else(|_| PathBuf::from(d)),
            None => backend::find_data_dir(),
        }
    }

    fn st(&self) -> MutexGuard<'_, State> {
        self.st.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// A capture or tour run must not change the person's data, unless it was pointed at its own data folder (--data).
    fn no_writes(&self) -> bool {
        self.opt.capture.is_some() && self.opt.data.is_none()
    }

    /// The theme that decides the page's colours at start: the run's --theme, else the saved one.
    pub fn theme(&self) -> Option<String> {
        self.opt.theme.clone().or_else(|| self.st().set.theme.clone())
    }

    // ---------- window ----------

    pub fn saved_bounds(&self) -> (Option<Vec<i32>>, bool) {
        let s = self.st();
        (s.set.bounds.clone(), s.set.maximized)
    }

    /// C# `SaveBounds()` (FormClosing)
    pub fn save_bounds(&self, bounds: Option<([i32; 4], bool)>) {
        if self.opt.capture.is_some() {
            return;
        }
        let Some((b, maximized)) = bounds else { return };
        let mut s = self.st();
        s.set.bounds = Some(b.to_vec());
        s.set.maximized = maximized;
        s.set.save();
    }

    /// The page's address with the run's options (C# `InitAsync`'s query).
    pub fn start_url(&self) -> String {
        let o = &self.opt;
        let mut q: Vec<String> = Vec::new();
        if let Some(p) = &o.page {
            q.push(format!("page={}", jsonx::escape_data_string(p)));
        }
        if let Some(t) = &o.theme {
            q.push(format!("theme={}", jsonx::escape_data_string(t)));
        }
        if let Some(m) = &o.mode {
            q.push(format!("mode={}", jsonx::escape_data_string(m)));
        }
        if let Some(extra) = o.query.as_deref().filter(|s| !s.is_empty()) {
            q.push(extra.trim_start_matches(['?', '&']).to_string());
        }
        // Pictures of the page are taken without the assistant in the corner, unless the run asks for it (--query assistant=1)
        if o.capture.is_some() && !o.query.as_deref().unwrap_or("").contains("assistant=") {
            q.push("assistant=0".to_string());
        }
        format!("https://{}/index.html{}", crate::site::HOST, if q.is_empty() { String::new() } else { format!("?{}", q.join("&")) })
    }

    // ---------- the page's messages ----------

    /// A message arrived from the page (on the UI thread): handle it on a worker thread and post the answer.
    pub fn on_message(self: &Arc<Self>, json: String, source: String) {
        // Only the app's own page may ask for things (navigation is already limited to it; this is a second lock).
        let prefix = format!("https://{}/", crate::site::HOST);
        if source.len() < prefix.len() || !source.is_char_boundary(prefix.len()) || !source[..prefix.len()].eq_ignore_ascii_case(&prefix) {
            return;
        }
        let Ok(m) = serde_json::from_str::<Value>(&json) else { return };
        if !m.is_object() {
            return;
        }
        let id = jsonx::int(m.get("id"), 0);
        let cmd = jsonx::str(m.get("cmd")).unwrap_or_default();
        let args = m.get("args").cloned().unwrap_or(Value::Null);
        let me = self.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| me.dispatch(&cmd, &args)));
            log_command(&cmd, started.elapsed().as_millis(), &r);
            let reply = match r {
                Ok(Ok(result)) => format!("{{\"id\":{id},\"ok\":true,\"result\":{result}}}"),
                Ok(Err(e)) => fail(id, &e.to_string(), if imagine::error::is_user_error(&e) { "user" } else { "error" }),
                Err(_) => fail(id, "The request failed unexpectedly.", "error"),
            };
            me.ui.post(reply);
        });
    }

    fn active_entry(&self) -> Option<CharacterEntry> {
        let s = self.st();
        s.active.as_ref().and_then(|n| s.chars.find(n)).cloned()
    }

    fn require_active(&self, msg: &str) -> Result<CharacterEntry> {
        self.active_entry().ok_or_else(|| user_error(msg))
    }

    fn char_dir(&self, e: &CharacterEntry) -> PathBuf {
        self.st().chars.data_dir(e)
    }

    fn dispatch(self: &Arc<Self>, cmd: &str, args: &Value) -> Result<String> {
        match cmd {
            "load" => self.load_data(),
            "state" => Ok(self.state_json()),

            "status" => {
                let pids = backend::client_pids();
                let s = self.st();
                let known: HashSet<u32> = s.clients.iter().map(|c| c.pid).collect();
                Ok(json!({ "gameRunning": !pids.is_empty(), "busy": s.busy, "clientsChanged": s.scanned && pids != known }).to_string())
            }

            "clients" => {
                // Read-only look at every running client: who is logged in where. About a second per client.
                // keepKnown (the page's background check): only clients with no character found yet are looked at.
                let keep_known = args.get("keepKnown").and_then(Value::as_bool).unwrap_or(false);
                let list = self.scan_clients(keep_known)?;
                {
                    let mut s = self.st();
                    s.clients = list;
                    s.scanned = true;
                }
                Ok(self.state_json())
            }

            "selectCharacter" => {
                let name = jsonx::str(args.get("name")).unwrap_or_default();
                let found = self.st().chars.find(&name).cloned();
                let e = match found {
                    Some(e) => e,
                    None => {
                        // A new character is only ever added from what a running client reports, never from the page's say-so.
                        let known = self.st().clients.iter().find(|c| c.character.as_deref().is_some_and(|n| imagine::text::eq_ic(n, &name))).cloned();
                        let Some(c) = known else {
                            return Err(user_error("That character isn't known yet. Log in with it, then look again."));
                        };
                        if self.no_writes() {
                            return Err(user_error("Not adding a new character during a capture run."));
                        }
                        self.st().chars.get_or_add(c.character.as_deref().unwrap_or(&name))?.clone()
                    }
                };
                {
                    let mut s = self.st();
                    s.active = Some(e.name().to_string());
                    if self.opt.capture.is_none() {
                        s.set.active_character = Some(e.name().to_string());
                        s.set.save();
                    }
                }
                Ok(self.state_json())
            }

            "snapshot" => {
                let e = self.require_active("No character is selected yet. Log in to the game, then pick the character.")?;
                let dir = self.char_dir(&e);
                self.guarded(|| {
                    let csv = e.csv.clone().unwrap_or_default();
                    let mut pictured = false;
                    let r = {
                        // One copy of the game's memory for the client lookup below and the whole snapshot: when the
                        // lookup is over a minute old it sweeps the client, and that sweep is then the snapshot's
                        // (it was a separate ~0.65 s one). Let go before the rebuild.
                        let _shared = imagine::memory::share_sweeps();
                        // While the Information window is open, its character view becomes this character's picture.
                        // Done first (the window is certainly still open) and never allowed to fail the snapshot.
                        if e.auto_picture != Some(false) && !self.no_writes() {
                            if let Ok(game) = self.backend.find_client(e.name()) {
                                if let Ok(true) = pic::from_info_window(game.main_window_handle(), &dir) {
                                    pictured = true;
                                }
                            }
                        }
                        self.backend.take_snapshot(e.name(), &dir, &csv)?
                    };
                    self.backend.rebuild(&dir, &csv)?;
                    Ok(json!({
                        "source": r.source, "statsNote": r.stats_note, "missing": r.missing.unwrap_or_default(),
                        "level": r.level, "ranks": r.ranks, "picture": pictured,
                    })
                    .to_string())
                })
            }

            "rebuild" => {
                let e = self.require_active("No character is selected yet.")?;
                let dir = self.char_dir(&e);
                self.guarded(|| {
                    let snapshots = self.backend.rebuild(&dir, &e.csv.clone().unwrap_or_default())?;
                    Ok(json!({ "snapshots": snapshots }).to_string())
                })
            }

            "setAlignment" => {
                // The game can't be asked for alignment, so the person sets it. A change recalculates the saved snapshots
                // (the spreadsheet is copied first) - see Pipeline.SetAlignment.
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not changing alignment during a capture run."));
                }
                let value = jsonx::str(args.get("alignment"));
                let dir = self.char_dir(&e);
                self.guarded(|| {
                    let r = self.backend.set_alignment(&dir, &e.csv.clone().unwrap_or_default(), value.as_deref())?;
                    let backup = r.backup.as_ref().and_then(|b| b.file_name()).map(|f| f.to_string_lossy().into_owned());
                    Ok(json!({ "changed": r.changed, "rows": r.rows, "changedRows": r.changed_rows, "backup": backup }).to_string())
                })
            }

            // ---- the digitalize demon: pick any COMP demon (Pipeline.CompDemons / UseDemon) ----
            "compDemons" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Ok("[]".to_string());
                }
                // "cached": the demons seen before, without looking at the game (the page shows those first, then asks again)
                let scan = !jsonx::truthy(args.get("cached"));
                let dir = self.char_dir(&e);
                self.guarded(|| Ok(self.backend.comp_demons(e.name(), &dir, scan)?.to_string()))
            }

            "useDemon" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not changing the digitalize demon during a capture run."));
                }
                let key = jsonx::str(args.get("key"));
                let dir = self.char_dir(&e);
                // from what is cached: the game is only read by a snapshot or by "Load demons from the game" (compDemons)
                self.guarded(|| Ok(self.backend.use_demon(e.name(), &dir, &e.csv.clone().unwrap_or_default(), key.as_deref())?.to_string()))
            }

            "forgetDemon" => {
                // take a demon off the picker's list (cache\comp-demons.json); not the one in use
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not changing the demon list during a capture run."));
                }
                let key = jsonx::str(args.get("key")).unwrap_or_default();
                let dir = self.char_dir(&e);
                self.guarded(|| {
                    self.backend.forget_demon(&dir, &key)?;
                    Ok("true".to_string())
                })
            }

            "deleteSnapshot" => {
                // one earlier snapshot out of the spreadsheet (copied first), then the page data again
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not deleting snapshots during a capture run."));
                }
                let taken_at = jsonx::str(args.get("takenAt")).unwrap_or_default();
                let dir = self.char_dir(&e);
                self.guarded(|| Ok(self.backend.delete_snapshot(&dir, &e.csv.clone().unwrap_or_default(), &taken_at)?.to_string()))
            }

            // ---- character picture: photo of the game window or an image file, cropped in the page ----
            "pictureSource" => {
                let e = self.active_entry();
                let size = e.as_ref().and_then(|e| pic::source_size(&self.char_dir(e)));
                Ok(self.picture_source_json(e.as_ref(), size, None))
            }

            "pictureCapture" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not taking pictures during a capture run."));
                }
                let dir = self.char_dir(&e);
                let game = self.backend.find_client(e.name())?; // the client this character is logged in to
                let (size, view) = pic::capture_window(game.main_window_handle(), &dir).map_err(pic_to_user)?;
                Ok(self.picture_source_json(Some(&e), Some(size), view))
            }

            "pictureFile" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not choosing pictures during a capture run."));
                }
                let owner = winutil::hwnd_of(self.hwnd.load(Ordering::Relaxed));
                let Some(file) = dialogs::open_image(owner, &format!("Picture for {}", e.name())) else { return Ok("null".to_string()) };
                match pic::use_file(&file, &self.char_dir(&e)) {
                    Ok(size) => Ok(self.picture_source_json(Some(&e), Some(size), None)),
                    Err(_) => Err(user_error("That file couldn't be opened as a picture.")),
                }
            }

            "pictureCrop" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not saving pictures during a capture run."));
                }
                let n = |k: &str| jsonx::int(args.get(k), 0);
                pic::crop(&self.char_dir(&e), n("x"), n("y"), n("w"), n("h")).map_err(pic_to_user)?;
                // A picture chosen by hand stays: snapshots stop replacing it until the person turns that back on.
                if e.auto_picture != Some(false) {
                    let mut s = self.st();
                    if let Some(m) = s.chars.find_mut(e.name()) {
                        m.auto_picture = Some(false);
                    }
                    s.chars.save()?;
                }
                Ok(self.state_json())
            }

            "pictureAuto" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Ok(self.state_json());
                }
                {
                    let mut s = self.st();
                    if let Some(m) = s.chars.find_mut(e.name()) {
                        m.auto_picture = Some(jsonx::truthy(args.get("on")));
                    }
                    s.chars.save()?;
                }
                Ok(self.state_json())
            }

            "pictureRemove" => {
                let e = self.require_active("No character is selected yet.")?;
                if self.no_writes() {
                    return Err(user_error("Not removing pictures during a capture run."));
                }
                pic::remove(&self.char_dir(&e))?;
                Ok(self.state_json())
            }

            "getSettings" => Ok(self.settings_json()),

            "saveSettings" => {
                let theme = jsonx::str(args.get("theme"));
                {
                    let mut s = self.st();
                    if let Some(t) = theme.as_deref().filter(|t| matches!(*t, "system" | "light" | "dark")) {
                        s.set.theme = Some(t.to_string());
                        self.ui.set_scheme(Some(t.to_string()));
                    }
                    s.set.save();
                }
                Ok(self.settings_json())
            }

            "pickCsv" => {
                let Some(e) = self.active_entry() else { return Ok(self.settings_json()) };
                let csv = e.csv.clone().unwrap_or_default();
                let parent = Path::new(&csv).parent().filter(|p| p.is_dir()).map(|p| p.to_path_buf());
                let file_name = Path::new(&csv).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                let owner = winutil::hwnd_of(self.hwnd.load(Ordering::Relaxed));
                let picked = dialogs::save_csv(owner, &format!("Spreadsheet file for {}'s snapshots", e.name()), &file_name, parent.as_deref());
                let Some(picked) = picked else { return Ok(self.settings_json()) };
                {
                    let mut s = self.st();
                    if let Some(m) = s.chars.find_mut(e.name()) {
                        m.csv = Some(picked.to_string_lossy().into_owned());
                    }
                    s.chars.save()?;
                }
                Ok(self.settings_json())
            }

            "reveal" => {
                // Only the places the app itself uses; never a path from the page.
                let e = self.active_entry();
                let what = jsonx::str(args.get("what"));
                let dir = match &e {
                    Some(e) => self.char_dir(e),
                    None => self.data_dir.clone(),
                };
                let csv = e.as_ref().and_then(|e| e.csv.clone());
                let csv_parent = csv.as_deref().and_then(|c| Path::new(c).parent().map(|p| p.to_path_buf()));
                let is_csv = what.as_deref() == Some("csv");
                let is_data = what.as_deref() == Some("data");
                let last_bundle = self.last_bundle.lock().unwrap_or_else(|p| p.into_inner()).clone();
                if what.as_deref() == Some("logs") {
                    let logs = self.data_dir.join("logs");
                    if logs.is_dir() {
                        winutil::explorer(&format!("\"{}\"", logs.display()));
                    } else if self.data_dir.is_dir() {
                        winutil::explorer(&format!("\"{}\"", self.data_dir.display()));
                    }
                } else if what.as_deref() == Some("bundle") {
                    // only the bundle this run saved
                    if let Some(b) = last_bundle.filter(|b| b.is_file()) {
                        winutil::explorer(&format!("/select,\"{}\"", b.display()));
                    }
                } else if is_csv && csv.as_deref().is_some_and(|c| Path::new(c).is_file()) {
                    winutil::explorer(&format!("/select,\"{}\"", csv.unwrap_or_default()));
                } else if is_csv && csv_parent.as_deref().is_some_and(|p| p.is_dir()) {
                    winutil::explorer(&format!("\"{}\"", csv_parent.unwrap_or_default().display()));
                } else if is_data && dir.is_dir() {
                    winutil::explorer(&format!("\"{}\"", dir.display()));
                } else if is_data && self.data_dir.is_dir() {
                    winutil::explorer(&format!("\"{}\"", self.data_dir.display()));
                }
                Ok("true".to_string())
            }

            "openIssue" => {
                // The issue the feedback form just opened, in the default browser. Only its number comes from the page;
                // the address is always the project's own issues.
                let n = jsonx::num(args.get("number")).filter(|n| *n >= 1.0 && *n < 1e9 && n.fract() == 0.0);
                if let Some(n) = n {
                    winutil::explorer(&format!("\"https://github.com/carlosrijo/megaten-player-tools/issues/{}\"", n as u64));
                }
                Ok("true".to_string())
            }

            "assistant" => {
                // The demon in the page's corner: its 3D model as one JSON text, made from the client's own files the
                // first time (Build\DemonModel.cs) and kept in <data>\cache\models. "null" while the game's folder
                // isn't known (never seen running) or the model can't be read.
                // { demon: <id> } asks for another demon's model (the assistant picker, a feature still behind a flag):
                // read from the game's files on the fly and only kept in memory, never written to cache\models
                // (only the default assistant's file is kept there). Its size, summoning circle size and colour come along as "summon".
                let demon = jsonx::int(args.get("demon"), 0);
                let picked = if demon > 0 { imagine::build::demon_model::model_of_demon(demon)? } else { None };
                if demon > 0 && picked.is_none() {
                    return Ok("null".to_string());
                }
                let model = picked.as_ref().map(|p| p.model.clone()).unwrap_or_else(|| imagine::build::demon_model::ASSISTANT.to_string());
                let e = self.active_entry();
                let save = !self.no_writes() && picked.is_none();
                let attempt = || -> Result<Option<String>> {
                    let mut client = match &e {
                        Some(e) => self.backend.item_icons_client_dir(&self.char_dir(e))?,
                        None => None,
                    };
                    if client.is_none() {
                        client = self.backend.item_icons_client_dir(&self.data_dir)?;
                    }
                    // ClientDir remembers "not found" for as long as the app runs: look once more, the game may have been started since
                    if client.is_none() {
                        for pid in backend::client_pids() {
                            if client.is_none() {
                                client = backend::process_exe_dir(pid);
                            }
                        }
                    }
                    self.backend.demon_model_load(&self.data_dir, client.as_deref(), &model, save)
                };
                match attempt() {
                    Ok(Some(json)) => {
                        // the model's JSON object with the game's model numbers put in front
                        let r = picked.unwrap_or_else(|| imagine::build::demon_model::model_of_demon(imagine::build::demon_model::ASSISTANT_DEMON).ok().flatten().unwrap_or(imagine::build::demon_model::DemonModelRef {
                            model: model.clone(),
                            scale: 1.0,
                            circle: 1.0,
                            color: [162, 70, 243],
                        }));
                        let head = format!("{{\"demon\":{demon},\"summon\":{{\"scale\":{},\"circle\":{},\"color\":{}}},", json!(r.scale), json!(r.circle), json!(r.color));
                        Ok(match json.strip_prefix('{') {
                            Some(rest) if rest.trim_start() != "}" => format!("{head}{rest}"),
                            _ => json,
                        })
                    }
                    Ok(None) => Ok("null".to_string()),
                    // C# catches NifException, IOException and UnauthorizedAccessException
                    Err(err) if err.downcast_ref::<std::io::Error>().is_some() || imagine::data::nif::is_nif_error(&err) => Ok("null".to_string()),
                    Err(err) => Err(err),
                }
            }

            "summonedDemon" => {
                // The active character's summoned demon right now, read from its client (read-only, one pass over the
                // demon records, ~30 ms): { type, name } or { type: 0 } when none is out. "null" when it can't be told
                // (no character, or its client isn't running). For the assistant picker's "same as my summoned demon"
                // (a feature behind a flag); the page asks every few seconds, so it is logged quietly.
                let Some(name) = self.active_entry().and_then(|e| e.name.clone()) else { return Ok("null".to_string()) };
                let proc = match imagine::game::client_reader::find_client(&name) {
                    Ok(p) => p,
                    Err(e) if imagine::error::is_user_error(&e) => return Ok("null".to_string()),
                    Err(e) => return Err(e),
                };
                let mem = imagine::memory::GameMemory::new(proc.id())?;
                // another player's demon in view has a record too, but no details: the partner is the loaded one
                let all = imagine::game::demon_reader::read_demons(&mem, true);
                Ok(match all.iter().find(|d| d.summoned() && d.loaded()) {
                    Some(d) => json!({ "type": d.r#type, "name": d.name }),
                    None => json!({ "type": 0 }),
                }
                .to_string())
            }

            "questProgress" => {
                // The active character's quests right now, read from its client (read-only, one pass for its progress
                // object): { active: [{ id, step, counters }] (step 0-based, counters one per objective), completed:
                // [ids] or null (the client only has that list once Quest Completion Bonuses has been opened), flags:
                // [valuable ids] or null (the game's hidden quest flags among them), zone: the zone id the character is
                // in, or null }. "null" when it can't be told (no
                // character, its client isn't running, or it isn't logged in). The Quest Guide asks every few seconds
                // while it is open, so it is logged quietly.
                let Some(name) = self.active_entry().and_then(|e| e.name.clone()) else { return Ok("null".to_string()) };
                let proc = match imagine::game::client_reader::find_client(&name) {
                    Ok(p) => p,
                    Err(e) if imagine::error::is_user_error(&e) => return Ok("null".to_string()),
                    Err(e) => return Err(e),
                };
                use imagine::game::demon_reader as dr;
                let mem = imagine::memory::GameMemory::new(proc.id())?;
                // finding the progress object takes a pass over the game's memory (~400 ms); reading it again ~1 ms, so
                // where it is is kept for this client until it stops being one (logout, another character), and looked
                // up again each minute: a freed object keeps its vtable, so a stale address could still look right
                let pid = proc.id();
                let known = self.quest_at.lock().unwrap().filter(|(p, _, t)| *p == pid && t.elapsed().as_secs() < 60).map(|(_, at, _)| at);
                let mut p = known.and_then(|at| dr::read_quest_state(&mem, at));
                if p.is_none() {
                    let at = dr::find_progress_at(&mem);
                    *self.quest_at.lock().unwrap() = at.map(|a| (pid, a, std::time::Instant::now()));
                    p = at.and_then(|a| dr::read_quest_state(&mem, a));
                }
                let Some(p) = p else { return Ok("null".to_string()) };
                let active = p.active.as_ref().map(|v| v.iter().map(|q| json!({ "id": q.id, "step": q.step, "counters": q.counters })).collect::<Vec<_>>());
                let completed = p.completed.as_ref().map(|s| s.iter().copied().collect::<Vec<_>>());
                let flags = p.valuables.as_ref().map(|s| s.iter().copied().collect::<Vec<_>>());
                Ok(json!({ "active": active, "completed": completed, "flags": flags, "zone": p.zone }).to_string())
            }

            "assistantSummon" => {
                // The game's summoning effect pieces (pictures from its effect models, its summoning sound) for the
                // assistant picker's change of demon (a feature behind a flag). Only read; "null" while the game's
                // folder isn't known.
                let e = self.active_entry();
                let mut client = match &e {
                    Some(e) => self.backend.item_icons_client_dir(&self.char_dir(e))?,
                    None => None,
                };
                if client.is_none() {
                    client = self.backend.item_icons_client_dir(&self.data_dir)?;
                }
                if client.is_none() {
                    client = backend::client_pids().into_iter().find_map(backend::process_exe_dir);
                }
                match client.map(|d| imagine::build::demon_model::summon_effect(&d)).transpose() {
                    Ok(Some(Some(json))) => Ok(json),
                    Ok(_) => Ok("null".to_string()),
                    Err(err) if err.downcast_ref::<std::io::Error>().is_some() => Ok("null".to_string()),
                    Err(err) => Err(err),
                }
            }

            "menuIcons" => {
                // The game's menu pictures (bar buttons PC, DEVIL, QUEST ... and some entries) for the first-snapshot
                // steps, cut from the client's own Interface\tga\menu_icon_00.tga. Only read; {} while the game's
                // folder isn't known.
                let e = self.active_entry();
                let mut client = match &e {
                    Some(e) => self.backend.item_icons_client_dir(&self.char_dir(e))?,
                    None => None,
                };
                if client.is_none() {
                    client = self.backend.item_icons_client_dir(&self.data_dir)?;
                }
                if client.is_none() {
                    client = backend::client_pids().into_iter().find_map(backend::process_exe_dir);
                }
                let icons: serde_json::Map<String, Value> =
                    client.map(|d| imagine::build::item_icons::menu_icons(&d)).unwrap_or_default().into_iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
                Ok(Value::Object(icons).to_string())
            }

            "theme" => {
                self.ui.apply_chrome(jsonx::truthy(args.get("dark")));
                Ok("true".to_string())
            }

            "ready" => {
                let start = {
                    let mut s = self.st();
                    if !s.captured && (self.opt.tour.is_some() || self.opt.capture.is_some()) {
                        s.captured = true;
                        true
                    } else {
                        false
                    }
                };
                if start {
                    let me = self.clone();
                    std::thread::spawn(move || {
                        if me.opt.tour.is_some() {
                            if me.opt.live { tour::live_tour_and_exit(&me) } else { tour::tour_and_exit(&me) }
                        } else {
                            tour::capture_and_exit(&me)
                        }
                    });
                }
                Ok("true".to_string())
            }

            // ---- the Simulator tool: what another build would give (Build\Simulator.cs). Reads saved files only, writes
            //      nothing, so it is not "busy" work: it just waits its turn behind a snapshot. ----
            "simInit" | "simulate" | "simCatalog" | "simDemons" | "simBuilds" | "simSaveBuild" | "simDeleteBuild" | "simShareCode" | "simImportCode" => {
                let e = self.require_active("No character is selected yet. Log in to the game, then pick the character.")?;
                if self.no_writes() && matches!(cmd, "simSaveBuild" | "simDeleteBuild") {
                    return Err(user_error("Not saving builds during a capture run."));
                }
                let (characters, owner, active_dir) = {
                    let s = self.st();
                    let characters: Vec<(String, PathBuf)> = s.chars.all.iter().map(|c| (c.name().to_string(), s.chars.data_dir(c))).collect();
                    // builds saved before they had bases were the first character's (the one whose folder is the data folder)
                    let owner = s
                        .chars
                        .all
                        .iter()
                        .find(|c| matches!(c.dir.as_deref(), None | Some("") | Some(".")))
                        .map(|c| c.name().to_string())
                        .unwrap_or_else(|| e.name().to_string());
                    (characters, owner, s.chars.data_dir(&e))
                };
                let skill_icons = |who: &str| -> Option<String> {
                    let ce = self.st().chars.find(who).cloned()?;
                    self.folder_url(&ce, &Path::new("dash").join("skill-icons"))
                };
                let _calc = self.calc.lock().unwrap_or_else(|p| p.into_inner());
                self.backend.sim_command(&SimRequest {
                    default_base: e.name(),
                    data_dir: &self.data_dir,
                    characters: &characters,
                    active_dir: &active_dir,
                    builds_path: self.data_dir.join("sim-builds.json"),
                    cmd,
                    args,
                    owner: &owner,
                    skill_icons: &skill_icons,
                })
            }

            // ---- troubleshooting (support.rs): the version block, the self-check, the support bundle ----
            "about" => Ok(support::about_json(&self.data_dir, &self.log_level).to_string()),

            // one pass over the files, the game and each reader; one piece of reader work at a time
            "doctor" => {
                let no_writes = self.no_writes();
                self.guarded(|| Ok(support::run_doctor(&self.data_dir, no_writes).to_json().to_string()))
            }

            "saveBundle" => {
                if self.no_writes() {
                    return Err(user_error("Not saving a support bundle during a capture run."));
                }
                let include_names = support::bundle_request(args)?;
                let out = match &self.opt.capture {
                    // a scripted run (tests) has no one to answer a dialog: the bundle goes beside its pictures
                    Some(c) => {
                        let c = Path::new(c);
                        let dir = if self.opt.tour.is_some() { c.to_path_buf() } else { c.parent().map(Path::to_path_buf).unwrap_or_default() };
                        std::fs::create_dir_all(&dir)?;
                        dir.join("support-bundle.zip")
                    }
                    None => {
                        let owner = winutil::hwnd_of(self.hwnd.load(Ordering::Relaxed));
                        match dialogs::save_zip(owner, "Save a support bundle", &imagine::build::bundle::default_name(), dialogs::desktop_dir().as_deref()) {
                            Some(p) => p,
                            None => return Ok("null".to_string()), // cancelled
                        }
                    }
                };
                let result = self.guarded(|| Ok(support::save_bundle(&self.data_dir, include_names, &out)?.to_string()))?;
                *self.last_bundle.lock().unwrap_or_else(|p| p.into_inner()) = Some(out);
                Ok(result)
            }

            // the support bundle's files as text, for the feedback form to attach (it encrypts them; nothing is written)
            "feedbackFiles" => {
                let include_names = support::bundle_request(args)?;
                self.guarded(|| Ok(support::feedback_files(&self.data_dir, include_names).to_string()))
            }

            // is each running client the build the reader was made for (the page warns, once, when not)
            "buildCheck" => Ok(support::build_check().to_string()),

            // the page's JavaScript errors; a capture run on real data writes nothing, not even a log line
            "log" => {
                if !self.no_writes() {
                    self.page_log.record(args);
                }
                Ok("true".to_string())
            }

            // test builds only: a panic on a page-command thread, to look at the crash report
            #[cfg(any(debug_assertions, feature = "crash-test"))]
            "debugPanic" => panic!("deliberate test panic (debugPanic)"),

            _ => Err(user_error(format!("Unknown request '{cmd}'."))),
        }
    }

    /// Run one piece of reader work; only one at a time.
    fn guarded(&self, work: impl FnOnce() -> Result<String>) -> Result<String> {
        {
            let mut s = self.st();
            if s.busy {
                return Err(user_error("Still working on the last request. Give it a few seconds."));
            }
            s.busy = true;
        }
        struct Reset<'a>(&'a MainForm);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.st().busy = false;
            }
        }
        let _reset = Reset(self);
        let _calc = self.calc.lock().unwrap_or_else(|p| p.into_inner());
        work()
    }

    /// C# `_scan is { IsCompleted: false } ? _scan : Task.Run(ClientReader.ListClients)`, awaited.
    /// A scan already running is joined whatever `keep_known` it was started with.
    fn scan_clients(self: &Arc<Self>, keep_known: bool) -> Result<Vec<ClientInfo>> {
        let job = {
            let mut slot = self.scan.lock().unwrap_or_else(|p| p.into_inner());
            match slot.as_ref() {
                Some(j) if j.done.lock().unwrap_or_else(|p| p.into_inner()).is_none() => j.clone(),
                _ => {
                    let j = Arc::new(ScanJob { done: Mutex::new(None), cv: Condvar::new() });
                    *slot = Some(j.clone());
                    let (me, jj) = (self.clone(), j.clone());
                    std::thread::spawn(move || {
                        let r = me.backend.list_clients(keep_known).map_err(|e| (e.to_string(), imagine::error::is_user_error(&e)));
                        *jj.done.lock().unwrap_or_else(|p| p.into_inner()) = Some(r);
                        jj.cv.notify_all();
                    });
                    j
                }
            }
        };
        let mut done = job.done.lock().unwrap_or_else(|p| p.into_inner());
        while done.is_none() {
            done = job.cv.wait(done).unwrap_or_else(|p| p.into_inner());
        }
        match done.as_ref().expect("finished") {
            Ok(list) => Ok(list.clone()),
            Err((msg, true)) => Err(user_error(msg.clone())),
            Err((msg, false)) => Err(anyhow::anyhow!("{msg}")),
        }
    }

    // ---------- characters ----------

    /// Address of one of a character's own folders for the page, ending in "/" (None when the folder doesn't exist).
    fn folder_url(&self, e: &CharacterEntry, folder: &Path) -> Option<String> {
        let path = self.char_dir(e).join(folder);
        if !path.is_dir() {
            return None;
        }
        let rel: Vec<String> = relative_path(&self.data_dir, &path).iter().map(|s| jsonx::escape_data_string(s)).collect();
        Some(format!("https://{DATA_HOST}/{}/", rel.join("/")))
    }

    /// Address the page can show one of a character's own files at (None when the file doesn't exist).
    fn file_url(&self, e: &CharacterEntry, file: &Path) -> Option<String> {
        let path = self.char_dir(e).join(file);
        let meta = std::fs::metadata(&path).ok().filter(|m| m.is_file())?;
        let rel: Vec<String> = relative_path(&self.data_dir, &path).iter().map(|s| jsonx::escape_data_string(s)).collect();
        // ?v= so a new picture is never shown from cache: the file's last write time in .NET ticks (100 ns since 0001-01-01)
        let ticks = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| 621_355_968_000_000_000i64 + (d.as_nanos() / 100) as i64)
            .unwrap_or(0);
        Some(format!("https://{DATA_HOST}/{}?v={ticks}", rel.join("/")))
    }

    /// The picture to crop, and (after a capture with the Information window open) where its character view is.
    fn picture_source_json(&self, e: Option<&CharacterEntry>, size: Option<(i32, i32)>, view: Option<pic::Rect>) -> String {
        let (Some(e), Some((w, h))) = (e, size) else { return "null".to_string() };
        json!({
            "url": self.file_url(e, Path::new(pic::SOURCE_FILE)), "width": w, "height": h,
            "view": view.map(|v| json!({ "x": v.x, "y": v.y, "w": v.w, "h": v.h })),
        })
        .to_string()
    }

    /// Which character is shown, every character with saved data, and what the last look at the clients found.
    fn state_json(&self) -> String {
        let (active, scanned, clients, entries) = {
            let s = self.st();
            (s.active.clone(), s.scanned, s.clients.clone(), s.chars.all.clone())
        };
        let in_game: Vec<&ClientInfo> = clients.iter().filter(|c| c.character.is_some()).collect();
        let characters: Vec<Value> = entries
            .iter()
            .map(|e| {
                let g = in_game.iter().find(|c| c.character.as_deref().is_some_and(|n| imagine::text::eq_ic(n, e.name())));
                let has_data = self.char_dir(e).join("dash").join("snapshots.json").is_file();
                json!({
                    "name": e.name, "hasData": has_data, "inGame": g.is_some(),
                    "folder": g.map(|g| g.folder.clone()).unwrap_or_default(), "level": g.map(|g| g.level).unwrap_or(0),
                    "picture": self.file_url(e, Path::new(pic::PICTURE_FILE)), "autoPicture": e.auto_picture != Some(false),
                })
            })
            .collect();
        json!({
            "active": active, "scanned": scanned,
            "clients": clients.iter().map(|c| json!({ "pid": c.pid, "folder": c.folder, "character": c.character, "level": c.level })).collect::<Vec<_>>(),
            "characters": characters,
        })
        .to_string()
    }

    fn settings_json(&self) -> String {
        let e = self.active_entry();
        let theme = self.st().set.theme.clone();
        let csv = e.as_ref().and_then(|e| e.csv.clone()).unwrap_or_default();
        json!({
            "character": e.as_ref().and_then(|e| e.name.clone()),
            "csv": csv,
            "theme": theme,
            "dataDir": match &e { Some(e) => self.char_dir(e), None => self.data_dir.clone() }.to_string_lossy(),
            "csvExists": e.is_some() && Path::new(&csv).is_file(),
        })
        .to_string()
    }

    /// Everything the page shows for the selected character: the four data files the reader builds, the expertise ranks, and a little context.
    fn load_data(&self) -> Result<String> {
        let e = self.active_entry();
        let store = Store::new(match &e {
            Some(e) => self.char_dir(e),
            None => self.data_dir.join("characters").join("_none"),
        });
        let dash = |name: &str| raw_json(&store.dash_dir().join(format!("{name}.json")), "[]");
        let meta = json!({
            "character": e.as_ref().and_then(|e| e.name.clone()),
            "alignment": if e.is_some() { store.alignment()? } else { String::new() },
            "alignmentKnown": e.is_some() && store.profile_path().is_file(),
            // when the toggle (switch) skills' on/off state was read ("game, <date time>"), for the Bonuses footnote
            "togglesSource": if e.is_some() { store.toggles_source().ok().flatten() } else { None },
            "csv": e.as_ref().and_then(|e| e.csv.clone()).unwrap_or_default(),
            "dataDir": store.dir.to_string_lossy(),
            "gameRunning": !backend::client_pids().is_empty(),
            "picture": e.as_ref().and_then(|e| self.file_url(e, Path::new(pic::PICTURE_FILE))),
            "autoPicture": e.as_ref().map(|e| e.auto_picture != Some(false)).unwrap_or(true),
            "demonPicture": e.as_ref().and_then(|e| self.file_url(e, &Path::new("dash").join("demon.png"))),
            "skillIcons": e.as_ref().and_then(|e| self.folder_url(e, &Path::new("dash").join("skill-icons"))),
        });
        let dd = store.dash_dir();
        Ok(format!(
            "{{\"snapshots\":{},\"gear\":{},\"expertise\":{},\"bonuses\":{},\"ranks\":{},\"digitalize\":{},\"unlocks\":{},\"summoned\":{},\"icons\":{},\"meta\":{}}}",
            dash("snapshots"),
            dash("gear"),
            dash("expertise"),
            dash("bonuses"),
            raw_json(&store.expertise_ranks_path(), "null"),
            raw_json(&dd.join("digitalize.json"), "null"),
            raw_json(&dd.join("digi-unlocks.json"), "[]"),
            raw_json(&dd.join("summoned.json"), "null"),
            raw_json(&dd.join("icons.json"), "{}"),
            meta
        ))
    }
}

/// One of the data files as it is, or the fallback when it is missing, empty or not JSON ("never hand the page
/// half-written JSON").
fn raw_json(path: &Path, fallback: &str) -> String {
    let Ok(text) = std::fs::read_to_string(path) else { return fallback.to_string() };
    let t = text.trim_start_matches('\u{feff}').trim();
    if t.is_empty() {
        return fallback.to_string();
    }
    match serde_json::from_str::<serde::de::IgnoredAny>(t) {
        Ok(_) => t.to_string(),
        Err(_) => fallback.to_string(),
    }
}

/// One line in the log per page command: its name, how long it took and how it went (a message for the person is a
/// "user error"). What the page asked with (the arguments) is never logged: it can hold personal data. The page's
/// status poll every few seconds is only logged at debug level (unless it fails).
fn log_command(cmd: &str, ms: u128, r: &std::thread::Result<Result<String>>) {
    use imagine::diag::log::{self, Level};
    // the name comes from the page: keep to what a command name looks like
    let name: String = cmd.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(32).collect();
    let name = if name.is_empty() { "?".to_string() } else { name };
    // the page's own error reports are not commands worth a line each
    if name == "log" {
        return;
    }
    let quiet = matches!(name.as_str(), "status" | "summonedDemon" | "questProgress");
    match r {
        Ok(Ok(_)) => log::write(if quiet { Level::Debug } else { Level::Info }, &format!("command {name}: ok ({ms} ms)")),
        Ok(Err(e)) if imagine::error::is_user_error(e) => log::write(Level::Info, &format!("command {name}: user error: {e} ({ms} ms)")),
        Ok(Err(e)) => log::write(Level::Error, &format!("command {name}: error: {e:#} ({ms} ms)")),
        Err(_) => log::write(Level::Error, &format!("command {name}: panicked ({ms} ms); see the crash report next to the log")),
    }
}

fn fail(id: i32, message: &str, kind: &str) -> String {
    json!({ "id": id, "ok": false, "error": message, "kind": kind }).to_string()
}

fn pic_to_user(e: PicError) -> anyhow::Error {
    match e {
        PicError::Invalid(m) => user_error(m),
        PicError::Other(e) => e,
    }
}

/// `Path.GetRelativePath(from, to)` split into its segments.
fn relative_path(from: &Path, to: &Path) -> Vec<String> {
    if let Ok(rest) = to.strip_prefix(from) {
        return rest.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    }
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(b.iter()).take_while(|(x, y)| x.as_os_str().eq_ignore_ascii_case(y.as_os_str())).count();
    let mut out: Vec<String> = Vec::new();
    for _ in common..a.len() {
        out.push("..".to_string());
    }
    for c in &b[common..] {
        out.push(c.as_os_str().to_string_lossy().into_owned());
    }
    out
}
