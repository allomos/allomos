# Task-A01-12-fh-commit-option

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner (2026-10-07) after asking whether `fh` changes are committed automatically: they are not.*

## What gets built (plain words)

Today an agent changes a task with `fh`, then has to run `git commit` and `git push` itself. After this card, `fh … --commit` also commits that change, and `fh … --push` commits it and pushes it, in one step. Without the options nothing changes: `fh` still never runs Git on its own.

| Piece | What it is | Where |
|---|---|---|
| `--commit` | After a successful change, commit **only the seabag's files** (other staged or changed files are left alone), with the message `fh: <command> <id>` and a line naming who did it (`Agent: <holder>`). | `crates/cli` |
| `--push` | The same, then `git push` of the current branch. | `crates/cli` |
| When Git fails | The change itself is already saved, so `fh` does not undo it and exits 0, but warns (`commit_failed` or `push_failed`, with Git's own words), so the agent can commit or push by hand. Nothing to commit (the change made no difference) is not an error. | `crates/cli`, `crates/core/src/problem.rs` (two codes) |
| Contract and rules | The options in the `fh` contract; CARDS.md tells agents to use `--push` for status changes on `main`. | `docs/workflow/CODING_RULES.md`, `docs/workflow/CARDS.md` |

**What you can see at the end:** `fh claim A01-12 --push` claims the card, and `git log` on GitHub shows a commit `fh: claim A01-12` from that agent, with nothing else in it.

## Details for the worker

- **Outcome.** An agent records a status change and shares it with everyone in one command.
- **Decisive unknowns.** None.
- **Files owned.** `crates/cli/src/lib.rs`, `crates/cli/tests/fh.rs`, `crates/core/src/problem.rs`, `apps/web/src/i18n/*` (the two codes' words), `docs/workflow/CODING_RULES.md`, `docs/workflow/CARDS.md`, this card.
- **External operations.** None by the code itself. `--push` pushes to the user's own configured remote when they ask for it.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run check:i18n`.
  - Tests in a temporary Git repository: `--commit` makes one commit holding only seabag files, with the message and agent line; another changed file stays uncommitted; outside Git, `--commit` warns `commit_failed` and the change is kept; `--push` to a temporary bare remote lands the commit there.
  - This repository's own status changes from this card on use `--push`.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Owner tried it in a test folder ("passed" / output as expected) | owner (chat) |
| 2026-10-07 | "yes, write the card and build it" | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent and worked in the card run (`claude/a01-run`).

## Result (2026-10-07)

**In plain words.** `fh … --commit` now commits the change it just made, and `fh … --push` also pushes it. Only the seabag's files go into that commit, with the message `fh: <command> <id>` and an `Agent: <holder>` line; anything else you were editing is left alone. If Git refuses (for example outside a Git repository, or a push is rejected), the change stays saved and `fh` warns `commit_failed` or `push_failed` with Git's own words. Without the options, `fh` still never runs Git. CARDS.md now tells agents to add `--push` to every status change.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (command tests: 12, two new, in a real temporary Git repository with a local remote) | PASS |
| New tests: `--commit` commits only `task.json`, message `fh: claim A01-01` + `Agent: agent-a`; another changed file stays uncommitted; `--push` lands on the remote; a no-op change makes no commit; outside Git, `commit_failed` warning and the change is kept | PASS |
| `npm --prefix apps/web run check:i18n` (81 texts, 37 codes) | PASS |
| This repository: A01-12's own status changes made with `--push` | see the `fh:` commits on `main` |
- 2026-10-08 — **Owner verified** in a throwaway folder (`~/fh-try`).
