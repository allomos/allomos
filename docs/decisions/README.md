# Decisions

One file per decision: context, options, the decision and its consequences. Status is one of **Open**, **Proposed**, **Accepted**, **Superseded**.

| # | Decision | Status |
|---|---|---|
| [0001](./0001-database-source-of-truth.md) | The database is the source of truth | Superseded by 0007 on 2026-10-04 |
| [0002](./0002-burndown-counts-plus-sizes.md) | Burndown uses task counts, plus optional sizes | Accepted 2026-09-27 |
| [0003](./0003-relationship-to-glidelines.md) | Relationship to Glidelines (standalone vs. reuse/extend) | **Accepted** — B, standalone |
| [0004](./0004-team-accounts-and-roles.md) | Team accounts and roles | Accepted in part 2026-09-27 (roles; owner/admin invite by email) |
| [0005](./0005-start-and-open-agent-sessions.md) | Start a task in an AI agent, and jump into it, from Allomos | Requirement accepted 2026-09-27; design proposed |
| [0006](./0006-local-and-cloud-projects.md) | Local-only projects without sign-in; optional cloud projects with sign-in | Requirement accepted 2026-10-01; implementation design open |
| [0007](./0007-git-like-project-sync.md) | Git-like local/cloud project history, push/pull and task-aware merge | Accepted 2026-10-04 (storage); sync and merge details proposed |
| [0008](./0008-build-our-own.md) | Build Allomos ourselves, not on an open-source base | Accepted 2026-10-04 |
| [0009](./0009-desktop-hub-with-web-and-mobile-companions.md) | The desktop app is the hub; web and mobile apps connect to it | Requirement accepted 2026-10-04; design open |
| [0010](./0010-project-data-folder.md) | A project's data lives in a folder the user chooses (standalone or inside an existing project folder) | Accepted 2026-10-04 |
| [0011](./0011-desktop-framework-and-shared-code.md) | Desktop for macOS and Windows; one shared interface code base; desktop framework: Tauri 2 | Accepted 2026-10-04 |
| [0012](./0012-free-local-app-paid-cloud.md) | The local app is free; payment only for the cloud | Accepted 2026-10-04; paid features and licence open |
| [0013](./0013-product-name-fliphopper.md) | The product is called FlipHopper; a project's data is its seabag, always in `.fliphopper/seabag/` | Superseded by 0015 (2026-10-10) |
| [0014](./0014-main-purpose-and-three-sets.md) | Main purpose: switch between AI agents and teammates; three sets (main, AI skills, progress management) | Accepted 2026-10-06; contents of sets 2–3 proposed |
| [0015](./0015-product-name-allomos.md) | The product is called Allomos (换肩); a project's data is its taskbag, always in `.allomos/taskbag/` | Accepted 2026-10-10 |
| [0016](./0016-waiting-and-blocked-states.md) | Waiting on a person and blocked by another task are two different states (`waiting` narrowed, new `blocked`) | **Accepted** 2026-10-14 |
| [0017](./0017-agents-run-git-not-alm.md) | `alm` and the app never run Git; each change tells the agent the exact Git commands to share it | **Accepted** 2026-10-14 |
| [0018](./0018-mcp-deferred-agents-use-alm.md) | MCP is deferred: agents use the `alm` command; Iteration 3 is closed | **Accepted** 2026-10-24 |
