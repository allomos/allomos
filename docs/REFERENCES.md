# References

External material glidebox learns from. **All of it is read-only**: read and quote, never edit, commit to, or run write operations in these projects. Read only the named files; re-read before quoting counts or rules (they change).

## SSR migration board — the motivating project

`/Users/wanghui/seamaple-work/smp-coding/products/jobs/jobs-dev-mig-to-ssr/` — the owner's Joyns migration, worked by several Claude and Codex sessions in parallel.

| File (under `next-ssr/docs/migration-plan-ssr/` unless noted) | Why it matters |
|---|---|
| `TASK_BOARD.md` | The board: Now, session registry, waiting on the owner, card index, timeline. Import target (W02). |
| `workstreams/Task-*.md`, `workstreams/readme.md`, `workstreams/W00…W15-*.md` | Card files and workstream index — the data model glidebox must represent. |
| `AGENT_HANDOFF.md` | Handoff format glidebox's own [AGENT_HANDOFF.md](./AGENT_HANDOFF.md) is modeled on. |
| `AI-WORKFLOW.md` | Roles, effort, handoff timing, new-session rule — adapted in [workflow/AI-WORKFLOW.md](./workflow/AI-WORKFLOW.md). |
| `ESTIMATE.md` §4 | Workstream estimates in person-weeks — shows why card-level forecasting was missing. |
| repo root `AGENTS.md`, `CLAUDE.md` | Working procedure, gates and labels — adapted in [AGENTS.md](../AGENTS.md). |
| `next-ssr/src/tools/board/check-board.mjs` | Since 2026-09-28: generates the board from card fields (single home of status). |

Analysis: [analysis/ssr-task-board.md](./analysis/ssr-task-board.md).

## Glidelines — prior art for charts, format and burndown

`/Users/wanghui/coding/glidepace/glidepace-vibe-cc` (remote `mypals-ml/glidepace-vibe`). Vite + React + TypeScript + Tailwind v4, Supabase realtime, GitHub Projects V2 as data source, Capacitor, i18next.

| File | Why it matters |
|---|---|
| `docs/ARCHITECTURE.md` | Overall design. |
| `public/task-files/FORMAT.md`, `task-list.schema.json` | The `tasklist.json` v1.1 format: project, workstreams, tasks, burndown history. |
| `src/task-files/burndown.ts` (+ tests), `BurndownChart.tsx`, `FileOverviewCharts.tsx` | Burndown with a reconstructed band for missing completion dates; overview charts. |
| `src/task-files/format.ts` | Validation, cross-record checks, dependency cycle detection. |
| `src/lib/forecastDashboardUtils.ts`, `src/lib/forecastAssumptionsConfig.ts`, `src/components/Dashboard/Views/ForecastDashboard.tsx` | Estimate-based forecast: capacity, workers, remaining percentage by status. |
| `docs/task-files/convert-ssr-plan.py`, `docs/task-files/examples/ssr-migration.tasklist.json` | One-shot converter and snapshot of the SSR board (2026-09-08). Private project content: do not copy its text into glidebox fixtures. |

Relationship: [decision 0003](./decisions/0003-relationship-to-glidelines.md) (open).
