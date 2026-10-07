# Task-A01-09-v0-1-exit-check

*Card type: verification. Area A01 (Tracking), Iteration 1. The [v0.1 exit check](../product/PLAN.md#9-roadmap--releases) says how we know Iteration 1 is usable. This card records each check with its evidence, so the owner can declare v0.1 done or name what is missing.*

## What gets built (plain words)

No code. A table of the five v0.1 exit checks: what proves each one, and who still has to look.

## Details for the worker

- **Outcome.** The owner sees in one place whether v0.1 (macOS local tracker) is usable as PLAN § 9 defines it.
- **Decisive unknowns.** Whether the owner counts this session's own use of `fh` (FlipHopper's cards, 2026-10-06/07) as "a real Claude Code session claims and finishes a task with `fh`" (owner).
- **Files owned.** This card.
- **External operations.** None.
- **Done means mechanically.** Every exit check has evidence or a named missing step; the owner accepts or names what is missing.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "find some easy task to do for the next 6 hrs … find them and do them" — the agent picked this card | owner (chat) |

## Notes

- 2026-10-07 — Card written and checked by the agent.

## The v0.1 exit check (2026-10-07)

| # | Check (PLAN § 9) | Evidence | Still needed |
|---|---|---|---|
| 1 | Create the seabag in a new folder, and inside a Git repository | App creates it in an empty folder ([`A01-05`](./Task-A01-05-readme-in-new-folder.md) built-app runs; [`A01-01`](./Task-A01-01-walking-skeleton.md)); `fh init` inside a Git repository, this repository's own seabag included ([`A01-03`](./Task-A01-03-gb-command.md), [`A01-07`](./Task-A01-07-own-cards-in-seabag.md)) | — |
| 2 | Add areas and tasks with the CLI, and see them in the app | `fh add area` / `fh add task` built this repository's 8 areas and 34 tasks; the app follows `fh` changes ([`A01-03`](./Task-A01-03-gb-command.md) end-to-end) | — (owner passed A01-07's check, 2026-10-07) |
| 3 | A real Claude Code session claims and finishes a task with `fh`, and the app shows it | This Claude Code session (`claude/a01-run`) claimed, labelled, parked and released A01-07, A01-05, A01-04, A01-08, A00-19, A04-01, A03-01 and this card with `fh`, each committed on `main` (`git log -- .fliphopper`) | owner confirms this counts |
| 4 | `git diff` shows readable changes | A claim → label → release shows as two added lines in one `task.json` ([`A01-07`](./Task-A01-07-own-cards-in-seabag.md) Result); every `fh:` commit on `main` is a small JSON diff | — |
| 5 | FlipHopper's own cards live in its seabag, and agents use `fh` instead of `TASK_BOARD.md` | [`A01-07`](./Task-A01-07-own-cards-in-seabag.md); rules in [CARDS.md](../workflow/CARDS.md#where-status-lives) | — (A01-07 owner verified 2026-10-07) |

**In plain words.** Every v0.1 check has evidence. What is left is the owner's own look: opening this repository in the app (A01-07), and agreeing that this session's use of `fh` counts as check 3. After that, v0.1 can be called done. A packaged release (an installable `.app` someone else can download) is not part of the exit check; PLAN says each iteration ends in a release, and publishing one is an external operation that needs its own approval.

- 2026-10-07 — The owner passed A01-07, A01-05 and A01-04 in the app. Only check 3's question is left.
