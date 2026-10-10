//! Moving a project that already tracks its tasks another way into FlipHopper (Task-A05-02).
//!
//! FlipHopper does not look for or convert anything itself. The app asks one AI agent the person
//! picks to find the project's tasks and move them, with the request built by
//! [`agent_convert_request_in`]. Before changing anything the agent shows its plan in the app with
//! `fh confirm`, where the person can cancel; it then writes the seabag only through `fh`.

use std::path::Path;

use crate::{relative_path, section_source};

/// Changes when the conversion request's steps change, so a report can say which it followed.
/// 2: the agent finds the tasks itself and asks with `fh confirm` before changing anything.
/// 3: the app has already created the seabag and recorded the project folder; the ladder is set
/// with `fh labels`, and card links are relative to the recorded project folder.
pub const CONVERT_REQUEST_VERSION: u32 = 3;

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
        "zh-CN" => {
            let create = "水手袋已经存在（由 FlipHopper 建立）：不要再建一个，也不要删除它。如果它已经有任务，只添加，不改动那些任务。如果项目自己的标签阶梯与水手袋的不同（见 project.json 中的 labels），在计划中写明，确认后用 `fh labels \"<阶梯，用逗号分隔>\"` 设置。";
            format!(
                "请找出这个项目的任务，并把它们搬进 FlipHopper（转换请求 v{version}）。

项目文件夹：{project}
水手袋（FlipHopper 的任务文件）：{seabag}
下面每条 `fh` 命令都请这样运行：`{with_seabag} …`

请按顺序做：

1. 先查看，不要改动。找出这个项目是否、以及在哪里记录任务、状态、负责人、标签、领域和下一步（例如任务看板、卡片文件、待办清单），以及 AGENTS.md / CLAUDE.md 中哪些规则描述了它们。不要运行项目的脚本或钩子。如果没有找到任务，告诉用户，然后停止。
2. 如果有不清楚的地方——领域和任务 ID 如何对应（保留项目自己的 ID，只去掉 `Task-` 这样的前缀和结尾的描述）、哪些词组成标签阶梯（保留项目自己的，按顺序）、哪些词表示 ready、waiting、done、dropped、还没有卡片的计划工作怎么处理——先在对话中问用户。
3. 在改动任何东西之前，请用户在 FlipHopper 应用中确认。把计划用 Markdown 写进项目之外的文件（例如系统的临时文件夹）：对应表、标签阶梯、状态词、要放进报告的内容、分支名、AGENTS.md 中要替换的规则。然后运行 `fh confirm --title \"{title}\" --plan <文件>`。它会在应用中弹出计划并等待用户回答。退出码 0 表示可以继续；其他退出码表示用户取消或没有回答：什么都不要改，然后停止，并告诉用户：没有任何改动；如果改主意了，可以在这里让你再给他看一次计划，或在 FlipHopper 中选择“文件 → 把任务搬进 FlipHopper…”重新开始。如果用户说出要改的地方，修改计划后再确认一次。
4. 如果项目是 Git 仓库，在新分支上工作（`fliphopper/convert`，除非用户另起名字）。转换期间不要用 `--push`；只有用户要求时才提交。
5. 只用 `fh` 写水手袋，绝不手工编辑它的文件。{create}先运行 `fh task-language`：任务标题用它给出的语言写（原文不是这种语言时就翻译；卡片链接保留原文）。然后每个领域 `fh add area <id> \"<名称>\"`，每个任务 `fh add task <id> --area <id> --title \"<标题>\" --description \"Card: [<路径>]({up}<从项目文件夹起的路径>)\"`（卡片是大文件中的一节时，链接到 `文件.md#节`）。
6. 照文件所写原样复制状态：`fh --as <负责人> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。只复制写明的内容：不明的状态、日期或负责人保持不明，并写进报告；绝不编造进度、验证或批准。批准和状态说明留在卡片文件里。`fh` 还记不下的值（完成日期、依赖、优先级等）也写进报告。
7. 写一份简短报告 `docs/fliphopper-conversion.md`：每个卡片文件和它对应的任务 ID，没有遗漏或重复；每个无法复制的值，注明文件和行；没有搬过来的计划工作。运行 `fh list --json`，确认没有 issues。
8. 切换说明。在 AGENTS.md 中，用 FlipHopper 的一节（`fh brief` 的输出，见最后一行）替换关于状态存放位置、如何认领和释放、会话登记表、重新生成看板的规则；其他规则一字不改。确保 CLAUDE.md 包含导入 AGENTS.md 的一行（半角 at 符号后接 AGENTS.md）。给用户看改动。
9. 列出、但不要执行收尾工作：说出仍然保存旧状态的东西（卡片文件里的状态表、生成的看板段落、生成看板的脚本、强制它们的钩子和 CI 检查），并问用户是否以及何时删除。用户同意之前保持原样。
10. 告诉用户改了什么，以及还在等他们决定的事。

{source}"
            )
        }
        "ja" => {
            let create = "シーバッグはすでにあります（FlipHopper が作りました）。新しく作ったり削除したりしないでください。すでにタスクがあれば追加だけにし、それらのタスクは変えません。プロジェクト自身のラベルの段階がシーバッグのもの（project.json の labels）と違う場合は、計画に書き、確認を得てから `fh labels \"<段階、カンマ区切り>\"` で設定します。";
            format!(
                "このプロジェクトのタスクを見つけて、FlipHopper に移してください（移行の依頼 v{version}）。

プロジェクトのフォルダー：{project}
シーバッグ（FlipHopper のタスクファイル）：{seabag}
以下の `fh` コマンドはすべて `{with_seabag} …` の形で実行してください。

順に進めてください：

1. まず見るだけで、何も変えません。このプロジェクトがタスク、状況、担当者、ラベル、エリア、次の一歩を記録しているか、どこに記録しているか（タスクボード、カードのファイル、TODO リストなど）、AGENTS.md / CLAUDE.md のどのルールがそれを説明しているかを調べます。プロジェクトのスクリプトやフックは実行しません。タスクが見つからなければ、そう伝えて終わります。
2. 判断のつかない点——エリアとタスク ID の対応（プロジェクト自身の ID を残し、`Task-` のような接頭辞と末尾の説明だけを外す）、どの言葉がラベルの段階になるか（プロジェクト自身のものを順に）、どの言葉が ready、waiting、done、dropped にあたるか、まだカードのない予定の作業をどうするか——があれば、先にチャットでユーザーに尋ねます。
3. 何かを変える前に、FlipHopper アプリで確認を取ります。計画を Markdown でプロジェクトの外のファイル（たとえばシステムの一時フォルダー）に書きます：対応表、ラベルの段階、状況の言葉、レポートに書く値、ブランチ名、AGENTS.md で置き換えるルール。そして `fh confirm --title \"{title}\" --plan <ファイル>` を実行します。アプリに計画が表示され、ユーザーの答えを待ちます。終了コード 0 なら続けます。それ以外はキャンセルか無回答です：何も変えずに終わり、ユーザーに伝えます：何も変わっていないこと、考えが変わったらここでもう一度計画を見せるよう頼めること、または FlipHopper の「ファイル → タスクを FlipHopper に移す…」からやり直せること。直したい点を言われたら、計画を直してもう一度確認します。
4. プロジェクトが Git リポジトリなら、新しいブランチで作業します（ユーザーが別の名前を言わなければ `fliphopper/convert`）。移行中は `--push` を使わず、ユーザーに頼まれたときだけコミットします。
5. シーバッグは `fh` だけで書き、ファイルを手で編集しません。{create}まず `fh task-language` を実行し、タスクの題名はその言語で書きます（原文が別の言語なら訳し、原文はカードへのリンクで残します）。続いてエリアごとに `fh add area <id> \"<名前>\"`、タスクごとに `fh add task <id> --area <id> --title \"<題名>\" --description \"Card: [<パス>]({up}<プロジェクトのフォルダーからのパス>)\"`（カードが大きなファイルの一節なら `ファイル.md#節` へリンク）。
6. 状況はファイルに書かれているとおりに写します：`fh --as <担当者> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。書かれていることだけを写し、わからない状況・日付・担当者はわからないままにしてレポートに書きます。進捗、検証、承認を作り出してはいけません。承認と状況の説明文はカードのファイルに残します。`fh` がまだ記録できない値（完了日、依存関係、優先度など）もレポートに書きます。
7. 短いレポート `docs/fliphopper-conversion.md` を書きます：カードのファイルごとに対応するタスク ID（抜けも重複もないこと）、写せなかった値とそのファイルと行、移さなかった予定の作業。`fh list --json` を実行し、issues がないことを確かめます。
8. 説明を切り替えます。AGENTS.md で、状況の置き場所、担当の取り方と手放し方、セッション一覧、ボードの再生成についてのルールを、FlipHopper の節（`fh brief` の出力。最後の行を参照）で置き換えます。ほかのルールは一字も変えません。CLAUDE.md に AGENTS.md を読み込む行（半角のアットマークに続けて AGENTS.md）があることを確かめます。差分をユーザーに見せます。
9. 切り替えの後片付けは、挙げるだけで実行しません：古い状況をまだ持っているもの（カードのファイルの状況表、生成されるボードの節、ボードを生成するスクリプト、それを強制するフックや CI のチェック）を挙げ、消すかどうか、いつ消すかをユーザーに尋ねます。ユーザーが同意するまでそのままにします。
10. 何を変えたか、何がユーザーの判断を待っているかを伝えます。

{source}"
            )
        }
        _ => {
            let create = "The seabag already exists (FlipHopper created it): never create another \
or delete it. If it already holds tasks, only add to them; never change those. If the project's own \
label ladder differs from the seabag's (`labels` in its project.json), say so in your plan and, \
once confirmed, set it with `fh labels \"<ladder, comma-separated>\"`. ";
            format!(
                "Please find this project's tasks and move them into FlipHopper (conversion request v{version}).

Project folder: {project}
Seabag (FlipHopper's task files): {seabag}
Run every `fh` command below as `{with_seabag} …`.

Work in this order:

1. Look first, change nothing. Find whether and where this project keeps its tasks, their \
status, who holds each one, their labels, areas and next steps (for example a task board, card \
files or a to-do list), and which rules in AGENTS.md / CLAUDE.md describe them. Do not run the \
project's scripts or hooks. If you find no tasks, tell the person and stop.
2. If anything is unclear — how areas and task IDs map (keep the project's own IDs, dropping \
only prefixes like `Task-` and the trailing description), which words form the label ladder \
(keep the project's own, in order), which words mean ready, waiting, done and dropped, what to do \
with planned work that has no card yet — ask the person in the chat first.
3. Before you change anything, ask the person to confirm in the FlipHopper app. Write your plan \
in Markdown to a file outside the project (for example in the system's temporary folder): the \
mapping table, the label ladder, the state words, what goes in the report, the branch name, and \
the rules you will replace in AGENTS.md. Then run `fh confirm --title \"{title}\" --plan <file>`. \
It shows the plan in the app and waits for the person's answer. Exit code 0 means go ahead; any \
other exit code means the person cancelled or did not answer: change nothing and stop, and tell \
the person that nothing changed and that, if they change their mind, they can ask you here to \
show the plan again, or choose File → Move Tasks Into FlipHopper… in FlipHopper to start again. \
If they say what to change, change the plan and confirm again.
4. If the project is a Git repository, work on a new branch (`fliphopper/convert` unless the \
person names one). Never use `--push` during the conversion; commit only when the person asks.
5. Write the seabag only with `fh`; never edit its files by hand. {create}Run `fh task-language` and write task titles in the \
language it gives (translate a card's title when it is in another language; the card link keeps \
the original). Then `fh add area <id> \
\"<name>\"` for each area and `fh add task <id> --area <id> --title \"<title>\" --description \
\"Card: [<path>]({up}<path from the project folder>)\"` for each task (link to `file.md#section` \
when a card is a section of a bigger file).
6. Copy status exactly as the files say it: `fh --as <holder> claim <id>`, `label`, `wait --on`, \
`needs`, `ready`, `done`, `drop`. Copy only what is written: an unknown state, date or holder \
stays unknown and goes in the report; never invent progress, verification or approvals. \
Approvals and status prose stay in the card files. Values `fh` cannot record yet (done dates, \
dependencies, priority and the like) go in the report too.
7. Write a short report, `docs/fliphopper-conversion.md`: every card file and the task ID it \
became, nothing dropped or duplicated; every value that could not be copied, with file and \
line; planned work left out. Run `fh list --json` and confirm it reports no issues.
8. Switch the instructions over. In AGENTS.md, replace the rules about where status lives, how \
to claim and release, session registries and regenerating the board with FlipHopper's section \
(the output of `fh brief`, see the last line); keep every other rule word for word. Make sure CLAUDE.md has the line that imports AGENTS.md (a plain ASCII at sign followed by \
AGENTS.md). \
Show the person the diff.
9. List, do not do, the cut-over: name what still holds the old status (status tables in card \
files, generated board sections, board-generator scripts, hooks and CI checks that enforce \
them) and ask the person whether and when to remove them. Leave them in place until they agree.
10. Tell the person what changed and what waits on them.

{source}"
            )
        }
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
        assert!(en.contains("Exit code 0 means go ahead"));
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
