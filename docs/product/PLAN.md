# Glidebox — plan

> Status: **draft, 2026-09-27.** Decisions marked *proposed* are recommendations awaiting the owner's confirmation. Full records: [decisions/](../decisions/README.md); requirements: [REQUIREMENTS.md](./REQUIREMENTS.md); discussions: [2026-09-27 kickoff](../discussions/2026-09-27-kickoff.md), [2026-09-30 docs and agent workflow](../discussions/2026-09-30-docs-and-agent-workflow.md). Glidebox is developed by AI agents following [AGENTS.md](../../AGENTS.md).

## 1. Problem

Work is carried out by several AI coding sessions (Claude Code, Codex) in parallel worktrees, plus the owner and teammates. Today the state of that work lives in hand-maintained markdown (reference: the Joyns SSR migration `TASK_BOARD.md` + `workstreams/` card files). That approach has these costs:

1. **Status is duplicated** across the card file, card index, Now, Timeline gantt, recently-finished table and workstream table — agents must keep six copies in sync, and they drift.
2. **Status is prose.** Labels such as "locally verified; owner verified, Done (test Worker not redeployed)" cannot be counted, filtered or charted.
3. **The owner is the bottleneck but has no queue.** Most open cards wait on an owner check or an exact approval; that is scattered across "Next dependency" cells.
4. **Claiming is manual.** Sessions claim cards by committing to a registry and resolving merge conflicts.
5. **No forecast.** Estimates exist only per workstream in person-weeks and are not tied to cards; the full scope (roadmap cards) is not on the board; nothing answers "when will we finish?"
6. **Expensive context.** Agents read tens to hundreds of KB of markdown to resume.
7. **No collaboration.** Teammates cannot see or update anything outside the repo.

## 2. Goals

- See every task's **state and progress**, **grouped by workstream**, as **lists and charts**.
- A **burndown chart** with an **estimated finish date** (a range, not a single guess).
- **Purely local projects**: project and task information stays on the user’s machine; no sign-in or hosted service is required for tracking.
- **Optional cloud projects**: users choose cloud storage and collaboration; teammates sign in, see live updates, and act on tasks.
- **Agents are first-class users**: Claude and Codex sessions claim, update and release tasks through a CLI / MCP server, cheaply and without merge conflicts.
- A **"waiting on me"** inbox for checks and approvals owed by a person.
- **Import** the existing SSR migration board as the first real dataset.
- **Start a task in a chosen AI agent** (Claude Code, Codex) from the app, and **jump into** an already-started task's agent session ([D5](../decisions/0005-start-and-open-agent-sessions.md)).

## 3. Non-goals (for now)

- Orchestrating agents automatically (scheduling, auto-assigning, chaining agents). Glidebox starts an agent only when a person asks it to.
- Replacing git, PRs or CI. Glidebox links to them.
- Storing secrets, credentials or evidence logs. Cards link to evidence; they do not hold it.
- Full project-management features (budgets, time sheets, resource leveling).

## 4. Decisions

| # | Decision | Status |
|---|---|---|
| [D1](../decisions/0001-database-source-of-truth.md) | **The database is the source of truth.** Agents write structured fields through the CLI/MCP; markdown is an optional export, not an input. A one-time importer brings in existing markdown boards. | **accepted 2026-09-27** |
| [D2](../decisions/0002-burndown-counts-plus-sizes.md) | **Burndown uses task counts, plus optional sizes.** Every task counts as 1; a task may carry an optional size (points or days), and the chart can switch to size-weighted when sizes are present. | **accepted 2026-09-27** |
| [D3](../decisions/0003-relationship-to-glidelines.md) | **Relationship to Glidelines.** Leaning: standalone product, reusing Glidelines ideas and code where it fits (same stack family; port the `tasklist.json` format and burndown/forecast logic from `glidepace-vibe-cc`, not fork the app). Options A–D are compared in the decision record. | **open — not decided** |
| [D4](../decisions/0004-team-accounts-and-roles.md) | **Cloud team accounts with roles**: owner, admin, approver, contributor, viewer, and agent (token-based, scoped to a project). **An owner or admin adds members by email address**; no public sign-up. | **roles and email invites accepted 2026-09-27**; role details proposed |
| [D5](../decisions/0005-start-and-open-agent-sessions.md) | **Start and open agent sessions from glidebox**, through a local runner daemon (`gb runner`) on a person's machine or a vendor cloud session. | **requirement accepted 2026-09-27**; design proposed |
| [D6](../decisions/0006-local-and-cloud-projects.md) | **Local-only and optional cloud projects.** Local tracking stores data on the user’s machine without sign-in; cloud projects require sign-in. Storage technology and transfer/sync design remain open. | **requirement accepted 2026-10-01** |

## 5. Data model (first cut)

- **Project** — storage mode (`local | cloud`), name, label ladder (configurable; SSR default: `implemented → locally verified → connected → deployed and verified`), forecast settings.
- **Workstream** — `id` (e.g. `W07`), title, order, optional estimate (low / likely / high).
- **Task** — hierarchical id (`W07-01-09`), title, workstream, parent, and:
  - `state`: `backlog | ready | in_progress | waiting | done | dropped`
  - `label`: current step on the project's ladder
  - `claimed_by`: one actor (agent session or person), with claim time
  - `waiting_on`: what blocks the next step — `owner_check | approval | task | external` + who + note
  - `size` (optional), `depends_on[]`, `links[]` (card file, branch, PR, commits, deploy URL)
  - narrative fields: outcome, done-when (markdown, short)
- **Approval** — task, exact operation requested, requester, decided by, decision, time.
- **Event** (append-only) — every change of state, label, claim, size or scope, with actor and timestamp. **Burndown, timeline and "recently finished" are all computed from events**, never maintained by hand.
- **Member** — person or agent in a project, with a role; **Invitation** — email, role, inviter, expiry, state.
- **Runner** — a registered machine (local daemon) or cloud connection that can launch agents; owner, allowed projects/repos/members, liveness.
- **Agent session** — agent (Claude / Codex), runner, task, session id, worktree, branch, resume command or URL, state (running / waiting for input / exited).
- **Actor** — person or agent session (`codex/w09-help-contact`, `claude/task-w07-01-09-…`), provider, owning person.

## 6. Views

1. **Workstream list** — tasks grouped by workstream, with state, label, claim and waiting-on; filters by state, actor and waiting-on.
2. **Board** — columns by state (or by label).
3. **Waiting on me** — every check/approval owed by the signed-in person; one click to approve, reject or mark checked.
4. **Progress charts** — per-workstream done/open bars, state distribution, throughput per week.
5. **Burndown + forecast** — see §7.
6. **Timeline** — gantt generated from claim and done events.
7. **Task detail** — fields, links, approvals and event history.
8. **Sessions** — which agent sessions hold which tasks right now, and whether they are alive (replaces the session registry).
9. **Start in agent / Open session** — buttons on every task (see [D5](../decisions/0005-start-and-open-agent-sessions.md)).

## 7. Burndown and forecast

- **Lines:** remaining tasks and total scope over time (scope growth is visible, not hidden); the size-weighted variant when sizes exist.
- **Forecast by throughput:** sample the tasks finished per week over the last N weeks (default 4–6) and run a Monte Carlo simulation of the remaining count → finish dates at **50% / 85%** confidence, drawn as a cone.
- **Forecast by estimate** (secondary): remaining size ÷ capacity, with remaining percentage by state (as in Glidelines: to do 100%, in progress 50%, waiting 20%).
- **Missing history:** for imported tasks without completion dates, reuse Glidelines' reconstructed-band approach instead of inventing dates.
- Optional target date line and "on track / at risk" indicator.

## 8. Architecture (proposed)

- **Monorepo** (pnpm, TypeScript):
  - `packages/core` — domain model, validation, burndown and forecast (pure, unit-tested; shared by all others)
  - `apps/web` — React + Vite + Tailwind web app (charts, lists, inbox)
  - `packages/cli` — `gb` command for people and agents (`gb claim`, `gb label`, `gb wait`, `gb done`, `gb release`, `gb next`)
  - `packages/mcp` — MCP server exposing the same operations to Claude Code and Codex
  - `packages/runner` — local daemon (`gb runner`): receives start requests, creates worktrees, launches and resumes agents, reports session state
  - `packages/importer` — markdown board → glidebox (first target: the SSR `TASK_BOARD.md` + `workstreams/`)
  - `supabase/` — Postgres schema, RLS policies, SQL functions (atomic claim), migrations
- **Storage modes:** local projects use a persistent database on the user’s machine; cloud projects use a hosted database and require sign-in. The domain model and event rules must work in both modes. Local tracking cannot depend on cloud authentication or hosted services. Local database technology and packaging remain open.
- **Cloud backend (proposed):** Supabase — Postgres, Auth (cloud sign-in), Row Level Security per project and role, Realtime for live updates. Cloud agents use scoped project tokens; local agent access must work without a cloud token.
- **Transfer and sync:** moving/copying a local project to the cloud, offline cloud edits, and two-way synchronization remain open; do not assume automatic uploads.
- **Hosting:** to decide (Vercel or Cloudflare).
- **Agent instructions:** a short `AGENTS.md` / `CLAUDE.md` snippet telling sessions to resume with `gb next` and report with `gb` instead of editing board markdown.

## 9. Milestones

| # | Milestone | Done when |
|---|---|---|
| M0 | Repo and plan | This plan agreed; repo pushed |
| M1 | Core + importer (local) | `packages/core` model, burndown and forecast tested; the SSR board imports with every card, workstream and label accounted for |
| M2 | Local and cloud read-only views | Local imported project persists and shows lists, charts and burndown without sign-in or hosted services; signed-in cloud projects show the same views through the proposed hosted backend |
| M3 | Agent write path | CLI + MCP: claim / label / wait / done / release, with atomic claims and an event log; one real Claude and one Codex session use it |
| M4 | Collaboration | Roles, owner/admin invites by email, realtime updates, "waiting on me" inbox with recorded approvals |
| M5 | Start and open agents | Research spike on each agent's launch/resume interfaces; local runner starts a task in Claude Code and in Codex, reports the session and claims the task; "Open session" works for local and cloud sessions |
| M6 | Polish | Generated timeline, markdown export, size-weighted burndown, target dates |

## 10. Open questions

Kept in one place: [TASK_BOARD § Waiting on the owner](../TASK_BOARD.md#waiting-on-the-owner).

## 11. References

[REFERENCES.md](../REFERENCES.md) — the SSR migration board and Glidelines, with the files that matter.
