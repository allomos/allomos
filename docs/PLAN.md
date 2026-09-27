# Glidebox — plan

> Status: **draft, 2026-09-27.** Decisions marked *proposed* are recommendations awaiting the owner's confirmation. Full records: [decisions/](./decisions/README.md); discussion: [2026-09-27 kickoff](./discussions/2026-09-27-kickoff.md).

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
- **Online collaboration**: teammates sign in, see live updates, and act on tasks.
- **Agents are first-class users**: Claude and Codex sessions claim, update and release tasks through a CLI / MCP server, cheaply and without merge conflicts.
- A **"waiting on me"** inbox for checks and approvals owed by a person.
- **Import** the existing SSR migration board as the first real dataset.

## 3. Non-goals (for now)

- Dispatching or running agents (glidebox tracks work; it does not start it). Possible later.
- Replacing git, PRs or CI. Glidebox links to them.
- Storing secrets, credentials or evidence logs. Cards link to evidence; they do not hold it.
- Full project-management features (budgets, time sheets, resource leveling).

## 4. Decisions

| # | Decision | Status |
|---|---|---|
| [D1](./decisions/0001-database-source-of-truth.md) | **The database is the source of truth.** Agents write structured fields through the CLI/MCP; markdown is an optional export, not an input. A one-time importer brings in existing markdown boards. | **accepted 2026-09-27** |
| [D2](./decisions/0002-burndown-counts-plus-sizes.md) | **Burndown uses task counts, plus optional sizes.** Every task counts as 1; a task may carry an optional size (points or days), and the chart can switch to size-weighted when sizes are present. | **accepted 2026-09-27** |
| [D3](./decisions/0003-relationship-to-glidelines.md) | **Relationship to Glidelines.** Leaning: standalone product, reusing Glidelines ideas and code where it fits (same stack family; port the `tasklist.json` format and burndown/forecast logic from `glidepace-vibe-cc`, not fork the app). Options A–D are compared in the decision record. | **open — not decided** |
| [D4](./decisions/0004-team-accounts-and-roles.md) | **Team accounts with roles**: owner, approver, contributor, viewer, and agent (token-based, scoped to a project). Invite-only; no public sign-up. | proposed |

## 5. Data model (first cut)

- **Project** — name, label ladder (configurable; SSR default: `implemented → locally verified → connected → deployed and verified`), forecast settings.
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
- **Actor** — person or agent session (`codex/w09-help-contact`, `claude/task-w07-01-09-…`), provider, owning person.

## 6. Views

1. **Workstream list** — tasks grouped by workstream, with state, label, claim and waiting-on; filters by state, actor and waiting-on.
2. **Board** — columns by state (or by label).
3. **Waiting on me** — every check/approval owed by the signed-in person; one click to approve, reject or mark checked.
4. **Progress charts** — per-workstream done/open bars, state distribution, throughput per week.
5. **Burndown + forecast** — see §7.
6. **Timeline** — gantt generated from claim and done events.
7. **Task detail** — fields, links, approvals and event history.
8. **Sessions** — which agent sessions hold which tasks right now (replaces the session registry).

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
  - `packages/importer` — markdown board → glidebox (first target: the SSR `TASK_BOARD.md` + `workstreams/`)
  - `supabase/` — Postgres schema, RLS policies, SQL functions (atomic claim), migrations
- **Backend:** Supabase — Postgres, Auth (team sign-in), Row Level Security per project and role, Realtime for live updates. Agents use scoped project tokens.
- **Hosting:** to decide (Vercel or Cloudflare).
- **Agent instructions:** a short `AGENTS.md` / `CLAUDE.md` snippet telling sessions to resume with `gb next` and report with `gb` instead of editing board markdown.

## 9. Milestones

| # | Milestone | Done when |
|---|---|---|
| M0 | Repo and plan | This plan agreed; repo pushed |
| M1 | Core + importer (local) | `packages/core` model, burndown and forecast tested; the SSR board imports with every card, workstream and label accounted for |
| M2 | Read-only web | Supabase schema + auth; web shows workstream lists, progress charts and burndown for the imported project |
| M3 | Agent write path | CLI + MCP: claim / label / wait / done / release, with atomic claims and an event log; one real Claude and one Codex session use it |
| M4 | Collaboration | Invites and roles, realtime updates, "waiting on me" inbox with recorded approvals |
| M5 | Polish | Generated timeline, markdown export, size-weighted burndown, target dates |

## 10. Open questions

1. Decide D3 (relationship to Glidelines) and confirm D4. M1 does not depend on D3.
2. Hosting target and domain.
3. Should the importer keep syncing from markdown during the transition, or run once and then switch the SSR project to glidebox?
4. Languages for the UI (Glidelines supports en / ja / zh-CN / zh-TW).

## 11. References

- SSR migration board: `/Users/wanghui/seamaple-work/smp-coding/products/jobs/jobs-dev-mig-to-ssr/next-ssr/docs/migration-plan-ssr/` (`TASK_BOARD.md`, `workstreams/`, `ESTIMATE.md`, `AI-WORKFLOW.md`)
- Glidelines: `/Users/wanghui/coding/glidepace/glidepace-vibe-cc` (`src/task-files/`, `src/lib/forecastDashboardUtils.ts`, `src/lib/forecastAssumptionsConfig.ts`, `public/task-files/task-list.schema.json`)
