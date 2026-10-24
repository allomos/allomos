//! Task IDs that cannot clash between branches (Task-A01-54, approver 2026-10-20): a new task
//! gets `<area>-<word><000-999>`, for example `A01-tiger042`, drawn at random from a built-in
//! list of about 1,000 short, common, easy-to-spell English words, so two people adding tasks on
//! different branches without pulling almost never pick the same ID (about 1,000,000 choices per
//! area). Existing IDs (`A01-53`) keep working and are never renamed.

use std::cmp::Ordering;
use std::collections::BTreeMap;

/// The words, one per line, lowercase, 3–6 letters.
const WORDS: &str = include_str!("ids/words.txt");

pub fn words() -> impl Iterator<Item = &'static str> {
    WORDS.lines().filter(|w| !w.is_empty())
}

/// A number that differs on every call, from the standard library's random hash keys.
fn random() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(CALLS.fetch_add(1, Ordering::Relaxed));
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    );
    h.write_u32(std::process::id());
    h.finish()
}

/// A new ID in `area` not in `taken`: `<area>-<word><three digits>`, drawn again on a clash.
pub fn new_task_id(area: &str, mut taken: impl FnMut(&str) -> bool) -> String {
    let words: Vec<&str> = words().collect();
    loop {
        let n = random();
        let word = words[(n % words.len() as u64) as usize];
        let id = format!("{area}-{word}{:03}", (n / words.len() as u64) % 1000);
        if !taken(&id) {
            return id;
        }
    }
}

/// Labor words for iteration IDs (Task-A00-rope803), one per line, lowercase, 3–6 letters.
const LABOR: &str = include_str!("ids/labor.txt");

/// A new iteration ID not in `taken`: `I-<labor word>`, an unused one drawn at random (approver,
/// 2026-10-24: iterations are named by a word, not by a number that looks like their place in the
/// plan). Once every word is used, `I-<word><two digits>`.
pub fn new_iteration_id(mut taken: impl FnMut(&str) -> bool) -> String {
    let words: Vec<&str> = LABOR.lines().filter(|w| !w.is_empty()).collect();
    let free: Vec<String> = words
        .iter()
        .map(|w| format!("I-{w}"))
        .filter(|id| !taken(id))
        .collect();
    if !free.is_empty() {
        return free[(random() % free.len() as u64) as usize].clone();
    }
    loop {
        let n = random();
        let word = words[(n % words.len() as u64) as usize];
        let id = format!("I-{word}{:02}", (n / words.len() as u64) % 100);
        if !taken(&id) {
            return id;
        }
    }
}

/// Whether `id` has the older, numbered shape (`A01-53`, `A01-03-01`): only digits after the
/// area.
pub fn numbered(id: &str) -> bool {
    id.split_once('-').is_some_and(|(_, rest)| {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '-')
    })
}

/// The order of task IDs in lists: numbered IDs first, by area and number as before; then the
/// newer word IDs in the order they were made (`created`, UTC times from the history; a task
/// without one counts as oldest), then by ID so the order never jumps.
pub fn compare(a: &str, b: &str, created: &BTreeMap<String, String>) -> Ordering {
    let key = |id: &str| {
        (
            !numbered(id),
            (!numbered(id)).then(|| created.get(id).cloned().unwrap_or_default()),
        )
    };
    key(a).cmp(&key(b)).then_with(|| natural(a, b))
}

/// Text order with runs of digits compared as numbers (`A01-9` before `A01-10`).
fn natural(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, run)) if *d == digit => run.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    };
    let (x, y) = (parts(a), parts(b));
    for ((dx, px), (dy, py)) in x.iter().zip(y.iter()) {
        let o = if *dx && *dy {
            let (tx, ty) = (px.trim_start_matches('0'), py.trim_start_matches('0'));
            tx.len()
                .cmp(&ty.len())
                .then_with(|| tx.cmp(ty))
                .then_with(|| px.len().cmp(&py.len()))
        } else {
            px.cmp(py)
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    x.len().cmp(&y.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_word_list_is_short_common_words() {
        let all: Vec<&str> = words().collect();
        assert!((900..=1200).contains(&all.len()), "{}", all.len());
        let mut sorted = all.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "no word twice");
        for w in &all {
            assert!(
                (3..=6).contains(&w.len()) && w.chars().all(|c| c.is_ascii_lowercase()),
                "{w}"
            );
        }
    }

    #[test]
    fn new_ids_are_a_word_and_three_digits_and_never_taken() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..2000 {
            let id = new_task_id("A01", |id| seen.contains(id));
            let rest = id.strip_prefix("A01-").unwrap();
            let (word, digits) = rest.split_at(rest.len() - 3);
            assert!(words().any(|w| w == word), "{id}");
            assert!(digits.chars().all(|c| c.is_ascii_digit()), "{id}");
            assert!(crate::ops::valid_id(&id), "{id}");
            assert!(!numbered(&id));
            assert!(seen.insert(id));
        }
        // A taken ID is drawn again.
        let only = "A02-tiger042";
        let mut first = true;
        let id = new_task_id("A02", |id| {
            let t = first;
            first = false;
            t || id == only
        });
        assert_ne!(id, only);
    }

    #[test]
    fn numbered_ids_first_then_word_ids_by_creation() {
        let created: BTreeMap<String, String> = [
            ("A01-tiger042", "2026-10-21T09:00:00Z"),
            ("A01-apple007", "2026-10-21T10:00:00Z"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        let mut ids = vec![
            "A01-apple007",
            "A01-10",
            "A02-02",
            "A01-tiger042",
            "A01-9",
            "A01-03-01",
            "A01-zebra001",
        ];
        ids.sort_by(|a, b| compare(a, b, &created));
        assert_eq!(
            ids,
            [
                "A01-03-01",
                "A01-9",
                "A01-10",
                "A02-02",
                "A01-zebra001",
                "A01-tiger042",
                "A01-apple007"
            ]
        );
        assert!(
            numbered("A01-53")
                && numbered("A01-03-01")
                && !numbered("A01")
                && !numbered("A01-tiger042")
        );
    }

    /// Task-A00-rope803: an unused `I-<labor word>`; once all are used, a word with two digits.
    #[test]
    fn iteration_ids_are_unused_labor_words() {
        let labor: Vec<&str> = LABOR.lines().filter(|w| !w.is_empty()).collect();
        assert!(labor.len() >= 100);
        for w in &labor {
            assert!((3..=6).contains(&w.len()) && w.chars().all(|c| c.is_ascii_lowercase()));
        }
        for w in [
            "dig", "plow", "sow", "knit", "haul", "tally", "stoke", "forge", "glean",
        ] {
            assert!(labor.contains(&w), "{w}");
        }
        let id = new_iteration_id(|_| false);
        assert!(labor.contains(&id.strip_prefix("I-").unwrap()), "{id}");
        // Only one left: that one.
        for _ in 0..20 {
            assert_eq!(new_iteration_id(|id| id != "I-sow"), "I-sow");
        }
        // None left: a word and two digits, never a used one.
        let id = new_iteration_id(|id| !id.ends_with(|c: char| c.is_ascii_digit()));
        let (word, digits) = id[2..].split_at(id.len() - 4);
        assert!(labor.contains(&word) && digits.len() == 2, "{id}");
    }
}
