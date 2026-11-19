//! What a monitored session is asked, and what the monitor says, in the task language
//! (Task-A03-cow112, moved from the interface's `monitorFlow.ts` and its texts). Languages other
//! than Chinese and Japanese get English.

use crate::session_tab::CloseTexts;

/// The three languages the monitor speaks: the task language when it is one of them, else English.
pub(crate) fn lang(language: &str) -> &'static str {
    match language {
        "zh-CN" | "zh" | "zh-Hans" => "zh-CN",
        "ja" => "ja",
        _ => "en",
    }
}

/// The name an agent passes to `alm claim --session-agent`.
pub fn session_agent(agent: &str) -> &'static str {
    match crate::accounts::base_agent(agent) {
        "claude_cli" => "claude-code",
        "antigravity_cli" => "antigravity",
        _ => "codex",
    }
}

/// How a terminal Claude Code session reads its quota (Task-A03-folk351): `claude -p /usage`.
/// Empty for the other agents. The paragraph ends with a blank line.
fn usage_paragraph(agent: &str, language: &str) -> String {
    if crate::accounts::base_agent(agent) != "claude_cli" {
        return String::new();
    }
    let text = match lang(language) {
        "zh-CN" => {
            r#"配额：这个终端会话没有 get_usage。AGENTS.md 或 alm 要你报告配额时，运行 claude -p /usage（Claude Code 自己回答，不问模型），按它的两行报告：“Current session” 用 alm quota --agent claude-code --left <100 减已用的百分数> --resets <它的重置时间，本地时间，例如 2026-11-09T11:20> --window 5h；“Current week” 一样，用 --window week。"#
        }
        "ja" => {
            r#"クォータ：このターミナルのセッションには get_usage がありません。AGENTS.md や alm がクォータの報告を求めたら、claude -p /usage を実行し（Claude Code が自分で答え、モデルには聞きません）、その 2 行を報告してください：「Current session」は alm quota --agent claude-code --left <100 から使用済みの割合を引いた数> --resets <そのリセット時刻、ローカル時刻、例 2026-11-09T11:20> --window 5h、「Current week」は同じように --window week で。"#
        }
        _ => {
            r#"Quota: this terminal session has no get_usage. When AGENTS.md or alm asks for your quota, run claude -p /usage (Claude Code answers it itself, without the model) and report its two lines: "Current session" with alm quota --agent claude-code --left <100 minus the percent used> --resets <its reset time, local, e.g. 2026-11-09T11:20> --window 5h, and "Current week" the same way with --window week."#
        }
    };
    format!("{text}\n\n")
}

/// Keep the user's usual quiet-mode instructions, while teaching only monitored Antigravity
/// sessions the exact standalone push form accepted by their temporary allow rule.
fn with_antigravity_push_rule(mode: String, agent: &str, quiet: bool, language: &str) -> String {
    if !quiet || crate::accounts::base_agent(agent) != "antigravity_cli" {
        return mode;
    }
    let instruction = match lang(language) {
        "zh-CN" => {
            "本安静 Antigravity 会话只有单独执行普通推送 `git push origin refs/heads/<branch>`（替换成当前分支名）才会免确认。其他推送写法（裸 `git push`、`git push origin main` 这类简写、强推、删除、镜像或清理远端分支、推标签、和别的命令写在一起）以及 `git reset --hard`、`git clean -f`、`git branch -D`、`rm -rf` 会被直接拒绝，不会弹出确认。被拒后不要换写法、换工具或用脚本绕过：改用允许的写法；确实非做不可时，用 alm wait 搁置并写明要批准的操作。其他操作仍遵守 CLI 的权限确认。"
        }
        "ja" => {
            "この静かな Antigravity セッションで確認なしに実行できる通常の push は、単独の `git push origin refs/heads/<branch>`（現在のブランチ名に置き換える）だけです。ほかの push の書き方（裸の `git push`、`git push origin main` のような省略形、強制 push、削除・mirror・prune、タグの push、他のコマンドとの連結）と `git reset --hard`、`git clean -f`、`git branch -D`、`rm -rf` は確認なしにすぐ拒否されます。拒否されたら、書き方や道具を変えたりスクリプトで回避したりせず、許可された書き方を使ってください。どうしても必要なら、承認が必要な操作を書いて alm wait で保留にしてください。それ以外の操作は CLI の権限確認に従ってください。"
        }
        _ => {
            "In this quiet Antigravity session, only a standalone ordinary push written as `git push origin refs/heads/<branch>` (replace it with the current branch name) is auto-allowed. Other pushes (bare `git push`, short forms such as `git push origin main`, force, delete, mirror or prune, tags, a push joined to other commands) and `git reset --hard`, `git clean -f`, `git branch -D` and `rm -rf` are refused at once, without a prompt. When refused, do not work around it with other wording, tools or scripts: use the allowed form; when you truly need the action, park the task with alm wait, naming the action to approve. Keep every other operation under the CLI's normal permission checks."
        }
    };
    format!("{mode}\n\n{instruction}")
}

/// A task a new session takes over from a session that went lost (A03-husky861).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Takeover {
    /// The former holder's `--as` name.
    pub holder: String,
    /// The working branch its history records, when it does: the new session carries on there.
    pub branch: Option<String>,
}

/// The paragraph that tells a new session it takes a lost task over, on its working branch.
fn takeover_paragraph(t: &Takeover, language: &str) -> String {
    let h = &t.holder;
    match (lang(language), t.branch.as_deref()) {
        ("zh-CN", Some(b)) => format!(
            "这个任务之前的会话（{h}）失联了，没有做完：由你接手。在它原来的工作分支 {b} 上继续：认领时用 --branch {b}，工作树就建在这个分支上（git worktree add <文件夹> {b}，不新建分支，也不从第 3 步说的基础分支切出）；先读交接说明和这个分支上已有的提交。\n\n"
        ),
        ("zh-CN", None) => format!(
            "这个任务之前的会话（{h}）失联了，没有做完：由你接手。它的工作分支没有记录：照交接说明找到它，在它上面继续；找不到时再从基础分支新开一个。\n\n"
        ),
        ("ja", Some(b)) => format!(
            "このタスクを持っていた前のセッション（{h}）は終わる前に連絡が途絶えました：あなたが引き継ぎます。元の作業ブランチ {b} で続けてください：引き受けるときは --branch {b} を付け、ワークツリーはこのブランチで作ります（git worktree add <フォルダー> {b}。新しいブランチは作らず、手順 3 のベースブランチからも作りません）。先に引き継ぎメモとこのブランチにあるコミットを読んでください。\n\n"
        ),
        ("ja", None) => format!(
            "このタスクを持っていた前のセッション（{h}）は終わる前に連絡が途絶えました：あなたが引き継ぎます。作業ブランチは記録されていません：引き継ぎメモから見つけてそこで続け、見つからなければベースブランチから新しく作ってください。\n\n"
        ),
        (_, Some(b)) => format!(
            "The session that held this task before ({h}) was lost before it finished: you take it over. Carry on on its working branch {b}: claim with --branch {b} and make your worktree on that branch (git worktree add <folder> {b}: no new branch, and not from the base branch step 3 names); read the handoff note and the commits already on the branch first.\n\n"
        ),
        (_, None) => format!(
            "The session that held this task before ({h}) was lost before it finished: you take it over. Its working branch is not recorded: find it from the handoff note and carry on there; only when there is none, start a new one from the base branch.\n\n"
        ),
    }
}

/// The request a new session gets for one task. `taskbag` is the taskbag's path as the agent,
/// working in the project folder, passes it to `--taskbag`; `session_id` the ID Allomos gave
/// Claude Code (none for the agents that pick their own).
#[allow(clippy::too_many_arguments)]
pub fn monitor_request(
    id: &str,
    title: &str,
    taskbag: &str,
    language: &str,
    agent: &str,
    session_id: Option<&str>,
    quiet: bool,
    takeover: Option<&Takeover>,
) -> String {
    let tb = format!("--taskbag \"{taskbag}\"");
    let sa = session_agent(agent);
    let usage = format!(
        "{}{}",
        takeover
            .map(|t| takeover_paragraph(t, language))
            .unwrap_or_default(),
        usage_paragraph(agent, language)
    );
    let branch = takeover.and_then(|t| t.branch.as_deref());
    match lang(language) {
        "zh-CN" => {
            let sid = session_id.unwrap_or("<你的会话 ID>");
            let mode = if quiet {
                format!(
                    r#"运行模式：安静模式（开始监测的人选的）。不要停下来等人：常规选择和用户看到的用词、样子都自己决定，做好、合入，并用 alm ai-decided 记下（能截图就附上），批准者在发布前审。只有数据格式、删掉东西、权限和安全、费用、对外影响、改变计划或其他任务的事才是真正的问题：带上你的提案，用 alm wait {id} --on "<问题> 提案：<你的提案>" 搁置，然后结束这个会话；不依赖回答、已做完测完的工作先合入。工作都做完了、只剩人的亲手检查时，用 alm done {id} --without-human-check 收尾（还有问题就用 alm ai-decided 记下，不再搁置）。"#,
                    id = id
                )
            } else {
                r#"运行模式：普通模式（开始监测的人选的）。需要人时照常问。"#.to_string()
            };
            let mode = with_antigravity_push_rule(mode, agent, quiet, language);
            format!(
                r#"请做这个项目任务袋里的任务 {id}（“{title}”）。「换肩」正在自动监测就绪任务，为这一个任务开了这个会话。每条 alm 命令都加上 {tb}。

{mode}

{usage}1. 阅读：alm show {id}（它的说明和最新的交接）。
2. 按 AGENTS.md 的说法认领它，带上这个会话的信息：alm claim {id} --as <你的会话名> --branch {br} --session-agent {sa} --session-id {sid} --session-kind terminal，并把认领分享出去。
3. 在它自己的工作树里做，工作树从认领时列出的基础分支切出。
4. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。
5. 用 alm handoff {id} 写交接，然后结束这个会话。只做这一个任务，不要去取下一个：「换肩」会为下一个任务另开会话。"#,
                id = id,
                mode = mode,
                br = branch.unwrap_or("<你的分支>"),
                sa = sa,
                sid = sid,
                tb = tb,
                title = title,
                usage = usage
            )
        }
        "ja" => {
            let sid = session_id.unwrap_or("<あなたのセッション ID>");
            let mode = if quiet {
                format!(
                    r#"実行モード：静かなモード（監視を始めた人が選びました）。人を待って止まらないでください：いつもの選択も、利用者が目にする言葉や見た目も自分で決め、作ってマージし、alm ai-decided で記録してください（撮れるならスクリーンショット付き）。承認者はリリース前に見直します。本当の質問は、データの形式、何かの削除、権限とセキュリティ、費用、外部への影響、計画や他のタスクの変更だけです：あなたの案を添えて alm wait {id} --on "<質問> 案：<あなたの案>" で保留にし、このセッションを終えてください。答えによらない、作業済みでテスト済みの部分は先にマージします。作業がすべて終わり、人の手による確認だけが残ったら alm done {id} --without-human-check で終えてください（質問が残っていれば保留にせず alm ai-decided で記録します）。"#,
                    id = id
                )
            } else {
                r#"実行モード：通常のモード（監視を始めた人が選びました）。人が必要なときはいつもどおり聞いてください。"#.to_string()
            };
            let mode = with_antigravity_push_rule(mode, agent, quiet, language);
            format!(
                r#"このプロジェクトのタスクバッグにあるタスク {id}（「{title}」）を進めてください。「肩代」が準備完了のタスクを自動で監視していて、このタスクひとつのためにこのセッションを開きました。alm コマンドはすべて {tb} を付けて実行してください。

{mode}

{usage}1. 読む：alm show {id}（説明と最新の引き継ぎ）。
2. AGENTS.md のとおりに、このセッションの情報を付けて引き受け（alm claim {id} --as <あなたのセッション名> --branch {br} --session-agent {sa} --session-id {sid} --session-kind terminal）、その引き受けを共有してください。
3. 専用のワークツリーで作業してください。ワークツリーは、引き受けたときに表示されるベースブランチから作ります。
4. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。
5. alm handoff {id} で引き継ぎを書き、このセッションを終えてください。このタスクだけを進め、次のタスクは取らないでください：次のタスクには「肩代」が新しいセッションを開きます。"#,
                id = id,
                mode = mode,
                br = branch.unwrap_or("<あなたのブランチ>"),
                sa = sa,
                sid = sid,
                tb = tb,
                title = title,
                usage = usage
            )
        }
        _ => {
            let sid = session_id.unwrap_or("<your session's ID>");
            let mode = if quiet {
                format!(
                    r#"Run mode: quiet (chosen by the person who started the monitoring). Never wait for the person: decide routine choices yourself, and the wording and look users see too: build them, merge them and record them with alm ai-decided (with a screenshot when you can take one); the approver reviews them before release. Only data formats, removing things, permissions and security, cost, outside effects and changes to the plan or another task are real questions: park one with your proposal, alm wait {id} --on "<the question> Proposal: <yours>", then end this session; merge first the work that is done and tested and does not depend on the answer. When all the work is done and only a person's hands-on check is left, finish with alm done {id} --without-human-check (record any question left with alm ai-decided instead of parking it)."#,
                    id = id
                )
            } else {
                r#"Run mode: normal (chosen by the person who started the monitoring). Ask the person as usual when you need them."#.to_string()
            };
            let mode = with_antigravity_push_rule(mode, agent, quiet, language);
            format!(
                r#"Work on task {id} ("{title}") in this project's taskbag. Allomos is monitoring ready tasks and opened this session for this one task. Run every alm command with {tb}.

{mode}

{usage}1. Read it: alm show {id} (its description and latest handoff note).
2. Claim it as AGENTS.md says, with this session's details: alm claim {id} --as <your session name> --branch {br} --session-agent {sa} --session-id {sid} --session-kind terminal, and share the claim.
3. Work it in a worktree of its own, cut from the base branch the claim prints.
4. Report progress with alm label; finish with alm done, or stop with alm release --note.
5. Write a handoff with alm handoff {id}, then end this session. Work only this task; do not take another one: Allomos opens a new session for the next."#,
                id = id,
                mode = mode,
                br = branch.unwrap_or("<your branch>"),
                sa = sa,
                sid = sid,
                tb = tb,
                title = title,
                usage = usage
            )
        }
    }
}

/// What the person is told when the run gives a task up after its sessions failed `times` times
/// (A03-husky861).
pub fn given_up(language: &str, id: &str, times: u32) -> String {
    match lang(language) {
        "zh-CN" => format!(
            "{id} 的会话已经 {times} 次没能做下去（失联或没有认领）：这次监测不再为它开会话。请看一下它的交接和会话；要再派发，重新开始监测。"
        ),
        "ja" => format!(
            "{id} のセッションが {times} 回続けられませんでした（連絡が途絶えたか、引き受けませんでした）：今回の監視ではもうセッションを開きません。引き継ぎとセッションを見てください。もう一度任せるには、監視を始め直してください。"
        ),
        _ => format!(
            "{id}'s sessions failed {times} times (lost, or never claimed it): this monitoring run opens no more sessions for it. Look at its handoff and sessions; to give it out again, start the monitoring again."
        ),
    }
}

/// What a done task's tab shows before it closes.
pub fn close_texts(language: &str) -> CloseTexts {
    let (prompt, kept) = match lang(language) {
        "zh-CN" => (
            "「换肩」：任务已完成。这个标签页 30 秒后关闭；按任意键保留。",
            "「换肩」：这个标签页保留。",
        ),
        "ja" => (
            "「肩代」：タスクが完了しました。このタブは 30 秒後に閉じます。残すには何かキーを押してください。",
            "「肩代」：このタブを残します。",
        ),
        _ => (
            "Allomos: the task is done. This tab closes in 30 seconds; press any key to keep it.",
            "Allomos: this tab stays open.",
        ),
    };
    CloseTexts {
        prompt: prompt.to_string(),
        kept: kept.to_string(),
    }
}

/// The system notification's title.
pub fn notify_title(language: &str) -> &'static str {
    match lang(language) {
        "zh-CN" => "「换肩」· 自动监测就绪任务",
        "ja" => "「肩代」· 準備完了のタスクを自動で監視",
        _ => "Allomos · Monitor ready tasks",
    }
}

/// A task's session parked a question for the person.
pub fn notify_waiting(language: &str, id: &str, on: &str) -> String {
    match lang(language) {
        "zh-CN" => format!("{id} 在等你：{on}"),
        "ja" => format!("{id} があなたを待っています：{on}"),
        _ => format!("{id} is waiting for you: {on}"),
    }
}

/// An agent could not be started for a task: it is taken off the run.
/// A quiet Antigravity session shows a confirmation (Task-A03-candle790): the person approves or
/// refuses it in the session's tab; meanwhile the monitor goes on with other tasks.
pub fn waiting_for_approval(language: &str, id: &str) -> String {
    match lang(language) {
        "zh-CN" => format!(
            "{id} 在等你批准一个操作：请到它的终端标签页（标题显示“等待批准”）选择允许或拒绝。监测先接着做别的任务。"
        ),
        "ja" => format!(
            "{id} が操作の承認を待っています：そのターミナルのタブ（タイトルに「承認待ち」）で許可か拒否を選んでください。監視はその間ほかのタスクを進めます。"
        ),
        _ => format!(
            "{id} is waiting for you to approve an action: choose allow or deny in its terminal tab (titled \"waiting for approval\"). The monitor goes on with other tasks meanwhile."
        ),
    }
}

pub fn start_failed(language: &str, agent: &str, id: &str, detail: &str) -> String {
    match lang(language) {
        "zh-CN" => format!("没能为 {id} 启动 {agent}。{detail}"),
        "ja" => format!("{id} のために {agent} を起動できませんでした。{detail}"),
        _ => format!("{agent} could not be started for {id}. {detail}"),
    }
}

/// An agent's quota is used up (A03-otter944): it gets no session until it comes back; `task`
/// is the task it was to work, which is given out again; `resets` when it comes back, as the
/// agent wrote it.
pub fn quota_used_up(
    language: &str,
    agent: &str,
    task: Option<&str>,
    resets: Option<&str>,
) -> String {
    match lang(language) {
        "zh-CN" => format!(
            "{agent} 的额度用完了{}，先不开新会话{}；额度恢复后再派出任务。",
            resets.map(|r| format!("（{r} 恢复）")).unwrap_or_default(),
            task.map(|t| format!("，{t} 仍是就绪")).unwrap_or_default(),
        ),
        "ja" => format!(
            "{agent} のクォータを使い切りました{}。新しいセッションはまだ開きません{}。クォータが戻ったらタスクをまた渡します。",
            resets
                .map(|r| format!("（{r} に回復）"))
                .unwrap_or_default(),
            task.map(|t| format!("（{t} は準備完了のまま）"))
                .unwrap_or_default(),
        ),
        _ => format!(
            "{agent}'s quota is used up{}: no new session for now{}; tasks are given out again once it comes back.",
            resets
                .map(|r| format!(" (back at {r})"))
                .unwrap_or_default(),
            task.map(|t| format!(", and {t} stays ready"))
                .unwrap_or_default(),
        ),
    }
}

/// A session never claimed its task, for no reason the monitor could read (A03-otter944).
pub fn not_started(language: &str, agent: &str, id: &str) -> String {
    match lang(language) {
        "zh-CN" => format!(
            "为 {id} 开的 {agent} 会话 10 分钟都没领取任务，算作没开成；它的标签页还开着，可以看看原因。{id} 下次检查时再派出。"
        ),
        "ja" => format!(
            "{id} のために開いた {agent} のセッションが 10 分たってもタスクを引き受けなかったので、開けなかったものとします。タブは開いたままなので理由を確かめられます。{id} は次の確認でまた渡します。"
        ),
        _ => format!(
            "The {agent} session started for {id} did not claim it in 10 minutes, so it counts as not started; its tab stays open for you to see why. {id} is given out again at the next check."
        ),
    }
}

/// The taskbag cannot be read; the run goes on trying (Task-A03-raven398). `detail` is the
/// failure in English, as the log has it: named only for an ordinary failure (a format problem is
/// said in the person's words, and its English is written for agents).
pub fn unreadable(language: &str, why: crate::unreadable::Why, detail: &str) -> String {
    use crate::unreadable::Why;
    match (lang(language), why) {
        ("zh-CN", Why::Format) => "自动监测读不了任务包：它的格式需要你先处理（多半是任务包比这个「换肩」新），请更新「换肩」。监测会继续运行，任务包能读了就接着做。"
            .to_string(),
        ("zh-CN", Why::Unreadable) => format!(
            "自动监测已有 2 分钟读不了任务包，请检查任务包。监测会继续运行，任务包能读了就接着做。{detail}"
        ),
        ("ja", Why::Format) => "自動監視がタスクバッグを読めません：その形式のため、先に対応が必要です（多くの場合、タスクバッグがこの「肩代」より新しい形式です）。「肩代」を更新してください。監視は動き続け、読めるようになれば続きを進めます。"
            .to_string(),
        ("ja", Why::Unreadable) => format!(
            "自動監視がタスクバッグを 2 分間読めていません。タスクバッグを確認してください。監視は動き続け、読めるようになれば続きを進めます。{detail}"
        ),
        (_, Why::Format) => "Monitoring cannot read the taskbag: its format needs you first (most often, the taskbag is newer than this Allomos). Update Allomos. Monitoring keeps running and carries on once the taskbag can be read."
            .to_string(),
        (_, Why::Unreadable) => format!(
            "Monitoring has not been able to read the taskbag for 2 minutes. Check it. Monitoring keeps running and carries on once the taskbag can be read. {detail}"
        ),
    }
}

/// The app moved a run to its new `alm` after an update (Task-A03-cookie729).
pub fn updated(language: &str, version: &str) -> String {
    match lang(language) {
        "zh-CN" => format!(
            "「换肩」已更新：自动监测换成新版（{version}）继续运行，设置不变；已开的会话照常做完各自的任务。"
        ),
        "ja" => format!(
            "「肩代」を更新しました：自動監視は新しいバージョン（{version}）で、同じ設定のまま続きます。開いているセッションは、それぞれのタスクをいつも通り最後まで進めます。"
        ),
        _ => format!(
            "Allomos was updated: monitoring carries on with the new version ({version}) and the same settings; the open sessions finish their tasks as usual."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreadable_taskbag_says_what_to_do_and_that_monitoring_goes_on() {
        use crate::unreadable::Why;
        assert!(unreadable("zh-CN", Why::Format, "x").contains("请更新「换肩」。监测会继续运行"));
        assert!(unreadable("ja", Why::Format, "x").contains("監視は動き続け"));
        assert!(unreadable("en", Why::Format, "x").contains("Monitoring keeps running"));
        assert!(unreadable("en", Why::Unreadable, "x").contains("2 minutes"));
        assert!(unreadable("zh-CN", Why::Unreadable, "x").ends_with('x'));
        assert!(!unreadable("zh-CN", Why::Format, "x").contains('x'));
    }

    #[test]
    fn a_monitor_moved_to_the_new_version_says_so_in_each_language() {
        assert!(updated("zh-CN", "0.1.64").contains("新版（0.1.64）"));
        assert!(updated("ja", "0.1.64").contains("（0.1.64）"));
        assert!(updated("en", "0.1.64").contains("(0.1.64)"));
    }

    /// The same request the interface wrote (monitorFlow.test.ts), word for word.
    #[test]
    fn the_request_names_the_task_the_taskbag_and_the_session() {
        let en = monitor_request(
            "A01-1",
            "Do it",
            "../tb",
            "en",
            "claude_cli",
            Some("ab-12"),
            true,
            None,
        );
        assert!(en.starts_with("Work on task A01-1 (\"Do it\") in this project's taskbag."));
        assert!(en.contains("Run every alm command with --taskbag \"../tb\"."));
        assert!(
            en.contains("--session-agent claude-code --session-id ab-12 --session-kind terminal")
        );
        assert!(en.contains("Run mode: quiet"));
        assert!(en.contains("Quota: this terminal session has no get_usage."));
        let zh = monitor_request(
            "A01-1",
            "做",
            "../tb",
            "zh-CN",
            "codex_cli",
            None,
            false,
            None,
        );
        assert!(zh.starts_with("请做这个项目任务袋里的任务 A01-1（“做”）。"));
        assert!(zh.contains("--session-agent codex --session-id <你的会话 ID>"));
        assert!(zh.contains("运行模式：普通模式"));
        assert!(!zh.contains("配额"));
        let ja = monitor_request(
            "A01-1",
            "x",
            "tb",
            "ja",
            "antigravity_cli",
            None,
            true,
            None,
        );
        assert!(ja.contains("--session-agent antigravity"));
        assert!(ja.contains("alm wait A01-1 --on"));
        assert!(ja.contains("git push origin refs/heads/<branch>"));
        let ja_normal = monitor_request(
            "A01-1",
            "x",
            "tb",
            "ja",
            "antigravity_cli",
            None,
            false,
            None,
        );
        assert!(!ja_normal.contains("git push origin refs/heads/<branch>"));
        let zh_agy = monitor_request(
            "A01-1",
            "x",
            "tb",
            "zh-CN",
            "antigravity_cli",
            None,
            true,
            None,
        );
        assert!(zh_agy.contains("git push origin refs/heads/<branch>"));
        // Another task language: English.
        assert!(
            monitor_request("A01-1", "x", "tb", "de", "codex_cli", None, true, None)
                .starts_with("Work on task")
        );
        let en_agy = monitor_request(
            "A01-1",
            "x",
            "tb",
            "en",
            "antigravity_cli",
            None,
            true,
            None,
        );
        assert!(en_agy.contains("only a standalone ordinary push"));
    }

    #[test]
    fn a_takeover_request_carries_on_on_the_former_working_branch() {
        let t = Takeover {
            holder: "claude-old".into(),
            branch: Some("claude/old-work".into()),
        };
        for language in ["en", "zh-CN", "ja"] {
            let text = monitor_request(
                "A01-1",
                "x",
                "tb",
                language,
                "claude_cli",
                Some("s"),
                true,
                Some(&t),
            );
            assert!(text.contains("claude-old"), "{language}");
            assert!(text.contains("alm claim A01-1 --as"), "{language}");
            assert!(
                text.contains("--branch claude/old-work --session-agent"),
                "{language}"
            );
            assert!(text.contains("git worktree add"), "{language}");
        }
        let none = Takeover {
            holder: "claude-old".into(),
            branch: None,
        };
        let en = monitor_request(
            "A01-1",
            "x",
            "tb",
            "en",
            "codex_cli",
            None,
            true,
            Some(&none),
        );
        assert!(en.contains("Its working branch is not recorded"));
        assert!(en.contains("--branch <your branch>"));
        let plain = monitor_request("A01-1", "x", "tb", "en", "codex_cli", None, true, None);
        assert!(!plain.contains("take it over"));
    }
}
