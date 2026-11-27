//! Throwaway: tgapng <in.tga> <out.png> [scale] [bg]: one of the client's Interface pictures as PNG, scaled up by
//! an integer and laid over a dark background when asked (white-on-transparent sheets are invisible otherwise).
use imagine::data::tga;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read");
    let scale: i32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let bg = a.get(3).is_some();
    let (px, w, h) = tga::decode_rgba(&bytes).expect("decode");
    let (ow, oh) = (w * scale, h * scale);
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    for y in 0..oh {
        for x in 0..ow {
            let i = (((y / scale) * w + x / scale) * 4) as usize;
            let (r, g, b, al) = (px[i] as u32, px[i + 1] as u32, px[i + 2] as u32, px[i + 3] as u32);
            let o = ((y * ow + x) * 4) as usize;
            if bg {
                let (br, bg_, bb) = (26u32, 27u32, 38u32);
                out[o] = ((r * al + br * (255 - al)) / 255) as u8;
                out[o + 1] = ((g * al + bg_ * (255 - al)) / 255) as u8;
                out[o + 2] = ((b * al + bb * (255 - al)) / 255) as u8;
                out[o + 3] = 255;
            } else {
                out[o..o + 4].copy_from_slice(&px[i..i + 4]);
            }
        }
    }
    std::fs::write(&a[1], tga::rgba_to_png(&out, ow, oh)).expect("write");
    println!("{}x{} -> {}", w, h, a[1]);
}
