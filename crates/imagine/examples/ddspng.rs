//! Throwaway: ddspng <client dir> <name without extension> <out.png>: one of the client's Interface pictures (tga, else dds) as PNG.
use imagine::build::item_icons;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let png = item_icons::client_png(Path::new(&a[0]), &a[1], &["tga", "tga2", "dds"]).expect("no such picture");
    std::fs::write(&a[2], &png).expect("write");
    let b = &png;
    println!("{}x{} -> {}", u32::from_be_bytes([b[16], b[17], b[18], b[19]]), u32::from_be_bytes([b[20], b[21], b[22], b[23]]), a[2]);
}
