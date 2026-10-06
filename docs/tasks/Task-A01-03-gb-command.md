# Task-A01-03-gb-command

*Third card of **Iteration 1 — v0.1 macOS local tracker** ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). A **coding** card. It delivers the `glidebox` command (alias `gb`, R88) and settles [PLAN § 8 open point 2](../product/PLAN.md#at-a-glance-updated-2026-10-04): how `gb` and the running desktop app avoid writing the same task box at once.*

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| The `glidebox` command | A program run in a terminal. Agents (and people) use it to create a task box and to add, list, show, claim, label, park, finish and release tasks. `gb` is the same program under a short name. | `crates/cli` (new) |
| Task changes (Rust) | The rules for each change, for example "you cannot claim a task someone else holds" or "a label must be on the ladder". The command only reads input and prints results. | `crates/core` |
| Safe writing (Rust) | Writes task box files so that a reader never sees a half-written file, and so that two writers never change the task box at the same moment. | `crates/store` |
| The app sees changes | While the desktop app shows a task box, changes made by `gb` appear in the window without reopening the folder. | `apps/desktop`, `apps/web` |
| Contracts | The command's inputs, outputs and exit codes; how writers take turns; any new file in the task box. | `docs/product/FILE-FORMAT.md`, `docs/workflow/CODING_RULES.md` |

**What you can do at the end:** in any repository, an agent runs `gb init`, adds areas and tasks, claims one, moves its label up, parks it as waiting, and finishes or releases it. Several agents can do this at the same time, each in its own terminal. If the glidebox app is open on that task box, the window follows along.

## How `gb` and the app take turns (decision for the owner)

In Iteration 1 the desktop app only **reads** a task box; it writes only when it creates a starter project. Editing in the app arrives in Iteration 2. So today the real risks are:

- **two agents** running `gb` at the same moment, for example both claiming the same task;
- the app reading a file **while `gb` is half-way through writing it**.

The answer must still work when the app starts writing in Iteration 2, and `gb` must work when the app is closed ([INTERFACES § Local mode](../product/INTERFACES.md#local-mode-across-clients)).

| Option | How it works | For | Against |
|---|---|---|---|
| **A. Short turn per change** *(recommended)* | Every writer, `gb` now and the app later, takes an exclusive **operating-system lock** for the few milliseconds of one change: read, check the rules, write, let go. Each file is written to a temporary file and then renamed into place, so readers only ever see whole files. The app watches the folder and reloads after a change. | Works with or without the app open. Two agents claiming at once: the second one sees the first's claim and is refused. The lock disappears by itself if a process crashes. The app uses the same code when it starts editing. | Needs a small lock target that Git ignores. The lock only protects glidebox's own writers; a person editing a file by hand is not blocked. Validation still catches bad files, as today. |
| B. One holder while open | Whoever opens the task box for writing holds a lock until it closes. | Very simple. | The app being open would block every agent. Agents would block each other for a whole session. This defeats R11. |
| C. Send changes through the app | When the app runs, `gb` sends each change to it; when it does not, `gb` writes itself. | One writer at all times while the app runs. | Needs a local connection to the app, which arrives only in Iteration 9. There would be two code paths to test, and the two-agents race still needs A when the app is closed. |

**Recommendation: A.** Option A also keeps R75's "one running desktop app writes to a folder at a time": a second desktop app on the same folder is a separate check, added when the app starts editing (Iteration 2).

## Proposed commands

The names and options below are a proposal for the owner's review. The worker writes the final form into the contract.

| Command | What it does |
|---|---|
| `gb init [--name <name>]` | Creates `.glidebox/` at the root of the current Git repository, or in the current folder outside Git, with `project.json` and empty `areas/` and `tasks/`. It refuses when a task box already exists, and never creates a Git repository. |
| `gb add area <id> <name>` | Adds an area. |
| `gb add task <id> --area <id> --title <text> [--description <text>]` | Adds a task in `backlog`. Its description goes into `description.md`. |
| `gb list [--area <id>] [--state <state>] [--mine]` | Lists tasks grouped by area: ID, state, label, holder and title. |
| `gb show <id>` | One task in full: the fields, then the description. |
| `gb claim <id>` | Sets you as the holder and the state to `in_progress`. It refuses when someone else holds the task, naming them. |
| `gb label <id> <label>` | Sets the label. It must be a step on the project's ladder. |
| `gb wait <id> --on <text>` | Sets the state to `waiting` and records what it waits on. |
| `gb done <id>` | Sets the state to `done`. It warns, without refusing, when the label is not the top step of the ladder (see owner question 3). |
| `gb release <id>` | Clears the holder. A task `in_progress` goes back to `ready`. |

Shared behaviour:
- **Finding the task box:** `--box <path>`, else the `GLIDEBOX_BOX` environment variable, else search upward from the current folder for `.glidebox/`, the same way Git finds its repository.
- **Who you are:** `--as <name>`, else `GLIDEBOX_HOLDER`, for example `claude/a01-03-gb`. `claim` refuses without one, so holders are never blank.
- **For agents:** `--json` prints one versioned result (`"schema": "gb/1"`) on standard output and nothing else. Messages for people go to the error stream. Exit codes: `0` done, `1` refused by a rule (the reason has a code), `2` wrong usage, `3` the task box is busy or could not be written. No command ever waits for typed input.
- **Words:** messages are English in Iteration 1, because their readers are agents and terminals. They use the same problem codes as the interface ([`Task-A01-02`](./Task-A01-02-ui-languages.md)), so they can be translated later without changing the rules.

## Details for the worker

- **Outcome.** In a fresh Git repository, an agent creates a task box with `gb init`, fills it with areas and tasks, and works one task from claim to done using only `gb`. A second agent running at the same moment cannot take the same task. The open desktop app shows each change within about two seconds.
- **Decisive unknowns** (the worker finds out):
  - **Which lock.** Rust 1.85 (pinned) does not yet have the standard file lock (`File::lock` arrives in 1.89). The worker picks a small, pinned crate (for example `fs4` or `fd-lock`) with its licence noted. They choose what is locked: a lock file such as `.glidebox/.lock`, kept out of Git by a `.glidebox/.gitignore` that `gb init` writes. This keeps the task box self-contained (R74).
  - **Whether rename-into-place is safe on macOS** for a reader that is part-way through reading. The worker checks it, and on Windows too when Iteration 2 begins (noted for that card).
  - **How the app learns about changes:** a folder watcher (for example the `notify` crate, pinned) or a reload when the window regains focus plus a reload button. The worker picks one, so that a change appears within about two seconds while the window is open.
  - **How `gb` is installed for the owner and agents** on this Mac: `cargo install --path crates/cli` and a `gb` binary next to `glidebox`. Bundling it with the app is a later card.
- **Builds on.** [`Task-A01-02`](./Task-A01-02-ui-languages.md): Rust problems as codes. If A01-02 is not finished first, this card adds its own codes in the same way, and A01-02 adds their words.
- **Files owned.**
  - `crates/cli/**` (new);
  - `crates/core/**` (task changes and their rules);
  - `crates/store/**` (writing, locking, `init`);
  - `apps/desktop/src-tauri/**` and `apps/web/src/**` (reloading on change only; any new interface text in all three languages, per R94);
  - `docs/product/FILE-FORMAT.md` (lock file, `.gitignore`, writing rules), `docs/workflow/CODING_RULES.md` (command rules: output, exit codes, installation);
  - this card. Shared: `Cargo.toml` (new workspace member), `Cargo.lock`.
- **External operations.** None. Tests use temporary folders, never this repository's own files. Moving glidebox's own cards into a task box is a separate card.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, plus the interface's `typecheck`, `build` and translation check.
  - Tests cover every command: its success, its refusals, and its `--json` output and exit codes.
  - **Race test:** two processes claim the same task at the same moment, many times over. Exactly one wins each time, and no file is ever left half-written or invalid.
  - **End to end, recorded on the card:** a fresh temporary Git repository; `gb init`; add two areas and three tasks; claim, label, wait, done and release; `gb list` and `gb show` match the files; `git status` shows only the task box's text files and no lock file; the desktop app, opened on that folder, follows the changes without reopening.
  - The owner runs a few commands and watches the app follow.

## Owner questions at review

1. **How writers take turns:** option A (recommended), B or C, above.
2. **What `gb init` creates.** Recommendation: an empty project with no sample tasks, because agents add their own and there is no delete command yet. The app keeps its sample starter project for an empty folder.
3. **What `gb done` requires.** Recommendation: it warns but allows finishing below the top label. On this project the workflow already says "Done" means owner verified, and other projects may use other ladders. The alternative is to refuse unless the task is at the top step.
4. **Delete and edit commands** (rename a task, change its area, remove it) are not in this card. Recommendation: add them with in-app editing in Iteration 2, together with the history of changes.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
