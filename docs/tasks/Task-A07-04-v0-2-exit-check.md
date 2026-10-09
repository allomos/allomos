# Task-A07-04-v0-2-exit-check

*Card type: verification. Area A07 (Platform & release), Iteration 2 (last card). Written 2026-10-09 at the owner's request for the rest of v0.2. Like [`A01-09`](./Task-A01-09-v0-1-exit-check.md) for v0.1. [PLAN § 9](../product/PLAN.md#9-roadmap--releases): "Iteration 2's exit check runs on a workmate's Windows machine".*

## What gets built (plain words)

v0.2 is finished when it works for real on both systems. The agent prepares the two builds and a short, numbered **check list** in plain words. The owner runs it on the Mac; a workmate installs the Windows build and runs the same list on Windows. Their results are recorded here, and anything that fails becomes a fix before v0.2 is called done.

| Piece | What it is | Where |
|---|---|---|
| The builds | The macOS and Windows builds from the pipeline ([`A07-02`](./Task-A07-02-build-pipeline.md)), from the same commit. | GitHub Actions run |
| The check list | About 10 steps, each with what to do and what should happen: install (including the Windows "unknown publisher" warning), open a folder, see tasks by area and by iteration, open a task, edit it, add a task, see History, use `fh` in a terminal. | this card |
| The results | Pass or fail for every step on each system, with notes and screenshots where something failed. | this card |

**What you can see at the end:** this card lists every step as passed on the Mac and on Windows, and v0.2 is the first release that runs on both.

## Details for the worker

- **Outcome.** v0.2 is proven usable on macOS and Windows by people other than the agent.
- **Needs first.** [`A01-17`](./Task-A01-17-change-history.md), [`A01-18`](./Task-A01-18-edit-task-in-app.md), [`A01-19`](./Task-A01-19-create-in-app.md), [`A07-02`](./Task-A07-02-build-pipeline.md), [`A07-03`](./Task-A07-03-windows-works.md) done.
- **Decisive unknowns.**
  - Who the workmate is and how they receive the build: the owner arranges it; the agent sends nothing to anyone.
- **Files owned.** This card (check list and results).
- **External operations.** None by the agent. The owner shares the Windows build with the workmate.
- **Done means mechanically.** Every step recorded as passed on both systems, or each failure linked to a fix card that is done.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the other v0.2 cards.
