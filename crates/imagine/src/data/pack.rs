//! Data kept in the repository only encrypted: reference\sealed\data.bin (README there). build.rs puts that file and its
//! key, scrambled, into the library; it is opened here, in memory only, the first time something asks for it. Nothing
//! opened is ever written to disk.
//!
//! A build without the key (a fork, BABEL_PACK_KEY=none) has none of it: [`available`] is false and everything falls
//! back to the other data (comp_hack's files, the client's text).
//!
//! The format: "BPK1", a 12-byte nonce, then ChaCha20-Poly1305 over a deflated list of named entries (for each: name
//! length u16, name, data length u32, data; little-endian).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use sha2::{Digest, Sha256};

use crate::Result;

include!(concat!(env!("OUT_DIR"), "/pack_gen.rs"));

const MAGIC: &[u8; 4] = b"BPK1";
/// The entry with tokusei XML (comments and layout taken out).
pub const TOKUSEI: &str = "tokusei.xml";
/// The entry with the Gear Database page's data made with it (the repo's app\ui\gear-db.json is made without it).
pub const GEAR_DB: &str = "gear-db.json";
/// The entry with wording for ids comp_hack's schema doesn't have (`data::stat_labels`).
pub const LABELS: &str = "labels.tsv";

static OPENED: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();

fn opened() -> &'static HashMap<String, Vec<u8>> {
    OPENED.get_or_init(|| {
        let Some(scrambled) = SEAL else { return HashMap::new() };
        let mask: [u8; 32] = Sha256::new().chain_update(b"pack-mask").chain_update(PACK).finalize().into();
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = scrambled[i] ^ mask[i];
        }
        open(PACK, &key).map(|v| v.into_iter().collect()).unwrap_or_default()
    })
}

/// One entry of the sealed data, or `None` (no key in this build, or no such entry).
pub fn entry(name: &str) -> Option<&'static [u8]> {
    opened().get(name).map(Vec::as_slice)
}

/// Whether this build has the sealed data.
pub fn available() -> bool {
    entry(TOKUSEI).is_some()
}

/// The entries, sealed with `key`.
pub fn seal(entries: &[(String, Vec<u8>)], key: &[u8; 32]) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    for (name, data) in entries {
        body.extend_from_slice(&u16::try_from(name.len())?.to_le_bytes());
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        body.extend_from_slice(data);
    }
    let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(&body)?;
    let packed = z.finish()?;
    // a nonce from the content: the same key never seals two different contents under one nonce
    let digest: [u8; 32] = Sha256::new().chain_update(b"pack-nonce").chain_update(&packed).finalize().into();
    let nonce = &digest[..12];
    let ct = ChaCha20Poly1305::new(Key::from_slice(key))
        .encrypt(Nonce::from_slice(nonce), packed.as_slice())
        .map_err(|_| anyhow::anyhow!("sealing failed"))?;
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// The entries of sealed data; an error when the key is wrong or the data was changed.
pub fn open(blob: &[u8], key: &[u8; 32]) -> Result<Vec<(String, Vec<u8>)>> {
    if blob.len() < 16 || &blob[..4] != MAGIC {
        anyhow::bail!("not sealed data");
    }
    let packed = ChaCha20Poly1305::new(Key::from_slice(key))
        .decrypt(Nonce::from_slice(&blob[4..16]), &blob[16..])
        .map_err(|_| anyhow::anyhow!("the key does not open this sealed data"))?;
    let mut body = Vec::new();
    flate2::read::DeflateDecoder::new(packed.as_slice()).read_to_end(&mut body)?;
    let mut res = Vec::new();
    let mut at = 0usize;
    let take = |at: &mut usize, n: usize| -> Result<&[u8]> {
        let s = body.get(*at..*at + n).ok_or_else(|| anyhow::anyhow!("sealed data cut short"))?;
        *at += n;
        Ok(s)
    };
    while at < body.len() {
        let n = u16::from_le_bytes(take(&mut at, 2)?.try_into()?) as usize;
        let name = String::from_utf8(take(&mut at, n)?.to_vec())?;
        let len = u32::from_le_bytes(take(&mut at, 4)?.try_into()?) as usize;
        res.push((name, take(&mut at, len)?.to_vec()));
    }
    Ok(res)
}

/// The key for sealing (the `seal` command): BABEL_PACK_KEY, else `.secrets\pack.key` in the current folder or one
/// above it. `None` when there is none.
pub fn key_for_sealing() -> Result<Option<[u8; 32]>> {
    let text = match std::env::var("BABEL_PACK_KEY") {
        Ok(v) if !v.trim().is_empty() && !v.trim().eq_ignore_ascii_case("none") => v,
        _ => match key_file() {
            Some(f) => std::fs::read_to_string(f)?,
            None => return Ok(None),
        },
    };
    let hex = text.trim();
    if hex.len() != 64 {
        anyhow::bail!("the pack key must be 64 hex digits");
    }
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)?;
    }
    Ok(Some(k))
}

/// `.secrets\pack.key` in the current folder or one above it.
pub fn key_file() -> Option<PathBuf> {
    std::env::current_dir().ok()?.ancestors().map(|d| d.join(".secrets").join("pack.key")).find(|p| p.is_file())
}

/// The sealed file in a checkout: `<root>\reference\sealed\data.bin`.
pub fn sealed_path(root: &Path) -> PathBuf {
    root.join("reference").join("sealed").join("data.bin")
}

/// An XML file without its comments and the layout between tags: the server keeps old tokusei and notes in comments,
/// none of which the reader uses or should carry.
pub fn strip_xml(text: &str) -> String {
    static COMMENT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)<!--.*?-->").unwrap());
    static GAP: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r">\s+<").unwrap());
    GAP.replace_all(&COMMENT.replace_all(text, ""), "><").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_open_round_trip() {
        let key = [7u8; 32];
        let entries = vec![("a.xml".to_string(), b"<x>1</x>".to_vec()), ("b.json".to_string(), vec![0u8; 1000])];
        let blob = seal(&entries, &key).unwrap();
        assert_eq!(&blob[..4], MAGIC);
        // nothing readable in the sealed bytes
        assert!(!blob.windows(3).any(|w| w == b"<x>"));
        assert_eq!(open(&blob, &key).unwrap(), entries);
        // another key, or a changed byte, opens nothing
        assert!(open(&blob, &[8u8; 32]).is_err());
        let mut bad = blob.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert!(open(&bad, &key).is_err());
    }

    #[test]
    fn strip_xml_drops_comments_and_layout() {
        assert_eq!(strip_xml("<a>\n  <!-- old <b>2</b> -->\n  <b>1</b>\n</a>\n"), "<a><b>1</b></a>");
    }
}
