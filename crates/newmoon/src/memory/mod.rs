//! Port of `src/Memory/GameMemory.cs`: read-only access to another process's memory.
//!
//! The handle is opened with PROCESS_QUERY_INFORMATION | PROCESS_VM_READ only, so nothing can be written or changed.
//! (Hard project rule: never write to the game, never ask for more rights.)
//!
//! The game is a 32-bit process read from this 64-bit program, so addresses are `i64` (always below 4 GB) and the
//! region walk stops at 0xFFFF0000 like the C#.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, OnceLock};

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQueryEx};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

use crate::Result;

const MEM_COMMIT: u32 = 0x1000;
const PAGE_GUARD: u32 = 0x100;
const PAGE_NOACCESS: u32 = 0x01;
const MAX_REGION: i64 = 512 * 1024 * 1024;

/// One committed, readable region of the 32-bit address space (C# `Region(long Base, long Size)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub base: i64,
    pub size: i64,
}

/// The bytes of one region as a sweep yields them (shared, so a kept sweep and its readers hold the same copy).
pub type Bytes = Arc<Vec<u8>>;

/// Little-endian readers over a byte buffer (C# `BitConverter.ToInt32(buf, o)` and friends). Like the C#, an offset
/// outside the buffer is a bug of the caller (it panics here, it threw there).
#[inline(always)]
pub fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}
#[inline(always)]
pub fn i32_at(b: &[u8], o: usize) -> i32 {
    u32_at(b, o) as i32
}
#[inline(always)]
pub fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().expect("2 bytes"))
}
#[inline(always)]
pub fn i16_at(b: &[u8], o: usize) -> i16 {
    u16_at(b, o) as i16
}
#[inline(always)]
pub fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o + 8].try_into().expect("8 bytes"))
}

/// The first offset `p >= from` (a multiple of 4, like the C# readers' `p += 4` loops) where the little-endian u32 at
/// `p` equals `val`, with `p + 4 <= buf.len()`. A vectorised version of "scan the buffer in 4-byte steps for a
/// vtable pointer": the object searches are all led by one.
pub fn find_u32(buf: &[u8], val: u32, from: usize) -> Option<usize> {
    let n = buf.len();
    let mut p = from;
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::*;
        let ptr = buf.as_ptr();
        // SAFETY: SSE2 is part of the x86_64 baseline; every 16-byte load stays inside `buf` (p + 64 <= n / p + 16 <= n).
        unsafe {
            let nd = _mm_set1_epi32(val as i32);
            while p + 64 <= n {
                let a = _mm_cmpeq_epi32(_mm_loadu_si128(ptr.add(p) as *const __m128i), nd);
                let b = _mm_cmpeq_epi32(_mm_loadu_si128(ptr.add(p + 16) as *const __m128i), nd);
                let c = _mm_cmpeq_epi32(_mm_loadu_si128(ptr.add(p + 32) as *const __m128i), nd);
                let d = _mm_cmpeq_epi32(_mm_loadu_si128(ptr.add(p + 48) as *const __m128i), nd);
                if _mm_movemask_epi8(_mm_or_si128(_mm_or_si128(a, b), _mm_or_si128(c, d))) != 0 {
                    break;
                }
                p += 64;
            }
            while p + 16 <= n {
                let m = _mm_movemask_epi8(_mm_cmpeq_epi32(_mm_loadu_si128(ptr.add(p) as *const __m128i), nd));
                if m != 0 {
                    return Some(p + (m as u32).trailing_zeros() as usize);
                }
                p += 16;
            }
        }
    }
    while p + 4 <= n {
        if u32_at(buf, p) == val {
            return Some(p);
        }
        p += 4;
    }
    None
}

/// The first offset `p >= from` (a multiple of 4) where the little-endian i32 at `p` lies in `lo..=hi`, with
/// `p + 4 <= buf.len()` (a vectorised prefilter for 4-byte-step scans that start with a range check).
pub fn find_i32_in_range(buf: &[u8], lo: i32, hi: i32, from: usize) -> Option<usize> {
    let n = buf.len();
    let mut p = from;
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::*;
        let ptr = buf.as_ptr();
        // SAFETY: SSE2 is part of the x86_64 baseline; every 16-byte load stays inside `buf` (p + 16 <= n).
        unsafe {
            let vlo = _mm_set1_epi32(lo.wrapping_sub(1));
            let vhi = _mm_set1_epi32(hi.wrapping_add(1));
            while p + 16 <= n {
                let v = _mm_loadu_si128(ptr.add(p) as *const __m128i);
                let m = _mm_movemask_epi8(_mm_and_si128(_mm_cmpgt_epi32(v, vlo), _mm_cmplt_epi32(v, vhi)));
                if m != 0 {
                    return Some(p + (m as u32).trailing_zeros() as usize);
                }
                p += 16;
            }
        }
    }
    while p + 4 <= n {
        let v = i32_at(buf, p);
        if v >= lo && v <= hi {
            return Some(p);
        }
        p += 4;
    }
    None
}

/// First index of `needle` in `hay` (a vectorised `memchr`).
pub fn find_byte(needle: u8, hay: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::*;
        let n = hay.len();
        let ptr = hay.as_ptr();
        let mut i = 0usize;
        // SAFETY: SSE2 is part of the x86_64 baseline; every load stays inside `hay` (i + 64 <= n / i + 16 <= n).
        unsafe {
            let nd = _mm_set1_epi8(needle as i8);
            while i + 64 <= n {
                let a = _mm_cmpeq_epi8(_mm_loadu_si128(ptr.add(i) as *const __m128i), nd);
                let b = _mm_cmpeq_epi8(_mm_loadu_si128(ptr.add(i + 16) as *const __m128i), nd);
                let c = _mm_cmpeq_epi8(_mm_loadu_si128(ptr.add(i + 32) as *const __m128i), nd);
                let d = _mm_cmpeq_epi8(_mm_loadu_si128(ptr.add(i + 48) as *const __m128i), nd);
                if _mm_movemask_epi8(_mm_or_si128(_mm_or_si128(a, b), _mm_or_si128(c, d))) != 0 {
                    break;
                }
                i += 64;
            }
            while i + 16 <= n {
                let m = _mm_movemask_epi8(_mm_cmpeq_epi8(_mm_loadu_si128(ptr.add(i) as *const __m128i), nd));
                if m != 0 {
                    return Some(i + (m as u32).trailing_zeros() as usize);
                }
                i += 16;
            }
        }
        while i < n {
            if hay[i] == needle {
                return Some(i);
            }
            i += 1;
        }
        None
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        hay.iter().position(|&x| x == needle)
    }
}

/// First index of `pat` in `hay` (C# `span.IndexOf(pat)`; an empty pattern is found at 0).
pub fn find_bytes(hay: &[u8], pat: &[u8]) -> Option<usize> {
    if pat.is_empty() {
        return Some(0);
    }
    if pat.len() > hay.len() {
        return None;
    }
    let last_start = hay.len() - pat.len();
    let first = pat[0];
    let mut p = 0usize;
    while p <= last_start {
        let k = find_byte(first, &hay[p..=last_start])?;
        let at = p + k;
        if &hay[at..at + pat.len()] == pat {
            return Some(at);
        }
        p = at + 1;
    }
    None
}

// ---------- One sweep shared by several searches ----------
// A snapshot searches the whole game about a dozen times (status, gear, demons, progress, expertise...), each
// copying ~1 GB of the client's memory; the copying was a third of a snapshot's time. Inside `share_sweeps` the copy is
// made once and every search walks the same bytes, so they also all see the same moment. Pointers found in it are
// still followed with `read`, live. The kept copy (~1 GB) is let go when the scope ends.
// (C# kept this in an AsyncLocal, i.e. per logical flow; here it is per thread.)
#[derive(Default)]
struct SharedState {
    sweeps: HashMap<u32, Vec<(Region, Bytes)>>,
    results: HashMap<(u32, String), Box<dyn Any>>,
}

thread_local! {
    static SHARED: RefCell<Option<SharedState>> = const { RefCell::new(None) };
}

/// The scope [`share_sweeps`] returns: dropping it ends the sharing (unless it joined an outer scope).
pub struct SweepScope {
    owner: bool,
}

impl Drop for SweepScope {
    fn drop(&mut self) {
        if self.owner {
            SHARED.with(|s| *s.borrow_mut() = None);
        }
    }
}

/// Until the returned scope is dropped, sweeps of a client are made once and shared (see above), and
/// [`GameMemory::remember`] answers repeat questions. Nested scopes join the outer one.
/// C# `using var shared = GameMemory.ShareSweeps();`
pub fn share_sweeps() -> SweepScope {
    let owner = SHARED.with(|s| {
        let mut s = s.borrow_mut();
        if s.is_some() {
            false
        } else {
            *s = Some(SharedState::default());
            true
        }
    });
    SweepScope { owner }
}

/// Inside [`share_sweeps`]: drops the kept copy of one client's memory (the next sweep of it reads it again).
/// Nothing outside a scope.
pub fn release_sweep(pid: u32) {
    SHARED.with(|s| {
        if let Some(st) = s.borrow_mut().as_mut() {
            st.sweeps.remove(&pid);
        }
    });
}

/// Read-only handle to a game process (C# `GameMemory`).
pub struct GameMemory {
    /// the process handle as a plain integer (keeps the type `Send`)
    handle: isize,
    regions: OnceLock<Vec<Region>>,
    pub pid: u32,
}

impl GameMemory {
    /// Opens the process with PROCESS_QUERY_INFORMATION | PROCESS_VM_READ only. Fails with the Win32 error.
    pub fn new(pid: u32) -> Result<GameMemory> {
        // SAFETY: plain OpenProcess; the access mask is the project's read-only pair, nothing else is ever requested.
        let h = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
            .map_err(|e| anyhow::anyhow!("OpenProcess({pid}) failed: {e}"))?;
        Ok(GameMemory { handle: h.0 as isize, regions: OnceLock::new(), pid })
    }

    fn h(&self) -> HANDLE {
        HANDLE(self.handle as *mut c_void)
    }

    /// Committed, readable regions of the 32-bit address space (cached per instance).
    pub fn regions(&self) -> &[Region] {
        self.regions.get_or_init(|| {
            let mut list = Vec::new();
            let mut addr: i64 = 0;
            let max: i64 = 0xFFFF_0000;
            while addr < max {
                let mut m = MEMORY_BASIC_INFORMATION::default();
                // SAFETY: `m` is a valid out buffer of the size passed.
                let got = unsafe {
                    VirtualQueryEx(self.h(), Some(addr as usize as *const c_void), &mut m, size_of::<MEMORY_BASIC_INFORMATION>())
                };
                if got == 0 {
                    break;
                }
                let b = m.BaseAddress as usize as i64;
                let s = m.RegionSize as i64;
                if s <= 0 {
                    break;
                }
                if m.State.0 == MEM_COMMIT && (m.Protect.0 & PAGE_GUARD) == 0 && (m.Protect.0 & PAGE_NOACCESS) == 0 {
                    list.push(Region { base: b, size: s });
                }
                addr = b + s;
            }
            list
        })
    }

    /// `len` bytes at `addr`, or None when they can't all be read.
    pub fn read(&self, addr: i64, len: usize) -> Option<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(len);
        let mut got: usize = 0;
        // SAFETY: the buffer has `len` bytes of capacity; ReadProcessMemory only writes into it and reports how many.
        // The memory is not read before it has been written (set_len only after a full read).
        let ok = unsafe {
            ReadProcessMemory(self.h(), addr as usize as *const c_void, buf.as_mut_ptr() as *mut c_void, len, Some(&mut got))
        };
        if ok.is_err() || got != len {
            return None;
        }
        // SAFETY: `got == len` bytes were written.
        unsafe { buf.set_len(len) };
        Some(buf)
    }

    /// Every readable region's bytes, one region at a time (skips huge ones). Inside [`share_sweeps`] the first full
    /// sweep of a client is kept and later sweeps of that client reuse it.
    pub fn scan(&self) -> Scan<'_> {
        let shared = SHARED.with(|s| s.borrow().is_some());
        let kept = if shared { SHARED.with(|s| s.borrow().as_ref().and_then(|st| st.sweeps.get(&self.pid).cloned())) } else { None };
        match kept {
            Some(k) => Scan { mem: self, source: ScanSource::Kept(k.into_iter()), done: false },
            None => Scan {
                mem: self,
                source: ScanSource::Live { next: 0, keep: if shared { Some(Vec::new()) } else { None } },
                done: false,
            },
        }
    }

    /// Inside [`share_sweeps`]: the answer a reader already gave for this client (same key) instead of searching
    /// again. Outside it, just runs the reader. (The C# handed back the same object; here it is a clone.)
    pub fn remember<T: Clone + 'static>(&self, key: &str, read: impl FnOnce() -> T) -> T {
        let known = SHARED.with(|s| {
            let s = s.borrow();
            let st = s.as_ref()?;
            let v = st.results.get(&(self.pid, key.to_string()))?;
            v.downcast_ref::<T>().cloned()
        });
        if let Some(k) = known {
            return k;
        }
        let in_scope = SHARED.with(|s| s.borrow().is_some());
        let v = read();
        if in_scope {
            SHARED.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    st.results.insert((self.pid, key.to_string()), Box::new(v.clone()));
                }
            });
        }
        v
    }

    /// Addresses of `pat` in the game's memory (at most `max_hits`).
    pub fn find_bytes(&self, pat: &[u8], max_hits: usize) -> Vec<i64> {
        let mut hits = Vec::new();
        for (r, buf) in self.scan() {
            let buf: &[u8] = &buf;
            let mut p = 0usize;
            while p + pat.len() <= buf.len() {
                let Some(k) = find_bytes(&buf[p..], pat) else { break };
                hits.push(r.base + (p + k) as i64);
                if hits.len() >= max_hits {
                    return hits;
                }
                p += k + 1;
            }
        }
        hits
    }

    /// Contiguous values stored as 2- or 4-byte little-endian integers (2-byte steps).
    pub fn find_seq(&self, values: &[i32], elem_size: usize, max_hits: usize) -> Vec<i64> {
        let mut hits = Vec::new();
        let span = values.len() * elem_size;
        for (r, buf) in self.scan() {
            let buf: &[u8] = &buf;
            let mut i = 0usize;
            while i + span <= buf.len() {
                let mut ok = true;
                for (k, &want) in values.iter().enumerate() {
                    let v = if elem_size == 2 { i16_at(buf, i + k * 2) as i32 } else { i32_at(buf, i + k * 4) };
                    if v != want {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    hits.push(r.base + i as i64);
                    if hits.len() >= max_hits {
                        return hits;
                    }
                }
                i += 2;
            }
        }
        hits
    }

    /// Places where at least `min_distinct` different ids from a set sit within `window` bytes (research helper).
    pub fn find_id_clusters(&self, ids: &[i32], window: usize, min_distinct: usize, elem_size: usize) -> Vec<(i64, usize)> {
        let set: std::collections::HashSet<i32> = ids.iter().copied().collect();
        let mut res = Vec::new();
        let val = |buf: &[u8], i: usize| -> i32 { if elem_size == 2 { u16_at(buf, i) as i32 } else { i32_at(buf, i) } };
        for (r, buf) in self.scan() {
            let buf: &[u8] = &buf;
            let mut last_reported: i64 = -1;
            let mut i = 0usize;
            while i + elem_size <= buf.len() {
                let v = val(buf, i);
                if !set.contains(&v) {
                    i += elem_size;
                    continue;
                }
                if last_reported >= 0 && r.base + (i as i64) < last_reported + window as i64 {
                    i += elem_size;
                    continue;
                }
                let mut seen = std::collections::HashSet::new();
                let end = (buf.len() + 1 - elem_size).min(i + window);
                let mut j = i;
                while j < end {
                    let w = val(buf, j);
                    if set.contains(&w) {
                        seen.insert(w);
                    }
                    j += elem_size;
                }
                if seen.len() >= min_distinct {
                    res.push((r.base + i as i64, seen.len()));
                    last_reported = r.base + i as i64;
                    if res.len() > 200 {
                        return res;
                    }
                }
                i += elem_size;
            }
        }
        res
    }

    pub fn i32(&self, b: &[u8], o: usize) -> i32 {
        i32_at(b, o)
    }

    pub fn u32(&self, b: &[u8], o: usize) -> u32 {
        u32_at(b, o)
    }
}

impl Drop for GameMemory {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcess in `new` and is closed once.
        unsafe {
            let _ = CloseHandle(self.h());
        }
    }
}

enum ScanSource {
    Kept(std::vec::IntoIter<(Region, Bytes)>),
    Live { next: usize, keep: Option<Vec<(Region, Bytes)>> },
}

/// The sweep [`GameMemory::scan`] returns: an iterator of `(region, its bytes)`.
pub struct Scan<'a> {
    mem: &'a GameMemory,
    source: ScanSource,
    done: bool,
}

impl Iterator for Scan<'_> {
    type Item = (Region, Bytes);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match &mut self.source {
            ScanSource::Kept(it) => {
                let x = it.next();
                if x.is_none() {
                    self.done = true;
                }
                x
            }
            ScanSource::Live { next, keep } => {
                let regions = self.mem.regions();
                while *next < regions.len() {
                    let r = regions[*next];
                    *next += 1;
                    if r.size > MAX_REGION {
                        continue;
                    }
                    let Some(buf) = self.mem.read(r.base, r.size as usize) else { continue };
                    let buf: Bytes = Arc::new(buf);
                    if let Some(k) = keep.as_mut() {
                        k.push((r, buf.clone()));
                    }
                    return Some((r, buf));
                }
                self.done = true;
                // Only a sweep that went all the way through is kept (a search may stop at its first hit).
                if let Some(k) = keep.take() {
                    let pid = self.mem.pid;
                    SHARED.with(|s| {
                        if let Some(st) = s.borrow_mut().as_mut() {
                            st.sweeps.insert(pid, k);
                        }
                    });
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bytes_like_index_of() {
        let hay: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
        assert_eq!(find_bytes(&hay, &[5, 6, 7]), Some(5));
        assert_eq!(find_bytes(&hay, &[250, 0, 1]), Some(250));
        assert_eq!(find_bytes(&hay, &[9, 9]), None);
        assert_eq!(find_bytes(&hay, &[]), Some(0));
        assert_eq!(find_byte(200, &hay), Some(200));
        let mut big = vec![0u8; 1000];
        big[999] = 7;
        assert_eq!(find_byte(7, &big), Some(999));
        assert_eq!(find_bytes(&big, &[0, 7]), Some(998));
    }

    #[test]
    fn share_scope_nests() {
        let a = share_sweeps();
        {
            let _b = share_sweeps();
        }
        assert!(SHARED.with(|s| s.borrow().is_some()));
        drop(a);
        assert!(SHARED.with(|s| s.borrow().is_none()));
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn find_u32_steps_by_four() {
        let mut b = vec![0u8; 200];
        b[100..104].copy_from_slice(&0x16CB8F4u32.to_le_bytes());
        b[142..146].copy_from_slice(&0x16CB8F4u32.to_le_bytes()); // not a multiple of 4: never reported
        assert_eq!(find_u32(&b, 0x16CB8F4, 0), Some(100));
        assert_eq!(find_u32(&b, 0x16CB8F4, 104), None);
        assert_eq!(find_u32(&b[..103], 0x16CB8F4, 0), None);
        let mut c = vec![0u8; 8];
        c[4..8].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(find_u32(&c, 7, 0), Some(4));
    }

    #[test]
    fn find_i32_in_range_matches_the_scalar_rule() {
        let mut b = vec![0u8; 400];
        for (i, v) in [(8usize, 100i32), (40, 0), (60, 99), (120, 1), (200, -5)] {
            b[i..i + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert_eq!(find_i32_in_range(&b, 1, 99, 0), Some(60));
        assert_eq!(find_i32_in_range(&b, 1, 99, 64), Some(120));
        assert_eq!(find_i32_in_range(&b, 1, 99, 124), None);
        assert_eq!(find_i32_in_range(&b, -10, -1, 0), Some(200));
    }

    #[test]
    fn own_process_opens_read_only_and_reads_nothing_at_null() {
        let mem = GameMemory::new(std::process::id()).expect("own process opens with the read-only rights");
        assert!(mem.read(0, 16).is_none());
        let _ = mem.regions();
    }
}
