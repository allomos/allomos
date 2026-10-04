# 0006 — Local-only and optional cloud projects

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; implementation design open** | 2026-10-01 | 2026-10-01 | will.w.hui |

## Context

The owner requires Glidebox to run purely locally, keeping project and task information on the user’s machine. Users may also choose cloud projects, which require sign-in. The earlier plan assumed a hosted database for all projects.

## Decision

- Local projects store project information, tasks and event history locally and support tracking without sign-in, a cloud account or a Glidebox hosted service.
- Cloud projects are optional and require sign-in; the existing cloud collaboration and invitation requirements apply there.
- Local data stays local unless the user explicitly chooses to transfer or share it.
- Each project’s database remains its source of truth under [0001](./0001-database-source-of-truth.md). Local mode does not restore Markdown as the live database.

## Consequences

- The shared model, progress calculations, importer and agent status operations must support local storage independently of the cloud backend.
- Cloud sign-in cannot gate starting the app or opening local projects.
- Local tracking works offline; launching AI agents or using other external integrations may require those providers’ network access and authentication.
- Local users still need recorded actors and approval history; the exact local identity and permission design remains open.

## Design questions still open

*Update 2026-10-04:* a local project's data lives in a folder the user chooses, standalone or inside an existing project folder ([0010](./0010-project-data-folder.md)).

- Local database technology, runtime and app packaging.
- How users select and manage local and cloud projects.
- Whether and how to move or copy projects between modes.
- Whether to support synchronization or offline cloud edits, and how to resolve conflicts.

The owner’s requirement authorizes documenting these boundaries, not choosing those designs or provisioning cloud resources.
