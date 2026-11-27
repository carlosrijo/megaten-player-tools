//! BABEL: the desktop window around the read-only reader (port of app\*.cs).
//!
//!   BABEL.exe                 normal use
//!   BABEL.exe --dev           load the page from app\ui in the repo (no rebuild after page edits) + DevTools (F12)
//!   BABEL.exe --capture out.png [--page gear] [--theme dark|light] [--mode digi] [--size 1280x860]
//!   ... see options.rs for the whole list.
//!
//! One self-contained exe: the page (app\ui), the picture patches (app\assets) and the icon are inside it; the WebView2
//! loader is linked in. The window is Microsoft Edge WebView2, which comes with Windows 11 (and updated Windows 10).
#![windows_subsystem = "windows"]

mod backend;
mod character_picture;
mod dialogs;
mod host;
mod jsonx;
mod live_tour;
mod main_form;
mod options;
mod settings;
mod site;
mod tour;
mod support;
mod ui;
mod update;
mod winutil;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::WindowBuilder;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

use imagine::diag::{about, crash, log};

use crate::host::WebHost;
use crate::main_form::MainForm;
use crate::options::AppOptions;
use crate::ui::{Ui, UserEvent};

const TITLE: &str = "BABEL";
const MIN_WIDTH: i32 = 760;
const MIN_HEIGHT: i32 = 520;

fn main() {
    about::init(about::BuildInfo {
        program: TITLE.to_string(),
        version: env!("IMAGINE_BUILD_VERSION").to_string(),
        commit: env!("IMAGINE_BUILD_COMMIT").to_string(),
        built: env!("IMAGINE_BUILD_DATE").to_string(),
    });
    // A crash with no window to show it would just vanish: write a crash report (logs\crash-<time>.txt, see
    // imagine::diag::crash) and say what happened.
    crash::install(Some(Box::new(|message, saved| {
        let note = match saved.and_then(|p| p.parent()) {
            Some(dir) => format!("\n\nA crash report was saved in {}.\nSettings > Troubleshooting > Save a support bundle sends it along.", dir.display()),
            None => String::new(),
        };
        winutil::error_box(None, &format!("BABEL stopped unexpectedly.\n\n{message}{note}"), TITLE);
    })));
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = AppOptions::parse(&args);
    let level = match log::level_from_args(&args) {
        Ok(l) => l,
        Err(m) => {
            winutil::error_box(None, &m, TITLE);
            return;
        }
    };
    // The local log, <data folder>\logs. A scripted run on the person's own data (no --data) writes nothing there.
    if opt.capture.is_none() || opt.data.is_some() {
        let logs = MainForm::data_dir_for(&opt).join("logs");
        if log::init(&logs, level, &about::build_info().line()) {
            crash::set_dir(&logs);
        }
    }
    // Just updated: wait for the old process to end (the two share the page's WebView2 profile), and remove the old exe
    // the swap left beside this one (update.rs). A scripted run never updates, and leaves those files alone.
    if opt.capture.is_none() {
        if let Some(from) = &opt.updated_from {
            imagine::log_info!("updated from {from}");
        }
        update::after_start(opt.wait_pid);
    }

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let ui = Ui::new(event_loop.create_proxy());
    let app = match MainForm::new(opt, ui) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            imagine::log_error!("start failed: {e:#}");
            winutil::error_box(None, &format!("BABEL stopped unexpectedly.\n\n{e}"), TITLE);
            return;
        }
    };
    let opt = app.opt.clone();

    // ---- the window ----
    let mut builder = WindowBuilder::new()
        .with_title(TITLE)
        .with_visible(false)
        .with_min_inner_size(LogicalSize::new(MIN_WIDTH as f64, MIN_HEIGHT as f64))
        .with_drag_and_drop(false);
    if opt.capture.is_some() {
        builder = builder.with_inner_size(PhysicalSize::new(opt.width.max(1) as u32, opt.height.max(1) as u32));
        if opt.offscreen {
            builder = builder.with_skip_taskbar(true); // `ShowInTaskbar = false`
        }
    }
    let window = match builder.build(&event_loop) {
        Ok(w) => w,
        Err(e) => {
            winutil::error_box(None, &format!("The app's window couldn't start.\n\n{e}"), TITLE);
            return;
        }
    };
    let hwnd = winutil::hwnd_of(window.hwnd());
    app.hwnd.store(window.hwnd(), Ordering::Relaxed);
    winutil::set_icons(hwnd);

    // where the window goes: a capture run is centred at the size asked for; otherwise where it was last (if still on a screen)
    let mut maximize = false;
    if opt.capture.is_some() {
        center_on_primary(hwnd);
        if opt.offscreen {
            // Beside the desktop, past its right edge: still a shown window (so the page keeps drawing), but nobody
            // sees it and the real mouse cannot reach it.
            let (w, h) = winutil::outer_size(hwnd);
            winutil::set_outer_bounds(hwnd, winutil::virtual_screen_right() + 200, 0, w, h);
        }
    } else {
        let (bounds, maximized) = app.saved_bounds();
        let placed = match bounds.as_deref() {
            Some(&[x, y, w, h]) if w >= MIN_WIDTH && h >= MIN_HEIGHT => {
                let r = windows::Win32::Foundation::RECT { left: x, top: y, right: x + w, bottom: y + h };
                if winutil::work_areas().iter().any(|a| winutil::intersects(a, &r)) {
                    winutil::set_outer_bounds(hwnd, x, y, w, h);
                    maximize = maximized;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if !placed {
            let (w, h) = (1280, 860);
            let wa = winutil::primary_work_area();
            winutil::set_outer_bounds(hwnd, wa.left + ((wa.right - wa.left) - w) / 2, wa.top + ((wa.bottom - wa.top) - h) / 2, w, h);
        }
    }

    // the title bar and the colour behind the page follow the page's theme, so there is no white flash
    let theme = app.theme();
    let dark = match theme.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => winutil::system_uses_dark(),
    };
    winutil::apply_chrome(hwnd, dark);
    let mut chrome_dark = dark;

    // ---- the page ----
    let local = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("."));
    // %LocalAppData%\BABEL (moved there from the old folder name on the first run, so the page storage is kept)
    let local_app = settings::app_folder(&local);
    if opt.capture.is_some() {
        // A capture or tour run starts from a clean page every time (nothing remembered from the run before).
        // Each run has its own folder (named after its process), so two scripted runs at once don't share one;
        // folders left by runs that have ended are cleared here.
        clear_old_capture_folders(&local_app);
    }
    // scripted runs keep their own page storage, so they never change what the real window remembers
    let user_data = local_app.join(if opt.capture.is_some() { format!("WebView2-capture-{}", std::process::id()) } else { "WebView2".to_string() });
    let _ = std::fs::create_dir_all(&app.data_dir);
    let site = Arc::new(site::Site::new(opt.dev, &app.data_dir));
    let handler: host::MessageHandler = {
        let app = app.clone();
        Arc::new(move |json, source| app.on_message(json, source))
    };
    let web = match WebHost::create(hwnd, &user_data, dark, opt.dev, opt.offscreen, opt.capture.is_some(), site.clone(), handler) {
        Ok(w) => w,
        Err(e) => {
            imagine::log_error!("the window's WebView2 couldn't start: {e}");
            winutil::error_box(
                Some(hwnd),
                &format!("The app's window couldn't start. It needs the Microsoft Edge WebView2 Runtime, which normally comes with Windows 11.\n\n{e}"),
                TITLE,
            );
            return;
        }
    };
    // what a bug report needs about this start: build, Windows, WebView2, where it runs from, how it was started
    imagine::log_info!(
        "app start: {} | Windows {} | WebView2 {} | exe {} | data folder {} | options:{}{}{}{}",
        about::build_info().line(),
        about::windows_info().text(),
        host::webview2_version().unwrap_or_else(|e| format!("not found ({e})")),
        std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default(),
        app.data_dir.display(),
        if opt.dev { " dev" } else { "" },
        if opt.capture.is_some() { " capture" } else { "" },
        if opt.tour.is_some() { " tour" } else { "" },
        if opt.offscreen { " offscreen" } else { "" },
    );
    web.set_scheme(theme.as_deref());
    if let Err(e) = web.navigate(&app.start_url()) {
        winutil::error_box(Some(hwnd), &format!("The page couldn't be opened.\n\n{e}"), TITLE);
        return;
    }
    if maximize {
        winutil::show_maximized(hwnd);
    } else {
        winutil::show_normal(hwnd);
    }
    web.resize();
    let mut web = Some(web);
    // the quest panel: made the first time it is asked for, then shown and hidden
    let mut panel: Option<Panel> = None;
    let panel_open = Arc::new(std::sync::atomic::AtomicBool::new(false));

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // the quest panel's own window: its size follows, closing it only hides it
            Event::WindowEvent { window_id, event, .. } if panel.as_ref().is_some_and(|p| p.window.id() == window_id) => {
                let p = panel.as_ref().expect("checked above");
                match event {
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => p.web.resize(),
                    WindowEvent::CloseRequested => {
                        panel_open.store(false, Ordering::Relaxed);
                        p.window.set_visible(false);
                    }
                    _ => {}
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    app.save_bounds(winutil::saved_bounds(hwnd));
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(w) = &web {
                        w.resize();
                    }
                }
                WindowEvent::Focused(true) => {
                    if let Some(w) = &web {
                        w.focus();
                    }
                }
                // tao follows Windows' own setting for the title bar; ours follows the page's theme
                WindowEvent::ThemeChanged(_) => winutil::apply_chrome(hwnd, chrome_dark),
                _ => {}
            },
            // ---- the quest panel ----
            Event::UserEvent(UserEvent::Panel { open, near }) => {
                if !open {
                    panel_open.store(false, Ordering::Relaxed);
                    if let Some(p) = &panel {
                        p.window.set_visible(false);
                    }
                    return;
                }
                if panel.is_none() {
                    match Panel::create(target, &user_data, chrome_dark, opt.dev, site.clone(), app.clone()) {
                        Ok(p) => panel = Some(p),
                        Err(e) => {
                            imagine::log_error!("the quest panel couldn't open: {e:#}");
                            return;
                        }
                    }
                }
                let p = panel.as_ref().expect("made above");
                p.place(near);
                p.window.set_visible(true);
                // while it is open, the app reads the quests itself and sends them over (every 3 s)
                if !panel_open.swap(true, Ordering::Relaxed) {
                    let (app, open) = (app.clone(), panel_open.clone());
                    std::thread::spawn(move || {
                        while open.load(Ordering::Relaxed) {
                            let json = app.quest_progress().unwrap_or_else(|_| "null".to_string());
                            app.ui.panel_post(format!("{{\"progress\":{json}}}"));
                            for _ in 0..30 {
                                if !open.load(Ordering::Relaxed) {
                                    break;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                        }
                    });
                }
            }
            Event::UserEvent(UserEvent::PanelPost(json)) => {
                if let Some(p) = &panel {
                    p.web.post_json(&json);
                }
            }
            Event::UserEvent(UserEvent::PanelDrag) => {
                if let Some(p) = &panel {
                    let _ = p.window.drag_window();
                }
            }
            Event::UserEvent(ev) => {
                let Some(w) = web.as_mut() else { return };
                match ev {
                    UserEvent::Post(json) => w.post_json(&json),
                    UserEvent::Exec { script, reply } => w.exec(&script, reply),
                    UserEvent::Capture { reply } => w.capture_png(reply),
                    UserEvent::DevTools { method, params, reply } => w.dev_tools(&method, &params, reply),
                    UserEvent::DevToolsEvents { name, sink } => w.dev_tools_events(&name, sink),
                    UserEvent::SetScheme(theme) => w.set_scheme(theme.as_deref()),
                    UserEvent::ApplyChrome(dark) => {
                        chrome_dark = dark;
                        winutil::apply_chrome(hwnd, dark);
                        w.set_background(dark);
                    }
                    UserEvent::Resize { w, h, reply } => {
                        winutil::resize_about_centre(hwnd, w, h);
                        let _ = reply.send(Ok(()));
                    }
                    UserEvent::Close => {
                        app.save_bounds(winutil::saved_bounds(hwnd));
                        *control_flow = ControlFlow::Exit;
                    }
                    // handled above
                    UserEvent::Panel { .. } | UserEvent::PanelPost(_) | UserEvent::PanelDrag => {}
                }
            }
            Event::LoopDestroyed => {
                panel_open.store(false, Ordering::Relaxed);
                if let Some(p) = panel.take() {
                    p.web.close();
                }
                if let Some(w) = web.take() {
                    w.close();
                }
                // an update put a new exe in place: start it now the window is gone (it waits for this process to end)
                update::restart_if_updated();
            }
            _ => {}
        }
    });
}

/// The quest panel: a small frameless window that stays above the game (always on top, not in the taskbar) with its own
/// page, panel.html, so a player sees the next step of their quests without leaving the game. The page only shows
/// what it is sent (the quest progress, read by the app while it is open) and can ask for three things: to be moved,
/// to be closed, and to be sent the progress now.
struct Panel {
    window: tao::window::Window,
    web: WebHost,
}

impl Panel {
    const WIDTH: f64 = 380.0;
    const HEIGHT: f64 = 260.0;

    fn create(
        target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
        user_data: &std::path::Path,
        dark: bool,
        dev: bool,
        site: Arc<site::Site>,
        app: Arc<MainForm>,
    ) -> anyhow::Result<Panel> {
        let window = WindowBuilder::new()
            .with_title("BABEL quests")
            .with_visible(false)
            .with_decorations(false)
            .with_always_on_top(true)
            .with_skip_taskbar(true)
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(Self::WIDTH, Self::HEIGHT))
            .with_min_inner_size(LogicalSize::new(260.0, 120.0))
            .with_drag_and_drop(false)
            .build(target)?;
        let hwnd = winutil::hwnd_of(window.hwnd());
        winutil::set_icons(hwnd);
        winutil::apply_chrome(hwnd, dark);
        // what the panel's page may ask: only from the app's own page, and only these
        let app2 = app.clone();
        let handler: host::MessageHandler = Arc::new(move |json, source| {
            let app = &app2;
            let prefix = format!("https://{}/", site::HOST);
            if !source.to_ascii_lowercase().starts_with(&prefix) {
                return;
            }
            let Ok(m) = serde_json::from_str::<serde_json::Value>(&json) else { return };
            match m.get("cmd").and_then(|c| c.as_str()) {
                Some("drag") => app.ui.panel_drag(),
                Some("close") => app.ui.panel(false, None),
                Some("ready") => {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let json = app.quest_progress().unwrap_or_else(|_| "null".to_string());
                        app.ui.panel_post(format!("{{\"progress\":{json}}}"));
                    });
                }
                _ => {}
            }
        });
        // the same browser profile as the main window: the panel and the Quest Tracker share which quests are hidden
        let theme = app.theme();
        let web = WebHost::create(hwnd, user_data, dark, dev, false, false, site, handler)?;
        web.set_scheme(theme.as_deref());
        web.navigate(&format!("https://{}/panel.html", site::HOST))?;
        web.resize();
        Ok(Panel { window, web })
    }

    /// Over the top right of the game's window (inside it, below its title bar), or of the screen without one.
    fn place(&self, near: Option<isize>) {
        let hwnd = winutil::hwnd_of(self.window.hwnd());
        let (w, h) = winutil::outer_size(hwnd);
        let (x, y) = match near.and_then(winutil::window_rect) {
            Some(r) => (r.right - w - 16, r.top + 48),
            None => {
                let wa = winutil::primary_work_area();
                (wa.right - w - 16, wa.top + 16)
            }
        };
        winutil::set_outer_bounds(hwnd, x, y, w, h);
    }
}

/// `StartPosition = CenterScreen` for a window whose size is already set.
fn center_on_primary(hwnd: windows::Win32::Foundation::HWND) {
    let (w, h) = winutil::outer_size(hwnd);
    let wa = winutil::primary_work_area();
    winutil::set_outer_bounds(hwnd, wa.left + ((wa.right - wa.left) - w) / 2, wa.top + ((wa.bottom - wa.top) - h) / 2, w, h);
}

/// Every `WebView2-capture*` folder whose trailing number is not the id of a running BABEL process: the
/// leftovers of scripted runs that have ended.
fn clear_old_capture_folders(app_data: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(app_data) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_ascii_lowercase().starts_with("webview2-capture") || !entry.path().is_dir() {
            continue;
        }
        let alive = name
            .rsplit('-')
            .next()
            .and_then(|last| last.parse::<u32>().ok())
            .and_then(winutil::process_name)
            .is_some_and(|p| p.to_ascii_lowercase().contains("babel"));
        if !alive {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
