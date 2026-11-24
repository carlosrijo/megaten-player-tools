//! Port of `src/Game/ClientReader.cs`: where the NewMoon client (32-bit ImagineClient) keeps things. All addresses are
//! heap addresses that change every session, so each structure is found by a signature, never a fixed address.
//!
//! The record types (Nameplate, GearGrid, WornItem, SetRecord, InfoWindow) are in `game::types`; `UserError` is in
//! `crate::error`.
//!
//! Notes on the port:
//!   * C# `FindClient` returned a `System.Diagnostics.Process`; here it is a [`ClientProcess`] (pid, exe path, folder,
//!     start time, and [`ClientProcess::main_window_handle`]).
//!   * C# `GearGrid.Tarot/Soul == null` ("the enchant ids could not be read, only the items are known") is an EMPTY
//!     `Vec` in `game::types::GearGrid` (a real grid always has 15 entries).

// The checks keep the C# shape (`x < 1 || x > 99`, index loops, nested ifs) so each line can be compared with the original.
#![allow(clippy::manual_range_contains, clippy::needless_range_loop, clippy::collapsible_if)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use indexmap::{IndexMap, IndexSet};

use super::types::{GearGrid, InfoWindow, Nameplate, SetRecord, WornItem};
use crate::memory::{GameMemory, find_bytes, find_i32_in_range, find_u32, i16_at, i32_at, u16_at, u32_at};
use crate::text::{eq_ic, ic_key};
use crate::{Result, user_error};

pub const STAT_NAMES: [&str; 12] =
    ["STR", "MAG", "VIT", "INT", "SPD", "LUCK", "CLS-R", "LNG-R", "Spell", "Support", "P.Def", "M.Def"];
pub const SLOT_NAMES: [&str; 15] = [
    "Weapon", "Bullets", "Ring", "Earring", "Talisman", "Head", "Face", "Top", "Neck", "Back", "Arms", "COMP", "Bottom", "Feet", "Extra",
];

/// Shift-JIS (code page 932) text from the game's memory, decoded the way .NET's code page 932 does (checked against
/// it for every one- and two-byte sequence): bytes that are no character become U+30FB (a lead byte takes the byte
/// after it along, a lone lead at the end stands alone) or, for 0xA0 and 0xFD-0xFF, U+F8F0-U+F8F3; `encoding_rs`
/// would give U+FFFD there. Everything valid is decoded by `encoding_rs`.
pub fn sjis(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            0x00..=0x80 => {
                out.push(b as char);
                i += 1;
            }
            0xA1..=0xDF => {
                out.push(char::from_u32(0xFF61 + (b - 0xA1) as u32).unwrap_or('\u{FFFD}'));
                i += 1;
            }
            0xA0 => {
                out.push('\u{F8F0}');
                i += 1;
            }
            0xFD..=0xFF => {
                out.push(char::from_u32(0xF8F1 + (b - 0xFD) as u32).unwrap_or('\u{FFFD}'));
                i += 1;
            }
            _ => {
                // a lead byte (0x81-0x9F, 0xE0-0xFC)
                if i + 1 < bytes.len() {
                    match encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(&bytes[i..i + 2]) {
                        Some(s) => out.push_str(&s),
                        None => out.push('\u{30FB}'),
                    }
                    i += 2;
                } else {
                    out.push('\u{30FB}');
                    i += 1;
                }
            }
        }
    }
    out
}


/// C# `Encoding.ASCII.GetBytes`: anything outside ASCII becomes '?'.
fn ascii_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().map(|u| if u < 0x80 { u as u8 } else { b'?' }).collect()
}

/// C# `DateTime.MinValue`.
pub fn min_date_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap()
}

// ---------- Running clients (the C# `Process` objects) ----------

/// What the callers used of a C# `System.Diagnostics.Process` for an ImagineClient: its id, exe, start time and
/// main window.
#[derive(Debug, Clone)]
pub struct ClientProcess {
    pub pid: u32,
    /// full path of ImagineClient.exe (None when it can't be read)
    pub exe_path: Option<PathBuf>,
    /// the install folder's name: the last part of the exe's directory ("" when unknown)
    pub folder: String,
    /// local start time ([`min_date_time`] when unknown)
    pub started: NaiveDateTime,
}

impl ClientProcess {
    /// C# `Process.Id`.
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// C# `Process.MainWindowHandle`: the first visible, unowned top-level window of the process, 0 when it has none.
    pub fn main_window_handle(&self) -> isize {
        main_window_of(self.pid)
    }
}

mod sys {
    use super::*;
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible};
    use windows::core::{BOOL, PWSTR};

    /// Pids of every running process called `name` (case-insensitive, with or without ".exe"), in system order.
    pub fn pids_named(name: &str) -> Vec<u32> {
        let mut res = Vec::new();
        // SAFETY: Toolhelp snapshot walk with a correctly sized entry; the handle is closed at the end.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return res };
            let mut e = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut ok = Process32FirstW(snap, &mut e).is_ok();
            while ok {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let exe = String::from_utf16_lossy(&e.szExeFile[..len]);
                let stem = if exe.len() > 4 && exe[exe.len() - 4..].eq_ignore_ascii_case(".exe") { &exe[..exe.len() - 4] } else { &exe[..] };
                if stem.eq_ignore_ascii_case(name) {
                    res.push(e.th32ProcessID);
                }
                ok = Process32NextW(snap, &mut e).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        res
    }

    /// (exe path, start time) of a process, each None when it can't be read. Opened with the limited query right
    /// (a subset of PROCESS_QUERY_INFORMATION): nothing of the process's memory is reachable through it.
    pub fn exe_and_start(pid: u32) -> (Option<PathBuf>, Option<NaiveDateTime>) {
        // SAFETY: plain Win32 calls with correctly sized buffers; the handle is closed.
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return (None, None) };
            let mut buf = vec![0u16; 32768];
            let mut n = buf.len() as u32;
            let exe = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut n)
                .ok()
                .map(|_| PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])));
            let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            let start = GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u).ok().and_then(|_| filetime_local(&c));
            let _ = CloseHandle(h);
            (exe, start)
        }
    }

    fn filetime_local(ft: &FILETIME) -> Option<NaiveDateTime> {
        let t = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
        let secs = (t / 10_000_000) as i64 - 11_644_473_600;
        let nanos = ((t % 10_000_000) * 100) as u32;
        let utc = DateTime::from_timestamp(secs, nanos)?;
        Some(utc.with_timezone(&Local).naive_local())
    }

    struct Finder {
        pid: u32,
        found: isize,
    }

    unsafe extern "system" fn each_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the address of the Finder `main_window_of` keeps alive for the whole EnumWindows call.
        let f = unsafe { &mut *(lparam.0 as *mut Finder) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == f.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let owned = unsafe { GetWindow(hwnd, GW_OWNER) }.map(|o| !o.0.is_null()).unwrap_or(false);
            if !owned {
                f.found = hwnd.0 as isize;
                return BOOL(0);
            }
        }
        BOOL(1)
    }

    pub fn main_window_of(pid: u32) -> isize {
        let mut f = Finder { pid, found: 0 };
        // SAFETY: the callback only touches `f`, which outlives the call.
        unsafe {
            let _ = EnumWindows(Some(each_window), LPARAM(&mut f as *mut Finder as *mut c_void as isize));
        }
        f.found
    }
}

use sys::main_window_of;

/// The exe of any running process by id (C# `Process.GetProcessById(pid).MainModule.FileName`); None when it can't be read.
pub fn process_exe_path(pid: u32) -> Option<PathBuf> {
    sys::exe_and_start(pid).0
}

/// Every running ImagineClient as the callers see it (C# `Process.GetProcessesByName("ImagineClient")`).
pub fn client_processes() -> Vec<ClientProcess> {
    sys::pids_named("ImagineClient")
        .into_iter()
        .map(|pid| {
            let (exe, start) = sys::exe_and_start(pid);
            // C#: Path.GetFileName(Path.GetDirectoryName(p.MainModule.FileName)) ?? ""
            let folder = exe
                .as_ref()
                .and_then(|e| e.parent())
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            ClientProcess { pid, exe_path: exe, folder, started: start.unwrap_or_else(min_date_time) }
        })
        .collect()
}

// ---------- Who is logged in ----------

/// The running client whose OWN character is `name` (several clients may be open).
/// Just finding the name is not enough: a client also holds status records of other players it can see, so with
/// two characters near each other each client contains both names. (That picked Rubito's client for "Rokujuu".)
/// The owner is decided by [`detect_character`]; the answer per client is remembered for a minute, since a snapshot
/// asks more than once.
pub fn find_client(name: &str) -> Result<ClientProcess> {
    let procs = client_processes();
    if procs.is_empty() {
        return Err(user_error("ImagineClient is not running."));
    }
    for p in procs {
        if owner_of(&p).is_some_and(|n| eq_ic(&n, name)) {
            return Ok(p);
        }
        // Inside share_sweeps, looking at another client kept a copy of its memory (~1-2 GB) that nothing else in
        // this scope will use: let it go now rather than at the end of the scope.
        crate::memory::release_sweep(p.pid);
    }
    Err(user_error(format!("No running client has '{name}' logged in.")))
}

type OwnerKey = (u32, NaiveDateTime);
/// (name, level, when looked up)
type OwnerAnswer = (Option<String>, i32, Instant);
static OWNERS: Mutex<Option<IndexMap<OwnerKey, OwnerAnswer>>> = Mutex::new(None);

fn with_owners<T>(f: impl FnOnce(&mut IndexMap<OwnerKey, OwnerAnswer>) -> T) -> T {
    let mut g = OWNERS.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(IndexMap::new))
}

/// Who is logged in to this client (None = nobody), looked up at most once a minute per client.
fn owner_of(p: &ClientProcess) -> Option<String> {
    owner(p).0
}

fn owner(p: &ClientProcess) -> (Option<String>, i32) {
    let key = (p.pid, p.started);
    let known = with_owners(|o| match o.get(&key) {
        Some((Some(n), lvl, at)) if at.elapsed() < Duration::from_secs(60) => Some((Some(n.clone()), *lvl)),
        _ => None,
    });
    if let Some(k) = known {
        return k;
    }
    let who = GameMemory::new(p.pid).ok().and_then(|mem| detect_character(&mem));
    let (name, level) = match who {
        Some((n, l)) => (Some(n), l),
        None => (None, 0),
    };
    with_owners(|o| {
        o.insert(key, (name.clone(), level, Instant::now()));
    });
    (name, level)
}

// ---------- Who is logged in: found without knowing the name ----------
// The client keeps many copies of status records shaped like the nameplate below: name (ASCII, zero-padded to 64
// bytes) at H-68, level int32 at H-4, then HP, MaxHP, MP, MaxMP. Seen 2026-11-22 in two clients at once:
//   own character   : int32 at name-8 = 0xEEA6, int32 at name-4 = 1   (Rubito x566, Rokujuu x86)
//   partner demon   : int32 at name-8 = 0x4291, int32 at name-4 = 0   (Alilat x70)
// Other players and NPCs did not have zero-padded records of this shape. So: the own character is the name with
// the most records marked 1; if no record is marked (a different client build), the most frequent name overall.
// Names with non-ASCII characters are not recognised.
pub fn detect_character(mem: &GameMemory) -> Option<(String, i32)> {
    let mut marked: IndexMap<String, (i32, i32)> = IndexMap::new();
    let mut any: IndexMap<String, (i32, i32)> = IndexMap::new();
    for (_, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        // the level int32 sits at H-4: the vectorised prefilter finds the words that are a level (1..=99)
        let mut at = 72usize;
        while let Some(q) = find_i32_in_range(buf, 1, 99, at) {
            at = q + 4;
            let h = q + 4;
            if h + 16 > buf.len() {
                break;
            }
            let lvl = i32_at(buf, h - 4);
            let cur = i32_at(buf, h);
            let max = i32_at(buf, h + 4);
            let mp = i32_at(buf, h + 8);
            let mmax = i32_at(buf, h + 12);
            if max < 1 || max > 30000 || cur < 0 || cur > max || mmax < 0 || mmax > 30000 || mp < 0 || mp > mmax {
                continue;
            }
            let n = h - 68;
            let mut len = 0usize;
            while len < 24 && buf[n + len] >= 0x21 && buf[n + len] < 0x7F {
                len += 1;
            }
            if len < 2 || len > 20 {
                continue;
            }
            if buf[n + len..h - 4].iter().any(|&b| b != 0) {
                continue;
            }
            let name: String = buf[n..n + len].iter().map(|&b| b as char).collect();
            let into = if i32_at(buf, n - 4) == 1 { &mut marked } else { &mut any };
            let seen = into.get(&name).copied().unwrap_or((0, 0));
            into.insert(name, (seen.0 + 1, seen.1.max(lvl)));
        }
    }
    let pool = if !marked.is_empty() { marked } else { any };
    // OrderByDescending(count) is stable: the first name wins a tie
    let mut best: Option<(&String, &(i32, i32))> = None;
    for kv in pool.iter().filter(|(_, v)| v.0 >= 3) {
        if best.is_none_or(|b| kv.1.0 > b.1.0) {
            best = Some(kv);
        }
    }
    best.map(|(k, v)| (k.clone(), v.1))
}

/// One running game client: which install folder it runs from and who is logged in (None = nobody yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientInfo {
    pub pid: u32,
    pub folder: String,
    pub started: NaiveDateTime,
    pub character: Option<String>,
    pub level: i32,
}

/// Every running client with its logged-in character. Takes about a second per client.
pub fn list_clients() -> Vec<ClientInfo> {
    list_clients_with(false)
}

/// [`list_clients`]; with `keep_known`, a client whose character was already found keeps that answer (however old)
/// and only the others are looked at again. For the page's background check while a client sits at the title
/// screen: it re-read every client, ~1 s of memory reading each, every half minute.
pub fn list_clients_with(keep_known: bool) -> Vec<ClientInfo> {
    let mut res = Vec::new();
    for p in client_processes() {
        let key = (p.pid, p.started);
        let known = if keep_known { with_owners(|o| o.get(&key).and_then(|(n, lvl, _)| n.clone().map(|n| (Some(n), *lvl)))) } else { None };
        let who = match known {
            Some(k) => k,
            None => {
                // A fresh look (logins change); it also refreshes what find_client remembers.
                with_owners(|o| {
                    o.shift_remove(&key);
                });
                owner(&p)
            }
        };
        res.push(ClientInfo { pid: p.pid, folder: p.folder.clone(), started: p.started, character: who.0, level: who.1 });
    }
    res.sort_by(|a, b| ic_key(&a.folder).cmp(&ic_key(&b.folder)).then(a.pid.cmp(&b.pid)));
    res
}

// ---------- Nameplate: always present while logged in ----------
// name (ASCII, NUL-terminated) at H-68, level int32 at H-4, then HP, MaxHP, MP, MaxMP int32 at H+0..H+12.
// HP/MP are NOT reliable (dozens of history copies); use it for the level.
pub fn read_nameplate(mem: &GameMemory, name: &str) -> Option<Nameplate> {
    let mut pat = ascii_bytes(name);
    pat.push(0);
    for (_, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while p + pat.len() <= buf.len() {
            let Some(k) = find_bytes(&buf[p..], &pat) else { break };
            let i = p + k;
            p = i + 1;
            let hp = i + 68;
            if hp + 16 > buf.len() || hp < 4 {
                continue;
            }
            let lvl = i32_at(buf, hp - 4);
            let cur = i32_at(buf, hp);
            let max = i32_at(buf, hp + 4);
            let mp = i32_at(buf, hp + 8);
            let mmax = i32_at(buf, hp + 12);
            if lvl < 1 || lvl > 99 || max < 1 || max > 30000 || cur < 0 || cur > max || mmax < 0 || mmax > 30000 || mp < 0 || mp > mmax {
                continue;
            }
            return Some(Nameplate { level: lvl, hp: cur, max_hp: max, mp, max_mp: mmax });
        }
    }
    None
}

// ---------- Information window ----------
// S = address of name + 76. Level int16 at S-40; HP, MaxHP, MP, MaxMP int32 at S-36..S-24;
// 12 stats as int32 (base, bonus) pairs at S+0..S+92 (StatNames order);
// 15 gear records of 2072 bytes from S+316: [itemId][int][0][itemId] then the tooltip text (Shift-JIS, NUL-terminated) at +16.
const REC_SIZE: usize = 2072;

pub fn read_info_window(mem: &GameMemory, name: &str, is_known_item: impl Fn(i32) -> bool) -> Option<InfoWindow> {
    let mut pat = ascii_bytes(name);
    pat.push(0);
    for name_at in mem.find_bytes(&pat, 2000) {
        let s = name_at + 76;
        let Some(b) = mem.read(s - 80, 80 + 96) else { continue };
        let lvl = i16_at(&b, 40) as i32;
        let hp = i32_at(&b, 44);
        let mhp = i32_at(&b, 48);
        let mp = i32_at(&b, 52);
        let mmp = i32_at(&b, 56);
        if !(1..=99).contains(&lvl) {
            continue;
        }
        if mhp < 1 || mhp > 30000 || hp < 0 || hp > mhp {
            continue;
        }
        if mmp < 0 || mmp > 30000 || mp < 0 || mp > mmp {
            continue;
        }
        let mut pairs = vec![0i32; 24];
        let mut ok = true;
        for (i, pr) in pairs.iter_mut().enumerate() {
            *pr = i32_at(&b, 80 + i * 4);
            if *pr < -2000 || *pr > 10000 {
                ok = false;
            }
        }
        if !ok {
            continue;
        }
        let Some(gear) = mem.read(s + 316, 15 * REC_SIZE) else { continue };
        let items: Vec<i32> = (0..15).map(|k| i32_at(&gear, k * REC_SIZE)).collect();
        if items[0] > 0 && !is_known_item(items[0]) {
            continue;
        }
        let mut texts = Vec::with_capacity(15);
        for k in 0..15 {
            if items[k] <= 0 {
                texts.push(String::new());
                continue;
            }
            let o = k * REC_SIZE + 16;
            let mut len = 0usize;
            while len < REC_SIZE - 16 && gear[o + len] != 0 {
                len += 1;
            }
            texts.push(sjis(&gear[o..o + len]));
        }
        return Some(InfoWindow { address: s, level: lvl, hp, max_hp: mhp, mp, max_mp: mmp, pairs, items, texts });
    }
    None
}

// ---------- Equipped gear grid: always present ----------
// 15 records of 56 bytes in slot order, 5 per row, rows 560 bytes apart.
// Record: +0 itemId, +8 object id, +12 0, +16 0x01??0001 (middle byte is an item state flag), +20 -1.0f, +36 ms timestamp,
// +46 tarot enchant id (int16), +48 soul enchant id (int16); 32767 = no enchant. Ids match enchants.json / EnchantSetData.
// Old grids linger after a gear change; the live one is the complete grid written most recently.
fn gear_sig(b: &[u8], o: i64) -> bool {
    o >= 0 && (o as usize) + 24 <= b.len() && {
        let o = o as usize;
        i32_at(b, o + 12) == 0 && (u32_at(b, o + 16) & 0xFF00FFFF) == 0x01000001 && u32_at(b, o + 20) == 0xBF800000
    }
}

fn gear_off(k: usize) -> usize {
    (k / 5) * 560 + (k % 5) * 56
}

/// An enchant id as stored (32767 or <= 0 = none).
fn ench(v: i16) -> i32 {
    if v <= 0 || v == 32767 { 0 } else { v as i32 }
}

pub fn read_gear_grids(mem: &GameMemory) -> Vec<GearGrid> {
    let mut res = Vec::new();
    for (r, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while p + gear_off(14) + 24 <= buf.len() {
            let pp = p as i64;
            p += 4;
            if !gear_sig(buf, pp) || !gear_sig(buf, pp + 56) || gear_sig(buf, pp - 56) {
                continue;
            }
            let base = pp as usize;
            let mut matched = 0;
            let mut ids = vec![0i32; 15];
            let mut tarot = vec![0i32; 15];
            let mut soul = vec![0i32; 15];
            let mut newest: i64 = 0;
            for k in 0..15 {
                let o = base + gear_off(k);
                if !gear_sig(buf, o as i64) || o + 50 > buf.len() {
                    continue;
                }
                matched += 1;
                ids[k] = i32_at(buf, o);
                tarot[k] = ench(i16_at(buf, o + 46));
                soul[k] = ench(i16_at(buf, o + 48));
                let ts = u32_at(buf, o + 36) as i64;
                if ts > newest {
                    newest = ts;
                }
            }
            if matched < 12 {
                continue;
            }
            res.push(GearGrid { address: r.base + pp, ids, tarot, soul, matched, newest_ms: newest, details: None });
            if res.len() >= 16 {
                return res;
            }
        }
    }
    res
}

/// What the logged-in character wears right now. The worn items come from the character's own equip map
/// ([`read_worn_items`]) and the tarot/soul enchant ids from each worn item's own record.
/// If those records can't be read, the gear grid that shows the same items supplies the enchant ids; when no grid
/// matches either (some clients have none at all), tarot and soul are EMPTY (C# null) and only the items are known.
/// If the equip map can't be found, falls back to the old rule: the complete grid written most recently.
pub fn live_gear(mem: &GameMemory) -> Option<GearGrid> {
    let equips = own_equipment_objects(mem);
    let worn = read_worn_items_in(mem, &equips);
    if let Some(w) = &worn {
        if let Some(details) = read_worn_details(mem, &equips, w) {
            return Some(GearGrid {
                address: 0,
                ids: w.clone(),
                tarot: details.iter().map(|d| d.as_ref().map_or(0, |d| d.tarot)).collect(),
                soul: details.iter().map(|d| d.as_ref().map_or(0, |d| d.soul)).collect(),
                matched: w.iter().filter(|&&i| i > 0).count() as i32,
                newest_ms: 0,
                details: Some(details),
            });
        }
    }
    let grids = read_gear_grids(mem);
    // OrderByDescending(NewestMs).FirstOrDefault(): the first of the newest
    let newest_first = |pred: &dyn Fn(&GearGrid) -> bool| -> Option<GearGrid> {
        let mut best: Option<&GearGrid> = None;
        for g in grids.iter().filter(|g| pred(g)) {
            if best.is_none_or(|b| g.newest_ms > b.newest_ms) {
                best = Some(g);
            }
        }
        best.cloned()
    };
    match worn {
        None => newest_first(&|g| g.matched == 15),
        Some(w) => newest_first(&|g| g.ids == w).or_else(|| {
            Some(GearGrid {
                address: 0,
                ids: w.clone(),
                tarot: Vec::new(),
                soul: Vec::new(),
                matched: w.iter().filter(|&&i| i > 0).count() as i32,
                newest_ms: 0,
                details: None,
            })
        }),
    }
}

// ---------- Worn items of the logged-in character: always present, empty slots included ----------
// Found 2026-11-22 (two clients, each seeing both characters). Class vtables are fixed addresses in this client
// build ("IMAGINE Version 1.670", image base 0x400000):
//   character record  [0x16CB8F4][1][flags][ptr][ptr2][-> equipment object][ptr3]...
//       the client's OWN character: flags 0x10F, ptr2 and ptr3 set.   Any other player: flags 5, ptr2 = ptr3 = 0.
//   equipment object  [0x16CB914][1][8 appearance bytes][2 words][map: worn items][x][map][x][map: looks]...
//       a map field is {head, size}; the worn-items map is at +24 (head) / +28 (size).
//   map nodes (old MSVC std::map): +0 left, +4 parent, +8 right, +12 key, +16 value, +21 "is head" byte;
//       head+4 is the root. Key = the game's equip type 0..14, value = item id. An empty slot has no node.
// Checked: Rubito's map equals his live gear grid; Rokujuu's (3 empty slots, no grid in that client) equals what
// the Information window showed.
const CHARACTER_VTABLE: u32 = 0x16CB8F4;
const EQUIPMENT_VTABLE: u32 = 0x16CB914;
/// game equip type -> this tool's slot order (SLOT_NAMES)
const SLOT_OF_EQUIP_TYPE: [usize; 15] = [5, 6, 8, 7, 10, 12, 13, 11, 2, 3, 14, 9, 4, 0, 1];

/// The game's equip type (0 Head ... 13 Weapon, 14 Bullets) of one of this tool's slots (-1 when not a slot).
pub fn equip_type_of_slot(slot: i32) -> i32 {
    SLOT_OF_EQUIP_TYPE.iter().position(|&s| s as i32 == slot).map_or(-1, |i| i as i32)
}

const STATUS_VTABLE: u32 = 0x16CB904;

/// The character's stats as the game holds them at all times (no window needed). C# `OwnStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnStatus {
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub pairs: Vec<i32>,
}

/// Level, HP/MP and the 12 stats (base, bonus) of the client's own character, from its status object (the same class
/// as demons'): own character record +12 -> status (vtable 0x16CB904): +132 level (one byte: the 3 above it can hold
/// leftover bytes, e.g. Rokujuu 0x1927D661 = level 97), +136 MAX HP, +140 CURRENT HP, +144 MAX MP (u16), +146
/// CURRENT MP (u16) (the demons' status objects have them the other way round: HP, max HP, MP, max MP); one 4-byte
/// entry per CorrectTbl index from +176: +2 base, +4 bonus (int16), so STR..LUCK
/// are indexes 0-5 and CLS-R..Support 12-15, P.Def 19, M.Def 20. Checked against the open Information window (all 24
/// numbers and HP/MP equal, at full HP/MP). None when unreadable or when two own records disagree.
///
/// The order of the HP/MP words was found 2026-11-24 with HP and MP below the maximum: Rubito's HUD showed HP 589/709
/// and MP 0/264 while +136 = 709, +140 = 589 (counting down with the HP), +144 = 264, +146 = 0. It was read the other
/// way round before, which only worked at full HP and MP (both words equal); with either below the maximum the record
/// was thrown away and a snapshot fell back to the saved stats. The digitalized case that led to `digitalized` was the
/// same thing (Rubito + Shikigami: +136 = 1317 = the game's Max HP, +140 = 1134 = his current HP, not a "stale max
/// HP"). `_digitalized` is therefore unused; it stays so the callers and the C# signature are unchanged.
pub fn read_own_status(mem: &GameMemory, digitalized: bool) -> Option<OwnStatus> {
    let mut res: Option<OwnStatus> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, CHARACTER_VTABLE, p) {
            p = pp + 4;
            if pp + 16 > buf.len() {
                break;
            }
            if (u32_at(buf, pp + 8) & 0x100) == 0 {
                continue;
            }
            let st = u32_at(buf, pp + 12);
            let s = if st > 0x10000 { mem.read(st as i64, 264) } else { None };
            let Some(s) = s else { continue };
            if u32_at(&s, 0) != STATUS_VTABLE {
                continue;
            }
            let Some(o) = own_status_of(&s, digitalized) else { continue };
            if let Some(prev) = &res {
                if prev.pairs != o.pairs || prev.max_hp != o.max_hp {
                    return None;
                }
            }
            res = Some(o);
        }
    }
    res
}

/// The numbers [`read_own_status`] takes from one own-character status object (`s`: its first 264 bytes, vtable already
/// checked), or None when they can't be a live status.
fn own_status_of(s: &[u8], _digitalized: bool) -> Option<OwnStatus> {
    const IDX: [usize; 12] = [0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 19, 20]; // STAT_NAMES order
    let mut pairs = vec![0i32; 24];
    for i in 0..12 {
        pairs[i * 2] = i16_at(s, 178 + IDX[i] * 4) as i32;
        pairs[i * 2 + 1] = i16_at(s, 180 + IDX[i] * 4) as i32;
    }
    let lvl = s[132] as i32;
    // The own character's words are max first, current second (the demons' are the other way round): +136 max HP,
    // +140 current HP, +144 max MP, +146 current MP. See [`read_own_status`].
    let mhp = i32_at(s, 136);
    let hp = i32_at(s, 140);
    let mmp = u16_at(s, 144) as i32;
    let mp = u16_at(s, 146) as i32;
    if lvl < 1 || lvl > 99 || mhp < 1 || hp < 0 {
        return None;
    }
    // The current values never really exceed the maximum; when the maximum has just dropped (a toggle switched on) the
    // current word can lag for a moment, which is no reason to drop the record.
    Some(OwnStatus { level: lvl, hp: hp.min(mhp), max_hp: mhp, mp: mp.min(mmp), max_mp: mmp, pairs })
}

/// The client's own character's alignment points (LNC), or None when unreadable: own character record +12 -> status
/// object (vtable 0x16CB904) +106 (int16). The same field in demons' status objects equals their LNC in the game
/// data (checked on 7 demons: Alilat -9600, Specter 5000, Poltergeist 5600, ...). Rubito: -4025, shown as Neutral.
pub fn read_lnc(mem: &GameMemory) -> Option<i32> {
    let mut res: Option<i32> = None;
    for (r, buf) in mem.scan() {
        if r.base < 0x02000000 {
            continue;
        }
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, CHARACTER_VTABLE, p) {
            p = pp + 4;
            if pp + 16 > buf.len() {
                break;
            }
            if (u32_at(buf, pp + 8) & 0x100) == 0 {
                continue;
            }
            let st = u32_at(buf, pp + 12);
            let s = if st > 0x10000 { mem.read(st as i64, 108) } else { None };
            let Some(s) = s else { continue };
            if u32_at(&s, 0) != STATUS_VTABLE {
                continue;
            }
            let v = i16_at(&s, 106) as i32;
            if res.is_some_and(|x| x != v) {
                return None; // two own records disagree: don't guess
            }
            res = Some(v);
        }
    }
    res
}

/// LAW / NEUTRAL / CHAOS from alignment points (server rule ActiveEntityState::CalculateLNCType).
pub fn lnc_name(lnc: i32) -> &'static str {
    if lnc >= 5000 {
        "CHAOS"
    } else if lnc <= -5000 {
        "LAW"
    } else {
        "NEUTRAL"
    }
}

/// The equipment objects of the client's own character (normally one).
fn own_equipment_objects(mem: &GameMemory) -> IndexSet<i64> {
    let mut own = IndexSet::new();
    for (_, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_u32(buf, CHARACTER_VTABLE, p) {
            p = pp + 4;
            if pp + 28 > buf.len() {
                break;
            }
            if i32_at(buf, pp + 4) != 1 {
                continue;
            }
            let flags = u32_at(buf, pp + 8);
            let ptr2 = u32_at(buf, pp + 16);
            let equip = u32_at(buf, pp + 20);
            if (flags & 0x100) == 0 || ptr2 == 0 || equip < 0x10000 {
                continue; // not the client's own character
            }
            own.insert(equip as i64);
        }
    }
    own
}

/// The 15 worn item ids in SLOT_NAMES order (0 = empty slot), or None when the structure isn't found.
pub fn read_worn_items(mem: &GameMemory) -> Option<Vec<i32>> {
    read_worn_items_in(mem, &own_equipment_objects(mem))
}

fn read_worn_items_in(mem: &GameMemory, own: &IndexSet<i64>) -> Option<Vec<i32>> {
    let mut result: Option<Vec<i32>> = None;
    for &equip in own {
        let Some(e) = mem.read(equip, 32) else { continue };
        if u32_at(&e, 0) != EQUIPMENT_VTABLE || i32_at(&e, 4) != 1 {
            continue;
        }
        let head = u32_at(&e, 24) as i64;
        let size = i32_at(&e, 28);
        if !(0..=15).contains(&size) {
            continue;
        }
        let Some(items) = walk_equip_map(mem, head, size) else { continue };
        if result.as_ref().is_some_and(|r| *r != items) {
            return None; // two different answers: don't guess
        }
        result = Some(items);
    }
    result
}

fn walk_equip_map(mem: &GameMemory, head: i64, size: i32) -> Option<Vec<i32>> {
    let hb = mem.read(head, 24)?;
    if hb[21] == 0 {
        return None;
    }
    let mut items = vec![0i32; 15];
    let mut found = 0;
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n == 0 || n == head || !seen.insert(n) {
            continue;
        }
        let nb = mem.read(n, 24);
        let nb = nb.filter(|_| seen.len() <= 16)?;
        if nb[21] != 0 {
            continue;
        }
        let key = i32_at(&nb, 12);
        let value = i32_at(&nb, 16);
        if key < 0 || key > 14 || value <= 0 || items[SLOT_OF_EQUIP_TYPE[key as usize]] != 0 {
            return None;
        }
        items[SLOT_OF_EQUIP_TYPE[key as usize]] = value;
        found += 1;
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if found == size { Some(items) } else { None }
}

// ---------- Tarot / soul enchant ids of the worn items: always present ----------
// Found 2026-11-23, after a gear change left the 56-byte gear grid showing two old items (state byte 01) while the
// new ones were worn, so no grid matched and enchant ids went missing.
//   equipment object +60 / +64: a fourth map {head, size} of the worn items' object ids (std::map<int, int64>: the
//       pair is 8-aligned, so +12 and +20 are padding). Node: +0 left, +4 parent, +8 right, +16 key = equip type
//       0..14, +24 that item's object id (int64; 0 = a slot that was emptied; never-used empty slots have no node),
//       +32 color (0 red, 1 black), +33 "is head" (1 only in the head node).
//       Fixed 2026-11-24: +20 was first read as an embedded item object's vtable (0x158EE68), but that address is code
//       (a jmp) and the word is padding left from the stack: nodes inserted later held the equipment object's address
//       there instead (Rubito, 4 of 15 nodes), so the walk gave up. The walk now checks the node's own color/head bytes.
//   item record (found by item id + object id; two copies, they agree from +20 on): the fields of comp_hack's
//       objects::Item (libcomp schema item.xml) in order:
//       +0 item id, +4 object id, +8 0, +12 flags (0x00000001 or 0x01010001),
//       +17 durability as shown (only in the 0x01010001 copy), +20 max durability,
//       +26 tarot enchant id (int16), +28 soul enchant id (int16); 32767 = none,
//       +30 five modification slot effect ids (int16; 0 = no slot, 255 = open slot),
//       +44 basic effect item id (int32; the item whose stats it carries; -1 = none),
//       +48 special effect item id (int32; -1 = none), +52 three fusion great-success bonuses (int8).
// Checked on Rubito: all 15 slots equal the enchants named in the item tooltips, including the two the grid had wrong;
// and on Rokujuu (6 empty slots, no gear grid in that client). Sources, mods and fuse bonuses checked against Rubito's
// tooltips (Spiral Ring License = Christmas Citrine Ring+5's stats; Levi Blaster fuse 50/50/50 = +45 LNG-R/Spell/Support).
/// The full record of each worn item, in SLOT_NAMES order (None for empty slots), or None when any worn slot can't
/// be read unambiguously.
fn read_worn_details(mem: &GameMemory, own: &IndexSet<i64>, worn: &[i32]) -> Option<Vec<Option<WornItem>>> {
    if own.len() != 1 {
        return None;
    }
    let e = mem.read(*own.first()?, 68)?;
    let head = u32_at(&e, 60) as i64;
    let size = i32_at(&e, 64);
    if head < 0x10000 || !(0..=15).contains(&size) {
        return None;
    }
    let hb = mem.read(head, 34)?;
    if hb[33] != 1 {
        return None; // not a map head
    }
    let mut obj_of: std::collections::HashMap<(i32, u32), usize> = std::collections::HashMap::new(); // (item id, object id) -> slot
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while let Some(n) = stack.pop() {
        if n == 0 || n == head || !seen.insert(n) {
            continue;
        }
        let nb = mem.read(n, 34)?;
        // a real node: not the head, red or black, an object id that fits the item record's 32-bit field
        if seen.len() > 16 || nb[33] != 0 || nb[32] > 1 || u32_at(&nb, 28) != 0 {
            return None;
        }
        let key = i32_at(&nb, 16);
        let obj = u32_at(&nb, 24);
        if key < 0 || key > 14 {
            return None;
        }
        let slot = SLOT_OF_EQUIP_TYPE[key as usize];
        if (obj == 0) != (worn[slot] <= 0) {
            return None; // object 0 = slot emptied
        }
        if obj != 0 {
            obj_of.insert((worn[slot], obj), slot);
        }
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if seen.len() as i32 != size || obj_of.len() != worn.iter().filter(|&&i| i > 0).count() {
        return None;
    }
    let src = |v: i32| if v <= 0 { 0 } else { v };
    let mut res: Vec<Option<WornItem>> = vec![None; 15];
    // Every record wanted starts with a worn item's id (+0 is the key's item id), so a vectorised range check on the
    // ids skips almost all of memory; the exact id test then drops the in-range strangers. Same hits, same order, as
    // the old step-by-4 loop over every offset (~0.45 s of a snapshot).
    let ids: Vec<i32> = obj_of.keys().map(|&(id, _)| id).collect();
    let (lo, hi) = (ids.iter().copied().min().unwrap_or(1), ids.iter().copied().max().unwrap_or(0));
    for (_, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        let mut p = 0usize;
        while let Some(pp) = find_i32_in_range(buf, lo, hi, p) {
            p = pp + 4;
            if pp + 56 > buf.len() {
                break;
            }
            if !ids.contains(&i32_at(buf, pp)) || i32_at(buf, pp + 8) != 0 {
                continue;
            }
            let flags = u32_at(buf, pp + 12);
            if flags != 0x0000_0001 && flags != 0x0101_0001 {
                continue;
            }
            let Some(&k) = obj_of.get(&(i32_at(buf, pp), u32_at(buf, pp + 4))) else { continue };
            let mut w = WornItem {
                id: worn[k],
                tarot: ench(i16_at(buf, pp + 26)),
                soul: ench(i16_at(buf, pp + 28)),
                basic: src(i32_at(buf, pp + 44)),
                special: src(i32_at(buf, pp + 48)),
                mods: (0..5).map(|i| u16_at(buf, pp + 30 + i * 2) as i32).collect(),
                fuse: (0..3).map(|i| buf[pp + 52 + i] as i8 as i32).collect(),
                durability: if flags == 0x0101_0001 { buf[pp + 17] as i32 } else { -1 },
                max_durability: buf[pp + 20] as i8 as i32,
            };
            if let Some(old) = &res[k] {
                // Copies must agree on everything but durability (only one copy has it): otherwise don't guess.
                if old.tarot != w.tarot || old.soul != w.soul || old.basic != w.basic || old.special != w.special || old.mods != w.mods || old.fuse != w.fuse {
                    return None;
                }
                if w.durability < 0 {
                    w.durability = old.durability;
                }
            }
            res[k] = Some(w);
        }
    }
    for k in 0..15 {
        if worn[k] > 0 && res[k].is_none() {
            return None;
        }
    }
    Some(res)
}

// ---------- Expertise tree: always present ----------
// Binary tree nodes: +0 left, +4 parent, +8 right, +12 key, +16 id (== key), +20 points (10000 per rank), +28 class, +32 rank.
// A 0-point node has the signature (k, k, 0, 256+k) at +12. Find one, climb to the head (key not a valid id), walk from head+4.
pub fn read_expertise_points(mem: &GameMemory) -> IndexMap<i32, i32> {
    for (r, buf) in mem.scan() {
        let buf: &[u8] = &buf;
        let mut p = 12usize;
        while p + 16 <= buf.len() {
            let pp = p;
            p += 4;
            let k = i32_at(buf, pp);
            if !(1..=37).contains(&k) {
                continue;
            }
            if i32_at(buf, pp + 4) != k || i32_at(buf, pp + 8) != 0 || i32_at(buf, pp + 12) != 256 + k {
                continue;
            }
            if let Some(result) = walk_expertise(mem, r.base + pp as i64 - 12) {
                return result;
            }
        }
    }
    IndexMap::new()
}

fn walk_expertise(mem: &GameMemory, node: i64) -> Option<IndexMap<i32, i32>> {
    let mut cur = node;
    let mut head = 0i64;
    for _ in 0..64 {
        let b = mem.read(cur, 20)?;
        let key = i32_at(&b, 12);
        let id = i32_at(&b, 16);
        if key < 0 || key > 63 || key != id {
            head = cur;
            break;
        }
        cur = u32_at(&b, 4) as i64;
    }
    if head == 0 {
        return None;
    }
    let hb = mem.read(head, 12)?;
    let mut res: IndexMap<i32, i32> = IndexMap::new();
    let mut seen: IndexSet<i64> = IndexSet::new();
    let mut stack: Vec<i64> = vec![u32_at(&hb, 4) as i64];
    while !stack.is_empty() && res.len() < 128 {
        let n = stack.pop().unwrap_or(0);
        if n == 0 || n == head || !seen.insert(n) {
            continue;
        }
        let nb = mem.read(n, 36)?;
        let key = i32_at(&nb, 12);
        let id = i32_at(&nb, 16);
        let pts = i32_at(&nb, 20);
        if key != id || key < 0 || key > 63 || pts < 0 || pts > 2_000_000 {
            return None;
        }
        res.insert(key, pts);
        stack.push(u32_at(&nb, 0) as i64);
        stack.push(u32_at(&nb, 8) as i64);
    }
    if res.len() >= 30 { Some(res) } else { None }
}

// ---------- Equipment-set table: static game data ----------
// int32 count, then 80-byte records: [setId][15 piece item ids, slot order][4 tokusei ids].
// Two-or-more-piece entries are sets; one-piece entries are the item's appearance bonus.
// Anchor: set 2021 (Resolute) = 2021, 0, 0, 0, 48002, 37286, 24359.
pub fn read_set_table(mem: &GameMemory) -> Option<Vec<SetRecord>> {
    let hit = mem.find_seq(&[2021, 0, 0, 0, 48002, 37286, 24359], 4, 1);
    if hit.is_empty() {
        return None;
    }
    const SPAN: i64 = 80 * 3000;
    let buf = mem.read(hit[0] - SPAN, (SPAN * 2) as usize)?;
    let at = |i: i64| -> i32 { i32_at(&buf, (SPAN + i * 80) as usize) };
    let mut lo: i64 = 0;
    while SPAN + (lo - 1) * 80 >= 0 && at(lo - 1) > 0 && at(lo - 1) < at(lo) {
        lo -= 1;
    }
    let count = i32_at(&buf, (SPAN + lo * 80 - 4) as usize) as i64;
    if count <= 0 || SPAN + (lo + count) * 80 > buf.len() as i64 {
        return None;
    }
    let mut sets = Vec::with_capacity(count as usize);
    for i in lo..lo + count {
        let o = (SPAN + i * 80) as usize;
        let pieces: Vec<i32> = (0..15).map(|k| i32_at(&buf, o + 4 + k * 4)).collect();
        let mut tok = Vec::new();
        for k in 0..4 {
            let t = i32_at(&buf, o + 64 + k * 4);
            if t != 0 {
                tok.push(t);
            }
        }
        sets.push(SetRecord { id: i32_at(&buf, o), pieces, tokusei: tok });
    }
    Some(sets)
}

#[cfg(test)]
mod own_status_tests {
    use super::*;

    /// Rubito's 12 stats (base, bonus) from the 2026-11-24 02:22 snapshot, in STAT_NAMES order.
    const PAIRS: [i32; 24] = [1, 109, 1, 65, 1, 66, 82, 66, 99, 156, 53, 33, 64, 71, 182, 362, 87, 141, 128, 45, 67, 7, 74, 16];

    /// A status object as the game holds it: the words at +136 / +140 / +144 / +146 as they sit in memory.
    fn status(level: u8, w136: i32, w140: i32, w144: u16, w146: u16) -> Vec<u8> {
        const IDX: [usize; 12] = [0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 19, 20];
        let mut s = vec![0u8; 264];
        s[0..4].copy_from_slice(&STATUS_VTABLE.to_le_bytes());
        s[132] = level;
        s[136..140].copy_from_slice(&w136.to_le_bytes());
        s[140..144].copy_from_slice(&w140.to_le_bytes());
        s[144..146].copy_from_slice(&w144.to_le_bytes());
        s[146..148].copy_from_slice(&w146.to_le_bytes());
        for i in 0..12 {
            s[178 + IDX[i] * 4..180 + IDX[i] * 4].copy_from_slice(&(PAIRS[i * 2] as i16).to_le_bytes());
            s[180 + IDX[i] * 4..182 + IDX[i] * 4].copy_from_slice(&(PAIRS[i * 2 + 1] as i16).to_le_bytes());
        }
        s
    }

    /// Rubito on 2026-11-24 (client1, not digitalized, Alilat summoned, a few MP-draining toggles on): the game's own
    /// HUD showed HP 589/709 and MP 0/264 while the status object held +136 = 709, +140 = 589 (it counted down 60
    /// every ~2.8 s: 709, 649, 589, ...), +144 = 264, +146 = 0. So +136 is the MAX HP and +140 the CURRENT HP, +144 the
    /// max MP and +146 the current MP. The reader took them the other way round and threw the record away whenever
    /// HP or MP was below the maximum, so a snapshot fell back to the saved stats (and the page said "digitalized").
    #[test]
    fn damaged_character_with_no_mp_is_read() {
        for digitalized in [false, true] {
            let o = own_status_of(&status(95, 709, 589, 264, 0), digitalized).expect("a live status, not a rejected one");
            assert_eq!((o.level, o.hp, o.max_hp, o.mp, o.max_mp), (95, 589, 709, 0, 264));
            assert_eq!(o.pairs, PAIRS.to_vec());
        }
    }

    /// Rubito + Shikigami digitalized (2026-11-23): +136 held 1317 and +140 1134; the game's Max HP was 1317 (checked
    /// against the digitalize formula), so 1134 is the current HP, not a "stale max HP". Same answer digitalized or not.
    #[test]
    fn digitalized_hp_below_the_maximum() {
        for digitalized in [false, true] {
            let o = own_status_of(&status(95, 1317, 1134, 453, 453), digitalized).expect("readable");
            assert_eq!((o.hp, o.max_hp, o.mp, o.max_mp), (1134, 1317, 453, 453));
        }
    }

    /// The 02:22 snapshot, taken at full HP and MP: both words equal.
    #[test]
    fn full_hp_and_mp() {
        let o = own_status_of(&status(95, 1063, 1063, 264, 264), false).expect("readable");
        assert_eq!((o.hp, o.max_hp, o.mp, o.max_mp), (1063, 1063, 264, 264));
    }

    /// Current values never go above the maximum: when the maximum has just dropped (a toggle switched on) the
    /// current word can lag for a moment; that is not a reason to throw the record away.
    #[test]
    fn current_above_the_maximum_is_clamped() {
        let o = own_status_of(&status(95, 709, 1063, 264, 300), false).expect("readable");
        assert_eq!((o.hp, o.max_hp, o.mp, o.max_mp), (709, 709, 264, 264));
    }

    #[test]
    fn garbage_is_still_rejected() {
        assert!(own_status_of(&status(0, 709, 589, 264, 0), false).is_none(), "level 0");
        assert!(own_status_of(&status(100, 709, 589, 264, 0), false).is_none(), "level 100");
        assert!(own_status_of(&status(95, 0, 0, 264, 0), false).is_none(), "no max HP");
        assert!(own_status_of(&status(95, 709, -1, 264, 0), false).is_none(), "negative HP");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_mapping_roundtrips() {
        for slot in 0..15 {
            let t = equip_type_of_slot(slot);
            assert_eq!(SLOT_OF_EQUIP_TYPE[t as usize] as i32, slot);
        }
        assert_eq!(equip_type_of_slot(99), -1);
        assert_eq!(equip_type_of_slot(0), 13); // Weapon
    }

    #[test]
    fn lnc_names() {
        assert_eq!(lnc_name(5000), "CHAOS");
        assert_eq!(lnc_name(-5000), "LAW");
        assert_eq!(lnc_name(-4025), "NEUTRAL");
    }

    #[test]
    fn gear_sig_bounds() {
        let b = vec![0u8; 10];
        assert!(!gear_sig(&b, -56));
        assert!(!gear_sig(&b, 0));
    }

    #[test]
    fn ascii_bytes_like_dotnet() {
        assert_eq!(ascii_bytes("Ab\u{3042}"), vec![b'A', b'b', b'?']);
    }
}

#[cfg(test)]
mod sjis_tests {
    use super::sjis;

    #[test]
    fn sjis_decodes_like_dotnet_932() {
        assert_eq!(sjis(b"Ab"), "Ab");
        assert_eq!(sjis(&[0x82, 0xA0]), "\u{3042}"); // hiragana a
        assert_eq!(sjis(&[0xB1]), "\u{FF71}"); // half-width katakana
        assert_eq!(sjis(&[0x81]), "\u{30FB}"); // lone lead byte
        assert_eq!(sjis(&[0x81, 0x00]), "\u{30FB}"); // invalid trail takes the lead's partner along
        assert_eq!(sjis(&[0xA0, 0x41]), "\u{F8F0}A");
        assert_eq!(sjis(&[0xFD, 0xFF]), "\u{F8F1}\u{F8F3}");
    }
}
