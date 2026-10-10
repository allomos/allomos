# Coding rules

A **contract** for code in this repository. Created by [`Task-A01-01`](../tasks/Task-A01-01-walking-skeleton.md). Extend it in the card that needs a new rule.

## Layout

| Where | What | Language |
|---|---|---|
| `crates/core` | Project rules: model and validation. **No disk, network or UI code.** | Rust |
| `crates/store` | Reading and writing seabags on disk. Never runs `git` by itself. | Rust |
| `crates/cli` | The `fliphopper` / `fh` command: reads options, calls the crates, prints results. **No rules of its own.** | Rust |
| `apps/desktop/src-tauri` | The Tauri desktop app: thin commands that call the crates. | Rust |
| `apps/web` | The interface. Talks to Rust **only through `src/messenger.ts`**. | TypeScript + React |

Project rules live in Rust. The interface displays results and sends requests; it never decides whether a change is valid.

## Versions

- **Rust 1.85.1**, pinned in `rust-toolchain.toml`. **Tauri 2.11.x**: Tauri 2.12 needs Rust 1.90, so moving up is its own card.
- Cargo uses resolver 3 with `rust-version = "1.85"`, so dependencies are chosen to fit that Rust. `Cargo.lock` is committed.
- JavaScript packages are pinned to exact versions. `@tauri-apps/api` must match the Tauri crate's minor version (2.11), and plugin packages must match their crates. npm is used; there is no pnpm.
- **Command and locking packages** ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)): `clap` =4.6.7 (MIT or Apache-2.0; needs Rust 1.85) reads `fh`'s options; `fs4` =1.1.0 (MIT or Apache-2.0; needs Rust 1.75) gives the file lock that Rust's own `File::lock` gives from 1.89. Switch to `File::lock` when Rust moves up.
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
```

All must pass. Report anything not run as NOT RUN.

## Writing code

- **Problems are values, not crashes.** Reading a seabag returns what could be read plus a list of problems. Each problem names the file and field, and explains it in plain words: what is wrong and how to fix it.
- **Never overwrite user content.** Creating a seabag into a non-empty folder is refused. Writing files keeps the format in [FILE-FORMAT.md](../product/FILE-FORMAT.md).
- **Tests use temporary folders and synthetic data only**, never the owner's real projects.
- **Test folders made by hand or by an agent go under `~/fliphopper-tests/`** (owner, 2026-10-09): one folder per check, named after the card, for example `~/fliphopper-tests/a05-02-bakery/`. This covers folders for the owner's manual checks and agent trials, never the system's temporary folder or the repository. Automated tests (`cargo test`) keep using the system's temporary folder and remove what they create. Delete a check's folder once the card is done.
- **Keep interface text short and plain**, and keep its terms consistent with the [glossary](../GLOSSARY.md): seabag, area, state, label.

## Interface languages

The interface speaks English, Simplified Chinese and Japanese (R94). English is the reference.

- **No interface text in the code.** Every text the interface shows is a key in `apps/web/src/i18n/en.json`, `zh-CN.json` and `ja.json`, shown with `t()` (or `rich()` when a placeholder holds an element). The user's own data, file paths and field names are not translated.
- **Write English first, and add Chinese and Japanese in the same edit.** Keep each text short and plain, and keep the same `{placeholders}` in every language.
- **Rust sends codes, not sentences.** A problem is a `Problem` in `crates/core/src/problem.rs`: a code, its details, and one English wording for logs and `fh`. After adding or changing a code, run `FLIPHOPPER_WRITE_CODES=1 cargo test -p fliphopper-core` to update `apps/web/src/i18n/codes.json`, then add `problem.<code>` to every language file.
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

## The `fh` command

A **contract** for agents and scripts ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)). `fliphopper` and `fh` are the same program.

| Command | What it does |
|---|---|
| `fh init [--name <name>] [--id <id>] [--labels <a,b,…>] [--write-agent-files]` | Creates an empty seabag in `.fliphopper/seabag/` at the root of the current Git repository, or of the current folder outside Git. Refused (`seabag_exists`) when one is there. Never creates a Git repository. `--id` sets the project ID (default: from the folder's name); `--labels` sets the label ladder, lowest step first (default: implemented, locally verified, deployed and verified, owner verified; empty → `ladder_empty`). It then prints numbered steps asking the AI agent that ran it to add FlipHopper's section to the project's `AGENTS.md` and `CLAUDE.md`, merged with what is there (also in `--json` as `agent_setup`) ([`Task-A01-14`](../tasks/Task-A01-14-init-asks-the-agent.md)). `--write-agent-files` writes the section itself instead (see [FILE-FORMAT](../product/FILE-FORMAT.md#where-the-seabag-is)) and lists the files in `agent_files`; with `--commit`, they go into the same commit. |
| `fh add area <id> <name>` | Adds an area, ordered after the existing ones. |
| `fh add iteration <id> <name> [--release <text>] [--start <date>] [--end <date>]` | Adds an iteration (when tasks are planned), ordered after the existing ones. Dates are `YYYY-MM-DD` (else `invalid_date`) ([`Task-A01-15`](../tasks/Task-A01-15-iterations-view.md)). |
| `fh add task <id> --area <id> --title <text> [--description <text>]` | Adds a task in `backlog`; the description goes into `description.md`. |
| `fh plan <id> [<iteration>]` | Plans the task in an iteration (`unknown_iteration` when it does not exist); without one, the task becomes unplanned. Planning is not progress: anyone may plan any task, held or not, in any state. |
| `fh list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area\|iteration]` | Tasks grouped by area: ID, state, label, holder, title. `--by iteration` groups them by iteration in order, then **Unplanned** (`--json`: `iterations`, each with its `tasks`, and `unplanned`). |
| `fh show <id>` | One task's fields, including its iteration and handoff note, then its description. |
| `fh claim <id> [--take]` | You hold the task; state becomes `in_progress`. Refused (`task_held`) when someone else holds it, unless `--take`, which takes it over with a `took_over` warning (for an agent whose quota ran out). Refused (`task_closed`) when done or dropped. |
| `fh label <id> <label>` | Sets the label; it must be on the project's ladder. |
| `fh wait <id> --on <text>` | State `waiting`, with what it waits on. |
| `fh done <id>` | State `done`. Warns (`label_below_top`) without refusing when the label is below the top step. |
| `fh release <id> [--note <text>]` | Clears the holder; `in_progress` goes back to `ready`. The note replaces the task's handoff note. |
| `fh needs <id> [<text>]` | Sets the task's next dependency; without text, clears it. |
| `fh ready <id>` | An unheld `backlog` or `waiting` task becomes `ready`, its *waiting on* cleared. Otherwise refused (`cannot_make_ready`). |
| `fh drop <id>` | State `dropped`; holder and *waiting on* cleared. Refused (`task_closed`) when done or dropped. |
| `fh next` | The first `ready` task no one holds (by area order, then ID), with its handoff note. |
| `fh brief` | Prints FlipHopper's section for a project's `AGENTS.md`: the short brief (how to work tasks with `fh`, the terms FlipHopper relies on) that points to `fh guide`. With a seabag outside the project folder, it names where the seabag is. The app's requests tell agents to use this output, so `AGENTS.md` matches the installed `fh`. |
| `fh guide [terms\|customize\|commands\|workflow]` | Prints the details the brief points to: what each term means, what a project may change and what is fixed, every command, and how to work a task. All topics without one. |
| `fh labels <a,b,…>` | Sets the project's label ladder, lowest step first. Refused when empty (`ladder_empty`) or when a task's label would not be on it (`label_not_on_ladder`). |
| `fh confirm --title <text> --plan <file.md> [--timeout <seconds>]` | Shows the plan to the person in the FlipHopper app and waits (default 600 s) for **Go ahead** (exit 0) or **Cancel** (`confirm_declined`, exit 1); no answer in time is `confirm_timeout` (exit 1). The question waits in `<project>/.fliphopper/requests/` (ignored by Git) and is removed once settled. If no open window shows it within 3 s, `fh` opens the app on the project (macOS; `FLIPHOPPER_NO_LAUNCH` turns this off). It never writes the seabag, which may not exist yet ([`Task-A05-02`](../tasks/Task-A05-02-convert-existing-project.md)). |

- **Holder rules.** `label`, `wait`, `done` and `release` are refused (`task_held`) on a task someone else holds; an unheld task may be changed by anyone.
- **Finding the seabag:** `--seabag <path>` (the seabag itself or a folder holding `.fliphopper/seabag/`), else `FLIPHOPPER_SEABAG`, else the nearest `.fliphopper/seabag/` at or above the current folder.
- **Who you are:** `--as <name>`, else `FLIPHOPPER_HOLDER`. `claim` and `list --mine` refuse without one (`holder_missing`).
- **`--commit` / `--push`** ([`Task-A01-12`](../tasks/Task-A01-12-fh-commit-option.md)): after a change, commit **only the seabag's files** with the message `fh: <command> <id>` and a line `Agent: <holder>`; `--push` then pushes the current branch. Other staged or changed files are left alone. A change that changes nothing makes no commit. When Git fails, the change stays saved, the exit code is still 0, and a warning says why (`commit_failed`, `push_failed`). Without these options `fh` never runs Git.
- **`--json`** prints exactly one line on standard output and nothing on the error stream: `{"schema":"fh/1","ok":true,"command":…,"seabag":…,"task"|"area"|"areas":…,"warnings":[…],"issues":[…]}`, or `{"schema":"fh/1","ok":false,"command":…,"error":{"code","params","message"}}`. Problems use the codes in `crates/core/src/problem.rs`; `issues` lists problems found in the seabag's files. A new field is an addition; any other change makes it `fh/2`.
- **Exit codes:** `0` done · `1` refused by a rule · `2` wrong usage (`bad_usage`) · `3` the seabag is busy or could not be read or written.
- **Never waits for typed input.** Messages are English (Iteration 1); their codes are worded in all three interface languages.
- `FLIPHOPPER_LOCK_WAIT_MS` changes how long a writer waits for the lock (default 5000); tests use it.
- **Install on this Mac:** `cargo install --path crates/cli` puts `fliphopper` and `fh` in `~/.cargo/bin`. Bundling with the app is a later card.

## Running the app

```bash
npm --prefix apps/web install
npm --prefix apps/desktop install
npm --prefix apps/desktop run dev
```

`npm --prefix apps/desktop run dev` starts the interface and the desktop window together and reloads on changes. To open a folder directly, pass it at start-up: `FLIPHOPPER_OPEN=/path/to/folder npm --prefix apps/desktop run dev`, or `open FlipHopper.app --args /path/to/folder` for a built app. The folder is the first argument that is not an option; macOS options such as `-AppleLanguages '(ja)'` may come first. A given path that is not a folder is reported in the window instead of reopening the last folder ([`Task-A01-04`](../tasks/Task-A01-04-startup-folder-argument.md)).

While a seabag is open, the window checks about once a second whether its files changed (a fingerprint of names, sizes and change times, `seabag_stamp`) and reloads when they did, so changes made with `fh` show without reopening the folder. Polling was chosen over a folder watcher because it needs no new package or thread, and inside the desktop app it is a cheap local call. **Companions are different:** over the network, the desktop app pushes a change notice to them; they never ask on a timer (owner, 2026-10-06).

To build a local, unsigned app: `npx --prefix apps/desktop tauri build --debug --bundles app`. The result is `target/debug/bundle/macos/FlipHopper.app`.
