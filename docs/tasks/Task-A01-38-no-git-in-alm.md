# Task-A01-38-no-git-in-alm

*Card type: decision, then coding. Area A01 (Tracking). Written 2026-10-14 at the owner's request, after one session's `alm wait --push` swept another agent's uncommitted migration into its commit.*

> **Approved for code.** Decision record [0017](../decisions/0017-agents-run-git-not-alm.md) was accepted by the owner on 2026-10-14. Where the details below differ from the record, the record wins.

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
- **Decisive unknowns.** None open; see [0017 § Decision](../decisions/0017-agents-run-git-not-alm.md#decision-every-row-agreed-with-the-owner-in-chat-2026-10-14).
- **External operations.** None.
- **Done means mechanically.**
  - The named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`).
  - Tests: each changing command lists exactly the files it wrote (and no others, with unrelated changes present); `--commit` / `--push` are refused as unknown options; `alm claim`'s wording; `alm init` prints no commands and reads no Git; the wizard finishes in a folder outside Git; "last used by" comes from events; app changes appear in the next `alm` output.
  - Human check: run `alm claim` in a test taskbag and read the printed commands; set up a new folder with the wizard.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Write the decision record (this card's first step) | owner | 2026-10-14 | chat with session claude/a01-38-no-git-in-alm |
| Accept 0017 | owner | 2026-10-14 | chat with session claude/a01-38-no-git-in-alm ("accept") |
| Code (approve the card) | owner | 2026-10-14 | same chat ("approve") |

## Progress notes

- 2026-10-14 — **Built and locally verified** (session claude/a01-38-no-git-in-alm).

  **In plain words.** `alm` and the app no longer run Git at all. After every change, `alm` prints the exact `git add`, `git commit` and `git push` commands for the files that change wrote, and nothing else; the agent runs them. `alm claim` says to push at once, before starting the work, because other sessions cannot see an unpushed claim. `alm init` prints no Git commands and no longer looks for a repository: the taskbag goes in the current folder (or where `--taskbag` says). The setup wizard only writes files; the request it sends to the agent in step 4 asks the agent to ask the person before setting up Git, then to commit the taskbag. Changes made in the app that no agent has shared yet are counted in the app ("N changes made here are not shared yet"), and the next `alm` change (and `alm next`) lists them with their own commands, to commit as the person's. During a taskbag update, changes print no commands; `alm migrate --finish` prints one commit for the whole taskbag, without a push. The conversion request commits at the end, commit only.

  **How it works.** The store records every file a change writes or removes (`allomos_store::with_written_files`; a file rewritten with the same text does not count), so the printed list is exactly that change's files, also when other changes are uncommitted. "Changes not shared yet" are the app's events after the newest `alm` event (`events::unshared_app_changes`); "last used by" is the newest named `alm` event (`events::last_agent_use`). Removed: `--commit` / `--push` (refused as unknown options, exit 2 `bad_usage`), `share()` and `git_root`, the wizard's `keep_in_git`, the Git-reading "last used" code, and the problem codes `commit_failed`, `push_failed`, `git_missing`, `git_init_failed`. The section is version 5; the conversion request version 8, the connect request version 2.

  **Evidence.** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (core 40, store 64 + 13 integration, cli 2 + 43, desktop 24 + 1 ignored); `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (338 texts, 69 codes), `test` PASS (64). New tests: `printed_git_commands_share_only_that_change` (a real repository with a remote: running the printed commands commits exactly the claim's two files, README and another uncommitted task stay as they were, the push lands; no-op change names nothing), `each_change_names_exactly_the_files_it_wrote` (28 changing commands, each compared with the files that actually changed on disk, with an unrelated change present; `handoff --remove` names the removed file), `alm_needs_no_git_and_refuses_the_removed_options` (no `git` on `PATH`, folder outside Git; `--push` / `--commit` refused), `app_changes_not_shared_yet_come_with_the_next_change`, the mid-update and `migrate --finish` checks in `migrate_from_3_to_4_blocked_state_and_human_check`, `init_refuses_twice_and_needs_no_git` (subfolder of a repository: taskbag in that subfolder, no commands), and in the app `the_wizard_sets_up_a_folder_outside_git_without_running_git`. The Windows workflow was not started.

  **Human check (steps).** Test folder: `~/allomos-tests/a01-38-no-git-in-alm/project` (a Git repository with a local remote `../remote.git`; task A01-01 claimed by `trial/s1`, not committed). 1. `cd` there and run `alm --as trial/s1 label A01-01 implemented` with this branch's `target/debug/alm`: it prints three commands naming only `tasks/A01-01/task.json` and one `events/…` file. 2. `git status --short` shows them uncommitted, with `README.md` changed too. 3. Run the printed commands: `git show --stat HEAD` lists only those two files; `README.md` stays changed. 4. In the app built from this branch, set up a new empty folder outside Git with the wizard: no error, no `.git` made, and step 4's request starts with "Git: Allomos itself never runs Git…".

  **Not done here.** The taskbag's `project-notes.md` still says to use `alm … --push` (its lines 5, 14, 24, 42); it should be changed when this `alm` is installed for every session, not before, since other sessions still run the installed `alm`.
