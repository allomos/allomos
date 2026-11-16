//! When the taskbag cannot be read (Task-A03-raven398; approver, 2026-11-16: tell, do not stop).
//! A run reads its taskbag every few seconds; before this task a read that failed was logged and
//! tried again without a word to the person, so a run whose `alm` was older than the taskbag's
//! format went on for hours, alive but doing nothing, and its log grew by a line every five
//! seconds.
//!
//! Now the run goes on trying, and tells the person once: at once when only a person can fix it
//! (the taskbag is newer than this `alm`, or must be upgraded or moved in first), and after
//! [`UNREADABLE_FOR`] for any other failure (a file half written, a folder briefly away), which
//! may pass by itself. The status bar shows it until the taskbag can be read again. The log names
//! a failure once, and again only when it changes, and says when the taskbag reads again.

use allomos_core::Problem;

/// How long an ordinary failure may last before the person is told (ms): two minutes, about 24
/// reads.
pub const UNREADABLE_FOR: i64 = 2 * 60 * 1000;

/// Why the taskbag cannot be read, as the status bar names it (the `unreadable` field of
/// [`crate::flow::Run`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The taskbag is in a format this `alm` cannot read or write: only a person can fix it
    /// (update Allomos, upgrade or move in the taskbag).
    Format,
    /// The taskbag could not be read for [`UNREADABLE_FOR`].
    Unreadable,
}

impl Why {
    /// The code kept in the last-run file.
    pub fn code(self) -> &'static str {
        match self {
            Why::Format => "taskbag_format",
            Why::Unreadable => "taskbag_unreadable",
        }
    }
}

/// What the run does with one failed read; it tries again in every case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Write this line in the log.
    Log(String),
    /// Nothing more (the same failure as before, or the person was told already).
    Quiet,
    /// Tell the person, once, and show it in the status bar.
    Tell(Why),
}

/// Whether only a person can fix `problem`: then trying again does not help.
pub fn needs_person(problem: &Problem) -> bool {
    matches!(
        problem,
        Problem::TaskbagNewer { .. }
            | Problem::TaskbagToUpgrade { .. }
            | Problem::TaskbagToMoveIn { .. }
    )
}

/// The failed reads since the taskbag was last read.
#[derive(Debug, Default)]
pub struct Failures {
    /// When the first of them happened (ms since 1970).
    since: Option<i64>,
    /// The last failure's words, to log a failure only when it changes.
    last: Option<String>,
    /// How many there were.
    count: u32,
    /// Whether the person was told.
    told: bool,
}

impl Failures {
    /// A read failed with `problem` at `now`.
    pub fn failed(&mut self, problem: &Problem, now: i64) -> Step {
        let since = *self.since.get_or_insert(now);
        self.count += 1;
        let words = problem.english();
        let changed = self.last.as_deref() != Some(words.as_str());
        if changed {
            self.last = Some(words.clone());
        }
        if !self.told {
            let why = if needs_person(problem) {
                Some(Why::Format)
            } else {
                (now - since >= UNREADABLE_FOR).then_some(Why::Unreadable)
            };
            if let Some(why) = why {
                self.told = true;
                return Step::Tell(why);
            }
        }
        if !changed {
            return Step::Quiet;
        }
        Step::Log(format!(
            "could not read the taskbag (trying again): {words}"
        ))
    }

    /// A read worked: the line for the log when reads had failed before it.
    pub fn read(&mut self) -> Option<String> {
        let count = std::mem::take(self).count;
        (count > 0).then(|| format!("read the taskbag again, after {count} failed reads"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn newer() -> Problem {
        Problem::TaskbagNewer {
            found: "allomos/9".into(),
            supported: "allomos/8".into(),
        }
    }

    fn busy(detail: &str) -> Problem {
        Problem::NotJson {
            detail: detail.into(),
        }
    }

    #[test]
    fn a_newer_taskbag_is_told_at_once_and_only_once() {
        let mut f = Failures::default();
        assert_eq!(f.failed(&newer(), 1_000), Step::Tell(Why::Format));
        assert_eq!(f.failed(&newer(), 6_000), Step::Quiet);
        assert_eq!(f.failed(&newer(), 999_000), Step::Quiet);
        let upgrade = Problem::TaskbagToUpgrade {
            found: "allomos/8".into(),
            current: "allomos/9".into(),
        };
        assert_eq!(
            Failures::default().failed(&upgrade, 0),
            Step::Tell(Why::Format)
        );
    }

    #[test]
    fn a_passing_failure_is_logged_once_and_tried_again() {
        let mut f = Failures::default();
        assert!(matches!(f.failed(&busy("eof"), 0), Step::Log(l) if l.contains("eof")));
        assert_eq!(f.failed(&busy("eof"), 5_000), Step::Quiet);
        // Another failure is named again.
        assert!(matches!(f.failed(&busy("other"), 10_000), Step::Log(l) if l.contains("other")));
        assert_eq!(
            f.read(),
            Some("read the taskbag again, after 3 failed reads".into())
        );
        // After a good read, nothing is left over: the next failure starts afresh.
        assert_eq!(f.read(), None);
        assert!(matches!(f.failed(&busy("eof"), 999_000), Step::Log(_)));
    }

    #[test]
    fn a_failure_that_lasts_two_minutes_is_told_once_and_the_run_goes_on() {
        let mut f = Failures::default();
        f.failed(&busy("eof"), 0);
        assert_eq!(f.failed(&busy("eof"), UNREADABLE_FOR - 1), Step::Quiet);
        assert_eq!(
            f.failed(&busy("eof"), UNREADABLE_FOR),
            Step::Tell(Why::Unreadable)
        );
        assert_eq!(f.failed(&busy("eof"), UNREADABLE_FOR + 5_000), Step::Quiet);
        // Read again, then a new spell of failures is told again.
        f.read();
        assert_eq!(f.failed(&newer(), 0), Step::Tell(Why::Format));
    }

    #[test]
    fn the_codes_are_the_ones_the_app_reads() {
        assert_eq!(Why::Format.code(), "taskbag_format");
        assert_eq!(Why::Unreadable.code(), "taskbag_unreadable");
    }
}
