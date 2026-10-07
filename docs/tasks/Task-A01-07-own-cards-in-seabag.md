# Task-A01-07-own-cards-in-seabag

*Card type: coding. Area A01 (Tracking), Iteration 1. The last missing piece of the [v0.1 exit check](../product/PLAN.md#9-roadmap--releases): "FlipHopper's own cards now live in its seabag, and agents use `fh` instead of `TASK_BOARD.md`." Requested by the owner 2026-10-06.*

## What gets built (plain words)

Today FlipHopper's own work is tracked by hand in [TASK_BOARD.md](../TASK_BOARD.md). After this card, this repository has its own seabag (`.fliphopper/seabag/`), every card is a task in it, and agents claim, label, park and finish cards with `fh` instead of editing the board. Opening this repository in the FlipHopper app shows FlipHopper's own work by area.

| Piece | What it is | Where |
|---|---|---|
| The seabag | `fh init --name FlipHopper` at the repository root, the eight areas A00–A07 from [CARDS.md § Areas](../workflow/CARDS.md#areas), and one task per card (open and finished), with state, label, holder, waiting-on and next dependency copied from the board row. | `.fliphopper/seabag/` (new) |
| The move | A small script that reads the board's card rows and calls `fh add`, `fh claim`, `fh label`, `fh wait`, `fh done` (no hand-written JSON), plus a check that every board row became exactly one task with the same state and label. Kept so the move can be repeated if the board changes before it lands. | `scripts/` (new script) |
| Card detail | Each task's `description.md` links to its card file in `docs/tasks/`. The card files stay where they are and keep holding outcome, approvals and notes. | `.fliphopper/seabag/tasks/*/description.md` |
| Rules switch | AGENTS.md, CARDS.md and AI-WORKFLOW say: status lives in the seabag; resume with `fh next`; claim with `fh claim`; report with `fh label` / `fh wait` / `fh done`. TASK_BOARD.md shrinks to a short pointer plus the owner-question list and Now, or is generated (see unknowns). | `AGENTS.md`, `docs/workflow/CARDS.md`, `docs/workflow/AI-WORKFLOW.md`, `docs/TASK_BOARD.md`, `docs/AGENT_HANDOFF.md` |

**What you can see at the end:** open this repository's folder in FlipHopper and see every card grouped by area with the same state and label as the old board; run `fh next` here and get the next ready card; `git diff` of a claim shows one readable `task.json` change.

## Details for the worker

- **Outcome.** Agents working on FlipHopper track their cards with `fh` in this repository's seabag, and the owner sees the same status in the app; the Markdown board is no longer the home of status.
- **Decisive unknowns** (owner; the worker proposes, the owner chooses on review):
  1. **Labels for document and decision cards.** The board uses `drafted` → `owner accepted` for them, but a project has one label ladder. Recommended: one ladder `drafted`, `implemented`, `locally verified`, `deployed and verified`, `owner verified`, with `owner accepted` mapped to `owner verified`. Alternative: leave such cards without a label.
  2. **What stays in TASK_BOARD.md.** Recommended: keep it only for **Now** and **Waiting on the owner** (which have no seabag field yet), with a line saying card status is in the seabag. Alternative: generate it from the seabag (needs an export command, Iteration 12).
  3. **Holder names.** Recommended: the session's branch name (`claude/a01-07-…`), as the Sessions table does today.
  4. **Recently finished history.** `fh` records no dates yet. Recommended: finished cards move in as `done`; the dated history stays in the card files and git log.
- **Not in scope.** New `fh` commands or file-format fields. If the move needs one (for example setting `next_dependency`, which `fh` cannot set today), the worker stops and asks: either a small A01 card for it, or the board keeps that column until then.
- **Files owned.** `.fliphopper/seabag/` (new); `scripts/` move script (new); `AGENTS.md`, `CLAUDE.md` only if its pointer changes; `docs/workflow/CARDS.md`, `docs/workflow/AI-WORKFLOW.md`, `docs/TASK_BOARD.md`, `docs/AGENT_HANDOFF.md`; this card.
- **Shared-file caution.** The board is edited by every session. Do the move in one commit when no other card is claimed, or re-run the script on the latest `main` just before merging.
- **External operations.** None.
- **Done means mechanically.**
  - The check script reports every board card present once in the seabag with matching state and label; `fh list` shows them.
  - `cargo test --workspace` still passes (no code changes expected); `git diff --check` is clean; all relative links resolve.
  - The built app, opened on this repository, shows the cards by area; a demo claim → label → release with `fh` shows in the app and as a readable `git diff`. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | Owner check passed ("passed") | owner (chat) |
| 2026-10-06 | Write a card for moving FlipHopper's own cards into a seabag | owner (chat) |
| 2026-10-06 | Card reviewed; all four recommended answers accepted (one combined label ladder with `owner accepted` → `owner verified`; TASK_BOARD.md keeps only Now and Waiting on the owner *for now*; holder = session branch name; finished cards move in as `done` *for now*) | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent for the owner's review. The v0.1 exit check also asks for a real Claude Code session to claim and finish a task with `fh` while the app shows it; the first card worked after this move does that naturally.
- 2026-10-06 — **Owner answered the four unknowns: the recommendations, as listed under Approvals.** On the board, the owner added: *in the end TASK_BOARD.md should be gone, or generated from FlipHopper every time*. So the trimmed board is a stopgap; a later card (likely with the Markdown export, Iteration 12, or earlier if Now and owner questions get a seabag home) removes or generates it. Card is ready.

## Result (2026-10-06)

**In plain words.** FlipHopper now tracks its own work. This repository has a seabag with eight areas and all 32 cards, each with the state, label and holder the board had. A script did the move through `fh`, and checked afterwards that every card matches. The board page now keeps only Now and the owner's questions; the rules in AGENTS.md, CARDS.md, AI-WORKFLOW and the handoff say to change status with `fh`, on `main` only.

- **How status changes now:** run `fh` against the main checkout's seabag (`FLIPHOPPER_SEABAG=<main checkout>`), commit only that change on `main`, push. Card branches carry an old copy of the seabag and never change it, so merging a card branch never undoes a status change.
- **Two things `fh` cannot do yet**, worked around and recorded here (a follow-up card adds them):
  - set the project ID and the label ladder: the script wrote both into `project.json` (`fliphopper`; `drafted` … `owner verified`);
  - set a task's *next dependency*: for waiting tasks it is their *waiting on*; for backlog and ready tasks the board's text went into `description.md`.
- **Mapping:** finished cards labelled owner verified / accepted / confirmed / decided → `owner verified`; three finished cards whose board text named no owner check (A00-02, A00-05, A00-16) → `done` without a label. Every task's `description.md` links to its card file.
- **Script:** [scripts/move-board-to-seabag.py](../../scripts/move-board-to-seabag.py) (`--check` compares only).

| Check | Result |
|---|---|
| `move-board-to-seabag.py --check` against the board before it was trimmed | PASS: 32 board cards, 0 differences |
| `fh list --json` | PASS: no `issues`; `fh next` → A00-19 |
| Demo `fh claim` → `label implemented` → `release --note` on A01-05, then `git diff` | PASS: one `task.json` changes, two readable added lines; demo reverted |
| `cargo test --workspace`, `git diff --check`, relative links | PASS |
| Built app (`tauri build --debug`) opened on this repository | Built and started; **not seen by the agent** (no screen access) — owner check |

**Owner check:** open this repository's folder (`/Users/wanghui/coding/glidepace/fliphopper`) in FlipHopper. Expected: areas A00–A07 (A06 empty), 32 cards, open ones with their labels; A01-07 shows as waiting on this check.
- 2026-10-07 — **Owner verified** in the built app ("passed").
