//! Throwaway checker for the model port (not part of the product):
//!   model export <client folder> <model> <out file> [--latin1]   the assistant JSON + the log on stdout
//!   model check <folder or file>                                 `model --check`: read every .nif/.kf, count failures
use imagine::build::demon_model;
use imagine::data::nif::{NifFile, NifKind, is_nif_error};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("export") => {
            if a.iter().any(|x| x == "--latin1") {
                demon_model::set_shift_jis_names(false);
            }
            let t = Instant::now();
            let mut log = |s: &str| println!("{s}");
            let json = demon_model::export(&PathBuf::from(&a[1]), &a[2], Some(&mut log)).expect("export");
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let json = json.expect("no such model");
            std::fs::write(&a[3], json.as_bytes()).expect("write");
            eprintln!("export {} took {ms:.1} ms ({} KB)", a[2], json.len() / 1024);
        }
        Some("check") => check(&a[1]),
        Some("exportall") => exportall(&a[1], &a[2], !a.iter().any(|x| x == "--latin1")),
        _ => eprintln!("usage: model export <client> <model> <out> | model check <folder>"),
    }
}

fn check(target: &str) {
    let p = PathBuf::from(target);
    let files: Vec<PathBuf> = if p.is_dir() {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&p)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|f| {
                let n = f.to_string_lossy().to_lowercase();
                n.ends_with(".nif") || n.ends_with(".kf")
            })
            .collect();
        v.sort();
        v
    } else {
        vec![p]
    };
    let t = Instant::now();
    let mut ok = 0;
    let mut why: Vec<(String, usize)> = Vec::new();
    let mut other = 0;
    let mut kinds: HashMap<String, usize> = HashMap::new();
    for f in &files {
        match NifFile::load(f) {
            Ok(nif) => {
                ok += 1;
                for b in &nif.blocks {
                    *kinds.entry(b.type_name.clone()).or_default() += 1;
                    let _ = matches!(b.kind, NifKind::Object);
                }
            }
            Err(e) if is_nif_error(&e) => {
                let m = regex::Regex::new(r"^.*?: |block \d+ | at byte \d+|^\S+ is ").unwrap().replace_all(&e.to_string(), "").into_owned();
                match why.iter_mut().find(|(k, _)| *k == m) {
                    Some(x) => x.1 += 1,
                    None => why.push((m, 1)),
                }
            }
            Err(e) => {
                other += 1;
                println!("OTHER ERROR {}: {e}", f.display());
            }
        }
    }
    println!("{ok} of {} files read.", files.len());
    why.sort_by(|a, b| b.1.cmp(&a.1));
    for (m, n) in &why {
        println!("{n:>5}  {m}");
    }
    if other > 0 {
        println!("{other} files failed another way");
    }
    let mut ks: Vec<_> = kinds.into_iter().collect();
    ks.sort();
    let total: usize = ks.iter().map(|k| k.1).sum();
    eprintln!("{} block types, {total} blocks; check took {:.1} ms", ks.len(), t.elapsed().as_secs_f64() * 1000.0);
}

/// exportall <client> <outdir> [--latin1]: every Devil\bone\*_Base.nif, one json (or .err) per model
fn exportall(client: &str, out: &str, sjis: bool) {
    demon_model::set_shift_jis_names(sjis);
    std::fs::create_dir_all(out).unwrap();
    let t = Instant::now();
    let mut dir: Vec<String> = std::fs::read_dir(PathBuf::from(client).join("Devil").join("bone"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with("_Base.nif"))
        .collect();
    dir.sort();
    let n = dir.len();
    for f in dir {
        let model = f.trim_end_matches("_Base.nif").to_string();
        match demon_model::export(&PathBuf::from(client), &model, None) {
            Ok(j) => std::fs::write(PathBuf::from(out).join(format!("{model}.json")), j.unwrap_or_else(|| "null".into())).unwrap(),
            Err(e) => std::fs::write(PathBuf::from(out).join(format!("{model}.err")), e.to_string()).unwrap(),
        }
    }
    eprintln!("exported {n} models in {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
}
