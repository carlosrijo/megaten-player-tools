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
