//! Card files kept beside their tasks, on disk (Task-A00-30): what agents reported as still in
//! the project folder, recording such reports, and moving a card file into `tasks/<id>/card.md`.
//! The rules are in `allomos_core::card`; nothing here looks for card files by itself.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use allomos_core::card::{self, Outside};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task};

use crate::convert::migration_header_in;
use crate::events::{self, Actor};
use crate::{lf, lock_for_change, read_text, shown, versions, write_atomic};

/// Changes when the text of [`agent_move_cards_request_in`] changes.
pub const MOVE_CARDS_REQUEST_VERSION: u32 = 1;

/// What **Move the tasks in…** types into the agent the person chooses when the taskbag is not
/// from an older Allomos (Task-A00-30; one button since Task-A01-45, through
/// [`agent_move_tasks_in_request_in`](crate::agent_move_tasks_in_request_in)), in an
/// interface language (`en`, `zh-CN`, `ja`; English otherwise): find the card files the project
/// still keeps in its own folder and move them in beside their tasks, following
/// `alm guide move-cards`, with the person's Go ahead in the app. Nothing is sent: the person
/// sends it.
pub fn agent_move_cards_request_in(language: &str, project: &Path, taskbag: &Path) -> String {
    let header = migration_header_in(language, project, taskbag);
    let guide = format!("alm --taskbag \"{}\" guide move-cards", shown(taskbag));
    let version = MOVE_CARDS_REQUEST_VERSION;
    match language {
        "zh-CN" => format!(
            "请把这个项目的卡片文件搬进 Allomos 的任务包，放在各自任务的旁边（搬卡片请求 v{version}）。

{header}

按 `{guide}` 打印的步骤做：先用 `alm cards` 看其他智能体已经报告的卡片文件，自己找出其余的，用 `alm cards found` 报告；然后用 `alm confirm` 在 Allomos 应用里给用户看完整计划。用户选择“继续”之前，什么都不要改。
"
        ),
        "ja" => format!(
            "このプロジェクトのカードのファイルを、Allomos のタスクバッグの各タスクの隣に移してください（カードの移し入れの依頼 v{version}）。

{header}

`{guide}` が表示する手順に従ってください：まず `alm cards` でほかのエージェントがすでに報告したカードのファイルを見て、残りを自分で探し、`alm cards found` で報告します。続いて `alm confirm` で計画全体を Allomos アプリでユーザーに見せます。ユーザーが「進める」を選ぶまで何も変えないでください。
"
        ),
        _ => format!(
            "Please move this project's card files into Allomos's taskbag, beside their tasks (move-cards request v{version}).

{header}

Follow what `{guide}` prints: first see what `alm cards` lists (card files agents already reported), find any others yourself and report them with `alm cards found`; then show the whole plan to the person in the Allomos app with `alm confirm`. Change nothing until the person chooses Go ahead.
"
        ),
    }
}

/// The card files agents reported as still in the project folder, for tasks this taskbag has,
/// in task order (the notice "N tasks keep their card files in the project folder").
pub fn outside(root: &Path) -> Vec<Outside> {
    let reported = card::outside(&events::list(root).0);
    if reported.is_empty() {
        return Vec::new();
    }
    let Some(bag) = crate::load(root).taskbag else {
        return Vec::new();
    };
    bag.tasks
        .iter()
        .filter_map(|t| reported.get(&t.id).cloned())
        .collect()
}

/// What [`report`] recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct Reported {
    /// The reports this change added or changed.
    pub found: Vec<Outside>,
    /// The tasks whose report this change withdrew.
    pub withdrawn: Vec<String>,
    /// Every card file still reported after the change.
    pub outside: Vec<Outside>,
    pub issues: Vec<Issue>,
}

/// Record the card files an agent found in the project folder (`alm cards found <id>=<path>…`):
/// one `cards found` event per task whose report changes, under the lock. `path` is relative to
/// the project folder; an empty one withdraws the task's report. All or nothing: a bad path, a
/// file that is not there, or one inside the taskbag refuses the whole report.
pub fn report(
    root: &Path,
    actor: Actor,
    wait: Duration,
    items: &[(String, String)],
) -> Result<Reported, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let project = versions::project_folder(root);
    let before = card::outside(&events::list(root).0);
    let inside_taskbag = root.canonicalize().ok();
    let mut planned = Vec::new();
    for (id, path) in items {
        let changes = card::report(&bag, id, path, before.get(id).map(|o| o.path.as_str()))?;
        if changes.is_empty() {
            continue;
        }
        if !path.trim().is_empty() {
            let clean = card::clean_path(path)?;
            let file = project.join(&clean);
            if !file.is_file() {
                return Err(Problem::CardNotFound { path: shown(&file) });
            }
            let real = file.canonicalize().ok();
            if let (Some(real), Some(bag)) = (&real, &inside_taskbag) {
                if real.starts_with(bag) {
                    return Err(Problem::CardPathInvalid { path: clean });
                }
            }
        }
        planned.push((id.clone(), changes));
    }
    for (id, changes) in planned {
        let event = actor.event(card::FOUND, Kind::Task, Some(&id), changes);
        lock.record(&event, |_| Ok(()))?;
    }
    drop(lock);
    let after = card::outside(&events::list(root).0);
    let mut found = Vec::new();
    let mut withdrawn = Vec::new();
    for (id, _) in items {
        match (before.get(id), after.get(id)) {
            (b, Some(a)) if b != Some(a) => found.push(a.clone()),
            (Some(_), None) => withdrawn.push(id.clone()),
            _ => {}
        }
    }
    found.dedup_by(|a, b| a.task == b.task);
    withdrawn.dedup();
    Ok(Reported {
        found,
        withdrawn,
        outside: outside(root),
        issues,
    })
}

/// What [`move_in`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Moved {
    pub task: Task,
    /// The card file in the taskbag, relative to it: `tasks/<id>/card.md`.
    pub card: String,
    /// The file it was read from.
    pub source: PathBuf,
    /// What had been reported for the task, relative to the project folder, now cleared.
    pub reported: Option<String>,
    /// The reported file in the project folder, which the agent removes as its plan says
    /// (`alm` never removes anything outside the taskbag).
    pub original: Option<PathBuf>,
    /// The project folder the reported path is relative to.
    pub project: PathBuf,
    pub issues: Vec<Issue>,
}

/// Move task `id`'s card file in (`alm card <id> [--from <file>]`): write it as
/// `tasks/<id>/card.md`, point the description's `Card:` link to it, and clear the task's report,
/// in one `card` event, under the lock. The text comes from `from` (an agent may first fix the
/// card's links there), else from the file reported for the task. A different `card.md` already
/// there is replaced only with `replace` (`alm card --replace`, to keep a moved card up to date),
/// else refused (`card_exists`); the same text again only fixes the link.
pub fn move_in(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    from: Option<&Path>,
    replace: bool,
) -> Result<Moved, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let reported = card::outside(&events::list(root).0)
        .remove(id)
        .map(|o| o.path);
    // An unknown task is named as such before anything about its card.
    allomos_core::ops::find_task(&bag, id)?;
    let project = versions::project_folder(root);
    let original = reported.as_ref().map(|p| project.join(p));
    let source = match (from, &original) {
        (Some(f), _) => f.to_path_buf(),
        (None, Some(o)) => o.clone(),
        (None, None) => return Err(Problem::CardNotReported { id: id.to_string() }),
    };
    let text = read_text(&source).map_err(|_| Problem::CardNotFound {
        path: shown(&source),
    })?;
    if text.trim().is_empty() {
        return Err(Problem::CardEmpty {
            path: shown(&source),
        });
    }
    let target = root.join("tasks").join(id).join(card::FILE_NAME);
    let (same, replacing) = match fs::read_to_string(&target) {
        Ok(old) if lf(&old) == lf(&text) => (true, false),
        Ok(_) if replace => (false, true),
        Ok(_) => return Err(Problem::CardExists { id: id.to_string() }),
        Err(_) => (false, false),
    };
    let (outcome, mut changes) = card::move_in(&bag, id, actor.by, reported.as_deref())?;
    if same {
        changes.retain(|c| c.field != card::CARD);
    }
    if replacing {
        // History shows "card.md → card.md": the card's text changed.
        for c in changes.iter_mut().filter(|c| c.field == card::CARD) {
            c.from = c.to.clone();
        }
    }
    let before = bag.tasks.iter().find(|t| t.id == id);
    let described = before.is_some_and(|b| b.description != outcome.task.description);
    let event = actor.event(card::MOVED, Kind::Task, Some(id), changes);
    lock.record(&event, |l| {
        if !same {
            write_atomic(&target, &text)?;
        }
        if described {
            l.save_description(&outcome.task)?;
        }
        Ok(())
    })?;
    Ok(Moved {
        task: outcome.task,
        card: format!("tasks/{id}/{}", card::FILE_NAME),
        source,
        reported,
        original,
        project,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewTask, add_area, add_task, init};

    const WAIT: Duration = Duration::from_millis(500);

    /// A project folder with a taskbag in `.allomos/taskbag` and one card file.
    fn project(name: &str) -> (PathBuf, PathBuf) {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("allomos-cards-{name}-{n}"));
        let root = dir.join(".allomos/taskbag");
        init(&root, "p", "P").unwrap();
        add_area(&root, Actor::alm(None), WAIT, Some("A01"), "Area").unwrap();
        add_task(
            &root,
            Actor::alm(None),
            WAIT,
            &NewTask {
                id: Some("A01-01".into()),
                area: "A01".into(),
                title: "First".into(),
                description: Some(
                    "Card: [docs/tasks/A01-01.md](../../../../docs/tasks/A01-01.md)\n\nMore."
                        .into(),
                ),
                ..Default::default()
            },
        )
        .unwrap();
        fs::create_dir_all(dir.join("docs/tasks")).unwrap();
        fs::write(dir.join("docs/tasks/A01-01.md"), "# A01-01\n\nThe card.\n").unwrap();
        (dir, root)
    }

    #[test]
    fn a_reported_card_moves_in_beside_its_task() {
        let (dir, root) = project("move");
        let me = Actor::alm(Some("claude/x"));
        assert!(matches!(
            report(
                &root,
                me,
                WAIT,
                &[("A01-01".into(), "docs/tasks/none.md".into())]
            ),
            Err(Problem::CardNotFound { .. })
        ));
        assert!(matches!(
            report(
                &root,
                me,
                WAIT,
                &[("A01-01".into(), "../outside.md".into())]
            ),
            Err(Problem::CardPathInvalid { .. })
        ));
        let r = report(
            &root,
            me,
            WAIT,
            &[("A01-01".into(), "docs/tasks/A01-01.md".into())],
        )
        .unwrap();
        assert_eq!(r.found.len(), 1);
        assert_eq!(outside(&root)[0].path, "docs/tasks/A01-01.md");

        let moved = move_in(&root, me, WAIT, "A01-01", None, false).unwrap();
        assert_eq!(moved.card, "tasks/A01-01/card.md");
        assert_eq!(
            fs::read_to_string(root.join("tasks/A01-01/card.md")).unwrap(),
            "# A01-01\n\nThe card.\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("tasks/A01-01/description.md")).unwrap(),
            "Card: [card.md](card.md)\n\nMore.\n"
        );
        assert!(outside(&root).is_empty());
        // `alm` never removes the original; the agent does, as its plan says.
        assert!(dir.join("docs/tasks/A01-01.md").is_file());
        // The link opens the moved file.
        assert!(matches!(
            crate::resolve_task_link(&root, "A01-01", "card.md"),
            Ok(crate::LinkTarget::File(_))
        ));
        // A card inside the taskbag is not a card outside it.
        assert!(matches!(
            report(
                &root,
                me,
                WAIT,
                &[(
                    "A01-01".into(),
                    ".allomos/taskbag/tasks/A01-01/card.md".into()
                )]
            ),
            Err(Problem::CardPathInvalid { .. })
        ));
        // Moving the same text again changes nothing; a different one is never replaced.
        move_in(
            &root,
            me,
            WAIT,
            "A01-01",
            Some(&dir.join("docs/tasks/A01-01.md")),
            false,
        )
        .unwrap();
        fs::write(dir.join("other.md"), "Other\n").unwrap();
        assert!(matches!(
            move_in(
                &root,
                me,
                WAIT,
                "A01-01",
                Some(&dir.join("other.md")),
                false
            ),
            Err(Problem::CardExists { .. })
        ));
        // Kept up to date on purpose.
        move_in(&root, me, WAIT, "A01-01", Some(&dir.join("other.md")), true).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("tasks/A01-01/card.md")).unwrap(),
            "Other\n"
        );
        // Without --from and no report: refused.
        fs::remove_file(root.join("tasks/A01-01/card.md")).unwrap();
        assert!(matches!(
            move_in(&root, me, WAIT, "A01-01", None, false),
            Err(Problem::CardNotReported { .. })
        ));
        // The taskbag still loads without problems: card.md is not a task file.
        assert!(crate::load(&root).issues.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_request_names_the_guide_and_the_go_ahead_in_every_language() {
        let project = PathBuf::from("/work/app");
        let taskbag = project.join(".allomos/taskbag");
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_move_cards_request_in(language, &project, &taskbag);
            assert!(r.contains("guide move-cards"), "{language}");
            assert!(r.contains("alm cards found"), "{language}");
            assert!(r.contains("alm confirm"), "{language}");
            assert!(r.contains(&shown(&project)), "{language}");
            assert!(
                r.contains(&format!("v{MOVE_CARDS_REQUEST_VERSION}")),
                "{language}"
            );
            assert!(
                !r.contains('@'),
                "the Claude app shows a typed-in @ as ＠: {language}"
            );
        }
    }

    #[test]
    fn a_report_can_be_withdrawn() {
        let (dir, root) = project("withdraw");
        let me = Actor::alm(Some("claude/x"));
        report(
            &root,
            me,
            WAIT,
            &[("A01-01".into(), "docs/tasks/A01-01.md".into())],
        )
        .unwrap();
        let r = report(&root, me, WAIT, &[("A01-01".into(), String::new())]).unwrap();
        assert_eq!(r.withdrawn, ["A01-01"]);
        assert!(outside(&root).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
