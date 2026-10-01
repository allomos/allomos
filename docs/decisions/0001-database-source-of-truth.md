# 0001 — The database is the source of truth

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-09-27 | 2026-09-27 | will.w.hui |

## Context

The reference board (Joyns SSR migration, see [analysis](../analysis/ssr-task-board.md)) keeps state in Markdown written by agents. Status is prose and is repeated in several places. A previous attempt to visualize it — Glidelines' `convert-ssr-plan.py` — is a one-shot converter tied to the board's exact wording; it refuses to run when the text changes and produces a snapshot, not a live view.

## Decision

Glidebox's **database is the source of truth**. People and agents change tasks through the web app, the `gb` CLI or the MCP server, which write **structured fields** (state, label, claim, waiting-on, size) and append an **event** for every change. Markdown is an optional **export**, never an input. Existing Markdown boards come in once through an **importer**.

## Consequences

- Burndown, timeline, "recently finished" and the session registry are computed from events; no one maintains them by hand.
- Claims become atomic database operations; no git merge conflicts over who holds a card.
- Under [0006](./0006-local-and-cloud-projects.md), a local project’s database lives on the user’s machine; a cloud project’s database is hosted. Local agent status updates must not require cloud authentication or network access. Cloud agents need authorized access; offline cloud updates remain a separate, undecided design.
- The database decision governs structured storage, not mandatory hosting. Each project has one authoritative database; local/cloud synchronization is not implied.
- Agent instructions in projects (`AGENTS.md` / `CLAUDE.md`) must change from "edit TASK_BOARD.md" to "use `gb`".
- Narrative detail (outcome, evidence, decisions) can still live in repo files; glidebox links to them.
- [R50–R51](../product/REQUIREMENTS.md#ai-generated-project-setup), accepted 2026-10-01, add AI-generated project/task files as validated creation/import input. The statement that Markdown is not live input excludes hand-maintained prose status as the source of truth; it does not prohibit controlled generated-file import. Live storage remains governed by this decision until explicitly superseded.

## Alternatives considered

- **Markdown stays the truth; glidebox parses the repo.** Rejected: prose status is fragile to parse (as the Glidelines converter shows), and it keeps the duplication and merge conflicts.
- **Two-way sync.** Rejected for now: two sources of truth recreate the drift problem.
