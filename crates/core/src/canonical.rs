//! Canonical JSON (RFC 8785, the JSON Canonicalization Scheme) and SHA-256 fingerprints, for
//! signed events and checkpoints (Task-A01-gusto404, decision 0021 § 3).
//!
//! The same JSON value always gives the same bytes, whatever order its object members were
//! written in and however it was indented, so a signature over an event still checks after the
//! event is packed into its month's file or re-indented.
//!
//! Objects' members are sorted by their names' UTF-16 code units; strings are written as
//! ECMAScript's `JSON.stringify` writes them (only `"`, `\` and control characters escaped, in
//! the short form where there is one, else `\u00xx`); numbers as ECMAScript writes them. Allomos
//! writes only whole numbers in its files, so a fraction or exponent is written as the shortest
//! form that reads back as the same number, with ECMAScript's exponent spelling (`1e+21`).

use serde_json::Value;
use sha2::{Digest, Sha256};

/// `value` in canonical JSON.
pub fn to_string(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(n)),
        Value::String(s) => string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (name, item)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(name, out);
                out.push(':');
                write(item, out);
            }
            out.push('}');
        }
    }
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn number(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    let f = n.as_f64().unwrap_or(0.0);
    if f == 0.0 {
        return "0".into();
    }
    if f.fract() == 0.0 && f.abs() < 1e21 {
        return format!("{f:.0}");
    }
    // Rust's shortest round-trip form, in ECMAScript's spelling for exponents.
    let a = f.abs();
    if (1e-6..1e21).contains(&a) {
        return format!("{f}");
    }
    let s = format!("{f:e}");
    match s.split_once('e') {
        Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
        _ => s,
    }
}

/// SHA-256 of `bytes`, as 64 lowercase hexadecimal digits.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// `bytes` as lowercase hexadecimal digits.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The bytes written as hexadecimal digits in `text`; `None` when it is not hexadecimal.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The fingerprint of an event or checkpoint (with its `sig`): SHA-256 of its canonical JSON.
/// A signed event's `sig.prev` is its key's previous event's fingerprint.
pub fn fingerprint(value: &Value) -> String {
    sha256_hex(to_string(value).as_bytes())
}

/// What a signature covers: `value` without its `sig`, in canonical JSON.
pub fn signed_bytes(value: &Value) -> Vec<u8> {
    let mut v = value.clone();
    if let Some(map) = v.as_object_mut() {
        map.remove("sig");
    }
    to_string(&v).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_members_and_drops_whitespace() {
        let v: Value =
            serde_json::from_str("{ \"b\": [1, 2], \"a\": {\"y\": null, \"x\": true} }").unwrap();
        assert_eq!(to_string(&v), r#"{"a":{"x":true,"y":null},"b":[1,2]}"#);
    }

    #[test]
    fn sorts_by_utf16_code_units() {
        // U+1F600 is D83D DE00 in UTF-16, before U+FB33; in UTF-8 it would sort after.
        let v = json!({"\u{fb33}": 1, "\u{1f600}": 2});
        assert_eq!(to_string(&v), "{\"\u{1f600}\":2,\"\u{fb33}\":1}");
    }

    #[test]
    fn escapes_as_json_stringify() {
        let v = json!("a\"b\\c\n\u{1}é/");
        assert_eq!(to_string(&v), "\"a\\\"b\\\\c\\n\\u0001é/\"");
    }

    #[test]
    fn numbers_as_ecmascript() {
        let v: Value = serde_json::from_str("[0, -3, 1.5, 1e21, 100.0, 1e-7]").unwrap();
        assert_eq!(to_string(&v), "[0,-3,1.5,1e+21,100,1e-7]");
    }

    #[test]
    fn signed_bytes_leave_out_sig() {
        let v = json!({"at": "t", "sig": {"value": "x"}});
        assert_eq!(signed_bytes(&v), br#"{"at":"t"}"#.to_vec());
        assert_eq!(unhex(&hex(&[0, 255, 16])), Some(vec![0, 255, 16]));
        assert_eq!(unhex("zz"), None);
    }
}
