//! Port of `src/Data/TgaImage.cs`.
//!
//! Turns the client's TGA icons (Interface\tga, e.g. demon icons: 32x32, 32-bit, uncompressed) into PNG files the page
//! can show. Handles uncompressed and RLE true-colour TGA (24 or 32 bits); anything else returns None.
//!
//! The PNG bytes come from the `image` crate, so they are a valid PNG of the same picture, but not the same bytes
//! as .NET's zlib writes (compare pixels, not files).

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

/// A rectangle of the picture (x, y from the top left): one icon out of a sheet. C# `(int X, int Y, int W, int H)`.
pub type Crop = (i32, i32, i32, i32);

/// C# `ToPng(byte[] tga)`: PNG bytes of a TGA file, or None when it isn't a TGA this reads.
pub fn to_png(tga: &[u8]) -> Option<Vec<u8>> {
    to_png_crop(tga, None)
}

/// C# `ToPng(byte[] tga, crop)`: the same, for one rectangle of the picture. None if it doesn't fit.
pub fn to_png_crop(tga: &[u8], crop: Option<Crop>) -> Option<Vec<u8>> {
    let (rgba, w, h) = decode_rgba_crop(tga, crop)?;
    Some(rgba_to_png(&rgba, w, h))
}

/// The picture of a TGA file as RGBA bytes, rows from the top, with its width and height (what [`to_png`] encodes).
pub fn decode_rgba(tga: &[u8]) -> Option<(Vec<u8>, i32, i32)> {
    decode_rgba_crop(tga, None)
}

/// [`decode_rgba`] for one rectangle of the picture.
pub fn decode_rgba_crop(tga: &[u8], crop: Option<Crop>) -> Option<(Vec<u8>, i32, i32)> {
    if tga.len() < 18 {
        return None;
    }
    let id_len = tga[0] as usize;
    let cmap_type = tga[1];
    let ty = tga[2];
    let w = u16::from_le_bytes([tga[12], tga[13]]) as i32;
    let h = u16::from_le_bytes([tga[14], tga[15]]) as i32;
    let bpp = tga[16];
    let desc = tga[17];
    if cmap_type != 0 || (ty != 2 && ty != 10) || (bpp != 24 && bpp != 32) || w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    let px = (bpp / 8) as usize;
    let mut p = 18 + id_len;
    let total = (w * h) as usize;
    // BGRA, rows in the file's order
    let mut bgra = vec![0u8; total * 4];
    let mut n = 0usize;
    // C# Put(at): reads px bytes at `at`; past the end of the file is an IndexOutOfRangeException (-> None)
    let put = |bgra: &mut [u8], at: usize, n: &mut usize| -> Option<()> {
        let s = tga.get(at..at + px)?;
        bgra[*n * 4] = s[0];
        bgra[*n * 4 + 1] = s[1];
        bgra[*n * 4 + 2] = s[2];
        bgra[*n * 4 + 3] = if px == 4 { s[3] } else { 255 };
        *n += 1;
        Some(())
    };
    if ty == 2 {
        while n < total {
            put(&mut bgra, p, &mut n)?;
            p += px;
        }
    } else {
        while n < total {
            let head = *tga.get(p)? as usize;
            p += 1;
            let count = (head & 0x7F) + 1;
            if (head & 0x80) != 0 {
                let mut i = 0;
                while i < count && n < total {
                    put(&mut bgra, p, &mut n)?;
                    i += 1;
                }
                p += px;
            } else {
                let mut i = 0;
                while i < count && n < total {
                    put(&mut bgra, p, &mut n)?;
                    p += px;
                    i += 1;
                }
            }
        }
    }
    let top_down = (desc & 0x20) != 0;
    let (cx, cy, cw, ch) = match crop {
        Some((x, y, cw, ch)) => {
            if x < 0 || y < 0 || cw <= 0 || ch <= 0 || x + cw > w || y + ch > h {
                return None;
            }
            (x, y, cw, ch)
        }
        None => (0, 0, w, h),
    };
    // rows from the top, RGBA (the file's rows are bottom up unless the descriptor says top down)
    let mut out = vec![0u8; (cw * ch * 4) as usize];
    for y in 0..ch {
        let file_row = if top_down { cy + y } else { h - 1 - (cy + y) };
        for x in 0..cw {
            let s = ((file_row * w + cx + x) * 4) as usize;
            let d = ((y * cw + x) * 4) as usize;
            out[d] = bgra[s + 2];
            out[d + 1] = bgra[s + 1];
            out[d + 2] = bgra[s];
            out[d + 3] = bgra[s + 3];
        }
    }
    Some((out, cw, ch))
}

/// C# `RgbaToPng`: PNG bytes of a picture given as RGBA bytes, rows from the top (a model's texture).
pub fn rgba_to_png(rgba: &[u8], w: i32, h: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let enc = PngEncoder::new(&mut out);
    // writing into a Vec cannot fail; a short buffer is a caller bug the C# would have thrown on as well
    enc.write_image(rgba, w as u32, h as u32, ExtendedColorType::Rgba8).expect("PNG encode of an RGBA buffer");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(ty: u8, w: u16, h: u16, bpp: u8, desc: u8) -> Vec<u8> {
        let mut v = vec![0u8; 18];
        v[2] = ty;
        v[12..14].copy_from_slice(&w.to_le_bytes());
        v[14..16].copy_from_slice(&h.to_le_bytes());
        v[16] = bpp;
        v[17] = desc;
        v
    }

    #[test]
    fn bottom_up_32_bit_is_flipped_and_swizzled() {
        let mut t = header(2, 1, 2, 32, 0);
        t.extend_from_slice(&[1, 2, 3, 4]); // file row 0 = bottom: B G R A
        t.extend_from_slice(&[5, 6, 7, 8]); // top
        let (rgba, w, h) = decode_rgba(&t).unwrap();
        assert_eq!((w, h), (1, 2));
        assert_eq!(rgba, vec![7, 6, 5, 8, 3, 2, 1, 4]);
    }

    #[test]
    fn rle_24_bit_top_down_and_crop() {
        let mut t = header(10, 2, 2, 24, 0x20);
        t.extend_from_slice(&[0x83, 9, 8, 7]); // run of 4 pixels
        let (rgba, _, _) = decode_rgba(&t).unwrap();
        assert_eq!(rgba, [7, 8, 9, 255].repeat(4));
        let (c, w, h) = decode_rgba_crop(&t, Some((1, 1, 1, 1))).unwrap();
        assert_eq!((w, h), (1, 1));
        assert_eq!(c, vec![7, 8, 9, 255]);
        assert!(decode_rgba_crop(&t, Some((1, 1, 2, 1))).is_none());
    }

    #[test]
    fn short_or_unsupported_files_are_none() {
        assert!(to_png(&[0; 10]).is_none());
        assert!(to_png(&header(1, 1, 1, 8, 0)).is_none());
        let mut t = header(2, 2, 2, 32, 0);
        t.extend_from_slice(&[0; 15]); // one byte short
        assert!(to_png(&t).is_none());
    }

    #[test]
    fn png_round_trips() {
        let rgba: Vec<u8> = (0..16u8).collect();
        let png = rgba_to_png(&rgba, 2, 2);
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.into_raw(), rgba);
    }
}
