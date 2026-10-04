# Glidebox — plan

> Status: **draft, first written 2026-09-27, last revised 2026-10-04.** Decisions marked *proposed* are recommendations awaiting the owner's confirmation. Full records: [decisions/](../decisions/README.md); requirements: [REQUIREMENTS.md](./REQUIREMENTS.md); discussions: [all records](../README.md#discussions). Glidebox is developed by AI agents following [AGENTS.md](../../AGENTS.md).

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
- **Agents are first-class users**: Claude and Codex sessions claim, update and release tasks through a CLI / MCP server, cheaply, with one holder per task (R12).
- **A desktop app as the hub**, with the full feature set. Web and mobile companion apps connect to the running desktop app to view projects, including progress and status ([D9](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md), R69–R71).
- **Interfaces for people and agents**, with desktop/mobile human use and browser/human CLI considered in the design. Shared structure and platform patterns are documented in [INTERFACES.md](./INTERFACES.md) (R60–R68; detailed design proposed).
- A **"waiting on me"** inbox for checks and approvals owed by a person.
- **Import** the existing SSR migration board as the first real dataset.
- **Generate project setup with an external AI agent:** a team leader supplies Glidebox’s prompt and team context; Glidebox reads/transforms the resulting project/task files ([feature contract](./AI-GENERATED-PROJECTS.md), R50–R51).
- **Start a task in a chosen AI agent** (Claude Code, Codex) from the app, and **jump into** an already-started task's agent session ([D5](../decisions/0005-start-and-open-agent-sessions.md)).

## 3. Non-goals (for now)

- Orchestrating agents automatically (scheduling, auto-assigning, chaining agents). Glidebox starts an agent only when a person asks it to.
- Replacing git, PRs or CI. Glidebox links to them.
- Storing secrets, credentials or evidence logs. Cards link to evidence; they do not hold it.
- Full project-management features (budgets, time sheets, resource leveling).
- Several glidebox projects inside one code repository ([D10](../decisions/0010-project-data-folder.md)); special handling for synced folders, network drives or removable disks.

## 4. Decisions

| # | Decision | Status |
|---|---|---|
| [D1](../decisions/0001-database-source-of-truth.md) | ~~The database is the source of truth.~~ Agents write structured fields through the CLI/MCP; hand-maintained markdown boards come in through an importer. | **superseded by D7, 2026-10-04** |
| [D2](../decisions/0002-burndown-counts-plus-sizes.md) | **Burndown uses task counts, plus optional sizes.** Every task counts as 1; a task may carry an optional size (points or days), and the chart can switch to size-weighted when sizes are present. | **accepted 2026-09-27** |
| [D3](../decisions/0003-relationship-to-glidelines.md) | **Relationship to Glidelines.** Leaning: standalone product, reusing Glidelines ideas and code where it fits (same stack family; port the `tasklist.json` format and burndown/forecast logic from `glidepace-vibe-cc`, not fork the app). Options A–D are compared in the decision record. | **open — not decided** |
| [D4](../decisions/0004-team-accounts-and-roles.md) | **Cloud team accounts with roles**: owner, admin, approver, contributor, viewer, and agent (token-based, scoped to a project). **An owner or admin adds members by email address**; no public sign-up. | **roles and email invites accepted 2026-09-27**; role details proposed |
| [D5](../decisions/0005-start-and-open-agent-sessions.md) | **Start and open agent sessions from glidebox**, through a local runner daemon (`gb runner`) on a person's machine or a vendor cloud session. | **requirement accepted 2026-09-27**; design proposed |
| [D6](../decisions/0006-local-and-cloud-projects.md) | **Local-only and optional cloud projects.** Local tracking stores data on the user’s machine without sign-in; cloud projects require sign-in. Storage technology and transfer/sync design remain open. | **requirement accepted 2026-10-01** |
| [D7](../decisions/0007-git-like-project-sync.md) | **Git with readable text files is the source of truth.** JSON fields and Markdown narrative in the data folder; a database is only a rebuildable index; push/pull/merge with a cloud copy. | **accepted 2026-10-04**; sync and merge details proposed |
| [D8](../decisions/0008-build-our-own.md) | **Build our own.** No open-source tracker (e.g. Backlog.md) is used as a base or fork; they are references only. | **accepted 2026-10-04** |
| [D9](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | **Desktop app is the hub.** Full features on desktop; web and mobile companions connect to the running desktop app to view progress and status. | **requirement accepted 2026-10-04**; design open |
| [D10](../decisions/0010-project-data-folder.md) | **User-chosen data folder.** Each local project's data lives in a folder the user picks: standalone, or a subfolder of an existing project folder such as a code repository. | **requirement accepted 2026-10-04**; details proposed |
| [D11](../decisions/0011-desktop-framework-and-shared-code.md) | **Desktop for macOS and Windows; one shared interface code base.** Desktop window, browser and phone use the same web UI; only the desktop adds native parts. Framework: Electron recommended. | **requirements accepted 2026-10-04**; framework proposed |

## 5. Data model (first cut)

- **Project** — storage mode (`local | cloud`), data folder (local projects: any folder the user chooses, standalone or inside an existing project folder — [D10](../decisions/0010-project-data-folder.md)), name, label ladder (configurable; SSR default: `implemented → locally verified → connected → deployed and verified`), forecast settings.
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

### AI-generated project setup

The leader gets a Glidebox prompt, asks their chosen external agent to generate a project bundle, then opens/imports it in Glidebox. Proposed flow: validate → preview → repair errors → accept locally → optionally publish after sign-in. The prompt/schema and adoption details are proposed in [AI-GENERATED-PROJECTS.md](./AI-GENERATED-PROJECTS.md). This path does not require the in-app agent runner or cloud access.

### Human and agent interfaces

Desktop/mobile human use is accepted; browser and human CLI must be considered. Proposed presentation: task list/detail and review panes on wide screens; My tasks, Waiting on me, quick capture and full-screen task detail on phones. CLI/MCP provide structured discovery, context and task operations for agents. All supported surfaces should use shared project rules and recorded history. [Interface contract](./INTERFACES.md) and [internet investigation](../research/interface-patterns.md) include platform tradeoffs, accessible interaction, agent tool patterns and the distinction between device-local and cloud access. Packaging direction is now set by [D9](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md): a desktop hub with web and mobile companions. Operating systems, the mobile form and release order remain open.

## 7. Burndown and forecast

- **Lines:** remaining tasks and total scope over time (scope growth is visible, not hidden); the size-weighted variant when sizes exist.
- **Forecast by throughput:** sample the tasks finished per week over the last N weeks (default 4–6) and run a Monte Carlo simulation of the remaining count → finish dates at **50% / 85%** confidence, drawn as a cone.
- **Forecast by estimate** (secondary): remaining size ÷ capacity, with remaining percentage by state (as in Glidelines: to do 100%, in progress 50%, waiting 20%).
- **Missing history:** for imported tasks without completion dates, reuse Glidelines' reconstructed-band approach instead of inventing dates.
- Optional target date line and "on track / at risk" indicator.

## 8. Architecture (proposed)

- **Interface separation:** human clients, CLI, MCP and generated-file import call shared application commands/queries for permissions, validation and event recording, above local/cloud storage adapters. `packages/core` retains pure model/calculation rules; the application layer owns side effects. Desktop/mobile presentation and platform capabilities adapt around this shared behavior; layout, framework and process boundaries remain open ([INTERFACES.md](./INTERFACES.md)).
- **Desktop hub** ([D9](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md)): one desktop application holds the local project store, runs the shared commands, and hosts a **local server** for the web and mobile companions. The CLI, MCP server and agent runner talk to the same local authority. It ships for **macOS and Windows**, and its window shows the **same web interface** that it serves to browser and phone companions; only the desktop adds native parts ([D11](../decisions/0011-desktop-framework-and-shared-code.md)). Framework: Electron recommended, owner to choose.
- **Monorepo** (pnpm, TypeScript):
  - `packages/core` — domain model, validation, burndown and forecast (pure, unit-tested; shared by all others)
  - `apps/desktop` — the desktop hub for macOS and Windows: native shell (Electron recommended), local store, Git, agent runner, MCP, local server for companions
  - `apps/web` — React + Vite + Tailwind UI (charts, lists, inbox), used inside the desktop app and served to browser and phone companions; the phone companion is a phone-friendly layout of this web UI ([D9](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md))
  - `packages/local-store` — local project storage adapter, reading and writing the project's user-chosen data folder ([D10](../decisions/0010-project-data-folder.md)); Git-tracked JSON/Markdown files per D7, plus a rebuildable index kept outside the folder
  - `packages/cli` — `gb` command for people and agents (`gb claim`, `gb label`, `gb wait`, `gb done`, `gb release`, `gb next`)
  - `packages/mcp` — MCP server exposing the same operations to Claude Code and Codex
  - `packages/runner` — local daemon (`gb runner`): receives start requests, creates worktrees, launches and resumes agents, reports session state
  - `packages/importer` — markdown board → glidebox (first target: the SSR `TASK_BOARD.md` + `workstreams/`)
  - `supabase/` — cloud side (proposed): sign-in, roles, atomic claims and the cloud project index; how it hosts the Git remote is open under D7
- **Storage modes ([D7](../decisions/0007-git-like-project-sync.md)):** local projects are Git-tracked text files in a user-chosen data folder, with a rebuildable index outside it. Cloud projects are a hosted copy (a Git remote with server-side permission checks) and require sign-in. The domain model and event rules must work in both modes. Local tracking cannot depend on cloud authentication or hosted services. Local database technology and packaging remain open.
- **Cloud backend (proposed):** Supabase — Postgres, Auth (cloud sign-in), Row Level Security per project and role, Realtime for live updates. Cloud agents use scoped project tokens; local agent access must work without a cloud token.
- **Transfer and sync:** moving/copying a local project to the cloud, offline cloud edits, and two-way synchronization remain open; do not assume automatic uploads.
- **Hosting:** to decide (Vercel or Cloudflare).
- **Agent instructions:** a short `AGENTS.md` / `CLAUDE.md` snippet telling sessions to resume with `gb next` and report with `gb` instead of editing board markdown.

## 9. Milestones

| # | Milestone | Done when |
|---|---|---|
| M0 | Repo and plan | This plan agreed; repo pushed |
| M1 | Core + importer (local) | `packages/core` model, burndown and forecast tested; the SSR board imports with every card, workstream and label accounted for |
| M2 | Desktop hub, read-only | The desktop app stores an imported local project and shows lists, charts and burndown, without sign-in or hosted services |
| M2b | Companions | A browser and a phone, paired by email and code (R79), reach the running desktop app from outside the local network (R78), show its progress and status (R71), and send actions such as approvals to it (R77) |
| M3 | Agent write path | CLI + MCP: claim / label / wait / done / release, with atomic claims and an event log; one real Claude and one Codex session use it |
| M4 | Cloud projects and collaboration | Signed-in cloud projects with the same views; roles, owner/admin invites by email, realtime updates, "waiting on me" inbox with recorded approvals |
| M5 | Start and open agents | Research spike on each agent's launch/resume interfaces; local runner starts a task in Claude Code and in Codex, reports the session and claims the task; "Open session" works for local and cloud sessions |
| M6 | Polish | Generated timeline, markdown export, size-weighted burndown, target dates |

**Not yet placed (owner to confirm):** AI-generated project setup (R50–R51, workstream W09) — proposed with M2, because it reuses the importer's validation. Human CLI and other interface work (R60–R62, workstream W10) — alongside M2–M3.

## 10. Open questions

Kept in one place: [TASK_BOARD § Waiting on the owner](../TASK_BOARD.md#waiting-on-the-owner).

## 11. References

[REFERENCES.md](../REFERENCES.md) — the SSR migration board and Glidelines, with the files that matter.

Competitor evidence: [investigation and feature comparison](../research/competitor-investigation.md) (researched 2026-09-30; local/cloud requirement added 2026-10-01). Recommendations are hypotheses, not decisions.

Interface evidence: [human and AI-agent investigation](../research/interface-patterns.md) (2026-10-03), covering desktop, mobile, browser, human CLI, MCP/tool design and optional embedded agent-host UI. Public-source research only; no hands-on competitor testing.
