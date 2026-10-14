//! Is this project up to date with this Allomos (Task-A01-25)? Two things carry a version: the
//! taskbag's format (`"format": "allomos/<n>"` in `project.json`) and Allomos's section in the
//! project's `AGENTS.md` / `CLAUDE.md` (its heading, `## Tasks: Allomos (v<n>)`).
//!
//! Allomos only *checks*. It never rewrites the section or the taskbag by itself. An older taskbag
//! is updated by an AI agent following one guided prompt, joined from the migration entries
//! between the taskbag's version and this Allomos's ([`MIGRATIONS`]); the agent changes the
//! taskbag only through `alm`, and `alm migrate --finish` writes the new version only when
//! `alm list --json` reports no issues (owner, 2026-10-14; Task-A01-32).

use std::fs;
use std::path::Path;

use allomos_core::{FORMAT, FORMAT_VERSION, OLDEST_FORMAT_VERSION, Problem, format_version};
use serde::Serialize;
use serde_json::Value;

use crate::convert::{Source, ask_once_in, migration_header_in};
use crate::shown;
use crate::{AGENT_SECTION_HEADING, Actor, Lock, PROJECT_FILE, picked_folder, tells_agents};
use allomos_core::event::{Change, Kind};

/// The version of Allomos's section that this Allomos writes. Raise it whenever the section's
/// wording changes, so projects with the older wording are asked to renew it.
///
/// 2: the handoff lines (Task-A03-06).
/// 3: the rule for finishing a task, and `alm next` listing every free ready task (Task-A03-15).
/// 4: `waiting` (on a person) and `blocked` (by another task), seven states (Task-A01-31); the
/// human check: `--without-human-check` with its steps, `alm human`, `alm checked` (Task-A03-18).
/// 5: no `--push`; `alm` never runs Git and prints the Git commands for each change, and a claim
/// is pushed before the work starts (decision 0017, Task-A01-38).
pub const SECTION_VERSION: u32 = 5;

/// How long `alm migrate --finish` waits for another writer.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The taskbag's format compared with this Allomos's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FormatState {
    /// The format this Allomos writes.
    Current,
    /// Written by an older Allomos that this one updates: an agent follows `alm migrate`.
    Older { found: String },
    /// Older than the oldest format this Allomos updates (`taskbag_too_old`).
    TooOld { found: String, oldest: String },
    /// Written by a newer Allomos: the person chooses (update Allomos, or let the agent go on).
    Newer { found: String },
    /// No `project.json`, or no `allomos/<n>` format in it: the usual checks report it.
    Unknown,
}

/// Allomos's section in one instruction file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SectionState {
    /// `AGENTS.md` or `CLAUDE.md`.
    pub file: String,
    /// The version in the section's heading; `None` when the heading has none (an older section,
    /// or one an agent merged in its own words).
    pub version: Option<u32>,
}

impl SectionState {
    /// Older than this Allomos's section, or without a version.
    pub fn outdated(&self) -> bool {
        self.version.is_none_or(|v| v < SECTION_VERSION)
    }
}

/// Everything the check found. Made only once an agent is connected with the project: the section
/// is in an instruction file and the project folder is known (owner, 2026-10-12). Before that,
/// connecting the agents writes the current section anyway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionCheck {
    /// What this Allomos writes, for messages: `allomos/1`.
    pub format_current: String,
    pub section_current: u32,
    pub format: FormatState,
    /// The instruction files that hold Allomos's section (a `CLAUDE.md` that only imports
    /// `@AGENTS.md` is not listed: it follows `AGENTS.md`).
    pub sections: Vec<SectionState>,
}

impl VersionCheck {
    /// The instruction files whose section is out of date.
    pub fn outdated_files(&self) -> Vec<String> {
        self.sections
            .iter()
            .filter(|s| s.outdated())
            .map(|s| s.file.clone())
            .collect()
    }

    /// Whether the running agent has something to update (the taskbag or the section).
    pub fn needs_update(&self) -> bool {
        matches!(self.format, FormatState::Older { .. }) || !self.outdated_files().is_empty()
    }

    /// The warning `alm` adds when the section is out of date.
    pub fn instructions_warning(&self) -> Option<Problem> {
        let files = self.outdated_files();
        (!files.is_empty()).then(|| Problem::InstructionsOutdated {
            files: files.join(" and "),
            current: self.section_current.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Migration entries (Task-A01-32)
// ---------------------------------------------------------------------------------------------

/// One text in the interface's three languages. The prompt follows the app's language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Words {
    pub en: &'static str,
    pub zh_cn: &'static str,
    pub ja: &'static str,
}

impl Words {
    /// The text in `language` (`en`, `zh-CN`, `ja`); English otherwise.
    pub fn get(&self, language: &str) -> &'static str {
        match language {
            "zh-CN" => self.zh_cn,
            "ja" => self.ja,
            _ => self.en,
        }
    }
}

/// What an agent needs to bring a taskbag from format `from` to `from + 1`. Adding a format
/// version is adding one of these to [`MIGRATIONS`]: data only, no code that rewrites files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub from: u32,
    /// What changed in the format, in plain words.
    pub changed: Words,
    /// What to add to every taskbag (folders, files, fields), and with which `alm` command.
    pub add: Words,
    /// What to decide task by task, with the rule for deciding.
    pub decide: Words,
    /// The `alm` commands the update uses, as `alm` names them (`rename label`). They, and no other
    /// command that changes the taskbag, work while the update is unfinished.
    pub commands: &'static [&'static str],
    /// How to check that this step is complete, besides `alm list --json` reporting no issues.
    pub check: Words,
}

/// The migration entries this Allomos knows, oldest first: one per format version from
/// [`OLDEST_FORMAT_VERSION`] up to [`FORMAT_VERSION`].
///
/// 3 → 4 holds two parts, each text starting with its card: A01-31's `blocked` state (decision
/// 0016), and A03-18's human check (the note's new name and steps, the top label "human
/// verified"). One entry, not 4 and 5 (owner, 2026-10-14); the rename is done by the agent with
/// `alm`, never by a code step, and a customised ladder is shown to the person, never guessed.
pub const MIGRATIONS: &[Migration] = &[MIGRATION_3_TO_4];

/// allomos/3 → allomos/4. Data only: what changed, what to add, what to decide task by task.
const MIGRATION_3_TO_4: Migration = Migration {
    from: 3,
    changed: Words {
        en: "[A01-31, the blocked state] A task that cannot go on is now either `waiting` (a person or an outside event must act: `waiting_on` says what, the new optional `who` says who; without `who` the approver is meant) or the new state `blocked` (another task in this taskbag must finish first: the new field `blocked_by` lists those task IDs). A task that is blocked and also waits on a person is `blocked`; the person part goes in its next dependency. Allomos now has seven states.\n  [A03-18, the human check] The word \"owner\" is split by meaning: a person trying the finished task is the **human check**, and the person who decides and approves is the **approver**. A task's `finished_without_owner` is now `without_human_check` (why it was finished without the human check, and in which run), with the new optional `human_check_steps` (Markdown: how a person checks it); `alm done --without-human-check \"<why>\" --steps <file.md>` writes them (`--without-owner` still works). `alm human` lists the tasks to check and to answer; `alm checked <id>` (or the top label on a done task) marks one checked and clears both, which its history keeps. A new taskbag's top label is \"human verified\" instead of \"owner verified\"; an existing ladder is the project's own and changes only with the person's answer.",
        zh_cn: "[A01-31，阻塞状态] 无法继续的任务现在分两种：`waiting`（等待某人或外部事件：`waiting_on` 写明等什么，新的可选字段 `who` 写明等谁；没有 `who` 就是在等审批人），或新状态 `blocked`（必须先完成本任务包里的其他任务：新字段 `blocked_by` 列出这些任务 ID）。既被阻塞又在等人的任务是 `blocked`，等人的部分写进它的下一个依赖。Allomos 现在有七个状态。\n  [A03-18，人工检查] \"owner\" 一词按含义拆开：由人亲手试用已完成的任务叫**人工检查**，做决定和审批的人叫**审批人**。任务的 `finished_without_owner` 现在叫 `without_human_check`（为什么没有人工检查就完成、在哪次运行中），另有新的可选字段 `human_check_steps`（Markdown：人怎样检查它）；用 `alm done --without-human-check \"<原因>\" --steps <file.md>` 写入（`--without-owner` 仍然可用）。`alm human` 列出待检查和待回答的任务；`alm checked <id>`（或给已完成的任务设最高标签）把它标为已检查并清除这两项，历史记录会保留。新任务包的最高标签是 \"human verified\"，不再是 \"owner verified\"；已有的标签阶梯属于项目自己，只有用户回答后才改。",
        ja: "[A01-31、ブロック状態] 先に進めないタスクは、`waiting`（人か外部の出来事を待つ：`waiting_on` に何を、新しい任意の `who` に誰を書く。`who` がなければ承認者を待つ）か、新しい状態 `blocked`（このタスクバッグのほかのタスクが先に終わる必要がある：新しいフィールド `blocked_by` にそのタスク ID を並べる）のどちらかになりました。ブロックされていて人も待つタスクは `blocked` で、人の部分は次の依存に書きます。Allomos の状態は七つになりました。\n  [A03-18、人による確認] \"owner\" という語を意味で分けました：終わったタスクを人が実際に試すことを**人による確認**、決めて承認する人を**承認者**と呼びます。タスクの `finished_without_owner` は `without_human_check`（人による確認なしで終えた理由と、どの実行か）になり、新しい任意の `human_check_steps`（Markdown：人がどう確認するか）が加わりました。`alm done --without-human-check \"<理由>\" --steps <file.md>` で書きます（`--without-owner` も使えます）。`alm human` は確認待ちと回答待ちのタスクを挙げ、`alm checked <id>`（または終わったタスクに最上位のラベル）で確認済みにし、両方を消します（履歴には残ります）。新しいタスクバッグの最上位のラベルは \"owner verified\" ではなく \"human verified\" です。既存のラダーはプロジェクトのもので、相手の答えがあるときだけ変えます。",
    },
    add: Words {
        en: "[A01-31] Nothing is required: `who` and `blocked_by` are optional, and no task moves by itself.\n  [A03-18] Nothing is required: `human_check_steps` is optional.",
        zh_cn: "[A01-31] 不需要添加任何东西：`who` 和 `blocked_by` 都是可选的，没有任务会自己改变状态。\n  [A03-18] 不需要添加任何东西：`human_check_steps` 是可选的。",
        ja: "[A01-31] 必須の追加はありません：`who` と `blocked_by` は任意で、どのタスクも自分では動きません。\n  [A03-18] 必須の追加はありません：`human_check_steps` は任意です。",
    },
    decide: Words {
        en: "[A01-31] For each `waiting` task: it stays `waiting` unless its waiting-on or next dependency shows that another task in this taskbag must finish first. Only then, and only with the person's answer to your plan, move it with `{alm} block <id> --by <task-id> [<task-id>…] --as <you>`; if it also waits on a person, keep that part with `{alm} needs <id> \"<what, from whom>\" --as <you>`. A task that waits on someone other than the approver may get `--who`: `{alm} wait <id> --on \"<what>\" --who \"<who>\" --as <you>`. When you cannot tell, leave the task `waiting` and list it under what you could not decide.\n  [A03-18] (1) The ladder (`labels` in the taskbag's `project.json`): show the person the whole ladder as it is. If its top step is \"owner verified\", propose `{alm} rename label \"owner verified\" \"human verified\" --as <you>` (it renames the label on every task that carries it too). If the ladder is customised in any other way, ask which step, if any, is the human check and what it should be called; never guess, and rename nothing without the answer. (2) Each task that still has `finished_without_owner` (`{alm} list --json` reports it as `finished_without_owner_renamed`): if its label is below the top step, no person has checked it yet: move the note unchanged with `{alm} done <id> --without-human-check \"<the same note>\" --as <you>`, adding `--steps <file.md>` only when the task's card or description says how a person checks it (otherwise list it under what is left for the person). If its label is already the top step, a person has checked it: `{alm} checked <id> --as <you>` clears the old note, which its history keeps.",
        zh_cn: "[A01-31] 对每个 `waiting` 任务：保持 `waiting`，除非它的等待内容或下一个依赖表明必须先完成本任务包里的另一个任务。只有这时，并且用户同意了你的计划，才用 `{alm} block <id> --by <任务 ID> [<任务 ID>…] --as <你>` 改为阻塞；如果它还在等人，用 `{alm} needs <id> \"<等什么、等谁>\" --as <你>` 保留这部分。等的不是审批人的任务可以加 `--who`：`{alm} wait <id> --on \"<什么>\" --who \"<谁>\" --as <你>`。无法判断时，保持 `waiting`，并列入你无法决定的事项。\n  [A03-18] (1) 标签阶梯（任务包 `project.json` 里的 `labels`）：把整个阶梯原样给用户看。如果最高一级是 \"owner verified\"，建议用 `{alm} rename label \"owner verified\" \"human verified\" --as <你>`（带这个标签的任务也会一起改名）。如果阶梯有其他自定义，问用户哪一级（如果有）是人工检查、应该叫什么；不要猜，没有回答就什么都不改名。(2) 每个仍带有 `finished_without_owner` 的任务（`{alm} list --json` 报告为 `finished_without_owner_renamed`）：如果它的标签低于最高一级，说明还没有人检查过：用 `{alm} done <id> --without-human-check \"<原样的说明>\" --as <你>` 原样移过去；只有当任务的卡片或描述写明了人怎样检查时才加 `--steps <file.md>`（否则把它列入留给用户的事项）。如果它的标签已经是最高一级，说明有人检查过了：用 `{alm} checked <id> --as <你>` 清除旧说明，历史记录会保留它。",
        ja: "[A01-31] `waiting` のタスクごとに：待っている内容か次の依存から、このタスクバッグのほかのタスクが先に終わる必要があるとわかる場合を除き、`waiting` のままにします。その場合だけ、しかも計画への相手の答えを得てから、`{alm} block <id> --by <タスク ID> [<タスク ID>…] --as <あなた>` で移します。人も待っているなら、その部分は `{alm} needs <id> \"<何を、誰から>\" --as <あなた>` で残します。承認者以外を待つタスクには `--who` を付けられます：`{alm} wait <id> --on \"<何>\" --who \"<誰>\" --as <あなた>`。判断できないときは `waiting` のままにし、決められなかったことに挙げます。\n  [A03-18] (1) ラダー（タスクバッグの `project.json` の `labels`）：ラダー全体をそのまま相手に見せます。最上位が \"owner verified\" なら、`{alm} rename label \"owner verified\" \"human verified\" --as <あなた>` を提案します（そのラベルを持つタスクも一緒に変わります）。ほかに独自の形になっているなら、どの段が（あれば）人による確認で、何と呼ぶかを尋ねます。推測せず、答えがなければ何も名前を変えません。(2) まだ `finished_without_owner` を持つタスクごとに（`{alm} list --json` が `finished_without_owner_renamed` と報告します）：ラベルが最上位より下なら、まだ誰も確認していません。`{alm} done <id> --without-human-check \"<同じメモ>\" --as <あなた>` でメモをそのまま移し、タスクのカードか説明に人による確認のしかたが書いてあるときだけ `--steps <file.md>` を付けます（なければ相手に残すことに挙げます）。ラベルがすでに最上位なら、人が確認済みです：`{alm} checked <id> --as <あなた>` で古いメモを消します（履歴には残ります）。",
    },
    commands: &["block", "wait", "needs", "rename label", "done", "checked"],
    check: Words {
        en: "[A01-31] `{alm} list --json` reports no issues: every `blocked` task names at least one other task of this taskbag in `blocked_by`. `{alm} list --state blocked` shows each task you moved, and `{alm} list --state waiting` the rest.\n  [A03-18] `{alm} list --json` reports no `finished_without_owner_renamed`; `{alm} human` lists under To check every task whose note you moved; the ladder's top step is the one the person chose.",
        zh_cn: "[A01-31] `{alm} list --json` 没有报告问题：每个 `blocked` 任务的 `blocked_by` 至少列出本任务包里的另一个任务。`{alm} list --state blocked` 显示你改过的每个任务，`{alm} list --state waiting` 显示其余的。\n  [A03-18] `{alm} list --json` 没有报告 `finished_without_owner_renamed`；`{alm} human` 的待检查里列出你移过说明的每个任务；阶梯的最高一级是用户选定的那个。",
        ja: "[A01-31] `{alm} list --json` が問題を報告しないこと：`blocked` のタスクはどれも `blocked_by` にこのタスクバッグのほかのタスクを一つ以上挙げている。`{alm} list --state blocked` に移したタスクが、`{alm} list --state waiting` に残りが出ること。\n  [A03-18] `{alm} list --json` が `finished_without_owner_renamed` を報告しないこと。`{alm} human` の確認待ちに、メモを移したタスクがすべて出ること。ラダーの最上位が相手の選んだ段であること。",
    },
};

/// Commands that change the taskbag but always work while an update is unfinished: the update's
/// own, and writing a handoff (a session may have to stop in the middle of an update).
const ALWAYS_ALLOWED: &[&str] = &["migrate", "handoff"];

/// The formats this Allomos reads and the entries between them. [`FORMATS`] is the real one;
/// tests make their own with a test-only entry.
#[derive(Debug, Clone, Copy)]
pub struct Formats {
    pub entries: &'static [Migration],
    pub current: u32,
    pub oldest: u32,
}

/// The formats and entries of this Allomos.
pub const FORMATS: Formats = Formats {
    entries: MIGRATIONS,
    current: FORMAT_VERSION,
    oldest: OLDEST_FORMAT_VERSION,
};

impl Formats {
    /// What this Allomos writes: `allomos/<current>`.
    pub fn current_format(&self) -> String {
        format!("allomos/{}", self.current)
    }

    /// The taskbag's format at `root` compared with these.
    pub fn state(&self, root: &Path) -> FormatState {
        let Some(found) = read_format(root) else {
            return FormatState::Unknown;
        };
        match format_version(&found) {
            Some(n) if n == self.current => FormatState::Current,
            Some(n) if n > self.current => FormatState::Newer { found },
            Some(n) if n < self.oldest => FormatState::TooOld {
                found,
                oldest: format!("allomos/{}", self.oldest),
            },
            Some(_) => FormatState::Older { found },
            None => FormatState::Unknown,
        }
    }

    /// The entries from version `found` to the current one, in order; `None` when one is
    /// missing (then nothing can be finished).
    pub fn pending(&self, found: u32) -> Option<Vec<&'static Migration>> {
        (found..self.current)
            .map(|v| self.entries.iter().find(|m| m.from == v))
            .collect()
    }

    /// Whether `command` (as `alm` names it) may change a taskbag that is being updated from
    /// version `found`.
    pub fn allows(&self, found: u32, command: &str) -> bool {
        ALWAYS_ALLOWED.contains(&command)
            || self
                .pending(found)
                .unwrap_or_default()
                .iter()
                .any(|m| m.commands.contains(&command))
    }

    /// The commands an unfinished update allows, for messages: the entries' own, then
    /// `migrate` and `handoff`.
    pub fn allowed_commands(&self, found: u32) -> Vec<&'static str> {
        let mut all: Vec<&'static str> = Vec::new();
        for m in self.pending(found).unwrap_or_default() {
            for c in m.commands {
                if !all.contains(c) {
                    all.push(c);
                }
            }
        }
        all.extend(ALWAYS_ALLOWED);
        all
    }
}

/// The taskbag's format, read from `project.json` without the usual checks (a newer taskbag may
/// have fields this Allomos does not know).
pub fn format_state(root: &Path) -> FormatState {
    FORMATS.state(root)
}

fn read_format(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join(PROJECT_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.get("format")?.as_str().map(String::from)
}

/// What is left to fix in the taskbag at `root`, in one line for messages: the issues
/// `alm list --json` reports (at most ten, then how many more).
pub fn left_to_fix(root: &Path) -> String {
    let issues = crate::load(root).issues;
    if issues.is_empty() {
        return "no problems reported by `alm list --json`; make sure every step of `alm migrate` is done".to_string();
    }
    let mut parts: Vec<String> = issues
        .iter()
        .take(10)
        .map(|i| match &i.field {
            Some(field) => format!("{} ({field}): {}", i.file, i.problem.english()),
            None => format!("{}: {}", i.file, i.problem.english()),
        })
        .collect();
    if issues.len() > 10 {
        parts.push(format!(
            "and {} more (see `alm list --json`)",
            issues.len() - 10
        ));
    }
    format!("{} problem(s): {}", issues.len(), parts.join("; "))
}

/// May `command` (as `alm` names it, for example `claim` or `rename label`) change the taskbag at
/// `root`? Refused for a newer or too old taskbag, and, while an update is unfinished, for every
/// command but the update's own (owner, 2026-10-14). Used by `alm` and by the app's changes.
pub fn write_allowed(root: &Path, command: &str) -> Result<(), Problem> {
    write_allowed_in(root, command, &FORMATS)
}

pub fn write_allowed_in(root: &Path, command: &str, formats: &Formats) -> Result<(), Problem> {
    match formats.state(root) {
        FormatState::Newer { found } => Err(Problem::TaskbagNewer {
            found,
            supported: formats.current_format(),
        }),
        FormatState::TooOld { found, oldest } => Err(Problem::TaskbagTooOld { found, oldest }),
        FormatState::Older { found } => {
            let n = format_version(&found).unwrap_or_default();
            if formats.allows(n, command) {
                Ok(())
            } else {
                Err(Problem::TaskbagOlder {
                    found,
                    current: formats.current_format(),
                    left: left_to_fix(root),
                })
            }
        }
        FormatState::Current | FormatState::Unknown => Ok(()),
    }
}

/// What `alm migrate --finish` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finished {
    pub from: String,
    pub to: String,
    /// False when the taskbag was already in this Allomos's format.
    pub changed: bool,
}

/// `alm migrate --finish`: write this Allomos's format into the taskbag at `root`, as `actor`,
/// only when `alm list --json` reports no issues (`migration_unfinished`, listing them). Only the
/// `format` line of `project.json` changes, and a `migrate` event records it.
pub fn finish(root: &Path, actor: Actor) -> Result<Finished, Problem> {
    finish_in(root, actor, &FORMATS)
}

pub fn finish_in(root: &Path, actor: Actor, formats: &Formats) -> Result<Finished, Problem> {
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let to = formats.current_format();
    let found = read_format(root).unwrap_or_default();
    let unchanged = |found: String| Finished {
        from: found,
        to: to.clone(),
        changed: false,
    };
    match formats.state(root) {
        FormatState::Current => return Ok(unchanged(found)),
        FormatState::Newer { found } => {
            return Err(Problem::TaskbagNewer {
                found,
                supported: to,
            });
        }
        FormatState::TooOld { found, oldest } => {
            return Err(Problem::TaskbagTooOld { found, oldest });
        }
        FormatState::Unknown => {
            return Err(Problem::WrongFormat {
                expected: to,
                found,
            });
        }
        FormatState::Older { .. } => {}
    }
    let version = format_version(&found).unwrap_or_default();
    if formats.pending(version).is_none() {
        return Err(Problem::MigrationMissing { found, current: to });
    }
    if !crate::load(lock.root()).issues.is_empty() {
        return Err(Problem::MigrationUnfinished {
            left: left_to_fix(lock.root()),
        });
    }
    let event = actor.event(
        "migrate",
        Kind::Project,
        None,
        vec![Change {
            field: "format".to_string(),
            from: Some(found.clone().into()),
            to: Some(to.clone().into()),
        }],
    );
    let path = lock.root().join(PROJECT_FILE);
    lock.record(&event, |_| set_format(&path, formats.current))?;
    Ok(Finished {
        from: found,
        to,
        changed: true,
    })
}

/// Write `allomos/<version>` as `project.json`'s format, keeping every other byte as it is (the
/// fields' order too), so the change is one line in a diff.
fn set_format(path: &Path, version: u32) -> Result<(), Problem> {
    let failed = |detail: &str| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: detail.to_string(),
    };
    let text = crate::read_text(path).map_err(|e| failed(&e.to_string()))?;
    let key = text
        .find("\"format\"")
        .ok_or_else(|| failed("no format field"))?;
    let colon = key
        + text[key..]
            .find(':')
            .ok_or_else(|| failed("no format value"))?;
    let open = colon
        + text[colon..]
            .find('"')
            .ok_or_else(|| failed("no format value"))?;
    let close = open
        + 1
        + text[open + 1..]
            .find('"')
            .ok_or_else(|| failed("no format value"))?;
    let out = format!(
        "{}\"allomos/{version}\"{}",
        &text[..open],
        &text[close + 1..]
    );
    crate::write_atomic(path, &out)
}

// ---------------------------------------------------------------------------------------------
// Checking the section, and connecting
// ---------------------------------------------------------------------------------------------

/// The version in Allomos's section heading in `text`: `Some(Some(n))` for
/// `## Tasks: Allomos (v<n>)`, `Some(None)` when the text tells agents about Allomos without a
/// versioned heading, `None` when it does not tell them at all.
pub fn section_version(text: &str) -> Option<Option<u32>> {
    let versioned = text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(AGENT_SECTION_HEADING)?;
        let n = rest.trim().strip_prefix("(v")?.strip_suffix(')')?;
        n.parse::<u32>().ok()
    });
    match versioned {
        Some(n) => Some(Some(n)),
        None => tells_agents(text).then_some(None),
    }
}

/// Check the taskbag at `root` and the instruction files in `project` (its project folder).
pub fn check(root: &Path, project: &Path) -> VersionCheck {
    let mut sections = Vec::new();
    for file in ["AGENTS.md", "CLAUDE.md"] {
        let text = fs::read_to_string(project.join(file)).unwrap_or_default();
        // A file that imports `@AGENTS.md` follows it; it is checked only when it holds its own
        // copy of the section (a heading such as "# Allomos — Claude entry point" is not one).
        if crate::imports_agents_md(&text) && !text.contains(AGENT_SECTION_HEADING) {
            continue;
        }
        if let Some(version) = section_version(&text) {
            sections.push(SectionState {
                file: file.to_string(),
                version,
            });
        }
    }
    VersionCheck {
        format_current: FORMAT.to_string(),
        section_current: SECTION_VERSION,
        format: format_state(root),
        sections,
    }
}

/// The project folder of the taskbag at `root` for the check: the one recorded in
/// `project.json`, else the folder holding `.allomos/` when its instruction files mention
/// Allomos. `None` when no agent is connected yet, so nothing is checked.
pub fn connected_project(root: &Path) -> Option<std::path::PathBuf> {
    let text = fs::read_to_string(root.join(PROJECT_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let recorded = value
        .get("project_folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let folder = match recorded {
        Some(r) => {
            let base = picked_folder(root).unwrap_or(root);
            normalize(&crate::setup::join_recorded(base, r))
        }
        None => picked_folder(root)?.to_path_buf(),
    };
    let told = ["AGENTS.md", "CLAUDE.md"].iter().any(|f| {
        fs::read_to_string(folder.join(f))
            .map(|t| tells_agents(&t))
            .unwrap_or(false)
    });
    told.then_some(folder)
}

fn normalize(path: &Path) -> std::path::PathBuf {
    let mut clean = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    clean
}

/// [`check`] for a taskbag, only when an agent is connected ([`connected_project`]).
pub fn check_connected(root: &Path) -> Option<VersionCheck> {
    connected_project(root).map(|project| check(root, &project))
}

// ---------------------------------------------------------------------------------------------
// The prompt
// ---------------------------------------------------------------------------------------------

/// The fixed sentences of the prompt, in one language.
struct Phrases {
    older_title: &'static str,
    section_title: &'static str,
    project: &'static str,
    taskbag: &'static str,
    read_only: &'static str,
    changes: &'static str,
    changed: &'static str,
    add: &'static str,
    decide: &'static str,
    commands: &'static str,
    check: &'static str,
    steps: &'static str,
    look: &'static str,
    plan: &'static str,
    apply: &'static str,
    finish: &'static str,
    section: &'static str,
    report_older: &'static str,
    report_section: &'static str,
    commit_older: &'static str,
    commit_section: &'static str,
    none: &'static str,
}

const EN: Phrases = Phrases {
    older_title: "Please migrate this taskbag to the newest Allomos format. It was written by an older Allomos (format {found}); this Allomos writes {current}. Migrate it the way another task system is moved into Allomos: Allomos does not change the files itself; you do, only through `alm`, after the person agrees to your plan.",
    section_title: "This project was set up by an older Allomos. Please update it to this version.",
    project: "Project folder",
    taskbag: "Taskbag (Allomos's task files)",
    read_only: "Until the update is finished, `alm` only reads, except for the update's own commands: {commands}. Commands that start or change work (claim, label, done, add …) are refused until then.",
    changes: "What changed, version by version",
    changed: "What changed",
    add: "What to add",
    decide: "What to decide, task by task",
    commands: "Commands to use",
    check: "How to check",
    steps: "Steps",
    look: "Look first, and change nothing. Read the changes above, look at the taskbag with `{alm} list --json` and `{alm} show <id>`, and work out what each change means for this taskbag, task by task.",
    plan: "The one question: show the plan and wait for the person's answer. Write the whole plan in Markdown to a file outside the project: every task you would change, how, and why by the rules above, and everything you cannot decide. Then run `{alm} confirm --title \"Migrate the taskbag\" --plan <file>`. Exit code 0 means go ahead with the whole plan; any other exit code means the person cancelled or did not answer: change nothing, stop, and tell the person that nothing changed.",
    apply: "Apply the plan only through `alm`, with the commands above and `--as <your session name>`. Never edit the taskbag's files by hand. While the update is unfinished, `alm` prints no Git commands for these changes: they are committed together at the end.",
    finish: "Check: `{alm} list --json` must report no issues, and the checks above must hold. Then run `{alm} migrate --finish`. It writes the new format only when nothing is left to fix; otherwise it lists what is left: fix that and run it again.",
    section: "In the project folder, replace Allomos's section in {files} (headed \"{heading}…\", or any other text about Allomos or `alm`) with the output of `{alm} brief`, exactly as it is. Keep the rest of each file as it is.",
    report_older: "Tell the person what changed, and what you could not decide.",
    report_section: "Show the user what you changed and the commits you made.",
    commit_older: "When `alm migrate --finish` has succeeded, commit the update as one commit: only the taskbag's files{files} (`alm migrate --finish` prints the Git commands for the taskbag's files). Commit only; do not push. Leave every other change as it is.",
    commit_section: "If the project is a Git repository, commit only these changes: the taskbag's files{files}. Leave every other change as it is; do not push unless the person asks.",
    none: "(none)",
};

const ZH_CN: Phrases = Phrases {
    older_title: "请把这个任务包迁移到最新的 Allomos 格式。它是旧版 Allomos 写的（格式 {found}）；这个版本的 Allomos 写的是 {current}。像把其他任务系统搬进 Allomos 那样迁移它：Allomos 不会自己改这些文件；由你来改，只通过 `alm`，并且在用户同意你的计划之后。",
    section_title: "这个项目是用旧版的 Allomos 设置的，请把它更新到当前版本。",
    project: "项目文件夹",
    taskbag: "任务包（Allomos 的任务文件）",
    read_only: "更新完成之前，`alm` 只读，只有更新本身的命令可以改动：{commands}。开始或改变工作的命令（claim、label、done、add …）在此之前都会被拒绝。",
    changes: "各版本的改动",
    changed: "改了什么",
    add: "要添加什么",
    decide: "要逐个任务决定什么",
    commands: "要用的命令",
    check: "怎样检查",
    steps: "步骤",
    look: "先看，什么都不改。读上面的改动，用 `{alm} list --json` 和 `{alm} show <id>` 查看任务包，逐个任务想清楚每项改动对这个任务包意味着什么。",
    plan: "唯一的问题：展示计划，等用户回答。把完整计划用 Markdown 写进项目之外的文件：你要改的每个任务、怎么改、按上面的规则为什么这样改，以及你无法决定的所有事项。然后运行 `{alm} confirm --title \"迁移任务包\" --plan <文件>`。退出码 0 表示用户同意整个计划；其他退出码表示取消或没有回答：什么都不要改，然后停止，并告诉用户没有任何改动。",
    apply: "只通过 `alm` 执行计划，使用上面的命令并加上 `--as <你的会话名>`。不要手动编辑任务包的文件。更新完成之前，`alm` 不会为这些改动打印 Git 命令：它们在最后一起提交。",
    finish: "检查：`{alm} list --json` 必须没有报告任何问题，上面的检查也都要满足。然后运行 `{alm} migrate --finish`。只有没有剩下要修的东西时，它才会写入新格式；否则它会列出剩下的问题：修好后再运行一次。",
    section: "在项目文件夹中，把 {files} 里 Allomos 的一节（标题以“{heading}”开头，或其他关于 Allomos 或 `alm` 的说明）替换为 `{alm} brief` 的输出，原样（英文）使用。文件的其他内容保持不变。",
    report_older: "告诉用户改了什么，以及你无法决定的事项。",
    report_section: "告诉用户你改了什么，以及你做的提交。",
    commit_older: "`alm migrate --finish` 成功后，把这次更新作为一次提交：只提交任务包的文件{files}（`alm migrate --finish` 会打印任务包文件的 Git 命令）。只提交，不要推送；其他改动保持原样。",
    commit_section: "如果项目是 Git 仓库，只提交这些改动：任务包的文件{files}。其他改动保持原样；除非用户要求，不要推送。",
    none: "（无）",
};

const JA: Phrases = Phrases {
    older_title: "このタスクバッグを最新の Allomos の形式に移行してください。古いバージョンの Allomos で書かれています（形式 {found}）。このバージョンの Allomos が書くのは {current} です。ほかのタスク管理を Allomos に移すときと同じように移行します：Allomos はファイルを自分では変えません。変えるのはあなたで、`alm` を通してだけ、相手が計画に同意してからです。",
    section_title: "このプロジェクトは古いバージョンの Allomos で設定されました。今のバージョンに更新してください。",
    project: "プロジェクトのフォルダー",
    taskbag: "タスクバッグ（Allomos のタスクファイル）",
    read_only: "更新が終わるまで、`alm` は読むだけで、変更できるのは更新のためのコマンドだけです：{commands}。作業を始めたり変えたりするコマンド（claim、label、done、add …）はそれまで拒否されます。",
    changes: "バージョンごとの変更",
    changed: "変わったこと",
    add: "追加するもの",
    decide: "タスクごとに決めること",
    commands: "使うコマンド",
    check: "確かめ方",
    steps: "手順",
    look: "まず見るだけで、何も変えません。上の変更を読み、`{alm} list --json` と `{alm} show <id>` でタスクバッグを見て、それぞれの変更がこのタスクバッグにとって何を意味するかをタスクごとに考えます。",
    plan: "ただ一つの質問：計画を見せて、相手の答えを待ちます。計画全体を Markdown でプロジェクトの外のファイルに書きます：変えるタスクすべて、どう変えるか、上のルールでなぜそうするか、そして決められないこと。続いて `{alm} confirm --title \"タスクバッグを移行\" --plan <ファイル>` を実行します。終了コード 0 は計画全体への同意です。それ以外はキャンセルか無回答です：何も変えずに終わり、何も変わっていないことを伝えます。",
    apply: "計画は `alm` を通してだけ実行します。上のコマンドに `--as <あなたのセッション名>` を付けて使います。タスクバッグのファイルを手で編集しません。更新が終わるまで、`alm` はこれらの変更の Git コマンドを表示せず、最後にまとめてコミットします。",
    finish: "確認：`{alm} list --json` が問題を何も報告せず、上の確かめ方もすべて満たすこと。それから `{alm} migrate --finish` を実行します。直すものが残っていないときだけ新しい形式を書き込み、残っていればそれを一覧にします：直してからもう一度実行します。",
    section: "プロジェクトのフォルダーで、{files} の Allomos の節（見出しが「{heading}」で始まるもの、または Allomos や `alm` についてのほかの説明）を `{alm} brief` の出力で置き換え、そのまま（英語のまま）使います。ファイルのほかの部分は変えません。",
    report_older: "何が変わったか、そして決められなかったことを相手に伝えます。",
    report_section: "何を変えたか、どんなコミットをしたかをユーザーに見せてください。",
    commit_older: "`alm migrate --finish` が成功したら、更新を一つのコミットにします：タスクバッグのファイル{files}だけ（タスクバッグのファイルの Git コマンドは `alm migrate --finish` が表示します）。コミットだけで、プッシュはしません。ほかの変更はそのままにします。",
    commit_section: "プロジェクトが Git リポジトリなら、これらの変更だけをコミットします：タスクバッグのファイル{files}。ほかの変更はそのままにし、頼まれない限りプッシュしません。",
    none: "（なし）",
};

fn phrases(language: &str) -> &'static Phrases {
    match language {
        "zh-CN" => &ZH_CN,
        "ja" => &JA,
        _ => &EN,
    }
}

/// What to update: the taskbag (from `found`, through `entries`) and/or the section in `files`.
struct Update<'a> {
    older: Option<(&'a str, Vec<&'static Migration>)>,
    files: Vec<String>,
}

fn request(
    language: &str,
    project: &Path,
    taskbag: &Path,
    formats: &Formats,
    update: &Update,
) -> String {
    let w = phrases(language);
    let alm = format!("alm --taskbag \"{}\"", shown(taskbag));
    let fill = |text: &str| text.replace("{alm}", &alm);
    let list = update.files.join(", ");
    let and_files = match (language, update.files.is_empty()) {
        (_, true) => String::new(),
        ("zh-CN", false) => format!("和 {list}"),
        ("ja", false) => format!("と {list}"),
        _ => format!(" and {list}"),
    };
    let mut out = String::new();
    let mut steps: Vec<String> = Vec::new();
    match &update.older {
        Some((found, _)) => {
            out.push_str(
                &w.older_title
                    .replace("{found}", found)
                    .replace("{current}", &formats.current_format()),
            );
        }
        None => out.push_str(w.section_title),
    }
    if update.older.is_none() {
        out.push_str(&format!(
            "\n\n{}: {}\n{}: {}\n",
            w.project,
            shown(project),
            w.taskbag,
            shown(taskbag)
        ));
    }
    if let Some((found, entries)) = &update.older {
        // The same opening as moving another task system into Allomos (Task-A05-02): an older
        // taskbag is one more task system to migrate (owner, 2026-10-14).
        out.push_str(&format!(
            "\n\n{}\n\n{}\n",
            migration_header_in(language, project, taskbag),
            ask_once_in(language, Source::OlderTaskbag)
        ));
        let n = format_version(found).unwrap_or_default();
        let commands: Vec<String> = formats
            .allowed_commands(n)
            .iter()
            .map(|c| format!("`alm {c}`"))
            .collect();
        out.push_str(&format!(
            "\n{}\n",
            w.read_only.replace("{commands}", &commands.join(", "))
        ));
        out.push_str(&format!("\n## {}\n", w.changes));
        for m in entries {
            out.push_str(&format!(
                "\n### allomos/{} → allomos/{}\n\n",
                m.from,
                m.from + 1
            ));
            let used: Vec<String> = m.commands.iter().map(|c| format!("`alm {c}`")).collect();
            let used = if used.is_empty() {
                w.none.to_string()
            } else {
                used.join(", ")
            };
            for (name, text) in [
                (w.changed, m.changed.get(language)),
                (w.add, m.add.get(language)),
                (w.decide, m.decide.get(language)),
                (w.commands, used.as_str()),
                (w.check, m.check.get(language)),
            ] {
                out.push_str(&format!("- **{name}:** {}\n", fill(text)));
            }
        }
        steps.push(fill(w.look));
        steps.push(fill(w.plan));
        steps.push(fill(w.apply));
        steps.push(fill(w.finish));
    }
    if !update.files.is_empty() {
        steps.push(fill(
            &w.section
                .replace("{files}", &list)
                .replace("{heading}", AGENT_SECTION_HEADING),
        ));
    }
    if update.older.is_some() {
        steps.push(w.report_older.to_string());
        steps.push(w.commit_older.replace("{files}", &and_files));
    } else {
        steps.push(w.commit_section.replace("{files}", &and_files));
        steps.push(w.report_section.to_string());
    }
    if update.older.is_some() {
        out.push_str(&format!("\n## {}\n\n", w.steps));
    } else {
        out.push('\n');
    }
    for (i, s) in steps.iter().enumerate() {
        out.push_str(&format!("{}. {s}\n", i + 1));
    }
    out
}

/// The guided prompt for bringing the taskbag at `taskbag` (format `found`) up to this Allomos's
/// format, in an interface language (`en`, `zh-CN`, `ja`; English otherwise): what `alm migrate`
/// prints. `migration_missing` when an entry on the way is missing.
pub fn migration_prompt_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    found: &str,
) -> Result<String, Problem> {
    migration_prompt_with(language, project, taskbag, found, &FORMATS)
}

pub fn migration_prompt_with(
    language: &str,
    project: &Path,
    taskbag: &Path,
    found: &str,
    formats: &Formats,
) -> Result<String, Problem> {
    let missing = || Problem::MigrationMissing {
        found: found.to_string(),
        current: formats.current_format(),
    };
    let n = format_version(found).ok_or_else(missing)?;
    let entries = formats.pending(n).ok_or_else(missing)?;
    let update = Update {
        older: Some((found, entries)),
        files: Vec::new(),
    };
    Ok(request(language, project, taskbag, formats, &update))
}

/// What **Migrate the taskbag…** (and File → Connect with the Agents… while something is out of
/// date) asks the chosen agent, in an interface language (`en`, `zh-CN`, `ja`; English
/// otherwise): when the taskbag is older, the guided migration prompt ([`migration_prompt_in`]);
/// when Allomos's section is out of date, replacing it with `alm brief`'s output in each such file.
/// The agent does the work; the person sends the request (owner, 2026-10-12).
pub fn agent_update_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    check: &VersionCheck,
) -> String {
    agent_update_request_with(language, project, taskbag, check, &FORMATS)
}

fn agent_update_request_with(
    language: &str,
    project: &Path,
    taskbag: &Path,
    check: &VersionCheck,
    formats: &Formats,
) -> String {
    let older = match &check.format {
        FormatState::Older { found } => format_version(found)
            .and_then(|n| formats.pending(n))
            .map(|entries| (found.as_str(), entries)),
        _ => None,
    };
    let update = Update {
        older,
        files: check.outdated_files(),
    };
    request(language, project, taskbag, formats, &update)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, agent_section, init, load};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-versions-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// A project folder with a taskbag holding one task.
    fn project(name: &str) -> (PathBuf, PathBuf) {
        let dir = tmp(name);
        let root = dir.join(DEFAULT_DIR);
        init(&root, "demo", "Demo").unwrap();
        let lock = Lock::acquire(&root, LOCK_WAIT).unwrap();
        lock.create_area(&serde_json::from_str(r#"{"id":"A1","name":"Area"}"#).unwrap())
            .unwrap();
        lock.create_task(
            &serde_json::from_str(r#"{"id":"A1-01","area":"A1","title":"Bake","state":"ready"}"#)
                .unwrap(),
        )
        .unwrap();
        (dir, root)
    }

    fn set_project_format(root: &Path, format: &str) {
        let path = root.join(PROJECT_FILE);
        let text = fs::read_to_string(&path).unwrap();
        let old = read_format(root).unwrap();
        let text = text.replace(&format!("\"{old}\""), &format!("\"{format}\""));
        fs::write(path, text).unwrap();
    }

    /// A made-up entry from 4 to 5, standing in for a later one (never shipped): the top
    /// label is renamed, and each task decides nothing.
    const TEST_ENTRY: Migration = Migration {
        from: 4,
        changed: Words {
            en: "The top label is now called \"person checked\".",
            zh_cn: "最高的标签现在叫 \"person checked\"。",
            ja: "いちばん上のラベルは \"person checked\" になりました。",
        },
        add: Words {
            en: "Nothing.",
            zh_cn: "无。",
            ja: "なし。",
        },
        decide: Words {
            en: "Nothing: every task with \"human verified\" becomes \"person checked\".",
            zh_cn: "无。",
            ja: "なし。",
        },
        commands: &["rename label"],
        check: Words {
            en: "`{alm} list --json` shows no task labelled \"human verified\".",
            zh_cn: "`{alm} list --json` 里没有 \"human verified\"。",
            ja: "`{alm} list --json` に \"human verified\" がないこと。",
        },
    };
    const TEST_FORMATS: Formats = Formats {
        entries: &[TEST_ENTRY],
        current: 5,
        oldest: 4,
    };

    #[test]
    fn section_versions_are_read_from_the_heading() {
        assert_eq!(
            section_version(&agent_section()),
            Some(Some(SECTION_VERSION))
        );
        assert_eq!(
            section_version("# Ours\n\n## Tasks: Allomos (v7)\n\nUse `alm next`.\n"),
            Some(Some(7))
        );
        // A section from before versions, or merged by an agent in its own words.
        assert_eq!(
            section_version("## Tasks: Allomos\n\nRun `alm next`.\n"),
            Some(None)
        );
        assert_eq!(section_version("Start with alm next.\n"), Some(None));
        // Nothing about Allomos: not connected, nothing to check.
        assert_eq!(section_version("# Rules\n\nBe kind.\n"), None);
        assert_eq!(section_version("@AGENTS.md\n"), None);
    }

    #[test]
    fn nothing_is_checked_before_an_agent_is_connected() {
        let (dir, root) = project("not-connected");
        assert_eq!(check_connected(&root), None);
        fs::write(dir.join("AGENTS.md"), "# Rules\n").unwrap();
        assert_eq!(check_connected(&root), None);
    }

    #[test]
    fn an_old_or_unversioned_section_is_out_of_date() {
        let (dir, root) = project("sections");
        fs::write(
            dir.join("AGENTS.md"),
            "# Rules\n\n## Tasks: Allomos\n\nUse `alm next`.\n",
        )
        .unwrap();
        fs::write(dir.join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.format, FormatState::Current);
        assert_eq!(c.outdated_files(), vec!["AGENTS.md".to_string()]);
        assert!(c.needs_update());
        assert_eq!(
            c.instructions_warning().unwrap().code(),
            "instructions_outdated"
        );

        // Renewed with the current section: nothing to do.
        fs::write(
            dir.join("AGENTS.md"),
            format!("# Rules\n\n{}", agent_section()),
        )
        .unwrap();
        let c = check_connected(&root).unwrap();
        assert!(!c.needs_update());
        assert_eq!(c.instructions_warning(), None);

        // A CLAUDE.md that imports AGENTS.md under a title naming Allomos follows AGENTS.md.
        fs::write(
            dir.join("CLAUDE.md"),
            "# Allomos — Claude entry point\n\n@AGENTS.md\n",
        )
        .unwrap();
        assert!(!check_connected(&root).unwrap().needs_update());

        // A CLAUDE.md with its own old copy is listed too.
        fs::write(
            dir.join("CLAUDE.md"),
            "## Tasks: Allomos (v0)\n\nalm next\n",
        )
        .unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.outdated_files(), vec!["CLAUDE.md".to_string()]);
    }

    #[test]
    fn the_recorded_project_folder_is_the_one_checked() {
        let base = tmp("recorded");
        let root = base.join("tasks-repo").join(DEFAULT_DIR);
        init(&root, "demo", "Demo").unwrap();
        let app = base.join("app");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        crate::record_project_folder(&root, &app).unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.outdated_files(), vec!["AGENTS.md".to_string()]);
    }

    #[test]
    fn format_states() {
        let (_, root) = project("formats");
        assert_eq!(format_state(&root), FormatState::Current);
        assert_eq!(
            TEST_FORMATS.state(&root),
            FormatState::Older {
                found: "allomos/4".into()
            }
        );
        set_project_format(&root, "allomos/5");
        assert_eq!(
            format_state(&root),
            FormatState::Newer {
                found: "allomos/5".into()
            }
        );
        assert_eq!(TEST_FORMATS.state(&root), FormatState::Current);
        set_project_format(&root, "fliphopper/1");
        assert_eq!(format_state(&root), FormatState::Unknown);
    }

    /// Every version from the oldest supported to the current one has an entry: raising
    /// `FORMAT_VERSION` without adding one fails here.
    #[test]
    fn every_version_since_the_oldest_has_an_entry() {
        assert!(
            FORMATS.pending(OLDEST_FORMAT_VERSION).is_some(),
            "add a migration entry to MIGRATIONS for each format version from allomos/{OLDEST_FORMAT_VERSION}"
        );
        for m in MIGRATIONS {
            let supported = FORMATS.oldest..FORMATS.current;
            assert!(
                supported.contains(&m.from),
                "allomos/{}: no such step",
                m.from
            );
            for words in [m.changed, m.add, m.decide, m.check] {
                for text in [words.en, words.zh_cn, words.ja] {
                    assert!(!text.trim().is_empty(), "allomos/{}: empty text", m.from);
                }
            }
        }
    }

    /// The owner, 2026-10-14: `allomos/3` is the oldest; version 2 is too old, with no code steps.
    #[test]
    fn version_2_is_too_old() {
        let (_, root) = project("too-old");
        set_project_format(&root, "allomos/2");
        let before = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        let state = format_state(&root);
        assert_eq!(
            state,
            FormatState::TooOld {
                found: "allomos/2".into(),
                oldest: "allomos/3".into()
            }
        );
        let loaded = load(&root);
        assert!(loaded.taskbag.is_none());
        assert_eq!(loaded.issues[0].problem.code(), "taskbag_too_old");
        let err = finish(&root, Actor::APP).unwrap_err();
        assert_eq!(err.code(), "taskbag_too_old");
        assert!(err.english().contains("too old"), "{}", err.english());
        assert_eq!(
            write_allowed(&root, "claim").unwrap_err().code(),
            "taskbag_too_old"
        );
        assert_eq!(fs::read_to_string(root.join(PROJECT_FILE)).unwrap(), before);
    }

    #[test]
    fn mid_update_only_the_updates_own_commands_change_the_taskbag() {
        let (_, root) = project("mid-update");
        for command in ["claim", "label", "done", "add task", "labels", "plan"] {
            let err = write_allowed_in(&root, command, &TEST_FORMATS).unwrap_err();
            assert_eq!(err.code(), "taskbag_older", "{command}");
            let Problem::TaskbagOlder { left, current, .. } = &err else {
                unreachable!()
            };
            assert_eq!(current, "allomos/5");
            assert!(left.contains("no problems reported"), "{left}");
        }
        for command in ["rename label", "migrate", "handoff"] {
            write_allowed_in(&root, command, &TEST_FORMATS).unwrap();
        }
        // The real formats: a current taskbag allows everything.
        write_allowed(&root, "claim").unwrap();
        assert_eq!(
            TEST_FORMATS.allowed_commands(4),
            vec!["rename label", "migrate", "handoff"]
        );
    }

    #[test]
    fn an_update_finishes_only_when_nothing_is_left() {
        let (_, root) = project("three-to-four");
        let path = root.join(PROJECT_FILE);
        let before = fs::read_to_string(&path).unwrap();
        // Something is left: a task carries a label that is not on the ladder.
        let task = root.join("tasks/A1-01/task.json");
        let text = fs::read_to_string(&task).unwrap();
        let mut v: Value = serde_json::from_str(&text).unwrap();
        v["label"] = "person checked".into();
        fs::write(&task, serde_json::to_string_pretty(&v).unwrap()).unwrap();

        let err = finish_in(&root, Actor::APP, &TEST_FORMATS).unwrap_err();
        assert_eq!(err.code(), "migration_unfinished");
        let Problem::MigrationUnfinished { left } = &err else {
            unreachable!()
        };
        assert!(
            left.starts_with("1 problem(s): tasks/A1-01/task.json (label)"),
            "{left}"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before,
            "nothing written"
        );
        // The refusal of other commands says the same.
        let Err(Problem::TaskbagOlder { left: same, .. }) =
            write_allowed_in(&root, "claim", &TEST_FORMATS)
        else {
            panic!()
        };
        assert_eq!(&same, left);

        // The agent renames the label through `alm`'s rule: nothing is left.
        crate::rename_label(
            &root,
            Actor::alm(Some("claude/m")),
            LOCK_WAIT,
            "human verified",
            "person checked",
        )
        .unwrap();
        assert!(load(&root).issues.is_empty());

        let done = finish_in(&root, Actor::alm(Some("claude/m")), &TEST_FORMATS).unwrap();
        assert_eq!(
            done,
            Finished {
                from: "allomos/4".into(),
                to: "allomos/5".into(),
                changed: true
            }
        );
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(
            after,
            before
                .replace("\"allomos/4\"", "\"allomos/5\"")
                .replace("\"human verified\"", "\"person checked\""),
            "the format line changes, and the label the agent renamed"
        );
        assert_eq!(TEST_FORMATS.state(&root), FormatState::Current);
        let (events, _) = crate::events::list(&root);
        let last = events.last().unwrap();
        assert_eq!(last.command, "migrate");
        assert_eq!(last.by.as_deref(), Some("claude/m"));
        assert_eq!(last.changes[0].to, Some("allomos/5".into()));

        // Finishing again changes nothing.
        let again = finish_in(&root, Actor::APP, &TEST_FORMATS).unwrap();
        assert!(!again.changed);
        assert_eq!(fs::read_to_string(&path).unwrap(), after);
        // Now every command works again.
        write_allowed_in(&root, "claim", &TEST_FORMATS).unwrap();
    }

    #[test]
    fn a_newer_taskbag_or_a_missing_entry_changes_nothing() {
        let (_, root) = project("refused");
        let no_entry = Formats {
            entries: &[],
            current: 5,
            oldest: 4,
        };
        assert_eq!(
            finish_in(&root, Actor::APP, &no_entry).unwrap_err().code(),
            "migration_missing"
        );
        assert_eq!(
            migration_prompt_with("en", &root, &root, "allomos/4", &no_entry)
                .unwrap_err()
                .code(),
            "migration_missing"
        );
        set_project_format(&root, "allomos/6");
        let newer = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            finish_in(&root, Actor::APP, &TEST_FORMATS)
                .unwrap_err()
                .code(),
            "taskbag_newer"
        );
        assert_eq!(fs::read_to_string(root.join(PROJECT_FILE)).unwrap(), newer);
    }

    /// The real 3 → 4 entry, both parts. A01-31: waiting tasks stay waiting unless the agent
    /// moves one. A03-18: the old ladder's top step is renamed with `rename label`, the old note
    /// is moved with `done` or cleared with `checked`, and the finish is refused until no task
    /// carries the old name. Mid-migration only the entry's commands (and `migrate`, `handoff`)
    /// change the taskbag; the finish writes only the format line.
    #[test]
    fn the_real_migration_from_3_to_4() {
        let (_, root) = project("real-three-to-four");
        let lock = Lock::acquire(&root, LOCK_WAIT).unwrap();
        for (id, state, extra) in [
            (
                "A1-02",
                "waiting",
                r#","waiting_on":"the approver approves the card""#,
            ),
            (
                "A1-03",
                "waiting",
                r#","waiting_on":"A1-01 must be done first","next_dependency":"A1-01 finished""#,
            ),
            (
                "A1-04",
                "done",
                r#","label":"locally verified","finished_without_owner":"loop run""#,
            ),
            (
                "A1-05",
                "done",
                r#","label":"human verified","finished_without_owner":"card run""#,
            ),
        ] {
            let json =
                format!(r#"{{"id":"{id}","area":"A1","title":"T","state":"{state}"{extra}}}"#);
            lock.create_task(&serde_json::from_str(&json).unwrap())
                .unwrap();
        }
        drop(lock);
        set_project_format(&root, "allomos/3");
        // A taskbag written by allomos/3: the old default ladder and the old note's name.
        let path = root.join(PROJECT_FILE);
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, text.replace("human verified", "owner verified")).unwrap();
        let task_file = |id: &str| root.join(format!("tasks/{id}/task.json"));
        let text = fs::read_to_string(task_file("A1-05")).unwrap();
        fs::write(
            task_file("A1-05"),
            text.replace("human verified", "owner verified"),
        )
        .unwrap();
        let before = fs::read_to_string(&path).unwrap();
        let waiting_before = fs::read_to_string(task_file("A1-02")).unwrap();
        assert_eq!(
            format_state(&root),
            FormatState::Older {
                found: "allomos/3".into()
            }
        );
        for command in ["claim", "label", "add task", "labels", "release"] {
            assert_eq!(
                write_allowed(&root, command).unwrap_err().code(),
                "taskbag_older",
                "{command}"
            );
        }
        for command in [
            "block",
            "wait",
            "needs",
            "rename label",
            "done",
            "checked",
            "migrate",
            "handoff",
        ] {
            write_allowed(&root, command).unwrap();
        }
        let prompt = migration_prompt_in("en", &root, &root, "allomos/3").unwrap();
        assert!(prompt.contains("allomos/3 → allomos/4"));
        assert!(prompt.contains("block <id> --by"), "{prompt}");
        assert!(prompt.contains("[A03-18"), "{prompt}");
        assert!(
            prompt.contains("rename label \"owner verified\" \"human verified\""),
            "{prompt}"
        );
        assert!(prompt.contains("never guess"), "{prompt}");
        assert!(!prompt.contains("is added before version 4 is merged"));
        // The old name is reported, so the finish is refused and writes nothing.
        let left = load(&root).issues;
        assert_eq!(
            left.iter()
                .filter(|i| i.problem.code() == "finished_without_owner_renamed")
                .count(),
            2
        );
        assert_eq!(
            finish(&root, Actor::alm(Some("claude/m")))
                .unwrap_err()
                .code(),
            "migration_unfinished"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        // A01-31: the agent moves the one task another task holds up, through `alm`'s rule.
        let me = Actor::alm(Some("claude/m"));
        crate::change_task(&root, me, LOCK_WAIT, |bag| {
            allomos_core::ops::block(bag, "A1-03", None, &["A1-01".to_string()])
                .map(|o| (o, "block"))
        })
        .unwrap();
        // A03-18: the person answered; the top step and the task carrying it are renamed.
        crate::rename_label(&root, me, LOCK_WAIT, "owner verified", "human verified").unwrap();
        // Below the top: the note moves unchanged. At the top: checked, the old note cleared.
        crate::change_task(&root, me, LOCK_WAIT, |bag| {
            allomos_core::ops::finish(bag, "A1-04", Some("claude/m"), Some("loop run"), None)
                .map(|o| (o, "done"))
        })
        .unwrap();
        crate::change_task(&root, me, LOCK_WAIT, |bag| {
            allomos_core::ops::checked(bag, "A1-05").map(|o| (o, "checked"))
        })
        .unwrap();
        let moved = fs::read_to_string(task_file("A1-04")).unwrap();
        assert!(
            moved.contains(r#""without_human_check": "loop run""#),
            "{moved}"
        );
        assert!(!moved.contains("finished_without_owner"), "{moved}");
        let checked = fs::read_to_string(task_file("A1-05")).unwrap();
        assert!(!checked.contains("finished_without_owner"), "{checked}");
        let done = finish(&root, me).unwrap();
        assert_eq!(
            (done.from.as_str(), done.to.as_str(), done.changed),
            ("allomos/3", "allomos/4", true)
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before
                .replace("\"allomos/3\"", "\"allomos/4\"")
                .replace("owner verified", "human verified")
        );
        // The other waiting task is untouched; the moved one is blocked by A1-01.
        assert_eq!(
            fs::read_to_string(task_file("A1-02")).unwrap(),
            waiting_before
        );
        let bag = load(&root).taskbag.unwrap();
        let moved = bag.tasks.iter().find(|t| t.id == "A1-03").unwrap();
        assert_eq!(moved.state, allomos_core::State::Blocked);
        assert_eq!(moved.blocked_by, vec!["A1-01".to_string()]);
        let to_check: Vec<&str> = bag
            .tasks
            .iter()
            .filter(|t| t.to_check())
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(to_check, ["A1-04"]);
        let top = bag.tasks.iter().find(|t| t.id == "A1-05").unwrap();
        assert_eq!(top.label.as_deref(), Some("human verified"));
        assert_eq!(format_state(&root), FormatState::Current);
        write_allowed(&root, "claim").unwrap();
    }

    #[test]
    fn finishing_a_current_taskbag_does_nothing() {
        let (_, root) = project("current");
        let done = finish(&root, Actor::APP).unwrap();
        assert!(!done.changed);
        assert_eq!(done.to, FORMAT);
    }

    #[test]
    fn the_prompt_joins_the_entries_in_the_apps_language() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        const SECOND: Migration = Migration {
            from: 5,
            ..TEST_ENTRY
        };
        let two = Formats {
            entries: &[TEST_ENTRY, SECOND],
            current: 6,
            oldest: 4,
        };
        for language in ["en", "zh-CN", "ja"] {
            let p = migration_prompt_with(language, &project, &taskbag, "allomos/4", &two).unwrap();
            let alm = format!("alm --taskbag \"{}\"", shown(&taskbag));
            assert!(p.contains(&shown(&project)), "{language}");
            assert!(p.contains("allomos/4 → allomos/5"), "{language}");
            assert!(p.contains("allomos/5 → allomos/6"), "{language}");
            assert!(p.contains("allomos/6"), "{language}");
            assert!(
                p.contains(&format!("`{alm} list --json`")),
                "{language}: {p}"
            );
            assert!(
                p.contains(&format!("`{alm} migrate --finish`")),
                "{language}"
            );
            assert!(p.contains(&format!("`{alm} confirm --title")), "{language}");
            assert!(p.contains("`alm rename label`"), "{language}");
            assert!(p.contains(TEST_ENTRY.changed.get(language)), "{language}");
            assert!(!p.contains("{alm}"), "{language}: placeholders filled");
            // The same opening as moving another task system into Allomos (Task-A05-02).
            assert!(
                p.contains(&migration_header_in(language, &project, &taskbag)),
                "{language}"
            );
            assert!(
                p.contains(&ask_once_in(language, Source::OlderTaskbag)),
                "{language}"
            );
            assert!(!p.contains("brief`"), "{language}: no section step");
        }
        let en = migration_prompt_with("en", &project, &taskbag, "allomos/4", &two).unwrap();
        for words in [
            "change nothing",
            "wait for the person's answer",
            "only through `alm`",
            "could not decide",
            "When `alm migrate --finish` has succeeded, commit the update as one commit",
            "Commit only; do not push.",
        ] {
            assert!(en.contains(words), "{words}");
        }
        // From 4, only the second entry.
        let p = migration_prompt_with("en", &project, &taskbag, "allomos/5", &two).unwrap();
        assert!(!p.contains("allomos/4 → allomos/5"));
    }

    #[test]
    fn the_update_request_names_only_what_is_out_of_date() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        let check = |format: FormatState, version: Option<u32>| VersionCheck {
            format_current: FORMAT.into(),
            section_current: SECTION_VERSION,
            format,
            sections: vec![SectionState {
                file: "AGENTS.md".into(),
                version,
            }],
        };
        let older = || FormatState::Older {
            found: "allomos/4".into(),
        };
        for language in ["en", "zh-CN", "ja"] {
            let both = check(older(), None);
            let r = agent_update_request_with(language, &project, &taskbag, &both, &TEST_FORMATS);
            assert!(
                r.contains(&format!(
                    "alm --taskbag \"{}\" migrate --finish",
                    shown(&taskbag)
                )),
                "{language}"
            );
            assert!(
                r.contains(&format!("{}\" brief`", shown(&taskbag))),
                "{language}"
            );
            assert!(r.contains("AGENTS.md"), "{language}");
            assert!(r.contains("allomos/4 → allomos/5"), "{language}");

            let section = check(FormatState::Current, None);
            let r =
                agent_update_request_with(language, &project, &taskbag, &section, &TEST_FORMATS);
            assert!(
                !r.contains("migrate"),
                "{language}: no migration when the format is current"
            );
            assert!(r.contains("brief`"), "{language}");

            let format = check(older(), Some(SECTION_VERSION));
            let r = agent_update_request_with(language, &project, &taskbag, &format, &TEST_FORMATS);
            assert!(
                r.contains("migrate --finish`") && !r.contains("brief`"),
                "{language}"
            );
        }
        // The section-only request reads as before (the app's tests rely on its first line).
        let r =
            agent_update_request_in("en", &project, &taskbag, &check(FormatState::Current, None));
        assert!(r.starts_with("This project was set up by an older Allomos"));
    }
}
