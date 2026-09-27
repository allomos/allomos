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
- Agents need access to glidebox (network + a project token) to report status. Offline work needs a fallback (queue and replay, or report later).
- Agent instructions in projects (`AGENTS.md` / `CLAUDE.md`) must change from "edit TASK_BOARD.md" to "use `gb`".
- Narrative detail (outcome, evidence, decisions) can still live in repo files; glidebox links to them.

## Alternatives considered

- **Markdown stays the truth; glidebox parses the repo.** Rejected: prose status is fragile to parse (as the Glidelines converter shows), and it keeps the duplication and merge conflicts.
- **Two-way sync.** Rejected for now: two sources of truth recreate the drift problem.
