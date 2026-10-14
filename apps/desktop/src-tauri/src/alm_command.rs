//! The `alm` command on a Mac (Task-A07-12). The app carries its own `alm` inside `Allomos.app`
//! (`Contents/MacOS/alm`); Terminal finds it through a link `/usr/local/bin/alm` to it, made by the
//! `.pkg` installer or, after a `.dmg` install, by **Install the alm command** in the app. Because
//! it is a link, updating the app updates `alm` too (owner, 2026-10-13).
//!
//! The app never replaces `alm` by itself: it only offers, and makes the link when the person
//! clicks. A real file at `/usr/local/bin/alm` is named in the offer before it is replaced.

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

/// What the window shows about `alm`. `offer` false: nothing to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct AlmStatus {
    pub offer: bool,
    /// The `alm` Terminal finds now, when it is not the app's own.
    pub found: Option<String>,
    /// A real file at [`LINK`] that installing would replace.
    pub replaces_file: Option<String>,
    /// An `alm` that Terminal finds before [`LINK`], so it would still be used after installing.
    pub shadowed_by: Option<String>,
}

/// What Terminal sees: its PATH, in order, and the `alm` it finds (fully resolved).
#[derive(Debug, Clone, Default)]
pub struct Terminal {
    pub path: Vec<PathBuf>,
    pub alm: Option<PathBuf>,
    /// The `alm` file Terminal runs, before links are resolved (its folder decides the order).
    pub alm_at: Option<PathBuf>,
}

/// The decision, from facts only, so it can be tested. `bundled`: the app's own `alm` (fully
/// resolved), or `None` outside a built app (development) or off macOS, where there is no offer.
pub fn decide(
    bundled: Option<&Path>,
    place: &Place,
    terminal: &Terminal,
    dismissed: bool,
) -> AlmStatus {
    let Some(bundled) = bundled else {
        return AlmStatus::default();
    };
    if terminal.alm.as_deref() == Some(bundled) || dismissed {
        return AlmStatus::default();
    }
    let link_dir = Path::new(LINK).parent().expect("LINK has a folder");
    // Another `alm` in a folder that Terminal searches before /usr/local/bin (or /usr/local/bin is
    // not searched at all) keeps winning after the link is made.
    let shadowed_by = terminal.alm_at.as_ref().filter(|at| {
        let rank = |dir: &Path| terminal.path.iter().position(|p| p == dir);
        match (at.parent().and_then(rank), rank(link_dir)) {
            (_, None) => true,
            (Some(other), Some(ours)) => other < ours,
            (None, Some(_)) => false,
        }
    });
    AlmStatus {
        offer: true,
        found: terminal.alm_at.as_ref().map(|p| p.display().to_string()),
        replaces_file: (place == &Place::File).then(|| LINK.to_string()),
        shadowed_by: shadowed_by
            .filter(|p| p.as_path() != Path::new(LINK))
            .map(|p| p.display().to_string()),
    }
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
/// ([`Task-A03-16`]): `Allomos.app/Contents/MacOS/alm` when installed, `target/debug/alm` under
/// `tauri dev`, `alm.exe` beside the app on Windows. `None` when there is none (for example
/// `tauri dev` before `cargo build -p allomos-cli`). Resolved on macOS; on Windows the path is kept
/// as it is, because a resolved one starts with `\\?\`, which `PATH` does not take.
///
/// [`Task-A03-16`]: ../../../../docs/tasks/Task-A03-16-agents-use-the-apps-own-alm.md
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
            decide(None, &Place::Missing, &t, false),
            AlmStatus::default()
        );
        let ours = Path::new(OURS);
        let t = term(PATH, Some(LINK), Some(OURS));
        let linked = Place::Link(Some(ours.into()));
        assert!(!decide(Some(ours), &linked, &t, false).offer);
    }

    #[test]
    fn missing_alm_is_offered() {
        let s = decide(
            Some(Path::new(OURS)),
            &Place::Missing,
            &term(PATH, None, None),
            false,
        );
        assert_eq!(
            s,
            AlmStatus {
                offer: true,
                ..AlmStatus::default()
            }
        );
    }

    #[test]
    fn another_alm_is_named_and_a_real_file_is_flagged() {
        let other = "/usr/local/bin/alm";
        let t = term(PATH, Some(other), Some(other));
        let s = decide(Some(Path::new(OURS)), &Place::File, &t, false);
        assert!(s.offer);
        assert_eq!(s.found.as_deref(), Some(other));
        assert_eq!(s.replaces_file.as_deref(), Some(LINK));
        assert_eq!(s.shadowed_by, None);

        // A link to an older Allomos.app elsewhere: offered, nothing to warn about.
        let old = "/Users/me/Downloads/Allomos.app/Contents/MacOS/alm";
        let t = term(PATH, Some(LINK), Some(old));
        let s = decide(
            Some(Path::new(OURS)),
            &Place::Link(Some(old.into())),
            &t,
            false,
        );
        assert!(s.offer && s.replaces_file.is_none() && s.shadowed_by.is_none());
    }

    #[test]
    fn an_alm_found_first_is_said_to_keep_winning() {
        let cargo = "/Users/me/.cargo/bin/alm";
        let path = ["/Users/me/.cargo/bin", "/usr/local/bin", "/usr/bin"];
        let t = term(&path, Some(cargo), Some(cargo));
        let s = decide(Some(Path::new(OURS)), &Place::Missing, &t, false);
        assert_eq!(s.shadowed_by.as_deref(), Some(cargo));

        // Found after /usr/local/bin: the link will win, so no warning.
        let late = ["/usr/local/bin", "/Users/me/.cargo/bin"];
        let t = term(&late, Some(cargo), Some(cargo));
        assert_eq!(
            decide(Some(Path::new(OURS)), &Place::Missing, &t, false).shadowed_by,
            None
        );

        // /usr/local/bin not searched at all: whatever is found keeps winning.
        let t = term(&["/Users/me/.cargo/bin"], Some(cargo), Some(cargo));
        assert!(
            decide(Some(Path::new(OURS)), &Place::Missing, &t, false)
                .shadowed_by
                .is_some()
        );
    }

    #[test]
    fn dont_ask_again_is_kept() {
        let t = term(PATH, None, None);
        assert!(!decide(Some(Path::new(OURS)), &Place::Missing, &t, true).offer);
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
