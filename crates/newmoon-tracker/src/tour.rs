//! The scripted runs: `--capture out.png`, `--tour steps.json --capture folder` and `--tour ... --live`
//! (C# `CaptureAndExit`, `TourAndExit`, `LiveTourAndExit` in MainForm.cs). They run on a worker thread and drive the
//! page through `Ui` (execute script, capture, DevTools protocol) the way the C# `await`ed.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

use crate::jsonx;
use crate::main_form::MainForm;

pub fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

fn ui_err(e: String) -> anyhow::Error {
    anyhow!(e)
}

pub fn capture_and_exit(app: &MainForm) {
    let capture = app.opt.capture.clone().unwrap_or_default();
    let r = (|| -> Result<()> {
        if let Some(then) = &app.opt.then_scheme {
            sleep(400);
            app.ui.set_scheme(Some(then.clone()));
        }
        sleep(700); // let fonts and the first layout settle
        if let Some(dir) = std::path::absolute(&capture)?.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let png = app.ui.capture_png().map_err(ui_err)?;
        std::fs::write(&capture, png)?;
        Ok(())
    })();
    if let Err(e) = r {
        let _ = std::fs::write(format!("{capture}.error.txt"), format!("{e:?}"));
    }
    app.ui.close();
}

/// The JavaScript that finds a step's target on the page: the first element matching the selector whose text starts with "text".
pub fn find_script(target: &str, text: &str) -> String {
    format!(
        "[...document.querySelectorAll({})].find(e => e.innerText.trim().startsWith({}))",
        jsonx::quote(target),
        jsonx::quote(text)
    )
}

/// The tour's steps: a JSON array.
pub fn read_steps(path: &str) -> Result<Vec<Value>> {
    let text = std::fs::read_to_string(path)?;
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))? {
        Value::Array(a) => Ok(a),
        _ => Err(anyhow!("The tour file must hold a JSON array.")),
    }
}

/// Keep the real mouse's hover tooltips out of the pictures.
pub const HIDE_TIP: &str = "document.head.appendChild(Object.assign(document.createElement('style'), { textContent: '#tip { display: none !important; }' }))";

/// `--tour steps.json --capture folder`: walk through the page like a person would and save a picture after each
/// step, plus manifest.json (file, caption, how long to show it, where the click was). tools\demo turns that into
/// the demo video. Steps: { "click" (or "point"): "css selector", "text": "starts with", "type": "text to type into it",
/// "before": "javascript run first", "until": "javascript that must become true before the picture",
/// "run": "javascript", "wait": ms, "hold": ms, "caption": "..." } - all optional.
pub fn tour_and_exit(app: &MainForm) {
    let dir = app.opt.capture.clone().unwrap_or_default();
    if let Err(e) = tour(app, &dir) {
        let _ = std::fs::write(Path::new(&dir).join("error.txt"), format!("{e:?}"));
    }
    app.ui.close();
}

fn tour(app: &MainForm, dir: &str) -> Result<()> {
    let ui = &app.ui;
    let js = |s: &str| ui.exec(s).map_err(ui_err);
    let mut shots: Vec<Value> = Vec::new();
    let mut n = 0;
    std::fs::create_dir_all(dir)?;
    let mut shot = |caption: &str, hold: i32, click: Option<&Vec<f64>>| -> Result<()> {
        n += 1;
        let name = format!("{n:03}.png");
        std::fs::write(Path::new(dir).join(&name), ui.capture_png().map_err(ui_err)?)?;
        shots.push(json!({ "file": name, "caption": caption, "hold": hold, "click": click }));
        Ok(())
    };
    // The real mouse may be resting over the window: keep its hover tooltips out of the pictures.
    js(HIDE_TIP)?;
    sleep(700);
    let steps = read_steps(app.opt.tour.as_deref().unwrap_or_default())?;
    for st in &steps {
        let get = |k: &str| st.get(k);
        let caption = jsonx::str(get("caption")).unwrap_or_default();
        let mut at: Option<Vec<f64>> = None;
        if get("before").is_some_and(|v| !v.is_null()) {
            js(&jsonx::str(get("before")).unwrap_or_default())?;
            sleep(250);
        }
        let target = jsonx::str(get("click")).or_else(|| jsonx::str(get("point"))); // "point" shows where, without clicking
        if let Some(target) = target {
            let find = find_script(&target, &jsonx::str(get("text")).unwrap_or_default());
            let r = js(&format!(
                "(() => {{ const e = {find}; if (!e) return null; e.scrollIntoView({{ block: 'nearest' }}); const r = e.getBoundingClientRect(); return [(r.left + r.width / 2) / innerWidth, (r.top + r.height / 2) / innerHeight]; }})()"
            ))?;
            if let Ok(Value::Array(a)) = serde_json::from_str::<Value>(&r) {
                at = Some(a.iter().filter_map(|x| x.as_f64()).collect());
            }
            match jsonx::str(get("type")) {
                None => {
                    if get("click").is_some_and(|v| !v.is_null()) {
                        js(&format!("(() => {{ const e = {find}; if (e) e.click(); }})()"))?;
                    }
                }
                Some(text) => {
                    let chars: Vec<char> = text.chars().collect();
                    for i in 1..=chars.len() {
                        let prefix: String = chars[..i].iter().collect();
                        js(&format!(
                            "(() => {{ const e = {find}; if (!e) return; e.focus(); e.value = {}; e.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()",
                            jsonx::quote(&prefix)
                        ))?;
                        sleep(90);
                        if i < chars.len() {
                            shot(&caption, 130, if i == 1 { at.as_ref() } else { None })?; // typing at a pace a viewer can follow
                        }
                    }
                }
            }
        }
        if get("run").is_some_and(|v| !v.is_null()) {
            js(&jsonx::str(get("run")).unwrap_or_default())?;
        }
        // "until": wait (up to 40 s) for something to be true on the page, e.g. a snapshot having finished
        if get("until").is_some_and(|v| !v.is_null()) {
            let until = jsonx::str(get("until")).unwrap_or_default();
            let mut tries = 0;
            while tries < 160 && js(&format!("!!({until})"))? != "true" {
                sleep(250);
                tries += 1;
            }
        }
        sleep(jsonx::int(get("wait"), 450).max(0) as u64);
        let typed = jsonx::str(get("type")).is_some();
        shot(&caption, jsonx::int(get("hold"), 2200), if typed { None } else { at.as_ref() })?;
    }
    std::fs::write(Path::new(dir).join("manifest.json"), serde_json::to_string_pretty(&shots)?)?;
    Ok(())
}

/// `--tour ... --live`: the tour filmed as it happens, with a moving pointer (live_tour.rs).
pub fn live_tour_and_exit(app: &MainForm) {
    let dir = app.opt.capture.clone().unwrap_or_default();
    let r = (|| -> Result<()> {
        sleep(700);
        crate::live_tour::run(&app.ui, app.opt.tour.as_deref().unwrap_or_default(), &dir)
    })();
    if let Err(e) = r {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(Path::new(&dir).join("error.txt"), format!("{e:?}"));
    }
    app.ui.close();
}
