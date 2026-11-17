//! LAN viewing and pairing commands (`Allow devices on your network to view`, Task-A04-02-01,
//! Task-A04-02-02).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use allomos_core::Problem;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::windows::Windows;
use crate::{companion_change, lan, pairing};

/// "Allow devices on your network to view" (Task-A04-02-01): the server while it runs, and the
/// paired devices (Task-A04-02-02).
pub(crate) struct Lan {
    server: Mutex<Option<lan::Server>>,
    pairing: pairing::Pairing,
}

impl Lan {
    pub(crate) fn new() -> Lan {
        Lan {
            server: Mutex::new(None),
            pairing: pairing::Pairing::load(allomos_store::settings_dir().as_deref()),
        }
    }
}

/// Tells the "Devices on your network" dialog that a request came or changed.
const PAIRING_EVENT: &str = "lan:pairing";

/// Record a pairing (`pair`) or a removal (`unpair`) in the history of every taskbag a window shows
/// (approver, 2026-10-25, A04-02 decision 5): a project event whose `device` field (new, so older
/// Allomos versions read past it) names the device. A taskbag that cannot be written to now is
/// left out.
fn record_pairing(app: &AppHandle, command: &str, device: &str) {
    for root in lan::Source::open_taskbags(&AppSource(app.clone())) {
        let Ok(lock) = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))
        else {
            continue;
        };
        let Some(bag) = lock.load().taskbag else {
            continue;
        };
        let name = serde_json::Value::String(device.to_string());
        let (from, to) = if command == "pair" {
            (None, Some(name.clone()))
        } else {
            (Some(name.clone()), None)
        };
        let mut event = allomos_store::Actor::APP.event(
            command,
            allomos_core::event::Kind::Project,
            Some(&bag.project.id),
            vec![allomos_core::event::Change {
                field: "device".into(),
                from,
                to,
                extra: Default::default(),
            }],
        );
        event.extra.insert("device".into(), name);
        let _ = lock.record(&event, |_| Ok(()));
    }
}

/// What the web interface's server reads from the app: its built files, and the taskbags the
/// windows show.
struct AppSource(AppHandle);

impl lan::Source for AppSource {
    fn asset(&self, path: &str) -> Option<(Vec<u8>, String)> {
        let asset = self.0.asset_resolver().get(path.to_string())?;
        Some((asset.bytes().to_vec(), asset.mime_type().to_string()))
    }
    fn open_taskbags(&self) -> Vec<PathBuf> {
        self.0.state::<Windows>().open_taskbags()
    }
    fn pairing(&self) -> &pairing::Pairing {
        &self.0.state::<Lan>().inner().pairing
    }
    fn pairing_changed(&self) {
        let _ = self.0.emit(PAIRING_EVENT, ());
    }
    fn paired(&self, device: &pairing::Device) {
        record_pairing(&self.0, "pair", &device.name);
    }
    fn change(
        &self,
        root: &Path,
        by: &str,
        op: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, Problem> {
        companion_change(root, by, op, args)
    }
}

/// What the "Devices on your network" dialog shows.
#[derive(Serialize)]
pub(crate) struct LanStatus {
    on: bool,
    /// The port in use, and the address and its QR code (SVG) while on; the address is `None`
    /// when this computer has no network address.
    port: Option<u16>,
    url: Option<String>,
    qr_svg: Option<String>,
    /// The certificate's SHA-256 fingerprint, to compare with what the browser shows.
    fingerprint: Option<String>,
    default_port: u16,
    /// Why it could not start, in plain words, when it could not.
    error: Option<String>,
}

fn lan_status_of(lan: &Lan, error: Option<String>) -> LanStatus {
    let server = lan.server.lock().unwrap_or_else(|e| e.into_inner());
    let port = server.as_ref().map(|s| s.port);
    let url = port.and_then(|p| lan::local_address().map(|ip| lan::address(ip, p)));
    let fingerprint = server
        .as_ref()
        .and_then(|_| allomos_store::settings_dir())
        .and_then(|d| lan::identity(&d).ok())
        .map(|id| id.fingerprint);
    LanStatus {
        on: port.is_some(),
        port,
        qr_svg: url.as_deref().and_then(lan::qr_svg),
        url,
        fingerprint,
        default_port: lan::DEFAULT_PORT,
        error,
    }
}

/// Start the server (it was off), or say why it cannot.
pub(crate) fn start_lan(app: &AppHandle, lan: &Lan) -> Result<(), String> {
    let mut server = lan.server.lock().unwrap_or_else(|e| e.into_inner());
    if server.is_some() {
        return Ok(());
    }
    let dir =
        allomos_store::settings_dir().ok_or("This computer has no settings folder for Allomos.")?;
    let id = lan::identity(&dir).map_err(|e| format!("The certificate could not be made: {e}"))?;
    let source: std::sync::Arc<dyn lan::Source> = std::sync::Arc::new(AppSource(app.clone()));
    let started = lan::start(lan::DEFAULT_PORT, &id, source)
        .map_err(|e| format!("No port could be opened: {e}"))?;
    *server = Some(started);
    Ok(())
}

#[tauri::command]
pub(crate) fn lan_status(lan: tauri::State<Lan>) -> LanStatus {
    lan_status_of(&lan, None)
}

/// Turn "Allow devices on your network to view" on or off, and remember it. Off stops listening.
#[tauri::command]
pub(crate) fn set_lan_viewing(app: AppHandle, lan: tauri::State<Lan>, on: bool) -> LanStatus {
    let _ = allomos_store::save_lan_viewing(on);
    let error = if on {
        start_lan(&app, &lan).err()
    } else {
        if let Some(server) = lan.server.lock().unwrap_or_else(|e| e.into_inner()).take() {
            server.stop();
        }
        None
    };
    lan_status_of(&lan, error)
}

/// The requests waiting for an answer and the paired devices (Task-A04-02-02).
#[tauri::command]
pub(crate) fn lan_pairing(lan: tauri::State<Lan>) -> pairing::View {
    lan.pairing.view()
}

/// Approve (a code is shown) or refuse a device's request to pair.
#[tauri::command]
pub(crate) fn answer_pairing(lan: tauri::State<Lan>, id: String, approve: bool) -> pairing::View {
    lan.pairing.answer(&id, approve);
    lan.pairing.view()
}

/// Remove a paired device: its access ends at once, and the removal is recorded.
#[tauri::command]
pub(crate) fn remove_paired_device(
    app: AppHandle,
    lan: tauri::State<Lan>,
    id: String,
) -> pairing::View {
    if let Some(device) = lan.pairing.remove(&id) {
        record_pairing(&app, "unpair", &device.name);
    }
    lan.pairing.view()
}
