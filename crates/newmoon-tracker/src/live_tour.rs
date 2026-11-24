//! `--tour steps.json --capture folder --live`: the same steps as a picture tour, but recorded as they happen
//! (C# `LiveTour`, app\LiveTour.cs). The page is filmed (every frame it draws, with the time it was drawn) while a
//! pointer really moves over it and clicks: the page gets true mouse events, so buttons light up under the pointer,
//! lists scroll and text is typed as a person would. Saved: the frames, and manifest.json with their times, the
//! pointer's path, the clicks and the captions. tools\demo\composer draws the pointer and the captions on top and
//! makes the video.
//! Nothing here touches the real mouse: the events go to the page only.

use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::jsonx;
use crate::tour::{find_script, read_steps, sleep, HIDE_TIP};
use crate::ui::Ui;

fn n(v: f64) -> String {
    newmoon::fmt::custom(v, "0.###")
}

/// A number the way .NET wrote it into the manifest (whole numbers without ".0").
fn nv(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 1e15 { Value::from(v as i64) } else { json!(v) }
}

struct Frames {
    count: usize,
    list: Vec<Value>,
}

fn ui_err(e: String) -> anyhow::Error {
    anyhow!(e)
}

pub fn run(ui: &Ui, tour_path: &str, dir: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let clock = Instant::now();
    let now = move || clock.elapsed().as_secs_f64();
    let mut path: Vec<Value> = Vec::new(); // the pointer: time, x, y (page pixels)
    let mut clicks: Vec<Value> = Vec::new();
    let mut captions: Vec<Value> = Vec::new();
    let mut missed: Vec<String> = Vec::new(); // steps whose target was not on the page

    let js = |s: &str| ui.exec(s).map_err(ui_err);
    let pair = |s: &str| -> Result<Option<Vec<f64>>> {
        Ok(match serde_json::from_str::<Value>(&js(s)?) {
            Ok(Value::Array(a)) => Some(a.iter().filter_map(|x| x.as_f64()).collect()),
            _ => None,
        })
    };

    // the real mouse may be resting over the window: keep its hover tooltips out of the film
    js(HIDE_TIP)?;
    let size = pair("[innerWidth, innerHeight]")?.ok_or_else(|| anyhow!("the page has no size"))?;

    // every frame the page draws: saved as f00001.jpg ... and acknowledged so the next one comes
    let frames = Arc::new(Mutex::new(Frames { count: 0, list: Vec::new() }));
    {
        let (frames, ui2, dir) = (frames.clone(), ui.clone(), PathBuf::from(dir));
        ui.dev_tools_events(
            "Page.screencastFrame",
            Arc::new(move |params: String| {
                let Ok(j) = serde_json::from_str::<Value>(&params) else { return };
                let Some(data) = j.get("data").and_then(|d| d.as_str()) else { return };
                let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else { return };
                let t = now();
                {
                    let mut f = frames.lock().unwrap_or_else(|p| p.into_inner());
                    f.count += 1;
                    let name = format!("f{:05}.jpg", f.count);
                    let _ = std::fs::write(Path::new(&dir).join(&name), bytes);
                    f.list.push(json!({ "file": name, "t": nv(t) }));
                }
                let session = j.get("sessionId").map(|s| s.to_string()).unwrap_or_else(|| "0".to_string());
                ui2.dev_tools_nowait("Page.screencastFrameAck", &format!("{{\"sessionId\":{session}}}"));
            }),
        );
    }
    ui.dev_tools("Page.startScreencast", "{\"format\":\"jpeg\",\"quality\":95,\"everyNthFrame\":1}").map_err(ui_err)?;

    let (mut x, mut y) = (size[0] * 0.55, size[1] * 0.6);
    let mouse = |typ: &str, x: f64, y: f64, extra: &str| -> Result<()> {
        ui.dev_tools("Input.dispatchMouseEvent", &format!("{{\"type\":\"{typ}\",\"x\":{},\"y\":{}{extra}}}", n(x), n(y))).map_err(ui_err)?;
        Ok(())
    };
    path.push(json!([nv(now()), nv(x), nv(y)]));
    mouse("mouseMoved", x, y, "")?;
    sleep(500);

    for st in read_steps(tour_path)? {
        let get = |k: &str| st.get(k);
        captions.push(json!({ "t": nv(now()), "text": jsonx::str(get("caption")).unwrap_or_default() }));
        if get("before").is_some_and(|v| !v.is_null()) {
            js(&jsonx::str(get("before")).unwrap_or_default())?;
            sleep(250);
        }
        let target = jsonx::str(get("click")).or_else(|| jsonx::str(get("point"))); // "point" goes there without clicking
        if let Some(target) = target {
            let text_start = jsonx::str(get("text"));
            let find = find_script(&target, text_start.as_deref().unwrap_or(""));
            let wherever = format!(
                "(() => {{ const e = {find}; if (!e) return null; const r = e.getBoundingClientRect(); return [r.left + Math.min(r.width / 2, 140), r.top + r.height / 2]; }})()"
            );
            // bring it into view the way a person scrolls there, then wait for the page to stop moving
            js(&format!(
                "(() => {{ const e = {find}; if (!e) return; const r = e.getBoundingClientRect(); if (r.top < 70 || r.bottom > innerHeight - 110) e.scrollIntoView({{ block: 'center', behavior: 'smooth' }}); }})()"
            ))?;
            let mut at: Option<Vec<f64>> = None;
            let mut last: Option<Vec<f64>> = None;
            for _ in 0..25 {
                at = pair(&wherever)?;
                if at.is_none() || last.as_ref().is_some_and(|l| (at.as_ref().map(|a| a[1]).unwrap_or(0.0) - l[1]).abs() < 0.5) {
                    break;
                }
                last = at.clone();
                sleep(90);
            }
            if at.is_none() {
                missed.push(format!("{target} {}", text_start.unwrap_or_default()));
            }
            if let Some(at) = at {
                // MoveTo: ease in and out
                let (sx, sy) = (x, y);
                let dist = ((at[0] - sx).powi(2) + (at[1] - sy).powi(2)).sqrt();
                if dist >= 2.0 {
                    let seconds = f64::min(0.95, 0.32 + dist / 2400.0);
                    let start = now();
                    loop {
                        let p = f64::min(1.0, (now() - start) / seconds);
                        let e = p * p * (3.0 - 2.0 * p);
                        x = sx + (at[0] - sx) * e;
                        y = sy + (at[1] - sy) * e;
                        path.push(json!([nv(now()), nv(x), nv(y)]));
                        mouse("mouseMoved", x, y, "")?;
                        if p >= 1.0 {
                            break;
                        }
                        sleep(12);
                    }
                }
                sleep(140);
                let text = jsonx::str(get("type"));
                if get("click").is_some_and(|v| !v.is_null()) || text.is_some() {
                    clicks.push(nv(now()));
                    mouse("mousePressed", x, y, ",\"button\":\"left\",\"clickCount\":1")?;
                    sleep(70);
                    mouse("mouseReleased", x, y, ",\"button\":\"left\",\"clickCount\":1")?;
                }
                if let Some(text) = text {
                    sleep(250);
                    for ch in text.chars() {
                        ui.dev_tools("Input.insertText", &format!("{{\"text\":{}}}", jsonx::quote(&ch.to_string()))).map_err(ui_err)?;
                        sleep(95); // typing at a pace a viewer can follow
                    }
                }
            }
        }
        if get("run").is_some_and(|v| !v.is_null()) {
            js(&jsonx::str(get("run")).unwrap_or_default())?;
        }
        // "until": wait (up to 40 s) for something to be true on the page
        if get("until").is_some_and(|v| !v.is_null()) {
            let until = jsonx::str(get("until")).unwrap_or_default();
            let mut tries = 0;
            while tries < 160 && js(&format!("!!({until})"))? != "true" {
                sleep(250);
                tries += 1;
            }
        }
        sleep(jsonx::int(get("hold"), 2200).max(0) as u64);
    }
    let end = now();
    sleep(150);
    ui.dev_tools("Page.stopScreencast", "{}").map_err(ui_err)?;
    let list = frames.lock().unwrap_or_else(|p| p.into_inner()).list.clone();
    let manifest = json!({
        "size": size.iter().map(|v| nv(*v)).collect::<Vec<_>>(), "end": nv(end), "frames": list,
        "cursor": path, "clicks": clicks, "captions": captions, "missed": missed,
    });
    std::fs::write(Path::new(dir).join("manifest.json"), serde_json::to_string(&manifest)?)?;
    Ok(())
}
