# Task-A01-03-gb-command

*The file name keeps "gb" because card files never move ([CARDS.md](../workflow/CARDS.md#card-ids-and-files)); the command is `fliphopper` / `fh` since the rename ([`Task-A00-17`](./Task-A00-17-rename-to-crossjump.md)).*

*Third card of **Iteration 1 — v0.1 macOS local tracker** ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). A **coding** card. It delivers the `fliphopper` command (alias `fh`, R88) and settles [PLAN § 8 open point 2](../product/PLAN.md#at-a-glance-updated-2026-10-04): how `fh` and the running desktop app avoid writing the same seabag at once.*

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| The `fliphopper` command | A program run in a terminal. Agents (and people) use it to create a seabag and to add, list, show, claim, label, park, finish and release tasks. `fh` is the same program under a short name. | `crates/cli` (new) |
| Task changes (Rust) | The rules for each change, for example "you cannot claim a task someone else holds" or "a label must be on the ladder". The command only reads input and prints results. | `crates/core` |
| Safe writing (Rust) | Writes seabag files so that a reader never sees a half-written file, and so that two writers never change the seabag at the same moment. | `crates/store` |
| The app sees changes | While the desktop app shows a seabag, changes made by `fh` appear in the window without reopening the folder. | `apps/desktop`, `apps/web` |
| Contracts | The command's inputs, outputs and exit codes; how writers take turns; any new file in the seabag. | `docs/product/FILE-FORMAT.md`, `docs/workflow/CODING_RULES.md` |

**What you can do at the end:** in any repository, an agent runs `fh init`, adds areas and tasks, claims one, moves its label up, parks it as waiting, and finishes or releases it. Several agents can do this at the same time, each in its own terminal. If the FlipHopper app is open on that seabag, the window follows along.

## How `fh` and the app take turns (owner chose A, 2026-10-06)

In Iteration 1 the desktop app only **reads** a seabag; it writes only when it creates a starter project. Editing in the app arrives in Iteration 2. So today the real risks are:

- **two agents** running `fh` at the same moment, for example both claiming the same task;
- the app reading a file **while `fh` is half-way through writing it**.

The answer must still work when the app starts writing in Iteration 2, and `fh` must work when the app is closed ([INTERFACES § Local mode](../product/INTERFACES.md#local-mode-across-clients)).

| Option | How it works | For | Against |
|---|---|---|---|
| **A. Short turn per change** *(recommended)* | Every writer, `fh` now and the app later, takes an exclusive **operating-system lock** for the few milliseconds of one change: read, check the rules, write, let go. Each file is written to a temporary file and then renamed into place, so readers only ever see whole files. The app watches the folder and reloads after a change. | Works with or without the app open. Two agents claiming at once: the second one sees the first's claim and is refused. The lock disappears by itself if a process crashes. The app uses the same code when it starts editing. | Needs a small lock target that Git ignores. The lock only protects FlipHopper's own writers; a person editing a file by hand is not blocked. Validation still catches bad files, as today. |
| B. One holder while open | Whoever opens the seabag for writing holds a lock until it closes. | Very simple. | The app being open would block every agent. Agents would block each other for a whole session. This defeats R11. |
| C. Send changes through the app | When the app runs, `fh` sends each change to it; when it does not, `fh` writes itself. | One writer at all times while the app runs. | Needs a local connection to the app, which arrives only in Iteration 9. There would be two code paths to test, and the two-agents race still needs A when the app is closed. |

**Chosen: A** (owner, 2026-10-06). Option A also keeps R75's "one running desktop app writes to a folder at a time": a second desktop app on the same folder is a separate check, added when the app starts editing (Iteration 2).

## Proposed commands

The names and options below are a proposal for the owner's review. The worker writes the final form into the contract.

| Command | What it does |
|---|---|
| `fh init [--name <name>]` | Creates `.fliphopper/seabag/` at the root of the current Git repository, or in the current folder outside Git, with `project.json` and empty `areas/` and `tasks/`. It refuses when a seabag already exists, and never creates a Git repository. |
| `fh add area <id> <name>` | Adds an area. |
| `fh add task <id> --area <id> --title <text> [--description <text>]` | Adds a task in `backlog`. Its description goes into `description.md`. |
| `fh list [--area <id>] [--state <state>] [--mine]` | Lists tasks grouped by area: ID, state, label, holder and title. |
| `fh show <id>` | One task in full: the fields (including the handoff note), then the description. |
| `fh claim <id>` | Sets you as the holder and the state to `in_progress`. It refuses when someone else holds the task, naming them. |
| `fh label <id> <label>` | Sets the label. It must be a step on the project's ladder. |
| `fh wait <id> --on <text>` | Sets the state to `waiting` and records what it waits on. |
| `fh done <id>` | Sets the state to `done`. It warns, without refusing, when the label is not the top step of the ladder (owner, 2026-10-06). |
| `fh release <id> [--note <text>]` | Clears the holder. A task `in_progress` goes back to `ready`. `--note` leaves a short **handoff note** for whoever picks the task up next (owner, 2026-10-06). |
| `fh next` | Shows the first task that is `ready` and held by no one (by area, then ID), with its handoff note, so a new agent or account resumes in one step (owner, 2026-10-06). |

Shared behaviour:
- **Finding the seabag:** `--seabag <path>`, else the `FLIPHOPPER_SEABAG` environment variable, else search upward from the current folder for `.fliphopper/seabag/`, the same way Git finds its repository.
- **Who you are:** `--as <name>`, else `FLIPHOPPER_HOLDER`, for example `claude/a01-03-fh`. `claim` refuses without one, so holders are never blank.
- **For agents:** `--json` prints one versioned result (`"schema": "fh/1"`) on standard output and nothing else. Messages for people go to the error stream. Exit codes: `0` done, `1` refused by a rule (the reason has a code), `2` wrong usage, `3` the seabag is busy or could not be written. No command ever waits for typed input.
- **Words:** messages are English in Iteration 1, because their readers are agents and terminals. They use the same problem codes as the interface ([`Task-A01-02`](./Task-A01-02-ui-languages.md)), so they can be translated later without changing the rules.

## Details for the worker

- **Outcome.** In a fresh Git repository, an agent creates a seabag with `fh init`, fills it with areas and tasks, and works one task from claim to done using only `fh`. A second agent running at the same moment cannot take the same task. The open desktop app shows each change within about two seconds.
- **Decisive unknowns** (the worker finds out):
  - **Which lock.** Rust 1.85 (pinned) does not yet have the standard file lock (`File::lock` arrives in 1.89). The worker picks a small, pinned crate (for example `fs4` or `fd-lock`) with its licence noted. They choose what is locked: a lock file such as `.fliphopper/seabag/.lock`, kept out of Git by a `.fliphopper/seabag/.gitignore` that `fh init` writes. This keeps the seabag self-contained (R74).
  - **Whether rename-into-place is safe on macOS** for a reader that is part-way through reading. The worker checks it, and on Windows too when Iteration 2 begins (noted for that card).
  - **How the app learns about changes:** a folder watcher (for example the `notify` crate, pinned) or a reload when the window regains focus plus a reload button. The worker picks one, so that a change appears within about two seconds while the window is open.
  - **How `fh` is installed for the owner and agents** on this Mac: `cargo install --path crates/cli` and a `fh` binary next to `fliphopper`. Bundling it with the app is a later card.
- **Builds on.** [`Task-A01-02`](./Task-A01-02-ui-languages.md): Rust problems as codes. If A01-02 is not finished first, this card adds its own codes in the same way, and A01-02 adds their words.
- **Files owned.**
  - `crates/cli/**` (new);
  - `crates/core/**` (task changes and their rules);
  - `crates/store/**` (writing, locking, `init`);
  - `apps/desktop/src-tauri/**` and `apps/web/src/**` (reloading on change only; any new interface text in all three languages, per R94);
  - `docs/product/FILE-FORMAT.md` (lock file, `.gitignore`, writing rules), `docs/workflow/CODING_RULES.md` (command rules: output, exit codes, installation);
  - this card. Shared: `Cargo.toml` (new workspace member), `Cargo.lock`.
- **External operations.** None. Tests use temporary folders, never this repository's own files. Moving FlipHopper's own cards into a seabag is a separate card.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, plus the interface's `typecheck`, `build` and translation check.
  - Tests cover every command: its success, its refusals, and its `--json` output and exit codes.
  - **Race test:** two processes claim the same task at the same moment, many times over. Exactly one wins each time, and no file is ever left half-written or invalid.
  - **End to end, recorded on the card:** a fresh temporary Git repository; `fh init`; add two areas and three tasks; claim, label, wait, done and release; `fh list` and `fh show` match the files; `git status` shows only the seabag's text files and no lock file; the desktop app, opened on that folder, follows the changes without reopening.
  - The owner runs a few commands and watches the app follow.

## Owner answers (2026-10-06)

1. **How writers take turns: option A**, a short lock for each change.
2. **`fh init` creates an empty project** with no sample tasks. The app keeps its sample starter project when it creates a seabag.
3. **`fh done` warns but allows** finishing below the top label.
4. **Delete and edit commands wait for Iteration 2**, together with in-app editing and the history of changes.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | Card approved: all local work (source, tests, docs, commits, pushes; installing pinned packages locally). No external operations. | owner (chat) |
| 2026-10-06 | Add a handoff note (`fh release --note`, a new optional task field recorded in FILE-FORMAT) and `fh next`, so switching agents works from the first release ([`Task-A00-18`](./Task-A00-18-positioning-and-deck.md), [0014](../decisions/0014-main-purpose-and-three-sets.md)) | owner (chat) |

## What was built (2026-10-06)

**In plain words.** Agents now have a `fh` command (also `fliphopper`). It creates a seabag, adds areas and tasks, and moves a task from claim to done. Each change takes a short lock, so when two agents claim the same task at the same moment, only one gets it. Files are written whole and then swapped in, so the app never reads half a file. The app checks about once a second whether the files changed, and reloads when they did. A released task can carry a **handoff note**, which `fh next` and `fh show` print and the app shows.

**Decisions taken inside the card** (contracts in [FILE-FORMAT § Writing rules](../product/FILE-FORMAT.md#writing-rules-how-writers-take-turns) and [CODING_RULES § The `fh` command](../workflow/CODING_RULES.md#the-fh-command)):
- **The lock:** `fs4` =1.1.0 (MIT or Apache-2.0). It locks `.fliphopper/seabag/.lock`, which the seabag's own `.gitignore` keeps out of Git. A writer waits up to 5 s, then exits with code 3.
- **How the app learns about changes:** it asks for a fingerprint of the files once a second, instead of using a folder watcher. This needs no new package. The check is a cheap local call inside the desktop app only. **Companions must not copy it:** over the network, the desktop app pushes a change notice to them instead of being asked on a timer (owner, 2026-10-06).
- **Swapping in a new file is safe on macOS:** a reader that already opened the old file keeps reading the old file whole, and the race test's reader never saw a broken file. Windows is still to check in Iteration 2.
- **Added beyond the proposal:** `fh claim --take` takes over a task someone else holds and prints a `took_over` warning. Without it, an agent whose quota ran out would block the task. Other agents may not change a held task (`task_held`). A release replaces the handoff note, or removes it when no note is given.
- **Installing:** run `cargo install --path crates/cli`.

**Checks (2026-10-06):**

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace`: 16 tests for the rules (`core`), 10 for reading and writing (`store`), 9 that run `fh` as a real program (`cli`) | PASS (35) |
| Race test: 30 rounds of 4 agents claiming the same task at once, while another thread keeps reading the seabag; run 4 times | PASS: exactly 1 winner each round, the losers get `task_held` with exit 1, no broken file, no leftover temporary files |
| `npm --prefix apps/web run typecheck` / `build` / `check:i18n` (66 texts, 31 codes) | PASS |
| End to end (below) | PASS for `fh` and the files. The agent did not see the window (access was declined); the owner checked it |
| **Owner check**: the window follows `fh` | **PASS** (owner, 2026-10-06): a claim of A02-01 showed "held by owner", and a release showed Ready with "handoff note: second try", without reopening the folder |

**End to end, 2026-10-06**, in a fresh Git repository (`scratchpad/e2e-repo`) with the desktop app open on it:
1. `fh init --name "E2E demo"`, two areas, three tasks.
2. `agent-a` claims A01-01 and labels it `implemented`. `agent-b` tries to claim it: `task_held`, exit 1.
3. `agent-a` parks A01-01 as waiting on "owner review". `agent-b` claims A01-02, then releases it with the note "list works; sorting left". `fh next` shows A01-02 with that note.
4. `agent-a` finishes A01-01: done, with the `label_below_top` warning.
5. `fh list` and `task.json` match. `git status` shows only the seabag's text files (`.gitignore`, `.keep`, JSON, `description.md`) and no `.lock`.

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
- 2026-10-06 — Built by `claude/a01-03-fh`; see "What was built" above.
- 2026-10-06 — Owner verified; merged into `main`. **Done.**
- 2026-10-06 — Scope added by the owner (accepting the A00-18 recommendation): handoff note on release, shown by `fh next` and `fh show`; `fh next` added. The note is a new optional field in `task.json`; the worker records it in [FILE-FORMAT.md](../product/FILE-FORMAT.md) with the other contracts.
