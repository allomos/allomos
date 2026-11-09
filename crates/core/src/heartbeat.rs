//! Heartbeats (decision 0022, Task-A01-nurse164): the session holding a task shows it is still at
//! work by writing the time of its last heartbeat into the task's own file. A heartbeat is no
//! event: burndown, timeline and history do not change (R7), and a task's version
//! ([`crate::ops::task_version`]) leaves it out, so a beat never makes another writer's copy out of
//! date.
//!
//! `alm beat` writes one ([`beat`]); every change the holder makes to its task writes one along
//! with it ([`along`]); read-only commands write none.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, State, Task};
use crate::problem::Problem;

/// How often a holding session beats, by default (decision 0022 § 3): 10 minutes. `alm beat`
/// writes nothing while the last heartbeat is younger than half of it.
pub const INTERVAL_SECS: i64 = 10 * 60;

/// The last heartbeat of a held task, in `task.json` as `heartbeat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heartbeat {
    /// When, in UTC (`2026-11-09T14:02:05Z`).
    pub at: String,
    /// The session that beat (its `--as` name): the task's holder then.
    pub by: String,
    /// The end of a busy window the session declared (`alm beat --busy`), in UTC: until then
    /// the task is not lost, however old `at` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busy_until: Option<String>,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// What [`beat`] decided for one task.
#[derive(Debug, Clone, PartialEq)]
pub enum Beat {
    /// The task with its new heartbeat, to be written.
    Written(Task),
    /// The last heartbeat is younger than half the interval: nothing is written.
    TooSoon {
        task: Task,
        /// Seconds since the last heartbeat.
        ago: i64,
        /// When the next heartbeat is due (the last one plus the interval), in UTC.
        due: String,
    },
}

/// `alm beat` for one task, by the session `me`, at `now` (seconds since 1970, UTC). Only the
/// holder of an `in_progress` task beats (`not_held`, `task_held`). `busy`: a busy window of that
/// many seconds from now (`0` ends the one running); a declared window is always written. Without
/// it, a heartbeat younger than half of `interval` (seconds) is not written again: [`Beat::TooSoon`].
/// A plain beat keeps a busy window that is still running.
pub fn beat(
    task: &Task,
    me: &str,
    now: i64,
    busy: Option<i64>,
    interval: i64,
) -> Result<Beat, Problem> {
    if task.state != State::InProgress || task.holder.is_none() {
        return Err(Problem::NotHeld {
            id: task.id.clone(),
            state: task.state.as_str().replace('_', " "),
        });
    }
    if task.holder.as_deref() != Some(me) {
        return Err(Problem::TaskHeld {
            id: task.id.clone(),
            holder: task.holder.clone().unwrap_or_default(),
        });
    }
    let last = task
        .heartbeat
        .as_ref()
        .filter(|h| h.by == me)
        .and_then(|h| parse_utc(&h.at).map(|at| (h, at)));
    if busy.is_none() {
        if let Some((_, at)) = last {
            let ago = now - at;
            if (0..interval / 2).contains(&ago) {
                return Ok(Beat::TooSoon {
                    task: task.clone(),
                    ago,
                    due: format_utc(at + interval),
                });
            }
        }
    }
    let mut task = task.clone();
    let kept = last
        .and_then(|(h, _)| h.busy_until.clone())
        .filter(|until| busy.is_none() && parse_utc(until).is_some_and(|u| u > now));
    let extra = task
        .heartbeat
        .as_ref()
        .map(|h| h.extra.clone())
        .unwrap_or_default();
    task.heartbeat = Some(Heartbeat {
        at: format_utc(now),
        by: me.to_string(),
        busy_until: match busy {
            Some(secs) if secs > 0 => Some(format_utc(now + secs)),
            Some(_) => None,
            None => kept,
        },
        extra,
    });
    Ok(Beat::Written(task))
}

/// The heartbeat a change to `task` by the session `me` writes along with it (decision 0022 § 5),
/// at `now`: when `me` holds the task and it is `in_progress`, the heartbeat becomes now (a busy
/// window still running is kept). A task nobody holds, or no longer `in_progress` (released,
/// waiting, blocked, done, dropped), loses its heartbeat. A change by someone else to a task the
/// session holds (the app, a person) leaves it as it is.
pub fn along(task: &mut Task, me: Option<&str>, now: i64) {
    if task.state != State::InProgress || task.holder.is_none() {
        task.heartbeat = None;
        return;
    }
    let Some(me) = me else { return };
    if task.holder.as_deref() != Some(me) {
        return;
    }
    let kept = task
        .heartbeat
        .as_ref()
        .filter(|h| h.by == me)
        .and_then(|h| h.busy_until.clone())
        .filter(|until| parse_utc(until).is_some_and(|u| u > now));
    let extra = task
        .heartbeat
        .as_ref()
        .map(|h| h.extra.clone())
        .unwrap_or_default();
    task.heartbeat = Some(Heartbeat {
        at: format_utc(now),
        by: me.to_string(),
        busy_until: kept,
        extra,
    });
}

/// A busy window's length (`45m`, `2h`, `1h30m`, `1d`, `90s`; `0m` ends one) in seconds, or `None`
/// when it is not written that way.
pub fn parse_duration(text: &str) -> Option<i64> {
    let text = text.trim().to_ascii_lowercase();
    if text.is_empty() {
        return None;
    }
    let (mut total, mut digits) = (0i64, String::new());
    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let unit = match c {
            'd' => 86_400,
            'h' => 3_600,
            'm' => 60,
            's' => 1,
            _ => return None,
        };
        let n: i64 = digits.parse().ok()?;
        total = total.checked_add(n.checked_mul(unit)?)?;
        digits.clear();
    }
    digits.is_empty().then_some(total)
}

/// A UTC time as Allomos writes it (`2026-11-09T14:02:05Z`, or without seconds) in seconds since
/// 1970.
pub fn parse_utc(text: &str) -> Option<i64> {
    let t = text.strip_suffix('Z')?;
    let (date, time) = t.split_once('T')?;
    let mut d = date.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let m: i64 = d.next()?.parse().ok()?;
    let day: i64 = d.next()?.parse().ok()?;
    let mut parts = time.split(':');
    let hh: i64 = parts.next()?.parse().ok()?;
    let mm: i64 = parts.next()?.parse().ok()?;
    let ss: i64 = match parts.next() {
        Some(s) => s.parse().ok()?,
        None => 0,
    };
    if d.next().is_some()
        || parts.next().is_some()
        || !(1..=12).contains(&m)
        || !(1..=31).contains(&day)
        || hh > 23
        || mm > 59
        || ss > 60
    {
        return None;
    }
    Some(days_from_civil(y, m, day) * 86_400 + hh * 3_600 + mm * 60 + ss)
}

/// Seconds since 1970 as a UTC time (`2026-11-09T14:02:05Z`).
pub fn format_utc(secs: i64) -> String {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// A length of time in plain words: `40 s`, `3 min`, `2 h 5 min`.
pub fn plain_duration(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{secs} s")
    } else if secs < 3_600 {
        format!("{} min", secs / 60)
    } else {
        let (h, m) = (secs / 3_600, secs % 3_600 / 60);
        if m == 0 {
            format!("{h} h")
        } else {
            format!("{h} h {m} min")
        }
    }
}

// Howard Hinnant's days_from_civil and civil_from_days.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(by: &str) -> Task {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": "A01-01", "title": "t", "area": "A01", "state": "in_progress", "holder": by
        }))
        .unwrap();
        t.description.clear();
        t
    }

    const T0: &str = "2026-11-09T14:00:00Z";

    fn t0() -> i64 {
        parse_utc(T0).unwrap()
    }

    #[test]
    fn times_and_durations_read_and_write() {
        assert_eq!(format_utc(t0()), T0);
        assert_eq!(parse_utc("2026-11-09T14:00Z"), Some(t0()));
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2026-11-09 14:00Z"), None);
        assert_eq!(parse_utc("2026-13-09T14:00Z"), None);
        assert_eq!(parse_duration("45m"), Some(2_700));
        assert_eq!(parse_duration("1h30m"), Some(5_400));
        assert_eq!(parse_duration("2H"), Some(7_200));
        assert_eq!(parse_duration("1d"), Some(86_400));
        assert_eq!(parse_duration("0m"), Some(0));
        for bad in ["", "45", "m", "1x", "1h30", "-5m"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
        assert_eq!(plain_duration(40), "40 s");
        assert_eq!(plain_duration(180), "3 min");
        assert_eq!(plain_duration(7_500), "2 h 5 min");
    }

    #[test]
    fn only_the_holder_of_a_task_in_progress_beats() {
        let t = held("s1");
        assert!(matches!(
            beat(&t, "s2", t0(), None, INTERVAL_SECS),
            Err(Problem::TaskHeld { .. })
        ));
        let mut ready = t.clone();
        ready.state = State::Ready;
        ready.holder = None;
        assert!(matches!(
            beat(&ready, "s1", t0(), None, INTERVAL_SECS),
            Err(Problem::NotHeld { .. })
        ));
        let Ok(Beat::Written(b)) = beat(&t, "s1", t0(), None, INTERVAL_SECS) else {
            panic!()
        };
        let h = b.heartbeat.unwrap();
        assert_eq!(
            (h.at.as_str(), h.by.as_str(), h.busy_until),
            (T0, "s1", None)
        );
    }

    #[test]
    fn a_beat_under_half_the_interval_writes_nothing() {
        let Ok(Beat::Written(t)) = beat(&held("s1"), "s1", t0(), None, INTERVAL_SECS) else {
            panic!()
        };
        match beat(&t, "s1", t0() + 299, None, INTERVAL_SECS).unwrap() {
            Beat::TooSoon { ago, due, .. } => {
                assert_eq!(ago, 299);
                assert_eq!(due, "2026-11-09T14:10:00Z");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            beat(&t, "s1", t0() + 300, None, INTERVAL_SECS).unwrap(),
            Beat::Written(_)
        ));
        // A heartbeat of another session (the one before a takeover) does not count.
        let mut other = t.clone();
        other.holder = Some("s2".into());
        assert!(matches!(
            beat(&other, "s2", t0() + 10, None, INTERVAL_SECS).unwrap(),
            Beat::Written(_)
        ));
    }

    #[test]
    fn busy_windows_are_written_kept_and_ended() {
        let Ok(Beat::Written(t)) = beat(&held("s1"), "s1", t0(), None, INTERVAL_SECS) else {
            panic!()
        };
        // Declared even right after a beat.
        let Ok(Beat::Written(t)) = beat(&t, "s1", t0() + 60, Some(7_200), INTERVAL_SECS) else {
            panic!()
        };
        let until = "2026-11-09T16:01:00Z";
        assert_eq!(
            t.heartbeat.as_ref().unwrap().busy_until.as_deref(),
            Some(until)
        );
        // A plain beat keeps it while it runs.
        let Ok(Beat::Written(t)) = beat(&t, "s1", t0() + 1_000, None, INTERVAL_SECS) else {
            panic!()
        };
        assert_eq!(
            t.heartbeat.as_ref().unwrap().busy_until.as_deref(),
            Some(until)
        );
        // So does a change along with it; once over, it goes.
        let mut c = t.clone();
        along(&mut c, Some("s1"), t0() + 2_000);
        assert_eq!(
            c.heartbeat.as_ref().unwrap().busy_until.as_deref(),
            Some(until)
        );
        along(&mut c, Some("s1"), t0() + 9_000);
        assert_eq!(c.heartbeat.as_ref().unwrap().busy_until, None);
        // `0m` ends it.
        let Ok(Beat::Written(t)) = beat(&t, "s1", t0() + 1_100, Some(0), INTERVAL_SECS) else {
            panic!()
        };
        assert_eq!(t.heartbeat.unwrap().busy_until, None);
    }

    #[test]
    fn changes_beat_along_only_for_the_holder() {
        let mut t = held("s1");
        along(&mut t, Some("s2"), t0());
        assert_eq!(t.heartbeat, None);
        along(&mut t, None, t0());
        assert_eq!(t.heartbeat, None);
        along(&mut t, Some("s1"), t0());
        assert_eq!(t.heartbeat.as_ref().unwrap().at, T0);
        // Released or waiting: no heartbeat.
        t.state = State::Waiting;
        along(&mut t, Some("s1"), t0() + 5);
        assert_eq!(t.heartbeat, None);
    }
}
