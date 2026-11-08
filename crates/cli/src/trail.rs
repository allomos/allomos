//! `alm audit` and `alm checkpoint` (Task-A01-gusto404, decision 0021 § 3): check the history's
//! signatures and chains, and fix the taskbag's fingerprint in a checkpoint.
//!
//! `alm` tells; the agent checks: `alm audit` lists what it found and never says whether the
//! history is "valid".

use std::path::Path;

use allomos_core::Problem;
use allomos_store::trail::Facts;
use serde_json::json;

use crate::{Done, lock_wait};

pub(crate) fn audit(root: &Path) -> Result<Done, Problem> {
    let facts = allomos_store::trail::audit(root);
    let text = facts_text(&facts);
    Ok(Done::new(json!({ "taskbag": root, "audit": facts }), text))
}

pub(crate) fn checkpoint(root: &Path, git_commit: Option<&str>) -> Result<Done, Problem> {
    let made = allomos_store::checkpoint::checkpoint(root, lock_wait(), git_commit)?;
    let text = format!(
        "Checkpoint {}: fingerprint {} of {} file(s), {} change(s), the last change of {} key(s). A later change to anything it covers shows in `alm audit`.\n",
        made.file, made.sha256, made.files, made.events, made.heads
    );
    Ok(Done::new(
        json!({ "taskbag": root, "checkpoint": made }),
        text,
    ))
}

fn list(title: &str, items: &[String], out: &mut String) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("{title} ({}):\n", items.len()));
    for i in items {
        out.push_str(&format!("  {i}\n"));
    }
}

/// The facts in plain lines, the facts that need a look first.
fn facts_text(f: &Facts) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "History: {} change(s); {} signed; {} recorded before signing.\n",
        f.events, f.signed, f.before_signing
    ));
    out.push_str(&format!(
        "Fingerprint now: {} ({} file(s)).\n",
        f.sha256, f.files
    ));
    if let Some((file, at)) = &f.rewritten {
        out.push_str(&format!(
            "This computer saw the checkpoint {file} (on {at}); the history no longer holds it: it was rewritten or the checkpoint removed since then.\n"
        ));
    }
    list(
        "Changed after signing (the signature does not match what the file says now)",
        &f.changed,
        &mut out,
    );
    if !f.redacted.is_empty() {
        // Task-A01-stem986: changed by a recorded redaction, so not listed as changed above.
        out.push_str(&format!(
            "Redacted (personal details removed, recorded by a redaction; the signature no longer matches) ({}):\n",
            f.redacted.len()
        ));
        for r in &f.redacted {
            out.push_str(&format!(
                "  {} ({}): {}, recorded in {}\n",
                r.file,
                r.fields.join(", "),
                r.redaction,
                r.by_event
            ));
        }
    }
    if !f.breaks.is_empty() {
        out.push_str(&format!(
            "Gaps in a computer's chain: the change before these is missing ({}):\n",
            f.breaks.len()
        ));
        for b in &f.breaks {
            out.push_str(&format!(
                "  {} (key {}): the change before it (fingerprint {}) is not in the history{}\n",
                b.file,
                b.key,
                &b.missing[..b.missing.len().min(16)],
                match (&b.after, b.after_changed) {
                    (Some(a), true) => format!(
                        "; the key's change before it, {a}, was changed after signing, which alone breaks this link"
                    ),
                    (Some(a), false) => format!("; the key's change before the gap still there: {a}"),
                    (None, _) => String::new(),
                }
            ));
        }
    }
    if !f.forks.is_empty() {
        out.push_str(&format!(
            "Forks: two or more changes follow the same change of one key ({}):\n",
            f.forks.len()
        ));
        for k in &f.forks {
            out.push_str(&format!("  key {}: {}\n", k.key, k.files.join(", ")));
        }
    }
    list("Signed with a key not in keys/", &f.unknown_key, &mut out);
    list("Unsigned after signing started", &f.unsigned, &mut out);
    list("Could not be read", &f.unreadable, &mut out);
    if f.chains.is_empty() {
        out.push_str("Keys: none has signed a change yet.\n");
    } else {
        out.push_str("Keys (one chain of changes each):\n");
        for c in &f.chains {
            out.push_str(&format!(
                "  {}{}: {} change(s), {} … {}\n",
                c.key,
                c.owner
                    .as_ref()
                    .map(|o| format!(" ({o})"))
                    .unwrap_or_default(),
                c.events,
                c.first,
                c.last
            ));
        }
    }
    if f.checkpoints.is_empty() {
        out.push_str("Checkpoints: none (`alm checkpoint` makes one).\n");
    } else {
        out.push_str(&format!("Checkpoints ({}):\n", f.checkpoints.len()));
        for c in &f.checkpoints {
            out.push_str(&format!(
                "  {} at {}: {}, {} change(s), {} receipt(s){}\n",
                c.file,
                c.at,
                if c.signed {
                    "signature matches"
                } else {
                    "signature does not match a key in keys/"
                },
                c.events,
                c.receipts,
                if c.heads_missing.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; its last change(s) no longer in the history as they were: {}",
                        c.heads_missing.join(", ")
                    )
                }
            ));
        }
        match f.changed_since_checkpoint {
            Some(true) => out.push_str(&format!(
                "Since the last checkpoint the taskbag has changed ({} signed change(s) newer than it).\n",
                f.events_since_checkpoint
            )),
            Some(false) => out.push_str("Since the last checkpoint nothing has changed.\n"),
            None => {}
        }
    }
    out.push_str("These are facts, not a verdict: you check what they mean.\n");
    out
}
