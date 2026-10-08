# Task-A01-08-fh-fills-move-gaps

*Card type: coding. Area A01 (Tracking), Iteration 1. Follows [`Task-A01-07`](./Task-A01-07-own-cards-in-seabag.md), which had to work around four things `fh` could not do.*

## What gets built (plain words)

Moving FlipHopper's own cards into its seabag showed four gaps in the `fh` command. After this card, an agent can do all of them with `fh`, and nobody edits seabag JSON by hand.

| Piece | What it is | Where |
|---|---|---|
| `fh needs <id> [<text>]` | Sets the task's *next dependency* (the one thing needed for its next label); without text it clears it. | `crates/core` (`ops`), `crates/cli` |
| `fh ready <id>` | Marks an unheld `backlog` or `waiting` task `ready` (the owner approved it), clearing its *waiting on*. Today this takes a claim and a release. | `crates/core`, `crates/cli` |
| `fh drop <id>` | Marks a task `dropped`, clearing its holder and *waiting on*. | `crates/core`, `crates/cli` |
| `fh init --id <id> --labels <a,b,…>` | Sets the project ID (today the folder's name, which is wrong in a worktree) and the label ladder (today only the default). | `crates/store`, `crates/cli` |
| This repository | Backlog cards get their next dependency through `fh needs`; the "Next dependency when the card moved in" lines leave their descriptions. CARDS.md uses the new commands. | `.fliphopper/seabag/` (on `main`, with `fh`), `docs/workflow/CARDS.md`, `docs/workflow/CODING_RULES.md` |

**What you can see at the end:** `fh show A02-01` prints `next dependency: Iteration 5 (burndown and forecast)`, and the card rules no longer mention a workaround.

## Details for the worker

- **Outcome.** Every status field of a card can be set with `fh`, so agents never hand-edit the seabag.
- **Decisive unknowns.** None; the rules follow `fh`'s existing ones (a task held by someone else is refused; a done or dropped task cannot be reopened by these commands).
- **Files owned.** `crates/core/src/ops.rs`, `crates/core/src/problem.rs` (one code if needed), `apps/web/src/i18n/*` (its words), `crates/store/src/lib.rs` (`init`), `crates/cli/src/lib.rs`, `docs/workflow/CODING_RULES.md` (the `fh` contract), `docs/workflow/CARDS.md`, this repository's seabag (with `fh` only), this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run check:i18n`.
  - Unit tests cover each new command's success and refusal cases, and `init` with an ID and a ladder (and an empty ladder refused).
  - In this repository: `fh list --json` reports no issues, and no task description still holds a "Next dependency when the card moved in" line.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Owner tried it in a test folder ("passed" / output as expected) | owner (chat) |
| 2026-10-06 | "find some easy task to do for the next 6 hrs … find them and do them" — the agent picked this card | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent from the A01-07 findings.
- 2026-10-06 — Worked in the card run (`claude/a01-run`).

## Result (2026-10-06)

**In plain words.** `fh` can now set every status field of a card: `fh needs` sets or clears the next dependency, `fh ready` marks an approved backlog or waiting card ready, `fh drop` drops a card, and `fh init --id … --labels …` sets the project ID and its label ladder. The rules follow `fh`'s existing ones: a task someone else holds is refused, and a done or dropped task stays closed. One new problem code, `cannot_make_ready`, has words in three languages. CARDS.md and the `fh` contract in CODING_RULES use the new commands.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (core: new `needs_ready_and_drop`; store: new `init_with_own_id_and_ladder`; command: new `needs_ready_drop_and_init_options`) | PASS |
| `npm --prefix apps/web run check:i18n` (74 texts, 35 codes) | PASS |
- 2026-10-06 — Applied here with `fh needs`: the backlog cards' next dependencies moved from their descriptions into the field; `fh list --json` reports no issues.
- 2026-10-08 — **Owner verified** in a throwaway folder (`~/fh-try`).
