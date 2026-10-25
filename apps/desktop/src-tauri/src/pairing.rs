//! Pairing a phone or another computer with the desktop app (Task-A04-02-02, step 3 of A04-02;
//! decision 0009, R80). Before pairing, a device on the network gets the page and nothing from a
//! taskbag; after it, the device reads with the access it was given, until the person removes it.
//!
//! The steps, as the approver set them (2026-10-25):
//! 1. The device's page asks for a device name ([`Pairing::ask`]).
//! 2. The desktop app shows "<name> asks to connect"; the person approves or refuses it
//!    ([`Pairing::answer`]).
//! 3. On approval the desktop app shows a code of 9 random digits in three groups (`482 913 076`).
//!    It works once, for about [`CODE_LIFE`]; after [`TRIES`] wrong tries it is replaced by a new
//!    one and the old one no longer works.
//! 4. Typed on the device (spaces may be left out), the code gives the device its access: a random
//!    token its browser keeps ([`Pairing::confirm`]). Only the token's SHA-256 is saved here.
//!
//! Paired devices are kept in the app's settings folder (`lan/devices.json`); removing one
//! ([`Pairing::remove`]) takes its access away at once, since every request checks the list.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// How long a code works.
pub const CODE_LIFE: Duration = Duration::from_secs(15 * 60);
/// Wrong tries before the code is replaced.
pub const TRIES: u8 = 5;
/// How long a request waits for the person's answer before it is dropped.
const ASK_LIFE: Duration = Duration::from_secs(15 * 60);
/// The most requests waiting at once; a new one drops the oldest.
const MAX_ASKING: usize = 8;
/// The longest device name, in characters.
pub const NAME_MAX: usize = 40;
const DEVICES_FILE: &str = "devices.json";
/// A device's "last seen" is saved at most this often (it is kept exact in memory).
const SAVE_SEEN_EVERY: Duration = Duration::from_secs(60);

/// Random bytes from the operating system, through the crypto provider the server already uses.
fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut buf)
        .expect("the system's random source works");
    buf
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn sha256_hex(text: &str) -> String {
    use sha2::Digest;
    hex(&sha2::Sha256::digest(text.as_bytes()))
}

/// 9 random digits, every value equally likely.
fn new_code() -> String {
    loop {
        let n = u32::from_le_bytes(random_bytes::<4>());
        // 4 × 10⁹ fits a u32; values past it would favour small codes, so they are drawn again.
        if n < 4_000_000_000 {
            return format!("{:09}", n % 1_000_000_000);
        }
    }
}

/// `482913076` → `482 913 076`, as the desktop app shows it.
pub fn grouped(code: &str) -> String {
    let c: Vec<char> = code.chars().collect();
    c.chunks(3)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Now in UTC, as the taskbag writes times (`2026-10-25T08:30:00Z`).
fn now_utc() -> String {
    allomos_store::events::now_utc()
}

/// One paired device, as saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// UTC.
    pub paired_at: String,
    #[serde(default)]
    pub last_seen: Option<String>,
    /// SHA-256 of its token, hex; the token itself is never saved.
    token_sha256: String,
}

/// A device's request, while it waits for the person and then for its code.
#[derive(Debug, Clone)]
struct Ask {
    id: String,
    name: String,
    asked: Instant,
    answer: Answer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    Waiting,
    Refused,
    Approved {
        code: String,
        made: Instant,
        tries_left: u8,
    },
}

/// Where a request stands, as the device's page asks for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum Status {
    /// The person has not answered yet.
    Waiting,
    /// Approved: type the code the desktop app shows.
    Approved {
        tries_left: u8,
    },
    Refused,
    /// The code (or the wait) ran out, or the request is not known (the app was restarted): ask
    /// again.
    Expired,
}

/// Why a code was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum Refusal {
    /// Wrong; this many tries are left for this code.
    WrongCode { tries_left: u8 },
    /// Wrong for the last time: the desktop app now shows a new code.
    NewCode,
    /// Not approved (yet), refused, expired or unknown.
    NotApproved,
}

/// What the desktop app's dialog shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    pub asking: Vec<AskView>,
    pub devices: Vec<DeviceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AskView {
    pub id: String,
    pub name: String,
    /// `482 913 076` once approved.
    pub code: Option<String>,
    /// Seconds until the code stops working, once approved.
    pub expires_in: Option<u64>,
    pub tries_left: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub paired_at: String,
    pub last_seen: Option<String>,
}

struct Inner {
    asking: Vec<Ask>,
    devices: Vec<Device>,
    seen_saved: Option<Instant>,
}

/// The pairing state of the running app: requests in memory, paired devices on disk.
pub struct Pairing {
    /// `None` keeps devices in memory only (tests, or no settings folder).
    file: Option<PathBuf>,
    inner: Mutex<Inner>,
}

impl Pairing {
    /// The paired devices saved in `dir/lan/` (the app's settings folder), if any.
    pub fn load(dir: Option<&Path>) -> Pairing {
        let file = dir.map(|d| d.join(crate::lan::LAN_DIR).join(DEVICES_FILE));
        let devices = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Pairing {
            file,
            inner: Mutex::new(Inner {
                asking: Vec::new(),
                devices,
                seen_saved: None,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, devices: &[Device]) -> io::Result<()> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = serde_json::to_string_pretty(devices).map_err(io::Error::other)?;
        text.push('\n');
        crate::lan::write_private(file, &text)
    }

    /// Drop requests whose wait or code ran out.
    fn tidy(inner: &mut Inner, now: Instant) {
        inner.asking.retain(|a| match &a.answer {
            Answer::Approved { made, .. } => now.duration_since(*made) < CODE_LIFE,
            _ => now.duration_since(a.asked) < ASK_LIFE,
        });
    }

    /// A device asks to pair under `name`. Returns the request's ID for the device to ask after,
    /// or `None` when the name is empty or too long.
    pub fn ask(&self, name: &str) -> Option<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control)
        {
            return None;
        }
        let mut inner = self.lock();
        Self::tidy(&mut inner, Instant::now());
        if inner.asking.len() >= MAX_ASKING {
            inner.asking.remove(0);
        }
        let id = hex(&random_bytes::<16>());
        inner.asking.push(Ask {
            id: id.clone(),
            name: name.to_string(),
            asked: Instant::now(),
            answer: Answer::Waiting,
        });
        Some(id)
    }

    /// The person approves (a new code is made) or refuses a waiting request. False when there is
    /// no such waiting request.
    pub fn answer(&self, id: &str, approve: bool) -> bool {
        let mut inner = self.lock();
        Self::tidy(&mut inner, Instant::now());
        let Some(ask) = inner
            .asking
            .iter_mut()
            .find(|a| a.id == id && a.answer == Answer::Waiting)
        else {
            return false;
        };
        ask.answer = if approve {
            Answer::Approved {
                code: new_code(),
                made: Instant::now(),
                tries_left: TRIES,
            }
        } else {
            Answer::Refused
        };
        true
    }

    /// Where request `id` stands.
    pub fn status(&self, id: &str) -> Status {
        let mut inner = self.lock();
        Self::tidy(&mut inner, Instant::now());
        match inner.asking.iter().find(|a| a.id == id).map(|a| &a.answer) {
            Some(Answer::Waiting) => Status::Waiting,
            Some(Answer::Refused) => Status::Refused,
            Some(Answer::Approved { tries_left, .. }) => Status::Approved {
                tries_left: *tries_left,
            },
            None => Status::Expired,
        }
    }

    /// The device types `code` for request `id`. Right: the device is paired; returns it and its
    /// token (given out this once). Wrong: one try fewer, and after the last one a new code.
    pub fn confirm(&self, id: &str, code: &str) -> Result<(Device, String), Refusal> {
        let typed: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        let mut inner = self.lock();
        Self::tidy(&mut inner, Instant::now());
        let Some(n) = inner.asking.iter().position(|a| a.id == id) else {
            return Err(Refusal::NotApproved);
        };
        let Answer::Approved {
            code: right,
            made,
            tries_left,
        } = &mut inner.asking[n].answer
        else {
            return Err(Refusal::NotApproved);
        };
        if !same(&typed, right) {
            *tries_left -= 1;
            if *tries_left == 0 {
                *right = new_code();
                *made = Instant::now();
                *tries_left = TRIES;
                return Err(Refusal::NewCode);
            }
            return Err(Refusal::WrongCode {
                tries_left: *tries_left,
            });
        }
        let ask = inner.asking.remove(n);
        let token = hex(&random_bytes::<32>());
        let device = Device {
            id: hex(&random_bytes::<6>()),
            name: ask.name,
            paired_at: now_utc(),
            last_seen: None,
            token_sha256: sha256_hex(&token),
        };
        inner.devices.push(device.clone());
        let devices = inner.devices.clone();
        drop(inner);
        let _ = self.save(&devices);
        Ok((device, token))
    }

    /// The paired device whose token this is, noting that it was seen now; `None` for any other
    /// token (never paired, or removed).
    pub fn device_for(&self, token: &str) -> Option<Device> {
        let hash = sha256_hex(token);
        let mut inner = self.lock();
        let now = Instant::now();
        let device = inner
            .devices
            .iter_mut()
            .find(|d| same(&d.token_sha256, &hash))?;
        device.last_seen = Some(now_utc());
        let found = device.clone();
        if inner
            .seen_saved
            .is_none_or(|t| now.duration_since(t) >= SAVE_SEEN_EVERY)
        {
            inner.seen_saved = Some(now);
            let devices = inner.devices.clone();
            drop(inner);
            let _ = self.save(&devices);
        }
        Some(found)
    }

    /// Remove a paired device: from now on its token gives nothing. Returns it, or `None` when no
    /// device has this ID.
    pub fn remove(&self, id: &str) -> Option<Device> {
        let mut inner = self.lock();
        let n = inner.devices.iter().position(|d| d.id == id)?;
        let device = inner.devices.remove(n);
        let devices = inner.devices.clone();
        drop(inner);
        let _ = self.save(&devices);
        Some(device)
    }

    /// What the desktop app's dialog shows.
    pub fn view(&self) -> View {
        let mut inner = self.lock();
        let now = Instant::now();
        Self::tidy(&mut inner, now);
        View {
            asking: inner
                .asking
                .iter()
                .filter(|a| a.answer != Answer::Refused)
                .map(|a| {
                    let (code, expires_in, tries_left) = match &a.answer {
                        Answer::Approved {
                            code,
                            made,
                            tries_left,
                        } => (
                            Some(grouped(code)),
                            Some(
                                CODE_LIFE
                                    .saturating_sub(now.duration_since(*made))
                                    .as_secs(),
                            ),
                            Some(*tries_left),
                        ),
                        _ => (None, None, None),
                    };
                    AskView {
                        id: a.id.clone(),
                        name: a.name.clone(),
                        code,
                        expires_in,
                        tries_left,
                    }
                })
                .collect(),
            devices: inner
                .devices
                .iter()
                .map(|d| DeviceView {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    paired_at: d.paired_at.clone(),
                    last_seen: d.last_seen.clone(),
                })
                .collect(),
        }
    }

    #[cfg(test)]
    fn code_of(&self, id: &str) -> Option<String> {
        self.lock()
            .asking
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| match &a.answer {
                Answer::Approved { code, .. } => Some(code.clone()),
                _ => None,
            })
    }
}

/// Equal, comparing every byte, so the time taken does not tell how much of a guess was right.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("allomos-pairing-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn codes_are_nine_digits_in_three_groups() {
        for _ in 0..50 {
            let c = new_code();
            assert_eq!(c.len(), 9);
            assert!(c.chars().all(|d| d.is_ascii_digit()));
        }
        assert_eq!(grouped("482913076"), "482 913 076");
    }

    #[test]
    fn a_device_pairs_after_approval_with_the_code() {
        let dir = tmp("pair");
        let p = Pairing::load(Some(&dir));
        assert_eq!(p.ask("  "), None);
        assert_eq!(p.ask(&"x".repeat(NAME_MAX + 1)), None);
        let id = p.ask("Kitchen iPad").unwrap();
        assert_eq!(p.status(&id), Status::Waiting);
        assert_eq!(p.confirm(&id, "000000000"), Err(Refusal::NotApproved));
        assert_eq!(p.view().asking[0].code, None);
        assert!(p.answer(&id, true));
        assert!(!p.answer(&id, true), "only a waiting request is answered");
        assert_eq!(p.status(&id), Status::Approved { tries_left: TRIES });
        let code = p.code_of(&id).unwrap();
        assert_eq!(
            p.view().asking[0].code.as_deref(),
            Some(grouped(&code).as_str())
        );
        // Spaces may be left out or typed.
        let (device, token) = p.confirm(&id, &grouped(&code)).unwrap();
        assert_eq!(device.name, "Kitchen iPad");
        assert_eq!(p.status(&id), Status::Expired, "the code works once");
        assert_eq!(p.device_for(&token).unwrap().id, device.id);
        assert!(p.device_for("not-a-token").is_none());
        // Saved, without the token itself.
        let saved = std::fs::read_to_string(dir.join("lan/devices.json")).unwrap();
        assert!(saved.contains("Kitchen iPad") && !saved.contains(&token));
        let again = Pairing::load(Some(&dir));
        assert!(again.device_for(&token).is_some());
        // Removing it takes the access away at once.
        assert_eq!(again.remove(&device.id).unwrap().name, "Kitchen iPad");
        assert!(again.device_for(&token).is_none());
        assert!(Pairing::load(Some(&dir)).device_for(&token).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn five_wrong_tries_make_a_new_code() {
        let p = Pairing::load(None);
        let id = p.ask("Phone").unwrap();
        p.answer(&id, true);
        let first = p.code_of(&id).unwrap();
        let wrong = if first == "000000000" {
            "111111111"
        } else {
            "000000000"
        };
        for left in (1..TRIES).rev() {
            assert_eq!(
                p.confirm(&id, wrong),
                Err(Refusal::WrongCode { tries_left: left })
            );
        }
        assert_eq!(p.confirm(&id, wrong), Err(Refusal::NewCode));
        let second = p.code_of(&id).unwrap();
        if second != first {
            assert!(
                p.confirm(&id, &first).is_err(),
                "the old code no longer works"
            );
        }
        assert!(p.confirm(&id, &second).is_ok());
    }

    #[test]
    fn a_refused_request_gives_nothing() {
        let p = Pairing::load(None);
        let id = p.ask("Laptop").unwrap();
        assert!(p.answer(&id, false));
        assert_eq!(p.status(&id), Status::Refused);
        assert_eq!(p.confirm(&id, "123456789"), Err(Refusal::NotApproved));
        assert!(p.view().asking.is_empty());
        assert_eq!(p.status("unknown"), Status::Expired);
    }

    #[test]
    fn an_old_code_runs_out() {
        let p = Pairing::load(None);
        let id = p.ask("Phone").unwrap();
        p.answer(&id, true);
        if let Answer::Approved { made, .. } = &mut p.lock().asking[0].answer {
            *made = made
                .checked_sub(CODE_LIFE)
                .expect("the clock is past 15 minutes");
        }
        assert_eq!(p.status(&id), Status::Expired);
        assert!(p.view().asking.is_empty());
    }
}
