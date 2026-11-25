//! A Build Planner build as a short text code, to pass to another player ("BABEL1-..."), and back.
//!
//! The code is INPUT FROM A STRANGER: whatever a player pastes goes through [`decode`]. So the format carries numbers
//! only (no name, no text, no path, no link), has no compression (nothing can expand), and [`decode`] accepts exactly
//! the codes [`encode`] writes: after reading one it writes the build again and compares, so a code with anything
//! extra, missing, out of range or written another way is refused. This module knows nothing of the game data: whether
//! the numbers name real pieces, crystals and expertises is checked by the Simulator (`Simulator::import`) before a
//! build reaches the page.
//!
//! Layout (version 1; the version is the digit in the prefix):
//!   text  = "BABEL1-" + base64url(bytes), no padding
//!   bytes = CRC-16/CCITT-FALSE of the rest (2 bytes, high first), then the bits below, zero-padded to a whole byte
//!   bits  = level (7) · alignment (2: 0 Neutral, 1 Law, 2 Chaos) · has cap (1) [cap (24)] · which of the 15 slots hold
//!           a piece (15) · each piece · number of expertises (6) · each expertise
//!   piece = id (18) · basic · special (each: 1 = the piece itself, else 0 + signed difference from the id)
//!           · tarot · soul (each: 0 = none, else 1 + number) · fusion bonuses (0 = none, else 1 + 3 numbers)
//!           · mods (0 = none, else 1 + 5 numbers)
//!   expertise = id (7), ids rising · 1 + whole ranks (7), or 0 + points (20)
//!   number = width (2: 6, 10, 14 or 19 bits; the smallest that holds it) + the value
//!   signed = the same, on 2n for n >= 0 and -2n-1 for n < 0
//!
//! The checksum is for a code cut short or mangled in a chat, not for safety: anyone can write a code with a right
//! checksum, which is why nothing in it is trusted.

use crate::{Result, user_error};

pub const PREFIX: &str = "BABEL1-";
/// What a player may paste (before spaces and line breaks are taken out); the longest real code is under 400.
pub const MAX_INPUT_CHARS: usize = 2048;
/// The longest code this version writes or reads (15 full pieces and 63 expertises are under it).
pub const MAX_CODE_CHARS: usize = 1024;

pub const SLOTS: usize = 15;
/// Item and crystal ids, mods and fusion bonuses all fit 18 bits.
pub const MAX_NUMBER: i32 = (1 << 18) - 1;
pub const MAX_LEVEL: i32 = 99;
pub const MAX_CAP: i32 = (1 << 24) - 1;
pub const MAX_EXPERTISE_ID: i32 = 127;
/// Class 10, rank 0 (10000 points per rank): no expertise goes further.
pub const MAX_POINTS: i32 = 1_000_000;
pub const MAX_EXPERTISES: usize = 63;

pub const ALIGNMENTS: [&str; 3] = ["NEUTRAL", "LAW", "CHAOS"];

/// One worn piece. `basic` and `special` are always item ids (the piece's own id when it carries its own).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub id: i32,
    pub basic: i32,
    pub special: i32,
    pub tarot: i32,
    pub soul: i32,
    pub fuse: [i32; 3],
    pub mods: [i32; 5],
}

/// What a code holds: a whole build and the level and alignment it was made at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shared {
    pub level: i32,
    /// Index into [`ALIGNMENTS`].
    pub alignment: usize,
    pub cap: Option<i32>,
    pub slots: [Option<Piece>; SLOTS],
    /// (expertise id, points), ids rising, points above 0.
    pub points: Vec<(i32, i32)>,
}

const BAD: &str = "That isn’t a build code. Copy the whole code, from BABEL1- to its last letter, and paste it again.";
const DAMAGED: &str = "That build code is damaged: a part of it is missing or was changed. Ask for it again and paste the whole code.";
const NEWER: &str = "That build code was made with a newer version of BABEL. Update BABEL to use it.";

/// Whether every number is one a code can hold. `Err`: what is not (for a build being shared; a code read never says).
fn check(b: &Shared) -> std::result::Result<(), &'static str> {
    if !(1..=MAX_LEVEL).contains(&b.level) {
        return Err("its level is not between 1 and 99");
    }
    if b.alignment >= ALIGNMENTS.len() {
        return Err("its alignment is not Law, Neutral or Chaos");
    }
    if b.cap.is_some_and(|c| !(0..=MAX_CAP).contains(&c)) {
        return Err("its expertise point cap is out of range");
    }
    for p in b.slots.iter().flatten() {
        let ids = [p.id, p.basic, p.special];
        if ids.iter().any(|v| !(1..=MAX_NUMBER).contains(v)) {
            return Err("one of its pieces has an id out of range");
        }
        if [p.tarot, p.soul].iter().chain(&p.fuse).chain(&p.mods).any(|v| !(0..=MAX_NUMBER).contains(v)) {
            return Err("one of its pieces has a number out of range");
        }
    }
    if b.points.len() > MAX_EXPERTISES {
        return Err("it has too many expertises");
    }
    let mut last = -1;
    for &(id, pts) in &b.points {
        if id <= last || id > MAX_EXPERTISE_ID {
            return Err("one of its expertises has an id out of range");
        }
        if !(1..=MAX_POINTS).contains(&pts) {
            return Err("one of its expertises has points out of range");
        }
        last = id;
    }
    Ok(())
}

// ---------- bits ----------

struct BitWriter {
    bytes: Vec<u8>,
    used: usize,
}

impl BitWriter {
    fn put(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if value >> i & 1 == 1 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 0x80 >> (self.used % 8);
            }
            self.used += 1;
        }
    }

    fn flag(&mut self, on: bool) {
        self.put(u32::from(on), 1);
    }

    /// A number in the smallest of the four widths that holds it.
    fn number(&mut self, value: u32) {
        let (pick, bits) = WIDTHS.iter().enumerate().find(|(_, w)| value >> **w == 0).map_or((3, 19), |(i, w)| (i as u32, *w));
        self.put(pick, 2);
        self.put(value, bits);
    }

    fn signed(&mut self, value: i32) {
        self.number(if value < 0 { (-(value as i64) * 2 - 1) as u32 } else { value as u32 * 2 });
    }
}

/// The last is 19: the difference between two 18-bit ids, as a signed number, needs it.
const WIDTHS: [u32; 4] = [6, 10, 14, 19];

struct BitReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl BitReader<'_> {
    /// `None`: the code ends before this.
    fn get(&mut self, bits: u32) -> Option<u32> {
        let mut v = 0u32;
        for _ in 0..bits {
            let byte = *self.bytes.get(self.at / 8)?;
            v = v << 1 | u32::from(byte >> (7 - self.at % 8) & 1);
            self.at += 1;
        }
        Some(v)
    }

    fn flag(&mut self) -> Option<bool> {
        Some(self.get(1)? == 1)
    }

    fn number(&mut self) -> Option<i32> {
        let pick = self.get(2)? as usize;
        Some(self.get(WIDTHS[pick])? as i32)
    }

    fn signed(&mut self) -> Option<i32> {
        let z = self.number()?;
        Some(if z & 1 == 1 { -((z + 1) / 2) } else { z / 2 })
    }
}

/// CRC-16/CCITT-FALSE (polynomial 0x1021, start 0xFFFF).
fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in bytes {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { crc << 1 ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn to_base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 3);
    for chunk in bytes.chunks(3) {
        let n = u32::from(chunk[0]) << 16 | u32::from(*chunk.get(1).unwrap_or(&0)) << 8 | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// `None`: a character that is not in the alphabet, or a length no whole number of bytes gives.
fn from_base64url(text: &str) -> Option<Vec<u8>> {
    if text.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = B64.iter().position(|&x| x == c)? as u32;
        acc = (acc << 6 | v) & 0xFFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

// ---------- the code ----------

/// The build as a code. `Err` (a user error saying what) when a number in it is one no code can hold.
pub fn encode(b: &Shared) -> Result<String> {
    check(b).map_err(|why| user_error(format!("This build can’t be shared: {why}.")))?;
    let mut w = BitWriter { bytes: Vec::new(), used: 0 };
    w.put(b.level as u32, 7);
    w.put(b.alignment as u32, 2);
    w.flag(b.cap.is_some());
    if let Some(cap) = b.cap {
        w.put(cap as u32, 24);
    }
    for s in &b.slots {
        w.flag(s.is_some());
    }
    for p in b.slots.iter().flatten() {
        w.put(p.id as u32, 18);
        for other in [p.basic, p.special] {
            w.flag(other == p.id);
            if other != p.id {
                w.signed(other - p.id);
            }
        }
        for crystal in [p.tarot, p.soul] {
            w.flag(crystal != 0);
            if crystal != 0 {
                w.number(crystal as u32);
            }
        }
        for list in [&p.fuse[..], &p.mods[..]] {
            let any = list.iter().any(|&v| v != 0);
            w.flag(any);
            if any {
                list.iter().for_each(|&v| w.number(v as u32));
            }
        }
    }
    w.put(b.points.len() as u32, 6);
    for &(id, pts) in &b.points {
        w.put(id as u32, 7);
        w.flag(pts % 10000 == 0);
        if pts % 10000 == 0 {
            w.put((pts / 10000) as u32, 7);
        } else {
            w.put(pts as u32, 20);
        }
    }
    let mut bytes = crc16(&w.bytes).to_be_bytes().to_vec();
    bytes.extend(w.bytes);
    Ok(format!("{PREFIX}{}", to_base64url(&bytes)))
}

/// The bits as a build; nothing is checked here but that the code is long enough. `None`: it ends too soon.
fn read(bytes: &[u8]) -> Option<Shared> {
    let mut r = BitReader { bytes, at: 0 };
    let level = r.get(7)? as i32;
    let alignment = r.get(2)? as usize;
    let cap = if r.flag()? { Some(r.get(24)? as i32) } else { None };
    let mut present = [false; SLOTS];
    for p in present.iter_mut() {
        *p = r.flag()?;
    }
    let mut slots: [Option<Piece>; SLOTS] = Default::default();
    for (k, _) in present.iter().enumerate().filter(|(_, on)| **on) {
        let id = r.get(18)? as i32;
        let mut others = [id; 2];
        for o in others.iter_mut() {
            if !r.flag()? {
                *o = id.checked_add(r.signed()?)?;
            }
        }
        let mut crystals = [0; 2];
        for c in crystals.iter_mut() {
            if r.flag()? {
                *c = r.number()?;
            }
        }
        let mut fuse = [0; 3];
        if r.flag()? {
            for v in fuse.iter_mut() {
                *v = r.number()?;
            }
        }
        let mut mods = [0; 5];
        if r.flag()? {
            for v in mods.iter_mut() {
                *v = r.number()?;
            }
        }
        slots[k] = Some(Piece { id, basic: others[0], special: others[1], tarot: crystals[0], soul: crystals[1], fuse, mods });
    }
    let count = r.get(6)? as usize;
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let id = r.get(7)? as i32;
        let pts = if r.flag()? { r.get(7)? as i32 * 10000 } else { r.get(20)? as i32 };
        points.push((id, pts));
    }
    Some(Shared { level, alignment, cap, slots, points })
}

/// What a player pasted, as the code in it: spaces and line breaks taken out, and the marks a chat puts around a code.
fn tidy(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace() && !matches!(c, '`' | '"' | '\'' | '<' | '>')).collect()
}

/// A pasted code as a build. Every failure is a user error with one of three plain messages; nothing of the input is
/// repeated in them.
pub fn decode(input: &str) -> Result<Shared> {
    if input.len() > MAX_INPUT_CHARS {
        return Err(user_error(BAD));
    }
    let code = tidy(input);
    let Some(body) = code.strip_prefix(PREFIX) else {
        // "BABEL2-...": this version can't know what it holds
        let newer = code.strip_prefix("BABEL").is_some_and(|rest| {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            digits > 0 && rest[digits..].starts_with('-')
        });
        return Err(user_error(if newer { NEWER } else { BAD }));
    };
    if code.len() > MAX_CODE_CHARS {
        return Err(user_error(BAD));
    }
    // from here on it is one of ours by its start: what is wrong with it is damage, unless it holds a letter no code has
    let Some(bytes) = from_base64url(body) else {
        return Err(user_error(if body.bytes().all(|c| B64.contains(&c)) { DAMAGED } else { BAD }));
    };
    if bytes.len() < 3 {
        return Err(user_error(DAMAGED));
    }
    if crc16(&bytes[2..]).to_be_bytes() != bytes[..2] {
        return Err(user_error(DAMAGED));
    }
    let build = read(&bytes[2..]).ok_or_else(|| user_error(DAMAGED))?;
    // Only what encode() writes is a code: anything out of range, left over or written another way differs here.
    match encode(&build) {
        Ok(again) if again == code => Ok(build),
        _ => Err(user_error(DAMAGED)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: i32) -> Piece {
        Piece { id, basic: id, special: id, tarot: 0, soul: 0, fuse: [0; 3], mods: [0; 5] }
    }

    fn sample() -> Shared {
        let mut slots: [Option<Piece>; SLOTS] = Default::default();
        slots[0] = Some(Piece { id: 2625, basic: 2625, special: 2616, tarot: 64, soul: 3105, fuse: [50, 50, 50], mods: [20, 15, 200, 319, 321] });
        slots[1] = Some(piece(2729));
        slots[4] = Some(Piece { id: 25262, basic: 25245, special: 4889, tarot: 64, soul: 37, fuse: [0; 3], mods: [0; 5] });
        slots[14] = Some(Piece { id: 135140, basic: 1, special: MAX_NUMBER, tarot: MAX_NUMBER, soul: 1, fuse: [1, 0, 0], mods: [0, 0, 0, 0, 255] });
        Shared { level: 95, alignment: 2, cap: Some(2_650_000), slots, points: vec![(3, 900_000), (4, 1_000_000), (11, 12_345), (127, 1)] }
    }

    /// A small generator with a fixed start, so a failure can be run again (xorshift).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn random_build(r: &mut Rng) -> Shared {
        let big = |r: &mut Rng| -> i32 {
            match r.below(4) {
                0 => r.below(64) as i32,
                1 => r.below(1024) as i32,
                2 => r.below(16384) as i32,
                _ => r.below(MAX_NUMBER as u64 + 1) as i32,
            }
        };
        let mut slots: [Option<Piece>; SLOTS] = Default::default();
        for s in slots.iter_mut() {
            if r.below(3) == 0 {
                continue;
            }
            let id = big(r).max(1);
            let near = |r: &mut Rng| if r.below(2) == 0 { id } else { big(r).max(1) };
            *s = Some(Piece {
                id,
                basic: near(r),
                special: near(r),
                tarot: if r.below(2) == 0 { 0 } else { big(r) },
                soul: if r.below(2) == 0 { 0 } else { big(r) },
                fuse: if r.below(2) == 0 { [0; 3] } else { [big(r), big(r), big(r)] },
                mods: if r.below(2) == 0 { [0; 5] } else { [big(r), big(r), big(r), big(r), big(r)] },
            });
        }
        let mut points = Vec::new();
        for id in 0..=MAX_EXPERTISE_ID {
            if points.len() < MAX_EXPERTISES && r.below(3) == 0 {
                points.push((id, if r.below(2) == 0 { (r.below(100) as i32 + 1) * 10000 } else { r.below(MAX_POINTS as u64) as i32 + 1 }));
            }
        }
        Shared {
            level: r.below(99) as i32 + 1,
            alignment: r.below(3) as usize,
            cap: if r.below(2) == 0 { None } else { Some(r.below(MAX_CAP as u64 + 1) as i32) },
            slots,
            points,
        }
    }

    fn message(r: Result<Shared>) -> String {
        let e = r.expect_err("the code should be refused");
        assert!(crate::error::is_user_error(&e), "a refused code is always a plain message for the player: {e:?}");
        e.downcast_ref::<crate::UserError>().map(|u| u.0.clone()).unwrap_or_default()
    }

    #[test]
    fn a_build_comes_back_as_it_went_in() {
        let b = sample();
        let code = encode(&b).unwrap();
        assert!(code.starts_with(PREFIX));
        assert!(code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'), "only letters a chat leaves alone: {code}");
        assert_eq!(decode(&code).unwrap(), b);
    }

    #[test]
    fn an_empty_build_is_a_few_letters() {
        let b = Shared { level: 1, alignment: 0, cap: None, slots: Default::default(), points: Vec::new() };
        let code = encode(&b).unwrap();
        assert_eq!(code.len(), PREFIX.len() + 8);
        assert_eq!(decode(&code).unwrap(), b);
    }

    #[test]
    fn the_fullest_build_fits_the_limit() {
        let full = Piece { id: MAX_NUMBER, basic: 1, special: 1, tarot: MAX_NUMBER, soul: MAX_NUMBER, fuse: [MAX_NUMBER; 3], mods: [MAX_NUMBER; 5] };
        let slots: [Option<Piece>; SLOTS] = std::array::from_fn(|_| Some(full.clone()));
        let b = Shared { level: 99, alignment: 2, cap: Some(MAX_CAP), slots, points: (0..MAX_EXPERTISES as i32).map(|i| (i, 999_999)).collect() };
        let code = encode(&b).unwrap();
        assert!(code.len() <= MAX_CODE_CHARS, "{} letters", code.len());
        assert_eq!(decode(&code).unwrap(), b);
    }

    #[test]
    fn what_a_chat_adds_around_a_code_is_ignored() {
        let b = sample();
        let code = encode(&b).unwrap();
        let (head, tail) = code.split_at(40);
        assert_eq!(decode(&format!("  `{head}\r\n{tail}`\n")).unwrap(), b);
        assert_eq!(decode(&format!("<{code}>")).unwrap(), b);
    }

    #[test]
    fn random_builds_round_trip() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..3000 {
            let b = random_build(&mut r);
            let code = encode(&b).unwrap();
            assert_eq!(decode(&code).unwrap(), b, "{code}");
        }
    }

    #[test]
    fn a_build_no_code_can_hold_is_not_shared() {
        let ok = sample();
        let bad: Vec<Box<dyn Fn(&mut Shared)>> = vec![
            Box::new(|b| b.level = 0),
            Box::new(|b| b.level = 100),
            Box::new(|b| b.alignment = 3),
            Box::new(|b| b.cap = Some(-1)),
            Box::new(|b| b.cap = Some(MAX_CAP + 1)),
            Box::new(|b| b.slots[0].as_mut().unwrap().id = 0),
            Box::new(|b| b.slots[0].as_mut().unwrap().basic = MAX_NUMBER + 1),
            Box::new(|b| b.slots[0].as_mut().unwrap().special = -3),
            Box::new(|b| b.slots[0].as_mut().unwrap().tarot = -1),
            Box::new(|b| b.slots[0].as_mut().unwrap().mods[4] = MAX_NUMBER + 1),
            Box::new(|b| b.slots[0].as_mut().unwrap().fuse[0] = i32::MIN),
            Box::new(|b| b.points.push((128, 10000))),
            Box::new(|b| b.points.insert(0, (5, 10000))),
            Box::new(|b| b.points[0].1 = 0),
            Box::new(|b| b.points[0].1 = MAX_POINTS + 1),
            Box::new(|b| b.points = (0..64).map(|i| (i, 10000)).collect()),
        ];
        for (i, change) in bad.iter().enumerate() {
            let mut b = ok.clone();
            change(&mut b);
            let e = encode(&b).expect_err("out of range");
            assert!(crate::error::is_user_error(&e), "case {i}");
        }
    }

    #[test]
    fn what_is_not_a_code_is_refused_in_plain_words() {
        for junk in ["", " ", "hello", "BABEL1", "babel1-AAAA", "BABEL1-", "BABEL1-A", "BABEL1-AAAA", "BABEL1-****", "BABEL1-AAA=", "<script>alert(1)</script>", "BABEL1-../../x", "BABEL-AAAA", "BABELx-AAAA"] {
            let m = message(decode(junk));
            assert!(m == BAD || m == DAMAGED, "{junk:?}: {m}");
            assert!(!m.contains("script"), "nothing of the input is repeated");
        }
        assert_eq!(message(decode("BABEL2-AAAAAAAA")), NEWER);
        assert_eq!(message(decode("BABEL17-xyz")), NEWER);
        let long = format!("{PREFIX}{}", "A".repeat(MAX_INPUT_CHARS));
        assert_eq!(message(decode(&long)), BAD);
        let longish = format!("{PREFIX}{}", "A".repeat(MAX_CODE_CHARS));
        assert_eq!(message(decode(&longish)), BAD);
    }

    #[test]
    fn a_changed_letter_or_a_cut_code_is_refused() {
        let code = encode(&sample()).unwrap();
        for i in PREFIX.len()..code.len() {
            let mut chars: Vec<char> = code.chars().collect();
            chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
            let changed: String = chars.into_iter().collect();
            message(decode(&changed));
            message(decode(&code[..i]));
        }
        message(decode(&format!("{code}A")));
        message(decode(&format!("{code}AAAA")));
    }

    /// A code with a right checksum around bits [`encode`] would not write: only the write-again check refuses these.
    #[test]
    fn a_code_written_another_way_is_refused() {
        let wrap = |bits: &BitWriter| {
            let mut bytes = crc16(&bits.bytes).to_be_bytes().to_vec();
            bytes.extend(&bits.bytes);
            format!("{PREFIX}{}", to_base64url(&bytes))
        };
        let head = |w: &mut BitWriter, level: u32, alignment: u32| {
            w.put(level, 7);
            w.put(alignment, 2);
            w.flag(false);
        };
        // level 0, level 127, alignment 3
        for (level, alignment) in [(0, 0), (127, 0), (50, 3)] {
            let mut w = BitWriter { bytes: Vec::new(), used: 0 };
            head(&mut w, level, alignment);
            w.put(0, 15);
            w.put(0, 6);
            message(decode(&wrap(&w)));
        }
        // a tarot of 5 written in 19 bits instead of 6
        let mut w = BitWriter { bytes: Vec::new(), used: 0 };
        head(&mut w, 50, 0);
        w.put(1 << 14, 15);
        w.put(100, 18);
        w.flag(true);
        w.flag(true);
        w.flag(true);
        w.put(3, 2);
        w.put(5, 19);
        w.flag(false);
        w.flag(false);
        w.flag(false);
        w.put(0, 6);
        message(decode(&wrap(&w)));
        // an empty build with a byte left over, and one with a bit set in the padding
        let mut w = BitWriter { bytes: Vec::new(), used: 0 };
        head(&mut w, 50, 0);
        w.put(0, 15);
        w.put(0, 6);
        let good = wrap(&w);
        assert!(decode(&good).is_ok());
        let mut more = BitWriter { bytes: w.bytes.clone(), used: w.used };
        more.put(0, 8 - (more.used % 8) as u32 % 8);
        more.put(0, 8);
        message(decode(&wrap(&more)));
        let mut padded = BitWriter { bytes: w.bytes.clone(), used: w.used };
        assert!(padded.used % 8 != 0);
        let last = padded.bytes.len() - 1;
        padded.bytes[last] |= 1;
        message(decode(&wrap(&padded)));
        // expertises not in rising order, and one with no points
        for points in [[(5u32, 2u32), (4, 2)], [(4, 2), (4, 2)], [(4, 0), (5, 1)]] {
            let mut w = BitWriter { bytes: Vec::new(), used: 0 };
            head(&mut w, 50, 0);
            w.put(0, 15);
            w.put(2, 6);
            for (id, ranks) in points {
                w.put(id, 7);
                w.flag(true);
                w.put(ranks, 7);
            }
            message(decode(&wrap(&w)));
        }
    }

    /// Whatever is pasted, the answer is a build or a plain message: never a panic, never a build that writes back as
    /// another code. Random text, random bytes behind the prefix, and real codes with bits flipped and the checksum
    /// made right again (so the reader itself is reached).
    #[test]
    fn garbage_never_panics_and_never_gets_through() {
        let mut r = Rng(0xD1B5_4A32_D192_ED03);
        let mut reached = 0;
        for round in 0..20000 {
            let text = match round % 4 {
                0 => (0..r.below(80)).map(|_| char::from_u32(r.below(0x250) as u32).unwrap_or('?')).collect::<String>(),
                1 => format!("{PREFIX}{}", (0..r.below(200)).map(|_| B64[r.below(64) as usize] as char).collect::<String>()),
                _ => {
                    // a real build's bits, damaged, under a fresh checksum
                    let code = encode(&random_build(&mut r)).unwrap();
                    let mut bytes = from_base64url(&code[PREFIX.len()..]).unwrap().split_off(2);
                    for _ in 0..=r.below(3) {
                        match r.below(3) {
                            0 if !bytes.is_empty() => {
                                let at = r.below(bytes.len() as u64) as usize;
                                bytes[at] ^= 1 << r.below(8);
                            }
                            1 => bytes.truncate(r.below(bytes.len() as u64 + 1) as usize),
                            _ => bytes.push(r.below(256) as u8),
                        }
                    }
                    let mut all = crc16(&bytes).to_be_bytes().to_vec();
                    all.extend(bytes);
                    format!("{PREFIX}{}", to_base64url(&all))
                }
            };
            match decode(&text) {
                Ok(b) => {
                    reached += 1;
                    assert!(check(&b).is_ok(), "{text}");
                    assert_eq!(encode(&b).unwrap(), tidy(&text), "a code that is read is the code that build writes");
                }
                Err(e) => assert!(crate::error::is_user_error(&e), "{text}: {e:?}"),
            }
        }
        // a flipped bit often still gives a (different) valid build: that is fine, and shows the reader was reached
        assert!(reached > 0);
    }
}
