//! A character's picture: a photo of the game window (or any image file) that the person crops in the app
//! (C# `CharacterPicture`, app\CharacterPicture.cs).
//!   picture-source.png   the whole photo or chosen image, kept so the crop can be redone
//!   picture.png          the cropped picture the app shows
//! Both live in the character's own folder.
//!
//! The photo is taken by asking Windows for the window's picture as the desktop compositor has it
//! (PrintWindow with PW_RENDERFULLCONTENT). Nothing is read from or sent into the game itself, and it works while
//! another window is in front of the game. It does not work while the game is minimized.

use image::{ImageBuffer, Rgb, RgbImage};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsIconic};

use crate::site::AssetFiles;

pub const SOURCE_FILE: &str = "picture-source.png";
pub const PICTURE_FILE: &str = "picture.png";
/// every saved picture has this shape: 4 wide to 3 tall
pub const ASPECT: f64 = 4.0 / 3.0;
/// and is at most this big
const MAX_WIDTH: i32 = 480;
const MAX_HEIGHT: i32 = 360;

/// C# `InvalidOperationException` (a message for the person) versus anything else that went wrong.
#[derive(Debug)]
pub enum PicError {
    /// shown to the person as it is
    Invalid(String),
    Other(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for PicError {
    fn from(e: E) -> Self {
        PicError::Other(e.into())
    }
}

fn invalid<T>(msg: &str) -> Result<T, PicError> {
    Err(PicError::Invalid(msg.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A photo of a window: BGRA as the DIB holds it.
pub struct Photo {
    pub w: i32,
    pub h: i32,
    bgra: Vec<u8>,
}

impl Photo {
    fn to_rgb(&self) -> RgbImage {
        let mut img = RgbImage::new(self.w as u32, self.h as u32);
        for (i, p) in img.pixels_mut().enumerate() {
            let o = i * 4;
            *p = Rgb([self.bgra[o + 2], self.bgra[o + 1], self.bgra[o]]);
        }
        img
    }
}

/// Greyscale copy: (blue + 2 green + red) / 4, row by row.
struct Grey {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Grey {
    fn at(&self, y: usize, x: usize) -> u8 {
        self.px[y * self.w + x]
    }
}

fn grey_of_bgra(w: usize, h: usize, bgra: &[u8]) -> Grey {
    let mut px = vec![0u8; w * h];
    for (i, g) in px.iter_mut().enumerate() {
        let o = i * 4;
        *g = ((bgra[o] as i32 + bgra[o + 1] as i32 * 2 + bgra[o + 2] as i32) / 4) as u8;
    }
    Grey { w, h, px }
}

/// Photo of a window's inside area.
fn photograph(hwnd: isize) -> Result<Photo, PicError> {
    if hwnd == 0 {
        return invalid("The game window wasn't found.");
    }
    let h = HWND(hwnd as *mut c_void);
    unsafe {
        if IsIconic(h).as_bool() {
            return invalid("The game window is minimized. Bring it back on screen, then capture again.");
        }
        let mut r = RECT::default();
        if GetClientRect(h, &mut r).is_err() || r.right < 64 || r.bottom < 64 {
            return invalid("The game window has no visible area to capture.");
        }
        let (w, hgt) = (r.right, r.bottom);
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -hgt; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        let dc = CreateCompatibleDC(None);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        let Ok(bmp) = bmp else {
            let _ = DeleteDC(dc);
            return Err(PicError::Other(anyhow::anyhow!("could not make a bitmap")));
        };
        let old = SelectObject(dc, HGDIOBJ(bmp.0));
        // PW_CLIENTONLY | PW_RENDERFULLCONTENT
        let ok = PrintWindow(h, dc, PRINT_WINDOW_FLAGS(1 | 2)).as_bool();
        let mut photo = Photo { w, h: hgt, bgra: vec![0u8; (w as usize) * (hgt as usize) * 4] };
        if !bits.is_null() {
            std::ptr::copy_nonoverlapping(bits as *const u8, photo.bgra.as_mut_ptr(), photo.bgra.len());
        }
        SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(dc);
        if ok && !mostly_black(&photo) {
            return Ok(photo);
        }
    }
    invalid("Windows returned an empty picture of the game window. If the game runs full screen, switch it to a window and try again.")
}

fn mostly_black(p: &Photo) -> bool {
    let (mut lit, mut total) = (0, 0);
    let ys = (p.h / 24).max(1);
    let xs = (p.w / 32).max(1);
    let mut y = 8;
    while y < p.h {
        let mut x = 8;
        while x < p.w {
            let o = ((y * p.w + x) * 4) as usize;
            total += 1;
            if p.bgra[o] as i32 + p.bgra[o + 1] as i32 + p.bgra[o + 2] as i32 > 24 {
                lit += 1;
            }
            x += xs;
        }
        y += ys;
    }
    total == 0 || lit * 20 < total // under 5% of the samples show anything
}

/// Photo of the game window, saved as the character's source picture. Returns its size and, when the in-game
/// Information window is open, where its character view is in the photo (a ready-made selection).
pub fn capture_window(hwnd: isize, folder: &Path) -> Result<((i32, i32), Option<Rect>), PicError> {
    let photo = photograph(hwnd)?;
    let origin = find_info_window(&photo);
    let size = save_source(&photo.to_rgb(), folder)?;
    Ok((size, origin.map(|(ox, oy)| Rect { x: ox + VIEW.x, y: oy + VIEW.y, w: VIEW.w, h: VIEW.h })))
}

// ---------- the Information window's character view ----------
// The game draws its windows at a fixed size with fixed, opaque pixels (checked 2026-11-23: every patch of the
// window compared against a photo matched exactly, in a 1920x1200 client). So the window is found by two patches of
// its own label column ("Level Sex Align. HP MP EXP" and "Str. Magic Vital. Intel. Speed Luck"), which are the
// same for every character. Positions below are relative to the window's top-left corner (window is 377x525).
const LABELS1_AT: (i32, i32) = (8, 56);
const LABELS2_AT: (i32, i32) = (8, 176);
/// the black panel the character is drawn in
const VIEW: Rect = Rect { x: 190, y: 116, w: 180, h: 382 };
/// the game's RESET button, drawn over that panel
const RESET_BUTTON: Rect = Rect { x: 195, y: 474, w: 52, h: 18 };
/// mean grey difference per pixel that still counts as "the same"
const MATCH_LIMIT: i64 = 4;

static LABELS: OnceLock<Option<(Grey, Grey)>> = OnceLock::new();

fn labels() -> Option<&'static (Grey, Grey)> {
    LABELS
        .get_or_init(|| {
            let load = |name: &str| -> Option<Grey> {
                let f = AssetFiles::get(name)?;
                let img = image::load_from_memory(&f.data).ok()?.to_rgba8();
                let (w, h) = (img.width() as usize, img.height() as usize);
                let mut bgra = Vec::with_capacity(w * h * 4);
                for p in img.pixels() {
                    bgra.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
                }
                Some(grey_of_bgra(w, h, &bgra))
            };
            let a = load("info-window-labels-1.png")?;
            let b = load("info-window-labels-2.png")?;
            Some((a, b))
        })
        .as_ref()
}

fn matches(photo: &Grey, patch: &Grey, x: i32, y: i32) -> bool {
    let (h, w) = (patch.h as i32, patch.w as i32);
    if x < 0 || y < 0 || y + h > photo.h as i32 || x + w > photo.w as i32 {
        return false;
    }
    let limit = MATCH_LIMIT * w as i64 * h as i64;
    let mut sum = 0i64;
    for j in 0..h as usize {
        for i in 0..w as usize {
            sum += (photo.at(y as usize + j, x as usize + i) as i64 - patch.at(j, i) as i64).abs();
        }
        if sum > limit {
            return false;
        }
    }
    true
}

/// Top-left corner of the Information window in a photo of the game, or None when it isn't open (or is covered).
pub fn find_info_window(photo: &Photo) -> Option<(i32, i32)> {
    let (l1, l2) = labels()?;
    let g = grey_of_bgra(photo.w as usize, photo.h as usize, &photo.bgra);
    let (ph, pw) = (g.h as i32, g.w as i32);
    let (h, w) = (l1.h as i32, l1.w as i32);
    // a few pixels of the first patch decide quickly whether a spot is worth a full comparison
    let probes = [(2, 2), (w / 2, h / 4), (w - 3, h / 2), (w / 3, h - 3), (w / 2, h / 2), (5, h * 3 / 4)];
    let mut y = 0;
    while y + h <= ph {
        let mut x = 0;
        while x + w <= pw {
            let maybe = probes
                .iter()
                .all(|&(i, j)| (g.at((y + j) as usize, (x + i) as usize) as i32 - l1.at(j as usize, i as usize) as i32).abs() <= 14);
            if maybe && matches(&g, l1, x, y) {
                let origin = (x - LABELS1_AT.0, y - LABELS1_AT.1);
                if matches(&g, l2, origin.0 + LABELS2_AT.0, origin.1 + LABELS2_AT.1)
                    && !(origin.0 + VIEW.x + VIEW.w > pw || origin.1 + VIEW.y + VIEW.h > ph)
                {
                    return Some(origin);
                }
            }
            x += 1;
        }
        y += 1;
    }
    None
}

/// If the Information window is open in the game: save the top of its character view as this character's picture
/// (and the whole view as the source, so it can still be re-cropped by hand). Returns false when the window isn't showing.
pub fn from_info_window(hwnd: isize, folder: &Path) -> Result<bool, PicError> {
    let photo = photograph(hwnd)?;
    let Some(origin) = find_info_window(&photo) else { return Ok(false) };
    let full = photo.to_rgb();
    let (vx, vy) = (origin.0 + VIEW.x, origin.1 + VIEW.y);
    let mut cut = RgbImage::new(VIEW.w as u32, VIEW.h as u32);
    for y in 0..VIEW.h as u32 {
        for x in 0..VIEW.w as u32 {
            cut.put_pixel(x, y, *full.get_pixel(vx as u32 + x, vy as u32 + y));
        }
    }
    // the RESET button is part of the game's window, not of the character: paint the panel's black over it
    for y in (RESET_BUTTON.y - VIEW.y)..(RESET_BUTTON.y - VIEW.y + RESET_BUTTON.h) {
        for x in (RESET_BUTTON.x - VIEW.x)..(RESET_BUTTON.x - VIEW.x + RESET_BUTTON.w) {
            if x >= 0 && y >= 0 && x < VIEW.w && y < VIEW.h {
                cut.put_pixel(x as u32, y as u32, Rgb([0, 0, 0]));
            }
        }
    }
    // The whole view is kept as the source (so it can be re-cropped by hand); the picture is its top part, where
    // the face is, in the one shape every picture has.
    save_source(&cut, folder)?;
    crop(folder, 0, 0, VIEW.w, newmoon::fmt::round(VIEW.w as f64 / ASPECT, 0) as i32)?;
    Ok(true)
}

/// Any image file as the source picture.
pub fn use_file(path: &Path, folder: &Path) -> Result<(i32, i32), PicError> {
    let img = image::open(path)?.to_rgba8();
    // drawn on black: transparent parts become black
    let mut out = RgbImage::new(img.width(), img.height());
    for (o, p) in out.pixels_mut().zip(img.pixels()) {
        let a = p[3] as u32;
        *o = Rgb([(p[0] as u32 * a / 255) as u8, (p[1] as u32 * a / 255) as u8, (p[2] as u32 * a / 255) as u8]);
    }
    save_source(&out, folder)
}

fn save_source(img: &RgbImage, folder: &Path) -> Result<(i32, i32), PicError> {
    std::fs::create_dir_all(folder)?;
    img.save_with_format(folder.join(SOURCE_FILE), image::ImageFormat::Png)?;
    Ok((img.width() as i32, img.height() as i32))
}

pub fn source_size(folder: &Path) -> Option<(i32, i32)> {
    let p = folder.join(SOURCE_FILE);
    if !p.is_file() {
        return None;
    }
    image::image_dimensions(p).ok().map(|(w, h)| (w as i32, h as i32))
}

/// Cut the chosen rectangle (source pixels) out of the source picture and save it as the picture.
pub fn crop(folder: &Path, x: i32, y: i32, w: i32, h: i32) -> Result<(), PicError> {
    let src = folder.join(SOURCE_FILE);
    if !src.is_file() {
        return invalid("There is no picture to crop yet.");
    }
    let img: RgbImage = image::open(&src)?.to_rgb8();
    let (iw, ih) = (img.width() as i64, img.height() as i64);
    // Rectangle.Intersect
    let l = (x as i64).max(0);
    let t = (y as i64).max(0);
    let r = (x as i64 + w as i64).min(iw);
    let b = (y as i64 + h as i64).min(ih);
    let (mut rw, mut rh) = if r > l && b > t { ((r - l) as i32, (b - t) as i32) } else { (0, 0) };
    if rw < 16 || rh < 16 {
        return invalid("That selection is too small.");
    }
    // one shape for every picture, measured from the top of the selection (where the face is)
    let tall = newmoon::fmt::round(rw as f64 / ASPECT, 0) as i32;
    if tall <= rh {
        rh = tall;
    } else {
        rw = newmoon::fmt::round(rh as f64 * ASPECT, 0) as i32;
    }
    let scale = f64::min(1.0, f64::min(MAX_WIDTH as f64 / rw as f64, MAX_HEIGHT as f64 / rh as f64));
    let ow = ((newmoon::fmt::round(rw as f64 * scale, 0)) as i32).max(1);
    let oh = ((newmoon::fmt::round(rh as f64 * scale, 0)) as i32).max(1);
    let view = image::imageops::crop_imm(&img, l as u32, t as u32, rw as u32, rh as u32).to_image();
    let cut: ImageBuffer<Rgb<u8>, Vec<u8>> = if ow == rw && oh == rh {
        view
    } else {
        image::imageops::resize(&view, ow as u32, oh as u32, image::imageops::FilterType::CatmullRom)
    };
    // write next to the target first, so a failure never leaves half a picture
    let target = folder.join(PICTURE_FILE);
    let tmp = PathBuf::from(format!("{}.tmp", target.display()));
    cut.save_with_format(&tmp, image::ImageFormat::Png)?;
    std::fs::rename(&tmp, &target)?;
    Ok(())
}

/// Removes the picture and its source (only these two files the app made itself).
pub fn remove(folder: &Path) -> std::io::Result<()> {
    for f in [PICTURE_FILE, SOURCE_FILE] {
        let p = folder.join(f);
        if p.is_file() {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}
