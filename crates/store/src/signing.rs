//! This computer's signing key, and signing the events it writes (Task-A01-gusto404, decision
//! 0021 § 3.1).
//!
//! Each computer has one Ed25519 key (RFC 8032), made on first use. The private key stays in the
//! operating system's key store: on macOS the login Keychain (item "Allomos signing key",
//! service `io.allomos.signing`), reached through the system's own `security` tool, so a rebuilt
//! `alm` is not asked again for access. Never in the taskbag. Where Allomos has no key store yet
//! (Windows, Linux), events are written unsigned, and the audit lists them as such. A test, or a
//! trial run with `ALLOMOS_SETTINGS_DIR`, keeps its key in that settings folder instead, never in
//! the person's Keychain.
//!
//! In an `allomos/8` taskbag every event gets `sig`: `{key, prev, value}`. `value` is the
//! signature over the event in canonical JSON without `sig` ([`allomos_core::canonical`]); `prev`
//! is the fingerprint of the same key's previous event in this taskbag, so the events one computer
//! wrote form a chain, and removing one from the middle breaks it. The public keys are in the
//! taskbag's `keys/<key ID>.json`; the first signed change on a computer adds its key there,
//! recorded as a signed `add key` event.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use allomos_core::Problem;
use allomos_core::canonical::{fingerprint, hex, signed_bytes, unhex};
use allomos_core::event::{Event, Kind};
use ring::rand::SystemRandom;
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const KEYS_DIR: &str = "keys";
/// The first format whose events are signed.
pub const SIGNED_FROM: u32 = 8;

const SERVICE: &str = "io.allomos.signing";
const ACCOUNT: &str = "this-computer";
const LABEL: &str = "Allomos signing key";
const KEY_FILE: &str = "signing-key.pk8";
const HEADS_FILE: &str = "signing-heads.json";

/// This computer's key.
pub(crate) struct Key {
    /// `k-` and the first 16 hexadecimal digits of the public key's SHA-256.
    pub id: String,
    pub public_hex: String,
    pair: Ed25519KeyPair,
}

impl Key {
    fn from_pkcs8(der: &[u8]) -> Option<Key> {
        let pair = Ed25519KeyPair::from_pkcs8(der).ok()?;
        let public = pair.public_key().as_ref().to_vec();
        Some(Key {
            id: key_id(&public),
            public_hex: hex(&public),
            pair,
        })
    }

    /// The signature over `value` without its `sig`, in hexadecimal.
    pub fn sign(&self, value: &Value) -> String {
        hex(self.pair.sign(&signed_bytes(value)).as_ref())
    }
}

/// A key's ID from its public key.
pub fn key_id(public: &[u8]) -> String {
    format!("k-{}", &allomos_core::canonical::sha256_hex(public)[..16])
}

/// Whether `sig_hex` is `public_hex`'s signature over `value` without its `sig`.
pub fn verify(public_hex: &str, value: &Value, sig_hex: &str) -> bool {
    let (Some(public), Some(sig)) = (unhex(public_hex), unhex(sig_hex)) else {
        return false;
    };
    UnparsedPublicKey::new(&ED25519, public)
        .verify(&signed_bytes(value), &sig)
        .is_ok()
}

/// Where this computer's private key is kept.
enum Store {
    /// A folder (tests, trial runs): `ALLOMOS_SETTINGS_DIR`.
    Folder(PathBuf),
    /// The macOS login Keychain.
    Keychain,
    /// No key store: events are written unsigned.
    None,
}

fn store() -> Store {
    // Unit tests run side by side in one process, some with a settings folder of their own for a
    // moment: they sign nothing (the signing tests are in `tests/signing.rs`).
    if cfg!(test) {
        return Store::None;
    }
    if let Some(dir) = std::env::var_os("ALLOMOS_SETTINGS_DIR") {
        return Store::Folder(PathBuf::from(dir));
    }
    // Tests never touch the person's Keychain.
    if cfg!(feature = "test-settings") {
        return Store::None;
    }
    if cfg!(target_os = "macos") {
        Store::Keychain
    } else {
        Store::None
    }
}

/// The key, loaded once per process.
static KEY: Mutex<Option<(PathBuf, Arc<Key>)>> = Mutex::new(None);

/// This computer's key, made and saved the first time; `None` without a key store (or when the
/// key store could not be read or written: then the change is written unsigned, never refused).
pub(crate) fn key() -> Option<Arc<Key>> {
    let place = match store() {
        Store::Folder(d) => d,
        Store::Keychain => PathBuf::from("keychain"),
        Store::None => return None,
    };
    let mut cached = KEY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((p, k)) = cached.as_ref() {
        if *p == place {
            return Some(k.clone());
        }
    }
    let key = Arc::new(load_or_make()?);
    *cached = Some((place, key.clone()));
    Some(key)
}

fn load_or_make() -> Option<Key> {
    if let Some(der) = read_private() {
        return Key::from_pkcs8(&der);
    }
    let der = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).ok()?;
    save_private(der.as_ref())?;
    // Read it back: another process may have saved one first.
    read_private().and_then(|d| Key::from_pkcs8(&d))
}

fn read_private() -> Option<Vec<u8>> {
    match store() {
        Store::Folder(dir) => unhex(fs::read_to_string(dir.join(KEY_FILE)).ok()?.trim()),
        Store::Keychain => {
            let out = Command::new("/usr/bin/security")
                .args(["find-generic-password", "-s", SERVICE, "-a", ACCOUNT, "-w"])
                .stderr(Stdio::null())
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            unhex(String::from_utf8(out.stdout).ok()?.trim())
        }
        Store::None => None,
    }
}

fn save_private(der: &[u8]) -> Option<()> {
    let text = hex(der);
    match store() {
        Store::Folder(dir) => {
            fs::create_dir_all(&dir).ok()?;
            let path = dir.join(KEY_FILE);
            if path.exists() {
                return Some(());
            }
            fs::write(&path, &text).ok()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
            Some(())
        }
        Store::Keychain => {
            // The key goes in on standard input, never on the command line, where other
            // programs could see it.
            use std::io::Write;
            let mut child = Command::new("/usr/bin/security")
                .arg("-i")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()?;
            let line = format!(
                "add-generic-password -s {SERVICE} -a {ACCOUNT} -l \"{LABEL}\" -w {text}\n"
            );
            child.stdin.take()?.write_all(line.as_bytes()).ok()?;
            child.wait().ok()?.success().then_some(())
        }
        Store::None => None,
    }
}

/// `keys/<key ID>.json`: one computer's public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey {
    pub key: String,
    /// Always `ed25519`.
    pub algorithm: String,
    /// The 32-byte public key, in hexadecimal.
    pub public_key: String,
    /// The pseudonym of this computer's person in the project, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// A name the person gave the computer; never guessed from the computer's own name, which
    /// often holds the person's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// When it was added, in UTC.
    pub added: String,
    /// When it was retired, if it was: a retired key stays, so its events can still be checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired: Option<String>,
    #[serde(flatten)]
    pub extra: allomos_core::Extra,
}

/// Every key in the taskbag's `keys/`, by ID, with the files that could not be read.
pub fn public_keys(root: &Path) -> (BTreeMap<String, PublicKey>, Vec<String>) {
    let mut keys = BTreeMap::new();
    let mut bad = Vec::new();
    for path in crate::sorted_entries(&root.join(KEYS_DIR)) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = format!(
            "{KEYS_DIR}/{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        match crate::read_text(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<PublicKey>(&t).ok())
        {
            Some(k) => {
                keys.insert(k.key.clone(), k);
            }
            None => bad.push(name),
        }
    }
    (keys, bad)
}

/// Whether `event` is to be signed: the taskbag is `allomos/8` or later, or the event makes it so
/// (`init`, `upgrade`, whose `format` change is written before `project.json`).
pub(crate) fn wanted(root: &Path, event: &Event) -> bool {
    let signed =
        |format: &str| allomos_core::format_version(format).is_some_and(|n| n >= SIGNED_FROM);
    let to_format = event.kind == Kind::Project
        && event.changes.iter().any(|c| {
            c.field == "format" && c.to.as_ref().and_then(Value::as_str).is_some_and(signed)
        });
    to_format || crate::versions::format_of(root).is_some_and(|f| signed(&f))
}

/// Sign `value`, the event to be written to `file` (`events/<name>.json`), with this computer's
/// key. Returns the key when the taskbag does not have it yet: once the event is written, the
/// caller adds it with [`add_key`]. Without a key store the event stays unsigned.
pub(crate) fn sign_event(root: &Path, value: &mut Value, file: &str) -> Option<Arc<Key>> {
    let key = key()?;
    let prev = head(root, &key.id);
    put_sig(&key, value, prev);
    remember_head(root, &key.id, file, &fingerprint(value));
    let missing = !root
        .join(KEYS_DIR)
        .join(format!("{}.json", key.id))
        .exists();
    missing.then_some(key)
}

fn put_sig(key: &Key, value: &mut Value, prev: Option<String>) {
    let signature = key.sign(value);
    if let Some(map) = value.as_object_mut() {
        map.insert(
            "sig".into(),
            json!({ "key": key.id, "prev": prev, "value": signature }),
        );
    }
}

/// Write `keys/<id>.json` and its signed `add key` event, made by whoever made `event` (the
/// first event this key signed in the taskbag, already written), which it follows in the chain.
pub(crate) fn add_key(
    root: &Path,
    project: Option<&str>,
    key: &Key,
    event: &Value,
) -> Result<(), Problem> {
    let owner = project.and_then(crate::identity::pseudonym);
    let public = PublicKey {
        key: key.id.clone(),
        algorithm: "ed25519".into(),
        public_key: key.public_hex.clone(),
        owner,
        device: None,
        added: crate::events::now_utc(),
        retired: None,
        extra: Default::default(),
    };
    let dir = root.join(KEYS_DIR);
    fs::create_dir_all(&dir).map_err(|e| crate::write_failed(&dir, e))?;
    let file = crate::events::new_file(root);
    let mut add = json!({
        "at": public.added,
        "via": event.get("via").cloned().unwrap_or(json!("alm")),
        "command": "add key",
        "kind": "project",
        "changes": [{ "field": "key", "to": serde_json::to_value(&public).unwrap_or_default() }],
    });
    if let Some(map) = add.as_object_mut() {
        for field in ["by", "agent", "tool", "actor", "for"] {
            if let Some(v) = event.get(field) {
                map.insert(field.into(), v.clone());
            }
        }
        if let Some(id) = project {
            map.insert("id".into(), json!(id));
        }
    }
    let prev = head(root, &key.id);
    put_sig(key, &mut add, prev);
    let events = root.join(crate::events::EVENTS_DIR);
    fs::create_dir_all(&events).map_err(|e| crate::write_failed(&events, e))?;
    crate::write_atomic(&root.join(&file), &crate::json_text(&add))?;
    remember_head(root, &key.id, &file, &fingerprint(&add));
    crate::write_atomic(
        &dir.join(format!("{}.json", key.id)),
        &crate::json_text(&public),
    )
}

/// The heads remembered on this computer: taskbag folder and key → last event file and its
/// fingerprint. Saves reading the whole history on every change; checked before use.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Heads {
    #[serde(default)]
    heads: BTreeMap<String, Head>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Head {
    file: String,
    fingerprint: String,
}

fn heads_file() -> Option<PathBuf> {
    match store() {
        Store::Folder(d) => Some(d.join(HEADS_FILE)),
        Store::Keychain => crate::settings::settings_dir().map(|d| d.join(HEADS_FILE)),
        Store::None => None,
    }
}

fn head_name(root: &Path, key: &str) -> String {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    format!("{}|{key}", root.display())
}

fn remember_head(root: &Path, key: &str, file: &str, fp: &str) {
    let Some(path) = heads_file() else { return };
    let mut heads: Heads = fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    heads.heads.insert(
        head_name(root, key),
        Head {
            file: file.to_string(),
            fingerprint: fp.to_string(),
        },
    );
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
        let tmp = dir.join(format!("{HEADS_FILE}.tmp-{}", std::process::id()));
        if fs::write(&tmp, crate::json_text(&heads)).is_ok() {
            let _ = crate::replace_file(&tmp, &path);
        }
    }
}

/// The fingerprint of `key`'s last event in the taskbag at `root`: the one remembered, when its
/// file is still there unchanged; else found in the history (the newest event of that key no
/// other of its events follows).
pub(crate) fn head(root: &Path, key: &str) -> Option<String> {
    let remembered = heads_file()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<Heads>(&t).ok())
        .and_then(|h| h.heads.get(&head_name(root, key)).cloned());
    if let Some(h) = remembered {
        let still = crate::read_text(&root.join(&h.file))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .is_some_and(|v| fingerprint(&v) == h.fingerprint);
        if still {
            return Some(h.fingerprint);
        }
    }
    crate::trail::tips(root).remove(key).map(|(_, fp)| fp)
}
