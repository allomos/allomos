//! "Allow devices on your network to view" (Task-A04-02-01, the first part of A04-02): while the
//! person has it on, the desktop app serves its web interface and a read-only interface to its data
//! over HTTPS on the local network, so a phone or another computer on the same Wi-Fi can open it.
//!
//! The approver's decisions (2026-10-25): self-signed HTTPS, the certificate made once and kept
//! (so a device trusts it once); off by default and remembered; a fixed default port
//! ([`DEFAULT_PORT`]), the next free one when it is taken. While it is off nothing listens.
//!
//! Only reading: every request other than GET or HEAD is refused. Until pairing (A04-02-02), the
//! data endpoints answer only this computer itself; another device gets the page and
//! `/api/hello`, nothing from a taskbag.

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
const LAN_DIR: &str = "lan";
const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
/// The most a request's head may take; a longer one is refused.
const MAX_HEAD: usize = 16 * 1024;

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
fn write_private(path: &Path, text: &str) -> io::Result<()> {
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

/// What the routes read from the app: a built file of the web interface (its bytes and type), and
/// the taskbags its windows show.
pub trait Source: Send + Sync {
    fn asset(&self, path: &str) -> Option<(Vec<u8>, String)>;
    fn open_taskbags(&self) -> Vec<PathBuf>;
}

/// The answer to one request. Reading only; data only for this computer until pairing exists.
pub fn route(req: &Request, source: &dyn Source) -> Response {
    if req.method != "GET" && req.method != "HEAD" {
        return Response::json(
            405,
            &serde_json::json!({ "error": "read_only", "message": "Devices on the network can only view. Nothing can be changed from here." }),
        );
    }
    if req.path == "/api/hello" {
        return Response::json(
            200,
            &serde_json::json!({ "app": "Allomos", "version": env!("CARGO_PKG_VERSION"), "pairing": false }),
        );
    }
    if let Some(rest) = req.path.strip_prefix("/api/") {
        // Pairing comes with A04-02-02; until then only this computer reads data.
        if !req.peer.is_loopback() {
            return Response::json(
                403,
                &serde_json::json!({ "error": "pairing_required", "message": "Pair this device with the Allomos desktop app first." }),
            );
        }
        let open = source.open_taskbags();
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
                _ => Response::json(404, &serde_json::json!({ "error": "not_open" })),
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
    let text = String::from_utf8_lossy(&head);
    let mut first = text.lines().next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let target = first.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let req = Request {
        method,
        path: percent_decode(path),
        query: query.to_string(),
        peer: peer.ip(),
    };
    let res = route(&req, source);
    answer(&mut tls, &res, req.method == "HEAD")
}

fn answer(out: &mut impl Write, res: &Response, head_only: bool) -> io::Result<()> {
    let reason = match res.status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
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
    }

    fn req(method: &str, target: &str, peer: IpAddr) -> Request {
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        Request {
            method: method.into(),
            path: path.into(),
            query: query.into(),
            peer,
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
        let f = Fake {
            index: true,
            open: vec![],
        };
        for m in ["POST", "PUT", "PATCH", "DELETE"] {
            let r = route(&req(m, "/api/taskbags", HERE), &f);
            assert_eq!(r.status, 405, "{m}");
        }
        assert_eq!(route(&req("HEAD", "/api/hello", LAN), &f).status, 200);
    }

    #[test]
    fn data_only_for_this_computer_until_pairing() {
        let root = tmp("bag");
        let f = Fake {
            index: true,
            open: vec![root.clone()],
        };
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
        let f = Fake {
            index: true,
            open: vec![],
        };
        assert_eq!(route(&req("GET", "/", LAN), &f).body, b"<html>app</html>");
        assert_eq!(
            route(&req("GET", "/some/page", LAN), &f).body,
            b"<html>app</html>"
        );
        assert_eq!(route(&req("GET", "/assets/app.js", LAN), &f).body, b"js");
        assert_eq!(route(&req("GET", "/assets/gone.js", LAN), &f).status, 404);
        // Without the built interface: a page that says so.
        let bare = Fake {
            index: false,
            open: vec![],
        };
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
        let source: Arc<dyn Source> = Arc::new(Fake {
            index: true,
            open: vec![],
        });
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
        let source: Arc<dyn Source> = Arc::new(Fake {
            index: true,
            open: vec![root],
        });
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
