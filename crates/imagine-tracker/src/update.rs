//! Updates: BABEL looks on its GitHub releases page for a newer version, and on the player's say-so downloads it,
//! checks it and puts it in place of the running exe, then starts again.
//!
//! - Where: the releases of the public repo (`REPO`), read with GitHub's API (no account needed; GitHub allows 60
//!   looks an hour from one address, and the page looks once at start and every hour while open).
//! - What is trusted: only an exe whose signature checks with the public key built in here (`PUBLIC_KEY`). The
//!   release workflow signs `BABEL.exe <version> <sha-256 of the exe>` with the private key, kept only as the repo's
//!   Actions secret BABEL_UPDATE_KEY (README "Updates"), and publishes the signature as BABEL.exe.sig. The version is
//!   in what is signed, so an older signed exe can't be passed off as a newer one; only a version above the running
//!   one is ever installed. The exe's SHA-256 must also match BABEL.exe.sha256 and the size GitHub lists.
//! - How it is swapped: Windows lets a running exe be renamed, not overwritten. The download goes to `<exe>.new`
//!   beside the exe; then `<exe>` becomes `<exe>.old` and `<exe>.new` becomes `<exe>`. The window closes, and the
//!   new exe is started with `--wait-pid <old pid> --updated-from <old version>`: it waits for the old process to end
//!   (they share the WebView2 profile), deletes `<exe>.old` and says on the page that it was updated. The data folder
//!   beside the exe is not touched.
//! - Only published builds update themselves: an exe stamped with a release version (tools\publish-portable.ps1
//!   -Version, build.rs sets BABEL_PUBLISHED_BUILD). A local `cargo build` says so and never looks.
//!
//! Network and crypto are Windows' own (WinHTTP with the system's proxy and certificates; CNG for ECDSA P-256), so
//! no new library is built in.
//!
//! Test builds: BABEL_UPDATE_FEED (a releases list in GitHub's JSON shape, may be http://127.0.0.1) and
//! BABEL_UPDATE_PUBKEY (hex X||Y) set at build time replace the real ones. Never set them for a release.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ui::Ui;

const REPO: &str = "carlosrijo/megaten-player-tools";
/// The release signing key's public half (P-256, X then Y, hex). Its private half: .secrets\babel-update-signing-key.txt
/// on the maintainer's PC and the repo's Actions secret BABEL_UPDATE_KEY.
const PUBLIC_KEY: &str = "7be9974af7c532819adf7cb6d83a50a97f148e5a7d73de14d706a0ac1e70d09a39474a1a53a7e36f7fed2cf3b10da8914cde3d8effb4e417512231a4da984a97";
/// A look is kept this long before asking GitHub again (unless the player presses Check now).
const KEEP: Duration = Duration::from_secs(30 * 60);
const EXE_MAX: u64 = 300 * 1024 * 1024;

fn test_feed() -> Option<&'static str> {
    option_env!("BABEL_UPDATE_FEED").filter(|s| !s.is_empty())
}
fn feed_url() -> String {
    test_feed().map(str::to_string).unwrap_or_else(|| format!("https://api.github.com/repos/{REPO}/releases?per_page=20"))
}
fn public_key() -> &'static str {
    option_env!("BABEL_UPDATE_PUBKEY").filter(|s| !s.is_empty()).unwrap_or(PUBLIC_KEY)
}
/// Stamped with a release version by tools\publish-portable.ps1 (build.rs).
pub fn published_build() -> bool {
    env!("BABEL_PUBLISHED_BUILD") == "1"
}
fn running_version() -> String {
    imagine::diag::about::build_info().version.clone()
}

// ---------- versions ----------

/// "0.10.0", "v0.10.0-beta.1": three numbers and an optional pre-release part (semver order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    nums: [u64; 3],
    pre: Vec<String>,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.trim().trim_start_matches(['v', 'V']);
        let s = s.split('+').next()?; // build metadata does not count
        let (core, pre) = match s.split_once('-') {
            Some((c, p)) if !p.is_empty() => (c, p.split('.').map(str::to_string).collect()),
            Some(_) => return None,
            None => (s, Vec::new()),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let mut nums = [0u64; 3];
        for (i, p) in parts.iter().enumerate() {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            nums[i] = p.parse().ok()?;
        }
        Some(Version { nums, pre })
    }
    pub fn is_pre(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        self.nums.cmp(&o.nums).then_with(|| match (self.pre.is_empty(), o.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater, // 1.0.0 > 1.0.0-beta
            (false, true) => Ordering::Less,
            (false, false) => {
                for (a, b) in self.pre.iter().zip(&o.pre) {
                    let c = match (a.parse::<u64>(), b.parse::<u64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        _ => a.cmp(b),
                    };
                    if c != Ordering::Equal {
                        return c;
                    }
                }
                self.pre.len().cmp(&o.pre.len())
            }
        })
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

// ---------- what GitHub lists ----------

#[derive(Debug, Clone)]
struct Asset {
    url: String,
    size: u64,
}

#[derive(Debug, Clone)]
pub struct Release {
    version: Version,
    /// the version as the tag has it, without the "v" (what was signed)
    name: String,
    tag: String,
    notes: String,
    exe: Option<Asset>,
    sha: Option<Asset>,
    sig: Option<Asset>,
}

impl Release {
    /// The release's own page (made here from the tag, never taken from the answer: it is opened in the browser).
    fn page(&self) -> String {
        format!("https://github.com/{REPO}/releases/tag/{}", self.tag)
    }
    fn installable(&self) -> bool {
        self.exe.is_some() && self.sha.is_some() && self.sig.is_some()
    }
    fn to_json(&self) -> Value {
        json!({
            "version": self.name,
            "pre": self.version.is_pre(),
            "notes": self.notes,
            "installable": self.installable(),
            "size": self.exe.as_ref().map(|a| a.size),
        })
    }
}

/// A download address the updater will use: the project's own release files on GitHub (a test build: its own feed's host).
fn allowed_asset_url(url: &str) -> bool {
    if let Some(feed) = test_feed() {
        let base = feed.split('/').take(3).collect::<Vec<_>>().join("/") + "/";
        if url.starts_with(&base) {
            return true;
        }
    }
    url.starts_with(&format!("https://github.com/{REPO}/releases/download/"))
}

/// The tag as it may appear in an address: letters, digits and . - + only.
fn safe_tag(t: &str) -> bool {
    !t.is_empty() && t.len() <= 64 && t.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}

/// The release notes players read: the part above the SHA-256 block the workflow adds.
fn notes_of(body: &str) -> String {
    let body = body.replace("\r\n", "\n");
    let cut = body.find("### Check the download").unwrap_or(body.len());
    let mut s = body[..cut].trim().to_string();
    if s.chars().count() > 6000 {
        s = s.chars().take(6000).collect::<String>() + "…";
    }
    s
}

/// The newest release above `current` from GitHub's list (pre-releases only when asked for).
fn pick(list: &Value, current: &Version, pre: bool) -> Option<Release> {
    let mut best: Option<Release> = None;
    for r in list.as_array()?.iter() {
        if r.get("draft").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let tag = r.get("tag_name").and_then(Value::as_str).unwrap_or_default();
        let Some(version) = Version::parse(tag) else { continue };
        if !safe_tag(tag) || (version.is_pre() && !pre) || version <= *current {
            continue;
        }
        if best.as_ref().is_some_and(|b| b.version >= version) {
            continue;
        }
        let asset = |name: &str| -> Option<Asset> {
            let a = r.get("assets")?.as_array()?.iter().find(|a| a.get("name").and_then(Value::as_str) == Some(name))?;
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            let size = a.get("size")?.as_u64()?;
            allowed_asset_url(&url).then_some(Asset { url, size })
        };
        best = Some(Release {
            name: tag.trim_start_matches(['v', 'V']).to_string(),
            tag: tag.to_string(),
            notes: notes_of(r.get("body").and_then(Value::as_str).unwrap_or_default()),
            exe: asset("BABEL.exe").filter(|a| a.size > 0 && a.size <= EXE_MAX),
            sha: asset("BABEL.exe.sha256"),
            sig: asset("BABEL.exe.sig"),
            version,
        });
    }
    best
}

// ---------- the checks a download must pass ----------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// What the release workflow signs for one exe.
fn signed_text(version: &str, sha256_hex: &str) -> String {
    format!("BABEL.exe {version} {sha256_hex}")
}

/// An ECDSA P-256 signature (r||s, 64 bytes) of SHA-256(`message`), checked with the built-in public key by Windows (CNG).
fn signature_ok(message: &[u8], sig: &[u8], key_hex: &str) -> bool {
    use windows::Win32::Security::Cryptography::*;
    let Some(key) = unhex(key_hex).filter(|k| k.len() == 64) else { return false };
    if sig.len() != 64 {
        return false;
    }
    let mut blob = Vec::with_capacity(8 + 64);
    blob.extend_from_slice(&BCRYPT_ECDSA_PUBLIC_P256_MAGIC.to_le_bytes());
    blob.extend_from_slice(&32u32.to_le_bytes());
    blob.extend_from_slice(&key);
    let hash = Sha256::digest(message);
    unsafe {
        let mut alg = BCRYPT_ALG_HANDLE::default();
        if BCryptOpenAlgorithmProvider(&mut alg, BCRYPT_ECDSA_P256_ALGORITHM, windows::core::PCWSTR::null(), BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0)).is_err() {
            return false;
        }
        let mut k = BCRYPT_KEY_HANDLE::default();
        let ok = BCryptImportKeyPair(alg, None, BCRYPT_ECCPUBLIC_BLOB, &mut k, &blob, 0).is_ok()
            && BCryptVerifySignature(k, None, &hash, sig, BCRYPT_FLAGS(0)).is_ok();
        if !k.is_invalid() {
            let _ = BCryptDestroyKey(k);
        }
        let _ = BCryptCloseAlgorithmProvider(alg, 0);
        ok
    }
}

/// Everything a downloaded exe must pass before it replaces the running one.
fn verify(rel: &Release, exe: &[u8], sha_file: &[u8], sig_file: &[u8]) -> Result<()> {
    let size = rel.exe.as_ref().map(|a| a.size).unwrap_or(0);
    if exe.len() as u64 != size {
        bail!("The download is {} bytes, but the release lists {size}.", exe.len());
    }
    let got = hex(&Sha256::digest(exe));
    let listed = String::from_utf8_lossy(sha_file).split_whitespace().next().unwrap_or_default().to_ascii_lowercase();
    if listed != got {
        bail!("The download's SHA-256 does not match the release's.");
    }
    use base64::Engine;
    let sig = base64::engine::general_purpose::STANDARD
        .decode(String::from_utf8_lossy(sig_file).trim())
        .map_err(|_| anyhow!("The release's signature file can't be read."))?;
    if !signature_ok(signed_text(&rel.name, &got).as_bytes(), &sig, public_key()) {
        bail!("The download's signature does not check out, so it was not installed.");
    }
    Ok(())
}

// ---------- the updater the page talks to ----------

#[derive(Debug, Clone)]
enum Phase {
    Idle,
    Downloading { done: u64, total: u64 },
    Checking,
    Restarting,
    Failed(String),
}

struct Look {
    at: Instant,
    pre: bool,
    when: String,
    found: Result<Option<Release>, String>,
}

struct Inner {
    last: Option<Look>,
    phase: Phase,
}

pub struct Updater {
    inner: Mutex<Inner>,
    /// why this run never updates (a local build, a scripted run), or None
    off: Option<&'static str>,
}

/// Set once the new exe is in place: started with these arguments when the window has closed (main.rs).
static RESTART: Mutex<Option<(PathBuf, Vec<String>)>> = Mutex::new(None);

impl Updater {
    pub fn new(scripted: bool) -> Updater {
        let off = if scripted {
            Some("A scripted run doesn't look for updates.")
        } else if !published_build() {
            Some("This is a local build, not a published one: it doesn't update itself.")
        } else {
            None
        };
        Updater { inner: Mutex::new(Inner { last: None, phase: Phase::Idle }), off }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// `updateCheck`: is there a newer version? Asks GitHub at most every half hour unless `force`.
    pub fn check(&self, pre: bool, force: bool) -> Value {
        let current = running_version();
        if let Some(why) = self.off {
            return json!({ "enabled": false, "reason": why, "current": current });
        }
        let cached = {
            let g = self.lock();
            g.last.as_ref().filter(|l| !force && l.pre == pre && l.at.elapsed() < KEEP).map(|l| (l.when.clone(), l.found.clone()))
        };
        let (when, found) = match cached {
            Some(c) => c,
            None => {
                let found = look(pre);
                match &found {
                    Ok(Some(r)) => imagine::log_info!("update check: {} is out (running {current})", r.name),
                    Ok(None) => imagine::log_info!("update check: {current} is the newest"),
                    Err(e) => imagine::log_warn!("update check failed: {e}"),
                }
                let when = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
                self.lock().last = Some(Look { at: Instant::now(), pre, when: when.clone(), found: found.clone() });
                (when, found)
            }
        };
        let mut out = json!({ "enabled": true, "current": current, "checkedAt": when, "testFeed": test_feed().is_some() });
        match found {
            Ok(r) => out["update"] = r.map(|r| r.to_json()).unwrap_or(Value::Null),
            Err(e) => out["error"] = json!(e),
        }
        out
    }

    /// `updateStatus`: how an install is going.
    pub fn status(&self) -> Value {
        match &self.lock().phase {
            Phase::Idle => json!({ "phase": "idle" }),
            Phase::Downloading { done, total } => json!({ "phase": "downloading", "done": done, "total": total }),
            Phase::Checking => json!({ "phase": "checking" }),
            Phase::Restarting => json!({ "phase": "restarting" }),
            Phase::Failed(e) => json!({ "phase": "failed", "error": e }),
        }
    }

    /// The release page of what the last look found (else the releases list), for "Release page".
    pub fn page(&self) -> String {
        let g = self.lock();
        match g.last.as_ref().and_then(|l| l.found.as_ref().ok().cloned().flatten()) {
            Some(r) => r.page(),
            None => format!("https://github.com/{REPO}/releases"),
        }
    }

    /// The release page of the running version (after an update: "What's new"), else the releases list.
    pub fn current_page(&self) -> String {
        let tag = format!("v{}", running_version());
        if published_build() && safe_tag(&tag) {
            format!("https://github.com/{REPO}/releases/tag/{tag}")
        } else {
            format!("https://github.com/{REPO}/releases")
        }
    }

    /// `updateInstall`: download `version` (the one the page showed; it must still be what the last look found),
    /// check it, put it in place and close the window, which starts it.
    pub fn install(&self, version: &str, ui: &Ui, args: &[String]) -> Result<Value> {
        if let Some(why) = self.off {
            return Err(imagine::user_error(why));
        }
        let rel = {
            let mut g = self.lock();
            if matches!(g.phase, Phase::Downloading { .. } | Phase::Checking | Phase::Restarting) {
                return Err(imagine::user_error("The update is already under way."));
            }
            let rel = g.last.as_ref().and_then(|l| l.found.as_ref().ok().cloned().flatten()).filter(|r| r.name == version);
            let Some(rel) = rel else { return Err(imagine::user_error("That version is no longer the newest one. Check again.")) };
            if !rel.installable() {
                return Err(imagine::user_error("This release can't be installed from inside BABEL. Download it from its release page."));
            }
            g.phase = Phase::Downloading { done: 0, total: rel.exe.as_ref().map(|a| a.size).unwrap_or(0) };
            rel
        };
        match self.download_and_swap(&rel) {
            Ok(exe) => {
                imagine::log_info!("update: {} is in place, starting it", rel.name);
                let mut next: Vec<String> = Vec::new();
                // the run's own options carry over (--data, --log-level ...); the update ones are new each time
                let mut i = 0;
                while i < args.len() {
                    if matches!(args[i].as_str(), "--wait-pid" | "--updated-from") {
                        i += 2;
                        continue;
                    }
                    next.push(args[i].clone());
                    i += 1;
                }
                next.extend(["--wait-pid".to_string(), std::process::id().to_string(), "--updated-from".to_string(), running_version()]);
                *RESTART.lock().unwrap_or_else(|p| p.into_inner()) = Some((exe, next));
                self.lock().phase = Phase::Restarting;
                ui.close();
                Ok(json!({ "restarting": true }))
            }
            Err(e) => {
                imagine::log_warn!("update to {} failed: {e:#}", rel.name);
                let msg = format!("{e:#}");
                self.lock().phase = Phase::Failed(msg.clone());
                Err(imagine::user_error(msg))
            }
        }
    }

    fn download_and_swap(&self, rel: &Release) -> Result<PathBuf> {
        let exe = std::env::current_exe().context("Where BABEL runs from is not known")?;
        let new = with_suffix(&exe, ".new");
        let old = with_suffix(&exe, ".old");
        // can BABEL write in its own folder at all (a copy under Program Files can't)?
        std::fs::write(&new, b"").map_err(|e| {
            anyhow!("BABEL can't write in its folder ({}): {e}. Download the new version from its release page instead.", exe.parent().map(|p| p.display().to_string()).unwrap_or_default())
        })?;
        let a = rel.exe.as_ref().expect("installable");
        let bytes = http::get(&a.url, "application/octet-stream", EXE_MAX, &mut |done, total| {
            self.lock().phase = Phase::Downloading { done, total: total.unwrap_or(a.size) };
        })
        .context("The download stopped")?;
        self.lock().phase = Phase::Checking;
        let sha = http::get(&rel.sha.as_ref().expect("installable").url, "application/octet-stream", 4096, &mut |_, _| {}).context("The checksum couldn't be downloaded")?;
        let sig = http::get(&rel.sig.as_ref().expect("installable").url, "application/octet-stream", 4096, &mut |_, _| {}).context("The signature couldn't be downloaded")?;
        if let Err(e) = verify(rel, &bytes, &sha, &sig) {
            let _ = std::fs::remove_file(&new);
            return Err(e);
        }
        std::fs::write(&new, &bytes).context("The new version couldn't be saved beside BABEL")?;
        // the swap: the running exe can be renamed (not replaced); put it back if the new one can't take its name
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).context("The running BABEL couldn't be moved aside")?;
        if let Err(e) = std::fs::rename(&new, &exe) {
            let _ = std::fs::rename(&old, &exe);
            let _ = std::fs::remove_file(&new);
            return Err(anyhow!(e).context("The new version couldn't be put in place"));
        }
        Ok(exe)
    }
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// One look at the releases list.
fn look(pre: bool) -> Result<Option<Release>, String> {
    let current = Version::parse(&running_version()).ok_or_else(|| "This build's version can't be compared.".to_string())?;
    let body = http::get(&feed_url(), "application/vnd.github+json", 4 * 1024 * 1024, &mut |_, _| {}).map_err(|e| {
        let m = format!("{e:#}");
        if m.contains("HTTP 403") || m.contains("HTTP 429") {
            "GitHub turned the check away for now (too many from this network). It tries again later.".to_string()
        } else {
            format!("Couldn't reach GitHub: {m}")
        }
    })?;
    let list: Value = serde_json::from_slice(&body).map_err(|_| "GitHub's answer couldn't be read.".to_string())?;
    Ok(pick(&list, &current, pre))
}

// ---------- start and end of a run ----------

/// At start: after an update, wait for the old process to end (`--wait-pid`), then delete what the swap left beside
/// the exe. Errors are fine: the next start tries again.
pub fn after_start(wait_pid: Option<u32>) {
    if let Some(pid) = wait_pid {
        wait_for_exit(pid, Duration::from_secs(20));
    }
    let Ok(exe) = std::env::current_exe() else { return };
    for suffix in [".old", ".new"] {
        let p = with_suffix(&exe, suffix);
        if p.is_file() {
            let _ = std::fs::remove_file(&p);
        }
    }
}

fn wait_for_exit(pid: u32, max: Duration) {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let r = WaitForSingleObject(h, max.as_millis() as u32);
            let _ = CloseHandle(h);
            if r != WAIT_OBJECT_0 {
                imagine::log_warn!("update: the previous BABEL was still running after {} s", max.as_secs());
            }
        }
    }
    // its WebView2 processes leave just after it
    std::thread::sleep(Duration::from_millis(600));
}

/// When the window has closed: start the new exe if an update put one in place.
pub fn restart_if_updated() {
    let Some((exe, args)) = RESTART.lock().unwrap_or_else(|p| p.into_inner()).take() else { return };
    let dir = exe.parent().map(Path::to_path_buf);
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&args);
    if let Ok(cwd) = std::env::current_dir() {
        cmd.current_dir(cwd);
    } else if let Some(d) = dir {
        cmd.current_dir(d);
    }
    if let Err(e) = cmd.spawn() {
        imagine::log_error!("update: the new version couldn't be started: {e}");
        crate::winutil::error_box(None, &format!("BABEL was updated but couldn't start again.\n\nStart it yourself: {}\n\n{e}", exe.display()), "BABEL");
    }
}

// ---------- HTTP (WinHTTP) ----------

mod http {
    use anyhow::{anyhow, bail, Result};
    use std::ffi::c_void;
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Networking::WinHttp::*;

    struct H(*mut c_void);
    impl Drop for H {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = WinHttpCloseHandle(self.0);
                }
            }
        }
    }
    fn open(p: *mut c_void, what: &str) -> Result<H> {
        if p.is_null() {
            bail!("{what}: {}", windows::core::Error::from_thread().message());
        }
        Ok(H(p))
    }

    /// (secure, host, port, path and query) of an https:// address (http:// only to this PC, for test feeds).
    fn split(url: &str) -> Result<(bool, String, u16, String)> {
        let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
            (true, r)
        } else if let Some(r) = url.strip_prefix("http://") {
            (false, r)
        } else {
            bail!("not a web address");
        };
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().map_err(|_| anyhow!("bad port"))?),
            None => (hostport, if secure { 443 } else { 80 }),
        };
        if host.is_empty() || host.contains('@') {
            bail!("bad address");
        }
        if !secure && !matches!(host, "127.0.0.1" | "localhost") {
            bail!("only https:// addresses are used");
        }
        Ok((secure, host.to_string(), port, path.to_string()))
    }

    /// GET `url` (following redirects, https to https only); fails on anything but 200 or more than `max` bytes.
    /// `progress(done, total)` is told as it arrives.
    pub fn get(url: &str, accept: &str, max: u64, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
        let (secure, host, port, path) = split(url)?;
        let agent = HSTRING::from(format!("BABEL/{}", super::running_version()));
        unsafe {
            let s = open(WinHttpOpen(&agent, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0), "WinHTTP")?;
            WinHttpSetTimeouts(s.0, 15_000, 15_000, 30_000, 30_000)?;
            let c = open(WinHttpConnect(s.0, &HSTRING::from(host), port, 0), "connect")?;
            let flags = if secure { WINHTTP_FLAG_SECURE } else { WINHTTP_OPEN_REQUEST_FLAGS(0) };
            let r = open(WinHttpOpenRequest(c.0, w!("GET"), &HSTRING::from(path), PCWSTR::null(), PCWSTR::null(), std::ptr::null(), flags), "request")?;
            let headers: Vec<u16> = format!("Accept: {accept}\r\nX-GitHub-Api-Version: 2022-11-28\r\n").encode_utf16().collect();
            WinHttpSendRequest(r.0, Some(&headers), None, 0, 0, 0)?;
            WinHttpReceiveResponse(r.0, std::ptr::null_mut())?;
            let mut status: u32 = 0;
            let mut len = 4u32;
            WinHttpQueryHeaders(r.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, PCWSTR::null(), Some(&mut status as *mut u32 as *mut c_void), &mut len, std::ptr::null_mut())?;
            if status != 200 {
                bail!("HTTP {status}");
            }
            // the length as the header has it (a 64-bit number; absent when the answer comes in chunks)
            let total = {
                let mut text = [0u16; 32];
                let mut len = (text.len() * 2) as u32;
                WinHttpQueryHeaders(r.0, WINHTTP_QUERY_CONTENT_LENGTH, PCWSTR::null(), Some(text.as_mut_ptr() as *mut c_void), &mut len, std::ptr::null_mut())
                    .ok()
                    .and_then(|_| String::from_utf16(&text[..(len as usize / 2).min(text.len())]).ok())
                    .and_then(|s| s.trim().parse::<u64>().ok())
            };
            if total.is_some_and(|t| t > max) {
                bail!("the file is bigger than expected");
            }
            let mut out: Vec<u8> = Vec::with_capacity(total.unwrap_or(64 * 1024).min(max) as usize);
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let mut n = 0u32;
                WinHttpReadData(r.0, buf.as_mut_ptr() as *mut c_void, buf.len() as u32, &mut n)?;
                if n == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n as usize]);
                if out.len() as u64 > max {
                    bail!("the file is bigger than expected");
                }
                progress(out.len() as u64, total);
            }
            Ok(out)
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn split_addresses() {
            assert_eq!(super::split("https://api.github.com/repos/a/b?x=1").unwrap(), (true, "api.github.com".into(), 443, "/repos/a/b?x=1".into()));
            assert_eq!(super::split("http://127.0.0.1:8080/f").unwrap(), (false, "127.0.0.1".into(), 8080, "/f".into()));
            assert!(super::split("http://example.com/").is_err());
            assert!(super::split("https://user@evil/").is_err());
            assert!(super::split("file:///c:/x").is_err());
        }

        /// Against a server on this PC: BABEL_TEST_GET=http://localhost:<port>/<file> cargo test -- --ignored get_
        #[test]
        #[ignore]
        fn get_from_a_local_server() {
            let url = std::env::var("BABEL_TEST_GET").expect("BABEL_TEST_GET");
            let mut seen = 0;
            let body = super::get(&url, "*/*", 1 << 30, &mut |done, _| seen = done).unwrap();
            assert!(!body.is_empty());
            assert_eq!(seen, body.len() as u64);
            assert!(super::get(&format!("{url}.missing"), "*/*", 1 << 30, &mut |_, _| {}).unwrap_err().to_string().contains("HTTP 404"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    /// The built-in key is the release key's public half, and a signature made the way the release workflow makes it
    /// (.NET's ECDsa.SignData, SHA-256) checks with Windows' CNG. Signed with the real key, of a version no build is
    /// ever below (0.0.0-keycheck), so it can't install anything.
    #[test]
    fn release_key_and_workflow_signatures_match() {
        use base64::Engine;
        let sig = base64::engine::general_purpose::STANDARD.decode("HwEp/Gvy5scp6Q7IMRNxJ6fDIvZY+25slHbvx5gvo+8Kn4dtB2angU/IX5zua0QvMHBCGQ0XpRtWWJ3SJI5+MQ==").unwrap();
        assert!(signature_ok(b"BABEL.exe 0.0.0-keycheck 0000", &sig, PUBLIC_KEY));
        assert!(!signature_ok(b"BABEL.exe 0.0.0-keycheck 0001", &sig, PUBLIC_KEY));
    }

    #[test]
    fn version_order() {
        assert!(v("0.10.0") > v("0.9.1"));
        assert!(v("v1.0.0") > v("1.0.0-beta.2"));
        assert!(v("1.0.0-beta.10") > v("1.0.0-beta.2"));
        assert!(v("1.0.0-rc.1") > v("1.0.0-beta.9"));
        assert!(v("1.0.0-beta") < v("1.0.0-beta.1"));
        assert_eq!(v("1.2.3+abc"), v("1.2.3"));
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.2.x").is_none());
        assert!(Version::parse("0.0.0-").is_none());
    }

    fn list() -> Value {
        let asset = |tag: &str, name: &str| json!({ "name": name, "size": 2_000_000, "browser_download_url": format!("https://github.com/{REPO}/releases/download/{tag}/{name}") });
        let rel = |tag: &str, pre: bool, draft: bool| {
            json!({ "tag_name": tag, "prerelease": pre, "draft": draft,
                    "body": format!("## What's new in {tag}\n\n- A thing\n\n### Check the download\n\nSHA-256 ..."),
                    "assets": [asset(tag, "BABEL.exe"), asset(tag, "BABEL.exe.sha256"), asset(tag, "BABEL.exe.sig")] })
        };
        json!([rel("v0.12.0", false, true), rel("v0.11.0-beta.1", true, false), rel("v0.10.1", false, false), rel("v0.10.0", false, false), rel("v0.9.1", false, false)])
    }

    #[test]
    fn picks_the_newest_allowed() {
        let r = pick(&list(), &v("0.10.0"), false).unwrap();
        assert_eq!(r.name, "0.10.1"); // the draft 0.12.0 is skipped, the beta too
        assert!(r.installable());
        assert_eq!(r.notes, "## What's new in v0.10.1\n\n- A thing");
        assert_eq!(r.page(), format!("https://github.com/{REPO}/releases/tag/v0.10.1"));
        assert_eq!(pick(&list(), &v("0.10.0"), true).unwrap().name, "0.11.0-beta.1");
        assert!(pick(&list(), &v("0.10.1"), false).is_none());
        assert!(pick(&list(), &v("0.11.0"), true).is_none());
    }

    #[test]
    fn refuses_files_from_elsewhere() {
        let mut l = list();
        l[2]["assets"][0]["browser_download_url"] = json!("https://evil.example/BABEL.exe");
        let r = pick(&l, &v("0.10.0"), false).unwrap();
        assert!(r.exe.is_none() && !r.installable());
    }

    /// Not a key of anything (not even a point on the curve): a signature must never check with it.
    const TEST_PUB: &str = "e0d6ef0fd3bc8dc3faf3d55bb33ea8a9da3ea9e5e0ee5bc4cfa3f1a28b19e2ec1e84d72a1b5a5e67c66a08d6f1b2c52dcf9c3d0c4f0d68d8b6e8b81c46a3df5a";

    #[test]
    fn signatures() {
        // made fresh each run: sign with CNG's own key, check with its public half
        let (pubkey, sig) = test_sign(b"BABEL.exe 1.2.3 abcd");
        assert!(signature_ok(b"BABEL.exe 1.2.3 abcd", &sig, &pubkey));
        assert!(!signature_ok(b"BABEL.exe 1.2.4 abcd", &sig, &pubkey));
        let mut bad = sig.clone();
        bad[10] ^= 1;
        assert!(!signature_ok(b"BABEL.exe 1.2.3 abcd", &bad, &pubkey));
        assert!(!signature_ok(b"BABEL.exe 1.2.3 abcd", &sig, TEST_PUB));
        assert!(!signature_ok(b"x", &sig[..63], &pubkey));
    }

    /// A throwaway P-256 key pair from CNG: (public X||Y as hex, signature of `msg`).
    fn test_sign(msg: &[u8]) -> (String, Vec<u8>) {
        use windows::Win32::Security::Cryptography::*;
        unsafe {
            let mut alg = BCRYPT_ALG_HANDLE::default();
            assert!(BCryptOpenAlgorithmProvider(&mut alg, BCRYPT_ECDSA_P256_ALGORITHM, windows::core::PCWSTR::null(), BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0)).is_ok());
            let mut k = BCRYPT_KEY_HANDLE::default();
            assert!(BCryptGenerateKeyPair(alg, &mut k, 256, 0).is_ok());
            assert!(BCryptFinalizeKeyPair(k, 0).is_ok());
            let mut len = 0u32;
            let _ = BCryptExportKey(k, None, BCRYPT_ECCPUBLIC_BLOB, None, &mut len, 0);
            let mut blob = vec![0u8; len as usize];
            assert!(BCryptExportKey(k, None, BCRYPT_ECCPUBLIC_BLOB, Some(&mut blob), &mut len, 0).is_ok());
            let hash = Sha256::digest(msg);
            let mut sig = vec![0u8; 64];
            let mut n = 0u32;
            assert!(BCryptSignHash(k, None, &hash, Some(&mut sig), &mut n, BCRYPT_FLAGS(0)).is_ok());
            let _ = BCryptDestroyKey(k);
            let _ = BCryptCloseAlgorithmProvider(alg, 0);
            (hex(&blob[8..]), sig)
        }
    }

    #[test]
    fn verify_checks_everything() {
        // the real public key can't sign, so only the failures are checked with it: size, checksum, signature
        let mut r = pick(&list(), &v("0.10.0"), false).unwrap();
        r.exe.as_mut().unwrap().size = 3;
        let exe = b"abc";
        let sha = format!("{}  BABEL.exe", hex(&Sha256::digest(exe)));
        assert!(verify(&r, b"abcd", sha.as_bytes(), b"").unwrap_err().to_string().contains("bytes"));
        assert!(verify(&r, exe, b"00  BABEL.exe", b"").unwrap_err().to_string().contains("SHA-256"));
        use base64::Engine;
        let fake = base64::engine::general_purpose::STANDARD.encode([7u8; 64]);
        assert!(verify(&r, exe, sha.as_bytes(), fake.as_bytes()).unwrap_err().to_string().contains("signature"));
    }
}
