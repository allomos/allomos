//! RFC 3161 time-stamping (Task-A01-wood982, decision 0021 § 3.3): ask an outside service to
//! countersign a checkpoint's fingerprint, and read its receipt (a time-stamp token) back.
//!
//! Only the fingerprint (a SHA-256 hash) and a random number leave the computer. The request is
//! sent with the system's `curl` (macOS, Windows 10 and later, and Linux have it), as signing uses
//! the system's own tools: `alm` gains no HTTP library.
//!
//! The token is kept as the service sent it (DER, in base64), so anyone can check it later
//! without Allomos (`openssl ts -verify`). What Allomos reads from it with hashing alone: the
//! fingerprint it countersigned, its time and serial number, the random number, whether its
//! signed attributes name the digest of what it says (the token is whole), and when the
//! service's certificate expires. It does not check the service's signature itself (FreeTSA signs
//! with ECDSA P-384 and SHA-512, which `ring` does not offer); `alm audit` names the command that
//! does, following "`alm` tells; the agent checks".

use std::io::Write;
use std::process::{Command, Stdio};

use ring::rand::{SecureRandom, SystemRandom};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha384, Sha512};

// DER object identifiers, as their encoded bytes.
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const OID_SHA384: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
const OID_SHA512: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03];
const OID_SIGNED_DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
const OID_TST_INFO: &[u8] = &[
    0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x01, 0x04,
];
const OID_MESSAGE_DIGEST: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x04];

/// How long one request may take before the service counts as unreachable.
const TIMEOUT_SECONDS: &str = "20";
/// How long reaching the service may take: a checkpoint made by itself (Task-A01-bake791) must
/// not hold a command up long on a poor network; the fingerprint waits instead.
const CONNECT_SECONDS: &str = "5";

// --- A small DER reader: only what a time-stamp token needs. ---

#[derive(Clone, Copy)]
struct Tlv<'a> {
    tag: u8,
    /// The value's bytes.
    value: &'a [u8],
    /// The whole element, tag and length included.
    whole: &'a [u8],
}

/// The first element of `input` and what follows it.
fn next(input: &[u8]) -> Option<(Tlv<'_>, &[u8])> {
    let tag = *input.first()?;
    if tag & 0x1f == 0x1f {
        return None; // high tag numbers: not used in these structures
    }
    let first = *input.get(1)?;
    let (len, head) = if first < 0x80 {
        (first as usize, 2)
    } else {
        let n = (first & 0x7f) as usize;
        if n == 0 || n > 4 {
            return None;
        }
        let mut len = 0usize;
        for b in input.get(2..2 + n)? {
            len = (len << 8) | *b as usize;
        }
        (len, 2 + n)
    };
    let end = head.checked_add(len)?;
    let value = input.get(head..end)?;
    Some((
        Tlv {
            tag,
            value,
            whole: &input[..end],
        },
        &input[end..],
    ))
}

/// Every element inside a constructed value.
fn children(value: &[u8]) -> Option<Vec<Tlv<'_>>> {
    let mut out = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        let (t, r) = next(rest)?;
        out.push(t);
        rest = r;
    }
    Some(out)
}

fn one(input: &[u8], tag: u8) -> Option<Tlv<'_>> {
    let (t, _) = next(input)?;
    (t.tag == tag).then_some(t)
}

const SEQUENCE: u8 = 0x30;
const INTEGER: u8 = 0x02;
const OCTET_STRING: u8 = 0x04;
const OID: u8 = 0x06;
const UTC_TIME: u8 = 0x17;
const GENERALIZED_TIME: u8 = 0x18;
const CONTEXT_0: u8 = 0xa0;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// An INTEGER's value as hex, without the sign byte DER adds before a high first byte.
fn int_hex(value: &[u8]) -> String {
    let v = match value {
        [0, rest @ ..] if !rest.is_empty() => rest,
        v => v,
    };
    hex(v)
}

/// A DER time as `YYYY-MM-DDTHH:MM:SSZ` (fractions of a second dropped).
fn time_of(t: &Tlv) -> Option<String> {
    let s = std::str::from_utf8(t.value).ok()?;
    let full = match t.tag {
        GENERALIZED_TIME => s.to_string(),
        UTC_TIME => {
            let yy: u32 = s.get(..2)?.parse().ok()?;
            format!("{}{s}", if yy >= 50 { "19" } else { "20" })
        }
        _ => return None,
    };
    let d = full.as_bytes();
    if d.len() < 15 || !d[..14].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let p = |a: usize, b: usize| &full[a..b];
    Some(format!(
        "{}-{}-{}T{}:{}:{}Z",
        p(0, 4),
        p(4, 6),
        p(6, 8),
        p(8, 10),
        p(10, 12),
        p(12, 14)
    ))
}

fn digest(oid: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    Some(match oid {
        OID_SHA256 => Sha256::digest(data).to_vec(),
        OID_SHA384 => Sha384::digest(data).to_vec(),
        OID_SHA512 => Sha512::digest(data).to_vec(),
        _ => return None,
    })
}

fn hash_name(oid: &[u8]) -> String {
    match oid {
        OID_SHA256 => "sha256".into(),
        OID_SHA384 => "sha384".into(),
        OID_SHA512 => "sha512".into(),
        other => format!("oid {}", hex(other)),
    }
}

// --- base64, for keeping the token in JSON. ---

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | ((*b as u32) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut n = 0u32;
    let mut bits = 0;
    for c in text
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
    {
        let v = B64.iter().position(|b| *b == c)? as u32;
        n = (n << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((n >> bits) as u8);
        }
    }
    Some(out)
}

// --- The request. ---

/// A TimeStampReq (RFC 3161 § 2.4.1) for a SHA-256 fingerprint, with a nonce, asking for the
/// service's certificate in the reply.
pub fn request(sha256: &[u8], nonce: &[u8]) -> Vec<u8> {
    fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        let n = value.len();
        if n < 0x80 {
            out.push(n as u8);
        } else {
            let bytes: Vec<u8> = n
                .to_be_bytes()
                .into_iter()
                .skip_while(|b| *b == 0)
                .collect();
            out.push(0x80 | bytes.len() as u8);
            out.extend(bytes);
        }
        out.extend_from_slice(value);
        out
    }
    let algorithm = [tlv(OID, OID_SHA256), vec![0x05, 0x00]].concat();
    let imprint = [tlv(SEQUENCE, &algorithm), tlv(OCTET_STRING, sha256)].concat();
    let body = [
        tlv(INTEGER, &[1]),
        tlv(SEQUENCE, &imprint),
        tlv(INTEGER, nonce),
        vec![0x01, 0x01, 0xff], // certReq TRUE
    ]
    .concat();
    tlv(SEQUENCE, &body)
}

/// Eight random bytes, as a positive DER INTEGER's value (first byte neither 0 nor ≥ 0x80).
pub fn nonce() -> [u8; 8] {
    let mut n = [0u8; 8];
    if cfg!(any(test, feature = "test-settings")) && std::env::var_os("ALLOMOS_TSA_REPLY").is_some()
    {
        // The test reply was made for this nonce (tests/fixtures/tsa-reply.der).
        return [0x41, 2, 3, 4, 5, 6, 7, 8];
    }
    let _ = SystemRandom::new().fill(&mut n);
    n[0] = (n[0] & 0x7f) | 0x40;
    n
}

// --- The token. ---

/// What a time-stamp token says, read with hashing alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    /// The hash it countersigned, as hex, and with what (`sha256`).
    pub imprint: String,
    pub hash: String,
    /// When the service says it countersigned it (UTC).
    pub time: String,
    pub serial: String,
    /// The random number, as hex, when the token has one.
    pub nonce: Option<String>,
    /// The token's signed attributes name the digest of what it says: it is whole.
    pub whole: bool,
    /// When the signing certificate expires (UTC), when the token carries it.
    pub certificate_expires: Option<String>,
    /// The signing certificate's subject, its readable parts (`CN=…, O=…`), when carried.
    pub signer: Option<String>,
}

/// Read a time-stamp token (a CMS ContentInfo holding SignedData over a TSTInfo).
pub fn read_token(der: &[u8]) -> Option<Token> {
    let info = children(one(der, SEQUENCE)?.value)?;
    if info.first()?.tag != OID || info[0].value != OID_SIGNED_DATA {
        return None;
    }
    let signed = one(info.get(1).filter(|t| t.tag == CONTEXT_0)?.value, SEQUENCE)?;
    let parts = children(signed.value)?;
    // version, digestAlgorithms, encapContentInfo, [0] certificates?, [1] crls?, signerInfos
    let encap = children(parts.get(2)?.value)?;
    if encap.first()?.value != OID_TST_INFO {
        return None;
    }
    let content = one(encap.get(1)?.value, OCTET_STRING)?.value;
    let tst = children(one(content, SEQUENCE)?.value)?;
    // version, policy, messageImprint, serialNumber, genTime, accuracy?, ordering?, nonce?, …
    let imprint = children(tst.get(2)?.value)?;
    let algorithm = children(imprint.first()?.value)?;
    let hash_oid = algorithm.first()?.value;
    let hashed = imprint.get(1)?.value;
    let serial = int_hex(tst.get(3).filter(|t| t.tag == INTEGER)?.value);
    let time = time_of(tst.get(4)?)?;
    let nonce = tst[5..]
        .iter()
        .find(|t| t.tag == INTEGER)
        .map(|t| int_hex(t.value));

    let certificates = parts
        .iter()
        .find(|t| t.tag == CONTEXT_0)
        .and_then(|t| children(t.value));
    let signer_info = children(parts.last()?.value)?;
    let signer_info = children(signer_info.first()?.value)?;
    // version, sid, digestAlgorithm, [0] signedAttrs, signatureAlgorithm, signature
    let sid_serial =
        children(signer_info.get(1)?.value).and_then(|sid| sid.get(1).map(|s| int_hex(s.value)));
    let attr_hash = children(signer_info.get(2)?.value)?.first()?.value;
    let attrs = signer_info.get(3).filter(|t| t.tag == CONTEXT_0);
    let named = attrs.and_then(|a| {
        children(a.value)?.into_iter().find_map(|attr| {
            let kv = children(attr.value)?;
            (kv.first()?.value == OID_MESSAGE_DIGEST)
                .then(|| Some(one(children(kv.get(1)?.value)?.first()?.whole, OCTET_STRING)?.value))
                .flatten()
        })
    });
    let whole = named.is_some_and(|n| digest(attr_hash, content).is_some_and(|d| d == n));

    let (certificate_expires, signer) = certificates
        .map(|certs| {
            let read: Vec<(String, Option<String>, Option<String>)> =
                certs.iter().filter_map(cert_facts).collect();
            let chosen = read
                .iter()
                .find(|(serial, _, _)| Some(serial) == sid_serial.as_ref())
                .or_else(|| read.first());
            chosen
                .map(|(_, until, subject)| (until.clone(), subject.clone()))
                .unwrap_or((None, None))
        })
        .unwrap_or((None, None));

    Some(Token {
        imprint: hex(hashed),
        hash: hash_name(hash_oid),
        time,
        serial,
        nonce,
        whole,
        certificate_expires,
        signer,
    })
}

/// A certificate's serial, notAfter and readable subject.
fn cert_facts(cert: &Tlv) -> Option<(String, Option<String>, Option<String>)> {
    let tbs = children(children(cert.value)?.first()?.value)?;
    let skip = usize::from(tbs.first()?.tag == CONTEXT_0);
    let serial = int_hex(tbs.get(skip)?.value);
    let validity = children(tbs.get(skip + 3)?.value)?;
    let until = validity.get(1).and_then(time_of);
    let subject = children(tbs.get(skip + 4)?.value).map(|rdns| {
        rdns.iter()
            .filter_map(|rdn| {
                let atv = children(children(rdn.value)?.first()?.value)?;
                let name = match atv.first()?.value {
                    [0x55, 0x04, 0x03] => "CN",
                    [0x55, 0x04, 0x0a] => "O",
                    [0x55, 0x04, 0x06] => "C",
                    _ => return None,
                };
                Some(format!(
                    "{name}={}",
                    String::from_utf8_lossy(atv.get(1)?.value)
                ))
            })
            .collect::<Vec<_>>()
            .join(", ")
    });
    Some((serial, until, subject.filter(|s| !s.is_empty())))
}

/// Why a service gave no receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Not reached (offline, a timeout, no `curl`): try again later.
    Unreachable(String),
    /// Reached, but its answer is not a granted receipt for what was asked.
    Answer(String),
}

/// Read a TimeStampResp: the token's DER when granted (status 0 or 1).
pub fn read_response(der: &[u8]) -> Result<Vec<u8>, String> {
    let parts = one(der, SEQUENCE)
        .and_then(|t| children(t.value))
        .ok_or("not a time-stamp response")?;
    let status = parts
        .first()
        .and_then(|s| children(s.value))
        .and_then(|s| {
            s.first()
                .filter(|t| t.tag == INTEGER)
                .map(|t| t.value.to_vec())
        })
        .ok_or("no status")?;
    if status != [0] && status != [1] {
        return Err(format!("the service refused it (status {})", hex(&status)));
    }
    parts
        .get(1)
        .map(|t| t.whole.to_vec())
        .ok_or_else(|| "granted, but no token".to_string())
}

/// A receipt, as a checkpoint keeps it in `receipts`.
pub fn receipt(service: &str, sent_at: &str, token_der: &[u8], t: &Token) -> Value {
    json!({
        "service": service,
        "sent_at": sent_at,
        "time": t.time,
        "serial": t.serial,
        "hash": t.hash,
        "imprint": t.imprint,
        "certificate_expires": t.certificate_expires,
        "token": base64(token_der),
    })
}

/// A kept receipt's token, read again from its DER (never from the summary beside it).
pub fn token_of(receipt: &Value) -> Option<Token> {
    let der = unbase64(receipt.get("token")?.as_str()?)?;
    read_token(&der)
}

/// Ask `service` to countersign the SHA-256 fingerprint `sha256` (hex). Returns the receipt, or
/// why there is none. Only the hash and a random number are sent.
pub fn countersign(service: &str, sha256: &str, sent_at: &str) -> Result<Value, Refused> {
    let hash = unhex(sha256)
        .filter(|h| h.len() == 32)
        .ok_or_else(|| Refused::Answer("the fingerprint is not a SHA-256 hash".into()))?;
    let nonce = nonce();
    let body = post(service, &request(&hash, &nonce))?;
    let token = read_response(&body).map_err(Refused::Answer)?;
    let t =
        read_token(&token).ok_or_else(|| Refused::Answer("its token could not be read".into()))?;
    if t.imprint != sha256 || t.hash != "sha256" {
        return Err(Refused::Answer(
            "its token countersigns a different fingerprint".into(),
        ));
    }
    if t.nonce.as_deref() != Some(int_hex(&nonce).as_str()) {
        return Err(Refused::Answer(
            "its token does not carry the random number sent".into(),
        ));
    }
    Ok(receipt(service, sent_at, &token, &t))
}

/// POST a request with `curl`; a test may set `ALLOMOS_TSA_REPLY` to a file whose bytes stand in
/// for the service's answer (or to `offline`).
fn post(service: &str, body: &[u8]) -> Result<Vec<u8>, Refused> {
    if cfg!(any(test, feature = "test-settings")) {
        if let Some(reply) = std::env::var_os("ALLOMOS_TSA_REPLY") {
            if reply == "offline" {
                return Err(Refused::Unreachable("offline (test)".into()));
            }
            return std::fs::read(&reply).map_err(|e| Refused::Unreachable(e.to_string()));
        }
    }
    let mut child = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            TIMEOUT_SECONDS,
            "--connect-timeout",
            CONNECT_SECONDS,
            "--header",
            "Content-Type: application/timestamp-query",
            "--data-binary",
            "@-",
            service,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Refused::Unreachable(format!("curl could not be started: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(body);
    }
    let out = child
        .wait_with_output()
        .map_err(|e| Refused::Unreachable(e.to_string()))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // curl's exit 22: the server answered with an HTTP error.
        return Err(if out.status.code() == Some(22) {
            Refused::Answer(why)
        } else {
            Refused::Unreachable(why)
        });
    }
    Ok(out.stdout)
}

/// A command that checks a kept receipt's signature without Allomos, for `alm audit` to name.
pub const VERIFY_HINT: &str = "openssl ts -verify -digest <checkpoint sha256> -in <receipt token, base64-decoded> -token_in -CAfile <the service's CA certificate>";

/// The fingerprint the test reply countersigns.
#[cfg(any(test, feature = "test-settings"))]
pub const FIXTURE_SHA256: &str = "2134d18b1e0af994285e1657e11157a5ba51ec371f009cbdab317237883a796d";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for n in 0..10 {
            let bytes: Vec<u8> = (0..n).map(|i| (i * 37 + 5) as u8).collect();
            assert_eq!(unbase64(&base64(&bytes)).unwrap(), bytes);
        }
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
    }

    #[test]
    fn the_request_is_rfc_3161() {
        let req = request(&[0xab; 32], &[0x41, 2, 3, 4, 5, 6, 7, 8]);
        // SEQUENCE { INTEGER 1, SEQUENCE { SEQUENCE { sha256, NULL }, OCTET STRING }, INTEGER, TRUE }
        let parts = children(one(&req, SEQUENCE).unwrap().value).unwrap();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].value, [1]);
        let imprint = children(parts[1].value).unwrap();
        assert_eq!(children(imprint[0].value).unwrap()[0].value, OID_SHA256);
        assert_eq!(imprint[1].value, [0xab; 32]);
        assert_eq!(parts[3].whole, [0x01, 0x01, 0xff]);
        let n = nonce();
        assert!(n[0] >= 0x40 && n[0] < 0x80);
    }

    #[test]
    fn times_read_as_utc() {
        let g = Tlv {
            tag: GENERALIZED_TIME,
            value: b"20261108133321.5Z",
            whole: &[],
        };
        assert_eq!(time_of(&g).unwrap(), "2026-11-08T13:33:21Z");
        let u = Tlv {
            tag: UTC_TIME,
            value: b"400202194422Z",
            whole: &[],
        };
        assert_eq!(time_of(&u).unwrap(), "2040-02-02T19:44:22Z");
    }

    #[test]
    fn a_refusal_is_read_as_one() {
        // SEQUENCE { SEQUENCE { INTEGER 2 } }: rejection.
        let refused = [0x30, 0x05, 0x30, 0x03, 0x02, 0x01, 0x02];
        assert!(read_response(&refused).unwrap_err().contains("status 02"));
        assert!(read_response(b"<html>").is_err());
    }

    /// A reply from a test service of our own (openssl ts, ECDSA P-384 with SHA-512, as FreeTSA
    /// signs): `openssl ts -verify` says OK for it.
    #[test]
    fn a_real_reply_is_read() {
        let der = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tsa-reply.der"
        ))
        .unwrap();
        let token = read_response(&der).unwrap();
        let t = read_token(&token).unwrap();
        assert_eq!(t.imprint, FIXTURE_SHA256);
        assert_eq!(t.hash, "sha256");
        assert_eq!(t.nonce.as_deref(), Some("4102030405060708"));
        assert!(t.whole);
        assert!(t.time.starts_with("2026-11-08T"));
        assert!(t.certificate_expires.as_deref().unwrap() > "2036");
        assert_eq!(
            t.signer.as_deref(),
            Some("O=Allomos test, CN=Allomos test TSA")
        );
        // A changed byte inside the countersigned content: no longer whole.
        let mut changed = token.clone();
        let at = changed
            .windows(32)
            .position(|w| w == unhex(FIXTURE_SHA256).unwrap())
            .unwrap();
        changed[at] ^= 1;
        let t = read_token(&changed).unwrap();
        assert!(!t.whole);
        let receipt = receipt(
            "https://tsa.test/",
            "2026-11-08T00:00:00Z",
            &token,
            &read_token(&token).unwrap(),
        );
        assert_eq!(token_of(&receipt).unwrap().imprint, FIXTURE_SHA256);
    }
}
