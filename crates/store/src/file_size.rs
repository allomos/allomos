//! The file-size rule (Task-A03-tide388, Task-A03-dapper008): a source file over the project's
//! line limit may only get shorter, task by task. Each project checks it in its own tests,
//! following `alm guide file-size` ([`file_size_guide`]); `alm claim` repeats the rule
//! ([`claimed_task_steps`]). alm gives the guide and the reminder, and shows the limit
//! (`alm file-size-settings`); it measures no file and stops nothing (approver 2026-11-16).
//!
//! Written in every language the Allomos app speaks, like `agent-files`: the person may read along.

const FILE_SIZE_EN: &str = "# File-size check: files over the line limit may only get shorter

For the agent: set this check up once for each project, in the project's own tests, so it runs
with them every time. alm only gives this guide and a reminder at `alm claim`: it measures no
file and stops nothing. The check belongs to the project, written in its own language and test
tool.

## The rule

- The line limit is a project setting: `alm file-size-settings` shows it (1000 lines by default;
  the person changes it there or in the Allomos app's project settings). Tests do not count.
- A source file over the limit must not be longer after a task than before it. How is your call:
  put the new things somewhere else, or move unrelated old things out of it.
- A new source file must not be over the limit.

## Setting it up

1. Look first: if the project notes (the taskbag's `project-notes.md`) have a
   `## File size check` section, or the tests already hold such a check, there is nothing to do.
2. Write the check as one test in the project's test suite, in its own language and test tool
   (a Rust test, a Vitest or Jest test, a pytest test, a script the test command runs…). It:
   - reads the limit from the project's setting: `alm file-size-settings --json`
     (`limit_lines`), or the taskbag's `project.json` (`file_size.limit_lines`; missing means
     1000). Where the tests also run without the taskbag (in CI, for example), write the limit
     at the top of the baseline file too, and change it there when the setting changes;
   - counts the lines of each source file. Say in the check which files are source and which
     are tests, by the project's own conventions (test folders, `*.test.*`, a test module at the
     end of a file…); generated files, lock files, vendored code and data files are not source;
   - compares with the baseline: a file in it may not be longer than its number there; a file not
     in it may not be over the limit;
   - fails with a message naming the file, its lines, the number allowed, and this guide.
3. Record the baseline: every source file over the limit now, with its number of lines. Its format
   and place are the project's choice; for example a text file in the repository with one
   `<path> <lines>` per line.
4. Run it with the project's tests: it must pass on the code as it is.
5. Record one line in the project notes, under `## File size check`: where the check is and how
   to run it. Save and share the check, the baseline and the project notes with your version
   control tool.

## Keeping the baseline true

- A file got shorter: lower its number in the baseline in the same commit; once it is under the
  limit, remove its line.
- A file truly has to get longer: raise its number in the same commit, and record why with
  `alm ai-decided` (below). Never raise a number only to make the check pass.

## Recording each choice

Each time the rule leads to a choice, record it, so it can be checked later: when a task's plan
reaches a file over the limit, split or not, and when a baseline number is raised:

    alm ai-decided <id> \"file-size: <file>: <lines before> → <lines after> (tests not counted); <what you did: what moved where, or where the new things went>; <if nothing was split, why>\"

Begin it with `file-size:` in every language. To find them later, search the taskbag for
`file-size:` (in Git: `git -C <taskbag> grep -n \"file-size:\"`), or read the AI decisions in
`alm human` and `alm show <id>`.
";

const FILE_SIZE_ZH_CN: &str = "# 文件大小检查：超过行数上限的文件只准变小

给智能体：每个项目在它自己的测试里搭一次这个检查，以后每次跑测试都跟着跑。alm 只给这份指南，并在
`alm claim` 时提醒：它不测量任何文件，也不拦任何事。检查属于项目自己，用项目自己的语言和测试工具写。

## 规则

- 行数上限是项目设置：`alm file-size-settings` 显示它（默认 1000 行；人在那里或在 Allomos 应用的
  项目设置里修改）。测试不算。
- 超过上限的源文件，每个任务之后都不能比原来长。怎么做由你判断：可以把新东西放到别处，也可以把不相干的
  旧东西搬走。
- 新的源文件不能超过上限。

## 搭建

1. 先看一下：如果项目说明（任务包的 `project-notes.md`）里已经有 `## File size check` 一节，或测试里
   已经有这样的检查，就不用做。
2. 把检查写成项目测试里的一个测试，用项目自己的语言和测试工具（Rust 测试、Vitest 或 Jest 测试、pytest
   测试、测试命令会跑的脚本……）。它：
   - 从项目设置读上限：`alm file-size-settings --json`（`limit_lines`），或任务包的 `project.json`
     （`file_size.limit_lines`；没有就是 1000）。测试也会在没有任务包的地方跑时（例如 CI），把上限也写在
     基线文件开头，设置改了就在那里一起改；
   - 数每个源文件的行数。在检查里写明哪些是源文件、哪些是测试，按项目自己的约定（测试文件夹、
     `*.test.*`、文件末尾的测试模块……）；生成的文件、锁文件、引入的第三方代码和数据文件不算源文件；
   - 和基线比：基线里的文件不能比它在基线里的行数长；不在基线里的文件不能超过上限；
   - 不通过时，说明是哪个文件、多少行、允许多少行，并指向这份指南。
3. 记一份基线：现在每个超过上限的源文件和它的行数。格式和位置由项目自己定；例如仓库里的一个文本文件，
   每行一条 `<路径> <行数>`。
4. 随项目的测试跑一遍：在现在的代码上必须通过。
5. 在项目说明的 `## File size check` 一节下记一行：检查在哪、怎么跑。用你的版本控制工具保存并分享检查、
   基线和项目说明。

## 让基线保持真实

- 文件变短了：在同一个提交里把它在基线里的行数往下调；降到上限以下就删掉那一行。
- 文件确实需要变长：在同一个提交里调高它的行数，并用 `alm ai-decided` 写明原因（见下）。不要只为了让
  检查通过而调高。

## 每次判断都记下来

每次照这条规则做了判断都要留下记录，便于以后检查：任务的计划碰到超限文件时，不管拆没拆；调高基线时也
一样：

    alm ai-decided <id> \"file-size: <文件>: <改前行数> → <改后行数>（不含测试）；<做了什么：把哪部分搬到哪里，或把新东西放在哪里>；<没拆的话，为什么>\"

开头固定写 `file-size:`（各语言都用这个词）。以后要查，就在任务包里搜 `file-size:`（用 Git 时：
`git -C <taskbag> grep -n \"file-size:\"`），或看 `alm human` 和 `alm show <id>` 里的 AI 决定。
";

const FILE_SIZE_JA: &str = "# ファイルサイズの確認：行数の上限を超えるファイルは小さくなることしか許されない

エージェントへ：プロジェクトごとに一度、プロジェクト自身のテストにこの確認を組み込みます。以後はテストの
たびに一緒に走ります。alm はこのガイドと `alm claim` での知らせを出すだけで、ファイルを測ることも、何かを
止めることもしません。確認はプロジェクトのもので、プロジェクト自身の言語とテストツールで書きます。

## ルール

- 行数の上限はプロジェクトの設定です：`alm file-size-settings` が示します（既定は 1000 行。人がそこか
  Allomos アプリのプロジェクト設定で変えます）。テストは数えません。
- 上限を超える元のファイルは、どのタスクのあとも前より長くなってはいけません。やり方はあなたが判断します：
  新しいものを別の場所に置いても、関係のない古いものを外へ移してもかまいません。
- 新しい元のファイルは上限を超えてはいけません。

## 組み込み方

1. まず確かめます：プロジェクトノート（タスクバッグの `project-notes.md`）に `## File size check` の節が
   あるか、テストにすでにそのような確認があれば、することはありません。
2. 確認をプロジェクトのテストの一つとして、プロジェクト自身の言語とテストツールで書きます（Rust の
   テスト、Vitest や Jest のテスト、pytest のテスト、テストのコマンドが走らせるスクリプト……）。確認は：
   - 上限をプロジェクトの設定から読みます：`alm file-size-settings --json`（`limit_lines`）、または
     タスクバッグの `project.json`（`file_size.limit_lines`。なければ 1000）。タスクバッグのない場所
     （たとえば CI）でもテストが走るなら、上限をベースラインのファイルの先頭にも書き、設定が変わったら
     そこも一緒に変えます；
   - 元のファイルごとに行数を数えます。どれが元のファイルでどれがテストかを、プロジェクト自身の決まり
     （テストのフォルダー、`*.test.*`、ファイル末尾のテストモジュール……）で確認の中に書きます。生成された
     ファイル、ロックファイル、取り込んだ外部のコード、データのファイルは元のファイルではありません；
   - ベースラインと比べます：ベースラインにあるファイルはそこに書かれた行数より長くなってはいけません。
     ないファイルは上限を超えてはいけません；
   - 通らないときは、ファイル、その行数、許される行数を示し、このガイドを指します。
3. ベースラインを記録します：いま上限を超えているすべての元のファイルとその行数です。形式と置き場所は
   プロジェクトが決めます。たとえばリポジトリの中のテキストファイルに、一行に一つ `<パス> <行数>`。
4. プロジェクトのテストと一緒に走らせます：いまのコードで通らなければなりません。
5. プロジェクトノートの `## File size check` の節に一行記録します：確認がどこにあり、どう走らせるか。
   確認、ベースライン、プロジェクトノートをバージョン管理ツールで保存して共有します。

## ベースラインを正しく保つ

- ファイルが短くなった：同じコミットでベースラインの行数を下げます。上限を下回ったらその行を消します。
- ファイルがどうしても長くなる：同じコミットで行数を上げ、理由を `alm ai-decided`（下）で記録します。
  確認を通すためだけに上げてはいけません。

## 判断のたびに記録する

このルールで判断をしたら、あとで確かめられるよう毎回記録します：タスクの計画が上限を超えるファイルに
及んだとき（分けても分けなくても）、そしてベースラインの行数を上げたときです：

    alm ai-decided <id> \"file-size: <ファイル>: <前の行数> → <後の行数>（テストを除く）；<したこと：どの部分をどこへ移したか、または新しいものをどこに置いたか>；<分けなかったなら、その理由>\"

どの言語でも先頭は `file-size:` と書きます。あとで探すには、タスクバッグで `file-size:` を検索します
（Git なら：`git -C <taskbag> grep -n \"file-size:\"`）。または `alm human` と `alm show <id>` の
AI の判断を読みます。
";

/// `alm guide file-size` in `language` (`en`, `zh-CN`, `ja`), or `None` when it is not written in
/// that language (the agent then translates the English text faithfully).
pub fn file_size_guide(language: &str) -> Option<&'static str> {
    match crate::guide::written_in(language)? {
        "ja" => Some(FILE_SIZE_JA),
        "zh-CN" => Some(FILE_SIZE_ZH_CN),
        _ => Some(FILE_SIZE_EN),
    }
}

/// `alm guide file-size` in English, as `alm guide` prints every topic.
pub(crate) const FILE_SIZE_GUIDE_EN: &str = FILE_SIZE_EN;

/// What `alm claim` prints after its Git commands (Task-A03-tide388, approver 2026-10-22; the
/// project's limit and the record, Task-A03-dapper008), in the task language (`en`, `zh-CN`,
/// `ja`; English otherwise), with the project's line limit (`alm file-size-settings`): a source
/// file over it may not get longer, each choice is recorded as `file-size:`, and a project
/// without the check sets it up once. alm reminds; it measures nothing and stops nothing.
pub fn claimed_task_steps(language: &str, limit: u32) -> String {
    match language {
        "zh-CN" => format!("做计划之前（alm 提醒，你来检查）：看一下计划要改的源文件有多大。这个项目的行数上限是 {limit} 行（测试不算；`alm file-size-settings`）。超过上限的文件，每个任务之后都不能比原来长：可以把新东西放到别处，也可以把不相干的旧东西搬走，由你判断；项目自己的测试会检查（`alm guide file-size`）。只拆本任务要改的那部分，不重构整个文件；重构单独一个提交，放在功能提交之前，提交后测试通过、行为不变。重构比功能本身还大，或要动本任务范围外的文件时，先另开一个任务做重构，再用 `alm block <本任务> --by <新任务>` 连上。计划碰到超限文件时，不管拆没拆，都记一条：`alm ai-decided <本任务> \"file-size: <文件>: <改前行数> → <改后行数>（不含测试）；<做了什么>；<没拆的话，为什么>\"`。项目说明里还没有 `## File size check` 一节的话，这个项目还没搭检查：照 `alm guide file-size` 搭一次（或为它另开一个任务）。
"),
        "ja" => format!("計画を立てる前に（alm は知らせるだけ、確かめるのはあなた）：計画で変える元のファイルの大きさを見ます。このプロジェクトの行数の上限は {limit} 行です（テストは数えません。`alm file-size-settings`）。上限を超えるファイルは、どのタスクのあとも前より長くなってはいけません。新しいものを別の場所に置いても、関係のない古いものを外へ移してもよく、あなたが判断します。プロジェクト自身のテストが確かめます（`alm guide file-size`）。このタスクで変える部分だけを切り出し、ファイル全体は書き直しません。リファクタリングは機能のコミットより前の独立したコミットにし、その後もテストが通り、動きが変わらないこと。リファクタリングが機能そのものより大きいとき、またはこのタスクの範囲外のファイルに及ぶときは、先にリファクタリングのタスクを別に追加し、`alm block <このタスク> --by <新しいタスク>` でつなぎます。計画が上限を超えるファイルに及んだら、分けても分けなくても記録します：`alm ai-decided <このタスク> \"file-size: <ファイル>: <前の行数> → <後の行数>（テストを除く）；<したこと>；<分けなかったなら、その理由>\"`。プロジェクトノートにまだ `## File size check` の節がなければ、このプロジェクトには確認がまだありません：`alm guide file-size` のとおり一度組み込みます（またはそのためのタスクを別に追加します）。
"),
        _ => format!("Before you plan this task (alm reminds; you check): look at the size of the source files your plan will change. This project's line limit is {limit} lines (tests do not count; `alm file-size-settings`). A file over it must not be longer after this task than before it: put the new things somewhere else, or move unrelated old things out of it, as you judge; the project's own tests check it (`alm guide file-size`). Split out only the part this task changes, never the whole file, as a commit of its own before the feature commit, with the tests passing and the behaviour unchanged after it. When the refactoring is bigger than the feature itself, or reaches files outside this task, add a task for it first and link them with `alm block <this task> --by <new task>`. When your plan reaches a file over the limit, split or not, record it: `alm ai-decided <this task> \"file-size: <file>: <lines before> → <lines after> (tests not counted); <what you did>; <if nothing was split, why>\"`. If the project notes have no `## File size check` section yet, the project has no check: set it up once, as `alm guide file-size` says (or add a task for it).
"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Task-A03-tide388 and Task-A03-dapper008: the claim reminder, in every language the app
    /// speaks, with the project's limit and no limit kept in the project notes any more.
    #[test]
    fn the_claim_reminder_names_the_limit_and_the_record_in_every_language() {
        for (language, words) in [
            (
                "en",
                [
                    "1200 lines",
                    "must not be longer after this task than before it",
                    "a commit of its own before the feature commit",
                    "`alm block <this task> --by <new task>`",
                    "## File size check",
                ],
            ),
            (
                "zh-CN",
                [
                    "1200 行",
                    "每个任务之后都不能比原来长",
                    "放在功能提交之前",
                    "`alm block <本任务> --by <新任务>`",
                    "## File size check",
                ],
            ),
            (
                "ja",
                [
                    "1200 行",
                    "前より長くなってはいけません",
                    "機能のコミットより前",
                    "`alm block <このタスク> --by <新しいタスク>`",
                    "## File size check",
                ],
            ),
        ] {
            let text = claimed_task_steps(language, 1200);
            for w in words {
                assert!(text.contains(w), "{language}: {w}");
            }
            for w in [
                "\"file-size: ",
                "alm file-size-settings",
                "alm guide file-size",
            ] {
                assert!(text.contains(w), "{language}: {w}");
            }
            // The limit is the project setting now, not a line in the project notes.
            assert!(!text.contains("project-notes.md"), "{language}");
            assert!(!text.contains("1000"), "{language}");
        }
        assert_eq!(claimed_task_steps("fr", 900), claimed_task_steps("en", 900));
    }

    #[test]
    fn the_guide_is_written_in_every_language_with_the_setting_baseline_and_record() {
        for language in ["en", "zh-CN", "ja"] {
            let text = file_size_guide(language).unwrap();
            assert!(text.starts_with("# "), "{language}");
            for w in [
                "alm file-size-settings",
                "file_size.limit_lines",
                "1000",
                "## File size check",
                "alm ai-decided <id> \"file-size: ",
                "grep -n \"file-size:\"",
                "project-notes.md",
            ] {
                assert!(text.contains(w), "{language}: {w}");
            }
            // No hard ban on adding code (approver 2026-11-16).
            assert!(!text.contains("no new code"), "{language}");
        }
        assert_eq!(file_size_guide("ja-JP"), file_size_guide("ja"));
        assert_eq!(file_size_guide("zh-Hans"), file_size_guide("zh-CN"));
        assert_eq!(file_size_guide("de"), None);
        assert_eq!(FILE_SIZE_GUIDE_EN, file_size_guide("en").unwrap());
    }
}
