//! Port of `src/Data/Nif.cs`.
//!
//! Reads the client's 3D files (Devil\bone\*.nif models, *.kf animations): Gamebryo "NIF" version 20.1.0.3.
//! That version has no block sizes, so every block type in a file must be known and read exactly; an unknown type
//! stops the read with its name ([`NifError`]). Only the types the demon models use are here.
//! Rotation matrices are kept as the file has them: 9 floats, row by row, for column vectors (v' = R v).
//! Quaternions are w, x, y, z.
//!
//! The C# class hierarchy (NifObject > NifNet > NifAv > NifNode / NifGeom ...) is a [`NifBlock`] (type name, index,
//! [`NifKind`]) here. `Get<T>` / `All<T>` are [`NifFile::get`] / [`NifFile::all`] with `T` any of the block structs
//! (or [`NifBlock`] itself for `NifObject`, or [`NifAv`] for "a node or a geometry"): see [`NifGet`].

use crate::Result;
use std::path::Path;

/// C# `NifException`: the file is not one this reader can read (not a Gamebryo file, another version, an unknown
/// block type, a block that could not be read). Callers that fall back on it (`catch (NifException)`) find it with
/// `err.downcast_ref::<NifError>()`; other problems (a missing file, a damaged header) are other errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct NifError(pub String);

/// True when the error is (or wraps) a [`NifError`].
pub fn is_nif_error(e: &anyhow::Error) -> bool {
    e.downcast_ref::<NifError>().is_some()
}

fn nif_err(message: String) -> anyhow::Error {
    anyhow::Error::new(NifError(message))
}

/// What was out of range in the file (C# `IndexOutOfRangeException` / `ArgumentException` / `OverflowException`).
#[derive(Debug)]
struct OutOfRange;

/// A block read either ran past the data (becomes a [`NifError`] naming the block) or threw a NifException itself.
enum BlockErr {
    Range,
    Nif(String),
}

impl From<OutOfRange> for BlockErr {
    fn from(_: OutOfRange) -> Self {
        BlockErr::Range
    }
}

type R<T> = std::result::Result<T, OutOfRange>;
type B<T> = std::result::Result<T, BlockErr>;

// ---------- block types ----------

/// C# `NifObject`: every block has its type name and its index in the file.
#[derive(Debug, Clone)]
pub struct NifBlock {
    pub type_name: String,
    pub index: i32,
    pub kind: NifKind,
}

/// The block classes of Nif.cs. `Object` is a block that was read and thrown away (C# `new NifObject()`).
#[derive(Debug, Clone)]
pub enum NifKind {
    Object,
    Node(NifNode),
    Geom(NifGeom),
    GeomData(NifGeomData),
    SkinInstance(NifSkinInstance),
    SkinData(NifSkinData),
    SkinPartition(NifSkinPartition),
    /// NiZBufferProperty, NiVertexColorProperty, NiSpecularProperty, NiWireframeProperty, NiDitherProperty, NiShadeProperty
    Property(NifProperty),
    /// NiStencilProperty (C# `NifStencil : NifProperty`)
    Stencil(NifProperty),
    Texturing(NifTexturing),
    SourceTexture(NifSourceTexture),
    PixelData(NifPixelData),
    Material(NifMaterial),
    Alpha(NifAlpha),
    Extra(NifExtra),
    TextKeys(NifTextKeys),
    Controller(NifController),
    TransformInterpolator(NifTransformInterpolator),
    TransformData(NifTransformData),
    SplineInterpolator(NifSplineInterpolator),
    SplineData(NifSplineData),
    SplineBasis(NifSplineBasis),
    Sequence(NifSequence),
    /// NiFloatInterpolator: a value, or its keys (NiFloatData) when data >= 0 (a material's alpha, NiAlphaController)
    FloatInterpolator(NifFloatInterpolator),
    /// NiFloatData: the keys of one number over time
    FloatData(Option<NifKeys>),
}

/// C# `NifNet`: name, extra data refs and controller of an object with a name.
#[derive(Debug, Clone, Default)]
pub struct NifNet {
    pub name: Option<String>,
    pub extra: Vec<i32>,
    pub controller: i32,
}

/// C# `NifAv`: a scene object (node or geometry) with its transform.
#[derive(Debug, Clone, Default)]
pub struct NifAv {
    pub net: NifNet,
    pub flags: i32,
    pub t: [f32; 3],
    pub r: [f32; 9],
    pub s: f32,
    pub properties: Vec<i32>,
}

impl NifAv {
    pub fn name(&self) -> Option<&str> {
        self.net.name.as_deref()
    }
    pub fn hidden(&self) -> bool {
        (self.flags & 1) != 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct NifNode {
    pub av: NifAv,
    pub children: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct NifGeom {
    pub av: NifAv,
    pub data: i32,
    pub skin: i32,
}

#[derive(Debug, Clone, Default)]
pub struct NifGeomData {
    pub count: i32,
    pub pos: Option<Vec<f32>>,
    pub nrm: Option<Vec<f32>>,
    pub col: Option<Vec<f32>>,
    pub uv: Option<Vec<f32>>,
    pub tris: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSkinInstance {
    pub data: i32,
    pub partition: i32,
    pub root: i32,
    pub bones: Vec<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSkinBone {
    pub r: [f32; 9],
    pub t: [f32; 3],
    pub s: f32,
    pub vertex: Vec<u16>,
    pub weight: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSkinData {
    pub r: [f32; 9],
    pub t: [f32; 3],
    pub s: f32,
    pub bones: Vec<NifSkinBone>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSkinPart {
    pub per_vertex: i32,
    pub bones: Vec<u16>,
    pub vertex_map: Option<Vec<u16>>,
    pub weights: Option<Vec<f32>>,
    pub bone_index: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSkinPartition {
    pub parts: Vec<NifSkinPart>,
}

#[derive(Debug, Clone, Default)]
pub struct NifProperty {
    pub net: NifNet,
    pub flags: i32,
}

#[derive(Debug, Clone)]
pub struct NifTexturing {
    pub net: NifNet,
    pub base: i32,
}

#[derive(Debug, Clone)]
pub struct NifSourceTexture {
    pub net: NifNet,
    pub external: bool,
    pub file: Option<String>,
    pub pixels: i32,
}

#[derive(Debug, Clone, Default)]
pub struct NifPixelData {
    pub format: i32,
    pub bits_per_pixel: i32,
    pub bytes_per_pixel: i32,
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
    /// What each stored component is, in file order: 0 red, 1 green, 2 blue, 3 alpha, 4 compressed, 19 none; and its bits.
    pub channels: [i32; 4],
    pub channel_bits: [i32; 4],
}

#[derive(Debug, Clone, Default)]
pub struct NifMaterial {
    pub net: NifNet,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub emissive: [f32; 3],
    pub gloss: f32,
    pub alpha: f32,
}

#[derive(Debug, Clone, Default)]
pub struct NifAlpha {
    pub net: NifNet,
    pub flags: i32,
    pub threshold: i32,
}

/// C# `NifExtra.Value` is an `object`: one of these (or null).
#[derive(Debug, Clone, PartialEq)]
pub enum NifExtraValue {
    None,
    Str(Option<String>),
    Int(i32),
    Bool(bool),
    Float(f32),
    Floats(Vec<f32>),
}

#[derive(Debug, Clone)]
pub struct NifExtra {
    pub name: Option<String>,
    pub value: NifExtraValue,
}

#[derive(Debug, Clone)]
pub struct NifTextKeys {
    pub name: Option<String>,
    /// (time, text)
    pub keys: Vec<(f32, Option<String>)>,
}

#[derive(Debug, Clone)]
pub struct NifController {
    pub next: i32,
    pub flags: i32,
    pub target: i32,
    pub interpolator: i32,
    pub frequency: f32,
    pub phase: f32,
    pub start: f32,
    pub stop: f32,
}

#[derive(Debug, Clone, Default)]
pub struct NifFloatInterpolator {
    pub value: f32,
    pub data: i32,
}

#[derive(Debug, Clone, Default)]
pub struct NifTransformInterpolator {
    pub t: [f32; 3],
    pub q: [f32; 4],
    pub s: f32,
    pub data: i32,
}

/// Keys of one channel. Type: 1 linear, 2 with in/out tangents, 3 TBC (read as linear), 5 constant.
#[derive(Debug, Clone, Default)]
pub struct NifKeys {
    pub type_: i32,
    pub size: i32,
    pub times: Vec<f32>,
    pub values: Vec<f32>,
    pub in_: Option<Vec<f32>>,
    pub out: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Default)]
pub struct NifTransformData {
    pub rotation_type: i32,
    pub rotation: Option<NifKeys>,
    pub translation: Option<NifKeys>,
    pub scale: Option<NifKeys>,
    pub euler: Option<[Option<NifKeys>; 3]>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSplineInterpolator {
    pub start: f32,
    pub stop: f32,
    pub s: f32,
    pub translation_offset: f32,
    pub translation_range: f32,
    pub rotation_offset: f32,
    pub rotation_range: f32,
    pub scale_offset: f32,
    pub scale_range: f32,
    pub data: i32,
    pub basis: i32,
    pub t: [f32; 3],
    pub q: [f32; 4],
    pub translation_at: u32,
    pub rotation_at: u32,
    pub scale_at: u32,
}

#[derive(Debug, Clone, Default)]
pub struct NifSplineData {
    pub floats: Vec<f32>,
    pub shorts: Vec<i16>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSplineBasis {
    pub count: i32,
}

#[derive(Debug, Clone, Default)]
pub struct NifSequenceLink {
    pub interpolator: i32,
    pub controller: i32,
    pub node: Option<String>,
    pub controller_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NifSequence {
    pub name: Option<String>,
    pub accum_root: Option<String>,
    pub links: Vec<NifSequenceLink>,
    pub weight: f32,
    pub frequency: f32,
    pub start: f32,
    pub stop: f32,
    pub text_keys: i32,
    pub cycle: i32,
}

// ---------- typed lookups (C# `Get<T>` / `All<T>`) ----------

/// A type `T` for [`NifFile::get`] / [`NifFile::all`]: the block struct inside a [`NifKind`] variant.
pub trait NifGet: Sized {
    fn from_block(b: &NifBlock) -> Option<&Self>;
}

macro_rules! nif_get {
    ($t:ty, $($variant:ident),+) => {
        impl NifGet for $t {
            fn from_block(b: &NifBlock) -> Option<&Self> {
                match &b.kind { $(NifKind::$variant(x) => Some(x),)+ _ => None }
            }
        }
    };
}

/// `Get<NifObject>`: any block.
impl NifGet for NifBlock {
    fn from_block(b: &NifBlock) -> Option<&Self> {
        Some(b)
    }
}
/// `Get<NifAv>`: a node or a geometry.
impl NifGet for NifAv {
    fn from_block(b: &NifBlock) -> Option<&Self> {
        match &b.kind {
            NifKind::Node(n) => Some(&n.av),
            NifKind::Geom(g) => Some(&g.av),
            _ => None,
        }
    }
}
nif_get!(NifNode, Node);
nif_get!(NifGeom, Geom);
nif_get!(NifGeomData, GeomData);
nif_get!(NifSkinInstance, SkinInstance);
nif_get!(NifSkinData, SkinData);
nif_get!(NifSkinPartition, SkinPartition);
// C# NifStencil derives from NifProperty, so Get<NifProperty> finds both
nif_get!(NifProperty, Property, Stencil);
nif_get!(NifTexturing, Texturing);
nif_get!(NifSourceTexture, SourceTexture);
nif_get!(NifPixelData, PixelData);
nif_get!(NifMaterial, Material);
nif_get!(NifAlpha, Alpha);
nif_get!(NifExtra, Extra);
nif_get!(NifTextKeys, TextKeys);
nif_get!(NifController, Controller);
nif_get!(NifTransformInterpolator, TransformInterpolator);
nif_get!(NifTransformData, TransformData);
nif_get!(NifSplineInterpolator, SplineInterpolator);
nif_get!(NifSplineData, SplineData);
nif_get!(NifSplineBasis, SplineBasis);
nif_get!(NifSequence, Sequence);

// ---------- the file ----------

pub struct NifFile {
    pub strings: Vec<String>,
    pub blocks: Vec<NifBlock>,
    pub roots: Vec<i32>,
}

pub const VERSION: u32 = 0x14010003;

impl NifFile {
    /// C# `Get<T>(index)`: the block at `index` when it is a `T`; None for an index out of range or another type.
    pub fn get<T: NifGet>(&self, index: i32) -> Option<&T> {
        if index >= 0 && (index as usize) < self.blocks.len() { T::from_block(&self.blocks[index as usize]) } else { None }
    }

    /// C# `All<T>()`: every block that is a `T`, in file order.
    pub fn all<'a, T: NifGet + 'a>(&'a self) -> impl Iterator<Item = &'a T> {
        self.blocks.iter().filter_map(T::from_block)
    }

    /// C# `NifFile.Load`: `Err` holds a [`NifError`] when the file is not readable as a NIF, or an I/O error.
    pub fn load(path: &Path) -> Result<NifFile> {
        let bytes = std::fs::read(path)?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        NifFile::read(&bytes, &name)
    }

    /// C# `NifFile.Read`. `Err` holds a [`NifError`] for every problem the C# raised as NifException; a damaged
    /// header (C# threw IndexOutOfRange/ArgumentOutOfRange there) is another error.
    pub fn read(d: &[u8], name: &str) -> Result<NifFile> {
        let Some(nl) = d.iter().position(|&b| b == b'\n') else {
            return Err(nif_err(format!("{name} is not a Gamebryo file.")));
        };
        let head: String = d[..nl].iter().map(|&b| if b < 128 { b as char } else { '?' }).collect();
        if !head.starts_with("Gamebryo File Format") {
            return Err(nif_err(format!("{name} is not a Gamebryo file.")));
        }
        let header_err = |_: OutOfRange| anyhow::anyhow!("{name}: the header is cut short or damaged");
        let mut r = Reader { d, pos: (nl + 1) as i32, strings: Vec::new() };
        let ver = r.u32().map_err(header_err)?;
        if ver != VERSION {
            return Err(nif_err(format!(
                "{name} is NIF version {}.{}.{}.{}; only 20.1.0.3 is read.",
                ver >> 24,
                (ver >> 16) & 255,
                (ver >> 8) & 255,
                ver & 255
            )));
        }
        if r.u8().map_err(header_err)? != 1 {
            return Err(nif_err(format!("{name} is not little-endian.")));
        }
        r.u32().map_err(header_err)?; // user version
        let count = r.i32().map_err(header_err)?;
        let type_count = r.u16().map_err(header_err)? as usize;
        let mut types = Vec::with_capacity(type_count);
        for _ in 0..type_count {
            types.push(r.sized().map_err(header_err)?);
        }
        if count < 0 || count as usize > d.len() {
            return Err(anyhow::anyhow!("{name}: the header is cut short or damaged"));
        }
        let mut type_of = Vec::with_capacity(count as usize);
        for _ in 0..count {
            type_of.push(r.u16().map_err(header_err)? as usize);
        }
        let string_count = r.i32().map_err(header_err)?;
        if string_count < 0 {
            return Err(anyhow::anyhow!("{name}: the header is cut short or damaged"));
        }
        r.u32().map_err(header_err)?; // longest string
        let mut strings = Vec::new();
        for _ in 0..string_count {
            strings.push(r.sized().map_err(header_err)?);
        }
        r.strings = strings;
        let groups = r.i32().map_err(header_err)?;
        r.skip(groups.wrapping_mul(4)).map_err(header_err)?; // groups
        let mut blocks = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            let Some(ty) = types.get(type_of[i]) else {
                return Err(anyhow::anyhow!("{name}: block {i} names a type that is not in the file's type list"));
            };
            let at = r.pos;
            let kind = match read_block(ty, &mut r) {
                Ok(k) => k,
                Err(BlockErr::Range) => return Err(nif_err(format!("{name}: block {i} ({ty}) at byte {at} could not be read."))),
                Err(BlockErr::Nif(m)) => return Err(nif_err(m)),
            };
            blocks.push(NifBlock { type_name: ty.clone(), index: i as i32, kind });
        }
        let roots = r.refs().map_err(header_err)?;
        if r.pos as i64 != d.len() as i64 {
            return Err(nif_err(format!(
                "{name}: {} bytes left after the last block (a block was read wrong).",
                d.len() as i64 - r.pos as i64
            )));
        }
        Ok(NifFile { strings: std::mem::take(&mut r.strings), blocks, roots })
    }
}

fn read_block(ty: &str, r: &mut Reader) -> B<NifKind> {
    Ok(match ty {
        "NiNode" => {
            let av = av(r)?;
            let children = r.refs()?;
            r.refs()?; // effects
            NifKind::Node(NifNode { av, children })
        }
        "NiTriShape" | "NiTriStrips" => {
            let av = av(r)?;
            let data = r.i32()?;
            let skin = r.i32()?;
            if r.bool()? {
                r.str_()?;
                r.i32()?;
            } // shader name, implementation
            NifKind::Geom(NifGeom { av, data, skin })
        }
        "NiTriShapeData" | "NiTriStripsData" => {
            let mut g = NifGeomData::default();
            r.i32()?; // group id
            let n = r.u16()? as i32;
            g.count = n;
            r.u8()?;
            r.u8()?; // keep, compress flags
            if r.bool()? {
                g.pos = Some(r.floats(n.wrapping_mul(3))?);
            }
            let flags = r.u16()? as i32;
            if r.bool()? {
                g.nrm = Some(r.floats(n.wrapping_mul(3))?);
                if (flags & 0xF000) != 0 {
                    r.skip(n.wrapping_mul(24))?; // tangents, bitangents
                }
            }
            r.skip(16)?; // bounding sphere
            if r.bool()? {
                g.col = Some(r.floats(n.wrapping_mul(4))?);
            }
            let sets = flags & 0x3F;
            for s in 0..sets {
                let uv = r.floats(n.wrapping_mul(2))?;
                if s == 0 {
                    g.uv = Some(uv);
                }
            }
            r.u16()?; // consistency
            r.i32()?; // additional data
            let tris = r.u16()? as i32;
            if ty == "NiTriShapeData" {
                r.u32()?; // triangle points
                g.tris = if r.bool()? { r.u16s(tris.wrapping_mul(3))? } else { Vec::new() };
                let groups = r.u16()?;
                for _ in 0..groups {
                    let n = r.u16()? as i32;
                    r.skip(2 * n)?;
                }
            } else {
                let n = r.u16()? as i32;
                let lengths = r.u16s(n)?;
                let mut list: Vec<u16> = Vec::new();
                if r.bool()? {
                    for len in lengths {
                        let strip = r.u16s(len as i32)?;
                        unstrip(&strip, &mut list);
                    }
                }
                g.tris = list;
            }
            NifKind::GeomData(g)
        }
        "NiSkinInstance" => {
            let data = r.i32()?;
            let partition = r.i32()?;
            let root = r.i32()?;
            let bones = r.refs()?;
            NifKind::SkinInstance(NifSkinInstance { data, partition, root, bones })
        }
        "NiSkinData" => {
            let mut s = NifSkinData { r: r.floats_n::<9>()?, t: r.floats_n::<3>()?, s: r.f()?, bones: Vec::new() };
            let n = r.i32()?;
            if n < 0 {
                return Err(BlockErr::Range);
            }
            let weights = r.u8()? != 0;
            for _ in 0..n {
                let mut b = NifSkinBone { r: r.floats_n::<9>()?, t: r.floats_n::<3>()?, s: r.f()?, ..Default::default() };
                r.skip(16)?; // bounding sphere
                let n = r.u16()? as usize;
                let n = if weights { n } else { 0 };
                for _ in 0..n {
                    b.vertex.push(r.u16()?);
                    b.weight.push(r.f()?);
                }
                s.bones.push(b);
            }
            NifKind::SkinData(s)
        }
        "NiSkinPartition" => {
            let n = r.i32()?;
            if n < 0 {
                return Err(BlockErr::Range);
            }
            let mut p = NifSkinPartition::default();
            for _ in 0..n {
                let mut part = NifSkinPart::default();
                let verts = r.u16()? as i32;
                let tris = r.u16()? as i32;
                let bones = r.u16()? as i32;
                let strips = r.u16()? as i32;
                part.per_vertex = r.u16()? as i32;
                part.bones = r.u16s(bones)?;
                if r.bool()? {
                    part.vertex_map = Some(r.u16s(verts)?);
                }
                if r.bool()? {
                    part.weights = Some(r.floats(verts.wrapping_mul(part.per_vertex))?);
                }
                let lengths = r.u16s(strips)?;
                if r.bool()? {
                    if strips > 0 {
                        for len in lengths {
                            r.skip(2 * len as i32)?;
                        }
                    } else {
                        r.skip(6 * tris)?;
                    }
                }
                if r.bool()? {
                    part.bone_index = Some(r.bytes(verts.wrapping_mul(part.per_vertex))?);
                }
                p.parts.push(part);
            }
            NifKind::SkinPartition(p)
        }
        "NiTexturingProperty" => {
            let net = net(r)?;
            let mut t = NifTexturing { net, base: -1 };
            r.u16()?; // flags
            let slots = r.i32()?;
            // base, dark, detail, gloss, glow, bump, then decals
            for s in 0..6 {
                if !r.bool()? {
                    continue;
                }
                let src = tex_desc(r)?;
                if s == 0 {
                    t.base = src;
                }
                if s == 5 {
                    r.skip(24)?; // bump map luma scale and offset, 2x2 matrix
                }
            }
            let mut s = 6;
            while s < slots {
                if r.bool()? {
                    tex_desc(r)?;
                }
                s += 1;
            }
            let shader = r.i32()?;
            let mut s = 0;
            while s < shader {
                if r.bool()? {
                    tex_desc(r)?;
                    r.u32()?;
                }
                s += 1;
            }
            NifKind::Texturing(t)
        }
        "NiSourceTexture" => {
            let net = net(r)?;
            let external = r.u8()? != 0;
            let file = r.str_()?;
            let pixels = r.i32()?;
            r.skip(12)?; // pixel layout, mipmaps, alpha format
            r.u8()?;
            r.u8()?; // static, direct render
            NifKind::SourceTexture(NifSourceTexture { net, external, file, pixels })
        }
        "NiPixelData" => {
            let mut p = NifPixelData { format: r.i32()?, bits_per_pixel: r.u8()? as i32, ..Default::default() };
            r.skip(4 + 4 + 1 + 4)?; // renderer hint, extra data, flags, tiling
            for c in 0..4 {
                p.channels[c] = r.i32()?;
                r.i32()?;
                p.channel_bits[c] = r.u8()? as i32;
                r.u8()?; // type, convention, bits, signed
            }
            r.i32()?; // palette
            let mips = r.i32()?;
            p.bytes_per_pixel = r.i32()?;
            let mut size0 = 0;
            let mut m = 0;
            while m < mips {
                let w = r.i32()?;
                let h = r.i32()?;
                let offset = r.i32()?;
                if m == 0 {
                    p.width = w;
                    p.height = h;
                }
                if m == 1 {
                    size0 = offset;
                }
                m += 1;
            }
            let total = r.i32()?;
            let faces = r.i32()?;
            let all = r.bytes(total.wrapping_mul(faces))?;
            p.data = if mips > 1 {
                slice_to(&all, size0)?
            } else if faces > 1 {
                slice_to(&all, total)?
            } else {
                all
            };
            NifKind::PixelData(p)
        }
        "NiMaterialProperty" => {
            let net = net(r)?;
            let m = NifMaterial {
                net,
                ambient: r.floats_n::<3>()?,
                diffuse: r.floats_n::<3>()?,
                specular: r.floats_n::<3>()?,
                emissive: r.floats_n::<3>()?,
                gloss: r.f()?,
                alpha: r.f()?,
            };
            NifKind::Material(m)
        }
        "NiAlphaProperty" => {
            let net = net(r)?;
            let flags = r.u16()? as i32;
            let threshold = r.u8()? as i32;
            NifKind::Alpha(NifAlpha { net, flags, threshold })
        }
        "NiStencilProperty" => {
            let net = net(r)?;
            let flags = r.u16()? as i32;
            r.skip(8)?; // reference, mask
            NifKind::Stencil(NifProperty { net, flags })
        }
        "NiZBufferProperty" | "NiVertexColorProperty" | "NiSpecularProperty" | "NiWireframeProperty" | "NiDitherProperty" | "NiShadeProperty" => {
            let net = net(r)?;
            let flags = r.u16()? as i32;
            NifKind::Property(NifProperty { net, flags })
        }
        "NiStringExtraData" => {
            let name = r.str_()?;
            let value = NifExtraValue::Str(r.str_()?);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiIntegerExtraData" => {
            let name = r.str_()?;
            let value = NifExtraValue::Int(r.i32()?);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiBooleanExtraData" => {
            let name = r.str_()?;
            let value = NifExtraValue::Bool(r.u8()? != 0);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiFloatExtraData" => {
            let name = r.str_()?;
            let value = NifExtraValue::Float(r.f()?);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiColorExtraData" => {
            let name = r.str_()?;
            let value = NifExtraValue::Floats(r.floats(4)?);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiIntegersExtraData" => {
            let name = r.str_()?;
            let n = r.i32()?;
            r.skip(n.wrapping_mul(4))?;
            NifKind::Extra(NifExtra { name, value: NifExtraValue::None })
        }
        "NiFloatsExtraData" => {
            let name = r.str_()?;
            let n = r.i32()?;
            let value = NifExtraValue::Floats(r.floats(n)?);
            NifKind::Extra(NifExtra { name, value })
        }
        "NiBinaryExtraData" => {
            let name = r.str_()?;
            let n = r.i32()?;
            r.skip(n)?;
            NifKind::Extra(NifExtra { name, value: NifExtraValue::None })
        }
        "NiTextKeyExtraData" => {
            let name = r.str_()?;
            let n = r.i32()?;
            if n < 0 {
                return Err(BlockErr::Range);
            }
            let mut keys = Vec::new();
            for _ in 0..n {
                let time = r.f()?;
                let text = r.str_()?;
                keys.push((time, text));
            }
            NifKind::TextKeys(NifTextKeys { name, keys })
        }
        "NiTransformController" | "NiVisController" | "NiAlphaController" | "NiMaterialColorController" | "NiTextureTransformController" | "NiFlipController" => {
            let mut c = time_controller(r)?;
            c.interpolator = r.i32()?;
            if ty == "NiMaterialColorController" {
                r.u16()?; // which colour
            }
            if ty == "NiTextureTransformController" {
                r.u8()?;
                r.u32()?;
                r.u32()?;
            } // shader map, slot, operation
            if ty == "NiFlipController" {
                r.u32()?;
                r.refs()?;
            } // slot, sources
            NifKind::Controller(c)
        }
        "NiMultiTargetTransformController" => {
            let c = time_controller(r)?;
            let n = r.u16()? as i32;
            r.skip(4 * n)?; // extra targets
            NifKind::Controller(c)
        }
        "NiControllerManager" => {
            let c = time_controller(r)?;
            r.u8()?;
            r.refs()?;
            r.i32()?; // cumulative, sequences, object palette
            NifKind::Controller(c)
        }
        "NiDefaultAVObjectPalette" => {
            r.i32()?; // scene
            let n = r.i32()?;
            let mut i = 0;
            while i < n {
                r.sized()?;
                r.i32()?;
                i += 1;
            }
            NifKind::Object
        }
        "NiBoneLODController" => {
            let c = time_controller(r)?;
            r.u32()?; // current level
            let lods = r.i32()?;
            r.u32()?;
            let mut i = 0;
            while i < lods {
                r.refs()?;
                i += 1;
            }
            let groups = r.i32()?;
            let mut i = 0;
            while i < groups {
                let n = r.i32()?;
                r.skip(n.wrapping_mul(8))?; // shape + skin instance
                i += 1;
            }
            r.refs()?;
            NifKind::Controller(c)
        }
        "NiCollisionData" => {
            r.i32()?;
            r.u32()?;
            r.u32()?; // target, propagation, collision mode
            if r.u8()? != 0 {
                bounding_volume(r)?;
            }
            NifKind::Object
        }
        "NiTransformInterpolator" => {
            let t = r.floats_n::<3>()?;
            let q = r.floats_n::<4>()?;
            let s = r.f()?;
            let data = r.i32()?;
            NifKind::TransformInterpolator(NifTransformInterpolator { t, q, s, data })
        }
        "NiTransformData" => {
            let mut t = NifTransformData::default();
            let n = r.i32()?;
            if n > 0 {
                t.rotation_type = r.i32()?;
                if t.rotation_type == 4 {
                    let a = key_group(r, 1)?;
                    let b = key_group(r, 1)?;
                    let c = key_group(r, 1)?;
                    t.euler = Some([a, b, c]);
                } else {
                    let mut k = NifKeys { type_: t.rotation_type, size: 4, ..Default::default() };
                    for _ in 0..n {
                        k.times.push(r.f()?);
                        for _ in 0..4 {
                            k.values.push(r.f()?);
                        }
                        if t.rotation_type == 3 {
                            r.skip(12)?; // tension, bias, continuity
                        }
                    }
                    t.rotation = Some(k);
                }
            }
            t.translation = key_group(r, 3)?;
            t.scale = key_group(r, 1)?;
            NifKind::TransformData(t)
        }
        "NiBSplineCompTransformInterpolator" => {
            let start = r.f()?;
            let stop = r.f()?;
            let data = r.i32()?;
            let basis = r.i32()?;
            let t = r.floats_n::<3>()?;
            let q = r.floats_n::<4>()?;
            let s = r.f()?;
            let translation_at = r.u32()?;
            let rotation_at = r.u32()?;
            let scale_at = r.u32()?;
            let translation_offset = r.f()?;
            let translation_range = r.f()?;
            let rotation_offset = r.f()?;
            let rotation_range = r.f()?;
            let scale_offset = r.f()?;
            let scale_range = r.f()?;
            NifKind::SplineInterpolator(NifSplineInterpolator {
                start,
                stop,
                s,
                translation_offset,
                translation_range,
                rotation_offset,
                rotation_range,
                scale_offset,
                scale_range,
                data,
                basis,
                t,
                q,
                translation_at,
                rotation_at,
                scale_at,
            })
        }
        "NiBSplineData" => {
            let n = r.i32()?;
            let floats = r.floats(n)?;
            let n = r.i32()?;
            let raw = r.u16s(n)?;
            NifKind::SplineData(NifSplineData { floats, shorts: raw.into_iter().map(|v| v as i16).collect() })
        }
        "NiBSplineBasisData" => NifKind::SplineBasis(NifSplineBasis { count: r.i32()? }),
        "NiBoolInterpolator" => {
            r.u8()?;
            r.i32()?;
            NifKind::Object
        }
        "NiFloatInterpolator" => {
            let value = r.f()?;
            NifKind::FloatInterpolator(NifFloatInterpolator { value, data: r.i32()? })
        }
        "NiPoint3Interpolator" => {
            r.skip(12)?;
            r.i32()?;
            NifKind::Object
        }
        "NiBoolData" => {
            let n = r.i32()?;
            if n > 0 {
                r.i32()?;
                r.skip(n.wrapping_mul(5))?;
            }
            NifKind::Object
        }
        "NiFloatData" => NifKind::FloatData(key_group(r, 1)?),
        "NiPosData" => {
            key_group(r, 3)?;
            NifKind::Object
        }
        "NiControllerSequence" => {
            let mut s = NifSequence { name: r.str_()?, ..Default::default() };
            let n = r.i32()?;
            if n < 0 {
                return Err(BlockErr::Range);
            }
            r.u32()?; // array grow by
            for _ in 0..n {
                let mut l = NifSequenceLink { interpolator: r.i32()?, controller: r.i32()?, node: r.str_()?, controller_type: None };
                r.str_()?;
                l.controller_type = r.str_()?;
                r.str_()?;
                r.str_()?; // property type, controller type, controller id, interpolator id
                s.links.push(l);
            }
            s.weight = r.f()?;
            s.text_keys = r.i32()?;
            s.cycle = r.i32()?;
            s.frequency = r.f()?;
            s.start = r.f()?;
            s.stop = r.f()?;
            r.i32()?; // manager
            s.accum_root = r.str_()?;
            NifKind::Sequence(s)
        }
        _ => return Err(BlockErr::Nif(format!("Block type {ty} is not one this reader knows."))),
    })
}

fn net(r: &mut Reader) -> R<NifNet> {
    Ok(NifNet { name: r.str_()?, extra: r.refs()?, controller: r.i32()? })
}

fn av(r: &mut Reader) -> R<NifAv> {
    let net = net(r)?;
    let flags = r.u16()? as i32;
    let t = r.floats_n::<3>()?;
    let rot = r.floats_n::<9>()?;
    let s = r.f()?;
    let properties = r.refs()?;
    r.i32()?; // collision object
    Ok(NifAv { net, flags, t, r: rot, s, properties })
}

fn time_controller(r: &mut Reader) -> R<NifController> {
    Ok(NifController {
        next: r.i32()?,
        flags: r.u16()? as i32,
        frequency: r.f()?,
        phase: r.f()?,
        start: r.f()?,
        stop: r.f()?,
        target: r.i32()?,
        interpolator: -1,
    })
}

/// One texture slot; returns its NiSourceTexture.
fn tex_desc(r: &mut Reader) -> R<i32> {
    let src = r.i32()?;
    r.u16()?; // clamp, filter, uv set
    if r.bool()? {
        r.skip(8 + 8 + 4 + 4 + 8)?; // texture transform
    }
    Ok(src)
}

fn bounding_volume(r: &mut Reader) -> B<()> {
    match r.i32()? {
        0 => r.skip(16)?,           // sphere
        1 => r.skip(12 + 36 + 12)?, // box
        2 => r.skip(12 + 12 + 8)?,  // capsule
        4 => {
            // union
            let n = r.i32()?;
            let mut i = 0;
            while i < n {
                bounding_volume(r)?;
                i += 1;
            }
        }
        5 => r.skip(16 + 12)?, // half space
        _ => return Err(BlockErr::Nif("Unknown bounding volume.".to_string())),
    }
    Ok(())
}

fn key_group(r: &mut Reader, size: i32) -> R<Option<NifKeys>> {
    let n = r.i32()?;
    if n == 0 {
        return Ok(None);
    }
    if n < 0 {
        return Err(OutOfRange);
    }
    let type_ = r.i32()?;
    let mut k = NifKeys { type_, size, ..Default::default() };
    if k.type_ == 2 {
        k.in_ = Some(Vec::new());
        k.out = Some(Vec::new());
    }
    for _ in 0..n {
        k.times.push(r.f()?);
        for _ in 0..size {
            k.values.push(r.f()?);
        }
        if k.type_ == 2 {
            for _ in 0..size {
                let v = r.f()?;
                k.in_.as_mut().expect("type 2").push(v);
            }
            for _ in 0..size {
                let v = r.f()?;
                k.out.as_mut().expect("type 2").push(v);
            }
        } else if k.type_ == 3 {
            r.skip(12)?;
        }
    }
    Ok(Some(k))
}

/// A triangle strip as separate triangles (every other one flipped back; empty ones dropped).
fn unstrip(s: &[u16], to: &mut Vec<u16>) {
    let mut i = 0;
    while i + 2 < s.len() {
        let (a, b, c) = (s[i], s[i + 1], s[i + 2]);
        if !(a == b || b == c || a == c) {
            if (i & 1) == 0 {
                to.extend_from_slice(&[a, b, c]);
            } else {
                to.extend_from_slice(&[a, c, b]);
            }
        }
        i += 1;
    }
}

/// `all.AsSpan(0, n).ToArray()`: throws (ArgumentOutOfRange) when n is negative or past the end.
fn slice_to(all: &[u8], n: i32) -> R<Vec<u8>> {
    if n < 0 || n as usize > all.len() {
        return Err(OutOfRange);
    }
    Ok(all[..n as usize].to_vec())
}

struct Reader<'a> {
    d: &'a [u8],
    /// C# int position (wraps like the C# arithmetic does)
    pos: i32,
    strings: Vec<String>,
}

impl Reader<'_> {
    fn at(&self, n: i64) -> R<usize> {
        // the n bytes from pos must lie inside the data
        if self.pos < 0 || n < 0 || self.pos as i64 + n > self.d.len() as i64 { Err(OutOfRange) } else { Ok(self.pos as usize) }
    }
    fn take(&mut self, n: usize) -> R<&[u8]> {
        let p = self.at(n as i64)?;
        self.pos = (p + n) as i32;
        Ok(&self.d[p..p + n])
    }
    fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> R<bool> {
        Ok(self.u8()? != 0)
    }
    fn u16(&mut self) -> R<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("2 bytes")))
    }
    fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn i32(&mut self) -> R<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn f(&mut self) -> R<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn skip(&mut self, n: i32) -> R<()> {
        // C#: n < 0 || Pos + n > d.Length (int arithmetic)
        if n < 0 || self.pos.wrapping_add(n) > self.d.len() as i32 {
            return Err(OutOfRange);
        }
        self.pos = self.pos.wrapping_add(n);
        Ok(())
    }
    fn bytes(&mut self, n: i32) -> R<Vec<u8>> {
        if n < 0 || self.pos.wrapping_add(n) > self.d.len() as i32 {
            return Err(OutOfRange);
        }
        Ok(self.take(n as usize)?.to_vec())
    }
    fn floats(&mut self, n: i32) -> R<Vec<f32>> {
        if n < 0 {
            return Err(OutOfRange);
        }
        let b = self.take(n as usize * 4)?;
        Ok(b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }
    fn floats_n<const N: usize>(&mut self) -> R<[f32; N]> {
        let v = self.floats(N as i32)?;
        let mut a = [0f32; N];
        a.copy_from_slice(&v);
        Ok(a)
    }
    fn u16s(&mut self, n: i32) -> R<Vec<u16>> {
        if n < 0 {
            return Err(OutOfRange);
        }
        let b = self.take(n as usize * 2)?;
        Ok(b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
    }
    fn refs(&mut self) -> R<Vec<i32>> {
        let n = self.i32()?;
        if n < 0 {
            return Err(OutOfRange);
        }
        let b = self.take(n as usize * 4)?;
        Ok(b.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }
    /// Length + bytes. Names are Shift-JIS in places; Latin-1 keeps every byte, which is all matching needs.
    fn sized(&mut self) -> R<String> {
        let n = self.i32()?;
        if n < 0 {
            return Err(OutOfRange);
        }
        let b = self.take(n as usize)?;
        Ok(b.iter().map(|&c| c as char).collect())
    }
    /// An entry of the file's string table (None for "none").
    fn str_(&mut self) -> R<Option<String>> {
        let i = self.u32()?;
        Ok(self.strings.get(i as usize).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_nif() {
        let e = NifFile::read(b"nope", "a.nif").err().expect("error");
        assert!(is_nif_error(&e));
        assert_eq!(e.to_string(), "a.nif is not a Gamebryo file.");
        let e = NifFile::read(b"Gamebryo File Format, Version 10.0.0.0\n\x00\x00\x00\x0a\x01", "b.nif").err().expect("error");
        assert_eq!(e.to_string(), "b.nif is NIF version 10.0.0.0; only 20.1.0.3 is read.");
    }

    #[test]
    fn unstrip_flips_every_other_triangle() {
        let mut out = Vec::new();
        unstrip(&[0, 1, 2, 3, 3, 4], &mut out);
        assert_eq!(out, vec![0, 1, 2, 1, 3, 2]);
    }
}
