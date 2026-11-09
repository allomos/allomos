//! A task's difficulty (Task-A01-actor813, approver 2026-11-08): a level from 1 (very easy) to 5
//! (very hard), which the agent that adds the task fills in. A task without one, as every task
//! written before it, counts as level 3; nothing rewrites its file. The level is an optional field
//! of `task.json` (`difficulty`), so it needs no new format: an older Allomos keeps it as a field
//! it does not know.

use crate::model::{Task, Taskbag};
use crate::ops::{Outcome, check_holder, find_task};
use crate::problem::Problem;

/// The lowest level: very easy.
pub const MIN: u8 = 1;
/// The highest level: very hard.
pub const MAX: u8 = 5;
/// The level of a task that has none (approver, 2026-11-08).
pub const DEFAULT: u8 = 3;

/// What each level means, in English, as `alm add task --help` and the guides word it.
pub const LEVELS: [(u8, &str); 5] = [
    (1, "one small change in one place; nothing to decide"),
    (
        2,
        "a small change across a few files, or one well-known step",
    ),
    (
        3,
        "an ordinary task: one feature or fix with its tests, a few routine choices",
    ),
    (
        4,
        "several parts change together (core, command, app), or something risky to get right",
    ),
    (
        5,
        "needs design decisions, crosses many parts, or is hard to get right",
    ),
];

/// A level as given, checked: a whole number from [`MIN`] to [`MAX`].
pub fn level(given: &str) -> Result<u8, Problem> {
    match given.trim().parse::<u8>() {
        Ok(n) if (MIN..=MAX).contains(&n) => Ok(n),
        _ => Err(Problem::DifficultyInvalid {
            value: given.to_string(),
        }),
    }
}

/// Whether a value found in a task file is a level ([`MIN`]..=[`MAX`]).
pub fn valid(n: i64) -> bool {
    (i64::from(MIN)..=i64::from(MAX)).contains(&n)
}

/// The task's level and whether it is the default (no level written, or one that is not a level).
pub fn of(task: &Task) -> (u8, bool) {
    match task.difficulty {
        Some(n) if valid(n) => (n as u8, false),
        _ => (DEFAULT, true),
    }
}

/// A range of levels to list (`alm next --difficulty 1-3`, `--difficulty 4`), both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub low: u8,
    pub high: u8,
}

impl Range {
    /// `<low>-<high>` or a single level; each end a level, `low` not above `high`.
    pub fn parse(given: &str) -> Result<Range, Problem> {
        let bad = || Problem::DifficultyRangeInvalid {
            value: given.to_string(),
        };
        let (low, high) = match given.split_once('-') {
            Some((a, b)) => (level(a).map_err(|_| bad())?, level(b).map_err(|_| bad())?),
            None => {
                let n = level(given).map_err(|_| bad())?;
                (n, n)
            }
        };
        if low > high {
            return Err(bad());
        }
        Ok(Range { low, high })
    }

    /// Every level, 1 to 5: the range an agent has when none is chosen (Task-A03-wave335).
    pub fn all() -> Range {
        Range {
            low: MIN,
            high: MAX,
        }
    }

    /// Whether the task's level (the default one included) is in the range.
    pub fn holds(&self, task: &Task) -> bool {
        (self.low..=self.high).contains(&of(task).0)
    }

    /// The range as `alm next --difficulty` takes it: `1-5`, or `4` for one level.
    pub fn text(&self) -> String {
        if self.low == self.high {
            self.low.to_string()
        } else {
            format!("{}-{}", self.low, self.high)
        }
    }
}

/// Set task `id`'s difficulty (`alm difficulty`, the task pane). `None` takes it off: the task
/// counts as level 3 again.
pub fn set(
    taskbag: &Taskbag,
    id: &str,
    me: Option<&str>,
    difficulty: Option<u8>,
) -> Result<Outcome, Problem> {
    let mut task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    if let Some(n) = difficulty {
        level(&n.to_string())?;
    }
    task.difficulty = difficulty.map(i64::from);
    Ok(Outcome::plain(task))
}

/// The task's level for a one-line listing: `3` or `3 (default)`.
pub fn text(task: &Task) -> String {
    match of(task) {
        (n, false) => n.to_string(),
        (n, true) => format!("{n} (default)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(difficulty: Option<i64>) -> Task {
        let mut t: Task = serde_json::from_str(
            r#"{ "id": "A01-01", "title": "t", "area": "A01", "state": "ready" }"#,
        )
        .unwrap();
        t.difficulty = difficulty;
        t
    }

    #[test]
    fn levels_from_one_to_five() {
        assert_eq!(level("1"), Ok(1));
        assert_eq!(level(" 5 "), Ok(5));
        for bad in ["0", "6", "-1", "x", "", "2.5"] {
            assert_eq!(
                level(bad),
                Err(Problem::DifficultyInvalid { value: bad.into() })
            );
        }
    }

    #[test]
    fn no_level_counts_as_three() {
        assert_eq!(of(&task(None)), (3, true));
        assert_eq!(of(&task(Some(9))), (3, true));
        assert_eq!(of(&task(Some(5))), (5, false));
        assert_eq!(text(&task(None)), "3 (default)");
        assert_eq!(text(&task(Some(1))), "1");
    }

    #[test]
    fn ranges() {
        assert_eq!(Range::parse("1-3"), Ok(Range { low: 1, high: 3 }));
        assert_eq!(Range::parse("4"), Ok(Range { low: 4, high: 4 }));
        for bad in ["3-1", "0-2", "1-6", "a", "1-", ""] {
            assert!(Range::parse(bad).is_err(), "{bad}");
        }
        let r = Range::parse("1-3").unwrap();
        assert!(r.holds(&task(None)));
        assert!(r.holds(&task(Some(1))));
        assert!(!r.holds(&task(Some(4))));
        assert!(!Range::parse("4-5").unwrap().holds(&task(None)));
    }

    #[test]
    fn the_file_keeps_no_level_when_none_is_set() {
        let t = task(None);
        let v = serde_json::to_value(&t).unwrap();
        assert!(v.get("difficulty").is_none());
        let v = serde_json::to_value(task(Some(2))).unwrap();
        assert_eq!(v["difficulty"], 2);
    }
}
