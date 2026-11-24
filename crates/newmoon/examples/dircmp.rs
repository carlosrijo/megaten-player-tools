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

fn textures_ok(xa: &str, xb: &str) -> bool {
    let plus = format!("{}u002B", '\u{5c}');
    let la: Vec<&str> = if xa.is_empty() { vec![] } else { xa.split("\",\"").collect() };
    let lb: Vec<&str> = if xb.is_empty() { vec![] } else { xb.split("\",\"").collect() };
    if la.len() != lb.len() {
        return false;
    }
    la.iter().zip(lb.iter()).all(|(x, y)| {
        let strip = |s: &str| s.trim_matches('"').replace(&plus, "+").replace("data:image/png;base64,", "");
        let ia = image::load_from_memory(&b64(&strip(x))).unwrap().to_rgba8();
        let ib = image::load_from_memory(&b64(&strip(y))).unwrap().to_rgba8();
        ia.as_raw() == ib.as_raw() && ia.dimensions() == ib.dimensions()
    })
}

/// dircmp <dir a> <dir b>
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut names: Vec<String> = std::fs::read_dir(&a[0]).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let (mut same, mut diff, mut errs, mut missing) = (0, 0, 0, 0);
    let mut png_same = 0;
    for n in &names {
        let pb = std::path::Path::new(&a[1]).join(n);
        if !pb.exists() {
            missing += 1;
            println!("MISSING in b: {n}");
            continue;
        }
        if n.ends_with(".err") {
            errs += 1;
            println!("ERR {n}: a = {:?} | b = {:?}", std::fs::read_to_string(std::path::Path::new(&a[0]).join(n)).unwrap(), std::fs::read_to_string(&pb).unwrap());
            continue;
        }
        let ta = std::fs::read_to_string(std::path::Path::new(&a[0]).join(n)).unwrap();
        let tb = std::fs::read_to_string(&pb).unwrap();
        if ta == tb {
            same += 1;
            png_same += 1;
            continue;
        }
        if ta == "null" || tb == "null" {
            diff += 1;
            println!("DIFF {n}: null mismatch");
            continue;
        }
        let (pa, xa, sa) = split(&ta);
        let (pb2, xb, sb) = split(&tb);
        if pa == pb2 && sa == sb && textures_ok(xa, xb) {
            same += 1;
        } else {
            diff += 1;
            println!("DIFF {n}: prefix {} suffix {} textures {}", pa == pb2, sa == sb, textures_ok(xa, xb));
        }
    }
    println!("{} files: {same} identical (texture PNG bytes aside; {png_same} fully byte-identical), {diff} different, {errs} errors, {missing} missing", names.len());
}
