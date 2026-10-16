# Task-A03-22-branch-rules

*Card type: coding.*

## What gets built (plain words)

Every task now remembers which Git branch it starts from and merges back into (its **base branch**), by the branch's real name, and which branch the work is done on (its **working branch**). `alm` never runs or reads Git: the agent tells it which branch it is on.

| Piece | In everyday words | Where |
|---|---|---|
| The base rules | Which base a new task gets, what a claim records, who may change a base | `crates/core/src/branch.rs` |
| Storage | The branches are written into the task's history (events `base_branch`, `working_branch`), not into `task.json`, so older Allomos builds still read the taskbag and the format stays `allomos/4` | `crates/store/src/lib.rs` (`add_task_with_base`, `claim_task`, `set_base`, `branches`) |
| `alm add task --branch <name> [--base <name>]` | On a task's working branch, the new task shares that task's base; otherwise the base is the branch you are on. Prints which and why; warns when you hold a task on another base | `crates/cli/src/lib.rs` |
| `alm claim <id> --branch <name> [--working <name>]` | Records the working branch; a task without a base gets the branch you are on; prints the base to start from and merge into, with the Git commands, and a fact when you are not on the base | `crates/cli/src/lib.rs` |
| `alm base <id> <branch>` | Changes a task's base (the holder, on an open task) | `crates/cli/src/lib.rs` |
| `alm show` | Shows `base branch` and `working branch` | `crates/cli/src/lib.rs` |
| The app's New task window | All open tasks on one base: used without asking (shown as a note). Different bases: the person must pick one, nothing preselected, Add waits for it. None: no base | `apps/web/src/createFlow.ts`, `CreateForm.tsx`; `open_taskbag` sends `branches` |
| Words for agents | `alm guide workflow` (step 2, Branches), `alm guide tasks` (step 6), `alm guide commands`, the brief (section v11); this repo's AGENTS.md section regenerated from `alm brief` | `crates/store/src/guide.rs`, `crates/store/src/lib.rs`, `AGENTS.md` |
| Old rules removed | CARDS.md's Claiming protocol (branch per card, holder = branch name) is gone; links now point at `alm guide workflow` | `docs/workflow/CARDS.md` and its links |

## Details for the worker

- **Outcome.** As the task's description in the taskbag (`alm show A03-22`), agreed with the approver 2026-10-16.
- **External operations.** None.
- **Done means mechanically.** Rust tests for the base rules (`crates/core/src/branch.rs` tests; `crates/cli/tests/alm.rs` `tasks_record_their_base_branch_and_claims_say_where_to_merge`; the desktop test `the_list_adds_areas_iterations_and_tasks_under_the_persons_name`), web tests for the window (`apps/web/tests/createFlow.test.ts`), `alm list --json` reports no issues, every check in CODING_RULES passes.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-16 | The rules in the task's description (chat) | will.w.hui |

## Notes

- 2026-10-16 — Decided: the branches live in the task's history only (event changes `base_branch` and `working_branch`), the way AI decisions do (A03-19). Reason: `task.json` refuses unknown fields, so a new field would make older Allomos builds refuse the taskbag and need format `allomos/5`; events with any field name are read by older builds (their history shows the field by its stored name). No format bump.
- 2026-10-16 — Decided: `alm claim --working <name>` names the working branch when it is not the branch the agent is on (default: `--branch`). Reason: the claim must record a working branch, and an agent that will make a new branch (for a worktree) knows its name before it exists.
- 2026-10-16 — Decided: `--branch` is optional on `alm add task` and `alm claim`; without it nothing is recorded for what is unknown and the warning `branch_missing` says how to pass it. Reason: tasks still work without a base (one gets it at claim), and existing scripts do not break.
- 2026-10-16 — Decided: a new command `alm base <id> <branch>` (holder only, open tasks). Reason: the `base_differs` warning asks the agent to decide, and after the task is added there must be a way to act on it.
- 2026-10-16 — Decided: when several tasks share the working branch the agent is on, an open task is preferred, then the lowest ID. Reason: a deterministic rule, and an open task is the line of work in progress.
- 2026-10-16 — Decided: the claim's Git commands are `git switch <base>`, `git pull --ff-only`, `git switch -c <working>` to start (or `git worktree add <folder> -b <working> <base>`), and `git switch <base>`, `git pull --ff-only`, `git merge <working>`, `git push` to merge; working on the base itself prints only the pull and "nothing to merge". alm runs none of them.
- 2026-10-16 — Decided: CARDS.md keeps "A new card" (writing the card file and adding its task) and moves the Now line into "Now — rules"; only the claiming and branch rules were removed, as A00-30 removes the rest later. The "Who you are" line now says session name, not branch.

## Evidence

2026-10-16, worktree `.claude/worktrees/a03-22-branch-rules`, branch `claude/a03-22-branch-rules` from `origin/main` at `eaa2259`.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (core 49, store lib 62, CLI 47, desktop 37 + 1 ignored, store integration tests all ok) |
| `npm --prefix apps/web run typecheck` | PASS |
| `npm --prefix apps/web run build` | PASS |
| `npm --prefix apps/web run check:i18n` | PASS (465 texts, 71 Rust codes) |
| `npm --prefix apps/web test` | PASS (114) |
| `git diff --check` | PASS |
| `alm list --json` issues | none |

Not run: the desktop app by hand (the New task window was checked by its flow tests only).
