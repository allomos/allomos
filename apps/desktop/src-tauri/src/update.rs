//! Is a newer Allomos on GitHub (Task-A07-16)? The app asks GitHub at most once a day, and
//! whenever the person chooses **Check for Updates…**; it remembers the answer, so a notice
//! still shows after a restart without asking again.
//!
//! GitHub is asked with `curl`, which macOS and Windows 10 and later already have, so the app
//! needs no web library. Any failure (no network, GitHub busy, no `curl`) is quiet: the app
//! simply does not know of a newer version.
//!
//! When a newer version is known, the app downloads its installer in the background and checks
//! it against the SHA-256 hash file published beside it in the release (Task-A07-17, approver
//! 2026-10-18: a hash, not a signing key). Only a download whose hash matches is kept; the menu
//! then offers **Restart to Update to Allomos <version>**, which runs the installer and starts
//! the new version. The hash proves only that the download is intact: whoever can upload to the
//! release could upload a matching hash too (the approver knows and accepted this).

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// GitHub's answer for the newest published release (drafts and pre-releases are left out).
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/allomos/allomos/releases/latest";
/// How long one answer is trusted before GitHub is asked again (approver, 2026-10-15).
pub const CHECK_EVERY_SECS: u64 = 24 * 60 * 60;
/// How long to wait for GitHub before giving up.
const TIMEOUT_SECS: &str = "10";

/// What the window shows. `newer` is the version to offer, when there is one to offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    /// This Allomos's version.
    pub current: String,
    /// The newest version known, from this check or the last one that worked.
    pub latest: Option<String>,
    /// The version to offer: newer than this one, and not skipped (unless asked from the menu).
    pub newer: Option<String>,
    /// A newer version than this one, even a skipped one: the menu offers it (approver,
    /// 2026-10-15: like the Claude app's "Restart to Update" item).
    pub available: Option<String>,
    /// GitHub was asked now and did not answer.
    pub failed: bool,
    /// `available`'s installer is downloaded and its hash matched: the menu offers **Restart to
    /// Update to Allomos <version>** (Task-A07-17).
    pub ready: Option<String>,
}

/// `0.1.5` or `v0.1.5` as numbers; `None` for anything else (`0.2.0-beta`, `nightly`).
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let text = text.trim();
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut parts = text.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// The newest release's version from GitHub's answer, without the leading `v`.
pub fn latest_from_json(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    parse_version(tag)?;
    Some(tag.trim().trim_start_matches('v').to_string())
}

/// Whether GitHub should be asked now: always from the menu, otherwise when the last answer is
/// missing or a day old. A clock set back makes the last answer count as old.
pub fn due(checked_at: Option<u64>, now: u64, asked: bool) -> bool {
    match checked_at {
        _ if asked => true,
        None => true,
        Some(at) => now < at || now - at >= CHECK_EVERY_SECS,
    }
}

/// What to offer, given what is known. A skipped version is not offered again unless the person
/// asked from the menu; a newer one than the skipped one is.
pub fn decide(
    current: &str,
    latest: Option<&str>,
    skipped: Option<&str>,
    asked: bool,
    failed: bool,
) -> UpdateStatus {
    let available = latest.filter(|l| {
        matches!(
            (parse_version(l), parse_version(current)),
            (Some(l), Some(c)) if l > c
        )
    });
    let newer = available.filter(|l| asked || skipped != Some(l));
    UpdateStatus {
        current: current.to_string(),
        latest: latest.map(String::from),
        newer: newer.map(String::from),
        available: available.map(String::from),
        failed,
        ready: None,
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Ask GitHub for the newest release's version.
pub fn fetch_latest(current: &str) -> Option<String> {
    let output = crate::agents::command("curl")
        .args([
            "--silent",
            "--fail",
            "--location",
            "--max-time",
            TIMEOUT_SECS,
        ])
        .args(["--header", "Accept: application/vnd.github+json"])
        .args(["--user-agent", &format!("Allomos/{current}")])
        .arg(LATEST_RELEASE_API)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    latest_from_json(&String::from_utf8_lossy(&output.stdout))
}

/// The whole check: ask GitHub if due, remember the answer, and say what to offer.
pub fn check(asked: bool) -> UpdateStatus {
    let current = env!("CARGO_PKG_VERSION");
    let mut memory = allomos_store::update_memory();
    let mut failed = false;
    let now = now();
    if due(memory.checked_at, now, asked) {
        match fetch_latest(current) {
            Some(latest) => {
                let _ = allomos_store::save_update_check(now, &latest);
                memory.latest = Some(latest);
            }
            None => failed = true,
        }
    }
    let mut status = decide(
        current,
        memory.latest.as_deref(),
        memory.skipped.as_deref(),
        asked,
        failed,
    );
    status.ready = ready_version().filter(|r| status.available.as_ref() == Some(r));
    status
}

// ---- Downloading and installing (Task-A07-17) ----

/// Where a release's files are downloaded from: `<this>/v<version>/<file name>`.
const RELEASE_DOWNLOAD: &str = "https://github.com/allomos/allomos/releases/download";
/// How long one download may take before it is given up (it is tried again at the next check).
const DOWNLOAD_TIMEOUT_SECS: &str = "900";
/// Where the macOS `.pkg` puts the app (`apps/desktop/scripts/build-mac-pkg.sh`).
#[cfg(target_os = "macos")]
const MAC_APP: &str = "/Applications/Allomos.app";

/// The systems Allomos publishes an installer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS on Apple Silicon: `Allomos_<v>_aarch64.pkg`.
    MacArm,
    /// 64-bit Windows: `Allomos_<v>_x64-setup.exe`.
    WindowsX64,
}

impl Platform {
    /// This computer's, or `None` where no installer is published (an Intel Mac, Linux): there
    /// the app keeps only A07-16's notice and download page.
    pub fn current() -> Option<Platform> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Platform::MacArm)
        } else if cfg!(all(windows, target_arch = "x86_64")) {
            Some(Platform::WindowsX64)
        } else {
            None
        }
    }
}

/// One release's installer for one system, and the hash file published beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    pub name: String,
    pub hash_name: String,
    pub url: String,
    pub hash_url: String,
}

/// The installer to download for `version` on `platform`, as `.github/workflows/release.yml`
/// names it; its hash file is the same name with `.sha256` added.
pub fn installer_for(platform: Platform, version: &str) -> Installer {
    let name = match platform {
        Platform::MacArm => format!("Allomos_{version}_aarch64.pkg"),
        Platform::WindowsX64 => format!("Allomos_{version}_x64-setup.exe"),
    };
    let hash_name = format!("{name}.sha256");
    Installer {
        url: format!("{RELEASE_DOWNLOAD}/v{version}/{name}"),
        hash_url: format!("{RELEASE_DOWNLOAD}/v{version}/{hash_name}"),
        name,
        hash_name,
    }
}

/// Why a download was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashRefused {
    /// The hash file does not start with 64 hexadecimal digits.
    NoHash,
    /// The download's hash is not the published one.
    Differs { expected: String, actual: String },
}

impl std::fmt::Display for HashRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashRefused::NoHash => write!(f, "the release's hash file holds no SHA-256 hash"),
            HashRefused::Differs { expected, actual } => write!(
                f,
                "the download's SHA-256 hash is {actual}, but the release says {expected}"
            ),
        }
    }
}

/// The hash a hash file names, in lower case. The file is what `shasum -a 256` and `sha256sum`
/// write (`<hash>  <file name>`, or `<hash> *<file name>`), or the hash alone.
pub fn published_hash(hash_file: &str) -> Option<String> {
    let hash = hash_file.split_whitespace().next()?;
    (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| hash.to_ascii_lowercase())
}

/// Whether `actual` (the download's SHA-256, in hexadecimal) is the hash the hash file names.
pub fn verify(actual: &str, hash_file: &str) -> Result<(), HashRefused> {
    let expected = published_hash(hash_file).ok_or(HashRefused::NoHash)?;
    let actual = actual.to_ascii_lowercase();
    if actual == expected {
        Ok(())
    } else {
        Err(HashRefused::Differs { expected, actual })
    }
}

/// Keep a downloaded file only if its hash is the published one; a refused file is deleted, so
/// it can never be installed. Gives the hash back on success.
pub fn accept_download(path: &Path, hash_file: &str) -> io::Result<String> {
    let actual = sha256_file(path)?;
    match verify(&actual, hash_file) {
        Ok(()) => Ok(actual),
        Err(refused) => {
            let _ = fs::remove_file(path);
            Err(io::Error::other(refused.to_string()))
        }
    }
}

/// A downloaded installer whose hash matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub version: String,
    pub path: PathBuf,
    pub sha256: String,
}

/// What this run of the app has downloaded, and whether a download is going on.
#[derive(Default)]
struct Downloads {
    ready: Option<Ready>,
    busy: bool,
}

static DOWNLOADS: Mutex<Downloads> = Mutex::new(Downloads {
    ready: None,
    busy: false,
});

fn downloads() -> std::sync::MutexGuard<'static, Downloads> {
    DOWNLOADS.lock().unwrap_or_else(|e| e.into_inner())
}

/// The version whose installer is downloaded and checked.
pub fn ready_version() -> Option<String> {
    downloads().ready.as_ref().map(|r| r.version.clone())
}

/// The folder downloads go to: the system's temporary folder, as an installer is needed only
/// until it has run.
fn download_dir() -> PathBuf {
    std::env::temp_dir().join("allomos-update")
}

/// Start downloading `version`'s installer in the background, unless it is downloaded already,
/// a download is going on, or no installer is published for this system. `on_ready` runs once
/// the download is kept. A failed or refused download is quiet: the menu keeps offering the
/// download page, and the next check tries again.
pub fn prepare_in_background(version: String, on_ready: impl FnOnce() + Send + 'static) {
    let Some(platform) = Platform::current() else {
        return;
    };
    {
        let mut state = downloads();
        if state.busy || state.ready.as_ref().is_some_and(|r| r.version == version) {
            return;
        }
        state.busy = true;
    }
    std::thread::spawn(move || {
        let result = download(platform, &version);
        let mut state = downloads();
        state.busy = false;
        if let Ok(ready) = result {
            state.ready = Some(ready);
            drop(state);
            on_ready();
        }
    });
}

/// Download `version`'s hash file and installer, and keep the installer if its hash matches.
fn download(platform: Platform, version: &str) -> io::Result<Ready> {
    let installer = installer_for(platform, version);
    let hash_file = curl_text(&installer.hash_url)?;
    let dir = download_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(&installer.name);
    // An installer downloaded earlier is kept if it still matches.
    if let Ok(sha256) = accept_download(&path, &hash_file) {
        return Ok(Ready {
            version: version.to_string(),
            path,
            sha256,
        });
    }
    // Older downloads are no longer needed.
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
    let part = dir.join(format!("{}.part", installer.name));
    let status = crate::agents::command("curl")
        .args(["--silent", "--fail", "--location"])
        .args(["--max-time", DOWNLOAD_TIMEOUT_SECS])
        .args([
            "--user-agent",
            &format!("Allomos/{}", env!("CARGO_PKG_VERSION")),
        ])
        .arg("--output")
        .arg(&part)
        .arg(&installer.url)
        .status()?;
    if !status.success() {
        let _ = fs::remove_file(&part);
        return Err(io::Error::other(format!("curl stopped with {status}")));
    }
    let sha256 = accept_download(&part, &hash_file)?;
    fs::rename(&part, &path)?;
    Ok(Ready {
        version: version.to_string(),
        path,
        sha256,
    })
}

/// A small text file from the web.
fn curl_text(url: &str) -> io::Result<String> {
    let output = crate::agents::command("curl")
        .args([
            "--silent",
            "--fail",
            "--location",
            "--max-time",
            TIMEOUT_SECS,
        ])
        .args([
            "--user-agent",
            &format!("Allomos/{}", env!("CARGO_PKG_VERSION")),
        ])
        .arg(url)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "curl stopped with {}",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// What **Restart to Update** did.
#[derive(Debug, PartialEq, Eq)]
pub enum Restart {
    /// The installer runs (or has run); the app must now quit, and the new version starts.
    Quit,
    /// The person chose Cancel when macOS asked for the password: nothing changed.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Cancelled,
}

/// **Restart to Update**: check the downloaded installer once more, then run it. On macOS the
/// installer runs now (macOS asks for the Mac password, as the `.pkg` installs into
/// /Applications) and a helper starts the new version once this app has quit. On Windows a
/// helper waits for this app to quit, runs the installer silently for the current user, and
/// starts the new version. The caller quits the app on [`Restart::Quit`].
pub fn restart_to_update() -> Result<Restart, String> {
    let ready = downloads()
        .ready
        .clone()
        .ok_or_else(|| "no checked download is waiting".to_string())?;
    // The file could have changed since it was checked.
    let actual = sha256_file(&ready.path).map_err(|e| e.to_string())?;
    if actual != ready.sha256 {
        let _ = fs::remove_file(&ready.path);
        downloads().ready = None;
        return Err(HashRefused::Differs {
            expected: ready.sha256,
            actual,
        }
        .to_string());
    }
    run_installer(&ready.path)
}

#[cfg(target_os = "macos")]
fn run_installer(pkg: &Path) -> Result<Restart, String> {
    // The path is handed over as an argument, so no character in it can change the command.
    let output = crate::agents::command("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "do shell script \"/usr/sbin/installer -pkg \" & quoted form of (item 1 of argv) & \" -target /\" with administrator privileges",
            "-e",
            "end run",
        ])
        .arg(pkg)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        // -128: the person chose Cancel.
        if detail.contains("-128") {
            return Ok(Restart::Cancelled);
        }
        return Err(detail.trim().to_string());
    }
    crate::agents::command("/bin/sh")
        .args([
            "-c",
            "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open \"$2\"",
            "sh",
        ])
        .arg(std::process::id().to_string())
        .arg(MAC_APP)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(Restart::Quit)
}

#[cfg(windows)]
fn run_installer(setup: &Path) -> Result<Restart, String> {
    // The installer cannot replace the app while it runs, so a helper waits for it to quit.
    // `/S`: silent; Tauri's installer installs for the current user, so Windows asks nothing.
    // The installer keeps the folder of the installed version, so the new one starts from this
    // app's own path. Paths are handed over in environment variables, so a quote in them cannot
    // break the command.
    let app = std::env::current_exe().map_err(|e| e.to_string())?;
    crate::agents::command("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Wait-Process -Id ([int]$env:ALLOMOS_PID) -Timeout 120 -ErrorAction SilentlyContinue; \
             Start-Process -FilePath $env:ALLOMOS_SETUP -ArgumentList '/S' -Wait; \
             Start-Process -FilePath $env:ALLOMOS_APP",
        ])
        .env("ALLOMOS_PID", std::process::id().to_string())
        .env("ALLOMOS_SETUP", setup)
        .env("ALLOMOS_APP", app)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(Restart::Quit)
}

#[cfg(not(any(target_os = "macos", windows)))]
fn run_installer(_: &Path) -> Result<Restart, String> {
    Err("Allomos cannot update itself on this system".to_string())
}

// ---- SHA-256 (FIPS 180-4), written here so the app needs no new library ----

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// A SHA-256 hash fed in pieces.
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Sha256 {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }
}

impl Sha256 {
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    /// The hash, as 64 lower-case hexadecimal digits.
    pub fn finish(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut hex = String::with_capacity(64);
        for word in self.state {
            let _ = write!(hex, "{word:08x}");
        }
        hex
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

/// The SHA-256 of `bytes`, in hexadecimal.
#[cfg(test)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hash = Sha256::default();
    hash.update(bytes);
    hash.finish()
}

/// The SHA-256 of a file, read in pieces.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::default();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            return Ok(hash.finish());
        }
        hash.update(&buffer[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_examples() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // A million "a", fed in uneven pieces.
        let mut hash = Sha256::default();
        let piece = [b'a'; 997];
        let mut left = 1_000_000;
        while left > 0 {
            let n = left.min(piece.len());
            hash.update(&piece[..n]);
            left -= n;
        }
        assert_eq!(
            hash.finish(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn a_matching_hash_passes_and_any_other_is_refused() {
        let actual = sha256_hex(b"installer");
        // As `shasum -a 256` writes it, as `sha256sum` writes it in binary mode, and alone.
        assert_eq!(
            verify(&actual, &format!("{actual}  Allomos_0.2.0_aarch64.pkg\n")),
            Ok(())
        );
        assert_eq!(
            verify(&actual, &format!("{actual} *Allomos_0.2.0_x64-setup.exe")),
            Ok(())
        );
        assert_eq!(verify(&actual, &actual.to_uppercase()), Ok(()));
        let other = sha256_hex(b"something else");
        assert_eq!(
            verify(&actual, &format!("{other}  Allomos_0.2.0_aarch64.pkg")),
            Err(HashRefused::Differs {
                expected: other,
                actual: actual.clone()
            })
        );
        assert_eq!(verify(&actual, ""), Err(HashRefused::NoHash));
        assert_eq!(
            verify(&actual, "<html>Not Found</html>"),
            Err(HashRefused::NoHash)
        );
        assert_eq!(verify(&actual, &actual[..63]), Err(HashRefused::NoHash));
    }

    #[test]
    fn a_refused_download_is_deleted() {
        let dir = std::env::temp_dir().join(format!("allomos-update-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Allomos_0.2.0_aarch64.pkg");
        fs::write(&file, b"installer").unwrap();
        let good = sha256_hex(b"installer");
        assert_eq!(accept_download(&file, &good).unwrap(), good);
        assert!(file.exists());
        assert!(accept_download(&file, &sha256_hex(b"tampered")).is_err());
        assert!(!file.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn each_system_gets_its_own_installer_and_hash_file() {
        let mac = installer_for(Platform::MacArm, "0.2.0");
        assert_eq!(mac.name, "Allomos_0.2.0_aarch64.pkg");
        assert_eq!(mac.hash_name, "Allomos_0.2.0_aarch64.pkg.sha256");
        assert_eq!(
            mac.url,
            "https://github.com/allomos/allomos/releases/download/v0.2.0/Allomos_0.2.0_aarch64.pkg"
        );
        assert_eq!(
            mac.hash_url,
            "https://github.com/allomos/allomos/releases/download/v0.2.0/Allomos_0.2.0_aarch64.pkg.sha256"
        );
        let windows = installer_for(Platform::WindowsX64, "0.2.0");
        assert_eq!(windows.name, "Allomos_0.2.0_x64-setup.exe");
        assert_eq!(windows.hash_name, "Allomos_0.2.0_x64-setup.exe.sha256");
        assert_eq!(
            windows.url,
            "https://github.com/allomos/allomos/releases/download/v0.2.0/Allomos_0.2.0_x64-setup.exe"
        );
        assert_eq!(
            windows.hash_url,
            "https://github.com/allomos/allomos/releases/download/v0.2.0/Allomos_0.2.0_x64-setup.exe.sha256"
        );
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(Platform::current(), Some(Platform::MacArm));
        }
        if cfg!(all(windows, target_arch = "x86_64")) {
            assert_eq!(Platform::current(), Some(Platform::WindowsX64));
        }
    }

    #[test]
    fn versions_are_read_with_or_without_a_v() {
        assert_eq!(parse_version("0.1.5"), Some((0, 1, 5)));
        assert_eq!(parse_version("v1.20.3"), Some((1, 20, 3)));
        assert_eq!(parse_version("0.2.0-beta"), None);
        assert_eq!(parse_version("0.2"), None);
        assert_eq!(parse_version("0.2.0.1"), None);
        assert_eq!(parse_version("nightly"), None);
    }

    #[test]
    fn the_latest_version_comes_from_the_tag_name() {
        let json = r#"{"tag_name":"v0.1.6","html_url":"https://github.com/x","draft":false}"#;
        assert_eq!(latest_from_json(json).as_deref(), Some("0.1.6"));
        assert_eq!(latest_from_json(r#"{"tag_name":"latest-build"}"#), None);
        assert_eq!(latest_from_json(r#"{"message":"Not Found"}"#), None);
        assert_eq!(latest_from_json("<html>"), None);
    }

    #[test]
    fn github_is_asked_once_a_day_or_from_the_menu() {
        let day = CHECK_EVERY_SECS;
        assert!(due(None, 5, false));
        assert!(!due(Some(1_000), 1_000 + day - 1, false));
        assert!(due(Some(1_000), 1_000 + day, false));
        assert!(due(Some(1_000), 1_001, true));
        // The clock was set back.
        assert!(due(Some(1_000), 999, false));
    }

    #[test]
    fn only_a_newer_version_is_offered() {
        assert_eq!(
            decide("0.1.5", Some("0.1.6"), None, false, false)
                .newer
                .as_deref(),
            Some("0.1.6")
        );
        // Compared as numbers, not as text.
        assert_eq!(
            decide("0.1.9", Some("0.1.10"), None, false, false)
                .newer
                .as_deref(),
            Some("0.1.10")
        );
        assert_eq!(
            decide("0.1.5", Some("0.1.5"), None, false, false).newer,
            None
        );
        assert_eq!(
            decide("0.2.0", Some("0.1.9"), None, false, false).newer,
            None
        );
        assert_eq!(decide("0.1.5", None, None, false, true).newer, None);
    }

    #[test]
    fn a_skipped_version_is_offered_only_from_the_menu() {
        let quiet = decide("0.1.5", Some("0.1.6"), Some("0.1.6"), false, false);
        assert_eq!(quiet.newer, None);
        assert_eq!(quiet.latest.as_deref(), Some("0.1.6"));
        // The menu still offers it.
        assert_eq!(quiet.available.as_deref(), Some("0.1.6"));
        let asked = decide("0.1.5", Some("0.1.6"), Some("0.1.6"), true, false);
        assert_eq!(asked.newer.as_deref(), Some("0.1.6"));
        // A version newer than the skipped one is offered again.
        let next = decide("0.1.5", Some("0.1.7"), Some("0.1.6"), false, false);
        assert_eq!(next.newer.as_deref(), Some("0.1.7"));
    }
}
