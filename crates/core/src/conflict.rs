//! Two claims of one task that met (Task-A01-lawn320). Two sessions claim the same ready task
//! at once, each in its own copy; the one that shares second has its push refused, and getting
//! the latest changes leaves the task's `task.json` with the version control tool's conflict
//! markers in it: both claims, side by side. This module only reads such a file; it never
//! resolves it (alm runs no version control tool).

use serde::Serialize;

use crate::model::{State, Task};
use crate::problem::Problem;

/// Whether `text` holds a version control tool's conflict markers (a line starting with
/// `<<<<<<<`, one with `=======` and one starting with `>>>>>>>`).
pub fn has_markers(text: &str) -> bool {
    let mut open = false;
    let mut middle = false;
    for line in text.lines() {
        if line.starts_with("<<<<<<<") {
            open = true;
        } else if open && line.trim_end() == "=======" {
            middle = true;
        } else if middle && line.starts_with(">>>>>>>") {
            return true;
        }
    }
    false
}

/// The file's two sides: the lines outside every conflict plus, inside each, the first part
/// (`<<<<<<<` to `=======`, without a `|||||||` base part) or the second (`=======` to
/// `>>>>>>>`). `None` when the file has no complete conflict.
pub fn sides(text: &str) -> Option<(String, String)> {
    if !has_markers(text) {
        return None;
    }
    #[derive(PartialEq)]
    enum At {
        Outside,
        First,
        Base,
        Second,
    }
    let (mut one, mut two) = (String::new(), String::new());
    let mut at = At::Outside;
    for line in text.lines() {
        match at {
            At::Outside if line.starts_with("<<<<<<<") => at = At::First,
            At::First | At::Base if line.trim_end() == "=======" => at = At::Second,
            At::First if line.starts_with("|||||||") => at = At::Base,
            At::Second if line.starts_with(">>>>>>>") => at = At::Outside,
            At::Outside => {
                one.push_str(line);
                one.push('\n');
                two.push_str(line);
                two.push('\n');
            }
            At::First => {
                one.push_str(line);
                one.push('\n');
            }
            At::Second => {
                two.push_str(line);
                two.push('\n');
            }
            At::Base => {}
        }
    }
    (at == At::Outside).then_some((one, two))
}

/// A task whose `task.json` holds two claims that met: both sides are in progress, held by two
/// different sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaimRace {
    pub id: String,
    /// The two holders, each with its side's last heartbeat (a claim is its first one), in the
    /// order the file has them.
    pub claims: Vec<Claim>,
    /// The session the claim lost to, when `--as` names one of the two: the other one, whose
    /// claim was shared first (it came in with the latest changes).
    pub held_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Claim {
    pub holder: String,
    pub at: Option<String>,
}

impl ClaimRace {
    /// The two sides of a task's conflicted file, read as tasks. `None` unless both are in
    /// progress and held by two different sessions. `me`: the `--as` session, if any.
    pub fn of(one: &Task, two: &Task, me: Option<&str>) -> Option<ClaimRace> {
        let held = |t: &Task| {
            (t.state == State::InProgress)
                .then(|| t.holder.clone())
                .flatten()
                .map(|holder| Claim {
                    holder,
                    at: t.heartbeat.as_ref().map(|h| h.at.clone()),
                })
        };
        let (a, b) = (held(one)?, held(two)?);
        if a.holder == b.holder {
            return None;
        }
        let held_by = match me {
            Some(me) if me == a.holder => Some(b.holder.clone()),
            Some(me) if me == b.holder => Some(a.holder.clone()),
            _ => None,
        };
        Some(ClaimRace {
            id: one.id.clone(),
            claims: vec![a, b],
            held_by,
        })
    }

    /// The notice `alm` gives first (a warning code, worded in every language).
    pub fn notice(&self) -> Problem {
        let when = |c: &Claim| c.at.clone().unwrap_or_else(|| "?".to_string());
        match &self.held_by {
            Some(holder) => {
                let c = self
                    .claims
                    .iter()
                    .find(|c| &c.holder == holder)
                    .expect("held_by is one of the claims");
                Problem::ClaimLost {
                    id: self.id.clone(),
                    holder: holder.clone(),
                    at: when(c),
                }
            }
            None => Problem::ClaimRace {
                id: self.id.clone(),
                first: self.claims[0].holder.clone(),
                first_at: when(&self.claims[0]),
                second: self.claims[1].holder.clone(),
                second_at: when(&self.claims[1]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MERGED: &str = "{\n  \"id\": \"A01-01\",\n<<<<<<< HEAD\n  \"holder\": \"s2\",\n=======\n  \"holder\": \"s1\",\n>>>>>>> abc123\n  \"state\": \"in_progress\"\n}\n";

    #[test]
    fn a_conflicted_file_splits_into_its_two_sides() {
        assert!(has_markers(MERGED));
        let (one, two) = sides(MERGED).unwrap();
        assert_eq!(
            one,
            "{\n  \"id\": \"A01-01\",\n  \"holder\": \"s2\",\n  \"state\": \"in_progress\"\n}\n"
        );
        assert!(two.contains("\"s1\"") && !two.contains("\"s2\""));
        // A base part (diff3 style) belongs to neither side.
        let diff3 = MERGED.replace("=======", "||||||| base\n  \"holder\": null,\n=======");
        assert_eq!(sides(&diff3).unwrap(), (one, two));
        // Plain JSON, or an unfinished conflict, has no sides.
        assert!(!has_markers("{\"id\": \"A01-01\"}"));
        assert_eq!(sides("<<<<<<< HEAD\n=======\n"), None);
    }

    fn task(holder: &str, at: &str) -> Task {
        serde_json::from_value(json!({
            "id": "A01-01", "title": "t", "area": "A01", "state": "in_progress",
            "holder": holder, "heartbeat": {"at": at, "by": holder}
        }))
        .unwrap()
    }

    #[test]
    fn the_session_whose_claim_lost_is_told_who_holds_the_task() {
        let (s2, s1) = (task("s2", "10:01"), task("s1", "10:00"));
        let race = ClaimRace::of(&s2, &s1, Some("s2")).unwrap();
        assert_eq!(race.held_by.as_deref(), Some("s1"));
        assert_eq!(race.notice().code(), "claim_lost");
        assert!(race.notice().english().contains("s1"));
        // Without --as, or as a third session: both claims, nobody chosen.
        let race = ClaimRace::of(&s2, &s1, None).unwrap();
        assert_eq!(race.held_by, None);
        assert_eq!(race.notice().code(), "claim_race");
        assert_eq!(ClaimRace::of(&s2, &s1, Some("s3")).unwrap().held_by, None);
        // The same holder on both sides is no race.
        assert_eq!(ClaimRace::of(&s1, &task("s1", "10:02"), None), None);
    }
}
