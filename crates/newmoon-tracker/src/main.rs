//! NewMoon Build Tracker: the desktop window around the read-only reader (port of app\*.cs).
//!
//!   NewMoonTracker.exe                 normal use
//!   NewMoonTracker.exe --dev           load the page from app\ui in the repo (no rebuild after page edits) + DevTools (F12)
//!   NewMoonTracker.exe --capture out.png [--page gear] [--theme dark|light] [--mode digi] [--size 1280x860]
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
mod ui;
mod winutil;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::WindowBuilder;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

use crate::host::WebHost;
use crate::main_form::MainForm;
use crate::options::AppOptions;
use crate::ui::{Ui, UserEvent};

const TITLE: &str = "NewMoon Build Tracker";
const MIN_WIDTH: i32 = 760;
const MIN_HEIGHT: i32 = 520;

fn main() {
    // A crash with no window to show it would just vanish: say what happened.
    std::panic::set_hook(Box::new(|info| {
        if std::thread::current().name() == Some("main") {
            winutil::error_box(None, &format!("NewMoon Build Tracker stopped unexpectedly.\n\n{info}"), TITLE);
        }
    }));
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = AppOptions::parse(&args);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let ui = Ui::new(event_loop.create_proxy());
    let app = Arc::new(MainForm::new(opt, ui));
    let opt = app.opt.clone();

    // ---- the window ----
    let mut builder = WindowBuilder::new()
        .with_title(TITLE)
        .with_visible(false)
        .with_min_inner_size(LogicalSize::new(MIN_WIDTH as f64, MIN_HEIGHT as f64))
        .with_drag_and_drop(false);
    if opt.capture.is_some() {
        builder = builder.with_inner_size(PhysicalSize::new(opt.width.max(1) as u32, opt.height.max(1) as u32));
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
    if opt.capture.is_some() {
        // A capture or tour run starts from a clean page every time (nothing remembered from the run before).
        let _ = std::fs::remove_dir_all(local.join("NewMoonTracker").join("WebView2-capture"));
    }
    // scripted runs keep their own page storage, so they never change what the real window remembers
    let user_data = local.join("NewMoonTracker").join(if opt.capture.is_some() { "WebView2-capture" } else { "WebView2" });
    let _ = std::fs::create_dir_all(&app.data_dir);
    let site = Arc::new(site::Site::new(opt.dev, &app.data_dir));
    let handler: host::MessageHandler = {
        let app = app.clone();
        Arc::new(move |json, source| app.on_message(json, source))
    };
    let mut web = match WebHost::create(hwnd, &user_data, dark, opt.dev, site, handler) {
        Ok(w) => w,
        Err(e) => {
            winutil::error_box(
                Some(hwnd),
                &format!("The app's window couldn't start. It needs the Microsoft Edge WebView2 Runtime, which normally comes with Windows 11.\n\n{e}"),
                TITLE,
            );
            return;
        }
    };
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

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
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
                    UserEvent::Close => {
                        app.save_bounds(winutil::saved_bounds(hwnd));
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::LoopDestroyed => {
                if let Some(w) = web.take() {
                    w.close();
                }
            }
            _ => {}
        }
    });
}

/// `StartPosition = CenterScreen` for a window whose size is already set.
fn center_on_primary(hwnd: windows::Win32::Foundation::HWND) {
    let (w, h) = winutil::outer_size(hwnd);
    let wa = winutil::primary_work_area();
    winutil::set_outer_bounds(hwnd, wa.left + ((wa.right - wa.left) - w) / 2, wa.top + ((wa.bottom - wa.top) - h) / 2, w, h);
}
