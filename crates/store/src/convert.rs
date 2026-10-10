//! Moving a project that already tracks its tasks another way into FlipHopper (Task-A05-02).
//!
//! FlipHopper does not look for or convert anything itself. The app asks one AI agent the person
//! picks to find the project's tasks and move them, with the request built by
//! [`agent_convert_request_in`]. The agent decides the details itself and asks the person exactly
//! once, with `fh confirm` in the app; **Go ahead** covers the whole move, including the commit
//! and the merge into the person's branch (never a push).

use std::path::Path;

use crate::{relative_path, section_source};

/// Changes when the conversion request's steps change, so a report can say which it followed.
/// 2: the agent finds the tasks itself and asks with `fh confirm` before changing anything.
/// 3: the app has already created the seabag and recorded the project folder; the ladder is set
/// with `fh labels`, and card links are relative to the recorded project folder.
/// 4: one question only (owner, 2026-10-10): no questions in the chat, the plan covers removing
/// the old status, committing and merging into the person's branch, and Go ahead does it all.
pub const CONVERT_REQUEST_VERSION: u32 = 4;

/// The heading of the popup `fh confirm` shows, as the request tells the agent to write it.
const CONFIRM_TITLE: &str = "Move tasks into FlipHopper";

/// The request that asks one AI agent to move the project's tasks into FlipHopper, in an
/// interface language (`en`, `zh-CN`, `ja`; English otherwise). `project` is the recorded project
/// folder, `seabag` the existing seabag. The steps follow the language; the commands, the paths
/// and FlipHopper's section stay as they are, so every agent runs the same commands.
pub fn agent_convert_request_in(language: &str, project_path: &Path, seabag_path: &Path) -> String {
    // A card's link, from a task's description.md to the project folder.
    let up = relative_path(&seabag_path.join("tasks").join("ID"), project_path)
        .map(|p| format!("{}/", p.display()))
        .unwrap_or_else(|| format!("{}/", project_path.display()));
    let (project, seabag) = (project_path.display(), seabag_path.display());
    let version = CONVERT_REQUEST_VERSION;
    let title = CONFIRM_TITLE;
    // Agents' shells often forget exported variables between commands, so every command names
    // the seabag; the request says so once instead of repeating the path.
    let with_seabag = format!("fh --seabag \"{seabag}\"");
    let source = section_source(language, seabag_path);
    let steps = match language {
        "zh-CN" => format!(
            "请找出这个项目的任务，并把它们搬进 FlipHopper（转换请求 v{version}）。

项目文件夹：{project}
水手袋（FlipHopper 的任务文件）：{seabag}
下面每条 `fh` 命令都请这样运行：`{with_seabag} …`

只问用户一次：在 FlipHopper 应用里用 `fh confirm` 确认完整计划。不要在对话中提问；细节按下面的默认做法自己决定，并写进计划，用户不同意可以取消。

请按顺序做：

1. 先查看，不要改动。找出这个项目是否、以及在哪里记录任务、状态、负责人、标签、领域和下一步（例如任务看板、卡片文件、待办清单），以及 AGENTS.md / CLAUDE.md 中哪些规则描述了它们。不要运行项目的脚本或钩子。如果没有找到任务，告诉用户，然后停止。
2. 自己决定，并写进计划：保留项目自己的任务 ID（只去掉 `Task-` 这样的前缀和结尾的描述）；标签阶梯用项目自己的，按顺序；按文件中的用词对应 ready、waiting、done、dropped；还没有卡片的计划工作作为 backlog 任务加入，说明里写“还没有卡片”；任务标题用 `fh task-language` 给出的语言写（原文是别的语言就翻译，卡片链接保留原文）。
3. 如果项目是 Git 仓库：记下当前分支（原分支）。如果你要改的文件（AGENTS.md、CLAUDE.md、卡片和看板文件）有未提交的改动，先告诉用户请他们提交或另存，然后停止；不要把它们混进去。
4. 唯一的问题。把完整计划用 Markdown 写进项目之外的文件（例如系统的临时文件夹）：对应表、标签阶梯、你做的决定、AGENTS.md 中要替换的规则、要删除的旧状态（卡片里的状态表、生成的看板段落、会话登记表）、工作分支 `fliphopper/convert`，以及“完成后提交，并合并到 <原分支>；不推送”。然后运行 `fh confirm --title \"{title}\" --plan <文件>`，它会在应用中显示计划并等待回答。退出码 0 表示用户同意整个计划；其他退出码表示取消或没有回答：什么都不要改，然后停止，并告诉用户：没有任何改动；如果改主意了，可以在这里让你再给他看一次计划，或在 FlipHopper 中选择“文件 → 把任务搬进 FlipHopper…”重新开始。
5. 从原分支新建分支 `fliphopper/convert` 并在上面工作。
6. 只用 `fh` 写水手袋，绝不手工编辑它的文件。水手袋已经存在（由 FlipHopper 建立）：不要再建一个，也不要删除它；如果它已经有任务，只添加，不改动那些任务。标签阶梯与水手袋的不同时，用 `fh labels \"<阶梯，用逗号分隔>\"` 设置。然后每个领域 `fh add area <id> \"<名称>\"`，每个任务 `fh add task <id> --area <id> --title \"<标题>\" --description \"Card: [<路径>]({up}<从项目文件夹起的路径>)\"`（卡片是大文件中的一节时，链接到 `文件.md#节`）。
7. 照文件所写原样复制状态：`fh --as <负责人> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。只复制写明的内容：不明的状态、日期或负责人保持不明，并写进报告；绝不编造进度、验证或批准。`fh` 还记不下的值（完成日期、依赖、优先级、批准等）写进报告。
8. 写一份简短报告 `docs/fliphopper-conversion.md`：每个卡片文件和它对应的任务 ID，没有遗漏或重复；每个无法复制的值，注明文件和行；运行 `fh list --json`，确认没有 issues。
9. 切换说明：在 AGENTS.md 中，用 FlipHopper 的一节（`fh brief` 的输出，见最后一行）替换关于状态存放位置、如何认领和释放、会话登记表、重新生成看板的规则；其他规则一字不改。确保 CLAUDE.md 包含导入 AGENTS.md 的一行（半角 at 符号后接 AGENTS.md）。
10. 删除旧状态：卡片文件里的状态表、生成的看板段落和会话登记表。卡片的结果、批准和备注改成普通文字保留下来。只删除计划里写到的东西；有其他用途的脚本、钩子和 CI 不要动。
11. 只提交你改动的文件（不要提交 .DS_Store 这类无关文件），用 `git merge --no-ff fliphopper/convert` 合并到原分支，然后删除 `fliphopper/convert`。绝不推送。
12. 告诉用户做了什么、合并提交的编号，以及如何整体撤销：`git revert -m 1 <合并提交>`。

{source}"
        ),
        "ja" => format!(
            "このプロジェクトのタスクを見つけて、FlipHopper に移してください（移行の依頼 v{version}）。

プロジェクトのフォルダー：{project}
シーバッグ（FlipHopper のタスクファイル）：{seabag}
以下の `fh` コマンドはすべて `{with_seabag} …` の形で実行してください。

ユーザーへの質問は一度だけです：FlipHopper アプリで `fh confirm` により計画全体を確認します。チャットでは質問しないでください。細かい点は下の決まりに従って自分で決め、計画に書きます。同意できなければユーザーはキャンセルできます。

順に進めてください：

1. まず見るだけで、何も変えません。このプロジェクトがタスク、状況、担当者、ラベル、エリア、次の一歩を記録しているか、どこに記録しているか（タスクボード、カードのファイル、TODO リストなど）、AGENTS.md / CLAUDE.md のどのルールがそれを説明しているかを調べます。プロジェクトのスクリプトやフックは実行しません。タスクが見つからなければ、そう伝えて終わります。
2. 自分で決めて計画に書きます：プロジェクト自身のタスク ID を残す（`Task-` のような接頭辞と末尾の説明だけを外す）、ラベルの段階はプロジェクト自身のものを順に使う、ファイルの言葉を ready、waiting、done、dropped に対応させる、まだカードのない予定の作業は backlog のタスクにして説明に「まだカードなし」と書く、タスクの題名は `fh task-language` の言語で書く（原文が別の言語なら訳し、原文はカードへのリンクで残す）。
3. プロジェクトが Git リポジトリなら、今のブランチ（元のブランチ）を控えます。変えるファイル（AGENTS.md、CLAUDE.md、カードやボードのファイル）にコミットされていない変更があれば、コミットするか退避するようユーザーに伝えて終わります。混ぜないでください。
4. ただ一つの質問。計画全体を Markdown でプロジェクトの外のファイル（たとえばシステムの一時フォルダー）に書きます：対応表、ラベルの段階、自分で決めたこと、AGENTS.md で置き換えるルール、消す古い状況（カードの状況表、生成されるボードの節、セッション一覧）、作業ブランチ `fliphopper/convert`、そして「最後にコミットし、<元のブランチ> にマージする。プッシュはしない」。続いて `fh confirm --title \"{title}\" --plan <ファイル>` を実行すると、アプリに計画が表示され、答えを待ちます。終了コード 0 は計画全体への同意です。それ以外はキャンセルか無回答です：何も変えずに終わり、ユーザーに伝えます：何も変わっていないこと、考えが変わったらここでもう一度計画を見せるよう頼めること、または FlipHopper の「ファイル → タスクを FlipHopper に移す…」からやり直せること。
5. 元のブランチから `fliphopper/convert` を作り、そこで作業します。
6. シーバッグは `fh` だけで書き、ファイルを手で編集しません。シーバッグはすでにあります（FlipHopper が作りました）：新しく作ったり削除したりしないでください。すでにタスクがあれば追加だけにし、それらは変えません。ラベルの段階がシーバッグと違えば `fh labels \"<段階、カンマ区切り>\"` で設定します。続いてエリアごとに `fh add area <id> \"<名前>\"`、タスクごとに `fh add task <id> --area <id> --title \"<題名>\" --description \"Card: [<パス>]({up}<プロジェクトのフォルダーからのパス>)\"`（カードが大きなファイルの一節なら `ファイル.md#節` へリンク）。
7. 状況はファイルに書かれているとおりに写します：`fh --as <担当者> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。書かれていることだけを写し、わからない状況・日付・担当者はわからないままにしてレポートに書きます。進捗、検証、承認を作り出してはいけません。`fh` がまだ記録できない値（完了日、依存関係、優先度、承認など）はレポートに書きます。
8. 短いレポート `docs/fliphopper-conversion.md` を書きます：カードのファイルごとに対応するタスク ID（抜けも重複もないこと）、写せなかった値とそのファイルと行。`fh list --json` を実行し、issues がないことを確かめます。
9. 説明を切り替えます。AGENTS.md で、状況の置き場所、担当の取り方と手放し方、セッション一覧、ボードの再生成についてのルールを、FlipHopper の節（`fh brief` の出力。最後の行を参照）で置き換えます。ほかのルールは一字も変えません。CLAUDE.md に AGENTS.md を読み込む行（半角のアットマークに続けて AGENTS.md）があることを確かめます。
10. 古い状況を消します：カードのファイルの状況表、生成されるボードの節、セッション一覧。カードの結果、承認、メモは普通の文として残します。計画に書いたものだけを消し、ほかの役目もあるスクリプト、フック、CI には触れません。
11. 変えたファイルだけをコミットし（.DS_Store のような関係のないファイルは入れない）、`git merge --no-ff fliphopper/convert` で元のブランチにマージしてから `fliphopper/convert` を削除します。プッシュはしません。
12. 何をしたか、マージコミットの番号、そして全体を元に戻す方法 `git revert -m 1 <マージコミット>` を伝えます。

{source}"
        ),
        _ => format!(
            "Please find this project's tasks and move them into FlipHopper (conversion request v{version}).

Project folder: {project}
Seabag (FlipHopper's task files): {seabag}
Run every `fh` command below as `{with_seabag} …`.

Ask the person exactly once: the full plan, confirmed in the FlipHopper app with `fh confirm`. \
Do not ask questions in the chat; decide the details yourself with the defaults below and put \
them in the plan, so the person can cancel if they disagree.

Work in this order:

1. Look first, change nothing. Find whether and where this project keeps its tasks, their \
status, who holds each one, their labels, areas and next steps (for example a task board, card \
files or a to-do list), and which rules in AGENTS.md / CLAUDE.md describe them. Do not run the \
project's scripts or hooks. If you find no tasks, tell the person and stop.
2. Decide, and write it in the plan: keep the project's own task IDs (drop only prefixes like \
`Task-` and the trailing description); keep the project's own label ladder, in order; map the \
files' words to ready, waiting, done and dropped; add planned work that has no card yet as a \
backlog task saying \"no card yet\"; write task titles in the language `fh task-language` gives \
(translate a card's title when it is in another language; the card link keeps the original).
3. If the project is a Git repository, note the current branch (the original branch). If the \
files you will change (AGENTS.md, CLAUDE.md, card and board files) have uncommitted changes, \
tell the person to commit or set them aside first, and stop; never mix them in.
4. The one question. Write the whole plan in Markdown to a file outside the project (for \
example in the system's temporary folder): the mapping table, the label ladder, the decisions \
you made, the rules you will replace in AGENTS.md, the old status you will remove (status tables \
in card files, generated board sections, session registries), the working branch \
`fliphopper/convert`, and \"then commit and merge into <original branch>; nothing is pushed\". \
Then run `fh confirm --title \"{title}\" --plan <file>`. It shows the plan in the app and waits \
for the person's answer. Exit code 0 means go ahead with the whole plan; any other exit code \
means the person cancelled or did not answer: change nothing and stop, and tell the person that \
nothing changed and that, if they change their mind, they can ask you here to show the plan \
again, or choose File → Move Tasks Into FlipHopper… in FlipHopper to start again.
5. Create the branch `fliphopper/convert` from the original branch and work on it.
6. Write the seabag only with `fh`; never edit its files by hand. The seabag already exists \
(FlipHopper created it): never create another or delete it; if it already holds tasks, only add \
to them and never change those. If the project's label ladder differs from the seabag's, set it \
with `fh labels \"<ladder, comma-separated>\"`. Then `fh add area <id> \"<name>\"` for each area \
and `fh add task <id> --area <id> --title \"<title>\" --description \"Card: \
[<path>]({up}<path from the project folder>)\"` for each task (link to `file.md#section` when a \
card is a section of a bigger file).
7. Copy status exactly as the files say it: `fh --as <holder> claim <id>`, `label`, `wait --on`, \
`needs`, `ready`, `done`, `drop`. Copy only what is written: an unknown state, date or holder \
stays unknown and goes in the report; never invent progress, verification or approvals. Values \
`fh` cannot record yet (done dates, dependencies, priority, approvals and the like) go in the \
report.
8. Write a short report, `docs/fliphopper-conversion.md`: every card file and the task ID it \
became, nothing dropped or duplicated; every value that could not be copied, with file and line. \
Run `fh list --json` and confirm it reports no issues.
9. Switch the instructions over. In AGENTS.md, replace the rules about where status lives, how \
to claim and release, session registries and regenerating the board with FlipHopper's section \
(the output of `fh brief`, see the last line); keep every other rule word for word. Make sure \
CLAUDE.md has the line that imports AGENTS.md (a plain ASCII at sign followed by AGENTS.md).
10. Remove the old status: status tables in card files, generated board sections and session \
registries. Keep each card's outcome, approvals and notes as plain text. Remove only what the \
plan named; leave scripts, hooks and CI checks that also do other things alone.
11. Commit only the files you changed (never unrelated files such as .DS_Store), merge with \
`git merge --no-ff fliphopper/convert` into the original branch, then delete \
`fliphopper/convert`. Never push.
12. Tell the person what you did, the merge commit, and how to undo the whole move: \
`git revert -m 1 <merge commit>`.

{source}"
        ),
    };
    format!("{steps}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, agent_section_for};
    use std::path::PathBuf;

    #[test]
    fn convert_request_carries_paths_confirm_and_the_section() {
        let project = PathBuf::from("/work/bakery");
        let seabag = project.join(DEFAULT_DIR);
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_convert_request_in(language, &project, &seabag);
            assert!(r.contains("/work/bakery"), "{language}");
            assert!(r.contains("fh --seabag \"/work/bakery/.fliphopper/seabag\""));
            assert!(r.contains(&format!("fh confirm --title \"{CONFIRM_TITLE}\" --plan")));
            assert!(r.contains("fh labels"));
            assert!(r.contains("fh task-language"), "{language}");
            assert!(
                r.contains("/work/bakery/.fliphopper/seabag\" brief`"),
                "{language}"
            );
            assert!(
                !r.contains("fh init"),
                "the app made the seabag: {language}"
            );
            assert!(r.contains("](../../../../"));
            assert!(r.contains("docs/fliphopper-conversion.md"));
            assert!(r.contains(&format!("v{CONVERT_REQUEST_VERSION}")));
            // The section comes from `fh brief`; the request carries no copy of it.
            assert!(!r.contains("fh claim <id>"), "{language}");
            assert!(
                !r.contains('@'),
                "the Claude app shows a typed-in @ as ＠: {language}"
            );
        }
        let en = agent_convert_request_in("en", &project, &seabag);
        assert!(en.contains("Look first, change nothing."));
        assert!(en.contains("If you find no tasks, tell the person and stop."));
        assert!(en.contains("Exit code 0 means go ahead with the whole plan"));
        // One question only, and Go ahead covers committing and merging, never pushing.
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_convert_request_in(language, &project, &seabag);
            assert!(
                r.contains("git merge --no-ff fliphopper/convert"),
                "{language}"
            );
            assert!(r.contains("git revert -m 1"), "{language}");
            assert!(!r.contains("--push"), "{language}");
        }
        assert!(en.contains("Do not ask questions in the chat"));
        assert!(agent_convert_request_in("ja", &project, &seabag).contains("まず見るだけ"));
        assert!(agent_convert_request_in("zh-CN", &project, &seabag).contains("先查看，不要改动"));
    }

    #[test]
    fn a_seabag_beside_the_project_is_named_in_links_and_the_section() {
        let project = PathBuf::from("/work/app");
        let seabag = PathBuf::from("/work/tasks-home").join(DEFAULT_DIR);
        let r = agent_convert_request_in("en", &project, &seabag);
        assert!(r.contains("](../../../../../app/"), "{r}");
        // `fh brief` (agent_section_for) names the seabag, relative to the project folder.
        let section = agent_section_for(&project, &seabag);
        assert!(
            section.contains("--seabag \"../tasks-home/.fliphopper/seabag\""),
            "{section}"
        );
    }
}
