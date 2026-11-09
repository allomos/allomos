//! Heartbeats (decision 0022, Task-A01-nurse164): the session holding a task shows it is still at
//! work by writing the time of its last heartbeat into the task's own file. A heartbeat is no
//! event: burndown, timeline and history do not change (R7), and a task's version
//! ([`crate::ops::task_version`]) leaves it out, so a beat never makes another writer's copy out of
//! date.
//!
//! `alm beat` writes one ([`beat`]); every change the holder makes to its task writes one along
//! with it ([`along`]); read-only commands write none.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project, State, Task};
use crate::problem::Problem;

/// How often a holding session beats, by default (decision 0022 § 3): 10 minutes. `alm beat`
/// writes nothing while the last heartbeat is younger than half of it.
pub const INTERVAL_SECS: i64 = 10 * 60;

/// After how long without a heartbeat a held task is lost, by default (decision 0022 § 3): 30
/// minutes.
pub const LOST_AFTER_SECS: i64 = 30 * 60;

/// `project.json`'s `heartbeat` (decision 0022 § 3, Task-A01-zero387): the project's two numbers,
/// in whole minutes. A missing one takes its default; `alm heartbeat-settings` changes them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// How often a holding session beats. Missing: 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_minutes: Option<u32>,
    /// After how long without a heartbeat (and no busy window running) a held task is lost.
    /// Missing: 30.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost_after_minutes: Option<u32>,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The two numbers as they apply, defaults filled in, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub interval_secs: i64,
    pub lost_after_secs: i64,
}

impl Default for Applied {
    fn default() -> Self {
        Applied {
            interval_secs: INTERVAL_SECS,
            lost_after_secs: LOST_AFTER_SECS,
        }
    }
}

/// The project's heartbeat numbers as they apply. A value of 0 (refused when written, reported by
/// validation) is read as the default.
pub fn applied(project: &Project) -> Applied {
    let s = project.heartbeat.clone().unwrap_or_default();
    let pick = |v: Option<u32>, default: i64| {
        v.filter(|m| *m > 0)
            .map(|m| i64::from(m) * 60)
            .unwrap_or(default)
    };
    Applied {
        interval_secs: pick(s.interval_minutes, INTERVAL_SECS),
        lost_after_secs: pick(s.lost_after_minutes, LOST_AFTER_SECS),
    }
}

/// A change to the heartbeat settings (`alm heartbeat-settings`): each given length (`10m`, `1h`,
/// or a bare number of minutes) replaces that number; `clear` first goes back to the defaults.
#[derive(Debug, Default)]
pub struct SettingsChange<'a> {
    pub interval: Option<&'a str>,
    pub lost_after: Option<&'a str>,
    pub clear: bool,
}

/// A length given for a heartbeat setting, in whole minutes (at least 1): `10m`, `1h`, `1h30m`, or
/// a bare number of minutes. `field` names the setting in the refusal.
pub fn parse_minutes(field: &str, text: &str) -> Result<u32, Problem> {
    let t = text.trim();
    let secs = if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        t.parse::<i64>().ok().and_then(|m| m.checked_mul(60))
    } else {
        parse_duration(t)
    };
    secs.filter(|s| *s >= 60 && s % 60 == 0)
        .and_then(|s| u32::try_from(s / 60).ok())
        .ok_or_else(|| Problem::HeartbeatMinutesInvalid {
            field: field.to_string(),
            value: text.to_string(),
        })
}

/// The project with its heartbeat settings changed; `heartbeat` goes away when both are back to
/// their defaults and nothing unknown is kept.
pub fn set_settings(project: &Project, change: &SettingsChange) -> Result<Project, Problem> {
    let interval = change
        .interval
        .map(|v| parse_minutes("--interval", v))
        .transpose()?;
    let lost = change
        .lost_after
        .map(|v| parse_minutes("--lost-after", v))
        .transpose()?;
    let mut s = project.heartbeat.clone().unwrap_or_default();
    if change.clear {
        s.interval_minutes = None;
        s.lost_after_minutes = None;
    }
    if interval.is_some() {
        s.interval_minutes = interval;
    }
    if lost.is_some() {
        s.lost_after_minutes = lost;
    }
    let mut project = project.clone();
    project.heartbeat = (s != Settings::default()).then_some(s);
    Ok(project)
}

/// A held task that is lost (decision 0022 § 3): `in_progress`, held, with no heartbeat by its
/// holder for longer than the project's limit and no busy window still running. `alm` only lists
/// it; the session decides whether to take it over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lost {
    pub id: String,
    pub holder: String,
    /// The holder's last heartbeat, in UTC; `None` when it never beat (a task claimed before
    /// heartbeats, or by an older Allomos).
    pub heartbeat_at: Option<String>,
    /// The end of the holder's last busy window, when it declared one (it is over).
    pub busy_until: Option<String>,
    /// The time the limit counts from: the later of the heartbeat and the busy window's end, or,
    /// with neither, the task's last recorded change.
    pub since: String,
    /// Seconds from `since` to now.
    pub silent_seconds: i64,
    /// The limit that applied, in seconds.
    pub lost_after_seconds: i64,
}

/// Whether `task` is lost at `now` (seconds since 1970, UTC). `last_change`: the time of the
/// task's newest recorded event, used when the holder never beat. A task that is not
/// `in_progress` or has no holder (waiting, blocked, released, ready, done, dropped) is never
/// lost, nor one whose silence cannot be dated.
pub fn lost(task: &Task, now: i64, lost_after: i64, last_change: Option<&str>) -> Option<Lost> {
    if task.state != State::InProgress {
        return None;
    }
    let holder = task.holder.as_deref()?;
    // Only the holder's own heartbeat counts (one left by a former holder does not).
    let mine = task.heartbeat.as_ref().filter(|h| h.by == holder);
    let at = mine.and_then(|h| parse_utc(&h.at));
    let until = mine
        .and_then(|h| h.busy_until.as_deref())
        .and_then(parse_utc);
    if until.is_some_and(|u| u > now) {
        return None;
    }
    let since = match at.max(until) {
        Some(t) => t,
        None => last_change.and_then(parse_utc)?,
    };
    let silent = now - since;
    (silent > lost_after).then(|| Lost {
        id: task.id.clone(),
        holder: holder.to_string(),
        heartbeat_at: mine.map(|h| h.at.clone()),
        busy_until: mine.and_then(|h| h.busy_until.clone()),
        since: format_utc(since),
        silent_seconds: silent,
        lost_after_seconds: lost_after,
    })
}

/// The end of the busy window the holder of `task` declared, while it still runs at `now`.
pub fn busy_until(task: &Task, now: i64) -> Option<String> {
    if task.state != State::InProgress {
        return None;
    }
    let holder = task.holder.as_deref()?;
    let h = task.heartbeat.as_ref().filter(|h| h.by == holder)?;
    let until = h.busy_until.as_ref()?;
    parse_utc(until)
        .is_some_and(|u| u > now)
        .then(|| until.clone())
}

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

    fn project(heartbeat: serde_json::Value) -> Project {
        serde_json::from_value(serde_json::json!({
            "format": crate::FORMAT, "id": "p", "name": "P", "heartbeat": heartbeat
        }))
        .unwrap()
    }

    #[test]
    fn settings_default_change_and_clear() {
        let p: Project = serde_json::from_value(
            serde_json::json!({ "format": crate::FORMAT, "id": "p", "name": "P" }),
        )
        .unwrap();
        assert_eq!(applied(&p), Applied::default());
        let change = SettingsChange {
            interval: Some("15m"),
            lost_after: Some("1h"),
            clear: false,
        };
        let p2 = set_settings(&p, &change).unwrap();
        assert_eq!(
            applied(&p2),
            Applied {
                interval_secs: 900,
                lost_after_secs: 3_600
            }
        );
        let s = p2.heartbeat.clone().unwrap();
        assert_eq!(
            (s.interval_minutes, s.lost_after_minutes),
            (Some(15), Some(60))
        );
        // A bare number is minutes.
        let p3 = set_settings(
            &p2,
            &SettingsChange {
                lost_after: Some("45"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(applied(&p3).lost_after_secs, 2_700);
        assert_eq!(applied(&p3).interval_secs, 900);
        let cleared = set_settings(
            &p3,
            &SettingsChange {
                clear: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.heartbeat, None);
        for bad in ["0m", "90s", "x", "", "-5"] {
            assert!(
                matches!(
                    set_settings(
                        &p,
                        &SettingsChange {
                            interval: Some(bad),
                            ..Default::default()
                        }
                    ),
                    Err(Problem::HeartbeatMinutesInvalid { .. })
                ),
                "{bad}"
            );
        }
        // A 0 written by hand is read as the default; unknown fields stay.
        let odd = project(serde_json::json!({ "interval_minutes": 0, "later": 1 }));
        assert_eq!(applied(&odd).interval_secs, INTERVAL_SECS);
        let kept = set_settings(
            &odd,
            &SettingsChange {
                clear: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(kept.heartbeat.unwrap().extra["later"], 1);
    }

    #[test]
    fn a_task_is_lost_after_the_limit_without_a_heartbeat_or_busy_window() {
        let Ok(Beat::Written(t)) = beat(&held("s1"), "s1", t0(), None, INTERVAL_SECS) else {
            panic!()
        };
        assert_eq!(
            lost(&t, t0() + LOST_AFTER_SECS, LOST_AFTER_SECS, None),
            None
        );
        let l = lost(&t, t0() + LOST_AFTER_SECS + 1, LOST_AFTER_SECS, None).unwrap();
        assert_eq!(l.holder, "s1");
        assert_eq!(l.heartbeat_at.as_deref(), Some(T0));
        assert_eq!(l.since, T0);
        assert_eq!(l.silent_seconds, LOST_AFTER_SECS + 1);
        // The project's own limit.
        assert!(lost(&t, t0() + 700, 600, None).is_some());
        // A busy window still running: not lost, however old the heartbeat.
        let Ok(Beat::Written(b)) = beat(&t, "s1", t0(), Some(7_200), INTERVAL_SECS) else {
            panic!()
        };
        assert_eq!(lost(&b, t0() + 7_000, LOST_AFTER_SECS, None), None);
        assert_eq!(
            busy_until(&b, t0() + 7_000).as_deref(),
            Some("2026-11-09T16:00:00Z")
        );
        // Once over, the limit counts from its end.
        assert_eq!(lost(&b, t0() + 7_200 + 1_800, LOST_AFTER_SECS, None), None);
        let l = lost(&b, t0() + 7_200 + 1_801, LOST_AFTER_SECS, None).unwrap();
        assert_eq!(l.since, "2026-11-09T16:00:00Z");
        assert_eq!(l.busy_until.as_deref(), Some("2026-11-09T16:00:00Z"));
        assert_eq!(busy_until(&b, t0() + 7_201), None);
    }

    #[test]
    fn only_held_tasks_in_progress_are_lost_and_never_beating_counts_from_the_last_change() {
        let t = held("s1");
        // Never beat: from the last change; with nothing to date it by, not listed.
        assert_eq!(lost(&t, t0() + 9_999, LOST_AFTER_SECS, None), None);
        let l = lost(&t, t0() + 1_801, LOST_AFTER_SECS, Some(T0)).unwrap();
        assert_eq!((l.heartbeat_at, l.since.as_str()), (None, T0));
        // A former holder's heartbeat does not count for the new one.
        let Ok(Beat::Written(mut other)) = beat(&t, "s1", t0() + 1_700, None, INTERVAL_SECS) else {
            panic!()
        };
        other.holder = Some("s2".into());
        assert_eq!(
            lost(&other, t0() + 1_801, LOST_AFTER_SECS, Some(T0))
                .unwrap()
                .heartbeat_at,
            None
        );
        for state in [State::Waiting, State::Blocked, State::Ready, State::Done] {
            let mut s = t.clone();
            s.state = state;
            assert_eq!(lost(&s, t0() + 9_999, LOST_AFTER_SECS, Some(T0)), None);
        }
        let mut released = t.clone();
        released.holder = None;
        assert_eq!(
            lost(&released, t0() + 9_999, LOST_AFTER_SECS, Some(T0)),
            None
        );
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
