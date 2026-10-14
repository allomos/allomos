# Task-A01-38-no-git-in-alm

*Card type: decision, then coding. Area A01 (Tracking). Written 2026-10-14 at the owner's request, after one session's `alm wait --push` swept another agent's uncommitted migration into its commit.*

> **Not approved for code yet.** Decision record [0017](../decisions/0017-agents-run-git-not-alm.md) is *Proposed*; code starts when the owner accepts it. Where the details below differ from the record, the record wins.

## What gets built (plain words)

Allomos stops running Git. `alm` and the app only change the task files; after each change `alm` prints the exact Git commands for that change's files, and the agent runs them. The setup wizard no longer runs `git init` or commits; the agent it connects does that, after asking the person.

| Piece | What it is | Where |
|---|---|---|
| Decision record | The fifteen use cases and what each becomes | [docs/decisions/0017](../decisions/0017-agents-run-git-not-alm.md) |
| No `--commit` / `--push` | Removed from every command; `share()` goes | `crates/cli/src/lib.rs` |
| Printed Git commands | After each change: `changed_files` and `git_commands` (`git -C <taskbag> add -- …`, `commit -m "alm: <command> <id>" -m "Agent: <you>" -- …`, `push`); `alm claim` says to push before starting the work; `alm init` prints none | `crates/cli/src/lib.rs`, `crates/store` (which files each operation writes) |
| `alm init` | No repository lookup; the taskbag goes in the current folder or `--taskbag` | `crates/cli`, `crates/store` |
| Setup wizard | No `git init`, no commits (steps 2, 3); step 4's prompt asks the agent to set up Git with the person and commit | `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/SetupWizard.tsx`, `crates/store` (setup request) |
| "Last used by" | From the change history (`events/`), not Git | `apps/desktop/src-tauri/src/agents.rs`, `lib.rs` |
| Changes made in the app | "N changes not shared yet" in the app; the next `alm` output lists them with the commands | `apps/web`, `crates/cli` |
| Texts | AGENTS.md section (`alm brief`, version raised), `alm guide`, hints, setup / conversion / migration prompts (conversion and migration: commit when done, no push) | `crates/store/src/lib.rs`, `guide.rs`, `versions.rs`, `convert.rs`, `apps/web/src/i18n/*` |

**What you can see at the end:** `alm claim A01-01` changes the task and prints three Git commands naming only that task's files, with "push now, before you start the work"; `git status` shows those files uncommitted until the agent runs them. The wizard sets up a new folder outside Git without errors, and its step 4 prompt asks the agent to set up Git.

## Details for the worker

- **Outcome.** Git never runs behind the agent's or the person's back; each commit holds exactly one change, made by the one who made it.
- **Decisive unknowns.** None open; see [0017 § Decision](../decisions/0017-agents-run-git-not-alm.md#decision-proposed-every-row-agreed-with-the-owner-in-chat-2026-10-14).
- **External operations.** None.
- **Done means mechanically.**
  - 0017 accepted by the owner.
  - The named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`).
  - Tests: each changing command lists exactly the files it wrote (and no others, with unrelated changes present); `--commit` / `--push` are refused as unknown options; `alm claim`'s wording; `alm init` prints no commands and reads no Git; the wizard finishes in a folder outside Git; "last used by" comes from events; app changes appear in the next `alm` output.
  - Human check: run `alm claim` in a test taskbag and read the printed commands; set up a new folder with the wizard.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Write the decision record (this card's first step) | owner | 2026-10-14 | chat with session claude/a01-38-no-git-in-alm |
| Code | — | — | waits on 0017 |
