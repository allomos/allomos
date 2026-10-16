//! The `alm` command on a Mac (Task-A07-12). The app carries its own `alm` inside `Allomos.app`
//! (`Contents/MacOS/alm`); Terminal finds it through a link `/usr/local/bin/alm` to it, made by the
//! `.pkg` installer or, after a `.dmg` install, by **Install the alm command** in the app. Because
//! it is a link, updating the app updates `alm` too (owner, 2026-10-13).
//!
//! One link, and the latest install wins (owner, 2026-10-14, Task-A07-15): installing the app
//! points it at the app's own `alm`, installing a development build (`scripts/install-dev-alm.sh`)
//! at `~/.cargo/bin/alm`. The app never changes the link by itself: it says which `alm` is in use
//! and offers to switch to its own, and makes the link when the person clicks.
//!
//! On Windows (Task-A07-14) the installer puts `alm.exe` beside the app and adds the app's folder
//! to the end of the person's own PATH (`windows/hooks.nsh`). The app checks which `alm.exe` a new
//! terminal finds first and, when it is not its own, offers the same fix: on a click it moves its
//! folder to the front of the person's own PATH. Windows searches the system-wide PATH before the
//! person's own, so an `alm.exe` found there keeps winning; the banner says so.
//!
//! "Don't ask again" silences only the mismatch it was clicked for, the path Terminal's `alm` led
//! to ([`AlmStatus::mismatch`]); when that changes, the app asks again (Task-A07-14).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::Serialize;

/// Where the link goes. `/usr/local/bin` is on every Mac terminal's PATH (`/etc/paths`).
pub const LINK: &str = "/usr/local/bin/alm";

/// What is at [`LINK`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Missing,
    /// A link, and where it leads (fully resolved; `None` when it leads nowhere).
    Link(Option<PathBuf>),
    /// A real file (or anything that is not a link).
    File,
}

/// Which `alm` Terminal uses, when it is not the app's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlmKind {
    /// Terminal finds no `alm`.
    Missing,
    /// One built from source: `~/.cargo/bin/alm`, or a `target/debug` or `target/release` build.
    Development,
    /// Anything else, for example another Allomos.app's `alm`.
    Other,
}

/// What the window shows about `alm`. `offer` false: nothing to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct AlmStatus {
    pub offer: bool,
    pub kind: Option<AlmKind>,
    /// The app's own `alm`, shown beside the one in use.
    pub own: Option<String>,
    /// The `alm` Terminal finds now, when it is not the app's own.
    pub found: Option<String>,
    /// Where [`AlmStatus::found`] leads, when it is a link.
    pub leads_to: Option<String>,
    /// A real file at [`LINK`] that installing would replace.
    pub replaces_file: Option<String>,
    /// An `alm` that Terminal finds before [`LINK`] (on Windows: in the system-wide PATH), so it
    /// would still be used after installing.
    pub shadowed_by: Option<String>,
    /// What "Don't ask again" remembers: the path Terminal's `alm` leads to (resolved), empty when
    /// Terminal finds none. The window sends it back with the click.
    pub mismatch: Option<String>,
}

/// How the app makes its own `alm` the one Terminal finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fix {
    /// Mac: the link [`LINK`], found where Terminal searches `/usr/local/bin`.
    Link,
    /// Windows: the app's folder first in the person's own PATH. Windows searches the system-wide
    /// PATH first: `system` is how many entries at the start of [`Terminal::path`] come from it.
    UserPath { system: usize },
}

/// What Terminal sees: its PATH, in order, and the `alm` it finds (fully resolved).
#[derive(Debug, Clone, Default)]
pub struct Terminal {
    pub path: Vec<PathBuf>,
    pub alm: Option<PathBuf>,
    /// The `alm` file Terminal runs, before links are resolved (its folder decides the order).
    pub alm_at: Option<PathBuf>,
}

/// The decision, from facts only, so it can be tested. `own`: the app's own `alm` (on a Mac fully
/// resolved), or `None` where there is no offer (a development build of the app, another system).
/// `dismissed_for`: the mismatch the person chose "Don't ask again" for, if any.
pub fn decide(
    own: Option<&Path>,
    place: &Place,
    terminal: &Terminal,
    fix: Fix,
    dismissed_for: Option<&str>,
) -> AlmStatus {
    let Some(own) = own else {
        return AlmStatus::default();
    };
    // Windows does not tell `C:\Allomos` from `c:\allomos\`.
    let windows = matches!(fix, Fix::UserPath { .. });
    let same = |a: &Path, b: &Path| {
        if windows {
            let fold = |p: &Path| {
                p.to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .to_lowercase()
            };
            fold(a) == fold(b)
        } else {
            a == b
        }
    };
    if terminal.alm.as_deref().is_some_and(|a| same(a, own)) {
        return AlmStatus::default();
    }
    let mismatch = terminal
        .alm
        .as_ref()
        .or(terminal.alm_at.as_ref())
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    if dismissed_for == Some(mismatch.as_str()) {
        return AlmStatus::default();
    }
    let rank = |dir: &Path| terminal.path.iter().position(|p| same(p, dir));
    let shadowed_by = terminal.alm_at.as_ref().filter(|at| match fix {
        // Another `alm` in a folder that Terminal searches before /usr/local/bin (or
        // /usr/local/bin is not searched at all) keeps winning after the link is made.
        Fix::Link => {
            let link_dir = Path::new(LINK).parent().expect("LINK has a folder");
            match (at.parent().and_then(rank), rank(link_dir)) {
                (_, None) => true,
                (Some(other), Some(ours)) => other < ours,
                (None, Some(_)) => false,
            }
        }
        // One in the system-wide PATH stays ahead of the person's own PATH.
        Fix::UserPath { system } => at.parent().and_then(rank).is_some_and(|r| r < system),
    });
    let kind = match (&terminal.alm_at, &terminal.alm) {
        (None, _) => AlmKind::Missing,
        (Some(at), real) if is_development(real.as_deref().unwrap_or(at)) => AlmKind::Development,
        _ => AlmKind::Other,
    };
    AlmStatus {
        offer: true,
        kind: Some(kind),
        own: Some(own.display().to_string()),
        found: terminal.alm_at.as_ref().map(|p| p.display().to_string()),
        leads_to: terminal
            .alm
            .as_ref()
            .filter(|real| Some(*real) != terminal.alm_at.as_ref())
            .map(|p| p.display().to_string()),
        replaces_file: (fix == Fix::Link && place == &Place::File).then(|| LINK.to_string()),
        shadowed_by: shadowed_by
            .filter(|p| p.as_path() != Path::new(LINK))
            .map(|p| p.display().to_string()),
        mismatch: Some(mismatch),
    }
}

/// An `alm` built from source: `cargo install` puts it in `~/.cargo/bin`, `cargo build` in
/// `target/debug` or `target/release`.
fn is_development(alm: &Path) -> bool {
    let Some(dir) = alm.parent() else {
        return false;
    };
    dir.ends_with(".cargo/bin")
        || ((dir.ends_with("debug") || dir.ends_with("release"))
            && dir.parent().is_some_and(|p| p.ends_with("target")))
}

/// The app's own `alm`, when this is a built `Allomos.app` that carries one.
pub fn bundled() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let dir = exe.parent()?;
    if !dir.ends_with("Contents/MacOS") {
        return None;
    }
    dir.join("alm").canonicalize().ok().filter(|p| p.is_file())
}

/// The `alm` beside the app's own program, on any system and in development too
/// (task A03-16): `Allomos.app/Contents/MacOS/alm` when installed, `target/debug/alm` under
/// `tauri dev`, `alm.exe` beside the app on Windows. `None` when there is none (for example
/// `tauri dev` before `cargo build -p allomos-cli`). Resolved on macOS; on Windows the path is kept
/// as it is, because a resolved one starts with `\\?\`, which `PATH` does not take.
pub fn own() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if cfg!(windows) {
        return Some(exe.parent()?.join("alm.exe")).filter(|p| p.is_file());
    }
    let exe = exe.canonicalize().ok()?;
    exe.parent()?
        .join("alm")
        .canonicalize()
        .ok()
        .filter(|p| p.is_file())
}

/// The `alm` the person's own shell finds, resolved: on Windows the first `alm.exe` on the app's
/// `PATH` (which comes from the person's settings), elsewhere what [`terminal`] reports.
pub fn shell_alm() -> Option<PathBuf> {
    if cfg!(windows) {
        let path = std::env::var_os("PATH")?;
        return std::env::split_paths(&path)
            .map(|d| d.join("alm.exe"))
            .find(|p| p.is_file());
    }
    terminal().alm
}

pub fn place() -> Place {
    let link = Path::new(LINK);
    match link.symlink_metadata() {
        Err(_) => Place::Missing,
        Ok(m) if m.file_type().is_symlink() => Place::Link(link.canonicalize().ok()),
        Ok(_) => Place::File,
    }
}

/// Ask the person's own shell what Terminal would see: a Mac app does not get the shell's PATH.
/// Interactive and login, so `~/.zprofile` and `~/.zshrc` are read, as in a new Terminal window.
pub fn terminal() -> Terminal {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    const MARK: &str = "__allomos__";
    let script = format!("printf '\\n{MARK}%s\\n{MARK}%s\\n' \"$PATH\" \"$(command -v alm)\"");
    let mut child = match Command::new(shell)
        .args(["-ilc", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Terminal::default(),
    };
    // A shell set-up that waits for something must not hold the window up.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                return Terminal::default();
            }
        }
    }
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        use std::io::Read;
        let _ = stdout.read_to_string(&mut out);
    }
    parse_terminal(&out, MARK)
}

/// Read the two marked lines the shell printed: PATH, then the `alm` it finds (a path, or empty).
fn parse_terminal(out: &str, mark: &str) -> Terminal {
    let mut lines = out.lines().filter_map(|l| l.strip_prefix(mark));
    let path = lines
        .next()
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    let alm_at = lines
        .next()
        .filter(|a| a.starts_with('/'))
        .map(PathBuf::from);
    Terminal {
        path,
        alm: alm_at.as_ref().and_then(|a| a.canonicalize().ok()),
        alm_at,
    }
}

/// Make the link, asking for the Mac password (the system's own dialog). `Ok(false)`: the person
/// cancelled the password dialog.
pub fn install(bundled: &Path) -> Result<bool, String> {
    let out = Command::new("osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            &format!(
                "do shell script \"mkdir -p /usr/local/bin && ln -sfn \" & quoted form of (item 1 of argv) & \" {LINK}\" with administrator privileges"
            ),
            "-e",
            "end run",
        ])
        .arg(bundled)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(true);
    }
    let text = String::from_utf8_lossy(&out.stderr).trim().to_string();
    // osascript's "User canceled." is error -128.
    if text.contains("-128") {
        Ok(false)
    } else {
        Err(text)
    }
}

/// Windows: the app's own `alm.exe`, in an installed app only (not a `cargo build`).
pub fn windows_own() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    own().filter(|p| !is_development(p))
}

/// Windows: what a new terminal sees, read fresh from the settings (the app's own PATH is the one
/// it was started with, so it misses a change made since, such as the fix). The system-wide PATH
/// comes first, then the person's own; returns their entries and how many are system-wide.
pub fn windows_terminal() -> (Terminal, usize) {
    const MARK: &str = "__allomos__";
    let script = format!(
        "foreach ($t in 'Machine', 'User') {{ Write-Output ('{MARK}' + [Environment]::GetEnvironmentVariable('Path', $t)) }}"
    );
    let out = crate::agents::command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    let (path, system) = match out {
        Some(out) => parse_windows_paths(&out, MARK),
        // PowerShell did not answer: the app's own PATH, order unknown.
        None => (
            std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            0,
        ),
    };
    let alm_at = path.iter().map(|d| d.join("alm.exe")).find(|p| p.is_file());
    (
        Terminal {
            path,
            alm: alm_at.clone(),
            alm_at,
        },
        system,
    )
}

/// Read the two marked lines PowerShell printed (system-wide PATH, then the person's own; entries
/// already expanded) into one list, and how many of its entries are system-wide.
fn parse_windows_paths(out: &str, mark: &str) -> (Vec<PathBuf>, usize) {
    let mut lines = out.lines().filter_map(|l| l.strip_prefix(mark));
    let entries = |line: Option<&str>| -> Vec<PathBuf> {
        line.unwrap_or_default()
            .split(';')
            .map(|e| e.trim().trim_matches('"'))
            .filter(|e| !e.is_empty())
            .map(PathBuf::from)
            .collect()
    };
    let mut path = entries(lines.next());
    let system = path.len();
    path.extend(entries(lines.next()));
    (path, system)
}

/// Windows: put the app's folder first in the person's own PATH (no administrator rights), the
/// same entry the installer adds at the end (`windows/hooks.nsh`), then tell Windows the
/// environment changed so new terminals see it. The raw value is edited, so entries such as
/// `%USERPROFILE%\bin` stay as written, and only the app's own entry moves.
pub fn install_windows(own: &Path) -> Result<(), String> {
    let dir = own.parent().ok_or("the app's alm has no folder")?;
    let out = crate::agents::command("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            WINDOWS_FIX,
        ])
        // Passed in a variable, so a quote in the folder's name cannot break the command.
        .env("ALLOMOS_DIR", dir)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The PowerShell behind [`install_windows`]. Setting a person's variable through .NET announces
/// the change (WM_SETTINGCHANGE) to running programs, Explorer included; removing a variable that
/// does not exist changes nothing else.
const WINDOWS_FIX: &str = "$ErrorActionPreference = 'Stop'; \
$d = $env:ALLOMOS_DIR.TrimEnd('\\'); \
$k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); \
$p = [string]$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); \
$keep = @($p -split ';' | Where-Object { $_.TrimEnd('\\') -ne $d }); \
$new = if ($p -eq '') { $d } else { (@($d) + $keep) -join ';' }; \
if ($new -ne $p) { $k.SetValue('Path', $new, 'ExpandString') }; \
$k.Close(); \
[Environment]::SetEnvironmentVariable('ALLOMOS_PATH_CHANGED', $null, 'User')";

#[cfg(test)]
mod tests {
    use super::*;

    const OURS: &str = "/Applications/Allomos.app/Contents/MacOS/alm";

    fn term(path: &[&str], alm_at: Option<&str>, alm: Option<&str>) -> Terminal {
        Terminal {
            path: path.iter().map(PathBuf::from).collect(),
            alm_at: alm_at.map(PathBuf::from),
            alm: alm.map(PathBuf::from),
        }
    }

    const PATH: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"];

    #[test]
    fn no_offer_outside_a_built_app_or_when_alm_is_ours() {
        let t = term(PATH, None, None);
        assert_eq!(
            decide(None, &Place::Missing, &t, Fix::Link, None),
            AlmStatus::default()
        );
        let ours = Path::new(OURS);
        let t = term(PATH, Some(LINK), Some(OURS));
        let linked = Place::Link(Some(ours.into()));
        assert!(!decide(Some(ours), &linked, &t, Fix::Link, None).offer);
    }

    #[test]
    fn missing_alm_is_offered() {
        let s = decide(
            Some(Path::new(OURS)),
            &Place::Missing,
            &term(PATH, None, None),
            Fix::Link,
            None,
        );
        assert_eq!(
            s,
            AlmStatus {
                offer: true,
                kind: Some(AlmKind::Missing),
                own: Some(OURS.to_string()),
                mismatch: Some(String::new()),
                ..AlmStatus::default()
            }
        );
    }

    #[test]
    fn a_development_build_through_the_link_is_named_with_both_paths() {
        let cargo = "/Users/me/.cargo/bin/alm";
        let t = term(PATH, Some(LINK), Some(cargo));
        let s = decide(
            Some(Path::new(OURS)),
            &Place::Link(Some(cargo.into())),
            &t,
            Fix::Link,
            None,
        );
        assert_eq!(
            s,
            AlmStatus {
                offer: true,
                kind: Some(AlmKind::Development),
                own: Some(OURS.to_string()),
                found: Some(LINK.to_string()),
                leads_to: Some(cargo.to_string()),
                mismatch: Some(cargo.to_string()),
                ..AlmStatus::default()
            }
        );
        // A `cargo build` straight from a checkout counts too; a bundled app in `target` does not.
        assert!(is_development(Path::new("/src/allomos/target/debug/alm")));
        assert!(is_development(Path::new("/src/allomos/target/release/alm")));
        assert!(!is_development(Path::new(
            "/src/allomos/target/release/bundle/macos/Allomos.app/Contents/MacOS/alm"
        )));
        assert!(!is_development(Path::new("/opt/homebrew/bin/alm")));
    }

    #[test]
    fn another_alm_is_named_and_a_real_file_is_flagged() {
        let other = "/usr/local/bin/alm";
        let t = term(PATH, Some(other), Some(other));
        let s = decide(Some(Path::new(OURS)), &Place::File, &t, Fix::Link, None);
        assert!(s.offer);
        assert_eq!(s.kind, Some(AlmKind::Other));
        assert_eq!(s.found.as_deref(), Some(other));
        assert_eq!(s.leads_to, None);
        assert_eq!(s.replaces_file.as_deref(), Some(LINK));
        assert_eq!(s.shadowed_by, None);

        // A link to an older Allomos.app elsewhere: offered, nothing to warn about.
        let old = "/Users/me/Downloads/Allomos.app/Contents/MacOS/alm";
        let t = term(PATH, Some(LINK), Some(old));
        let s = decide(
            Some(Path::new(OURS)),
            &Place::Link(Some(old.into())),
            &t,
            Fix::Link,
            None,
        );
        assert!(s.offer && s.replaces_file.is_none() && s.shadowed_by.is_none());
        assert_eq!(s.kind, Some(AlmKind::Other));
        assert_eq!(s.leads_to.as_deref(), Some(old));
    }

    #[test]
    fn an_alm_found_first_is_said_to_keep_winning() {
        let cargo = "/Users/me/.cargo/bin/alm";
        let path = ["/Users/me/.cargo/bin", "/usr/local/bin", "/usr/bin"];
        let t = term(&path, Some(cargo), Some(cargo));
        let s = decide(Some(Path::new(OURS)), &Place::Missing, &t, Fix::Link, None);
        assert_eq!(s.shadowed_by.as_deref(), Some(cargo));
        assert_eq!(s.kind, Some(AlmKind::Development));

        // Found after /usr/local/bin: the link will win, so no warning.
        let late = ["/usr/local/bin", "/Users/me/.cargo/bin"];
        let t = term(&late, Some(cargo), Some(cargo));
        assert_eq!(
            decide(Some(Path::new(OURS)), &Place::Missing, &t, Fix::Link, None).shadowed_by,
            None
        );

        // /usr/local/bin not searched at all: whatever is found keeps winning.
        let t = term(&["/Users/me/.cargo/bin"], Some(cargo), Some(cargo));
        assert!(
            decide(Some(Path::new(OURS)), &Place::Missing, &t, Fix::Link, None)
                .shadowed_by
                .is_some()
        );
    }

    #[test]
    fn dont_ask_again_silences_only_the_mismatch_it_was_clicked_for() {
        let ours = Some(Path::new(OURS));
        let cargo = "/Users/me/.cargo/bin/alm";
        let to_cargo = term(PATH, Some(LINK), Some(cargo));
        let link_cargo = Place::Link(Some(cargo.into()));
        // The window gets the resolved path to send back with "Don't ask again".
        let s = decide(ours, &link_cargo, &to_cargo, Fix::Link, None);
        assert_eq!(s.mismatch.as_deref(), Some(cargo));
        // Clicked for it: silent while Terminal's alm still leads there.
        assert!(!decide(ours, &link_cargo, &to_cargo, Fix::Link, Some(cargo)).offer);

        // The link moved (another Allomos, a different build): asked again.
        let old = "/Users/me/Downloads/Allomos.app/Contents/MacOS/alm";
        let to_old = term(PATH, Some(LINK), Some(old));
        let s = decide(
            ours,
            &Place::Link(Some(old.into())),
            &to_old,
            Fix::Link,
            Some(cargo),
        );
        assert!(s.offer);
        assert_eq!(s.mismatch.as_deref(), Some(old));

        // No alm at all is its own mismatch (empty): silenced for it, asked again for one found.
        let none = term(PATH, None, None);
        let s = decide(ours, &Place::Missing, &none, Fix::Link, None);
        assert_eq!(s.mismatch.as_deref(), Some(""));
        assert!(!decide(ours, &Place::Missing, &none, Fix::Link, Some("")).offer);
        assert!(decide(ours, &link_cargo, &to_cargo, Fix::Link, Some("")).offer);
        // Silenced for one alm: when Terminal finds none any more, asked again.
        assert!(decide(ours, &Place::Missing, &none, Fix::Link, Some(cargo)).offer);

        // A link that leads nowhere: remembered by where it is.
        let broken = term(PATH, Some(LINK), None);
        let s = decide(ours, &Place::Link(None), &broken, Fix::Link, None);
        assert_eq!(s.mismatch.as_deref(), Some(LINK));
        assert!(!decide(ours, &Place::Link(None), &broken, Fix::Link, Some(LINK)).offer);

        // Once the app's own alm is in use there is nothing to silence, whatever was remembered.
        let to_ours = term(PATH, Some(LINK), Some(OURS));
        let linked = Place::Link(Some(OURS.into()));
        assert_eq!(
            decide(ours, &linked, &to_ours, Fix::Link, Some(cargo)),
            AlmStatus::default()
        );
    }

    // Windows cases, written with `/` so they run on any system (the logic does not read disks).
    const WIN_OWN: &str = "C:/Users/me/AppData/Local/Allomos/alm.exe";
    const WIN_SYS: &[&str] = &["C:/Windows/system32", "C:/Windows"];

    fn win(user: &[&str], found: Option<&str>) -> Terminal {
        let path: Vec<&str> = WIN_SYS.iter().chain(user).copied().collect();
        term(&path, found, found)
    }

    #[test]
    fn windows_the_apps_own_alm_first_is_no_offer_whatever_its_case() {
        let fix = Fix::UserPath {
            system: WIN_SYS.len(),
        };
        let t = win(
            &["c:/users/me/appdata/local/allomos/"],
            Some("c:/users/me/appdata/local/allomos/alm.exe"),
        );
        assert_eq!(
            decide(Some(Path::new(WIN_OWN)), &Place::Missing, &t, fix, None),
            AlmStatus::default()
        );
        // A development build of the app on Windows has no own alm to offer.
        assert_eq!(
            decide(None, &Place::Missing, &win(&[], None), fix, None),
            AlmStatus::default()
        );
    }

    #[test]
    fn windows_an_alm_first_in_the_persons_path_is_offered_and_fixable() {
        let fix = Fix::UserPath {
            system: WIN_SYS.len(),
        };
        let cargo = "C:/Users/me/.cargo/bin/alm.exe";
        let t = win(
            &[
                "C:/Users/me/.cargo/bin",
                "C:/Users/me/AppData/Local/Allomos",
            ],
            Some(cargo),
        );
        let s = decide(Some(Path::new(WIN_OWN)), &Place::Missing, &t, fix, None);
        assert_eq!(
            s,
            AlmStatus {
                offer: true,
                kind: Some(AlmKind::Development),
                own: Some(WIN_OWN.to_string()),
                found: Some(cargo.to_string()),
                mismatch: Some(cargo.to_string()),
                ..AlmStatus::default()
            }
        );
        // Another program's alm.exe: offered as "other"; "Don't ask again" for it holds.
        let other = "C:/Tools/alm.exe";
        let t = win(
            &["C:/Tools", "C:/Users/me/AppData/Local/Allomos"],
            Some(other),
        );
        let s = decide(Some(Path::new(WIN_OWN)), &Place::Missing, &t, fix, None);
        assert_eq!((s.kind, s.shadowed_by), (Some(AlmKind::Other), None));
        assert!(
            !decide(
                Some(Path::new(WIN_OWN)),
                &Place::Missing,
                &t,
                fix,
                Some(other)
            )
            .offer
        );
    }

    #[test]
    fn windows_an_alm_in_the_system_path_is_said_to_keep_winning() {
        let fix = Fix::UserPath {
            system: WIN_SYS.len(),
        };
        let sys = "C:/Windows/alm.exe";
        let t = win(&["C:/Users/me/AppData/Local/Allomos"], Some(sys));
        let s = decide(Some(Path::new(WIN_OWN)), &Place::Missing, &t, fix, None);
        assert!(s.offer);
        assert_eq!(s.shadowed_by.as_deref(), Some(sys));
        assert_eq!(s.replaces_file, None);
    }

    #[test]
    fn windows_no_alm_on_the_path_is_offered() {
        let fix = Fix::UserPath {
            system: WIN_SYS.len(),
        };
        let s = decide(
            Some(Path::new(WIN_OWN)),
            &Place::File,
            &win(&[], None),
            fix,
            None,
        );
        assert_eq!(
            s,
            AlmStatus {
                offer: true,
                kind: Some(AlmKind::Missing),
                own: Some(WIN_OWN.to_string()),
                mismatch: Some(String::new()),
                ..AlmStatus::default()
            }
        );
    }

    #[test]
    fn windows_paths_are_read_system_first() {
        let out = "noise\r\n__m__C:\\Windows\\system32;C:\\Windows;\r\n__m__\"C:\\Tools\";;C:\\Users\\me\\AppData\\Local\\Allomos\r\n";
        let (path, system) = parse_windows_paths(out, "__m__");
        assert_eq!(system, 2);
        assert_eq!(
            path,
            [
                r"C:\Windows\system32",
                r"C:\Windows",
                r"C:\Tools",
                r"C:\Users\me\AppData\Local\Allomos"
            ]
            .map(PathBuf::from)
        );
        assert_eq!(parse_windows_paths("", "__m__"), (Vec::new(), 0));
    }

    #[test]
    fn the_shell_answer_is_read_past_any_greeting() {
        // PATH joined with the system's own separator (`:`, on Windows `;`).
        let path = std::env::join_paths(["/a/bin", "/usr/local/bin"]).unwrap();
        let out = format!(
            "Welcome!\n\n__m__{}\n__m__/no/such/alm\n",
            path.to_string_lossy()
        );
        let t = parse_terminal(&out, "__m__");
        assert_eq!(
            t.path,
            vec![PathBuf::from("/a/bin"), PathBuf::from("/usr/local/bin")]
        );
        assert_eq!(t.alm_at, Some(PathBuf::from("/no/such/alm")));
        assert_eq!(t.alm, None);
        let t = parse_terminal("__m__/usr/bin\n__m__\n", "__m__");
        assert_eq!(t.alm_at, None);
    }
}
