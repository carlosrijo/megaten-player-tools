//! The NIF reader and the model export against what the C# reader produced from the same game files.
//! Needs the game's client folder: set `IMAGINE_CLIENT_DIR` (e.g. C:\SMTImagineNM\client1); without it these tests do
//! nothing (the game's art is not in the repository). The expected numbers come from the C# `DemonModel.Export` and
//! `NifFile.Load` run over that folder.
//!
//! A model's JSON is compared with its texture PNG cut out (a different zlib writes other bytes for the same picture):
//! FNV-1a 64 of the rest of the text, and of the texture's decoded RGBA pixels.
use imagine::build::demon_model;
use imagine::data::nif::{NifFile, is_nif_error};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../examples/jsoncmp.rs"]
mod jsoncmp;

fn client() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os("IMAGINE_CLIENT_DIR")?);
    p.join("Devil").join("bone").is_dir().then_some(p)
}

#[test]
fn models_match_the_csharp_export() {
    let Some(client) = client() else { return };
    // (model, hash of the JSON without textures, pixel hashes of the textures) from the C# export
    let expected: [(&str, u64, &[u64]); 4] = [
        ("D2_11_003a", 0x53741b7d91a78539, &[0xca6179864c26ca80]),
        ("D1_01_001a", 0xf9dcd7ef1513ada0, &[0x2b832fdc0ec399fe]),
        ("D2_04_009a", 0x2152e953ecefc517, &[0x1c67fbb1e6e17692]),
        ("D3_01_004a", 0xa8dd36493b3db7ea, &[0x0bc895f43114b5e9]),
    ];
    for (model, hash, pixels) in expected {
        let json = demon_model::export(&client, model, None).expect("export").expect("model exists");
        let (h, p) = jsoncmp::fingerprint(&json);
        assert_eq!(h, hash, "{model}: JSON text");
        assert_eq!(p, pixels, "{model}: texture pixels");
    }
    assert!(demon_model::export(&client, "no_such_model", None).unwrap().is_none());
}

#[test]
fn bone_folder_check_matches_the_csharp_counts() {
    let Some(client) = client() else { return };
    let dir = client.join("Devil").join("bone");
    let (mut ok, mut total, mut other) = (0, 0, 0);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.to_string_lossy().to_lowercase();
        if !(name.ends_with(".nif") || name.ends_with(".kf")) {
            continue;
        }
        total += 1;
        match NifFile::load(&p) {
            Ok(_) => ok += 1,
            Err(e) if is_nif_error(&e) => *why.entry(e.to_string()).or_default() += 1,
            Err(_) => other += 1,
        }
    }
    assert_eq!((ok, total, other), (6978, 7000, 0));
    let want: BTreeMap<String, usize> = [
        ("NiBillboardNode", 6),
        ("NiBSplineTransformInterpolator", 5),
        ("NiPathInterpolator", 4),
        ("NiDirectionalLight", 4),
        ("NiBSplineCompFloatInterpolator", 2),
        ("NiLODNode", 1),
    ]
    .into_iter()
    .map(|(t, n)| (format!("Block type {t} is not one this reader knows."), n))
    .collect();
    assert_eq!(why, want);
}

/// Every icon under Interface\tga decoded: the number read, and one FNV-1a 64 over the sizes and pixels in path order.
/// The same pictures, converted to PNG by the C# `TgaImage.ToPng` and decoded, had identical pixels file by file.
#[test]
fn tga_icons_match_the_csharp_decoding() {
    use imagine::data::tga;
    let Some(client) = client() else { return };
    let dir = client.join("Interface").join("tga");
    if !dir.is_dir() {
        return;
    }
    fn walk(d: &std::path::Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tga")) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort_by(|a, b| a.to_string_lossy().as_bytes().cmp(b.to_string_lossy().as_bytes()));
    let (mut read, mut h) = (0usize, 0xcbf29ce484222325u64);
    let mut mix = |bytes: &[u8]| {
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for f in &files {
        if let Some((rgba, w, hh)) = tga::decode_rgba(&std::fs::read(f).unwrap()) {
            read += 1;
            mix(&w.to_le_bytes());
            mix(&hh.to_le_bytes());
            mix(&rgba);
        }
    }
    assert_eq!((files.len(), read), (10124, 10123));
    assert_eq!(h, 14264501325445993525, "pixel fingerprint of all icons");
}
