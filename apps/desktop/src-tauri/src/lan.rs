//! "Allow devices on your network to view" (Task-A04-02-01, the first part of A04-02): while the
//! person has it on, the desktop app serves its web interface and a read-only interface to its data
//! over HTTPS on the local network, so a phone or another computer on the same Wi-Fi can open it.
//!
//! The approver's decisions (2026-10-25): self-signed HTTPS, the certificate made once and kept
//! (so a device trusts it once); off by default and remembered; a fixed default port
//! ([`DEFAULT_PORT`]), the next free one when it is taken. While it is off nothing listens.
//!
//! Only reading: every request other than GET or HEAD is refused, except the two that pair a device
//! (`POST /api/pair`, `POST /api/pair/code`; Task-A04-02-02, [`crate::pairing`]). The data
//! endpoints answer this computer itself and paired devices, which send their token as
//! `Authorization: Bearer <token>`; any other device gets the page and `/api/hello`, nothing from a
//! taskbag.

use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;

/// The port the server tries first (an AI decision of Task-A04-02-01: high, unassigned and easy to
/// recognise); when it is taken, the next of [`PORT_TRIES`] ports that is free.
pub const DEFAULT_PORT: u16 = 47470;
const PORT_TRIES: u16 = 20;
/// The folder, in the app's settings folder, holding the certificate and its key.
pub(crate) const LAN_DIR: &str = "lan";
const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
/// The most a request's head may take; a longer one is refused.
const MAX_HEAD: usize = 16 * 1024;
/// The most a request's body may take: pairing sends a name or a code; a change from the companion
/// page (Task-A04-02-04) may carry a long description.
const MAX_BODY: usize = 512 * 1024;

/// The certificate the server shows, as PEM, and its SHA-256 fingerprint as people read it
/// (`AB:CD:…`), shown beside the address so a person can compare it with what the browser shows.
pub struct Identity {
    cert_pem: String,
    key_pem: String,
    pub fingerprint: String,
}

/// This computer's certificate: the one saved in `dir/lan/`, else a new one, saved there. Made once
/// and kept (approver, 2026-10-25), so a device that trusted it does not have to again.
pub fn identity(dir: &Path) -> io::Result<Identity> {
    let folder = dir.join(LAN_DIR);
    let (cert_path, key_path) = (folder.join(CERT_FILE), folder.join(KEY_FILE));
    let (cert_pem, key_pem) = match (
        std::fs::read_to_string(&cert_path),
        std::fs::read_to_string(&key_path),
    ) {
        (Ok(c), Ok(k)) => (c, k),
        _ => {
            let made = rcgen::generate_simple_self_signed(vec![
                "allomos.local".to_string(),
                "localhost".to_string(),
            ])
            .map_err(io::Error::other)?;
            let (cert_pem, key_pem) = (made.cert.pem(), made.key_pair.serialize_pem());
            std::fs::create_dir_all(&folder)?;
            std::fs::write(&cert_path, &cert_pem)?;
            write_private(&key_path, &key_pem)?;
            (cert_pem, key_pem)
        }
    };
    let der = first_cert_der(&cert_pem)?;
    Ok(Identity {
        fingerprint: fingerprint(&der),
        cert_pem,
        key_pem,
    })
}

/// The key, readable by this user only where the system allows it.
pub(crate) fn write_private(path: &Path, text: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(text.as_bytes())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, text)
    }
}

fn first_cert_der(pem: &str) -> io::Result<rustls::pki_types::CertificateDer<'static>> {
    use rustls::pki_types::pem::PemObject;
    rustls::pki_types::CertificateDer::pem_slice_iter(pem.as_bytes())
        .next()
        .ok_or_else(|| io::Error::other("no certificate"))?
        .map_err(io::Error::other)
}

/// SHA-256 of the certificate, as two-digit hex groups joined by colons, as browsers show it.
fn fingerprint(der: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(der)
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn tls_config(id: &Identity) -> io::Result<Arc<rustls::ServerConfig>> {
    use rustls::pki_types::PrivateKeyDer;
    use rustls::pki_types::pem::PemObject;
    let cert = first_cert_der(&id.cert_pem)?;
    let key = PrivateKeyDer::from_pem_slice(id.key_pem.as_bytes()).map_err(io::Error::other)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(io::Error::other)?;
    Ok(Arc::new(config))
}

/// One request, as far as the server reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// The path, without the query.
    pub path: String,
    pub query: String,
    pub peer: IpAddr,
    /// The token after `Authorization: Bearer`, when sent.
    pub token: Option<String>,
    pub body: Vec<u8>,
}

impl Request {
    /// The query's value for `name`, percent-decoded.
    pub fn param(&self, name: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (k == name).then(|| percent_decode(v))
        })
    }
}

/// `a%20b+c` → `a b c`; a broken escape is kept as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
                _ => out.push(b'%'),
            },
            b'+' => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What the server sends back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(status: u16, value: &impl Serialize) -> Self {
        Response {
            status,
            content_type: "application/json; charset=utf-8".into(),
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }
}

/// What the routes read from the app: a built file of the web interface (its bytes and type), the
/// taskbags its windows show, and its pairing state, which it is told about when it changes.
pub trait Source: Send + Sync {
    fn asset(&self, path: &str) -> Option<(Vec<u8>, String)>;
    fn open_taskbags(&self) -> Vec<PathBuf>;
    fn pairing(&self) -> &crate::pairing::Pairing;
    /// A request came or changed: the desktop app shows it again.
    fn pairing_changed(&self);
    /// A device paired: recorded in every open taskbag's history (approver, 2026-10-25).
    fn paired(&self, device: &crate::pairing::Device);
    /// This computer's name, which the companion page shows as where the data comes from (R68).
    fn computer(&self) -> String {
        computer_name()
    }
    /// One change from a paired device's page (Task-A04-02-04), made as the desktop window makes
    /// it, under `by` ("will (iPhone)"). A source that cannot change anything refuses.
    fn change(
        &self,
        _root: &Path,
        _by: &str,
        op: &str,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value, allomos_core::Problem> {
        Err(allomos_core::Problem::CompanionDesktopOnly { op: op.to_string() })
    }
}

/// This computer's name: `COMPUTERNAME` on Windows, else what `hostname` prints, without a
/// `.local` ending; empty when neither tells.
pub fn computer_name() -> String {
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        return name;
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(|n| n.strip_suffix(".local").map(str::to_string).unwrap_or(n))
        .unwrap_or_default()
}

/// The JSON object in a request's body, if it is one.
fn body_json(req: &Request) -> Option<serde_json::Map<String, serde_json::Value>> {
    match serde_json::from_slice(&req.body).ok()? {
        serde_json::Value::Object(map) => Some(map),
        _ => None,
    }
}

/// `POST /api/pair` `{"name": …}` and `POST /api/pair/code` `{"request": …, "code": …}`
/// (Task-A04-02-02).
fn pair_route(req: &Request, source: &dyn Source) -> Response {
    let body = body_json(req).unwrap_or_default();
    let text = |name: &str| body.get(name).and_then(|v| v.as_str()).unwrap_or_default();
    let pairing = source.pairing();
    if req.path == "/api/pair" {
        return match pairing.ask(text("name")) {
            Some(id) => {
                source.pairing_changed();
                Response::json(200, &serde_json::json!({ "request": id }))
            }
            None => Response::json(
                400,
                &serde_json::json!({ "error": "name_missing", "max": crate::pairing::NAME_MAX }),
            ),
        };
    }
    match pairing.confirm(text("request"), text("code")) {
        Ok((device, token)) => {
            source.paired(&device);
            source.pairing_changed();
            Response::json(
                200,
                &serde_json::json!({ "token": token, "device": device.name }),
            )
        }
        Err(refusal) => {
            source.pairing_changed();
            Response::json(400, &refusal)
        }
    }
}

/// `POST /api/change` `{"root", "name", "op", "args"}` (Task-A04-02-04, R77): a paired device
/// changes a taskbag a window shows, through the desktop app's own rules. Every other device is
/// refused, and so is any folder a window does not show. A refusal carries the Problem, so the
/// page words it (and shows both versions when a title or description changed meanwhile).
fn change_route(req: &Request, source: &dyn Source) -> Response {
    let Some(device) = req
        .token
        .as_deref()
        .and_then(|t| source.pairing().device_for(t))
    else {
        return Response::json(
            403,
            &serde_json::json!({ "error": "pairing_required", "message": "Pair this device with the Allomos desktop app first." }),
        );
    };
    let body = body_json(req).unwrap_or_default();
    let text = |name: &str| body.get(name).and_then(|v| v.as_str()).unwrap_or_default();
    let root = PathBuf::from(text("root"));
    if !source.open_taskbags().iter().any(|o| o == &root) {
        return Response::json(404, &serde_json::json!({ "error": "not_open" }));
    }
    let name = text("name").trim();
    if name.is_empty() {
        return Response::json(
            400,
            &serde_json::json!({ "problem": allomos_core::Problem::HolderMissing }),
        );
    }
    let by = format!("{name} ({})", device.name);
    let args = body.get("args").cloned().unwrap_or(serde_json::Value::Null);
    match source.change(&root, &by, text("op"), args) {
        Ok(v) => Response::json(200, &serde_json::json!({ "ok": v })),
        Err(problem) => Response::json(400, &serde_json::json!({ "problem": problem })),
    }
}

/// The answer to one request. Reading, pairing, and changes from paired devices
/// (Task-A04-02-04); data only for this computer and paired devices.
pub fn route(req: &Request, source: &dyn Source) -> Response {
    if req.method == "POST" && (req.path == "/api/pair" || req.path == "/api/pair/code") {
        return pair_route(req, source);
    }
    if req.method == "POST" && req.path == "/api/change" {
        return change_route(req, source);
    }
    if req.method != "GET" && req.method != "HEAD" {
        return Response::json(
            405,
            &serde_json::json!({ "error": "read_only", "message": "Devices on the network can only view. Nothing can be changed from here." }),
        );
    }
    let device = req
        .token
        .as_deref()
        .and_then(|t| source.pairing().device_for(t));
    if req.path == "/api/hello" {
        return Response::json(
            200,
            &serde_json::json!({
                "app": "Allomos",
                "version": env!("CARGO_PKG_VERSION"),
                "pairing": true,
                "device": device.as_ref().map(|d| &d.name),
                // Which computer the data comes from (R68): the companion page names it.
                "computer": source.computer(),
            }),
        );
    }
    if req.path == "/api/pair" {
        let status = req
            .param("request")
            .map(|id| source.pairing().status(&id))
            .unwrap_or(crate::pairing::Status::Expired);
        return Response::json(200, &status);
    }
    if let Some(rest) = req.path.strip_prefix("/api/") {
        // Only this computer itself and paired devices read data (Task-A04-02-02).
        if !req.peer.is_loopback() && device.is_none() {
            return Response::json(
                403,
                &serde_json::json!({ "error": "pairing_required", "message": "Pair this device with the Allomos desktop app first." }),
            );
        }
        let open = source.open_taskbags();
        // Only a taskbag a window shows: never any folder a request names.
        let shown = || {
            req.param("root")
                .map(PathBuf::from)
                .filter(|r| open.iter().any(|o| o == r))
        };
        let not_open = || Response::json(404, &serde_json::json!({ "error": "not_open" }));
        return match rest {
            "taskbags" => {
                let list: Vec<_> = open
                    .iter()
                    .map(|root| {
                        let name = allomos_store::load(root)
                            .taskbag
                            .map(|b| b.project.name)
                            .unwrap_or_default();
                        serde_json::json!({ "root": root, "name": name })
                    })
                    .collect();
                Response::json(200, &list)
            }
            "taskbag" => match req.param("root").map(PathBuf::from) {
                // Only a taskbag a window shows: never any folder a request names.
                Some(root) if open.iter().any(|r| r == &root) => {
                    Response::json(200, &allomos_store::load(&root))
                }
                _ => not_open(),
            },
            // What the desktop window reads when it opens a taskbag, so the companion page shows
            // the same views (Task-A04-02-03).
            "opened" => match shown() {
                Some(root) => Response::json(200, &crate::open_taskbag(root)),
                None => not_open(),
            },
            // Changes when any of the taskbag's files change: the page reads it again only then.
            // Text, since a 64-bit number does not fit a JavaScript number.
            "stamp" => match shown() {
                Some(root) => Response::json(200, &allomos_store::stamp(&root).to_string()),
                None => not_open(),
            },
            "history" => match (shown(), req.param("id")) {
                (Some(root), Some(id)) => {
                    Response::json(200, &allomos_store::events::history(&root, &id))
                }
                _ => not_open(),
            },
            "folders" => match shown() {
                Some(root) => Response::json(
                    200,
                    &allomos_store::folders::list(&root).unwrap_or_default(),
                ),
                None => not_open(),
            },
            _ => Response::json(404, &serde_json::json!({ "error": "not_found" })),
        };
    }
    // The web interface: its files, and index.html for any page path (no extension).
    let file = req.path.trim_start_matches('/');
    let file = if file.is_empty() { "index.html" } else { file };
    if let Some((body, content_type)) = source.asset(file) {
        return Response {
            status: 200,
            content_type,
            body,
        };
    }
    let page = file == "index.html" || !file.rsplit('/').next().unwrap_or(file).contains('.');
    if page {
        if let Some((body, content_type)) = source.asset("index.html") {
            return Response {
                status: 200,
                content_type,
                body,
            };
        }
        // A development build without the built web interface (`npm --prefix apps/web run build`).
        return Response {
            status: 200,
            content_type: "text/html; charset=utf-8".into(),
            body: FALLBACK_PAGE.as_bytes().to_vec(),
        };
    }
    Response::json(404, &serde_json::json!({ "error": "not_found" }))
}

const FALLBACK_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Allomos</title><p style=\"font:16px system-ui;margin:24px\">Connected to Allomos. This development build has no built web interface to show; run <code>npm --prefix apps/web run build</code>.</p>";

/// The server while it runs. Dropping it (or [`Server::stop`]) stops listening.
pub struct Server {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    pub fn stop(mut self) {
        self.shut();
    }

    fn shut(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shut();
    }
}

/// Start listening on every network address of this computer, at `first` or the next free of
/// [`PORT_TRIES`] ports. Each connection is answered on a thread of its own.
pub fn start(first: u16, id: &Identity, source: Arc<dyn Source>) -> io::Result<Server> {
    let config = tls_config(id)?;
    let mut last = None;
    let listener = (0..PORT_TRIES)
        .filter_map(|i| first.checked_add(i))
        .find_map(|port| match TcpListener::bind(("0.0.0.0", port)) {
            Ok(l) => Some(l),
            Err(e) => {
                last = Some(e);
                None
            }
        })
        .ok_or_else(|| last.unwrap_or_else(|| io::Error::other("no free port")))?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let thread = std::thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    let (config, source) = (config.clone(), source.clone());
                    std::thread::spawn(move || {
                        let _ = serve(stream, peer, config, source.as_ref());
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
        // The listener is dropped here: the port is free again.
    });
    Ok(Server {
        port,
        stop,
        thread: Some(thread),
    })
}

/// One connection: TLS, one request, one answer, then closed.
fn serve(
    stream: TcpStream,
    peer: SocketAddr,
    config: Arc<rustls::ServerConfig>,
    source: &dyn Source,
) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let conn = rustls::ServerConnection::new(config).map_err(io::Error::other)?;
    let mut tls = rustls::StreamOwned::new(conn, stream);
    let mut head = Vec::new();
    let mut buf = [0u8; 2048];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = tls.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        head.extend_from_slice(&buf[..n]);
        if head.len() > MAX_HEAD {
            return answer(
                &mut tls,
                &Response::json(431, &serde_json::json!({ "error": "too_large" })),
                false,
            );
        }
    }
    let end = head
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(head.len(), |n| n + 4);
    let mut body = head.split_off(end);
    let text = String::from_utf8_lossy(&head);
    let header = |name: &str| {
        text.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    };
    let length: usize = header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY {
        return answer(
            &mut tls,
            &Response::json(413, &serde_json::json!({ "error": "too_large" })),
            false,
        );
    }
    while body.len() < length {
        let n = tls.read(&mut buf)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(length);
    let token = header("authorization").and_then(|v| {
        v.strip_prefix("Bearer ")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    });
    let mut first = text.lines().next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let target = first.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let req = Request {
        method,
        path: percent_decode(path),
        query: query.to_string(),
        peer: peer.ip(),
        token,
        body,
    };
    let res = route(&req, source);
    answer(&mut tls, &res, req.method == "HEAD")
}

fn answer(out: &mut impl Write, res: &Response, head_only: bool) -> io::Result<()> {
    let reason = match res.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        _ => "",
    };
    let mut text = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n",
        res.status,
        res.content_type,
        res.body.len()
    );
    if res.status == 405 {
        text.push_str("Allow: GET, HEAD\r\n");
    }
    text.push_str("\r\n");
    out.write_all(text.as_bytes())?;
    if !head_only {
        out.write_all(&res.body)?;
    }
    out.flush()
}

/// This computer's address on the local network: the one its traffic to another network would
/// leave from. Nothing is sent (a UDP socket only picks its route); `None` without a network.
pub fn local_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    // TEST-NET-1 (RFC 5737): never answered; connecting a UDP socket sends no packet.
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// The address a device opens, and its QR code as SVG.
pub fn address(ip: IpAddr, port: u16) -> String {
    format!("https://{ip}:{port}")
}

pub fn qr_svg(text: &str) -> Option<String> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    Some(
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(180, 180)
            .quiet_zone(true)
            .build(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    struct Fake {
        index: bool,
        open: Vec<PathBuf>,
        pairing: crate::pairing::Pairing,
        paired: std::sync::Mutex<Vec<String>>,
        changes: std::sync::Mutex<Vec<(PathBuf, String, String)>>,
    }
    fn fake(index: bool, open: Vec<PathBuf>) -> Fake {
        Fake {
            index,
            open,
            pairing: crate::pairing::Pairing::load(None),
            paired: Default::default(),
            changes: Default::default(),
        }
    }
    impl Source for Fake {
        fn asset(&self, path: &str) -> Option<(Vec<u8>, String)> {
            match path {
                "index.html" if self.index => {
                    Some((b"<html>app</html>".to_vec(), "text/html".into()))
                }
                "assets/app.js" => Some((b"js".to_vec(), "text/javascript".into())),
                _ => None,
            }
        }
        fn open_taskbags(&self) -> Vec<PathBuf> {
            self.open.clone()
        }
        fn pairing(&self) -> &crate::pairing::Pairing {
            &self.pairing
        }
        fn pairing_changed(&self) {}
        fn paired(&self, device: &crate::pairing::Device) {
            self.paired.lock().unwrap().push(device.name.clone());
        }
        fn change(
            &self,
            root: &Path,
            by: &str,
            op: &str,
            _args: serde_json::Value,
        ) -> Result<serde_json::Value, allomos_core::Problem> {
            self.changes
                .lock()
                .unwrap()
                .push((root.to_path_buf(), by.to_string(), op.to_string()));
            Ok(serde_json::json!({ "done": op }))
        }
        fn computer(&self) -> String {
            "studio".into()
        }
    }

    fn req(method: &str, target: &str, peer: IpAddr) -> Request {
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        Request {
            method: method.into(),
            path: path.into(),
            query: query.into(),
            peer,
            token: None,
            body: Vec::new(),
        }
    }
    const LAN: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
    const HERE: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    fn tmp(name: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-lan-{name}-{n}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn only_reading_is_answered() {
        let f = fake(true, vec![]);
        for m in ["POST", "PUT", "PATCH", "DELETE"] {
            let r = route(&req(m, "/api/taskbags", HERE), &f);
            assert_eq!(r.status, 405, "{m}");
        }
        assert_eq!(route(&req("HEAD", "/api/hello", LAN), &f).status, 200);
    }

    #[test]
    fn data_only_for_this_computer_until_pairing() {
        let root = tmp("bag");
        let f = fake(true, vec![root.clone()]);
        let hello = route(&req("GET", "/api/hello", LAN), &f);
        assert_eq!(hello.status, 200);
        for path in ["/api/taskbags", "/api/taskbag?root=x", "/api/anything"] {
            let r = route(&req("GET", path, LAN), &f);
            assert_eq!(r.status, 403, "{path}");
            assert!(String::from_utf8_lossy(&r.body).contains("pairing_required"));
        }
        assert_eq!(route(&req("GET", "/api/taskbags", HERE), &f).status, 200);
        // Only a taskbag a window shows, never another folder.
        let other = format!("/api/taskbag?root={}", percent_encode("/etc"));
        assert_eq!(route(&req("GET", &other, HERE), &f).status, 404);
        let mine = format!(
            "/api/taskbag?root={}",
            percent_encode(&root.display().to_string())
        );
        assert_eq!(route(&req("GET", &mine, HERE), &f).status, 200);
        // What the companion page reads (Task-A04-02-03), for a shown taskbag only, and the
        // computer's name.
        let enc = percent_encode(&root.display().to_string());
        for path in [
            format!("/api/opened?root={enc}"),
            format!("/api/stamp?root={enc}"),
            format!("/api/history?root={enc}&id=A01-01"),
            format!("/api/folders?root={enc}"),
        ] {
            assert_eq!(route(&req("GET", &path, HERE), &f).status, 200, "{path}");
            assert_eq!(route(&req("GET", &path, LAN), &f).status, 403, "{path}");
            let elsewhere = path.replace(&enc, &percent_encode("/etc"));
            assert_eq!(
                route(&req("GET", &elsewhere, HERE), &f).status,
                404,
                "{path}"
            );
        }
        let opened = route(&req("GET", &format!("/api/opened?root={enc}"), HERE), &f);
        assert!(String::from_utf8_lossy(&opened.body).contains("\"loaded\""));
        assert!(String::from_utf8_lossy(&hello.body).contains("\"computer\":\"studio\""));
    }

    /// Not a test: serves a taskbag and a built web interface for a person or agent to look at,
    /// as the desktop app would with that taskbag open (Task-A04-02-03). Pairing requests are
    /// approved at once and their code printed. `ALLOMOS_LAN_LOOK_ROOT=<taskbag>
    /// ALLOMOS_LAN_LOOK_DIST=apps/web/dist cargo test -p allomos-desktop serve_for_a_look --
    /// --ignored --nocapture`, for `ALLOMOS_LAN_LOOK_SECS` seconds (default 600).
    #[test]
    #[ignore]
    fn serve_for_a_look() {
        struct Disk {
            dist: PathBuf,
            root: PathBuf,
            pairing: crate::pairing::Pairing,
        }
        impl Source for Disk {
            fn asset(&self, path: &str) -> Option<(Vec<u8>, String)> {
                let bytes = std::fs::read(self.dist.join(path)).ok()?;
                let kind = match path.rsplit('.').next() {
                    Some("html") => "text/html",
                    Some("js") => "text/javascript",
                    Some("css") => "text/css",
                    Some("svg") => "image/svg+xml",
                    Some("png") => "image/png",
                    _ => "application/octet-stream",
                };
                Some((bytes, kind.into()))
            }
            fn open_taskbags(&self) -> Vec<PathBuf> {
                vec![self.root.clone()]
            }
            fn pairing(&self) -> &crate::pairing::Pairing {
                &self.pairing
            }
            fn pairing_changed(&self) {
                for ask in self.pairing.view().asking {
                    if ask.code.is_none() {
                        self.pairing.answer(&ask.id, true);
                    }
                }
                for ask in self.pairing.view().asking {
                    println!("pairing code for {}: {:?}", ask.name, ask.code);
                }
            }
            fn paired(&self, device: &crate::pairing::Device) {
                println!("paired {}", device.name);
            }
            // Changes from the page, as the desktop app makes them (Task-A04-02-04).
            fn change(
                &self,
                root: &Path,
                by: &str,
                op: &str,
                args: serde_json::Value,
            ) -> Result<serde_json::Value, allomos_core::Problem> {
                crate::companion_change(root, by, op, args)
            }
        }
        let env = |k: &str| std::env::var(k).ok();
        let (Some(root), Some(dist)) = (env("ALLOMOS_LAN_LOOK_ROOT"), env("ALLOMOS_LAN_LOOK_DIST"))
        else {
            return;
        };
        let secs = env("ALLOMOS_LAN_LOOK_SECS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);
        let id = identity(&tmp("look")).unwrap();
        let source = Arc::new(Disk {
            dist: dist.into(),
            root: root.into(),
            pairing: crate::pairing::Pairing::load(None),
        });
        let server = start(DEFAULT_PORT, &id, source).unwrap();
        println!("serving on https://127.0.0.1:{}", server.port);
        std::thread::sleep(Duration::from_secs(secs));
    }

    #[test]
    fn a_paired_device_reads_and_a_removed_one_does_not() {
        let root = tmp("paired");
        let f = fake(true, vec![root.clone()]);
        let post = |path: &str, body: serde_json::Value| {
            let mut r = req("POST", path, LAN);
            r.body = body.to_string().into_bytes();
            let res = route(&r, &f);
            let v: serde_json::Value = serde_json::from_slice(&res.body).unwrap();
            (res.status, v)
        };
        let get = |path: &str, token: Option<&str>| {
            let mut r = req("GET", path, LAN);
            r.token = token.map(String::from);
            let res = route(&r, &f);
            let v: serde_json::Value = serde_json::from_slice(&res.body).unwrap_or_default();
            (res.status, v)
        };
        assert_eq!(post("/api/pair", serde_json::json!({ "name": "" })).0, 400);
        let (status, v) = post("/api/pair", serde_json::json!({ "name": "Phone" }));
        assert_eq!(status, 200);
        let id = v["request"].as_str().unwrap().to_string();
        assert_eq!(
            get(&format!("/api/pair?request={id}"), None).1["status"],
            "waiting"
        );
        let asking = f.pairing.view().asking;
        assert_eq!(asking[0].name, "Phone");
        f.pairing.answer(&asking[0].id, true);
        assert_eq!(
            get(&format!("/api/pair?request={id}"), None).1["status"],
            "approved"
        );
        let code = f.pairing.view().asking[0].code.clone().unwrap();
        let (status, v) = post(
            "/api/pair/code",
            serde_json::json!({ "request": id, "code": "x" }),
        );
        assert_eq!(
            (status, &v["error"]),
            (400, &serde_json::json!("wrong_code"))
        );
        let (status, v) = post(
            "/api/pair/code",
            serde_json::json!({ "request": id, "code": code }),
        );
        assert_eq!(status, 200, "{v}");
        let token = v["token"].as_str().unwrap().to_string();
        assert_eq!(*f.paired.lock().unwrap(), ["Phone"]);
        // Data now, with the token only.
        assert_eq!(get("/api/taskbags", None).0, 403);
        assert_eq!(get("/api/taskbags", Some("wrong")).0, 403);
        assert_eq!(get("/api/taskbags", Some(&token)).0, 200);
        assert_eq!(get("/api/hello", Some(&token)).1["device"], "Phone");
        // Removed: nothing any more.
        let device = f.pairing.view().devices[0].id.clone();
        f.pairing.remove(&device);
        assert_eq!(get("/api/taskbags", Some(&token)).0, 403);
        assert_eq!(
            get("/api/hello", Some(&token)).1["device"],
            serde_json::Value::Null
        );
    }

    /// Task-A04-02-04: only a paired device changes, only a taskbag a window shows, under
    /// "name (device)"; anything else is refused before the source is asked.
    #[test]
    fn only_paired_devices_change_and_their_name_is_recorded() {
        let root = tmp("change");
        let f = fake(true, vec![root.clone()]);
        let id = f.pairing.ask("Phone").unwrap();
        f.pairing.answer(&id, true);
        let code = f.pairing.view().asking[0].code.clone().unwrap();
        let (_, token) = f.pairing.confirm(&id, &code).unwrap();
        let post = |token: Option<&str>, body: serde_json::Value| {
            let mut r = req("POST", "/api/change", LAN);
            r.token = token.map(String::from);
            r.body = body.to_string().into_bytes();
            let res = route(&r, &f);
            let v: serde_json::Value = serde_json::from_slice(&res.body).unwrap();
            (res.status, v)
        };
        let ok = serde_json::json!({ "root": root, "name": "Will", "op": "edit_task", "args": { "id": "A01-01" } });
        assert_eq!(post(None, ok.clone()).0, 403);
        assert_eq!(post(Some("wrong"), ok.clone()).0, 403);
        let other = serde_json::json!({ "root": "/etc", "name": "Will", "op": "edit_task" });
        assert_eq!(post(Some(&token), other).0, 404);
        let nameless = serde_json::json!({ "root": root, "name": " ", "op": "edit_task" });
        let (status, v) = post(Some(&token), nameless);
        assert_eq!(
            (status, &v["problem"]["code"]),
            (400, &serde_json::json!("holder_missing"))
        );
        assert!(f.changes.lock().unwrap().is_empty());
        let (status, v) = post(Some(&token), ok);
        assert_eq!(status, 200, "{v}");
        let changes = f.changes.lock().unwrap();
        assert_eq!(changes[0].1, "Will (Phone)");
        assert_eq!(changes[0].2, "edit_task");
    }

    fn percent_encode(s: &str) -> String {
        s.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    }

    #[test]
    fn the_web_interface_is_served_with_index_for_pages() {
        let f = fake(true, vec![]);
        assert_eq!(route(&req("GET", "/", LAN), &f).body, b"<html>app</html>");
        assert_eq!(
            route(&req("GET", "/some/page", LAN), &f).body,
            b"<html>app</html>"
        );
        assert_eq!(route(&req("GET", "/assets/app.js", LAN), &f).body, b"js");
        assert_eq!(route(&req("GET", "/assets/gone.js", LAN), &f).status, 404);
        // Without the built interface: a page that says so.
        let bare = fake(false, vec![]);
        let r = route(&req("GET", "/", LAN), &bare);
        assert!(String::from_utf8_lossy(&r.body).contains("npm --prefix apps/web run build"));
    }

    #[test]
    fn queries_are_decoded() {
        let r = req("GET", "/x?root=%2FUsers%2Fa+b&n=1", HERE);
        assert_eq!(r.param("root").as_deref(), Some("/Users/a b"));
        assert_eq!(r.param("n").as_deref(), Some("1"));
        assert_eq!(r.param("missing"), None);
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    /// Made once, then the same certificate every time (approver, 2026-10-25).
    #[test]
    fn the_certificate_is_made_once_and_kept() {
        let dir = tmp("cert");
        let a = identity(&dir).unwrap();
        let b = identity(&dir).unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.fingerprint.split(':').count(), 32);
        assert!(dir.join("lan/cert.pem").is_file() && dir.join("lan/key.pem").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("lan/key.pem"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "the key is this user's only");
        }
    }

    /// Listening while on, at the first free port; the port is free again once stopped.
    #[test]
    fn it_listens_only_while_on_and_moves_off_a_taken_port() {
        let id = identity(&tmp("listen")).unwrap();
        let source: Arc<dyn Source> = Arc::new(fake(true, vec![]));
        let first = 47_900;
        let server = start(first, &id, source.clone()).unwrap();
        let port = server.port;
        assert!((first..first + PORT_TRIES).contains(&port));
        assert!(TcpStream::connect(("127.0.0.1", port)).is_ok());
        // Taken: the next server moves to another port.
        let second = start(port, &id, source).unwrap();
        assert_ne!(second.port, port);
        second.stop();
        server.stop();
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            TcpStream::connect(("127.0.0.1", port)).is_err(),
            "nothing listens once off"
        );
    }

    /// End to end over real TLS, with `curl` (skipped where there is none): this computer reads,
    /// a request through the network address gets no data, nothing can be changed.
    #[test]
    fn over_https_with_curl() {
        if std::process::Command::new("curl")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let root = tmp("e2e-bag");
        let id = identity(&tmp("e2e")).unwrap();
        let shared = Arc::new(fake(true, vec![root]));
        let source: Arc<dyn Source> = shared.clone();
        let server = start(47_950, &id, source).unwrap();
        let curl = |args: &[&str]| {
            let out = std::process::Command::new("curl")
                .args(["-sk", "--max-time", "5", "-w", "\n%{http_code}"])
                .args(args)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        let here = format!("https://127.0.0.1:{}", server.port);
        let hello = curl(&[&format!("{here}/api/hello")]);
        assert!(
            hello.contains("\"app\":\"Allomos\"") && hello.ends_with("200"),
            "{hello}"
        );
        assert!(curl(&[&format!("{here}/api/taskbags")]).ends_with("200"));
        assert!(curl(&[&format!("{here}/")]).contains("<html>app</html>"));
        assert!(curl(&["-X", "POST", &format!("{here}/api/taskbags")]).ends_with("405"));
        if let Some(ip) = local_address() {
            let lan = address(ip, server.port);
            let r = curl(&[&format!("{lan}/api/taskbags")]);
            assert!(r.contains("pairing_required") && r.ends_with("403"), "{r}");
            assert!(curl(&[&format!("{lan}/api/hello")]).ends_with("200"));
            // Pairing over the network (Task-A04-02-02): a body and a token header are read.
            let body = |text: &str| text.rsplit_once('\n').unwrap().0.to_string();
            let json =
                |text: &str| -> serde_json::Value { serde_json::from_str(&body(text)).unwrap() };
            let post = |path: &str, data: &str| {
                curl(&[
                    "-X",
                    "POST",
                    "-H",
                    "Content-Type: application/json",
                    "--data",
                    data,
                    &format!("{lan}{path}"),
                ])
            };
            let asked = json(&post("/api/pair", r#"{"name":"Curl"}"#));
            let request = asked["request"].as_str().unwrap();
            let pending = shared.pairing.view().asking[0].clone();
            shared.pairing.answer(&pending.id, true);
            let code = shared.pairing.view().asking[0].code.clone().unwrap();
            let paired = post(
                "/api/pair/code",
                &format!(r#"{{"request":"{request}","code":"{code}"}}"#),
            );
            assert!(paired.ends_with("200"), "{paired}");
            let token = json(&paired)["token"].as_str().unwrap().to_string();
            let auth = format!("Authorization: Bearer {token}");
            assert!(curl(&["-H", &auth, &format!("{lan}/api/taskbags")]).ends_with("200"));
            let big = std::env::temp_dir().join(format!("allomos-big-{}", std::process::id()));
            std::fs::write(&big, format!(r#"{{"name":"{}"}}"#, "x".repeat(MAX_BODY))).unwrap();
            let r = curl(&[
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                &format!("@{}", big.display()),
                &format!("{lan}/api/pair"),
            ]);
            let _ = std::fs::remove_file(&big);
            assert!(r.ends_with("413"), "{r}");
        }
        server.stop();
    }

    #[test]
    fn the_address_and_its_qr_code() {
        let url = address(LAN, DEFAULT_PORT);
        assert_eq!(url, "https://192.168.1.20:47470");
        assert!(qr_svg(&url).unwrap().starts_with("<?xml"));
    }
}
