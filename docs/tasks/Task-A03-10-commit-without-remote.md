# Task-A03-10-commit-without-remote

*Card type: coding. Area A03 (Agents). Written 2026-10-12 at the owner's request. **Dropped** the same day: the sentence was taken back out.*

*Background:* Allomos's instructions tell agents to add `--push` to every status change; in a repository with no remote (for example one the setup wizard just made with `git init`, [`A05-03`](./Task-A05-03-taskbag-setup-wizard.md)), every such change ends with a `push_failed` warning.*

## What gets built (plain words)

The instructions agents read say one more thing: **when the taskbag's Git repository has no remote, use `--commit` instead of `--push`**. `--commit` commits the same taskbag files without trying to push, so no warning appears. Where it is said:

| Where | What agents read |
|---|---|
| `alm brief` (`agent_section` in `crates/store/src/lib.rs`) | The short section a project's `AGENTS.md` carries |
| `alm guide` (`crates/store/src/guide.rs`) | The longer guide |
| This repository's `AGENTS.md` | Its copy of the short section |

## Details for the worker

- **Outcome.** An agent following the instructions in a repository without a remote commits status changes without a `push_failed` warning.
- **Decisive unknowns.** None.
- **Files owned.** `crates/store/src/lib.rs` (section text), `crates/store/src/guide.rs`, `AGENTS.md` (its copy of the section).
- **External operations.** None.
- **Done means mechanically.** `alm brief` and `alm guide` print the `--commit` sentence; a store test checks it; `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` pass.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Tell agents to use `--commit` when the repository has no remote | owner (chat: "ok") |

## Notes

- 2026-10-12 — Card written from the owner's request.
- 2026-10-12 — Done: the sentence "When the taskbag's Git repository has no remote (`git remote` prints nothing), use `--commit` instead" is in `agent_section` (`alm brief`), in the guide's rules (`alm guide workflow`) and in this repository's `AGENTS.md`. Test `agents_are_told_to_commit_when_there_is_no_remote`. Checks: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` — PASS. Projects whose `AGENTS.md` already has the older section get the new sentence when their section is renewed ([`A03-07`](./Task-A03-07-renew-agent-instructions.md)).
- 2026-10-12 — **Dropped.** On reading `alm brief`, the owner judged the sentence unnecessary: without it nothing breaks (`alm … --push` still commits, then only warns `push_failed`), and a capable agent adapts by itself; every line of the section is read by every session. The sentence and its test were removed from `crates/store/src/lib.rs`, `crates/store/src/guide.rs` and `AGENTS.md` (restored as before the merge `cbcfcef`). Not chosen: making `alm --push` stay quiet when there is no remote.
