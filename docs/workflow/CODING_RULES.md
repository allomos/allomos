# Coding rules

A **contract** for code in this repository. Created by [`Task-A01-01`](../tasks/Task-A01-01-walking-skeleton.md). Extend it in the card that needs a new rule.

## Layout

| Where | What | Language |
|---|---|---|
| `crates/core` | Project rules: model and validation. **No disk, network or UI code.** | Rust |
| `crates/store` | Reading and writing taskbags on disk. Never runs `git` by itself. | Rust |
| `crates/cli` | The `allomos` / `alm` command: reads options, calls the crates, prints results. **No rules of its own.** | Rust |
| `apps/desktop/src-tauri` | The Tauri desktop app: thin commands that call the crates. | Rust |
| `apps/web` | The interface. Talks to Rust **only through `src/messenger.ts`**. | TypeScript + React |

Project rules live in Rust. The interface displays results and sends requests; it never decides whether a change is valid.

## Versions

- **Rust 1.85.1**, pinned in `rust-toolchain.toml`. **Tauri 2.11.x**: Tauri 2.12 needs Rust 1.90, so moving up is its own card.
- Cargo uses resolver 3 with `rust-version = "1.85"`, so dependencies are chosen to fit that Rust. `Cargo.lock` is committed.
- JavaScript packages are pinned to exact versions. `@tauri-apps/api` must match the Tauri crate's minor version (2.11), and plugin packages must match their crates. npm is used; there is no pnpm.
- **Command and locking packages** ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)): `clap` =4.6.7 (MIT or Apache-2.0; needs Rust 1.85) reads `alm`'s options; `fs4` =1.1.0 (MIT or Apache-2.0; needs Rust 1.75) gives the file lock that Rust's own `File::lock` gives from 1.89. Switch to `File::lock` when Rust moves up.
- Never upgrade a dependency on the side. Upgrade it on purpose, in a card, with its licence noted.

## Checks before a commit

From the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
npm --prefix apps/web run check:i18n
npm --prefix apps/web test
```

All must pass. Report anything not run as NOT RUN.

## Writing code

- **Problems are values, not crashes.** Reading a taskbag returns what could be read plus a list of problems. Each problem names the file and field, and explains it in plain words: what is wrong and how to fix it.
- **Never overwrite user content.** Creating a taskbag into a non-empty folder is refused. Writing files keeps the format in [FILE-FORMAT.md](../product/FILE-FORMAT.md).
- **Tests use temporary folders and synthetic data only**, never the owner's real projects.
- **Test folders made by hand or by an agent go under `~/allomos-tests/`** (owner, 2026-10-09; renamed from `~/fliphopper-tests/` 2026-10-11): one folder per check, named after the card, for example `~/allomos-tests/a05-02-bakery/`. This covers folders for the owner's manual checks and agent trials, never the system's temporary folder or the repository. Automated tests (`cargo test`) keep using the system's temporary folder and remove what they create. Delete a check's folder once the card is done.
- **Interface tests** ([`Task-A05-03`](../tasks/Task-A05-03-taskbag-setup-wizard.md)): decisions the interface makes (which step or button to show) live in plain TypeScript modules with only type imports, such as `apps/web/src/setupFlow.ts`, and are tested in `apps/web/tests/*.test.ts` with Node's own test runner (`node --experimental-strip-types --test`, Node 22.6 or newer). No test packages are installed.
- **Keep interface text short and plain**, and keep its terms consistent with the [glossary](../GLOSSARY.md): taskbag, area, state, label.

## Interface languages

The interface speaks English, Simplified Chinese and Japanese (R94). English is the reference.

- **No interface text in the code.** Every text the interface shows is a key in `apps/web/src/i18n/en.json`, `zh-CN.json` and `ja.json`, shown with `t()` (or `rich()` when a placeholder holds an element). The user's own data, file paths and field names are not translated.
- **The product's name in Chinese text** (owner, 2026-10-12): in a Chinese sentence that explains something, write it as 「换肩」, in corner brackets; where the name stands alone (the top bar, a label, a heading) it is 换肩 without them. Japanese text does the same with **肩代** (owner, 2026-10-13): 「肩代」 in a sentence, 肩代 alone.
- **Write English first, and add Chinese and Japanese in the same edit.** Keep each text short and plain, and keep the same `{placeholders}` in every language.
- **Rust sends codes, not sentences.** A problem is a `Problem` in `crates/core/src/problem.rs`: a code, its details, and one English wording for logs and `alm`. After adding or changing a code, run `ALLOMOS_WRITE_CODES=1 cargo test -p allomos-core` to update `apps/web/src/i18n/codes.json`, then add `problem.<code>` to every language file.
- **The checks catch slips:** `typecheck` fails when a language file misses a key; `check:i18n` also fails on extra or empty texts, different placeholders, and Rust codes without words; `cargo test` fails when `codes.json` is out of date.
- The computer's language is read in Rust (`sys-locale` 0.3.2, MIT or Apache-2.0), so it does not depend on what the macOS window reports. The user's own choice is kept in the window's storage.
- **Formatting:** `cargo fmt` defaults for Rust; two-space indentation and double quotes for TypeScript.

## Build output (`target/`)

Rust puts everything it builds in `target/`, one per worktree. A Tauri app's `target/` reaches several gigabytes, so these rules keep disk use to about one `target/` per active card (owner, 2026-10-06).

- **Development builds keep line numbers only.** The workspace `Cargo.toml` sets `[profile.dev] debug = "line-tables-only"`: crash reports and test failures still name the file and line, but the full information a debugger needs is not kept. When you really need a debugger (`lldb`), turn it on for that one build only, `CARGO_PROFILE_DEV_DEBUG=true cargo build`, and do not commit a change to the setting.
- **One `target/` per worktree.** Do not set a shared `CARGO_TARGET_DIR`: parallel sessions would wait on each other's builds and keep rebuilding over each other's changes.
- **No routine `cargo clean`.** It throws away work that the next build must redo. Run it only in your own worktree, and only when builds behave strangely after a Rust or Tauri upgrade, or when the disk is short of space.
- **Remove a worktree when its card is merged.** This deletes its `target/` with it. See [CARDS.md § Claiming protocol](./CARDS.md#claiming-protocol).
- Do not build in the main checkout; build in your card's worktree.

## The `alm` command

A **contract** for agents and scripts ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)). `allomos` and `alm` are the same program.

**`alm` tells; the agent checks** (owner, 2026-10-13). When a task needs judging (are these handoffs finished? is this card ready?), `alm` lays out the facts and asks the agent to check; it does not decide for it. A new check made by `alm` itself needs the owner's approval first, recorded on the card. Refusing invalid input (unknown IDs, bad dates, the holder rule) is not such a check.

| Command | What it does |
|---|---|
| `alm init [--name <name>] [--id <id>] [--labels <a,b,…>] [--write-agent-files]` | Creates an empty taskbag in `.allomos/taskbag/` at the root of the current Git repository, or of the current folder outside Git. Refused (`taskbag_exists`) when one is there. Never creates a Git repository. `--id` sets the project ID (default: from the folder's name); `--labels` sets the label ladder, lowest step first (default: implemented, locally verified, deployed and verified, owner verified; empty → `ladder_empty`). It then prints numbered steps asking the AI agent that ran it to add Allomos's section to the project's `AGENTS.md` and `CLAUDE.md`, merged with what is there (also in `--json` as `agent_setup`) ([`Task-A01-14`](../tasks/Task-A01-14-init-asks-the-agent.md)). `--write-agent-files` writes the section itself instead (see [FILE-FORMAT](../product/FILE-FORMAT.md#where-the-taskbag-is)) and lists the files in `agent_files`; with `--commit`, they go into the same commit. |
| `alm add area [<id>] <name>` | Adds an area, ordered after the existing ones. With only a name, the ID is the next free one: the letters of the last area's ID with one more than the highest number used with them, as wide as the widest, at least two digits (`A08` after `A07`; `A01` in a taskbag with none) ([`Task-A01-19`](../tasks/Task-A01-19-create-in-app.md)). |
| `alm add iteration [<id>] <name> [--release <text>] [--start <date>] [--end <date>]` | Adds an iteration (when tasks are planned), ordered after the existing ones. Dates are `YYYY-MM-DD` (else `invalid_date`) ([`Task-A01-15`](../tasks/Task-A01-15-iterations-view.md)). With only a name, the ID is the next free one, the same way (`I16` after `I15`; `I01` with none) ([`Task-A01-19`](../tasks/Task-A01-19-create-in-app.md)). |
| `alm rename area <id> <name>` | Gives an area a new name; its ID and order stay. Unknown area: `unknown_area`; empty name: `area_name_missing` ([`Task-A01-29`](../tasks/Task-A01-29-rename-delete-areas-iterations.md)). |
| `alm rename label <old> <new>` | Renames a step of the label ladder (its place stays) and the label of every task that carries it, done and dropped ones too; one event for the project and one per task. Unknown old label: `label_not_on_ladder`; new one already on the ladder: `label_exists`; empty: `label_name_missing` ([`Task-A01-32`](../tasks/Task-A01-32-guided-migration.md)). |
| `alm change iteration <id> [--name <text>] [--release <text>] [--start <date>] [--end <date>]` | Changes an iteration's name, release and dates; its ID and order stay. What is not given stays; an empty release or date (`""`) clears it; the name cannot be emptied (`iteration_name_missing`). Dates are checked as in `alm add iteration` (`invalid_date`) (A01-29). |
| `alm delete area <id>` / `alm delete iteration <id>` | Removes the area's or iteration's file. Refused while any task is in the area or planned in the iteration, done and dropped ones included, so history never points at one that is gone (`area_not_empty`, `iteration_not_empty`; owner-approved checks, 2026-10-14). The event lists what it had (A01-29). |
| `alm add task [<id>] --area <id> --title <text> [--description <text>]` | Adds a task in `backlog`; the description goes into `description.md`. Without an ID it takes the next free one in the area, `<area>-<number>` (A01-04 after A01-03; children like A01-03-01 do not count), as wide as the widest used, at least two digits ([`Task-A05-02`](../tasks/Task-A05-02-convert-existing-project.md)). |
| `alm plan <id> [<iteration>]` | Plans the task in an iteration (`unknown_iteration` when it does not exist); without one, the task becomes unplanned. Planning is not progress: anyone may plan any task, held or not, in any state. |
| `alm list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--without-owner] [--by area\|iteration]` | Tasks grouped by area: ID, state, label, holder, title; under a waiting task "waiting for <who or the owner>: <what>", under a blocked one "blocked by A01-17 (in progress), …" with each blocker's current state (`--json`: a blocked task adds `blockers` and `all_blockers_done`). `--state` takes any of the seven states. `--without-owner`: only tasks finished without the owner's check (A03-15). `--by iteration` groups them by iteration in order, then **Unplanned** (`--json`: `iterations`, each with its `tasks`, and `unplanned`). |
| `alm show <id>` | One task's fields, including its iteration, handoff note, who it waits for and what blocks it (each blocker with its state; a dropped one flagged "dropped; still blocked?"), then its description. When every blocker is done it says so; it moves nothing. |
| `alm history <id>` | The task's recorded changes, newest first: when, who (`--as`, and `alm` or `app`), the command, then each changed field `from → to`. Says when the taskbag started recording; changes before that are not known (`--json`: `task`, `since`, `events`; unreadable event files go in `issues`). Every command that changes a task, an area, an iteration or `project.json` writes exactly one event file in `events/` under the same lock; a refused change, or one that changes nothing, writes none ([FILE-FORMAT § Events](../product/FILE-FORMAT.md#events-allomos3), [`Task-A01-17`](../tasks/Task-A01-17-change-history.md)). |
| `alm claim <id> [--take]` | You hold the task; state becomes `in_progress`. Refused (`task_held`) when someone else holds it, unless `--take`, which takes it over with a `took_over` warning (for an agent whose quota ran out). Refused (`task_closed`) when done or dropped. |
| `alm label <id> <label>` | Sets the label; it must be on the project's ladder. |
| `alm wait <id> --on <text> [--who <name>]` | State `waiting`: a person (or an outside event) must act. `--on` says what (empty: `waiting_on_missing`); `--who` who, when it is not the owner (stored as `who`, cleared without it). Clears `blocked_by` ([0016](../decisions/0016-waiting-and-blocked-states.md), [`Task-A01-31`](../tasks/Task-A01-31-blocked-state.md)). |
| `alm block <id> --by <task-id>…` | State `blocked`: those tasks must finish first (several, or separated by commas; stored as `blocked_by`). Refused for an empty list (`blocked_by_missing`), an ID not in the taskbag (`blocker_unknown`), the task itself (`blocked_by_self`), or a done or dropped task (`task_closed`). A dropped blocker is allowed with the warning `blocker_dropped`. Clears `waiting_on` and `who`; same holder rule as `label` (A01-31). |
| `alm unblock <id>` | A `blocked` task moves on: `in_progress` when someone holds it, else `ready`; `blocked_by` cleared. Refused (`not_blocked`) for any other state. `alm` does not check the blockers: the agent does (owner, 2026-10-14). |
| `alm done <id> [--without-owner <text>]` | State `done`. `alm` does not judge whether the task is really finished: the rule (finish at the top label; in an automatic or loop run, below it with `--without-owner`) is in the guide and the brief. `--without-owner` records why and in which run in the task's `finished_without_owner` (empty text: `without_owner_missing`); the label stays as it was ([`Task-A03-15`](../tasks/Task-A03-15-done-rule-instead-of-warning.md)). Always ends with the rule for the agent to check, itself, the handoffs that name the task: get the list with `alm handoffs --task <id>`, look up each task a handoff names, remove the handoff with `alm handoff --remove <file> --push` when they are all done or dropped, otherwise leave it. `alm` looks at no handoff (`--json`: `handoff_check.instruction`) ([`Task-A03-13`](../tasks/Task-A03-13-done-cleans-up-handoffs.md)). |
| `alm release <id> [--note <text>]` | Clears the holder; `in_progress` goes back to `ready`. The note replaces the task's handoff note. |
| `alm needs <id> [<text>]` | Sets the task's next dependency; without text, clears it. |
| `alm ready <id>` | An unheld `backlog`, `waiting` or `blocked` task becomes `ready`, its *waiting on*, *who* and *blocked by* cleared. Otherwise refused (`cannot_make_ready`). |
| `alm title <id> <title>` | Gives a task a new title. Same holder rule as `label`; an empty title is refused (`task_title_missing`); allowed in any state ([`Task-A00-22`](../tasks/Task-A00-22-leftover-fliphopper-names.md)). |
| `alm project [--name <name>] [--id <id>]` | Changes the project's name and/or ID in `project.json`; without either, shows them. An empty name (`project_name_missing`) or an invalid ID (`invalid_id`) is refused. |
| `alm drop <id>` | State `dropped`; holder, *waiting on*, *who* and *blocked by* cleared. Refused (`task_closed`) when done or dropped. |
| `alm next [--iteration <id>]` | Every `ready` task no one holds (earliest iteration first, unplanned last, then area order and ID), each with its handoff note and latest handoff, and the rule for choosing one; `alm` chooses nothing. `--iteration` lists only that iteration's tasks; one that does not exist is refused (`unknown_iteration`). With nothing to take, the rule says so (`--json`: `tasks`, each with `handoff`; `instruction`; `handoff_rules`) ([`Task-A03-14`](../tasks/Task-A03-14-next-lists-ready-tasks.md)). Blocked and waiting tasks are never listed: skipped by their state. After the rule, each `blocked` task whose blockers are all done ("unblock it?") or include a dropped one ("still blocked?"); `alm next` changes nothing (`--json`: `blocked`) (A01-31). |
| `alm migrate [--language <code>]` / `alm migrate --finish` | For a taskbag in an older format: prints the guided migration prompt (the migration entries from its version to this Allomos's, with the project's paths, in the app's language unless `--language` says otherwise) and writes nothing (`--json`: `migrate` with `from`, `to`, `needed`, and `prompt`). `--finish` writes this Allomos's `format` and records a `migrate` event, refused with `migration_unfinished` (listing what is left) while `alm list --json` reports issues; on a current taskbag both change nothing ([`Task-A01-32`](../tasks/Task-A01-32-guided-migration.md)). Replaces `alm upgrade`. |
| `alm handoff-rules [--quota <n>] [--context <n>] [--write-when <text>] [--agent <name>] [--clear]` | Shows when to write a handoff: the gates that apply to you (`--as`, with your override), the overrides, and the template's path. With options, changes them: the project's own, or with `--agent` that agent's or account's override; `--clear` goes back to the defaults (with `--agent`: removes the override). A gate that is not a whole number from 0 to 100 is refused (`invalid_percent`) ([`Task-A03-06`](../tasks/Task-A03-06-agent-handoff-rules.md)). |
| `alm handoff-template [--set <file>]` | Shows the handoff template, or replaces it with a file (`template_empty` when empty). |
| `alm handoff [--tasks <id,id>] [--from <file>]` | This session's handoff (`--as` required): the first call makes `handoffs/<date>-<time>-<writer>.md` from the template, later calls update the same file. `--tasks` sets the tasks it covers (each must exist), `--from` its text. Prints the path (`--json`: `handoff`, `path`, `created`). `--remove <file>` instead deletes that handoff (named as `alm handoffs` prints it, or its bare file name); Git history keeps it. A file that is not a handoff is refused (`handoff_not_found`) (`--json`: `removed`). |
| `alm handoffs [--task <id>] [--latest]` | Handoffs, newest first. `alm show <id>` adds the task's latest one (`--json`: `handoff`), `alm next` each listed task's (`tasks[].handoff`), and `alm next` the asking agent's handoff rules (`handoff_rules`). `--check` prints only the rule for checking the handoffs yourself (all, or with `--task` those naming it), the same rule `alm done` gives; it lists nothing and gives no verdict (`--json`: `instruction`) ([`Task-A03-13`](../tasks/Task-A03-13-done-cleans-up-handoffs.md)). |
| `alm brief` | Prints Allomos's section for a project's `AGENTS.md`: the short brief (how to work tasks with `alm`, the terms Allomos relies on) that points to `alm guide`. With a taskbag outside the project folder, it names where the taskbag is. The app's requests tell agents to use this output, so `AGENTS.md` matches the installed `alm`. |
| `alm guide [terms\|customize\|tasks\|commands\|workflow]` | Prints the details the brief points to: what each term means, what a project may change and what is fixed, how to add a task that fits the project, every command, and how to work a task. All topics without one. |
| `alm task-language [<code>\|ui]` (also `task_language`) | Without a value: the language to write new and changed tasks in, and where it comes from (`taskbag`, `app`, or `system` before the app has run). With one: sets `task_language` (`invalid_language` when it is neither `ui` nor a language code). Agents run it before adding a task or changing its text ([`Task-A01-21`](../tasks/Task-A01-21-task-language.md)). `ALLOMOS_SETTINGS_DIR` moves the per-user settings folder (tests). |
| `alm labels <a,b,…>` | Sets the project's label ladder, lowest step first. Refused when empty (`ladder_empty`) or when a task's label would not be on it (`label_not_on_ladder`). |
| `alm confirm --title <text> --plan <file.md> [--timeout <seconds>]` | Shows the plan to the person in the Allomos app and waits (default 600 s) for **Go ahead** (exit 0) or **Cancel** (`confirm_declined`, exit 1); no answer in time is `confirm_timeout` (exit 1). The question waits in `<project>/.allomos/requests/` (ignored by Git) and is removed once settled. If no open window shows it within 3 s, `alm` opens the app on the project (macOS; `ALLOMOS_NO_LAUNCH` turns this off). It never writes the taskbag, which may not exist yet ([`Task-A05-02`](../tasks/Task-A05-02-convert-existing-project.md)). |

- **Version check before every command** ([`Task-A01-25`](../tasks/Task-A01-25-taskbag-format-upgrade.md)): except `init`, `brief`, `guide` and `confirm`, a taskbag in a newer format stops the command (`taskbag_newer`, exit 4), one older than the oldest it migrates refuses it (`taskbag_too_old`), and while an older one is being migrated only reading commands, `migrate`, `handoff` and the commands its migration entries name work (others: `taskbag_older`, listing what is left; [`Task-A01-32`](../tasks/Task-A01-32-guided-migration.md)); once an agent is connected, an out-of-date Allomos section in the project's `AGENTS.md` / `CLAUDE.md` adds the warning `instructions_outdated`. The section's heading carries its version, `## Tasks: Allomos (v3)` (`SECTION_VERSION` in `crates/store/src/versions.rs`): raise it whenever the section's wording changes.
- **Holder rules.** `label`, `wait`, `done` and `release` are refused (`task_held`) on a task someone else holds; an unheld task may be changed by anyone.
- **Finding the taskbag:** `--taskbag <path>` (the taskbag itself or a folder holding `.allomos/taskbag/`), else `ALLOMOS_TASKBAG`, else the nearest `.allomos/taskbag/` at or above the current folder.
- **Who you are:** `--as <name>`, else `ALLOMOS_HOLDER`. `claim` and `list --mine` refuse without one (`holder_missing`).
- **`--commit` / `--push`** ([`Task-A01-12`](../tasks/Task-A01-12-fh-commit-option.md)): after a change, commit **only the taskbag's files** with the message `alm: <command> <id>` and a line `Agent: <holder>`; `--push` then pushes the current branch. Other staged or changed files are left alone. A change that changes nothing makes no commit. When Git fails, the change stays saved, the exit code is still 0, and a warning says why (`commit_failed`, `push_failed`). Without these options `alm` never runs Git.
- **`--json`** prints exactly one line on standard output and nothing on the error stream: `{"schema":"alm/1","ok":true,"command":…,"taskbag":…,"task"|"area"|"areas":…,"warnings":[…],"issues":[…]}`, or `{"schema":"alm/1","ok":false,"command":…,"error":{"code","params","message"}}`. Problems use the codes in `crates/core/src/problem.rs`; `issues` lists problems found in the taskbag's files. A new field is an addition; any other change makes it `alm/2`.
- **Exit codes:** `0` done · `1` refused by a rule · `2` wrong usage (`bad_usage`) · `3` the taskbag is busy or could not be read or written · `4` the taskbag needs a newer Allomos (`taskbag_newer`).
- **Never waits for typed input.** Messages are English (Iteration 1); their codes are worded in all three interface languages.
- `ALLOMOS_LOCK_WAIT_MS` changes how long a writer waits for the lock (default 5000); tests use it.
- **Install on a Mac:** the app carries `alm` (`Allomos.app/Contents/MacOS/alm`, put there by `tauri.macos.conf.json`, which builds it before bundling), and `/usr/local/bin/alm` is a link to it, so updating the app updates `alm`. The `.pkg` makes the link when it installs (`apps/desktop/pkg/postinstall`; never over someone else's `alm`); after a `.dmg` install the app offers **Install the alm command** (asks for the Mac password; a real file there is named first; "Don't ask again" is remembered) ([`Task-A07-12`](../tasks/Task-A07-12-mac-package-installs-app-and-alm.md)). **Developing:** `cargo install --path crates/cli` puts `allomos` and `alm` in `~/.cargo/bin`, but Terminal uses that copy only if `~/.cargo/bin` comes before `/usr/local/bin` in `PATH`, which is not always so (check with `which -a alm`); otherwise rebuild the app and its `.pkg` and install it, so the linked `alm` updates too. The app's offer names an earlier `alm` that would hide the linked one. **Agents the app opens use the app's own `alm`** ([`Task-A03-16`](../tasks/Task-A03-16-agents-use-the-apps-own-alm.md)): a terminal window it opens puts the folder of the `alm` beside the app first in `PATH` (`target/debug` under `tauri dev`, so build `cargo build -p allomos-cli` first; with none there, `PATH` stays as it is), and a request sent through a link names that `alm` by full path when the shell would find another. **On Windows the installer installs `alm`** ([`Task-A07-06`](../tasks/Task-A07-06-windows-installer-installs-alm.md)): `alm.exe` goes beside the app, and the app's folder is added to the person's own PATH (removed again on uninstall), so `alm` works in terminals opened after installing.

## Running the app

```bash
npm --prefix apps/web install
npm --prefix apps/desktop install
npm --prefix apps/desktop run dev
```

`npm --prefix apps/desktop run dev` starts the interface and the desktop window together and reloads on changes. To open a folder directly, pass it at start-up: `ALLOMOS_OPEN=/path/to/folder npm --prefix apps/desktop run dev`, or `open Allomos.app --args /path/to/folder` for a built app. The folder is the first argument that is not an option; macOS options such as `-AppleLanguages '(ja)'` may come first. A given path that is not a folder is reported in the window instead of reopening the last folder ([`Task-A01-04`](../tasks/Task-A01-04-startup-folder-argument.md)).

While a taskbag is open, the window checks about once a second whether its files changed (a fingerprint of names, sizes and change times, `taskbag_stamp`) and reloads when they did, so changes made with `alm` show without reopening the folder. Polling was chosen over a folder watcher because it needs no new package or thread, and inside the desktop app it is a cheap local call. **Companions are different:** over the network, the desktop app pushes a change notice to them; they never ask on a timer (owner, 2026-10-06).

To build a local, unsigned app: `npm --prefix apps/desktop run tauri build -- --debug --bundles app`. The result is `target/debug/bundle/macos/Allomos.app`. Run it through `npm run` as shown, the same way GitHub's release build does: `npm run` starts Tauri inside `apps/desktop`, whereas `npx --prefix apps/desktop tauri …` starts it in the repository's root, where Tauri can pick the user guide's `docs/guide/package.json` and fail looking for `docs/web/package.json` (seen 2026-10-13).

## Building for Windows

**Releases are built by GitHub** ([`Task-A07-11`](../tasks/Task-A07-11-github-builds-releases.md), 2026-10-13): pushing a version tag such as `v0.2.0` runs `.github/workflows/release.yml`, which builds the macOS (Apple Silicon) `.dmg` and `.pkg` (made from the built app by `apps/desktop/scripts/build-mac-pkg.sh`, which also works on this Mac), the Windows installer and `alm` for both on GitHub's computers and attaches them, unsigned, to that version's release. The tag must match `version` in `tauri.conf.json` and `Cargo.toml`, or the run stops. GitHub charges no build time since the repository became public; it was private when [`Task-A07-02`](../tasks/Task-A07-02-build-pipeline.md) chose to build on the Mac (owner, 2026-10-11).

The Mac script below stays as a backup (owner, 2026-10-13). The Mac can only *build* the Windows version, not run it: nothing is tested on Windows this way.

```bash
bash apps/desktop/scripts/build-windows.sh
```

It leaves `Allomos_<version>_x64-setup.exe` (the unsigned installer; it needs no administrator rights, and it installs `alm` too) and `alm.exe` in `target/windows/`. The installer's Windows-only settings are in `apps/desktop/src-tauri/tauri.windows.conf.json` and `windows/hooks.nsh`; the script copies `alm.exe` to `apps/desktop/src-tauri/binaries/` first, which a Windows `cargo` check of the app also needs. Windows warns that the publisher is unknown until builds are signed at v1.0. The first build downloads Microsoft's Windows SDK libraries into a local cache and takes a few minutes.

One-time tools (the script names any that are missing):

```bash
brew install rustup makensis llvm
/opt/homebrew/opt/rustup/bin/rustup toolchain install 1.85.1 --profile minimal -c rustfmt,clippy -t x86_64-pc-windows-msvc
/opt/homebrew/opt/rustup/bin/cargo +1.85.1 install cargo-xwin --version 0.19.2 --locked
```

Homebrew's `rustup` is keg-only: it does not replace Homebrew's `rust`, and only the script puts it on PATH. `cargo-xwin` 0.19.2 is the last release that works with Rust 1.85. The script sets `LC_ALL=en_US.UTF-8` because `makensis` 3.13 crashes (`std::bad_alloc`) when no locale is set, as in agent shells.
