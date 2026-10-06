# 0001 — The database is the source of truth

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Superseded** by [0007](./0007-git-like-project-sync.md) on 2026-10-04 | 2026-09-27 | 2026-09-27 | will.w.hui |

> **Superseded 2026-10-04.** The owner approved storing each project as readable text files tracked by Git ([0007](./0007-git-like-project-sync.md)); a database is now only a rebuildable index. What still holds from this record: status is structured fields, not prose; it has one home; every change is recorded as an event; and agents write it through the CLI/MCP. Hand-maintained Markdown boards are still only an import source.

## Context

The reference board (Joyns SSR migration, see [analysis](../analysis/ssr-task-board.md)) keeps state in Markdown written by agents. Status is prose and is repeated in several places. A previous attempt to visualize it — Glidelines' `convert-ssr-plan.py` — is a one-shot converter tied to the board's exact wording; it refuses to run when the text changes and produces a snapshot, not a live view.

## Decision

FlipHopper's **database is the source of truth**. People and agents change tasks through the web app, the `fh` CLI or the MCP server, which write **structured fields** (state, label, claim, waiting-on, size) and append an **event** for every change. Markdown is an optional **export**, never an input. Existing Markdown boards come in once through an **importer**.

## Consequences

- Burndown, timeline, "recently finished" and the session registry are computed from events; no one maintains them by hand.
- Claims become atomic database operations; no git merge conflicts over who holds a card.
- Under [0006](./0006-local-and-cloud-projects.md), a local project’s database lives on the user’s machine; a cloud project’s database is hosted. Local agent status updates must not require cloud authentication or network access. Cloud agents need authorized access; offline cloud updates remain a separate, undecided design.
- The database decision governs structured storage, not mandatory hosting. Each project has one authoritative database; local/cloud synchronization is not implied.
- Agent instructions in projects (`AGENTS.md` / `CLAUDE.md`) must change from "edit TASK_BOARD.md" to "use `fh`".
- Narrative detail (outcome, evidence, decisions) can still live in repo files; FlipHopper links to them.
- [R50–R51](../product/REQUIREMENTS.md#ai-generated-project-setup), accepted 2026-10-01, add AI-generated project/task files as validated creation/import input. The statement that Markdown is not live input excludes hand-maintained prose status as the source of truth; it does not prohibit controlled generated-file import. Live storage remains governed by this decision until explicitly superseded.

## Alternatives considered

- **Markdown stays the truth; FlipHopper parses the repo.** Rejected: prose status is fragile to parse (as the Glidelines converter shows), and it keeps the duplication and merge conflicts.
- **Two-way sync.** Rejected for now: two sources of truth recreate the drift problem.
