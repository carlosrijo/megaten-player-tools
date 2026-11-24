//! Throwaway: compare two model JSON files; everything but the texture PNG bytes must be identical, and the
//! textures must decode to the same pixels.  jsoncmp <a.json> <b.json>
fn b64(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        } as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

fn split(t: &str) -> (&str, &str, &str) {
    let a = t.find("\"textures\":[").unwrap() + "\"textures\":[".len();
    let b = t.find("],\"anims\":").unwrap();
    (&t[..a], &t[a..b], &t[b..])
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("hash") {
        for f in std::env::args().skip(2) {
            let (h, pix) = fingerprint(&std::fs::read_to_string(&f).unwrap());
            println!("{f} {h:#018x} {:#018x?}", pix);
        }
        return;
    }
    let a: Vec<String> = std::env::args().skip(1).collect();
    let ta = std::fs::read_to_string(&a[0]).unwrap();
    let tb = std::fs::read_to_string(&a[1]).unwrap();
    let (pa, xa, sa) = split(&ta);
    let (pb, xb, sb) = split(&tb);
    println!("before textures identical: {} ({} bytes)", pa == pb, pa.len());
    println!("after textures identical:  {} ({} bytes)", sa == sb, sa.len());
    let plus = format!("{}u002B", '\u{5c}');
    let la: Vec<&str> = if xa.is_empty() { vec![] } else { xa.split("\",\"").collect() };
    let lb: Vec<&str> = if xb.is_empty() { vec![] } else { xb.split("\",\"").collect() };
    println!("texture count {} / {}", la.len(), lb.len());
    for (i, (x, y)) in la.iter().zip(lb.iter()).enumerate() {
        let strip = |s: &str| s.trim_matches('"').replace(&plus, "+").replace("data:image/png;base64,", "");
        let ia = image::load_from_memory(&b64(&strip(x))).unwrap().to_rgba8();
        let ib = image::load_from_memory(&b64(&strip(y))).unwrap().to_rgba8();
        println!(
            "texture {i}: {}x{} vs {}x{}, pixels identical: {}, png bytes identical: {}",
            ia.width(),
            ia.height(),
            ib.width(),
            ib.height(),
            ia.as_raw() == ib.as_raw(),
            x == y
        );
    }
}

/// FNV-1a 64 of the JSON with the textures cut out, and of each texture's decoded pixels (what tests/model_golden.rs pins).
pub fn fingerprint(t: &str) -> (u64, Vec<u64>) {
    fn fnv(bytes: &[u8]) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
    let (p, x, s) = split(t);
    let plus = format!("{}u002B", '\u{5c}');
    let pix = if x.is_empty() {
        vec![]
    } else {
        x.split("\",\"")
            .map(|y| {
                let st = y.trim_matches('"').replace(&plus, "+").replace("data:image/png;base64,", "");
                fnv(image::load_from_memory(&b64(&st)).unwrap().to_rgba8().as_raw())
            })
            .collect()
    };
    (fnv(format!("{p}{s}").as_bytes()), pix)
}
