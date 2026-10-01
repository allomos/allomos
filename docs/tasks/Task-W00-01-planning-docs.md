# Task-W00-01-planning-docs

- **Outcome.** The owner and any new agent session can understand what glidebox is, what has been decided, what is still open, and how to work on it — by reading `AGENTS.md`, the board and the documents they link, without the original conversation.
- **Decisive unknowns.** Whether the owner accepts the plan, requirements, workflow and handoff rules as written (owner).
- **Files owned.** `AGENTS.md`, `CLAUDE.md`, `README.md`, `.gitignore`, everything under `docs/`.
- **External operations.** None.
- **Done means mechanically.** Every relative link in the repo resolves; `git diff --check` is clean; the owner has reviewed and accepted (or corrected) the documents.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-09-27 | Plan and decision records; writing the discussion down as documents | owner (chat) |
| 2026-09-30 | Writing and organizing all necessary documents; agent handoff rules modeled on the SSR project's `AGENT_HANDOFF.md` | owner (chat) |
| 2026-10-01 | Record the owner’s requirement for purely local projects and optional signed-in cloud projects; update related planning documents locally | owner (chat) |

| 2026-10-01 | Save the competitor investigation, feature matrix and sources in the repository | owner (chat) |

## Notes

- 2026-09-27 — Plan, decisions 0001–0005, kickoff discussion and SSR board analysis written ([discussion](../discussions/2026-09-27-kickoff.md)).
- 2026-09-30 — Documents reorganized (`docs/product`, `docs/workflow`, `docs/tasks`); agent guide, board, handoff, requirements, glossary and references added ([discussion](../discussions/2026-09-30-docs-and-agent-workflow.md)).

- 2026-10-01 — Added R40–R43 and [0006](../decisions/0006-local-and-cloud-projects.md); revised the plan and earlier database/account decisions so local tracking needs no cloud account or service. Storage technology, packaging and transfer/sync remain undecided.

- 2026-10-01 — PASS: all relative Markdown link paths resolve; instruction consistency reviewed; `git diff --check` clean. Runtime checks NOT RUN: documentation only, no app implementation. The remaining planning review stays on the board.

- 2026-10-01 — Saved [competitor investigation](../research/competitor-investigation.md), including the requested feature-first matrix with Glidebox last, dated sources, unverified cells and local/cloud requirements. No fresh competitor tests or implementation were performed.

- 2026-10-01 — Research documentation checks PASS: relative paths resolve, instruction consistency reviewed, `git diff --check` clean. Competitor runtime testing NOT RUN. Git-like synchronization raised by the owner afterward remains a design discussion, not an accepted implementation.

- 2026-10-01 — Recorded [Git-like project synchronization proposal](../decisions/0007-git-like-project-sync.md) after the owner’s question. Push/pull, field conflicts, offline claim limitations and approval validation are proposals, not accepted implementation choices. Documentation link and whitespace checks PASS; prototype/runtime checks NOT RUN.

- 2026-10-01 — Refined 0007 after the owner asked about actual Git with structured text folders: proposed Git as canonical storage, database as rebuildable index, stable-ID JSON/Markdown layout and domain-aware merge. 0001 remains accepted pending explicit supersession. Documentation checks PASS; prototype NOT RUN.

- 2026-10-01 — Extended competitor evidence with actual Git-backed examples: MrLesk/Backlog.md, git-bug, veggiemonk/backlog and Obsidian Git. Maintainer sources checked, no tools installed or products tested. Relative links and whitespace checks PASS.

- 2026-10-01 — At the owner’s request, consolidated the original and Git-backed comparisons into one feature-first table: 10 competitors plus Glidebox last, with storage/sync/index rows and source register entries 16–20. Removed the duplicate follow-up table. PASS: column consistency, relative document paths and `git diff --check`; product tests NOT RUN.

- 2026-10-01 — Recorded possible paid-service intent and preliminary reuse/license evidence in competitor research: Backlog.md MIT, veggiemonk/backlog Apache-2.0, git-bug GPLv3. Recommendation is bounded evaluation before selecting a fork. PASS: document link/whitespace checks; architecture, dependency audit and prototype NOT RUN.
