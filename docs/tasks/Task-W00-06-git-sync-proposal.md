# Task-W00-06-git-sync-proposal

*Split out of `Task-W00-01` on 2026-10-04; the proposal was written on 2026-10-01.*

- **Outcome.** The owner can decide whether glidebox projects should be stored like a Git repository — work locally, then push, pull and merge with a cloud copy — using a written proposal that sets out how it would work and what it costs.
- **Decisive unknowns.** The owner's decision: accept actual Git with structured files as the source of truth (which would supersede [0001](../decisions/0001-database-source-of-truth.md)), or keep a database (owner).
- **Files owned.** [decisions/0007](../decisions/0007-git-like-project-sync.md).
- **External operations.** None.
- **Done means mechanically.** 0007 has status Accepted or Rejected. If accepted, 0001 is marked Superseded, and PLAN and REQUIREMENTS R10 are updated to match.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-01 | Write the Git-like synchronization proposal after the owner's question | owner (chat) |
| 2026-10-04 | Approve option B: readable text files tracked by Git as the source of truth (accept 0007, supersede 0001) | owner (chat) |

## Notes

- 2026-10-04 — **Owner approved.** 0007 is now Accepted (storage foundation) and 0001 is Superseded. R10, PLAN, 0003, 0006, 0010, INTERFACES, AI-GENERATED-PROJECTS and `Task-W01-01` were updated. Sync and merge details remain proposals, to be prototyped in implementation cards.

- 2026-10-01 — First proposal, then refined after the owner asked about actual Git with structured text folders. It proposes Git as the canonical storage and the database as a rebuildable index, with stable-ID JSON/Markdown files and task-aware merge. It is not accepted; 0001 stays in force.
