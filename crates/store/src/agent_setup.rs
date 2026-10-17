//! An AI agent not yet set up for the project is one more thing **Update the taskbag…** does
//! (Task-A03-24, approver 2026-10-17): the app finds the agents on this computer with no line
//! under `## Agents set up` in the project notes ([`crate::handoff::agents_set_up`]) and names them
//! here; the request asks the chosen agent to follow `alm guide agent-files` once and record each.
//! `alm` checks nothing about them: it tells, the agent checks.

use std::path::Path;

use crate::convert::{agent_convert_request_in, agent_move_in_request_in, migration_header_in};
use crate::{cards, shown, versions};

/// Changes when the text of [`agents_set_up_request_in`] changes.
pub const AGENTS_SET_UP_REQUEST_VERSION: u32 = 1;

/// The request for `agents` (names such as `Codex`), in an interface language (`en`, `zh-CN`,
/// `ja`; English otherwise).
pub fn agents_set_up_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    agents: &[String],
) -> String {
    let header = migration_header_in(language, project, taskbag);
    let guide = format!("alm --taskbag \"{}\" guide agent-files", shown(taskbag));
    let notes = shown(&taskbag.join("project-notes.md"));
    let version = AGENTS_SET_UP_REQUEST_VERSION;
    match language {
        "zh-CN" => {
            let names = agents.join("、");
            format!(
                "请为这个项目的 AI 智能体设置一次共用规则（智能体设置请求 v{version}）。

{header}

Allomos 在这台电脑上找到的这些 AI 智能体，在任务包的项目笔记（{notes}）的 `## Agents set up` 一节里还没有记录：{names}。

1. 运行 `{guide}` 并照做一次：为你自己，也为项目文件夹里已有的每个智能体规则文件。
2. 在 `## Agents set up` 一节下，按指南的格式（智能体、它的文件、日期）为上面列出的每个智能体各加一行；如果你不在其中，也为你自己加一行。没有自己的文件、直接读取 AGENTS.md 的智能体也这样记录（例如 \"Codex: reads AGENTS.md directly; no file of its own.\"）。
3. 按指南提交；除非用户要求，不要推送。

上面列出的每个智能体都有了自己的一行后，Allomos 里的提示就会消失。
"
            )
        }
        "ja" => {
            let names = agents.join("、");
            format!(
                "このプロジェクトの AI エージェントのために、共通ルールを一度設定してください（エージェント設定の依頼 v{version}）。

{header}

Allomos がこのコンピューターで見つけた次の AI エージェントは、タスクバッグのプロジェクトノート（{notes}）の `## Agents set up` の節にまだ記録がありません：{names}。

1. `{guide}` を実行し、そのとおりに一度行います：自分自身について、そしてプロジェクトのフォルダーにすでにあるすべてのエージェント用ルールファイルについて。
2. `## Agents set up` の節に、ガイドの書き方（エージェント、そのファイル、日付）で、上に挙げた各エージェントについて一行ずつ加えます。自分がその中にいなければ、自分についても一行加えます。自分のファイルを持たず AGENTS.md を直接読むエージェントもそのように記録します（例：\"Codex: reads AGENTS.md directly; no file of its own.\"）。
3. ガイドのとおりにコミットします。頼まれない限りプッシュしません。

上に挙げた各エージェントの行がそろうと、Allomos の通知は消えます。
"
            )
        }
        _ => {
            let names = agents.join(", ");
            format!(
                "Please set up the shared rules for this project's AI agents, once (agent setup request v{version}).

{header}

Allomos found these AI agents on this computer with no line yet under `## Agents set up` in the taskbag's project notes ({notes}): {names}.

1. Run `{guide}` and follow it once: for yourself and for every agent rule file already in the project folder.
2. Under `## Agents set up`, add one line for each agent named above, as the guide says (agent, its file, date), and one for yourself if you are not among them. An agent with no file of its own that reads AGENTS.md directly is recorded so too (for example \"Codex: reads AGENTS.md directly; no file of its own.\").
3. Commit as the guide says; do not push unless the person asks.

The notice in Allomos goes away once each agent named above has its line.
"
            )
        }
    }
}

/// What **Update the taskbag…** asks of the agent the person chooses, from the app, as one request
/// (approver 2026-10-17: one request covers every update due):
/// - an older taskbag's tasks to move in ([`agent_move_in_request_in`]); else
/// - a taskbag with no tasks: look for the tasks the project already keeps and move them in after
///   the person's Go ahead ([`agent_convert_request_in`]; approver 2026-10-17: a connected project
///   with no tasks is told to update the taskbag, not to connect again); else
/// - card files kept outside the taskbag
///   ([`agent_move_cards_request_in`](cards::agent_move_cards_request_in)), when some are known or
///   nothing else is to be done (the agent still looks for card files no link names);
/// - then, when `agents` names agents not yet set up, [`agents_set_up_request_in`] after it.
pub fn agent_update_taskbag_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    agents: &[String],
) -> String {
    let no_tasks = crate::load(taskbag)
        .taskbag
        .is_some_and(|bag| bag.tasks.is_empty());
    let first = match versions::move_in(taskbag) {
        Some(m) => Some(agent_move_in_request_in(language, project, taskbag, &m)),
        None if no_tasks => Some(agent_convert_request_in(language, project, taskbag)),
        None if agents.is_empty() || !cards::outside(taskbag).is_empty() => Some(
            cards::agent_move_cards_request_in(language, project, taskbag),
        ),
        None => None,
    };
    let setup =
        (!agents.is_empty()).then(|| agents_set_up_request_in(language, project, taskbag, agents));
    match (first, setup) {
        (Some(first), Some(setup)) => format!("{}\n\n---\n\n{setup}", first.trim_end()),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => unreachable!("no agents means the move-cards request"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, init};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn project(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-agent-setup-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn the_setup_request_names_the_agents_the_guide_and_the_taskbag() {
        let p = PathBuf::from("/work/bakery");
        let tb = p.join(DEFAULT_DIR);
        let agents = ["Claude".to_string(), "Codex".to_string()];
        for language in ["en", "zh-CN", "ja"] {
            let r = agents_set_up_request_in(language, &p, &tb, &agents);
            assert!(
                r.contains(&format!(
                    "alm --taskbag \"{}\" guide agent-files",
                    shown(&tb)
                )),
                "{language}"
            );
            assert!(r.contains("`## Agents set up`"), "{language}");
            assert!(r.contains("Claude") && r.contains("Codex"), "{language}");
            assert!(
                r.contains(&shown(&tb.join("project-notes.md"))),
                "{language}"
            );
            assert!(r.contains("v1"), "{language}");
        }
    }

    #[test]
    fn update_the_taskbag_adds_the_setup_part_only_when_agents_are_named() {
        let p = project("update");
        let tb = p.join(DEFAULT_DIR);
        init(&tb, "p", "P").unwrap();
        let none: [String; 0] = [];
        let codex = ["Codex".to_string()];
        // No tasks yet: look for the project's own tasks, and set up the agents named.
        let r = agent_update_taskbag_request_in("en", &p, &tb, &none);
        assert!(
            r.contains("conversion request") && !r.contains("agent setup request"),
            "{r}"
        );
        let r = agent_update_taskbag_request_in("en", &p, &tb, &codex);
        assert!(r.contains("conversion request") && r.contains("agent setup request"));
        // With a task: nobody to set up is the move-cards request, as before.
        crate::add_area(
            &tb,
            crate::Actor::APP,
            std::time::Duration::from_secs(1),
            Some("A1"),
            "Area",
        )
        .unwrap();
        crate::add_task(
            &tb,
            crate::Actor::APP,
            std::time::Duration::from_secs(1),
            &serde_json::from_str(r#"{"area":"A1","title":"Bake"}"#).unwrap(),
        )
        .unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb, &none);
        assert!(r.contains("move-cards request") && !r.contains("agent setup request"));
        // Agents to set up and no card file outside: the setup request alone.
        let r = agent_update_taskbag_request_in("en", &p, &tb, &codex);
        assert!(r.contains("agent setup request") && !r.contains("move-cards request"));
        assert!(!r.contains("conversion request") && r.contains(": Codex."));
    }
}
