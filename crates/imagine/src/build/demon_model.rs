//! Port of `src/Build/DemonModel.cs`.
//!
//! A demon's 3D model for the page's assistant (the animated demon in the corner): reads the client's own files
//! (Devil\bone\<model>_Base.nif, the animations its .kfm lists) and writes one JSON file the page can draw
//! without knowing the game's formats: nodes, meshes, the texture as PNG, and every animation sampled at 30 frames a
//! second. Made once and kept in <data>\cache\models\<model>.json (game art: not part of the repository).
//!
//! The JSON is written by hand, character for character as System.Text.Json wrote it for the C# anonymous objects
//! (compact, default string escaping, doubles by [`fmt::g`], the one `float` (the material alpha) by [`fmt::g32`]).
//! The only bytes that differ from the C# output are the texture PNGs inside the `textures` data URIs: they are the
//! same pictures, compressed by a different zlib.

use crate::data::nif::*;
use crate::data::tga;
use crate::fmt;
use crate::text::ic_key;
use crate::Result;
use anyhow::anyhow;
use fancy_regex::Regex;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Raise when the JSON layout changes: older cached files are then made again.
pub const FORMAT: i32 = 1;
/// The assistant's model: Black Frost (demon 723; its icon is icon_D2_11_003a, named after the model).
pub const ASSISTANT: &str = "D2_11_003a";
pub const ASSISTANT_NAME: &str = "Black Frost";
/// Its demon id (DevilData 723), for its row in reference\devil-models.tsv.
pub const ASSISTANT_DEMON: i32 = 723;
const FPS: i32 = 30;

/// A demon's model in the game's own tables (reference\devil-models.tsv: DevilData's modelID looked up in the client's
/// CModelData2): the model's file name (Devil\bone\<model>_Base.nif), the size the game draws it at, and the size of its
/// summoning circle. `None` for a demon with no model.
#[derive(Clone, Debug, PartialEq)]
pub struct DemonModelRef {
    pub model: String,
    pub scale: f64,
    pub circle: f64,
    /// the summoning effect's colour (CModelData color), 0-255 each
    pub color: [u8; 3],
}

pub fn model_of_demon(demon: i32) -> Result<Option<DemonModelRef>> {
    static TABLE: LazyLock<std::sync::Mutex<Option<HashMap<i32, DemonModelRef>>>> = LazyLock::new(|| std::sync::Mutex::new(None));
    let mut t = TABLE.lock().map_err(|_| anyhow!("model table lock"))?;
    if t.is_none() {
        let mut m = HashMap::new();
        for line in crate::data::game_data::reference_lines("devil-models.tsv")?.unwrap_or_default() {
            if line.starts_with('#') {
                continue;
            }
            let r: Vec<&str> = line.split('\t').collect();
            if r.len() < 5 {
                continue;
            }
            let (Some(id), Ok(scale), Ok(circle)) = (crate::fmt::parse_i32(r[0]), r[3].trim().parse::<f64>(), r[4].trim().parse::<f64>()) else { continue };
            // it becomes a file name: letters, digits and _ only
            let model = r[2].trim();
            if model.is_empty() || !model.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            let mut color = [162u8, 70, 243];
            if let Some(c) = r.get(5) {
                let v: Vec<u8> = c.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                if v.len() == 3 {
                    color = [v[0], v[1], v[2]];
                }
            }
            m.insert(id, DemonModelRef { model: model.to_string(), scale, circle, color });
        }
        *t = Some(m);
    }
    Ok(t.as_ref().and_then(|m| m.get(&demon).cloned()))
}

pub fn cache_path(data_root: &Path, model: &str) -> PathBuf {
    data_root.join("cache").join("models").join(format!("{model}.json"))
}

/// The model as JSON text: the cached file, made from the client's files when missing or of an older layout
/// (and kept, unless `save` is off). `None` when it can't be made (no client folder, no such model).
pub fn load(data_root: &Path, client_dir: Option<&Path>, model: &str, save: bool) -> Result<Option<String>> {
    let path = cache_path(data_root, model);
    if is_current(&path) {
        return Ok(Some(read_text(&path)?));
    }
    let Some(client_dir) = client_dir else { return Ok(None) };
    let Some(json) = export(client_dir, model, None)? else { return Ok(None) };
    if !save {
        return Ok(Some(json));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.clone().into_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, json.as_bytes())?;
    std::fs::rename(&tmp, &path)?;
    Ok(Some(json))
}

/// `File.ReadAllText`: UTF-8, a byte order mark is dropped.
fn read_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

fn is_current(path: &Path) -> bool {
    use std::io::Read;
    // C#: any IOException (also a missing file) -> not current
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    let mut head = [0u8; 24];
    let mut n = 0;
    while n < head.len() {
        match f.read(&mut head[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(_) => return false,
        }
    }
    let head: String = head[..n].iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
    head.starts_with(&format!("{{\"format\":{FORMAT},"))
}

/// A scene node of the model (C# `Node`). Nodes live in an arena and refer to each other by arena index.
struct Node {
    name: String,
    parent: Option<usize>,
    children: Vec<usize>,
    t: [f64; 3],
    q: [f64; 4],
    s: f64,
    hidden: bool,
    /// the NIF block this node was made from (None for the "NonAccum" nodes made here)
    block: Option<i32>,
    index: i32,
    /// rest pose
    world: [f64; 12],
}

impl Node {
    fn new(name: String, parent: Option<usize>) -> Node {
        Node { name, parent, children: Vec::new(), t: [0.0; 3], q: [1.0, 0.0, 0.0, 0.0], s: 1.0, hidden: false, block: None, index: 0, world: [0.0; 12] }
    }
}

fn walk(nif: &NifFile, block: i32, parent: Option<usize>, arena: &mut Vec<Node>, by_block: &mut IndexMap<i32, usize>) -> Option<usize> {
    let av = nif.get::<NifAv>(block)?;
    if by_block.contains_key(&block) {
        return None;
    }
    let mut n = Node::new(av.name().unwrap_or("").to_string(), parent);
    n.block = Some(block);
    n.hidden = av.hidden();
    n.t = d(&av.t);
    n.q = quat_of(&av.r);
    n.s = av.s as f64;
    let id = arena.len();
    arena.push(n);
    by_block.insert(block, id);
    if let Some(nn) = nif.get::<NifNode>(block) {
        for &c in &nn.children {
            if let Some(child) = walk(nif, c, Some(id), arena, by_block) {
                arena[id].children.push(child);
            }
        }
    }
    Some(id)
}

fn flatten(id: usize, arena: &mut Vec<Node>, order: &mut Vec<usize>) {
    arena[id].index = order.len() as i32;
    order.push(id);
    let local = trs(&arena[id].t, &mat_of(&arena[id].q), arena[id].s);
    arena[id].world = match arena[id].parent {
        Some(p) => mul(&arena[p].world, &local),
        None => local,
    };
    let children = arena[id].children.clone();
    for c in children {
        flatten(c, arena, order);
    }
}

/// `Prop<T>()` of the mesh loop: the first property of type T on the node or the nearest parent that has one.
fn prop<'a, T: NifGet>(nif: &'a NifFile, arena: &[Node], start: usize) -> Option<&'a T> {
    let mut a = Some(start);
    while let Some(i) = a {
        if let Some(b) = arena[i].block {
            if let Some(av) = nif.get::<NifAv>(b) {
                for &p in &av.properties {
                    if let Some(found) = nif.get::<T>(p) {
                        return Some(found);
                    }
                }
            }
        }
        a = arena[i].parent;
    }
    None
}

fn is_hidden(arena: &[Node], start: usize) -> bool {
    let mut hidden = false;
    let mut a = Some(start);
    while let Some(i) = a {
        hidden |= arena[i].hidden;
        a = arena[i].parent;
    }
    hidden
}

static KF_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z0-9_\-]+\.kf(?![A-Za-z0-9])").expect("kf name pattern"));

/// The numbers of every mesh and animation, one after another; each block starts on a multiple of 4 bytes.
#[derive(Default)]
struct Bin(Vec<u8>);

impl Bin {
    fn pad(&mut self) {
        while self.0.len() % 4 != 0 {
            self.0.push(0);
        }
    }
    fn at(&mut self) -> i32 {
        self.pad();
        self.0.len() as i32
    }
    fn add_f32(&mut self, v: &[f32]) -> i32 {
        let at = self.at();
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
        at
    }
    fn add_u16(&mut self, v: &[u16]) -> i32 {
        let at = self.at();
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
        at
    }
    fn add_i16(&mut self, v: &[i16]) -> i32 {
        let at = self.at();
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
        at
    }
    fn add_u8(&mut self, v: &[u8]) -> i32 {
        let at = self.at();
        self.0.extend_from_slice(v);
        at
    }
    fn into_bytes(mut self) -> Vec<u8> {
        self.pad();
        self.0
    }
}

/// The JSON text being written; `bad` is set when a number is not finite (System.Text.Json throws on those).
struct Json {
    s: String,
    bad: bool,
}

impl Json {
    fn raw(&mut self, t: &str) {
        self.s.push_str(t);
    }
    fn num(&mut self, v: f64) {
        if !v.is_finite() {
            self.bad = true;
        }
        self.s.push_str(&fmt::g(v));
    }
    fn num32(&mut self, v: f32) {
        if !v.is_finite() {
            self.bad = true;
        }
        self.s.push_str(&fmt::g32(v));
    }
    fn int(&mut self, v: i32) {
        self.s.push_str(&v.to_string());
    }
    fn bool(&mut self, v: bool) {
        self.s.push_str(if v { "true" } else { "false" });
    }
    fn str(&mut self, v: &str) {
        escape_into(&mut self.s, v);
    }
    fn doubles(&mut self, v: &[f64]) {
        self.s.push('[');
        for (i, x) in v.iter().enumerate() {
            if i > 0 {
                self.s.push(',');
            }
            self.num(*x);
        }
        self.s.push(']');
    }
    fn key(&mut self, first: bool, k: &str) {
        if !first {
            self.s.push(',');
        }
        self.s.push('"');
        self.s.push_str(k);
        self.s.push_str("\":");
    }
}

/// System.Text.Json's default string escaping (JavaScriptEncoder.Default): the HTML-sensitive characters, everything
/// outside printable ASCII as \uXXXX (upper case, UTF-16 units), and the short forms for \b \t \n \f \r \\.
pub fn escape_into(out: &mut String, v: &str) {
    out.push('"');
    for u in v.encode_utf16() {
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

/// `Convert.ToBase64String`.
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    s
}

/// C# `Math.Max(double, double)`: NaN wins, and +0 is above -0.
fn net_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a == b { if a.is_sign_negative() { b } else { a } } else if b < a { a } else { b }
}

/// C# `Math.Min(double, double)`.
fn net_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return a;
    }
    if b.is_nan() {
        return b;
    }
    if a == b { if a.is_sign_negative() { a } else { b } } else if a < b { a } else { b }
}

/// A float as `{x:0.##}`-style custom formats see it: .NET rounds a float to 7 significant digits first.
fn custom32(x: f32, pattern: &str) -> String {
    if !x.is_finite() {
        return fmt::custom(x as f64, pattern);
    }
    let seven: f64 = format!("{:.6e}", x).parse().unwrap_or(x as f64);
    fmt::custom(seven, pattern)
}

/// `{value,-width}` (pads to the UTF-16 length)
fn pad_right(s: &str, width: usize) -> String {
    let n = s.encode_utf16().count();
    format!("{s}{}", " ".repeat(width.saturating_sub(n)))
}

/// `Comparer<float>.Default`: NaN sorts below every number and equal to itself.
fn cmp_f32(a: f32, b: f32) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Equal,
        (true, false) => Less,
        (false, true) => Greater,
        _ => a.partial_cmp(&b).unwrap_or(Equal),
    }
}

fn oob(what: &str) -> anyhow::Error {
    anyhow!("index out of range ({what}): the model file does not hold together")
}

fn at<T: Copy>(v: &[T], i: i64, what: &str) -> Result<T> {
    if i < 0 {
        return Err(oob(what));
    }
    v.get(i as usize).copied().ok_or_else(|| oob(what))
}

/// The model as JSON text (see app\ui\assistant.js for the reader), or `None` when the client has no such model.
/// `log` gets a summary.
pub fn export(client_dir: &Path, model: &str, mut log: Option<&mut dyn FnMut(&str)>) -> Result<Option<String>> {
    let dir = client_dir.join("Devil").join("bone");
    let nif_path = dir.join(format!("{model}_Base.nif"));
    if !nif_path.is_file() {
        return Ok(None);
    }
    let nif = NifFile::load(&nif_path)?;
    let mut say = |s: String| {
        if let Some(l) = log.as_mut() {
            l(&s);
        }
    };

    // ---- nodes ----
    let mut arena: Vec<Node> = Vec::new();
    let mut by_block: IndexMap<i32, usize> = IndexMap::new();
    let mut top: Vec<usize> = Vec::new();
    for &r in &nif.roots {
        if let Some(root) = walk(&nif, r, None, &mut arena, &mut by_block) {
            top.push(root);
        }
    }
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for &id in by_block.values() {
        by_name.entry(arena[id].name.clone()).or_insert(id);
    }

    // ---- animations: the files the model's .kfm lists ----
    struct Seq {
        name: String,
        file: NifFile,
        /// block index of the NiControllerSequence
        seq: usize,
    }
    let mut sequences: Vec<Seq> = Vec::new();
    let kfm = dir.join(format!("{model}_Base.kfm"));
    if kfm.is_file() {
        let text: String = std::fs::read(&kfm)?.iter().map(|&b| b as char).collect();
        let mut names: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for m in KF_NAME.find_iter(&text) {
            let m = m?;
            if seen.insert(ic_key(m.as_str())) {
                names.push(m.as_str().to_string());
            }
        }
        let mut prefix: String = match names.first() {
            Some(n0) => n0[..n0.rfind('_').map(|i| i + 1).unwrap_or(0)].to_string(),
            None => String::new(),
        };
        while !prefix.is_empty() && !names.iter().all(|n| n.to_uppercase().starts_with(&prefix.to_uppercase())) {
            let trimmed = prefix.trim_end_matches('_');
            let cut = trimmed.rfind('_').map(|i| i + 1).unwrap_or(0);
            prefix = prefix[..cut].to_string();
        }
        for name in &names {
            let path = dir.join(name);
            if !path.is_file() {
                continue;
            }
            match NifFile::load(&path) {
                Ok(kf) => {
                    let first = kf.blocks.iter().position(|b| matches!(b.kind, NifKind::Sequence(_)));
                    if let Some(seq) = first {
                        let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        let short = stem.get(prefix.len()..).ok_or_else(|| oob("animation name"))?.to_string();
                        sequences.push(Seq { name: short, file: kf, seq });
                    }
                }
                Err(e) if is_nif_error(&e) => say(format!("skipped {e}")),
                Err(e) => return Err(e),
            }
        }
    }
    // The game splits the animated root in two at run time: "<root>" (movement across the ground) and
    // "<root> NonAccum" (everything else), with the root's children under the second. Same here.
    for s in &sequences {
        let Some(seq) = s.file.get::<NifSequence>(s.seq as i32) else { continue };
        for link in &seq.links {
            let Some(node) = &link.node else { continue };
            if !by_name.contains_key(node) && node.ends_with(" NonAccum") {
                let Some(&accum) = by_name.get(&node[..node.len() - 9]) else { continue };
                let na = arena.len();
                let mut n = Node::new(node.clone(), Some(accum));
                n.children = std::mem::take(&mut arena[accum].children);
                arena.push(n);
                let kids = arena[na].children.clone();
                for c in kids {
                    arena[c].parent = Some(na);
                }
                arena[accum].children = vec![na];
                by_name.insert(node.clone(), na);
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    for &n in &top {
        flatten(n, &mut arena, &mut order);
    }

    // ---- meshes ----
    let mut bin = Bin::default();
    let mut textures: Vec<String> = Vec::new();
    let mut texture_of: HashMap<i32, i32> = HashMap::new();
    let mut meshes: Vec<String> = Vec::new();
    let mut worst_bind = 0f64;
    let mut bad = false;
    for &ni in &order {
        let Some(block) = arena[ni].block else { continue };
        let Some(geom) = nif.get::<NifGeom>(block) else { continue };
        let Some(gd) = nif.get::<NifGeomData>(geom.data) else { continue };
        let Some(gpos) = gd.pos.as_ref() else { continue };
        if gd.tris.is_empty() {
            continue;
        }
        let n_name = arena[ni].name.clone();
        let hidden = is_hidden(&arena, ni);
        let mat = prop::<NifMaterial>(&nif, &arena, ni);
        let alpha = prop::<NifAlpha>(&nif, &arena, ni);
        let mut tex = -1;
        if gd.uv.is_some() {
            if let Some(tp) = prop::<NifTexturing>(&nif, &arena, ni) {
                if tp.base >= 0 {
                    if let Some(src) = nif.get::<NifSourceTexture>(tp.base) {
                        match texture_of.get(&tp.base) {
                            Some(&t) => tex = t,
                            None => {
                                let png = texture_png(&nif, src, &dir)?;
                                if let Some(pd) = nif.get::<NifPixelData>(src.pixels) {
                                    let join = |v: &[i32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
                                    say(format!(
                                        "texture {}: {}x{} format {}, {} bits, channels {} bits {}{}",
                                        src.file.as_deref().unwrap_or(""),
                                        pd.width,
                                        pd.height,
                                        pd.format,
                                        pd.bits_per_pixel,
                                        join(&pd.channels),
                                        join(&pd.channel_bits),
                                        if png.is_none() { " (not read)" } else { "" }
                                    ));
                                }
                                tex = if png.is_some() { textures.len() as i32 } else { -1 };
                                texture_of.insert(tp.base, tex);
                                if let Some(png) = png {
                                    textures.push(format!("data:image/png;base64,{}", base64(&png)));
                                }
                            }
                        }
                    }
                }
            }
        }
        // the skin object (its bin data is added before the mesh's own)
        let mut skin_json: Option<String> = None;
        let mut bones = 0usize;
        let count = gd.count.max(0) as usize;
        if let Some(si) = nif.get::<NifSkinInstance>(geom.skin) {
            if let (Some(sd), Some(&root_id)) = (nif.get::<NifSkinData>(si.data), by_block.get(&si.root)) {
                let mut per_vertex: Vec<Vec<(i32, f32)>> = vec![Vec::new(); count];
                let mut any = false;
                for (b, bone) in sd.bones.iter().enumerate() {
                    for k in 0..bone.vertex.len() {
                        let v = bone.vertex[k] as usize;
                        if v < count {
                            per_vertex[v].push((b as i32, bone.weight[k]));
                            any = true;
                        }
                    }
                }
                if !any {
                    if let Some(sp) = nif.get::<NifSkinPartition>(si.partition) {
                        // weights kept only per partition
                        for part in &sp.parts {
                            let (Some(vmap), Some(weights), Some(bone_index)) = (&part.vertex_map, &part.weights, &part.bone_index) else { continue };
                            for v in 0..vmap.len() {
                                for k in 0..part.per_vertex as usize {
                                    let at_i = (v * part.per_vertex as usize + k) as i64;
                                    let w = at(weights, at_i, "partition weight")?;
                                    let vi = vmap[v] as usize;
                                    let b = at(&part.bones, at(bone_index, at_i, "partition bone index")? as i64, "partition bone")? as i32;
                                    if w > 0.0 && vi < count && !per_vertex[vi].iter().any(|x| x.0 == b) {
                                        per_vertex[vi].push((b, w));
                                    }
                                }
                            }
                        }
                    }
                }
                let mut joints = vec![0u8; count * 4];
                let mut weights = vec![0f32; count * 4];
                for v in 0..count {
                    let mut sorted = per_vertex[v].clone();
                    sorted.sort_by(|a, b| cmp_f32(b.1, a.1)); // OrderByDescending: stable
                    sorted.truncate(4);
                    // LINQ Sum over floats adds up in a double
                    let sum = sorted.iter().map(|x| x.1 as f64).sum::<f64>() as f32;
                    for (k, x) in sorted.iter().enumerate() {
                        joints[v * 4 + k] = x.0 as u8;
                        weights[v * 4 + k] = if sum > 0.0 { x.1 / sum } else { 0.0 };
                    }
                    if sorted.is_empty() {
                        weights[v * 4] = 1.0;
                    }
                }
                let root_to_skin = trs(&d(&sd.t), &d(&sd.r), sd.s as f64);
                let bone_nodes: Vec<usize> = si.bones.iter().map(|b| by_block.get(b).copied().unwrap_or(root_id)).collect();
                let bind: Vec<[f64; 12]> = sd.bones.iter().map(|b| trs(&d(&b.t), &d(&b.r), b.s as f64)).collect();
                // In the rest pose every bone must put the mesh where it already is: a check of the matrix conventions.
                let pre = mul(&mul(&arena[ni].world, &root_to_skin), &inv(&arena[root_id].world));
                for b in 0..bind.len().min(bone_nodes.len()) {
                    let m = mul(&mul(&pre, &arena[bone_nodes[b]].world), &bind[b]);
                    for i in 0..12 {
                        worst_bind = net_max(worst_bind, (m[i] - arena[ni].world[i]).abs());
                    }
                }
                bones = bind.len();
                let mut j = Json { s: String::new(), bad: false };
                j.raw("{");
                j.key(true, "root");
                j.int(arena[root_id].index);
                j.key(false, "rootToSkin");
                j.doubles(&round_all(&root_to_skin));
                j.key(false, "bones");
                j.raw("[");
                for (i, b) in bone_nodes.iter().enumerate() {
                    if i > 0 {
                        j.raw(",");
                    }
                    j.int(arena[*b].index);
                }
                j.raw("]");
                j.key(false, "bind");
                j.raw("[");
                for (i, b) in bind.iter().enumerate() {
                    if i > 0 {
                        j.raw(",");
                    }
                    j.doubles(&round_all(b));
                }
                j.raw("]");
                j.key(false, "joints");
                let at_j = bin.add_u8(&joints);
                j.int(at_j);
                j.key(false, "weights");
                let at_w = bin.add_f32(&weights);
                j.int(at_w);
                j.raw("}");
                bad |= j.bad;
                skin_json = Some(j.s);
            }
        }
        let face;
        let normals: &[f32] = match &gd.nrm {
            Some(n) => n,
            None => {
                face = face_normals(gd, gpos)?;
                &face
            }
        };
        let mut j = Json { s: String::new(), bad: false };
        j.raw("{");
        j.key(true, "name");
        j.str(&n_name);
        j.key(false, "node");
        j.int(arena[ni].index);
        j.key(false, "hidden");
        j.bool(hidden);
        j.key(false, "verts");
        j.int(gd.count);
        j.key(false, "indices");
        j.int(gd.tris.len() as i32);
        j.key(false, "pos");
        let v = bin.add_f32(gpos);
        j.int(v);
        j.key(false, "nrm");
        let v = bin.add_f32(normals);
        j.int(v);
        j.key(false, "uv");
        let v = match &gd.uv {
            Some(u) => bin.add_f32(u),
            None => -1,
        };
        j.int(v);
        j.key(false, "col");
        let v = match &gd.col {
            Some(c) => bin.add_f32(c),
            None => -1,
        };
        j.int(v);
        j.key(false, "idx");
        let v = bin.add_u16(&gd.tris);
        j.int(v);
        j.key(false, "texture");
        j.int(tex);
        j.key(false, "diffuse");
        match mat {
            Some(m) => j.doubles(&round_all(&d(&m.diffuse))),
            None => j.doubles(&[1.0, 1.0, 1.0]),
        }
        j.key(false, "emissive");
        match mat {
            Some(m) => j.doubles(&round_all(&d(&m.emissive))),
            None => j.doubles(&[0.0, 0.0, 0.0]),
        }
        j.key(false, "ambient");
        match mat {
            Some(m) => j.doubles(&round_all(&d(&m.ambient))),
            None => j.doubles(&[1.0, 1.0, 1.0]),
        }
        j.key(false, "alpha");
        j.num32(mat.map(|m| m.alpha).unwrap_or(1.0));
        j.key(false, "blend");
        j.bool(alpha.is_some_and(|a| (a.flags & 1) != 0));
        j.key(false, "alphaTest");
        match alpha {
            Some(a) if (a.flags & 0x200) != 0 => j.num(a.threshold as f64 / 255.0),
            _ => j.num(-1.0),
        }
        j.key(false, "skin");
        match &skin_json {
            Some(s) => j.raw(s),
            None => j.raw("null"),
        }
        j.raw("}");
        bad |= j.bad;
        meshes.push(j.s);
        say(format!(
            "mesh {} {:>5} vertices {:>5} triangles  bones {:>3}  texture {}  {}{}{}",
            pad_right(&show(&n_name), 16),
            gd.count,
            gd.tris.len() / 3,
            bones,
            tex,
            if hidden { "hidden" } else { "" },
            if gd.col.is_some() { " colours" } else { "" },
            match alpha {
                Some(a) => format!(" alpha flags {:X}", a.flags),
                None => String::new(),
            }
        ));
    }
    say(format!(
        "{} nodes, {} meshes, {} textures; rest pose check: off by {} at most",
        order.len(),
        meshes.len(),
        textures.len(),
        fmt::custom(worst_bind, "0.#####")
    ));

    // ---- animations ----
    let mut anims: Vec<String> = Vec::new();
    for s in &sequences {
        let kf = &s.file;
        let Some(seq) = kf.get::<NifSequence>(s.seq as i32) else { continue };
        let start: f32 = seq.start;
        let stop: f32 = seq.stop.max(seq.start);
        let frames = ((((stop - start) * FPS as f32) as f64 - 1e-4).ceil() as i32).wrapping_add(1).max(1);
        let mut tracks: Vec<String> = Vec::new();
        let mut fades: Vec<String> = Vec::new();
        let mut moving = 0;
        for link in &seq.links {
            let Some(node_name) = &link.node else { continue };
            let Some(&node_id) = by_name.get(node_name) else { continue };
            // A mesh fading in and out (NiAlphaController on its material: Alilat's glyph shell over its stone): its
            // alpha every frame, as "fades" [{node, alpha}] beside the tracks (only in animations that have one).
            if link.controller_type.as_deref() == Some("NiAlphaController") {
                if let Some(alpha) = float_frames(kf, link.interpolator, start, stop, frames) {
                    let mut j = Json { s: String::new(), bad: false };
                    j.raw("{");
                    j.key(true, "node");
                    j.int(arena[node_id].index);
                    j.key(false, "alpha");
                    let at = bin.add_f32(&alpha);
                    j.int(at);
                    j.raw("}");
                    bad |= j.bad;
                    fades.push(j.s);
                }
                continue;
            }
            if let Some(ct) = &link.controller_type {
                if ct != "NiTransformController" {
                    continue;
                }
            }
            let Some(interp) = kf.get::<NifBlock>(link.interpolator) else { continue };
            if !matches!(interp.kind, NifKind::TransformInterpolator(_) | NifKind::SplineInterpolator(_)) {
                continue;
            }
            let frames_u = frames as usize;
            let mut rot = vec![0i16; frames_u * 4];
            let mut pos = vec![0f32; frames_u * 3];
            let mut scl = vec![0f32; frames_u];
            let mut q0: Option<[f64; 4]> = None;
            let mut prev: Option<[f64; 4]> = None;
            let (mut rot_moves, mut pos_moves, mut scl_moves) = (false, false, false);
            for f in 0..frames_u {
                let time = net_min(stop as f64, start as f64 + f as f64 / FPS as f64);
                let (mut t, mut q, sc) = eval(kf, interp, time, &arena[node_id]);
                // The root's travel across the ground is the game's to apply; the assistant stays in its corner.
                if seq.accum_root.as_deref() == Some(arena[node_id].name.as_str()) && f > 0 {
                    t[0] = pos[0] as f64;
                    t[1] = pos[1] as f64;
                }
                if let Some(p) = prev {
                    if p[0] * q[0] + p[1] * q[1] + p[2] * q[2] + p[3] * q[3] < 0.0 {
                        for x in &mut q {
                            *x = -*x;
                        }
                    }
                }
                prev = Some(q);
                let q0v = *q0.get_or_insert(q);
                rot[f * 4] = q15(q[1]);
                rot[f * 4 + 1] = q15(q[2]);
                rot[f * 4 + 2] = q15(q[3]);
                rot[f * 4 + 3] = q15(q[0]);
                for i in 0..3 {
                    pos[f * 3 + i] = t[i] as f32;
                }
                scl[f] = sc as f32;
                for i in 0..4 {
                    rot_moves |= (q[i] - q0v[i]).abs() > 1e-4;
                }
                for i in 0..3 {
                    pos_moves |= ((pos[f * 3 + i] - pos[i]).abs() as f64) > 1e-4;
                }
                scl_moves |= ((scl[f] - scl[0]).abs() as f64) > 1e-4;
            }
            if rot_moves || pos_moves || scl_moves {
                moving += 1;
            }
            let q0 = q0.unwrap_or([1.0, 0.0, 0.0, 0.0]);
            let mut j = Json { s: String::new(), bad: false };
            j.raw("{");
            j.key(true, "node");
            j.int(arena[node_id].index);
            j.key(false, "r");
            if rot_moves {
                j.raw("null");
            } else {
                j.doubles(&[r5(q0[1]), r5(q0[2]), r5(q0[3]), r5(q0[0])]);
            }
            j.key(false, "rot");
            let v = if rot_moves { bin.add_i16(&rot) } else { -1 };
            j.int(v);
            j.key(false, "p");
            if pos_moves {
                j.raw("null");
            } else {
                j.doubles(&[r5(pos[0] as f64), r5(pos[1] as f64), r5(pos[2] as f64)]);
            }
            j.key(false, "pos");
            let v = if pos_moves { bin.add_f32(&pos) } else { -1 };
            j.int(v);
            j.key(false, "s");
            if scl_moves {
                j.raw("null");
            } else {
                j.num(r5(scl[0] as f64));
            }
            j.key(false, "scl");
            let v = if scl_moves { bin.add_f32(&scl) } else { -1 };
            j.int(v);
            j.raw("}");
            bad |= j.bad;
            tracks.push(j.s);
        }
        let keys: &[(f32, Option<String>)] = kf.get::<NifTextKeys>(seq.text_keys).map(|t| t.keys.as_slice()).unwrap_or(&[]);
        let mut j = Json { s: String::new(), bad: false };
        j.raw("{");
        j.key(true, "name");
        j.str(&s.name);
        j.key(false, "seconds");
        j.num(r5((stop - start) as f64));
        j.key(false, "frames");
        j.int(frames);
        j.key(false, "fps");
        j.int(FPS);
        j.key(false, "loop");
        j.bool(seq.cycle == 0);
        j.key(false, "tracks");
        j.raw("[");
        j.raw(&tracks.join(","));
        j.raw("]");
        if !fades.is_empty() {
            j.key(false, "fades");
            j.raw("[");
            j.raw(&fades.join(","));
            j.raw("]");
        }
        j.raw("}");
        bad |= j.bad;
        anims.push(j.s);
        say(format!(
            "animation {} {:>5} s {} {:>3} nodes ({} moving)  keys: {}",
            pad_right(&s.name, 12),
            custom32(stop - start, "0.00"),
            if seq.cycle == 0 { "loop " } else { "once " },
            tracks.len(),
            moving,
            keys.iter().map(|(t, text)| format!("{}@{}", text.as_deref().unwrap_or(""), custom32(*t, "0.##"))).collect::<Vec<_>>().join(" ")
        ));
    }

    // bounds of the visible meshes in the rest pose, so the page can frame the model
    let mut min = [1e9f64; 3];
    let mut max = [-1e9f64; 3];
    for &ni in &order {
        let Some(block) = arena[ni].block else { continue };
        let Some(geom) = nif.get::<NifGeom>(block) else { continue };
        let Some(gd) = nif.get::<NifGeomData>(geom.data) else { continue };
        let Some(gpos) = gd.pos.as_ref() else { continue };
        if is_hidden(&arena, ni) {
            continue;
        }
        let w = &arena[ni].world;
        for v in 0..gd.count.max(0) as usize {
            for i in 0..3 {
                let x = w[i * 4] * at(gpos, (v * 3) as i64, "vertex")? as f64
                    + w[i * 4 + 1] * at(gpos, (v * 3 + 1) as i64, "vertex")? as f64
                    + w[i * 4 + 2] * at(gpos, (v * 3 + 2) as i64, "vertex")? as f64
                    + w[i * 4 + 3];
                min[i] = net_min(min[i], x);
                max[i] = net_max(max[i], x);
            }
        }
    }
    let f1 = |x: f64| fmt::custom(x, "0.#");
    say(format!(
        "bounds {} {} {} .. {} {} {}; data {} KB",
        f1(min[0]),
        f1(min[1]),
        f1(min[2]),
        f1(max[0]),
        f1(max[1]),
        f1(max[2]),
        bin.0.len() / 1024
    ));

    let mut j = Json { s: String::new(), bad };
    j.raw("{");
    j.key(true, "format");
    j.int(FORMAT);
    j.key(false, "model");
    j.str(model);
    // Z is up, as in the game. Quaternions are x, y, z, w; matrices are 3 rows of 4 (rotation and scale, then translation).
    j.key(false, "min");
    j.doubles(&round_all(&min));
    j.key(false, "max");
    j.doubles(&round_all(&max));
    j.key(false, "nodes");
    j.raw("[");
    for (i, &id) in order.iter().enumerate() {
        if i > 0 {
            j.raw(",");
        }
        let n = &arena[id];
        j.raw("{");
        j.key(true, "name");
        j.str(&show(&n.name));
        j.key(false, "parent");
        j.int(n.parent.map(|p| arena[p].index).unwrap_or(-1));
        j.key(false, "t");
        j.doubles(&round_all(&n.t));
        j.key(false, "q");
        j.doubles(&[r5(n.q[1]), r5(n.q[2]), r5(n.q[3]), r5(n.q[0])]);
        j.key(false, "s");
        j.num(r5(n.s));
        j.raw("}");
    }
    j.raw("]");
    j.key(false, "meshes");
    j.raw("[");
    j.raw(&meshes.join(","));
    j.raw("]");
    j.key(false, "textures");
    j.raw("[");
    for (i, t) in textures.iter().enumerate() {
        if i > 0 {
            j.raw(",");
        }
        j.str(t);
    }
    j.raw("]");
    j.key(false, "anims");
    j.raw("[");
    j.raw(&anims.join(","));
    j.raw("]");
    j.key(false, "bin");
    j.str(&base64(&bin.into_bytes()));
    j.raw("}");
    if j.bad {
        return Err(anyhow!("The model has a number that is not finite (JSON cannot hold it)."));
    }
    Ok(Some(j.s))
}

// ---------- animation ----------

fn valid(v: f32) -> bool {
    v > -3.0e38f32 && !v.is_nan()
}

/// A node's translation, rotation (w, x, y, z) and scale at a time of one animation; channels the animation leaves
/// alone keep the rest pose.
fn eval(kf: &NifFile, interp: &NifBlock, time: f64, rest: &Node) -> ([f64; 3], [f64; 4], f64) {
    let mut t = rest.t;
    let mut q = rest.q;
    let mut s = rest.s;
    match &interp.kind {
        NifKind::TransformInterpolator(ti) => {
            if valid(ti.t[0]) {
                t = d(&ti.t);
            }
            if valid(ti.q[0]) {
                q = d(&ti.q);
            }
            if valid(ti.s) {
                s = ti.s as f64;
            }
            if let Some(dd) = kf.get::<NifTransformData>(ti.data) {
                if let Some(r) = &dd.rotation {
                    q = keys(r, time).try_into().unwrap_or(q);
                } else if let Some(e) = dd.euler.as_ref().filter(|e| e.iter().any(|k| k.is_some())) {
                    let axis = |i: usize| -> [f64; 4] {
                        let a = e[i].as_ref().map(|k| keys(k, time)[0]).unwrap_or(0.0);
                        let mut r = [0.0; 4];
                        r[0] = (a / 2.0).cos();
                        r[1 + i] = (a / 2.0).sin();
                        r
                    };
                    // turn about X first, then Y, then Z: the order that gives back the model's own rest pose (checked on the collar bones)
                    q = qmul(&qmul(&axis(2), &axis(1)), &axis(0));
                }
                if let Some(tr) = &dd.translation {
                    t = keys(tr, time).try_into().unwrap_or(t);
                }
                if let Some(sc) = &dd.scale {
                    s = keys(sc, time)[0];
                }
            }
        }
        NifKind::SplineInterpolator(si) => {
            if valid(si.t[0]) {
                t = d(&si.t);
            }
            if valid(si.q[0]) {
                q = d(&si.q);
            }
            if valid(si.s) {
                s = si.s as f64;
            }
            let data = kf.get::<NifSplineData>(si.data);
            let count = kf.get::<NifSplineBasis>(si.basis).map(|b| b.count).unwrap_or(0);
            if let (Some(data), true) = (data, count > 0) {
                let u = if si.stop > si.start { ((time - si.start as f64) / ((si.stop - si.start) as f64)).clamp(0.0, 1.0) } else { 0.0 };
                if si.translation_at != 0xFFFF && si.translation_at != u32::MAX {
                    if let Some(v) = spline(&data.shorts, si.translation_at as i32, count, 3, u, si.translation_offset as f64, si.translation_range as f64) {
                        t = v.try_into().unwrap_or(t);
                    }
                }
                if si.rotation_at != 0xFFFF && si.rotation_at != u32::MAX {
                    if let Some(v) = spline(&data.shorts, si.rotation_at as i32, count, 4, u, si.rotation_offset as f64, si.rotation_range as f64) {
                        q = v.try_into().unwrap_or(q);
                    }
                }
                if si.scale_at != 0xFFFF && si.scale_at != u32::MAX {
                    if let Some(v) = spline(&data.shorts, si.scale_at as i32, count, 1, u, si.scale_offset as f64, si.scale_range as f64) {
                        s = v[0];
                    }
                }
            }
        }
        _ => {}
    }
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len > 1e-9 {
        for x in &mut q {
            *x /= len;
        }
    } else {
        q = [1.0, 0.0, 0.0, 0.0];
    }
    (t, q, s)
}

fn keys(k: &NifKeys, time: f64) -> Vec<f64> {
    let n = k.times.len();
    let size = k.size as usize;
    let at_key = |i: usize| -> Vec<f64> { (0..size).map(|c| k.values[i * size + c] as f64).collect() };
    if n == 1 || time <= k.times[0] as f64 {
        return at_key(0);
    }
    if time >= k.times[n - 1] as f64 {
        return at_key(n - 1);
    }
    let (mut lo, mut hi) = (0usize, n - 1);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if k.times[mid] as f64 <= time {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let span = (k.times[hi] - k.times[lo]) as f64;
    let u = if span > 0.0 { (time - k.times[lo] as f64) / span } else { 0.0 };
    let a = at_key(lo);
    let mut b = at_key(hi);
    if k.type_ == 5 {
        return a;
    }
    let mut res = vec![0f64; size];
    if size == 4 {
        let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        if dot < 0.0 {
            dot = -dot;
            for x in &mut b {
                *x = -*x;
            }
        }
        let (mut wa, mut wb) = (1.0 - u, u);
        if dot < 0.9995 {
            let th = dot.acos();
            let sn = th.sin();
            wa = ((1.0 - u) * th).sin() / sn;
            wb = (u * th).sin() / sn;
        }
        for c in 0..4 {
            res[c] = wa * a[c] + wb * b[c];
        }
    } else if let (2, Some(kin), Some(kout)) = (k.type_, &k.in_, &k.out) {
        for c in 0..size {
            // the game's own curve between two such keys: leaves along the first key's out tangent, arrives along the second's in tangent
            let dd = b[c] - a[c];
            let o = kout[lo * size + c] as f64;
            let i = kin[hi * size + c] as f64;
            res[c] = a[c] + u * (o + u * ((3.0 * dd - (i + 2.0 * o)) + u * (o + i - 2.0 * dd)));
        }
    } else {
        for c in 0..size {
            res[c] = a[c] + (b[c] - a[c]) * u;
        }
    }
    res
}

/// A point of a compressed cubic B-spline (open uniform knots): control points are shorts, value = offset + range * short / 32767.
fn spline(shorts: &[i16], at: i32, count: i32, size: i32, u: f64, offset: f64, range: f64) -> Option<Vec<f64>> {
    const DEG: i32 = 3;
    if at < 0 || at as i64 + count as i64 * size as i64 > shorts.len() as i64 {
        return None;
    }
    let cp = |i: i32| -> Vec<f64> { (0..size).map(|c| offset + range * shorts[(at + i * size + c) as usize] as f64 / 32767.0).collect() };
    if count <= DEG {
        return Some(cp(0));
    }
    let spans = count - DEG;
    let x = u * spans as f64;
    let k = (x as i32).min(spans - 1) + DEG;
    let knot = |j: i32| -> f64 {
        if j <= DEG {
            0.0
        } else if j >= count {
            spans as f64
        } else {
            (j - DEG) as f64
        }
    };
    let mut p: Vec<Vec<f64>> = (0..=DEG).map(|j| cp(k - DEG + j)).collect();
    for r in 1..=DEG {
        for j in (r..=DEG).rev() {
            let i = k - DEG + j;
            let den = knot(i + DEG - r + 1) - knot(i);
            let a = if den > 0.0 { (x - knot(i)) / den } else { 0.0 };
            for c in 0..size as usize {
                p[j as usize][c] = (1.0 - a) * p[j as usize - 1][c] + a * p[j as usize][c];
            }
        }
    }
    Some(p.swap_remove(DEG as usize))
}

// ---------- texture ----------

fn texture_png(nif: &NifFile, src: &NifSourceTexture, dir: &Path) -> Result<Option<Vec<u8>>> {
    let mut rgba: Option<Vec<u8>> = None;
    let (mut w, mut h) = (0, 0);
    if let Some(px) = nif.get::<NifPixelData>(src.pixels) {
        w = px.width;
        h = px.height;
        rgba = match px.format {
            0 if px.bits_per_pixel == 24 => expand(&px.data, w, h, 3),
            1 if px.bits_per_pixel == 32 => expand(&px.data, w, h, 4),
            1 if px.bits_per_pixel == 16 && px.channel_bits.starts_with(&[5, 5, 5, 1]) => expand_5551(&px.data, w, h),
            4 => dxt(&px.data, w, h, 1),
            5 => dxt(&px.data, w, h, 3),
            6 => dxt(&px.data, w, h, 5),
            _ => None,
        };
    } else if let Some(file) = &src.file {
        // a texture kept beside the model: a .dds file
        let name = file.replace('\\', "/");
        let name = name.rsplit('/').next().unwrap_or("").to_string();
        let found = [dir.join(&name), dir.join("Textures").join(&name)].into_iter().find(|p| p.is_file());
        let Some(path) = found else { return Ok(None) };
        return Ok(dds_to_png(&std::fs::read(path)?));
    }
    Ok(rgba.map(|r| tga::rgba_to_png(&r, w, h)))
}

/// A NiFloatInterpolator's value at every frame of an animation (30 a second from `start`), its keys read linearly;
/// None when the block isn't one.
fn float_frames(kf: &NifFile, interpolator: i32, start: f32, stop: f32, frames: i32) -> Option<Vec<f32>> {
    let NifKind::FloatInterpolator(fi) = &kf.get::<NifBlock>(interpolator)?.kind else { return None };
    let keys = match kf.get::<NifBlock>(fi.data).map(|b| &b.kind) {
        Some(NifKind::FloatData(Some(k))) if !k.times.is_empty() && k.values.len() >= k.times.len() => Some(k),
        _ => None,
    };
    let at = |time: f32| -> f32 {
        let Some(k) = keys else { return fi.value };
        let (t, v) = (&k.times, &k.values);
        if time <= t[0] {
            return v[0];
        }
        for i in 1..t.len() {
            if time <= t[i] {
                let span = t[i] - t[i - 1];
                let a = if span > 0.0 { (time - t[i - 1]) / span } else { 1.0 };
                return v[i - 1] + (v[i] - v[i - 1]) * a;
            }
        }
        v[t.len() - 1]
    };
    Some((0..frames.max(1)).map(|f| at((start + f as f32 / FPS as f32).min(stop)).clamp(0.0, 1.0)).collect())
}

// ---------- the summoning effect ----------

/// The pieces of the game's summoning effect the page draws when the assistant changes demon, as one JSON object:
/// `{"textures":{name: PNG data URI},"sound": WAV data URI or null}`. They are the client's own files: the light pillar,
/// lightning, glow and swirl models and the smoke and sparkle pictures of the effect G002_07_P00_01H (the effect the
/// client picks next to the summoning sound, System_syoukan.wav "召喚", sound 2022), and that sound. How the effect plays
/// them over time (effect\eff\*.eff) is not decoded: the page's timing is its own. `None` without the client's folder.
pub fn summon_effect(client_dir: &Path) -> Result<Option<String>> {
    let fx = client_dir.join("effect");
    if !fx.is_dir() {
        return Ok(None);
    }
    // the effect's models each hold one picture (NiPixelData); smoke and sparkles are loose .dds files
    const MODELS: [(&str, &str); 5] = [
        ("pillar", "Eg48_m0s0_H_T"),
        ("lightning", "Eg52_m0s1_H_N"),
        ("glow", "Eg32_m0s0_H_F"),
        ("arc", "Eg42_m0s0_H_T"),
        ("sparks", "Ec13_m0s1_H_T"),
    ];
    const PICTURES: [(&str, &str); 2] = [("smoke", "E004_01_a0p00c0H_F"), ("sparkle", "E020_02_a1p09c0H_F")];
    let mut out = String::from("{\"textures\":{");
    let mut first = true;
    let mut add = |out: &mut String, name: &str, png: Option<Vec<u8>>| {
        let Some(png) = png else { return };
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("\"{name}\":\"data:image/png;base64,{}\"", base64(&png)));
    };
    for (name, file) in MODELS {
        let path = fx.join("model").join(format!("{file}.nif"));
        let png = match path.is_file().then(|| NifFile::load(&path)) {
            Some(Ok(nif)) => match nif.all::<NifSourceTexture>().next() {
                Some(src) => texture_png(&nif, src, &fx.join("tex"))?,
                None => None,
            },
            Some(Err(e)) if is_nif_error(&e) => None,
            Some(Err(e)) => return Err(e),
            None => None,
        };
        add(&mut out, name, png);
    }
    for (name, file) in PICTURES {
        let path = fx.join("tex").join(format!("{file}.dds"));
        add(&mut out, name, if path.is_file() { dds_to_png(&std::fs::read(&path)?) } else { None });
    }
    out.push_str("},\"sound\":");
    let wav = client_dir.join("sound").join("se").join("System_syoukan.wav");
    match std::fs::read(&wav).ok().filter(|b| b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WAVE") {
        Some(b) => out.push_str(&format!("\"data:audio/wav;base64,{}\"", base64(&b))),
        None => out.push_str("null"),
    }
    out.push('}');
    Ok(Some(out))
}

/// A .dds picture (DXT1 / DXT3 / DXT5, or plain 32-bit BGRA) as PNG; None for anything else. The client keeps some
/// icons this way beside the .tga ones (skill icons such as icon_SD01_0019a.DDS).
pub fn dds_to_png(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 128 || d[0] != b'D' || d[1] != b'D' || d[2] != b'S' {
        return None;
    }
    let i32_at = |o: usize| i32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let h = i32_at(12);
    let w = i32_at(16);
    if w <= 0 || h <= 0 || w > 4096 || h > 4096 {
        return None;
    }
    let four_cc: String = d[84..88].iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
    let body = &d[128..];
    let rgba = match four_cc.as_str() {
        "DXT1" => dxt(body, w, h, 1),
        "DXT3" => dxt(body, w, h, 3),
        "DXT5" => dxt(body, w, h, 5),
        _ if (i32_at(80) & 0x4) == 0 && i32_at(88) == 32 && body.len() >= (w * h * 4) as usize => Some(bgra(body, w, h)),
        _ => None,
    };
    rgba.map(|r| tga::rgba_to_png(&r, w, h))
}

fn bgra(d: &[u8], w: i32, h: i32) -> Vec<u8> {
    let n = (w * h) as usize;
    let mut o = vec![0u8; n * 4];
    for i in 0..n {
        o[i * 4] = d[i * 4 + 2];
        o[i * 4 + 1] = d[i * 4 + 1];
        o[i * 4 + 2] = d[i * 4];
        o[i * 4 + 3] = d[i * 4 + 3];
    }
    o
}

fn expand(d: &[u8], w: i32, h: i32, bytes: usize) -> Option<Vec<u8>> {
    if w <= 0 || h <= 0 || w > 65536 || h > 65536 {
        return None;
    }
    let n = (w as usize) * (h as usize);
    if d.len() < n * bytes {
        return None;
    }
    let mut o = vec![0u8; n * 4];
    for i in 0..n {
        o[i * 4] = d[i * bytes];
        o[i * 4 + 1] = d[i * bytes + 1];
        o[i * 4 + 2] = d[i * bytes + 2];
        o[i * 4 + 3] = if bytes == 4 { d[i * 4 + 3] } else { 255 };
    }
    Some(o)
}

/// 16-bit pixels (NiPixelData format 1 with 5,5,5,1 bit channels R,G,B,A) to RGBA. The channels sit in the order the
/// file lists them from the lowest bit up, as the 32-bit RGBA format's bytes do (R in the lowest). Alilat's two
/// pictures are kept this way.
fn expand_5551(d: &[u8], w: i32, h: i32) -> Option<Vec<u8>> {
    if w <= 0 || h <= 0 || w > 65536 || h > 65536 {
        return None;
    }
    let n = (w as usize) * (h as usize);
    if d.len() < n * 2 {
        return None;
    }
    let five = |v: u16| ((v as u32 * 255 + 15) / 31) as u8;
    let mut o = vec![0u8; n * 4];
    for i in 0..n {
        let v = u16::from_le_bytes([d[i * 2], d[i * 2 + 1]]);
        o[i * 4] = five(v & 31);
        o[i * 4 + 1] = five((v >> 5) & 31);
        o[i * 4 + 2] = five((v >> 10) & 31);
        o[i * 4 + 3] = if v >> 15 != 0 { 255 } else { 0 };
    }
    Some(o)
}

/// DXT1 / DXT3 / DXT5 blocks to RGBA.
fn dxt(d: &[u8], w: i32, h: i32, kind: i32) -> Option<Vec<u8>> {
    if w <= 0 || h <= 0 || w > 65536 || h > 65536 {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let block = if kind == 1 { 8 } else { 16 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if d.len() < bw * bh * block {
        return None;
    }
    let mut o = vec![0u8; w * h * 4];
    let mut col = [0u8; 16];
    let mut al = [0u8; 8];
    for by in 0..bh {
        for bx in 0..bw {
            let p = (by * bw + bx) * block;
            let c = if kind == 1 { p } else { p + 8 };
            let c0 = d[c] as i32 | (d[c + 1] as i32) << 8;
            let c1 = d[c + 2] as i32 | (d[c + 3] as i32) << 8;
            let mut rgb = |v: i32, i: usize| {
                col[i * 4] = ((v >> 11) * 255 / 31) as u8;
                col[i * 4 + 1] = (((v >> 5) & 63) * 255 / 63) as u8;
                col[i * 4 + 2] = ((v & 31) * 255 / 31) as u8;
                col[i * 4 + 3] = 255;
            };
            rgb(c0, 0);
            rgb(c1, 1);
            let four = kind != 1 || c0 > c1;
            for i in 0..3 {
                let (a, b) = (col[i] as i32, col[4 + i] as i32);
                col[8 + i] = if four { (2 * a + b + 1) / 3 } else { (a + b) / 2 } as u8;
                col[12 + i] = if four { (a + 2 * b + 1) / 3 } else { 0 } as u8;
            }
            col[11] = 255;
            col[15] = if four { 255 } else { 0 };
            let bits = u32::from_le_bytes([d[c + 4], d[c + 5], d[c + 6], d[c + 7]]);
            let mut abits: u64 = 0;
            if kind == 5 {
                al[0] = d[p];
                al[1] = d[p + 1];
                for i in 2..8i32 {
                    let (a0, a1) = (al[0] as i32, al[1] as i32);
                    al[i as usize] = if a0 > a1 {
                        ((8 - i) * a0 + (i - 1) * a1) / 7
                    } else if i < 6 {
                        ((6 - i) * a0 + (i - 1) * a1) / 5
                    } else if i == 6 {
                        0
                    } else {
                        255
                    } as u8;
                }
                for i in 0..6 {
                    abits |= (d[p + 2 + i] as u64) << (8 * i);
                }
            }
            for y in 0..4 {
                for x in 0..4 {
                    let (px, py, i) = (bx * 4 + x, by * 4 + y, y * 4 + x);
                    if px >= w || py >= h {
                        continue;
                    }
                    let ci = ((bits >> (2 * i)) & 3) as usize;
                    let to = (py * w + px) * 4;
                    o[to] = col[ci * 4];
                    o[to + 1] = col[ci * 4 + 1];
                    o[to + 2] = col[ci * 4 + 2];
                    o[to + 3] = col[ci * 4 + 3];
                    if kind == 3 {
                        o[to + 3] = (((d[p + i / 2] >> (4 * (i & 1))) & 15) as u32 * 17) as u8;
                    } else if kind == 5 {
                        o[to + 3] = al[((abits >> (3 * i)) & 7) as usize];
                    }
                }
            }
        }
    }
    Some(o)
}

// ---------- small maths: matrices are 3 rows of 4 (rotation * scale | translation), for column vectors ----------

fn d<const N: usize>(a: &[f32; N]) -> [f64; N] {
    let mut o = [0f64; N];
    for i in 0..N {
        o[i] = a[i] as f64;
    }
    o
}

fn r5(v: f64) -> f64 {
    fmt::round(v, 5)
}

fn round_all(a: &[f64]) -> Vec<f64> {
    a.iter().map(|&x| r5(x)).collect()
}

fn q15(v: f64) -> i16 {
    fmt::round(v.clamp(-1.0, 1.0) * 32767.0, 0) as i16
}

static SJIS_NAMES: AtomicBool = AtomicBool::new(true);

/// C# `Show` only decodes Shift-JIS when the process has registered the code pages (`Encoding.RegisterProvider`), which
/// the desktop app has by then (ClientReader) and the command-line `model` command has not (its JSON keeps the raw
/// Latin-1 characters of such names, escaped). On by default (the app's behaviour); pass `false` to get the CLI's bytes.
pub fn set_shift_jis_names(on: bool) {
    SJIS_NAMES.store(on, Ordering::Relaxed);
}

/// Node names are Shift-JIS in places ("Bip01 尾"); shown as text only, never matched.
fn show(latin1: &str) -> String {
    if latin1.is_ascii() || !SJIS_NAMES.load(Ordering::Relaxed) {
        return latin1.to_string();
    }
    // the names were read one byte per char (Latin-1)
    let bytes: Vec<u8> = latin1.chars().map(|c| c as u32 as u8).collect();
    encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes).0.into_owned()
}

fn trs(t: &[f64; 3], r: &[f64; 9], s: f64) -> [f64; 12] {
    [r[0] * s, r[1] * s, r[2] * s, t[0], r[3] * s, r[4] * s, r[5] * s, t[1], r[6] * s, r[7] * s, r[8] * s, t[2]]
}

fn mul(a: &[f64; 12], b: &[f64; 12]) -> [f64; 12] {
    let mut m = [0f64; 12];
    for i in 0..3 {
        for j in 0..4 {
            m[i * 4 + j] = a[i * 4] * b[j] + a[i * 4 + 1] * b[4 + j] + a[i * 4 + 2] * b[8 + j] + if j == 3 { a[i * 4 + 3] } else { 0.0 };
        }
    }
    m
}

fn inv(m: &[f64; 12]) -> [f64; 12] {
    let (a, b, c, d, e, f, g, h, i) = (m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]);
    let aa = e * i - f * h;
    let bb = -(d * i - f * g);
    let cc = d * h - e * g;
    let det = a * aa + b * bb + c * cc;
    if det.abs() < 1e-12 {
        return [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    }
    let mut r = [
        aa / det,
        (c * h - b * i) / det,
        (b * f - c * e) / det,
        0.0,
        bb / det,
        (a * i - c * g) / det,
        (c * d - a * f) / det,
        0.0,
        cc / det,
        (b * g - a * h) / det,
        (a * e - b * d) / det,
        0.0,
    ];
    for k in 0..3 {
        r[k * 4 + 3] = -(r[k * 4] * m[3] + r[k * 4 + 1] * m[7] + r[k * 4 + 2] * m[11]);
    }
    r
}

/// Rotation matrix (9, by rows) of a quaternion w, x, y, z.
fn mat_of(q: &[f64; 4]) -> [f64; 9] {
    let (w, x, y, z) = (q[0], q[1], q[2], q[3]);
    [
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y - w * z),
        2.0 * (x * z + w * y),
        2.0 * (x * y + w * z),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z - w * x),
        2.0 * (x * z - w * y),
        2.0 * (y * z + w * x),
        1.0 - 2.0 * (x * x + y * y),
    ]
}

/// Quaternion w, x, y, z of a rotation matrix (9, by rows).
fn quat_of(m: &[f32; 9]) -> [f64; 4] {
    // The matrix is float[] in C#: sums and differences of its entries are float arithmetic, widened afterwards.
    let tr = (m[0] + m[4] + m[8]) as f64;
    let (w, x, y, z);
    if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        w = s / 4.0;
        x = (m[7] - m[5]) as f64 / s;
        y = (m[2] - m[6]) as f64 / s;
        z = (m[3] - m[1]) as f64 / s;
    } else if m[0] > m[4] && m[0] > m[8] {
        let s = ((1.0f32 + m[0] - m[4] - m[8]) as f64).sqrt() * 2.0;
        w = (m[7] - m[5]) as f64 / s;
        x = s / 4.0;
        y = (m[1] + m[3]) as f64 / s;
        z = (m[2] + m[6]) as f64 / s;
    } else if m[4] > m[8] {
        let s = ((1.0f32 + m[4] - m[0] - m[8]) as f64).sqrt() * 2.0;
        w = (m[2] - m[6]) as f64 / s;
        x = (m[1] + m[3]) as f64 / s;
        y = s / 4.0;
        z = (m[5] + m[7]) as f64 / s;
    } else {
        let s = ((1.0f32 + m[8] - m[0] - m[4]) as f64).sqrt() * 2.0;
        w = (m[3] - m[1]) as f64 / s;
        x = (m[2] + m[6]) as f64 / s;
        y = (m[5] + m[7]) as f64 / s;
        z = s / 4.0;
    }
    let len = (w * w + x * x + y * y + z * z).sqrt();
    if len > 1e-9 { [w / len, x / len, y / len, z / len] } else { [1.0, 0.0, 0.0, 0.0] }
}

fn qmul(a: &[f64; 4], b: &[f64; 4]) -> [f64; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

fn face_normals(g: &NifGeomData, pos: &[f32]) -> Result<Vec<f32>> {
    let count = g.count.max(0) as usize;
    let mut n = vec![0f32; count * 3];
    let mut t = 0;
    while t + 2 < g.tris.len() {
        let a = g.tris[t] as i64 * 3;
        let b = g.tris[t + 1] as i64 * 3;
        let c = g.tris[t + 2] as i64 * 3;
        let p = |i: i64| at(pos, i, "vertex");
        let (ux, uy, uz) = (p(b)? - p(a)?, p(b + 1)? - p(a + 1)?, p(b + 2)? - p(a + 2)?);
        let (vx, vy, vz) = (p(c)? - p(a)?, p(c + 1)? - p(a + 1)?, p(c + 2)? - p(a + 2)?);
        let (nx, ny, nz) = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx);
        for v in [a, b, c] {
            if v < 0 || v as usize + 2 >= n.len() {
                return Err(oob("normal"));
            }
            let v = v as usize;
            n[v] += nx;
            n[v + 1] += ny;
            n[v + 2] += nz;
        }
        t += 3;
    }
    let mut v = 0;
    while v < n.len() {
        let len = (n[v] * n[v] + n[v + 1] * n[v + 1] + n[v + 2] * n[v + 2]).sqrt();
        if len > 0.0 {
            n[v] /= len;
            n[v + 1] /= len;
            n[v + 2] /= len;
        } else {
            n[v + 2] = 1.0;
        }
        v += 3;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_system_text_json() {
        let mut s = String::new();
        escape_into(&mut s, "a\"b\\c/d<e>f&g'h+i`j\u{7f}\u{1}\n\u{e9}Bip01 尾😀");
        assert_eq!(s, "\"a\\u0022b\\\\c/d\\u003Ce\\u003Ef\\u0026g\\u0027h\\u002Bi\\u0060j\\u007F\\u0001\\n\\u00E9Bip01 \\u5C3E\\uD83D\\uDE00\"");
    }

    #[test]
    fn base64_matches_convert() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xfb, 0xff, 0xfe]), "+//+");
    }

    #[test]
    fn bin_blocks_start_on_multiples_of_four() {
        let mut b = Bin::default();
        assert_eq!(b.add_u8(&[1, 2, 3]), 0);
        assert_eq!(b.add_u16(&[7]), 4);
        assert_eq!(b.add_f32(&[1.0]), 8);
        assert_eq!(b.into_bytes().len(), 12);
    }

    #[test]
    fn dxt1_block_decodes() {
        // one 4x4 block: colour0 = pure red (0xF800), colour1 = pure blue (0x001F), indices all 0
        let blk = [0x00, 0xF8, 0x1F, 0x00, 0, 0, 0, 0];
        let o = dxt(&blk, 4, 4, 1).unwrap();
        assert_eq!(&o[0..4], &[255, 0, 0, 255]);
        let blk = [0x00, 0xF8, 0x1F, 0x00, 0b11_11_11_11, 0, 0, 0]; // index 3 = (c0+2*c1+1)/3 for four-colour blocks
        let o = dxt(&blk, 4, 4, 1).unwrap();
        assert_eq!(&o[0..4], &[85, 0, 170, 255]);
    }

    #[test]
    fn quaternion_of_identity_and_back() {
        let q = quat_of(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(q, [1.0, 0.0, 0.0, 0.0]);
        let m = mat_of(&[0.0, 0.0, 0.0, 1.0]);
        assert!((m[0] - -1.0).abs() < 1e-12 && (m[4] - -1.0).abs() < 1e-12 && (m[8] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn float_helpers_follow_dotnet() {
        assert_eq!(custom32(2.675, "0.00"), "2.68");
        assert!(net_max(f64::NAN, 1.0).is_nan());
        assert_eq!(pad_right("尾x", 4), "尾x  ");
    }
}
