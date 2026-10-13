//! Moving a project that already tracks its tasks another way into Allomos (Task-A05-02).
//!
//! Allomos does not look for or convert anything itself. The app asks one AI agent the person
//! picks to find the project's tasks and move them, with the request built by
//! [`agent_convert_request_in`]. The agent decides the details itself and asks the person exactly
//! once, with `alm confirm` in the app; **Go ahead** covers the whole move, including the commit
//! and the merge into the person's branch (never a push).

use std::path::Path;

use crate::{relative_path, section_source, setup_steps_in};

/// Changes when the conversion request's steps change, so a report can say which it followed.
/// 2: the agent finds the tasks itself and asks with `alm confirm` before changing anything.
/// 3: the app has already created the taskbag and recorded the project folder; the ladder is set
/// with `alm labels`, and card links are relative to the recorded project folder.
/// 4: one question only (owner, 2026-10-10): no questions in the chat, the plan covers removing
/// the old status, committing and merging into the person's branch, and Go ahead does it all.
/// 6: starting again is File → Connect with the Agents… (Task-A05-03).
/// 7: the project's handoff notes move too: into handoff files, `project-notes.md` and the
/// handoff rules (Task-A03-06, owner 2026-10-12).
pub const CONVERT_REQUEST_VERSION: u32 = 7;

/// Changes when the connect request's parts change (Task-A05-03).
pub const CONNECT_REQUEST_VERSION: u32 = 1;

/// The heading of the popup `alm confirm` shows, as the request tells the agent to write it.
const CONFIRM_TITLE: &str = "Move tasks into Allomos";

/// The request that asks one AI agent to move the project's tasks into Allomos, in an
/// interface language (`en`, `zh-CN`, `ja`; English otherwise). `project` is the recorded project
/// folder, `taskbag` the existing taskbag. The steps follow the language; the commands, the paths
/// and Allomos's section stay as they are, so every agent runs the same commands.
pub fn agent_convert_request_in(
    language: &str,
    project_path: &Path,
    taskbag_path: &Path,
) -> String {
    let steps = convert_steps_in(language, project_path, taskbag_path);
    let source = section_source(language, taskbag_path);
    format!("{steps}\n\n{source}\n")
}

/// The conversion request's text without its last line (where Allomos's section comes from).
fn convert_steps_in(language: &str, project_path: &Path, taskbag_path: &Path) -> String {
    // A card's link, from a task's description.md to the project folder.
    let up = relative_path(&taskbag_path.join("tasks").join("ID"), project_path)
        .map(|p| format!("{}/", p.display()))
        .unwrap_or_else(|| format!("{}/", project_path.display()));
    let (project, taskbag) = (project_path.display(), taskbag_path.display());
    let version = CONVERT_REQUEST_VERSION;
    let title = CONFIRM_TITLE;
    // Agents' shells often forget exported variables between commands, so every command names
    // the taskbag; the request says so once instead of repeating the path.
    let with_taskbag = format!("alm --taskbag \"{taskbag}\"");
    match language {
        "zh-CN" => format!(
            "请找出这个项目的任务，并把它们搬进 Allomos（转换请求 v{version}）。

项目文件夹：{project}
任务包（Allomos 的任务文件）：{taskbag}
下面每条 `alm` 命令都请这样运行：`{with_taskbag} …`

只问用户一次：在 Allomos 应用里用 `alm confirm` 确认完整计划。不要在对话中提问；细节按下面的默认做法自己决定，并写进计划，用户不同意可以取消。

请按顺序做：

1. 先查看，不要改动。找出这个项目是否、以及在哪里记录任务、状态、负责人、标签、领域和下一步（例如任务看板、卡片文件、待办清单），以及 AGENTS.md / CLAUDE.md 中哪些规则描述了它们。也找出交接记录（例如 AGENT_HANDOFF.md、恢复或交接段落、会话笔记）以及何时写交接的规则（例如额度或上下文的百分比）。不要运行项目的脚本或钩子。如果没有找到任务，告诉用户，然后停止。
2. 自己决定，并写进计划：保留项目自己的任务 ID（只去掉 `Task-` 这样的前缀和结尾的描述）；标签阶梯用项目自己的，按顺序；按文件中的用词对应 ready、waiting、done、dropped；还没有卡片的计划工作作为 backlog 任务加入，说明里写“还没有卡片”；任务标题用 `alm task-language` 给出的语言写（原文是别的语言就翻译，卡片链接保留原文）。
3. 如果项目是 Git 仓库：记下当前分支（原分支）。先把前面 Allomos 步骤留下的未提交改动各自单独提交，不带其他文件：任务包（`git -C \"{taskbag}\" add . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`），然后是只含 Allomos 一节改动的 AGENTS.md 和 CLAUDE.md（`git -C \"{project}\" add AGENTS.md CLAUDE.md && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md`）。如果你要改的其他文件（卡片和看板文件，或 AGENTS.md 中其他改动）有未提交的改动，先告诉用户请他们提交或另存，然后停止；不要把它们混进去。
4. 唯一的问题。把完整计划用 Markdown 写进项目之外的文件（例如系统的临时文件夹）：对应表、标签阶梯、你做的决定、AGENTS.md 中要替换的规则、要删除的旧状态（卡片里的状态表、生成的看板段落、会话登记表）、要搬的交接记录和它们的去处、工作分支 `allomos/convert`，以及“完成后提交，并合并到 <原分支>；不推送”。然后运行 `alm confirm --title \"{title}\" --plan <文件>`，它会在应用中显示计划并等待回答。退出码 0 表示用户同意整个计划；其他退出码表示取消或没有回答：什么都不要改，然后停止，并告诉用户：没有任何改动；如果改主意了，可以在这里让你再给他看一次计划，或在 Allomos 中选择“文件 → 连接 AI 智能体…”重新开始。
5. 从原分支新建分支 `allomos/convert` 并在上面工作。
6. 只用 `alm` 写任务包，绝不手工编辑它的文件。任务包已经存在（由 Allomos 建立）：不要再建一个，也不要删除它；如果它已经有任务，只添加，不改动那些任务。标签阶梯与任务包的不同时，用 `alm labels \"<阶梯，用逗号分隔>\"` 设置。然后每个领域 `alm add area <id> \"<名称>\"`，每个任务 `alm add task <id> --area <id> --title \"<标题>\" --description \"Card: [<路径>]({up}<从项目文件夹起的路径>)\"`（卡片是大文件中的一节时，链接到 `文件.md#节`）。
7. 照文件所写原样复制状态：`alm --as <负责人> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。只复制写明的内容：不明的状态、日期或负责人保持不明，并写进报告；绝不编造进度、验证或批准。`alm` 还记不下的值（完成日期、依赖、优先级、批准等）写进报告。然后搬交接记录：当前的每段交接（每个会话或卡片一段）原文写进一个交接文件：`alm --as <它的会话> handoff --tasks <id,id> --from <文件>`；全项目都要知道的内容（在运行的服务、陷阱、凭据存放的位置，绝不写凭据本身）写进任务包的 project-notes.md（这个文件可以直接编辑）；项目写明的交接规则用 `alm handoff-rules --quota <n> --context <n> --write-when \"<何时>\"` 设置。旧的交接历史存档留在原处，在 project-notes.md 里链接它。
8. 写一份简短报告 `docs/allomos-conversion.md`：每个卡片文件和它对应的任务 ID，没有遗漏或重复；每个无法复制的值，注明文件和行；运行 `alm list --json`，确认没有 issues。
9. 切换说明：在 AGENTS.md 中，用 Allomos 的一节（`alm brief` 的输出，见最后一行）替换关于状态存放位置、如何认领和释放、会话登记表、重新生成看板、交接写在哪里和何时写的规则；其他规则一字不改。确保 CLAUDE.md 包含导入 AGENTS.md 的一行（半角 at 符号后接 AGENTS.md）。
10. 删除旧状态：卡片文件里的状态表、生成的看板段落和会话登记表，以及内容已经搬走的交接文件。卡片的结果、批准和备注改成普通文字保留下来。只删除计划里写到的东西；有其他用途的脚本、钩子和 CI 不要动。
11. 只提交你改动的文件（不要提交 .DS_Store 这类无关文件），用 `git merge --no-ff allomos/convert` 合并到原分支，然后删除 `allomos/convert`。绝不推送。
12. 告诉用户做了什么、合并提交的编号，以及如何整体撤销：`git revert -m 1 <合并提交>`。"
        ),
        "ja" => format!(
            "このプロジェクトのタスクを見つけて、Allomos に移してください（移行の依頼 v{version}）。

プロジェクトのフォルダー：{project}
タスクバッグ（Allomos のタスクファイル）：{taskbag}
以下の `alm` コマンドはすべて `{with_taskbag} …` の形で実行してください。

ユーザーへの質問は一度だけです：Allomos アプリで `alm confirm` により計画全体を確認します。チャットでは質問しないでください。細かい点は下の決まりに従って自分で決め、計画に書きます。同意できなければユーザーはキャンセルできます。

順に進めてください：

1. まず見るだけで、何も変えません。このプロジェクトがタスク、状況、担当者、ラベル、エリア、次の一歩を記録しているか、どこに記録しているか（タスクボード、カードのファイル、TODO リストなど）、AGENTS.md / CLAUDE.md のどのルールがそれを説明しているかを調べます。引き継ぎの記録（AGENT_HANDOFF.md、再開や引き継ぎの節、セッションのメモなど）と、いつ引き継ぎを書くかのルール（使用量やコンテキストの割合など）も探します。プロジェクトのスクリプトやフックは実行しません。タスクが見つからなければ、そう伝えて終わります。
2. 自分で決めて計画に書きます：プロジェクト自身のタスク ID を残す（`Task-` のような接頭辞と末尾の説明だけを外す）、ラベルの段階はプロジェクト自身のものを順に使う、ファイルの言葉を ready、waiting、done、dropped に対応させる、まだカードのない予定の作業は backlog のタスクにして説明に「まだカードなし」と書く、タスクの題名は `alm task-language` の言語で書く（原文が別の言語なら訳し、原文はカードへのリンクで残す）。
3. プロジェクトが Git リポジトリなら、今のブランチ（元のブランチ）を控えます。まず、前の「肩代」の手順が残したコミットされていない変更を、それぞれ単独でコミットします（ほかのファイルは入れません）：タスクバッグ（`git -C \"{taskbag}\" add . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`）、次に Allomos の節の変更だけを含む AGENTS.md と CLAUDE.md（`git -C \"{project}\" add AGENTS.md CLAUDE.md && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md`）。変えるほかのファイル（カードやボードのファイル、AGENTS.md のほかの変更）にコミットされていない変更があれば、コミットするか退避するようユーザーに伝えて終わります。混ぜないでください。
4. ただ一つの質問。計画全体を Markdown でプロジェクトの外のファイル（たとえばシステムの一時フォルダー）に書きます：対応表、ラベルの段階、自分で決めたこと、AGENTS.md で置き換えるルール、消す古い状況（カードの状況表、生成されるボードの節、セッション一覧）、移す引き継ぎとその行き先、作業ブランチ `allomos/convert`、そして「最後にコミットし、<元のブランチ> にマージする。プッシュはしない」。続いて `alm confirm --title \"{title}\" --plan <ファイル>` を実行すると、アプリに計画が表示され、答えを待ちます。終了コード 0 は計画全体への同意です。それ以外はキャンセルか無回答です：何も変えずに終わり、ユーザーに伝えます：何も変わっていないこと、考えが変わったらここでもう一度計画を見せるよう頼めること、または Allomos の「ファイル → AI エージェントとつなぐ…」からやり直せること。
5. 元のブランチから `allomos/convert` を作り、そこで作業します。
6. タスクバッグは `alm` だけで書き、ファイルを手で編集しません。タスクバッグはすでにあります（Allomos が作りました）：新しく作ったり削除したりしないでください。すでにタスクがあれば追加だけにし、それらは変えません。ラベルの段階がタスクバッグと違えば `alm labels \"<段階、カンマ区切り>\"` で設定します。続いてエリアごとに `alm add area <id> \"<名前>\"`、タスクごとに `alm add task <id> --area <id> --title \"<題名>\" --description \"Card: [<パス>]({up}<プロジェクトのフォルダーからのパス>)\"`（カードが大きなファイルの一節なら `ファイル.md#節` へリンク）。
7. 状況はファイルに書かれているとおりに写します：`alm --as <担当者> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。書かれていることだけを写し、わからない状況・日付・担当者はわからないままにしてレポートに書きます。進捗、検証、承認を作り出してはいけません。`alm` がまだ記録できない値（完了日、依存関係、優先度、承認など）はレポートに書きます。続いて引き継ぎを移します：今の引き継ぎ（セッションかカードごとに一つ）を、言葉を変えずに引き継ぎファイルにします：`alm --as <そのセッション> handoff --tasks <id,id> --from <ファイル>`。プロジェクト全体で知っておくこと（動いているサーバー、落とし穴、資格情報の置き場所。値そのものは書かない）はタスクバッグの project-notes.md に書きます（このファイルは直接編集してかまいません）。プロジェクトが定めている引き継ぎのルールは `alm handoff-rules --quota <n> --context <n> --write-when \"<いつ>\"` で設定します。古い引き継ぎの履歴はその場に残し、project-notes.md からリンクします。
8. 短いレポート `docs/allomos-conversion.md` を書きます：カードのファイルごとに対応するタスク ID（抜けも重複もないこと）、写せなかった値とそのファイルと行。`alm list --json` を実行し、issues がないことを確かめます。
9. 説明を切り替えます。AGENTS.md で、状況の置き場所、担当の取り方と手放し方、セッション一覧、ボードの再生成、引き継ぎの置き場所と書くタイミングについてのルールを、Allomos の節（`alm brief` の出力。最後の行を参照）で置き換えます。ほかのルールは一字も変えません。CLAUDE.md に AGENTS.md を読み込む行（半角のアットマークに続けて AGENTS.md）があることを確かめます。
10. 古い状況を消します：カードのファイルの状況表、生成されるボードの節、セッション一覧、そして中身を移し終えた引き継ぎのファイル。カードの結果、承認、メモは普通の文として残します。計画に書いたものだけを消し、ほかの役目もあるスクリプト、フック、CI には触れません。
11. 変えたファイルだけをコミットし（.DS_Store のような関係のないファイルは入れない）、`git merge --no-ff allomos/convert` で元のブランチにマージしてから `allomos/convert` を削除します。プッシュはしません。
12. 何をしたか、マージコミットの番号、そして全体を元に戻す方法 `git revert -m 1 <マージコミット>` を伝えます。"
        ),
        _ => format!(
            "Please find this project's tasks and move them into Allomos (conversion request v{version}).

Project folder: {project}
Taskbag (Allomos's task files): {taskbag}
Run every `alm` command below as `{with_taskbag} …`.

Ask the person exactly once: the full plan, confirmed in the Allomos app with `alm confirm`. \
Do not ask questions in the chat; decide the details yourself with the defaults below and put \
them in the plan, so the person can cancel if they disagree.

Work in this order:

1. Look first, change nothing. Find whether and where this project keeps its tasks, their \
status, who holds each one, their labels, areas and next steps (for example a task board, card \
files or a to-do list), and which rules in AGENTS.md / CLAUDE.md describe them. Also find its \
handoff notes (for example AGENT_HANDOFF.md, resume or handoff sections, session notes) and the \
rules for when to write them (for example quota or context percentages). Do not run the \
project's scripts or hooks. If you find no tasks, tell the person and stop.
2. Decide, and write it in the plan: keep the project's own task IDs (drop only prefixes like \
`Task-` and the trailing description); keep the project's own label ladder, in order; map the \
files' words to ready, waiting, done and dropped; add planned work that has no card yet as a \
backlog task saying \"no card yet\"; write task titles in the language `alm task-language` gives \
(translate a card's title when it is in another language; the card link keeps the original).
3. If the project is a Git repository, note the current branch (the original branch). First \
commit what the earlier Allomos steps left uncommitted, each as its own commit and nothing else: \
the taskbag (`git -C \"{taskbag}\" add . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`), \
then AGENTS.md and CLAUDE.md when their only changes are Allomos's section (`git -C \"{project}\" add \
AGENTS.md CLAUDE.md && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md \
CLAUDE.md`). If other files you will change (card and board files, or other edits in AGENTS.md) \
have uncommitted changes, tell the person to commit or set them aside first, and stop; never mix \
them in.
4. The one question. Write the whole plan in Markdown to a file outside the project (for \
example in the system's temporary folder): the mapping table, the label ladder, the decisions \
you made, the rules you will replace in AGENTS.md, the old status you will remove (status tables \
in card files, generated board sections, session registries), the handoffs you will move and \
where each goes, the working branch \
`allomos/convert`, and \"then commit and merge into <original branch>; nothing is pushed\". \
Then run `alm confirm --title \"{title}\" --plan <file>`. It shows the plan in the app and waits \
for the person's answer. Exit code 0 means go ahead with the whole plan; any other exit code \
means the person cancelled or did not answer: change nothing and stop, and tell the person that \
nothing changed and that, if they change their mind, they can ask you here to show the plan \
again, or choose File → Connect with the Agents… in Allomos to start again.
5. Create the branch `allomos/convert` from the original branch and work on it.
6. Write the taskbag only with `alm`; never edit its files by hand. The taskbag already exists \
(Allomos created it): never create another or delete it; if it already holds tasks, only add \
to them and never change those. If the project's label ladder differs from the taskbag's, set it \
with `alm labels \"<ladder, comma-separated>\"`. Then `alm add area <id> \"<name>\"` for each area \
and `alm add task <id> --area <id> --title \"<title>\" --description \"Card: \
[<path>]({up}<path from the project folder>)\"` for each task (link to `file.md#section` when a \
card is a section of a bigger file).
7. Copy status exactly as the files say it: `alm --as <holder> claim <id>`, `label`, `wait --on`, \
`needs`, `ready`, `done`, `drop`. Copy only what is written: an unknown state, date or holder \
stays unknown and goes in the report; never invent progress, verification or approvals. Values \
`alm` cannot record yet (done dates, dependencies, priority, approvals and the like) go in the \
report. Then move the handoffs: write each current handoff (one per session or card) into a \
handoff file, in its own words: `alm --as <its session> handoff --tasks <id,id> --from <file>`. \
What the whole project must know (running servers, pitfalls, where credentials live, never their \
values) goes into the taskbag's project-notes.md (you may edit that file directly). Handoff \
rules the project states are set with `alm handoff-rules --quota <n> --context <n> --write-when \
\"<when>\"`. An old handoff history stays where it is; link it from project-notes.md.
8. Write a short report, `docs/allomos-conversion.md`: every card file and the task ID it \
became, nothing dropped or duplicated; every value that could not be copied, with file and line. \
Run `alm list --json` and confirm it reports no issues.
9. Switch the instructions over. In AGENTS.md, replace the rules about where status lives, how \
to claim and release, session registries, regenerating the board, and where and when to write \
handoffs with Allomos's section \
(the output of `alm brief`, see the last line); keep every other rule word for word. Make sure \
CLAUDE.md has the line that imports AGENTS.md (a plain ASCII at sign followed by AGENTS.md).
10. Remove the old status: status tables in card files, generated board sections, session \
registries, and handoff files whose content you moved. Keep each card's outcome, approvals and notes as plain text. Remove only what the \
plan named; leave scripts, hooks and CI checks that also do other things alone.
11. Commit only the files you changed (never unrelated files such as .DS_Store), merge with \
`git merge --no-ff allomos/convert` into the original branch, then delete \
`allomos/convert`. Never push.
12. Tell the person what you did, the merge commit, and how to undo the whole move: \
`git revert -m 1 <merge commit>`."
        ),
    }
}

/// What **Connect with the agents** asks the first chosen agent (Task-A05-03): first the setup
/// request (add Allomos's section to `AGENTS.md` and `CLAUDE.md`), then the conversion request
/// (find the project's tasks; if there are some, offer to move them with `alm confirm`, so the
/// person still decides). Other agents chosen at the same time get only the setup request: two
/// agents moving tasks would add every task twice.
pub fn agent_connect_request_in(
    language: &str,
    project_path: &Path,
    taskbag_path: &Path,
) -> String {
    let version = CONNECT_REQUEST_VERSION;
    let setup = setup_steps_in(language, project_path, taskbag_path);
    let convert = convert_steps_in(language, project_path, taskbag_path);
    let source = section_source(language, taskbag_path);
    let (intro, one, two) = match language {
        "zh-CN" => (
            format!(
                "请按顺序完成两部分（连接请求 v{version}）：第一部分把 Allomos 告诉以后的 AI 智能体；第二部分找出这个项目已有的任务，并询问用户是否把它们搬进 Allomos。"
            ),
            "第一部分：告诉 AI 智能体",
            "第二部分：已有的任务（第一部分完成后再做）",
        ),
        "ja" => (
            format!(
                "二つの部分を順に進めてください（接続の依頼 v{version}）。第一部では、これからの AI エージェントに Allomos を伝えます。第二部では、このプロジェクトにすでにあるタスクを探し、Allomos に移すかどうかをユーザーに尋ねます。"
            ),
            "第一部：AI エージェントに伝える",
            "第二部：すでにあるタスク（第一部が終わってから）",
        ),
        _ => (
            format!(
                "Please do two parts, in order (connect request v{version}). Part 1 tells future AI \
agents about Allomos. Part 2 finds the tasks this project already keeps and asks the person \
whether to move them into Allomos."
            ),
            "Part 1: tell AI agents",
            "Part 2: existing tasks (after part 1 is done)",
        ),
    };
    format!("{intro}\n\n## {one}\n\n{setup}\n\n## {two}\n\n{convert}\n\n{source}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, agent_section_for};
    use std::path::PathBuf;

    #[test]
    fn convert_request_carries_paths_confirm_and_the_section() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_convert_request_in(language, &project, &taskbag);
            assert!(r.contains("/work/bakery"), "{language}");
            assert!(r.contains("alm --taskbag \"/work/bakery/.allomos/taskbag\""));
            assert!(r.contains(&format!("alm confirm --title \"{CONFIRM_TITLE}\" --plan")));
            assert!(r.contains("alm labels"));
            assert!(r.contains("alm task-language"), "{language}");
            assert!(
                r.contains("/work/bakery/.allomos/taskbag\" brief`"),
                "{language}"
            );
            assert!(
                !r.contains("alm init"),
                "the app made the taskbag: {language}"
            );
            assert!(r.contains("](../../../../"));
            assert!(r.contains("docs/allomos-conversion.md"));
            // Handoff notes move too (Task-A03-06).
            assert!(r.contains("AGENT_HANDOFF.md"), "{language}");
            assert!(r.contains(" handoff --tasks <id,id> --from "), "{language}");
            assert!(r.contains("project-notes.md"), "{language}");
            assert!(
                r.contains("alm handoff-rules --quota <n> --context <n>"),
                "{language}"
            );
            // Leftovers of the earlier steps are committed first, each on its own (Task-A03-08).
            assert!(
                r.contains("commit -m \"Add Allomos taskbag\" -- ."),
                "{language}"
            );
            assert!(
                r.contains("commit -m \"Tell AI agents about Allomos\""),
                "{language}"
            );
            assert!(r.contains(&format!("v{CONVERT_REQUEST_VERSION}")));
            // The section comes from `alm brief`; the request carries no copy of it.
            assert!(!r.contains("alm claim <id>"), "{language}");
            assert!(
                !r.contains('@'),
                "the Claude app shows a typed-in @ as ＠: {language}"
            );
        }
        let en = agent_convert_request_in("en", &project, &taskbag);
        assert!(en.contains("Look first, change nothing."));
        assert!(en.contains("If you find no tasks, tell the person and stop."));
        assert!(en.contains("Exit code 0 means go ahead with the whole plan"));
        // One question only, and Go ahead covers committing and merging, never pushing.
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_convert_request_in(language, &project, &taskbag);
            assert!(
                r.contains("git merge --no-ff allomos/convert"),
                "{language}"
            );
            assert!(r.contains("git revert -m 1"), "{language}");
            assert!(!r.contains("--push"), "{language}");
        }
        assert!(en.contains("Do not ask questions in the chat"));
        assert!(agent_convert_request_in("ja", &project, &taskbag).contains("まず見るだけ"));
        assert!(agent_convert_request_in("zh-CN", &project, &taskbag).contains("先查看，不要改动"));
    }

    #[test]
    fn a_taskbag_beside_the_project_is_named_in_links_and_the_section() {
        let project = PathBuf::from("/work/app");
        let taskbag = PathBuf::from("/work/tasks-home").join(DEFAULT_DIR);
        let r = agent_convert_request_in("en", &project, &taskbag);
        assert!(r.contains("](../../../../../app/"), "{r}");
        // `alm brief` (agent_section_for) names the taskbag, relative to the project folder.
        let section = agent_section_for(&project, &taskbag);
        assert!(
            section.contains("--taskbag \"../tasks-home/.allomos/taskbag\""),
            "{section}"
        );
    }

    #[test]
    fn connect_request_tells_agents_then_offers_to_move_tasks() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_connect_request_in(language, &project, &taskbag);
            let setup = crate::setup_steps_in(language, &project, &taskbag);
            let convert = convert_steps_in(language, &project, &taskbag);
            // Both requests, in order: instructions first, then the tasks.
            let (a, b) = (r.find(&setup).unwrap(), r.find(&convert).unwrap());
            assert!(a < b, "{language}");
            assert!(
                r.contains(&format!("v{CONNECT_REQUEST_VERSION}")),
                "{language}"
            );
            // The person still decides whether tasks move: the plan goes through `alm confirm`.
            assert!(r.contains("alm confirm --title"), "{language}");
            // Where the section comes from is said once, at the end.
            let source = section_source(language, &taskbag);
            assert_eq!(r.matches(&source).count(), 1, "{language}");
            assert!(r.trim_end().ends_with(&source), "{language}");
            assert!(!r.contains('@'), "{language}");
            // Starting again names the one menu item.
            assert!(!r.contains("Move Tasks Into"), "{language}");
        }
        assert!(
            agent_connect_request_in("en", &project, &taskbag)
                .contains("File → Connect with the Agents…")
        );
    }
}
