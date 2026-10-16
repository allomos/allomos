//! Moving a project that already tracks its tasks another way into Allomos (Task-A05-02).
//!
//! Allomos does not look for or convert anything itself. The app asks one AI agent the person
//! picks to find the project's tasks and move them, with the request built by
//! [`agent_convert_request_in`]. The agent decides the details itself and asks the person exactly
//! once, with `alm confirm` in the app; **Go ahead** covers the whole move, including the commit
//! and the merge into the person's branch (never a push).

use std::path::Path;

use allomos_core::FORMAT;

use crate::versions::MoveIn;
use crate::{section_source, setup_steps_in, shown};

/// Changes when the conversion request's steps change, so a report can say which it followed.
/// 2: the agent finds the tasks itself and asks with `alm confirm` before changing anything.
/// 3: the app has already created the taskbag and recorded the project folder; the ladder is set
/// with `alm labels`, and card links are relative to the recorded project folder (until 9).
/// 4: one question only (owner, 2026-10-10): no questions in the chat, the plan covers removing
/// the old status, committing and merging into the person's branch, and Go ahead does it all.
/// 6: starting again is File → Connect with the Agents… (Task-A05-03).
/// 7: the project's handoff notes move too: into handoff files, `project-notes.md` and the
/// handoff rules (Task-A03-06, owner 2026-10-12).
/// 8: `alm` prints Git commands after each change; during the move they are not run one by one:
/// the agent commits the taskbag, the AGENTS.md change and the report at the end, commit only,
/// never push (decision 0017).
/// 9: each card file moves into the taskbag beside its task (`alm card <id> --from <file>`,
/// `tasks/<id>/card.md`), its description links to it, and the original is removed from the
/// project folder (Task-A00-30, approver 2026-10-16).
pub const CONVERT_REQUEST_VERSION: u32 = 9;

/// Changes when the move-in request's text changes (Task-A01-45): moving the tasks of a taskbag
/// written by an older Allomos into a new one, on top of the conversion request's steps.
/// 2: a task whose old description links to a card file in the project folder gets that card
/// file moved in beside it (Task-A00-30).
pub const MOVE_IN_REQUEST_VERSION: u32 = 2;

/// Changes when the connect request's parts change (Task-A05-03).
/// 2: the setup part asks the person before setting up Git, and commits the taskbag itself (the
/// app no longer runs Git, decision 0017).
pub const CONNECT_REQUEST_VERSION: u32 = 2;

/// The heading of the popup `alm confirm` shows, as the request tells the agent to write it.
const CONFIRM_TITLE: &str = "Move tasks into Allomos";

/// The lines every request that moves tasks in starts with, after its title: where the project
/// and the taskbag are, and how to run `alm`. Agents' shells often forget exported variables
/// between commands, so every command names the taskbag; the request says so once.
pub(crate) fn migration_header_in(language: &str, project: &Path, taskbag: &Path) -> String {
    let (p, t) = (shown(project), shown(taskbag));
    let alm = format!("alm --taskbag \"{t}\"");
    match language {
        "zh-CN" => format!(
            "项目文件夹：{p}\n任务包（Allomos 的任务文件）：{t}\n下面每条 `alm` 命令都请这样运行：`{alm} …`"
        ),
        "ja" => format!(
            "プロジェクトのフォルダー：{p}\nタスクバッグ（Allomos のタスクファイル）：{t}\n以下の `alm` コマンドはすべて `{alm} …` の形で実行してください。"
        ),
        _ => format!(
            "Project folder: {p}\nTaskbag (Allomos's task files): {t}\nRun every `alm` command below as `{alm} …`."
        ),
    }
}

/// The rule the conversion request gives: ask the person exactly once, with the whole plan in the
/// app (`alm confirm`), and decide the details by the request's own defaults.
pub(crate) fn ask_once_in(language: &str) -> String {
    match language {
        "zh-CN" => "只问用户一次：在 Allomos 应用里用 `alm confirm` 确认完整计划。不要在对话中提问；细节按下面的默认做法自己决定，并写进计划，用户不同意可以取消。".to_string(),
        "ja" => "ユーザーへの質問は一度だけです：Allomos アプリで `alm confirm` により計画全体を確認します。チャットでは質問しないでください。細かい点は下の決まりに従って自分で決め、計画に書きます。同意できなければユーザーはキャンセルできます。".to_string(),
        _ => "Ask the person exactly once: the full plan, confirmed in the Allomos app with `alm confirm`. \
Do not ask questions in the chat; decide the details yourself with the defaults below and put \
them in the plan, so the person can cancel if they disagree."
            .to_string(),
    }
}

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
    let (project, taskbag) = (shown(project_path), shown(taskbag_path));
    let version = CONVERT_REQUEST_VERSION;
    let title = CONFIRM_TITLE;
    let header = migration_header_in(language, project_path, taskbag_path);
    let ask = ask_once_in(language);
    match language {
        "zh-CN" => format!(
            "请找出这个项目的任务，并把它们搬进 Allomos（转换请求 v{version}）。

{header}

{ask}

请按顺序做：

1. 先查看，不要改动。找出这个项目是否、以及在哪里记录任务、状态、负责人、标签、领域和下一步（例如任务看板、卡片文件、待办清单），以及 AGENTS.md / CLAUDE.md 中哪些规则描述了它们。也找出交接记录（例如 AGENT_HANDOFF.md、恢复或交接段落、会话笔记）以及何时写交接的规则（例如额度或上下文的百分比）。不要运行项目的脚本或钩子。如果没有找到任务，告诉用户，然后停止。
2. 自己决定，并写进计划：保留项目自己的任务 ID（只去掉 `Task-` 这样的前缀和结尾的描述）；标签阶梯用项目自己的，按顺序；按文件中的用词对应 ready、waiting、done、dropped；还没有卡片的计划工作作为 backlog 任务加入，说明里写“还没有卡片”；任务标题用 `alm task-language` 给出的语言写（原文是别的语言就翻译，卡片链接保留原文）。
3. 如果项目是 Git 仓库：记下当前分支（原分支）。先把前面 Allomos 步骤留下的未提交改动各自单独提交，不带其他文件：任务包（`git -C \"{taskbag}\" add . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`），然后是只含 Allomos 一节改动的 AGENTS.md 和 CLAUDE.md（`git -C \"{project}\" add AGENTS.md CLAUDE.md && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md`）。如果你要改的其他文件（卡片和看板文件，或 AGENTS.md 中其他改动）有未提交的改动，先告诉用户请他们提交或另存，然后停止；不要把它们混进去。
4. 唯一的问题。把完整计划用 Markdown 写进项目之外的文件（例如系统的临时文件夹）：对应表、标签阶梯、你做的决定、AGENTS.md 中要替换的规则、要删除的旧状态（卡片里的状态表、生成的看板段落、会话登记表）、每个卡片文件搬到哪个任务旁边（任务包里的 `tasks/<id>/card.md`，原文件从项目文件夹删除）、要搬的交接记录和它们的去处、工作分支 `allomos/convert`，以及“完成后提交，并合并到 <原分支>；不推送”。然后运行 `alm confirm --title \"{title}\" --plan <文件>`，它会在应用中显示计划并等待回答。退出码 0 表示用户同意整个计划；其他退出码表示取消或没有回答：什么都不要改，然后停止，并告诉用户：没有任何改动；如果改主意了，可以在这里让你再给他看一次计划，或在 Allomos 中选择“文件 → 连接 AI 智能体…”重新开始。
5. 从原分支新建分支 `allomos/convert` 并在上面工作。
6. 只用 `alm` 写任务包，绝不手工编辑它的文件。任务包已经存在（由 Allomos 建立）：不要再建一个，也不要删除它；如果它已经有任务，只添加，不改动那些任务。标签阶梯与任务包的不同时，用 `alm labels \"<阶梯，用逗号分隔>\"` 设置。然后每个领域 `alm add area <id> \"<名称>\"`，每个任务 `alm add task <id> --area <id> --title \"<标题>\"`。然后把每个任务的卡片文件搬到它旁边：`alm card <id> --from <文件>` 把它写成任务包里的 `tasks/<id>/card.md`，并在描述里加上指向它的 `Card:` 链接。传给它的是项目之外的一份副本：去掉旧状态（第 10 步的状态表），并按 `alm guide move-cards` 改好卡片里的相对链接，让它们在 `tasks/<id>/` 里也能打开；卡片是大文件中的一节时，副本只含那一节。每条改动任务包的 `alm` 命令都会打印用来分享这次改动的 Git 命令；搬迁期间不要逐条运行它们，第 11 步一起提交。
7. 照文件所写原样复制状态：`alm --as <负责人> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。只复制写明的内容：不明的状态、日期或负责人保持不明，并写进报告；绝不编造进度、验证或批准。`alm` 还记不下的值（完成日期、依赖、优先级、批准等）写进报告。然后搬交接记录：当前的每段交接（每个会话或卡片一段）原文写进一个交接文件：`alm --as <它的会话> handoff --tasks <id,id> --from <文件>`；全项目都要知道的内容（在运行的服务、陷阱、凭据存放的位置，绝不写凭据本身）写进任务包的 project-notes.md（这个文件可以直接编辑）；项目写明的交接规则用 `alm handoff-rules --quota <n> --context <n> --write-when \"<何时>\"` 设置。旧的交接历史存档留在原处，在 project-notes.md 里链接它。
8. 写一份简短报告 `docs/allomos-conversion.md`：每个卡片文件和它对应的任务 ID，没有遗漏或重复；每个无法复制的值，注明文件和行；运行 `alm list --json`，确认没有 issues。
9. 切换说明：在 AGENTS.md 中，用 Allomos 的一节（`alm brief` 的输出，见最后一行）替换关于状态存放位置、如何认领和释放、会话登记表、重新生成看板、交接写在哪里和何时写的规则；其他规则一字不改。确保 CLAUDE.md 包含导入 AGENTS.md 的一行（半角 at 符号后接 AGENTS.md）。
10. 删除旧状态：卡片文件里的状态表、生成的看板段落和会话登记表，以及内容已经搬走的交接文件。已经搬进任务包的原卡片文件从项目文件夹删除（`git rm`），卡片文件夹空了就删掉；其他文档里指向卡片旧位置的链接改为指向任务。卡片的结果、批准和备注改成普通文字保留下来。只删除计划里写到的东西；有其他用途的脚本、钩子和 CI 不要动。
11. 搬迁完成后提交：任务包、AGENTS.md / CLAUDE.md 的改动、报告，以及你改动或删除的卡片和看板文件，只提交这些（不要提交 .DS_Store 这类无关文件）。用 `git merge --no-ff allomos/convert` 合并到原分支，然后删除 `allomos/convert`。只提交，绝不推送。
12. 告诉用户做了什么、合并提交的编号，以及如何整体撤销：`git revert -m 1 <合并提交>`。"
        ),
        "ja" => format!(
            "このプロジェクトのタスクを見つけて、Allomos に移してください（移行の依頼 v{version}）。

{header}

{ask}

順に進めてください：

1. まず見るだけで、何も変えません。このプロジェクトがタスク、状況、担当者、ラベル、エリア、次の一歩を記録しているか、どこに記録しているか（タスクボード、カードのファイル、TODO リストなど）、AGENTS.md / CLAUDE.md のどのルールがそれを説明しているかを調べます。引き継ぎの記録（AGENT_HANDOFF.md、再開や引き継ぎの節、セッションのメモなど）と、いつ引き継ぎを書くかのルール（使用量やコンテキストの割合など）も探します。プロジェクトのスクリプトやフックは実行しません。タスクが見つからなければ、そう伝えて終わります。
2. 自分で決めて計画に書きます：プロジェクト自身のタスク ID を残す（`Task-` のような接頭辞と末尾の説明だけを外す）、ラベルの段階はプロジェクト自身のものを順に使う、ファイルの言葉を ready、waiting、done、dropped に対応させる、まだカードのない予定の作業は backlog のタスクにして説明に「まだカードなし」と書く、タスクの題名は `alm task-language` の言語で書く（原文が別の言語なら訳し、原文はカードへのリンクで残す）。
3. プロジェクトが Git リポジトリなら、今のブランチ（元のブランチ）を控えます。まず、前の「肩代」の手順が残したコミットされていない変更を、それぞれ単独でコミットします（ほかのファイルは入れません）：タスクバッグ（`git -C \"{taskbag}\" add . && git -C \"{taskbag}\" commit -m \"Add Allomos taskbag\" -- .`）、次に Allomos の節の変更だけを含む AGENTS.md と CLAUDE.md（`git -C \"{project}\" add AGENTS.md CLAUDE.md && git -C \"{project}\" commit -m \"Tell AI agents about Allomos\" -- AGENTS.md CLAUDE.md`）。変えるほかのファイル（カードやボードのファイル、AGENTS.md のほかの変更）にコミットされていない変更があれば、コミットするか退避するようユーザーに伝えて終わります。混ぜないでください。
4. ただ一つの質問。計画全体を Markdown でプロジェクトの外のファイル（たとえばシステムの一時フォルダー）に書きます：対応表、ラベルの段階、自分で決めたこと、AGENTS.md で置き換えるルール、消す古い状況（カードの状況表、生成されるボードの節、セッション一覧）、カードのファイルごとの行き先（タスクバッグの `tasks/<id>/card.md`。元のファイルはプロジェクトのフォルダーから消す）、移す引き継ぎとその行き先、作業ブランチ `allomos/convert`、そして「最後にコミットし、<元のブランチ> にマージする。プッシュはしない」。続いて `alm confirm --title \"{title}\" --plan <ファイル>` を実行すると、アプリに計画が表示され、答えを待ちます。終了コード 0 は計画全体への同意です。それ以外はキャンセルか無回答です：何も変えずに終わり、ユーザーに伝えます：何も変わっていないこと、考えが変わったらここでもう一度計画を見せるよう頼めること、または Allomos の「ファイル → AI エージェントとつなぐ…」からやり直せること。
5. 元のブランチから `allomos/convert` を作り、そこで作業します。
6. タスクバッグは `alm` だけで書き、ファイルを手で編集しません。タスクバッグはすでにあります（Allomos が作りました）：新しく作ったり削除したりしないでください。すでにタスクがあれば追加だけにし、それらは変えません。ラベルの段階がタスクバッグと違えば `alm labels \"<段階、カンマ区切り>\"` で設定します。続いてエリアごとに `alm add area <id> \"<名前>\"`、タスクごとに `alm add task <id> --area <id> --title \"<題名>\"`。続いて、タスクごとのカードのファイルをそのタスクの隣に移します：`alm card <id> --from <ファイル>` がタスクバッグの `tasks/<id>/card.md` として書き、説明にそこへの `Card:` リンクを付けます。渡すのはプロジェクトの外に作った写しです：古い状況（手順 10 の状況表）を除き、カードの中の相対リンクを `alm guide move-cards` のとおり `tasks/<id>/` から開けるように直します。カードが大きなファイルの一節なら、写しはその節だけにします。タスクバッグを変える `alm` コマンドは、その変更を共有する Git コマンドを毎回表示します。移行の間はそれを一つずつ実行せず、手順 11 でまとめてコミットします。
7. 状況はファイルに書かれているとおりに写します：`alm --as <担当者> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。書かれていることだけを写し、わからない状況・日付・担当者はわからないままにしてレポートに書きます。進捗、検証、承認を作り出してはいけません。`alm` がまだ記録できない値（完了日、依存関係、優先度、承認など）はレポートに書きます。続いて引き継ぎを移します：今の引き継ぎ（セッションかカードごとに一つ）を、言葉を変えずに引き継ぎファイルにします：`alm --as <そのセッション> handoff --tasks <id,id> --from <ファイル>`。プロジェクト全体で知っておくこと（動いているサーバー、落とし穴、資格情報の置き場所。値そのものは書かない）はタスクバッグの project-notes.md に書きます（このファイルは直接編集してかまいません）。プロジェクトが定めている引き継ぎのルールは `alm handoff-rules --quota <n> --context <n> --write-when \"<いつ>\"` で設定します。古い引き継ぎの履歴はその場に残し、project-notes.md からリンクします。
8. 短いレポート `docs/allomos-conversion.md` を書きます：カードのファイルごとに対応するタスク ID（抜けも重複もないこと）、写せなかった値とそのファイルと行。`alm list --json` を実行し、issues がないことを確かめます。
9. 説明を切り替えます。AGENTS.md で、状況の置き場所、担当の取り方と手放し方、セッション一覧、ボードの再生成、引き継ぎの置き場所と書くタイミングについてのルールを、Allomos の節（`alm brief` の出力。最後の行を参照）で置き換えます。ほかのルールは一字も変えません。CLAUDE.md に AGENTS.md を読み込む行（半角のアットマークに続けて AGENTS.md）があることを確かめます。
10. 古い状況を消します：カードのファイルの状況表、生成されるボードの節、セッション一覧、そして中身を移し終えた引き継ぎのファイル。タスクバッグに移した元のカードのファイルはプロジェクトのフォルダーから消し（`git rm`）、カードのフォルダーが空になれば消します。ほかの文書にあるカードの古い場所へのリンクは、タスクを指すように直します。カードの結果、承認、メモは普通の文として残します。計画に書いたものだけを消し、ほかの役目もあるスクリプト、フック、CI には触れません。
11. 移行が終わったらコミットします：タスクバッグ、AGENTS.md / CLAUDE.md の変更、レポート、そして変えたり消したりしたカードやボードのファイルだけ（.DS_Store のような関係のないファイルは入れない）。`git merge --no-ff allomos/convert` で元のブランチにマージしてから `allomos/convert` を削除します。コミットだけで、プッシュはしません。
12. 何をしたか、マージコミットの番号、そして全体を元に戻す方法 `git revert -m 1 <マージコミット>` を伝えます。"
        ),
        _ => format!(
            "Please find this project's tasks and move them into Allomos (conversion request v{version}).

{header}

{ask}

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
in card files, generated board sections, session registries), where each card file goes (the \
taskbag's `tasks/<id>/card.md`, its original removed from the project folder), the handoffs you \
will move and where each goes, the working branch \
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
and `alm add task <id> --area <id> --title \"<title>\"` for each task. Then move each task's \
card file in beside it: `alm card <id> --from <file>` writes it as the taskbag's \
`tasks/<id>/card.md` and gives the description a `Card:` link to it. Pass a copy made outside \
the project, without the old status (step 10's status tables) and with the card's relative links \
fixed to work from `tasks/<id>/`, as `alm guide move-cards` says; when a card is a section of a \
bigger file, the copy holds that section only. Every `alm` command that changes the taskbag prints the Git \
commands that share that change; during the move do not run them one by one: step 11 commits \
everything together.
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
registries, and handoff files whose content you moved. Remove the original card files, now in \
the taskbag, from the project folder (`git rm`), and the cards folder once it is empty; change \
links to a card's old place in other documents to point to its task. Keep each card's outcome, approvals and notes as plain text. Remove only what the \
plan named; leave scripts, hooks and CI checks that also do other things alone.
11. When the move is done, commit the taskbag, the AGENTS.md / CLAUDE.md change, the report and \
the card and board files you changed or removed, and nothing else (never unrelated files such as \
.DS_Store). Merge with `git merge --no-ff allomos/convert` into the original branch, then delete \
`allomos/convert`. Commit only; never push.
12. Tell the person what you did, the merge commit, and how to undo the whole move: \
`git revert -m 1 <merge commit>`."
        ),
    }
}

/// The request that asks one AI agent to move the tasks of a taskbag written by an older Allomos
/// into a new taskbag (Task-A01-45, owner 2026-10-15), in an interface language (`en`, `zh-CN`,
/// `ja`; English otherwise): the conversion request's steps (Task-A05-02), told that the tasks to
/// move are the old taskbag's (`move_in.old`), with everything it holds: tasks, history, handoff
/// notes and project notes. The old folder is removed only as part of the plan the person
/// confirms. When the app has not renamed the old folder aside yet (`alm move-in` on the old
/// taskbag itself), the agent does that, and creates the new taskbag, after the Go ahead.
pub fn agent_move_in_request_in(
    language: &str,
    project_path: &Path,
    taskbag_path: &Path,
    move_in: &MoveIn,
) -> String {
    let steps = convert_steps_in(language, project_path, taskbag_path);
    // The conversion request's own title gives way to this one; its header and steps follow.
    let rest = steps
        .split_once("\n\n")
        .map(|(_, rest)| rest)
        .unwrap_or(&steps);
    let version = MOVE_IN_REQUEST_VERSION;
    let (taskbag, old) = (shown(taskbag_path), shown(&move_in.old));
    let found = &move_in.found;
    let current = FORMAT;
    let alm = format!("alm --taskbag \"{taskbag}\"");
    let (title, intro, moves) = match language {
        "zh-CN" => (
            format!("请把旧版 Allomos 写的任务包里的任务搬进一个新的任务包（搬入请求 v{version}）。"),
            if move_in.prepared {
                format!(
                    "Allomos 已把旧任务包（格式 {found}；这个版本的 Allomos 写的是 {current}）改名放到一边：{old}，并在原处建了一个新的空任务包：{taskbag}。`alm` 不读旧格式：请直接读旧文件夹里的文件，绝不改动它们。下面步骤里说的“这个项目的任务”，就是旧文件夹里的任务；不要把别的东西当成任务系统。"
                )
            } else {
                format!(
                    "这个项目的任务包 {taskbag} 是旧版 Allomos 写的（格式 {found}；这个版本的 Allomos 写的是 {current}）。`alm` 不读也不改它：请直接读它的文件。用户同意计划后，作为计划的第一项改动，把它改名放到一边：{old}（例如 `mv \"{taskbag}\" \"{old}\"`），再在原处用 `{alm} init --id <旧 project.json 的 id> --name \"<它的名称>\"` 建新的任务包。在此之前，下面的步骤从 {taskbag} 读旧任务包，之后从 {old} 读。下面步骤里说的“这个项目的任务”，就是旧任务包里的任务；不要把别的东西当成任务系统。"
                )
            },
            format!(
                "用 `alm` 搬走旧任务包里的一切，保留 ID 和顺序（命令见 `alm guide commands`）：
- `project.json`：标签阶梯（`alm labels`；如果最高一级是 \"owner verified\"，改叫 \"human verified\"，这是这个版本的 Allomos 对人工检查的叫法，任务上的也一起改）、任务语言（`alm task-language`）和交接规则（`alm handoff-rules`）。
- `areas/` 和 `iterations/`：用 `alm add area`、`alm add iteration`，ID、名称和日期都相同。
- `tasks/<id>/task.json` 和 `description.md`：每个任务用相同的 ID、标题和领域加入（`alm add task`），描述用 `alm describe <id> --from <旧的 description.md>`，迭代用 `alm plan`；然后是状态、标签、负责人、等待内容、下一个依赖和交接备注（`alm claim`、`label`、`wait`、`needs`、`ready`、`done`、`drop`、`note`）。已完成任务的 `finished_without_owner` 说明改为 `alm done <id> --without-human-check \"<原样的说明>\"`，除非它的标签是阶梯的最高一级（那就是已经有人检查过）。
- `handoffs/`（allomos/2 起）：每份交接原文不改，用 `alm --as <它的作者> handoff --tasks <它的任务> --from <文件>`。
- `project-notes.md`：内容写进新任务包的 `project-notes.md`（这个文件可以直接编辑）。
- `events/`（allomos/3 起）：历史记录。`alm` 只记录它自己做的改动，所以旧事件不复制，它们留在 Git 历史里。在报告和 `project-notes.md` 里写明仍包含旧任务包的最后一次提交（在提交任何东西之前运行 `git -C \"{taskbag}\" log -1 --format=%H -- .`）。
对下面步骤的改动：第 3 步不要单独提交新任务包：在 Git 里它取代旧任务包，所以在第 11 步和搬迁一起提交。第 6 步：保留每个任务自己的描述；旧描述的 `Card:` 链接指向项目文件夹里的卡片文件时，把那个卡片文件也这样搬到任务旁边（`alm card <id> --from <文件>`），旧任务文件夹里的 `card.md` 也一样。第 9 步要替换的是 AGENTS.md / CLAUDE.md 里旧的 Allomos 一节。第 10 步要删除的旧状态就是旧文件夹 {old}，别的都不删：把它写进计划，用户同意后，只有当 Git 历史里有它（以前提交过）时才删除；否则保留它，并在报告里说明。"
            ),
        ),
        "ja" => (
            format!("古いバージョンの Allomos が書いたタスクバッグのタスクを、新しいタスクバッグに移してください（移し入れの依頼 v{version}）。"),
            if move_in.prepared {
                format!(
                    "Allomos は古いタスクバッグ（形式 {found}。このバージョンの Allomos が書くのは {current}）を {old} に名前を変えて脇に置き、元の場所に新しい空のタスクバッグを作りました：{taskbag}。`alm` は古い形式を読みません：古いフォルダーのファイルを直接読み、決して変えないでください。下の手順の「このプロジェクトのタスク」は古いフォルダーのタスクです。ほかのものをタスク管理と見なさないでください。"
                )
            } else {
                format!(
                    "このプロジェクトのタスクバッグ {taskbag} は古いバージョンの Allomos が書いたものです（形式 {found}。このバージョンの Allomos が書くのは {current}）。`alm` はそれを読みも変えもしません：ファイルを直接読んでください。計画への同意を得たら、計画の最初の変更として、名前を {old} に変えて脇に置き（たとえば `mv \"{taskbag}\" \"{old}\"`）、元の場所に `{alm} init --id <古い project.json の id> --name \"<その名前>\"` で新しいタスクバッグを作ります。それまでは下の手順は {taskbag} の古いタスクバッグを読み、そのあとは {old} を読みます。下の手順の「このプロジェクトのタスク」は古いタスクバッグのタスクです。ほかのものをタスク管理と見なさないでください。"
                )
            },
            format!(
                "古いタスクバッグにあるものはすべて、ID と順序を保って `alm` で移します（コマンドは `alm guide commands`）：
- `project.json`：ラベルの段階（`alm labels`。最上位が \"owner verified\" なら、このバージョンの Allomos での人による確認の呼び名 \"human verified\" にし、タスクのラベルも合わせる）、タスクの言語（`alm task-language`）、引き継ぎのルール（`alm handoff-rules`）。
- `areas/` と `iterations/`：`alm add area`、`alm add iteration` で、同じ ID、名前、日付で。
- `tasks/<id>/task.json` と `description.md`：タスクごとに同じ ID、題名、エリアで追加し（`alm add task`）、説明は `alm describe <id> --from <古い description.md>`、イテレーションは `alm plan` で。続いて状態、ラベル、担当者、待っている内容、次の依存、引き継ぎメモ（`alm claim`、`label`、`wait`、`needs`、`ready`、`done`、`drop`、`note`）。終わったタスクの `finished_without_owner` のメモは `alm done <id> --without-human-check \"<同じメモ>\"` にします。ただしラベルが最上位の段なら（人が確認済み）そうしません。
- `handoffs/`（allomos/2 から）：引き継ぎごとに、文を変えずに `alm --as <書いた人> handoff --tasks <そのタスク> --from <ファイル>`。
- `project-notes.md`：中身を新しいタスクバッグの `project-notes.md` に書きます（このファイルは直接編集してかまいません）。
- `events/`（allomos/3 から）：履歴。`alm` は自分の変更しか履歴に残さないので、古いイベントは写しません。Git の履歴に残ります。古いタスクバッグを含む最後のコミットを、レポートと `project-notes.md` に書きます（何かをコミットする前に `git -C \"{taskbag}\" log -1 --format=%H -- .`）。
下の手順の変更点：手順 3 では新しいタスクバッグを単独でコミットしません。Git では古いものを置き換えるので、手順 11 で移行と一緒にコミットします。手順 6 ではタスク自身の説明を残します。古い説明の `Card:` リンクがプロジェクトのフォルダーにあるカードのファイルを指していれば、そのカードのファイルも同じようにタスクの隣に移します（`alm card <id> --from <ファイル>`）。古いタスクのフォルダーにある `card.md` も同じです。手順 9 で置き換えるのは AGENTS.md / CLAUDE.md の古い Allomos の節です。手順 10 で消す古い状況は古いフォルダー {old} だけです：計画に書き、同意のあとで、Git の履歴にある（以前コミットされた）ときだけ消します。そうでなければ残し、レポートにそう書きます。"
            ),
        ),
        _ => (
            format!(
                "Please move the tasks of a taskbag written by an older Allomos into a new taskbag (move-in request v{version})."
            ),
            if move_in.prepared {
                format!(
                    "Allomos renamed the old taskbag (format {found}; this Allomos writes {current}) aside, to {old}, and created a new, empty taskbag in its place, at {taskbag}. `alm` does not read the old format: read the old folder's files directly, and never change them. In the steps below, this project's tasks are the ones in the old folder; take nothing else for a task system."
                )
            } else {
                format!(
                    "This project's taskbag, {taskbag}, was written by an older Allomos (format {found}; this Allomos writes {current}). `alm` does not read or change it: read its files directly. After the person's Go ahead, as the first change of the plan, rename it aside to {old} (for example `mv \"{taskbag}\" \"{old}\"`), then create the new taskbag in its place with `{alm} init --id <the old project.json's id> --name \"<its name>\"`. Until then the steps below read the old taskbag at {taskbag}; afterwards at {old}. In the steps below, this project's tasks are the ones in the old taskbag; take nothing else for a task system."
                )
            },
            format!(
                "Move everything the old taskbag holds through `alm`, keeping IDs and order (commands: `alm guide commands`):
- `project.json`: the label ladder (`alm labels`; if its top step is \"owner verified\", call it \"human verified\", this Allomos's name for the human check, on every task too), the task language (`alm task-language`) and the handoff rules (`alm handoff-rules`).
- `areas/` and `iterations/`: `alm add area` and `alm add iteration`, with the same IDs, names and dates.
- `tasks/<id>/task.json` and `description.md`: each task with the same ID, title and area (`alm add task`), its description with `alm describe <id> --from <the old description.md>`, its iteration with `alm plan`; then its state, label, holder, waiting-on, next dependency and handoff note (`alm claim`, `label`, `wait`, `needs`, `ready`, `done`, `drop`, `note`). A done task's `finished_without_owner` note becomes `alm done <id> --without-human-check \"<the same note>\"`, unless its label is the ladder's top step (then a person has checked it).
- `handoffs/` (allomos/2 and later): each handoff, its text unchanged, with `alm --as <its writer> handoff --tasks <its tasks> --from <file>`.
- `project-notes.md`: its content goes into the new taskbag's `project-notes.md` (you may edit that file directly).
- `events/` (allomos/3): the history. `alm` records history only for its own changes, so the old events are not copied; they stay in Git history. Name, in the report and in `project-notes.md`, the last commit that holds the old taskbag (`git -C \"{taskbag}\" log -1 --format=%H -- .`, before anything is committed).
Changes to the steps below: in step 3, do not commit the new taskbag on its own: in Git it replaces the old one, so it is committed with the move in step 11. In step 6, keep each task's own description; when the old description's `Card:` link points to a card file in the project folder, move that card file in beside the task the same way (`alm card <id> --from <file>`), and so a `card.md` in the old task folder. In step 9, what you replace is Allomos's old section in AGENTS.md / CLAUDE.md. In step 10, the old status to remove is the old folder {old}, and nothing else: name it in the plan, and after Go ahead remove it only when Git history holds it (it was committed before); otherwise keep it and say so in the report."
            ),
        ),
    };
    let source = section_source(language, taskbag_path);
    format!("{title}\n\n{intro}\n\n{moves}\n\n{rest}\n\n{source}\n")
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
            let tb = shown(&taskbag);
            assert!(r.contains(&shown(&project)), "{language}");
            assert!(r.contains(&format!("alm --taskbag \"{tb}\"")));
            assert!(r.contains(&format!("alm confirm --title \"{CONFIRM_TITLE}\" --plan")));
            assert!(r.contains("alm labels"));
            assert!(r.contains("alm task-language"), "{language}");
            assert!(r.contains(&format!("{tb}\" brief`")), "{language}");
            assert!(
                !r.contains("alm init"),
                "the app made the taskbag: {language}"
            );
            // Card files move in beside their tasks (Task-A00-30); no link into the project.
            assert!(!r.contains("](../../../../"), "{language}");
            assert!(r.contains("alm card <id> --from "), "{language}");
            assert!(r.contains("tasks/<id>/card.md"), "{language}");
            assert!(r.contains("git rm"), "{language}");
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
            // Row 13 of decision 0017: commit when the move is done; the report is part of it.
            let end = &r[r.find("11. ").unwrap()..];
            let report = match language {
                "zh-CN" => "报告",
                "ja" => "レポート",
                _ => "the report",
            };
            assert!(end.contains(report), "{language}");
        }
        assert!(en.contains("Do not ask questions in the chat"));
        assert!(agent_convert_request_in("ja", &project, &taskbag).contains("まず見るだけ"));
        assert!(agent_convert_request_in("zh-CN", &project, &taskbag).contains("先查看，不要改动"));
    }

    #[test]
    fn a_taskbag_beside_the_project_is_named_in_the_request_and_the_section() {
        let project = PathBuf::from("/work/app");
        let taskbag = PathBuf::from("/work/tasks-home").join(DEFAULT_DIR);
        let r = agent_convert_request_in("en", &project, &taskbag);
        assert!(
            r.contains(&format!("Project folder: {}", shown(&project))),
            "{r}"
        );
        // `alm brief` (agent_section_for) names the taskbag, relative to the project folder.
        let section = agent_section_for(&project, &taskbag);
        assert!(
            section.contains("--taskbag \"../tasks-home/.allomos/taskbag\""),
            "{section}"
        );
    }

    /// Task-A01-45: an older Allomos's taskbag is moved in like another task system; the request
    /// names the old folder and everything in it, and keeps the conversion request's steps.
    #[test]
    fn move_in_request_names_the_old_folder_and_what_it_holds() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        let old = project.join(".allomos").join("taskbag-allomos-3");
        for prepared in [true, false] {
            let m = MoveIn {
                old: old.clone(),
                found: "allomos/3".into(),
                prepared,
            };
            for language in ["en", "zh-CN", "ja"] {
                let r = agent_move_in_request_in(language, &project, &taskbag, &m);
                assert!(
                    r.contains(&format!("v{MOVE_IN_REQUEST_VERSION}")),
                    "{language}"
                );
                assert!(r.contains(&shown(&old)), "{language}");
                assert!(r.contains("allomos/3") && r.contains(FORMAT), "{language}");
                // Tasks, history, handoff notes and project notes.
                for part in [
                    "tasks/<id>/task.json",
                    "events/",
                    "handoffs/",
                    "project-notes.md",
                    "alm describe <id> --from",
                    "--without-human-check",
                    "\"human verified\"",
                ] {
                    assert!(r.contains(part), "{language}: {part}");
                }
                // The conversion request's steps, with its header, its one question and the
                // section from `alm brief`; not its title.
                let convert = convert_steps_in(language, &project, &taskbag);
                let (title, steps) = convert.split_once("\n\n").unwrap();
                assert!(r.contains(steps), "{language}");
                assert!(!r.contains(title), "{language}");
                assert!(r.contains(&migration_header_in(language, &project, &taskbag)));
                assert!(r.contains(&ask_once_in(language)), "{language}");
                assert!(r.contains(&format!("alm confirm --title \"{CONFIRM_TITLE}\"")));
                assert!(r.trim_end().ends_with(&section_source(language, &taskbag)));
                assert!(!r.contains("--push"), "{language}");
                assert!(!r.contains('@'), "{language}");
                // Not prepared: the agent renames the folder aside and creates the new taskbag,
                // after the Go ahead.
                let init = format!("alm --taskbag \"{}\" init --id", shown(&taskbag));
                assert_eq!(r.contains(&init), !prepared, "{language}");
                assert_eq!(r.contains("mv \""), !prepared, "{language}");
            }
        }
        let m = MoveIn {
            old: old.clone(),
            found: "allomos/1".into(),
            prepared: true,
        };
        let en = agent_move_in_request_in("en", &project, &taskbag, &m);
        assert!(en.starts_with("Please move the tasks of a taskbag written by an older Allomos"));
        assert!(en.contains("Allomos renamed the old taskbag (format allomos/1"));
        assert!(en.contains("remove it only when Git history holds it"));
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
            // The app runs no Git (decision 0017): the agent asks before `git init`, then commits
            // the taskbag (with the README Allomos left) itself, and never pushes it.
            assert!(r.contains("`git init`"), "{language}");
            assert!(r.contains("../../README-allomos.md"), "{language}");
            assert!(!r.contains("--push"), "{language}");
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
