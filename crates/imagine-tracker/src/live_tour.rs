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
use crate::tour::{read_steps, sleep};
use crate::ui::Ui;

fn n(v: f64) -> String {
    imagine::fmt::custom(v, "0.###")
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

/// Hover tooltips stay out of the film (the real mouse may be resting over the window, and the pointer crosses
/// many buttons on its way), except in a step that asks for them ("tip": true).
const HIDE_TIP: &str = "document.head.appendChild(Object.assign(document.createElement('style'), { id: 'tour-no-tip', textContent: '#tip { display: none !important; }' }))";

/// The JavaScript that finds a step's target on the page: the first element matching the selector whose text starts
/// with "text", else the first whose text has it anywhere.
fn find_script(target: &str, text: &str) -> String {
    format!(
        "((all, t) => all.find(e => e.innerText.trim().startsWith(t)) || all.find(e => e.innerText.includes(t)))([...document.querySelectorAll({})], {})",
        jsonx::quote(target),
        jsonx::quote(text)
    )
}

/// The pointer's place and path: ease in and out from here to there (C# `MoveTo`).
struct Pointer {
    x: f64,
    y: f64,
    /// the pointer: time, x, y (page pixels)
    path: Vec<Value>,
}

pub fn run(ui: &Ui, tour_path: &str, dir: &str, resize: &dyn Fn(i32, i32) -> Result<()>) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let clock = Instant::now();
    let now = move || clock.elapsed().as_secs_f64();
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

    let mut p = Pointer { x: size[0] * 0.55, y: size[1] * 0.6, path: Vec::new() };
    let mouse = |typ: &str, x: f64, y: f64, extra: &str| -> Result<()> {
        ui.dev_tools("Input.dispatchMouseEvent", &format!("{{\"type\":\"{typ}\",\"x\":{},\"y\":{}{extra}}}", n(x), n(y))).map_err(ui_err)?;
        Ok(())
    };
    let move_to = |p: &mut Pointer, tx: f64, ty: f64| -> Result<()> {
        let (sx, sy) = (p.x, p.y);
        let dist = ((tx - sx) * (tx - sx) + (ty - sy) * (ty - sy)).sqrt();
        if dist < 2.0 {
            return Ok(());
        }
        let seconds = f64::min(0.8, 0.28 + dist / 2600.0);
        let start = now();
        loop {
            let q = f64::min(1.0, (now() - start) / seconds);
            let e = q * q * (3.0 - 2.0 * q); // ease in and out
            p.x = sx + (tx - sx) * e;
            p.y = sy + (ty - sy) * e;
            p.path.push(json!([nv(now()), nv(p.x), nv(p.y)]));
            mouse("mouseMoved", p.x, p.y, "")?;
            if q >= 1.0 {
                break;
            }
            sleep(12);
        }
        Ok(())
    };
    p.path.push(json!([nv(now()), nv(p.x), nv(p.y)]));
    mouse("mouseMoved", p.x, p.y, "")?;
    sleep(500);

    for st in read_steps(tour_path)? {
        let get = |k: &str| st.get(k).filter(|v| !v.is_null());
        captions.push(json!({ "t": nv(now()), "text": jsonx::str(get("caption")).unwrap_or_default() }));
        // C# `st["tip"]?.GetValue<bool>()`: anything but a boolean throws
        let tip = match get("tip") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return Err(anyhow!("The step's \"tip\" must be true or false.")),
        };
        js(&format!("document.getElementById('tour-no-tip').disabled = {}", if tip { "true" } else { "false" }))?;
        if get("before").is_some() {
            js(&jsonx::str(get("before")).unwrap_or_default())?;
            sleep(250);
        }
        let target = jsonx::str(get("click")).or_else(|| jsonx::str(get("point"))); // "point" goes there without clicking
        if let Some(target) = target {
            let text_start = jsonx::str(get("text"));
            // the first whose text starts with "text", else the first whose text has it anywhere
            let find = find_script(&target, text_start.as_deref().unwrap_or(""));
            let wherever = format!(
                "(() => {{ const e = {find}; if (!e) return null; const r = e.getBoundingClientRect(); return [r.left + Math.min(r.width / 2, 140), r.top + r.height / 2]; }})()"
            );
            // bring it into view the way a person scrolls there (also when a scrolling list hides it, like a tile far down the
            // demon dropdown: then something else is at its place), then wait for the page to stop moving
            js(&format!(
                "(() => {{ const e = {find}; if (!e) return; const r = e.getBoundingClientRect(); const hit = document.elementFromPoint(r.left + Math.min(r.width / 2, 140), r.top + r.height / 2); if (r.top < 70 || r.bottom > innerHeight - 110 || !(hit && (hit === e || e.contains(hit) || hit.contains(e)))) e.scrollIntoView({{ block: 'center', behavior: 'smooth' }}); }})()"
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
                move_to(&mut p, at[0], at[1])?;
                sleep(140);
                // the page may have changed while the pointer travelled (a list that finished loading): follow the target
                for _ in 0..5 {
                    let Some(now_at) = pair(&wherever)? else { break };
                    if (now_at[0] - p.x).abs() < 3.0 && (now_at[1] - p.y).abs() < 3.0 {
                        break;
                    }
                    move_to(&mut p, now_at[0], now_at[1])?;
                    sleep(200);
                }
                let text = jsonx::str(get("type"));
                if get("click").is_some() || text.is_some() {
                    clicks.push(nv(now()));
                    mouse("mousePressed", p.x, p.y, ",\"button\":\"left\",\"clickCount\":1")?;
                    sleep(70);
                    mouse("mouseReleased", p.x, p.y, ",\"button\":\"left\",\"clickCount\":1")?;
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
        // "scroll": turn the mouse wheel under the pointer, that many pixels over "over" ms (2 s), easing in and out
        if let Some(scroll) = get("scroll") {
            let amount = scroll.as_f64().ok_or_else(|| anyhow!("The step's \"scroll\" must be a number."))?;
            let seconds = jsonx::int(get("over"), 2000) as f64 / 1000.0;
            let start = now();
            let mut done = 0.0;
            loop {
                let q = f64::min(1.0, (now() - start) / seconds);
                let e = q * q * (3.0 - 2.0 * q);
                let step = imagine::fmt::round(amount * e - done, 0);
                if step != 0.0 {
                    done += step;
                    mouse("mouseWheel", p.x, p.y, &format!(",\"deltaX\":0,\"deltaY\":{}", n(step)))?;
                }
                if q >= 1.0 {
                    break;
                }
                sleep(14);
            }
        }
        // "resize": "1680x920" grows or shrinks the window to that page size over "over" ms (0.7 s), about its centre
        if get("resize").is_some() {
            let spec = jsonx::str(get("resize")).ok_or_else(|| anyhow!("The step's \"resize\" must be like 1680x920."))?;
            let to = spec
                .split('x')
                .map(|s| imagine::fmt::parse_f64(s).ok_or_else(|| anyhow!("The step's \"resize\" must be like 1680x920.")))
                .collect::<Result<Vec<f64>>>()?;
            let from = pair("[innerWidth, innerHeight]")?.ok_or_else(|| anyhow!("the page has no size"))?;
            if to.len() < 2 || from.len() < 2 {
                return Err(anyhow!("The step's \"resize\" must be like 1680x920."));
            }
            let seconds = jsonx::int(get("over"), 700) as f64 / 1000.0;
            let start = now();
            loop {
                let q = f64::min(1.0, (now() - start) / seconds);
                let e = q * q * (3.0 - 2.0 * q);
                resize(
                    imagine::fmt::round(from[0] + (to[0] - from[0]) * e, 0) as i32,
                    imagine::fmt::round(from[1] + (to[1] - from[1]) * e, 0) as i32,
                )?;
                if q >= 1.0 {
                    break;
                }
                sleep(16);
            }
            sleep(200);
        }
        if get("run").is_some() {
            js(&jsonx::str(get("run")).unwrap_or_default())?;
        }
        // "save": "name" with "run": what that script returned goes to <folder>\name.txt (to look at a page while writing a tour)
        if get("save").is_some() {
            let name = jsonx::str(get("save")).unwrap_or_default();
            let result = js(&jsonx::str(get("value")).unwrap_or_else(|| "null".to_string()))?;
            std::fs::write(Path::new(dir).join(format!("{name}.txt")), json_node_to_string(&result)?)?;
        }
        // "until": wait (up to 40 s) for something to be true on the page
        if get("until").is_some() {
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
        "cursor": p.path, "clicks": clicks, "captions": captions, "missed": missed,
    });
    std::fs::write(Path::new(dir).join("manifest.json"), serde_json::to_string(&manifest)?)?;
    Ok(())
}

// ---------- "save": JsonNode.Parse(text)?.ToString() ----------

/// A parsed JSON value that keeps numbers as the text they came in (a `JsonElement` writes its number text as it was).
enum Node {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Node>),
    Obj(Vec<(String, Node)>),
}

struct Reader<'a> {
    s: &'a [char],
    i: usize,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], ' ' | '\t' | '\r' | '\n') {
            self.i += 1;
        }
    }

    fn bad<T>(&self) -> Result<T> {
        Err(anyhow!("The \"save\" step's value is not JSON (at {}).", self.i))
    }

    fn eat(&mut self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        if self.s[self.i..].starts_with(&w) {
            self.i += w.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Node> {
        self.ws();
        let Some(&c) = self.s.get(self.i) else { return self.bad() };
        match c {
            '{' => {
                self.i += 1;
                let mut members = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&'}') {
                    self.i += 1;
                    return Ok(Node::Obj(members));
                }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&'"') {
                        return self.bad();
                    }
                    let key = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&':') {
                        return self.bad();
                    }
                    self.i += 1;
                    let v = self.value()?;
                    members.push((key, v));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(',') => self.i += 1,
                        Some('}') => {
                            self.i += 1;
                            return Ok(Node::Obj(members));
                        }
                        _ => return self.bad(),
                    }
                }
            }
            '[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&']') {
                    self.i += 1;
                    return Ok(Node::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(',') => self.i += 1,
                        Some(']') => {
                            self.i += 1;
                            return Ok(Node::Arr(items));
                        }
                        _ => return self.bad(),
                    }
                }
            }
            '"' => Ok(Node::Str(self.string()?)),
            't' if self.eat("true") => Ok(Node::Bool(true)),
            'f' if self.eat("false") => Ok(Node::Bool(false)),
            'n' if self.eat("null") => Ok(Node::Null),
            '-' | '0'..='9' => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], '-' | '+' | '.' | 'e' | 'E' | '0'..='9') {
                    self.i += 1;
                }
                Ok(Node::Num(self.s[start..self.i].iter().collect()))
            }
            _ => self.bad(),
        }
    }

    fn string(&mut self) -> Result<String> {
        self.i += 1; // the opening quote
        let mut units: Vec<u16> = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else { return self.bad() };
            self.i += 1;
            match c {
                '"' => return Ok(String::from_utf16_lossy(&units)),
                '\\' => {
                    let Some(&e) = self.s.get(self.i) else { return self.bad() };
                    self.i += 1;
                    match e {
                        'b' => units.push(8),
                        't' => units.push(9),
                        'n' => units.push(10),
                        'f' => units.push(12),
                        'r' => units.push(13),
                        '"' | '\\' | '/' => units.push(e as u16),
                        'u' => {
                            let hex: String = self.s[self.i..(self.i + 4).min(self.s.len())].iter().collect();
                            let Ok(v) = u16::from_str_radix(&hex, 16) else { return self.bad() };
                            self.i += 4;
                            units.push(v);
                        }
                        _ => return self.bad(),
                    }
                }
                c => {
                    let mut b = [0u16; 2];
                    units.extend_from_slice(c.encode_utf16(&mut b));
                }
            }
        }
    }
}

/// `JavaScriptEncoder.Default`, which `JsonNode.ToString()` writes with: every non-ASCII character as \uXXXX,
/// and `" & ' + < > \`` and control characters too.
fn escape(s: &str, out: &mut String) {
    out.push('"');
    for u in s.encode_utf16() {
        match u {
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0A => out.push_str("\\n"),
            0x0C => out.push_str("\\f"),
            0x0D => out.push_str("\\r"),
            0x5C => out.push_str("\\\\"),
            0x20..=0x7E if !matches!(u as u8, b'"' | b'&' | b'\'' | b'+' | b'<' | b'>' | b'`') => out.push(u as u8 as char),
            _ => out.push_str(&format!("\\u{u:04X}")),
        }
    }
    out.push('"');
}

fn write_node(v: &Node, depth: usize, out: &mut String) {
    const NL: &str = "\r\n"; // Environment.NewLine
    let pad = |d: usize, out: &mut String| out.push_str(&"  ".repeat(d));
    match v {
        Node::Null => out.push_str("null"),
        Node::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Node::Num(t) => out.push_str(t),
        Node::Str(s) => escape(s, out),
        Node::Arr(items) if items.is_empty() => out.push_str("[]"),
        Node::Arr(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(NL);
                pad(depth + 1, out);
                write_node(it, depth + 1, out);
            }
            out.push_str(NL);
            pad(depth, out);
            out.push(']');
        }
        Node::Obj(members) if members.is_empty() => out.push_str("{}"),
        Node::Obj(members) => {
            out.push('{');
            for (i, (k, it)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(NL);
                pad(depth + 1, out);
                escape(k, out);
                out.push_str(": ");
                write_node(it, depth + 1, out);
            }
            out.push_str(NL);
            pad(depth, out);
            out.push('}');
        }
    }
}

/// C# `JsonNode.Parse(text)?.ToString() ?? ""`: a string as it is (no quotes), JSON null as "", the rest written
/// indented by 2 spaces.
fn json_node_to_string(text: &str) -> Result<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut r = Reader { s: &chars, i: 0 };
    let v = r.value()?;
    r.ws();
    if r.i < chars.len() {
        return r.bad();
    }
    Ok(match v {
        Node::Null => String::new(),
        Node::Str(s) => s,
        v => {
            let mut out = String::new();
            write_node(&v, 0, &mut out);
            out
        }
    })
}
