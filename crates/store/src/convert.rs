//! Moving a project that already tracks its tasks another way into FlipHopper (Task-A05-02).
//!
//! FlipHopper does not convert anything itself. [`find_task_system`] only notices the signs of
//! another task system (a board file, card files, agent rules about claiming); the app then asks
//! one AI agent the person picks to do the move, with the request built by
//! [`agent_convert_request_in`]. The agent writes the seabag only through `fh`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{DEFAULT_DIR, agent_section};

/// Changes when the conversion request's steps change, so a report can say which it followed.
pub const CONVERT_REQUEST_VERSION: u32 = 1;

/// How deep [`find_task_system`] looks below the project folder. Deep enough for
/// `<app>/docs/<plan>/<area>/Task-….md`.
const MAX_DEPTH: usize = 6;
/// At most this many folder entries are read, so a huge repository cannot stall the app.
const MAX_ENTRIES: usize = 50_000;
/// Folders never looked into: build output and installed packages, never task files.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "build",
    "dist",
    "out",
    "vendor",
    "venv",
    "Pods",
    "DerivedData",
];
/// At most this many paths of each kind are listed (the count of card files is exact).
const MAX_LISTED: usize = 5;

/// Signs that a project already tracks its tasks without FlipHopper. Paths are relative to the
/// project folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TaskSystem {
    /// Task board files, for example `docs/TASK_BOARD.md`.
    pub boards: Vec<PathBuf>,
    /// How many card files (`Task-….md`) there are.
    pub cards: usize,
    /// The folders holding the most card files, most first.
    pub card_folders: Vec<PathBuf>,
    /// `AGENTS.md` / `CLAUDE.md` at the project folder with rules about a task board or claiming.
    pub rules: Vec<PathBuf>,
}

impl TaskSystem {
    /// A board, or at least two card files. Agent rules alone are not enough: many projects
    /// mention a board without keeping one in the repository.
    pub fn found(&self) -> bool {
        !self.boards.is_empty() || self.cards >= 2
    }
}

/// A Markdown file named like a task board: `TASK_BOARD.md`, `task-board.md`, `taskboard.md`,
/// `BOARD.md`, `KANBAN.md`, `TASKS.md` (any case).
fn is_board(name: &str) -> bool {
    let lower = name.to_lowercase();
    let Some(stem) = lower.strip_suffix(".md") else {
        return false;
    };
    let squashed: String = stem.chars().filter(|c| *c != '-' && *c != '_').collect();
    squashed.contains("taskboard") || matches!(squashed.as_str(), "board" | "kanban" | "tasks")
}

/// A Markdown file named like a card: `task-` or `task_`, then an ID with a digit in it
/// (`Task-A01-02-something.md`, `TASK_100_PLAN.md`, `task_7.md`). `TASK_REORDERING.md` is a
/// note, not a card.
fn is_card(name: &str) -> bool {
    let lower = name.to_lowercase();
    let Some(stem) = lower.strip_suffix(".md") else {
        return false;
    };
    let Some(rest) = stem
        .strip_prefix("task-")
        .or_else(|| stem.strip_prefix("task_"))
    else {
        return false;
    };
    rest.split(['-', '_'])
        .next()
        .is_some_and(|id| id.chars().any(|c| c.is_ascii_digit()))
}

/// Agent instructions that describe another way of tracking tasks.
fn has_task_rules(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "task board",
        "task_board",
        "taskboard",
        "claiming protocol",
        "session registry",
    ]
    .iter()
    .any(|w| lower.contains(w))
}

/// Look in `project` for the signs of another task system. Reads names only, plus the root's
/// `AGENTS.md` and `CLAUDE.md`; changes nothing. Hidden folders (`.git`, `.claude`, `.fliphopper`
/// and other tools' copies of the project) and build output are not looked into.
pub fn find_task_system(project: &Path) -> TaskSystem {
    let mut found = TaskSystem::default();
    let mut folders: Vec<(PathBuf, usize)> = Vec::new();
    let mut stack = vec![(project.to_path_buf(), 0usize)];
    let mut read = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        let mut cards_here = 0usize;
        for entry in entries {
            read += 1;
            if read > MAX_ENTRIES {
                break;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if depth < MAX_DEPTH && !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push((path, depth + 1));
                }
            } else if kind.is_file() {
                let relative = path.strip_prefix(project).unwrap_or(&path).to_path_buf();
                // A board first: `TASK_BOARD.md` also starts like a card.
                if is_board(&name) {
                    found.boards.push(relative);
                } else if is_card(&name) {
                    cards_here += 1;
                }
            }
        }
        if cards_here > 0 {
            found.cards += cards_here;
            folders.push((
                dir.strip_prefix(project).unwrap_or(&dir).to_path_buf(),
                cards_here,
            ));
        }
        if read > MAX_ENTRIES {
            break;
        }
    }
    found.boards.sort();
    found.boards.truncate(MAX_LISTED);
    folders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    found.card_folders = folders
        .into_iter()
        .take(MAX_LISTED)
        .map(|(p, _)| p)
        .collect();
    for file in ["AGENTS.md", "CLAUDE.md"] {
        if fs::read_to_string(project.join(file)).is_ok_and(|t| has_task_rules(&t)) {
            found.rules.push(PathBuf::from(file));
        }
    }
    found
}

/// A path for the request: relative paths as written, `.` for the project folder itself.
fn shown(path: &Path) -> String {
    let text = path.display().to_string();
    if text.is_empty() { ".".into() } else { text }
}

/// What was found, as plain lines for the request (the same in every language: paths only).
fn findings(found: &TaskSystem) -> String {
    let mut lines = Vec::new();
    for board in &found.boards {
        lines.push(format!("- {}", shown(board)));
    }
    if found.cards > 0 {
        let folders: Vec<String> = found.card_folders.iter().map(|f| shown(f)).collect();
        lines.push(format!(
            "- {} × Task-*.md: {}",
            found.cards,
            folders.join(", ")
        ));
    }
    for rules in &found.rules {
        lines.push(format!("- {}", shown(rules)));
    }
    lines.join("\n")
}

/// The request that asks one AI agent to move the project's tasks into FlipHopper, in an
/// interface language (`en`, `zh-CN`, `ja`; English otherwise). `seabag` is where the seabag is
/// or will be; `seabag_exists` says which. The steps follow the language; the commands, the paths
/// and FlipHopper's section stay as they are, so every agent runs the same commands.
pub fn agent_convert_request_in(
    language: &str,
    project: &Path,
    seabag: &Path,
    found: &TaskSystem,
    seabag_exists: bool,
) -> String {
    let (project, seabag) = (project.display(), seabag.display());
    let findings = findings(found);
    let version = CONVERT_REQUEST_VERSION;
    // From a task's description.md, the project folder is four levels up, when the seabag is in
    // the usual place.
    let up = if seabag.to_string().ends_with(DEFAULT_DIR) {
        "../../../../"
    } else {
        ""
    };
    let fh = format!("fh --seabag \"{seabag}\"");
    let steps = match (language, seabag_exists) {
        ("zh-CN", exists) => {
            let create = if exists {
                format!(
                    "水手袋已经存在。用 `{fh} list` 查看它；如果它还没有任务，而商定的标签阶梯与 project.json 中的不同，先征得用户同意，再删除这个空的水手袋文件夹，然后像下面这样重新创建。"
                )
            } else {
                "水手袋还不存在。".to_string()
            };
            format!(
                "请把这个项目的任务搬进 FlipHopper（转换请求 v{version}）。

项目文件夹：{project}
水手袋（FlipHopper 的任务文件）：{seabag}

FlipHopper 发现这个项目已经用别的方式跟踪任务：
{findings}

请按顺序做：

1. 先查看，不要改动。找出这个项目把任务、状态、负责人、标签、领域和下一步记在哪里，以及 AGENTS.md / CLAUDE.md 中哪些规则描述了它们。不要运行项目的脚本或钩子。
2. 给用户看计划并提问，然后等待回答：领域和任务 ID 如何对应（保留项目自己的 ID，只去掉 `Task-` 这样的前缀和结尾的描述）；哪些词组成标签阶梯（保留项目自己的，按顺序）；哪些词表示 ready、waiting、done、dropped；还没有卡片的计划工作怎么处理；以及任何不确定的地方。
3. 如果项目是 Git 仓库，在新分支上工作（`fliphopper/convert`，除非用户另起名字）。转换期间不要用 `--push`；只有用户要求时才提交。
4. 只用 `fh` 写水手袋，绝不手工编辑它的文件。{create}创建：`{fh} init --name \"<名称>\" --id <id> --labels \"<阶梯，用逗号分隔>\"`。然后每个领域 `{fh} add area <id> \"<名称>\"`，每个任务 `{fh} add task <id> --area <id> --title \"<标题>\" --description \"Card: [<路径>]({up}<从项目文件夹起的路径>)\"`（卡片是大文件中的一节时，链接到 `文件.md#节`）。
5. 照文件所写原样复制状态：`{fh} --as <负责人> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。只复制写明的内容：不明的状态、日期或负责人保持不明，并写进报告；绝不编造进度、验证或批准。批准和状态说明留在卡片文件里。`fh` 还记不下的值（完成日期、依赖、优先级等）也写进报告。
6. 写一份简短报告 `docs/fliphopper-conversion.md`：每个卡片文件和它对应的任务 ID，没有遗漏或重复；每个无法复制的值，注明文件和行；没有搬过来的计划工作。运行 `{fh} list --json`，确认没有 issues。
7. 切换说明。在 AGENTS.md 中，用下面的 FlipHopper 一节替换关于状态存放位置、如何认领和释放、会话登记表、重新生成看板的规则；其他规则一字不改。确保 CLAUDE.md 包含一行 @AGENTS.md。给用户看改动。（如果 `fh init` 打印了添加说明的请求，以这一步为准。）
8. 列出、但不要执行收尾工作：说出仍然保存旧状态的东西（卡片文件里的状态表、生成的看板段落、生成看板的脚本、强制它们的钩子和 CI 检查），并问用户是否以及何时删除。用户同意之前保持原样。
9. 告诉用户改了什么，以及还在等他们决定的事。

下面这一节请保持英文原样："
            )
        }
        ("ja", exists) => {
            let create = if exists {
                format!(
                    "シーバッグはすでにあります。`{fh} list` で確かめてください。まだタスクがなく、合意したラベルの段階が project.json と違う場合は、ユーザーの同意を得てからその空のシーバッグのフォルダーを削除し、下のように作り直します。"
                )
            } else {
                "シーバッグはまだありません。".to_string()
            };
            format!(
                "このプロジェクトのタスクを FlipHopper に移してください（移行の依頼 v{version}）。

プロジェクトのフォルダー：{project}
シーバッグ（FlipHopper のタスクファイル）：{seabag}

FlipHopper は、このプロジェクトがすでに別の方法でタスクを管理していることを見つけました：
{findings}

順に進めてください：

1. まず見るだけで、何も変えません。タスク、状況、担当者、ラベル、エリア、次の一歩がどこに書かれているか、AGENTS.md / CLAUDE.md のどのルールがそれを説明しているかを調べます。プロジェクトのスクリプトやフックは実行しません。
2. 計画をユーザーに見せて質問し、答えを待ちます：エリアとタスク ID の対応（プロジェクト自身の ID を残し、`Task-` のような接頭辞と末尾の説明だけを外す）、どの言葉がラベルの段階になるか（プロジェクト自身のものを順に）、どの言葉が ready、waiting、done、dropped にあたるか、まだカードのない予定の作業をどうするか、そのほか判断のつかない点。
3. プロジェクトが Git リポジトリなら、新しいブランチで作業します（ユーザーが別の名前を言わなければ `fliphopper/convert`）。移行中は `--push` を使わず、ユーザーに頼まれたときだけコミットします。
4. シーバッグは `fh` だけで書き、ファイルを手で編集しません。{create}作成：`{fh} init --name \"<名前>\" --id <id> --labels \"<段階、カンマ区切り>\"`。続いてエリアごとに `{fh} add area <id> \"<名前>\"`、タスクごとに `{fh} add task <id> --area <id> --title \"<題名>\" --description \"Card: [<パス>]({up}<プロジェクトのフォルダーからのパス>)\"`（カードが大きなファイルの一節なら `ファイル.md#節` へリンク）。
5. 状況はファイルに書かれているとおりに写します：`{fh} --as <担当者> claim <id>`、`label`、`wait --on`、`needs`、`ready`、`done`、`drop`。書かれていることだけを写し、わからない状況・日付・担当者はわからないままにしてレポートに書きます。進捗、検証、承認を作り出してはいけません。承認と状況の説明文はカードのファイルに残します。`fh` がまだ記録できない値（完了日、依存関係、優先度など）もレポートに書きます。
6. 短いレポート `docs/fliphopper-conversion.md` を書きます：カードのファイルごとに対応するタスク ID（抜けも重複もないこと）、写せなかった値とそのファイルと行、移さなかった予定の作業。`{fh} list --json` を実行し、issues がないことを確かめます。
7. 説明を切り替えます。AGENTS.md で、状況の置き場所、担当の取り方と手放し方、セッション一覧、ボードの再生成についてのルールを、下の FlipHopper の節で置き換えます。ほかのルールは一字も変えません。CLAUDE.md に @AGENTS.md という行があることを確かめます。差分をユーザーに見せます。（`fh init` が説明を追加する依頼を表示しても、この手順を優先します。）
8. 切り替えの後片付けは、挙げるだけで実行しません：古い状況をまだ持っているもの（カードのファイルの状況表、生成されるボードの節、ボードを生成するスクリプト、それを強制するフックや CI のチェック）を挙げ、消すかどうか、いつ消すかをユーザーに尋ねます。ユーザーが同意するまでそのままにします。
9. 何を変えたか、何がユーザーの判断を待っているかを伝えます。

下の節は英語のままにしてください："
            )
        }
        (_, exists) => {
            let create = if exists {
                format!(
                    "The seabag already exists. Look at it with `{fh} list`; if it has no tasks yet and the agreed label ladder differs from the one in its project.json, ask the person, then delete that empty seabag folder and create it again as below. "
                )
            } else {
                "The seabag does not exist yet. ".to_string()
            };
            format!(
                "Please move this project's tasks into FlipHopper (conversion request v{version}).

Project folder: {project}
Seabag (FlipHopper's task files): {seabag}

FlipHopper found that this project already tracks its tasks another way:
{findings}

Work in this order:

1. Look first, change nothing. Find where this project keeps its tasks, their status, who \
holds each one, their labels, areas and next steps, and which rules in AGENTS.md / CLAUDE.md \
describe them. Do not run the project's scripts or hooks.
2. Show the person your plan, ask, and wait for the answer: how areas and task IDs map (keep \
the project's own IDs, dropping only prefixes like `Task-` and the trailing description); which \
words form the label ladder (keep the project's own, in order); which words mean ready, waiting, \
done and dropped; what to do with planned work that has no card yet; and anything ambiguous.
3. If the project is a Git repository, work on a new branch (`fliphopper/convert` unless the \
person names one). Never use `--push` during the conversion; commit only when the person asks.
4. Write the seabag only with `fh`; never edit its files by hand. {create}Create it: `{fh} init \
--name \"<name>\" --id <id> --labels \"<ladder, comma-separated>\"`. Then `{fh} add area <id> \
\"<name>\"` for each area and `{fh} add task <id> --area <id> --title \"<title>\" --description \
\"Card: [<path>]({up}<path from the project folder>)\"` for each task (link to `file.md#section` \
when a card is a section of a bigger file).
5. Copy status exactly as the files say it: `{fh} --as <holder> claim <id>`, `label`, `wait \
--on`, `needs`, `ready`, `done`, `drop`. Copy only what is written: an unknown state, date or \
holder stays unknown and goes in the report; never invent progress, verification or approvals. \
Approvals and status prose stay in the card files. Values `fh` cannot record yet (done dates, \
dependencies, priority and the like) go in the report too.
6. Write a short report, `docs/fliphopper-conversion.md`: every card file and the task ID it \
became, nothing dropped or duplicated; every value that could not be copied, with file and \
line; planned work left out. Run `{fh} list --json` and confirm it reports no issues.
7. Switch the instructions over. In AGENTS.md, replace the rules about where status lives, how \
to claim and release, session registries and regenerating the board with FlipHopper's section \
below; keep every other rule word for word. Make sure CLAUDE.md contains the line @AGENTS.md. \
Show the person the diff. (If `fh init` prints a request to add instructions, this step \
replaces it.)
8. List, do not do, the cut-over: name what still holds the old status (status tables in card \
files, generated board sections, board-generator scripts, hooks and CI checks that enforce \
them) and ask the person whether and when to remove them. Leave them in place until they agree.
9. Tell the person what changed and what waits on them.

FlipHopper's section, to add as it is:"
            )
        }
    };
    format!("{steps}\n\n{}", agent_section())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("fliphopper-convert-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// A made-up Markdown board shaped like the owner's: a board, card files in an area
    /// folder, a card copy inside a hidden worktree, and agent rules about claiming.
    fn markdown_board(root: &Path) {
        write(
            root,
            "AGENTS.md",
            "# Guide\n\nClaim a card in the session registry first.\n",
        );
        write(root, "CLAUDE.md", "@AGENTS.md\n");
        write(root, "app/docs/plan/TASK_BOARD.md", "# Task board\n");
        write(root, "app/docs/plan/areas/A01-shell.md", "# A1 — Shell\n");
        write(
            root,
            "app/docs/plan/areas/Task-A01-01-sign-in.md",
            "# Task-A01-01\n",
        );
        write(
            root,
            "app/docs/plan/areas/Task-A01-02-sign-out.md",
            "# Task-A01-02\n",
        );
        write(
            root,
            "app/docs/plan/old/Task-A00-01-plan.md",
            "# Task-A00-01\n",
        );
        write(
            root,
            ".claude/worktrees/x/app/docs/plan/areas/Task-A09-09-copy.md",
            "",
        );
        write(root, "node_modules/pkg/Task-A9-dep.md", "");
        write(root, "app/docs/plan/TASK_SORTING_NOTES.md", "");
    }

    #[test]
    fn notices_a_markdown_board() {
        let dir = tmp("board");
        markdown_board(&dir);
        let found = find_task_system(&dir);
        assert!(found.found());
        assert_eq!(found.boards, [PathBuf::from("app/docs/plan/TASK_BOARD.md")]);
        // Hidden folders and installed packages are not looked into; area files are not cards.
        assert_eq!(found.cards, 3);
        assert_eq!(
            found.card_folders,
            [
                PathBuf::from("app/docs/plan/areas"),
                PathBuf::from("app/docs/plan/old")
            ]
        );
        assert_eq!(found.rules, [PathBuf::from("AGENTS.md")]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_plain_project_or_an_empty_folder_is_not_a_task_system() {
        let dir = tmp("plain");
        assert_eq!(find_task_system(&dir), TaskSystem::default());
        write(&dir, "README.md", "# App\n");
        write(&dir, "src/main.rs", "fn main() {}\n");
        write(&dir, "docs/tasks.txt", "not markdown\n");
        // One card file, or rules alone, are not enough; notes named Task-… are not cards.
        write(&dir, "docs/Task-1-notes.md", "");
        write(&dir, "docs/TASK_REORDERING.md", "");
        write(&dir, "AGENTS.md", "Keep the task board tidy.\n");
        let found = find_task_system(&dir);
        assert!(!found.found());
        assert_eq!(found.cards, 1);
        assert_eq!(found.rules, [PathBuf::from("AGENTS.md")]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn board_and_card_names() {
        for name in [
            "TASK_BOARD.md",
            "task-board.md",
            "TaskBoard.md",
            "BOARD.md",
            "kanban.md",
            "TASKS.md",
        ] {
            assert!(is_board(name), "{name}");
        }
        for name in ["README.md", "TASK_BOARD.txt", "boarding.md", "TODO.md"] {
            assert!(!is_board(name), "{name}");
        }
        for name in ["Task-A01-02-x.md", "task_7.md", "TASK_100_PLAN.md"] {
            assert!(is_card(name), "{name}");
        }
        for name in [
            "Tasks.md",
            "Task-A01.txt",
            "TASK_REORDERING.md",
            "Task-notes.md",
        ] {
            assert!(!is_card(name), "{name}");
        }
    }

    #[test]
    fn convert_request_carries_findings_paths_and_the_section() {
        let dir = tmp("request");
        markdown_board(&dir);
        let found = find_task_system(&dir);
        let seabag = dir.join(DEFAULT_DIR);
        for language in ["en", "zh-CN", "ja"] {
            for exists in [false, true] {
                let r = agent_convert_request_in(language, &dir, &seabag, &found, exists);
                assert!(r.contains(&dir.display().to_string()), "{language}");
                assert!(r.contains(&format!("fh --seabag \"{}\"", seabag.display())));
                assert!(r.contains("- app/docs/plan/TASK_BOARD.md"));
                assert!(r.contains("3 × Task-*.md: app/docs/plan/areas, app/docs/plan/old"));
                assert!(r.contains("- AGENTS.md"));
                assert!(r.contains("--labels"));
                assert!(r.contains("](../../../../"));
                assert!(r.contains("docs/fliphopper-conversion.md"));
                assert!(r.contains(&format!("v{CONVERT_REQUEST_VERSION}")));
                // The section stays English in every language, and comes last.
                assert!(r.ends_with(&agent_section()), "{language}");
            }
        }
        let en = agent_convert_request_in("en", &dir, &seabag, &found, false);
        assert!(en.contains("Look first, change nothing."));
        assert!(en.contains("does not exist yet"));
        let ja = agent_convert_request_in("ja", &dir, &seabag, &found, true);
        assert!(ja.contains("まず見るだけ") && ja.contains("シーバッグはすでにあります"));
        let zh = agent_convert_request_in("zh-CN", &dir, &seabag, &found, false);
        assert!(zh.contains("先查看，不要改动") && zh.contains("水手袋还不存在"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
