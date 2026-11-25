//! Puts the sealed data (reference\sealed\data.bin, see its README) into the library, and with it the key that opens
//! it, scrambled: XORed with a hash of the sealed file, so there is no plain key in the exe.
//!
//! The key: the BABEL_PACK_KEY environment variable (the release build: a GitHub Actions secret), else a `.secrets\pack.key`
//! file in this folder or one above it (the main checkout, also from a worktree under it). BABEL_PACK_KEY=none builds
//! without it, like a fork does: the library then has no sealed data and says so (`data::pack::available`).
//!
//! Nothing readable is ever written: the generated file holds the sealed bytes' path and the scrambled key only. It is in
//! target\ (OUT_DIR); release.yml does not cache target\.
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let sealed = manifest.join("..").join("..").join("reference").join("sealed").join("data.bin");
    println!("cargo:rerun-if-changed={}", sealed.display());
    println!("cargo:rerun-if-env-changed=BABEL_PACK_KEY");

    let blob = std::fs::read(&sealed).ok();
    let key = key(&manifest);
    let mut out = String::new();
    match &blob {
        Some(_) => out.push_str(&format!("static PACK: &[u8] = include_bytes!({:?});\n", sealed.canonicalize().unwrap())),
        None => out.push_str("static PACK: &[u8] = &[];\n"),
    }
    match (key, &blob) {
        (Some(k), Some(b)) => {
            let mask: [u8; 32] = Sha256::new().chain_update(b"pack-mask").chain_update(b).finalize().into();
            let scrambled: Vec<String> = k.iter().zip(mask).map(|(a, m)| (a ^ m).to_string()).collect();
            out.push_str(&format!("static SEAL: Option<[u8; 32]> = Some([{}]);\n", scrambled.join(", ")));
        }
        _ => out.push_str("static SEAL: Option<[u8; 32]> = None;\n"),
    }
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("pack_gen.rs");
    std::fs::write(dest, out).unwrap();
}

fn key(manifest: &Path) -> Option<[u8; 32]> {
    let text = match std::env::var("BABEL_PACK_KEY") {
        Ok(v) if v.trim().eq_ignore_ascii_case("none") => return None,
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            let file = manifest.ancestors().map(|d| d.join(".secrets").join("pack.key")).find(|p| p.is_file())?;
            println!("cargo:rerun-if-changed={}", file.display());
            std::fs::read_to_string(file).ok()?
        }
    };
    let hex = text.trim();
    if hex.len() != 64 {
        println!("cargo:warning=the pack key is not 64 hex digits: building without the sealed data");
        return None;
    }
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(k)
}
