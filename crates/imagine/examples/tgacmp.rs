//! Throwaway: tgacmp <client tga dir> <dir of C# png output>: every .tga in both, decoded pixels compared.
use imagine::data::tga;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tga")) {
            out.push(p);
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.get(2).map(String::as_str) == Some("crops") {
        let list: [(&str, i32, i32, i32, i32); 8] = [
            ("menu_icon_00.tga", 0, 0, 32, 32), ("menu_icon_00.tga", 32, 0, 32, 32), ("menu_icon_00.tga", 64, 32, 32, 32), ("menu_icon_00.tga", 5, 7, 20, 11),
            ("union_mark_mitama.tga", 0, 0, 14, 14), ("menu_icon_00.tga", 224, 224, 32, 32), ("menu_icon_00.tga", 230, 230, 32, 32), ("menu_icon_00.tga", -1, 0, 4, 4),
        ];
        for (i, (f, x, y, w, h)) in list.iter().enumerate() {
            let mine = tga::to_png_crop(&std::fs::read(PathBuf::from(&a[0]).join(f)).unwrap(), Some((*x, *y, *w, *h)));
            let theirs = PathBuf::from(&a[1]).join(format!("crop{i}.png"));
            match (mine, theirs.exists()) {
                (None, false) => println!("crop {i}: both null"),
                (Some(m), true) => {
                    let ia = image::load_from_memory(&m).unwrap().to_rgba8();
                    let ib = image::load_from_memory(&std::fs::read(theirs).unwrap()).unwrap().to_rgba8();
                    println!("crop {i}: {}x{} pixels identical: {}", ia.width(), ia.height(), ia.as_raw() == ib.as_raw());
                }
                _ => println!("crop {i}: MISMATCH"),
            }
        }
        return;
    }
    let root = PathBuf::from(&a[0]);
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    let (mut same, mut bad, mut both_null, mut png_bytes_same) = (0, 0, 0, 0);
    let t = std::time::Instant::now();
    for f in &files {
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace([char::from(92u8), '/'], "_");
        let bytes = std::fs::read(f).unwrap();
        let mine = tga::to_png(&bytes);
        let cs_png = PathBuf::from(&a[1]).join(format!("{rel}.png"));
        let cs_null = PathBuf::from(&a[1]).join(format!("{rel}.null"));
        match (mine, cs_png.exists()) {
            (None, false) if cs_null.exists() => both_null += 1,
            (Some(m), true) => {
                let theirs = std::fs::read(&cs_png).unwrap();
                let ia = image::load_from_memory(&m).unwrap().to_rgba8();
                let ib = image::load_from_memory(&theirs).unwrap().to_rgba8();
                if ia.dimensions() == ib.dimensions() && ia.as_raw() == ib.as_raw() {
                    same += 1;
                    if m == theirs {
                        png_bytes_same += 1;
                    }
                } else {
                    bad += 1;
                    println!("DIFF {}", f.display());
                }
            }
            _ => {
                bad += 1;
                println!("NULL MISMATCH {}", f.display());
            }
        }
    }
    println!("{} tga files: {same} identical pixels ({png_bytes_same} identical PNG bytes), {both_null} unreadable in both, {bad} different; {:.1} s", files.len(), t.elapsed().as_secs_f64());
}
