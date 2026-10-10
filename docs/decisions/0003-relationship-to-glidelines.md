# 0003 — Relationship to Glidelines

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted — B, standalone** | 2026-09-27 | 2026-10-06 | will.w.hui |

## Decision

**Allomos is a new standalone product** (owner, 2026-10-06: "Allomos is a new standalone product"). This is option B.

- Allomos lives in its own repository (this one) with its own stack (Tauri 2, Rust, React; [0011](./0011-desktop-framework-and-shared-code.md)) and its own file format ([FILE-FORMAT.md](../product/FILE-FORMAT.md)). It does not extend Glidelines, and the two products share no library (not A or C).
- Glidelines stays a **read-only reference** ([REFERENCES.md](../REFERENCES.md)): its burndown, forecast and chart ideas may be learned from and rewritten here; nothing in Glidelines is changed.
- Glidelines' `tasklist.json` is not Allomos's storage; it may become an import or export format later, if a card asks for it.
- Option D (Glidelines later showing an Allomos project) is not planned; it would need a new decision.

## Question

Is Allomos a **new standalone product**, or should it **reuse or extend the Glidelines code** (`glidepace-vibe-cc`) — the same stack and the same `tasklist.json` format?

## Context

**Glidelines** (`/Users/wanghui/coding/glidepace/glidepace-vibe-cc`, remote `mypals-ml/glidepace-vibe`) is a React + Vite + TypeScript + Tailwind v4 app (≈33k lines in `src/`), with Capacitor for mobile, i18next (en / ja / zh-CN / zh-TW) and Vercel serverless functions. Its main product is a **Gantt dashboard over GitHub Projects V2**: GitHub is the data source, and Supabase is used only as a realtime broadcast relay for GitHub webhooks. Most of its state lives in one large `DashboardProvider`.

Inside it there is already a separate, relevant feature:

- **Task file workspace** (`/task-files`, `src/task-files/`): upload a `*.tasklist.json` file (format v1.1 — `public/task-files/FORMAT.md`, `task-list.schema.json`) and view overview charts, workstreams, board/list, task details and a **burndown** (`burndown.ts`, including a reconstructed band when completion dates are missing). Data is held **in memory only**; there is no persistence, sync or collaboration.
- **An SSR migration converter** (`docs/task-files/convert-ssr-plan.py`) and a reviewed snapshot (`docs/task-files/examples/ssr-migration.tasklist.json`, 44 tasks / 16 workstreams, as of 2026-09-08). The converter is specific to that board and refuses to run once the board's text changes.
- **An estimate-based forecast** for GitHub projects (`src/lib/forecastDashboardUtils.ts`, `forecastAssumptionsConfig.ts`): capacity days per week, workers, remaining-percentage by status.

Allomos needs (see [PLAN.md](../product/PLAN.md)): Git-tracked text files as source of truth ([0007](./0007-git-like-project-sync.md), which replaced the database decision 0001 on 2026-10-04), team accounts and roles, realtime collaboration, agent write access through a CLI/MCP, an event log, and a throughput-based burndown forecast.

## Options

> **Third-party bases are ruled out.** On 2026-10-04 the owner decided to build Allomos ourselves rather than start from an open-source tracker such as Backlog.md ([0008](./0008-build-our-own.md)). This record is only about reusing the owner's **own** Glidelines code.

### A. Extend Glidelines — grow the `/task-files` workspace into Allomos

Add persistence (Supabase tables), accounts, agent API and forecasting to the existing app.

- **For:** fastest route to a first screen; charts, board/list, i18n, format and burndown already exist; one app to deploy; the task-file feature is already isolated from `DashboardProvider`.
- **Against:** Glidelines' identity is GitHub-first (GitHub OAuth, Projects V2); Allomos needs its own accounts, roles and a database source of truth — a different product shape in the same repo. Capacitor, GitHub sync and webhook code become baggage. Two products share one release cycle and one set of risks. The `tasklist.json` v1.1 format rejects unknown fields and has no claims, labels, waiting-on, approvals or events, so it must change version anyway.

### B. New standalone product — reuse ideas, port selected code

New repository (this one) and new app; port `burndown.ts`, the forecast math and relevant chart components into `packages/core` / `apps/web`; keep `tasklist.json` as an **import/export format**, not the storage model.

- **For:** clean domain model built around agents, claims, labels, approvals and events; no GitHub-specific baggage; freedom to pick the stack (e.g. Next.js vs Vite) for server-side needs such as agent tokens and APIs; Glidelines stays stable.
- **Against:** more work before the first usable screen; ported code can drift from the original; two codebases to maintain.

### C. Shared core library — both products depend on it

Extract the task-list format, validation, burndown and forecast into a shared package (e.g. `@glidepace/core`) used by both Glidelines and Allomos. Allomos is a separate app.

- **For:** one implementation of the format and the math; Glidelines' task-file workspace and Allomos stay compatible (an Allomos export opens in Glidelines and vice versa).
- **Against:** needs package publishing or a monorepo spanning both products; changes must be coordinated across two apps; more setup up front.

### D. Glidelines becomes a view of Allomos

Allomos is standalone and owns the data; Glidelines later gains a "Allomos project" data source next to GitHub Projects, reusing its Gantt.

- **For:** each product does what it is good at (Allomos = tracking and forecasting; Glidelines = Gantt).
- **Against:** only meaningful after Allomos exists; it is a later step compatible with B or C, not an alternative for now.

## Comparison

| | A. Extend | B. Standalone, port | C. Shared core | D. Glidelines as view |
|---|---|---|---|---|
| Time to first usable screen | fastest | slower | slowest | n/a (later) |
| Fit with database + agents + roles | poor–medium | good | good | good |
| Code duplication | none | some (ported) | none | none |
| Coupling between the products | high | none | medium | low |
| Risk to Glidelines | high | none | low | low |

## Questions that would settle it

1. Should Glidelines and Allomos be **one product for users**, or two products?
2. Must the two stay **format-compatible** (open each other's files)?
3. Is there a **stack preference** for server-side work (Vite + serverless functions like Glidelines, or Next.js, or another)?
4. How much weight does **time to first screen** carry compared with a clean model?

## Leaning before the decision

This is still open after [0008](./0008-build-our-own.md). *Update 2026-10-04:* with Tauri 2 chosen ([0011](./0011-desktop-framework-and-shared-code.md)), project rules are written in Rust, so Glidelines' TypeScript burndown and forecast code can only be reused as ideas. Its React chart components could still be reused in the TypeScript interface. B now, with the option to move the ported core into a shared package (C) if Glidelines keeps evolving its task-file workspace; D as a possible later integration. **Until this is decided, work on the core model, importer, burndown and forecast is valid under B, C and D**, so it need not wait.
