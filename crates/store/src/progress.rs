//! What an AI agent is doing on a job the app gave it (Task-A05-08, approver 2026-10-17).
//!
//! `alm progress` leaves one small file per agent in `<project>/.allomos/requests/progress/`:
//! the job's title, the step it is on, how far along it is, and at the end what happened. The
//! app, which watches the folder it shows, draws it as a progress popup that can go to the
//! background. `alm` only passes the words on; it checks nothing. Like `alm confirm`'s questions,
//! these are working files, never committed (the requests folder's `.gitignore` covers them).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use allomos_core::Problem;
use serde::{Deserialize, Serialize};

use crate::confirm::{ensure_requests_dir, requests_dir, write_file_whole};

/// Reports older than this are removed when listed: no agent is still on that job.
pub const FORGET_AFTER_SECS: u64 = 24 * 60 * 60;

/// One agent's progress on its job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Who reports (made from `--as`, or `agent`): one report per agent.
    pub id: String,
    /// The job, as the popup's heading.
    pub title: String,
    /// What the agent is doing now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Steps done, of `of`, when the agent counts them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<u32>,
    /// The job is over: what happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
    /// Who reports (`--as`), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// When the job started and when it last reported, in seconds since 1970.
    pub started: u64,
    pub updated: u64,
}

/// What one `alm progress` says.
#[derive(Debug, Clone, Default)]
pub struct Report<'a> {
    pub title: Option<&'a str>,
    pub step: Option<&'a str>,
    pub done: Option<u32>,
    pub of: Option<u32>,
    pub finished: Option<&'a str>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn progress_dir(project: &Path) -> PathBuf {
    requests_dir(project).join("progress")
}

/// The file name for `from`: letters, digits and dashes only, so it is always a plain name.
fn key(from: Option<&str>) -> String {
    let made: String = from
        .unwrap_or("agent")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let made = made.trim_matches('-').to_string();
    if made.is_empty() {
        "agent".to_string()
    } else {
        made.chars().take(64).collect()
    }
}

fn valid_key(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn read(path: &Path) -> Option<Progress> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Record `report` as `from`'s progress in `project`. A report while the agent's job is still
/// open updates it (a missing title, step or count keeps the last one); otherwise it starts a new
/// job, which needs a title. Refused: no title for a new job, `--done` above `--of`, or nothing
/// to say.
pub fn report(project: &Path, from: Option<&str>, report: &Report) -> Result<Progress, Problem> {
    let usage = |detail: &str| Problem::BadUsage {
        detail: detail.to_string(),
    };
    let clean = |s: Option<&str>| s.map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    let (title, step, finished) = (
        clean(report.title),
        clean(report.step),
        clean(report.finished),
    );
    if title.is_none() && step.is_none() && finished.is_none() && report.done.is_none() {
        return Err(usage(
            "say what the agent is doing: --step, --done/--of or --finished (and --title for a new job)",
        ));
    }
    let dir = progress_dir(project);
    let id = key(from);
    let path = dir.join(format!("{id}.json"));
    let open = read(&path).filter(|p| p.finished.is_none());
    let at = now();
    let mut p = match open {
        Some(p) => p,
        None => Progress {
            id: id.clone(),
            title: title
                .clone()
                .ok_or_else(|| usage("a new job needs a title: --title \"<the job>\""))?,
            step: None,
            done: None,
            of: None,
            finished: None,
            from: from.map(String::from),
            started: at,
            updated: at,
        },
    };
    if let Some(t) = title {
        p.title = t;
    }
    if step.is_some() {
        p.step = step;
    }
    if report.done.is_some() {
        p.done = report.done;
    }
    if report.of.is_some() {
        p.of = report.of;
    }
    if let (Some(d), Some(o)) = (p.done, p.of) {
        if d > o {
            return Err(usage(&format!("--done {d} is more than --of {o}")));
        }
    }
    p.finished = finished;
    p.updated = at;
    let write = |e: std::io::Error| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    ensure_requests_dir(&requests_dir(project)).map_err(write)?;
    fs::create_dir_all(&dir).map_err(write)?;
    let text = serde_json::to_string_pretty(&p).map_err(|e| write(std::io::Error::other(e)))?;
    write_file_whole(&path, &text).map_err(write)?;
    Ok(p)
}

/// Every agent's progress in `project`, the earliest started first. Reports not updated for a
/// day are removed.
pub fn reports(project: &Path) -> Vec<Progress> {
    let dir = progress_dir(project);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let at = now();
    let mut found: Vec<Progress> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?;
            if !valid_key(id) {
                return None;
            }
            let p = read(&e.path()).filter(|p| p.id == id)?;
            if at.saturating_sub(p.updated) > FORGET_AFTER_SECS {
                let _ = fs::remove_file(e.path());
                return None;
            }
            Some(p)
        })
        .collect();
    found.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.id.cmp(&b.id)));
    found
}

/// The person closed report `id` (finished, or its agent stopped reporting): remove it.
pub fn dismiss(project: &Path, id: &str) {
    if valid_key(id) {
        let _ = fs::remove_file(progress_dir(project).join(format!("{id}.json")));
    }
}

/// The jobs the app hands to an agent, for the progress popup's heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    Connect,
    Update,
    MoveIn,
    Cards,
    BuildApp,
}

/// `job`'s title in an interface language (`en`, `zh-CN`, `ja`; English otherwise).
pub fn job_title_in(language: &str, job: Job) -> &'static str {
    match (language, job) {
        ("zh-CN", Job::Connect) => "连接 AI 智能体",
        ("zh-CN", Job::Update) => "更新 Allomos 的说明",
        ("zh-CN", Job::MoveIn) => "搬入任务",
        ("zh-CN", Job::Cards) => "更新任务包",
        ("zh-CN", Job::BuildApp) => "构建应用",
        ("ja", Job::Connect) => "AI エージェントとつなぐ",
        ("ja", Job::Update) => "Allomos の説明を更新",
        ("ja", Job::MoveIn) => "タスクを移し入れる",
        ("ja", Job::Cards) => "タスクバッグを更新",
        ("ja", Job::BuildApp) => "アプリを作る",
        (_, Job::Connect) => "Connect with the agents",
        (_, Job::Update) => "Update Allomos's instructions",
        (_, Job::MoveIn) => "Move the tasks in",
        (_, Job::Cards) => "Update the taskbag",
        (_, Job::BuildApp) => "Build the app",
    }
}

/// What every job's request adds (Task-A05-08): keep the person informed in the app with
/// `alm progress` at each step, and say what happened at the end.
pub fn request_line_in(language: &str, job: Job) -> String {
    let title = job_title_in(language, job);
    match language {
        "zh-CN" => format!(
            "工作期间，请让用户在 Allomos 应用里看到进度：开始时运行 `alm progress --title \"{title}\" --step \"<正在做什么>\"`，之后每一步都运行 `alm progress --step \"<正在做什么>\" --done <已完成数> --of <总数>`（知道总数时），结束时运行 `alm progress --finished \"<做了什么>\"`。"
        ),
        "ja" => format!(
            "作業中は、Allomos アプリでユーザーに進み具合を見せてください：始めに `alm progress --title \"{title}\" --step \"<いましていること>\"`、その後は一歩ごとに `alm progress --step \"<いましていること>\" --done <済んだ数> --of <全体の数>`（全体の数が分かるとき）、終わったら `alm progress --finished \"<したこと>\"` を実行します。"
        ),
        _ => format!(
            "While you work, keep the person informed in the Allomos app: start with `alm progress --title \"{title}\" --step \"<what you are doing>\"`, then at each step run `alm progress --step \"<what you are doing>\" --done <steps done> --of <all steps>` (when you know the count), and at the end `alm progress --finished \"<what happened>\"`."
        ),
    }
}

/// `request` with [`request_line_in`] after it.
pub fn with_progress_in(language: &str, job: Job, request: &str) -> String {
    format!(
        "{}\n\n{}\n",
        request.trim_end(),
        request_line_in(language, job)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-progress-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_job_is_reported_step_by_step_then_finished_and_dismissed() {
        let dir = tmp("flow");
        let me = Some("claude/x y");
        // A new job needs a title.
        assert!(matches!(
            report(
                &dir,
                me,
                &Report {
                    step: Some("Looking"),
                    ..Default::default()
                }
            ),
            Err(Problem::BadUsage { .. })
        ));
        let p = report(
            &dir,
            me,
            &Report {
                title: Some("Update the taskbag"),
                step: Some("Looking"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p.id, "claude-x-y");
        let p = report(
            &dir,
            me,
            &Report {
                step: Some("Moving C01-02"),
                done: Some(1),
                of: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            (p.title.as_str(), p.done, p.of),
            ("Update the taskbag", Some(1), Some(3))
        );
        // The count can go up without repeating --of; above it is refused.
        assert_eq!(
            report(
                &dir,
                me,
                &Report {
                    done: Some(2),
                    ..Default::default()
                }
            )
            .unwrap()
            .of,
            Some(3)
        );
        assert!(matches!(
            report(
                &dir,
                me,
                &Report {
                    done: Some(4),
                    ..Default::default()
                }
            ),
            Err(Problem::BadUsage { .. })
        ));
        assert!(matches!(
            report(&dir, me, &Report::default()),
            Err(Problem::BadUsage { .. })
        ));
        // Another agent has its own report.
        report(
            &dir,
            None,
            &Report {
                title: Some("Build the app"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reports(&dir).len(), 2);
        let p = report(
            &dir,
            me,
            &Report {
                finished: Some("3 cards moved in"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p.finished.as_deref(), Some("3 cards moved in"));
        // A report after the end starts a new job (and needs a title again).
        assert!(matches!(
            report(
                &dir,
                me,
                &Report {
                    step: Some("Again"),
                    ..Default::default()
                }
            ),
            Err(Problem::BadUsage { .. })
        ));
        dismiss(&dir, "claude-x-y");
        dismiss(&dir, "../../etc");
        let left = reports(&dir);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, "agent");
        // Never committed.
        assert!(
            fs::read_to_string(requests_dir(&dir).join(".gitignore"))
                .unwrap()
                .contains('*')
        );
        // A report not updated for a day is forgotten.
        let mut old = left[0].clone();
        old.updated = 1;
        fs::write(
            progress_dir(&dir).join("agent.json"),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        assert!(reports(&dir).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_job_asks_for_progress_in_every_language() {
        for language in ["en", "zh-CN", "ja"] {
            for job in [
                Job::Connect,
                Job::Update,
                Job::MoveIn,
                Job::Cards,
                Job::BuildApp,
            ] {
                let r = with_progress_in(language, job, "Do it.\n");
                assert!(r.starts_with("Do it.\n\n"), "{language}");
                assert!(
                    r.contains(&format!("--title \"{}\"", job_title_in(language, job))),
                    "{language}"
                );
                assert!(r.contains("alm progress --finished"), "{language}");
                assert!(!r.contains('@'), "{language}");
            }
        }
    }
}
