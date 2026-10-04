# 0008 — Build glidebox ourselves, not on an open-source base

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-01 | 2026-10-04 | will.w.hui |

## In plain words

Other tools already do part of what glidebox will do. The research recommended looking at one of them, Backlog.md, as a starting point that we could copy and extend (a "fork"). The owner decided against that: **we write glidebox's own code**. Other projects remain useful for learning; we read them, but we don't build on them.

## Context

The [competitor investigation](../research/competitor-investigation.md) found that glidebox overlaps substantially with existing trackers and agent workspaces. On 2026-10-01 the owner said glidebox may be sold as a service. The research then recommended evaluating **Backlog.md** (MIT licence) as a possible base, and named veggiemonk/backlog (Apache-2.0) and git-bug (GPLv3) as other candidates.

## Decision (owner, 2026-10-04)

- **Build our own.** Glidebox's code is written for glidebox. Backlog.md, backlog, git-bug and other open-source trackers are **not** used as a base, fork or embedded core.
- They stay **references**. We may read their public documentation and source to learn designs, such as task-file layout, distributed history and agent interfaces. We do not copy their code.

## Consequences

- [`Task-A02-01`](../tasks/Task-A02-01-burndown-and-forecast.md), our own core model with burndown and forecast, remains the first code card.
- Copying source from these projects requires a new owner decision and a licence review. The GPL (git-bug) in particular has obligations that apply when distributing.
- Ordinary third-party libraries (for example a chart library or a Git library) are still allowed. Each is chosen and pinned in the card that introduces it, with its licence noted.
- **Not decided by this record:**
  - Reuse of the owner's own Glidelines code: [0003](./0003-relationship-to-glidelines.md), still open.
  - Whether glidebox is sold, and under which source licence: on the board's "Waiting on the owner" list.
