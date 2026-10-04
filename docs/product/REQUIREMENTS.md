# Requirements

A **contract**: what glidebox must do. Each requirement has an ID that cards and tests cite. The *why* and design are in [PLAN.md](./PLAN.md) and the [decisions](../decisions/README.md). Status: **accepted** = the owner asked for it or accepted it; **proposed** = recommended, awaiting the owner.

## Visibility and progress

| ID | Requirement | Source | Status |
|---|---|---|---|
| R1 | Show every task's **state and progress**. | owner, 2026-09-27 | accepted |
| R2 | Group tasks by **area**; tasks have hierarchical IDs (area → task → child). | owner, 2026-09-27 | accepted |
| R3 | Present progress as **lists and charts** (per-area progress, state distribution, throughput). | owner, 2026-09-27 | accepted |
| R4 | A **burndown chart** showing remaining work and total scope over time. | owner, 2026-09-27 | accepted |
| R5 | **Estimate the project's finish date** from the burndown, shown as a range (50% / 85% confidence). | owner, 2026-09-27; [0002](../decisions/0002-burndown-counts-plus-sizes.md) | accepted |
| R6 | The burndown counts **tasks**; a task may carry an **optional size**, with a size-weighted view when sizes exist. | [0002](../decisions/0002-burndown-counts-plus-sizes.md) | accepted |
| R7 | Burndown, timeline, "recently finished" and the session list are **computed from recorded events**, never maintained by hand. | [0001](../decisions/0001-database-source-of-truth.md), kept by [0007](../decisions/0007-git-like-project-sync.md) | accepted |
| R8 | A **timeline** (gantt) generated from task events. | PLAN | proposed |
| R89 | Tasks are organised along **two independent axes**: **area** (*what* part of the product it belongs to) and **iteration** (*when* it is planned: a time-box ending in a release). Iteration is optional. Progress, burndown and forecast can be shown per area, per iteration, or for the whole project. | owner, 2026-10-04 | accepted |
| R90 | Each project can **rename these terms** to the words its team already uses (for example area → epic or workstream; iteration → sprint, milestone or release). Glidebox's screens and command messages then use those words. Defaults: **Area** and **Iteration**. | owner, 2026-10-04 | accepted |

## Source of truth and agents

| ID | Requirement | Source | Status |
|---|---|---|---|
| R10 | Each project's **source of truth is readable text files tracked by Git** (JSON fields, Markdown narrative) in its task box; any database is a rebuildable index. Hand-maintained Markdown boards are only an import source. | owner, 2026-10-04; [0007](../decisions/0007-git-like-project-sync.md) | accepted |
| R11 | **Agents write structured status** (state, label, claim, waiting-on, size) through a CLI and an MCP server. | [0001](../decisions/0001-database-source-of-truth.md), kept by [0007](../decisions/0007-git-like-project-sync.md) | accepted |
| R12 | **Claiming is exclusive**: one task, one holder. A claim is atomic within one project authority (a local project's desktop app, or the cloud service for a connected cloud project). A claim made while disconnected from that authority is only tentative and is shown as such ([0007](../decisions/0007-git-like-project-sync.md)). | PLAN; revised 2026-10-04 | proposed |
| R13 | A configurable **label ladder** per project (default: implemented → locally verified → connected → deployed and verified). | PLAN | proposed |
| R14 | **Waiting-on** is a structured field (owner check, approval, another task, external) with who and a note. | PLAN | proposed |
| R15 | **Import** an existing Markdown board (first: the SSR migration board). | owner, 2026-09-27 | accepted |

## Local and cloud projects

| ID | Requirement | Source | Status |
|---|---|---|---|
| R40 | Users can create and use a purely local project, with project information, tasks and their event history stored on their machine. | owner, 2026-10-01 | accepted |
| R41 | Local project tracking requires no sign-in, cloud account or connection to a Glidebox hosted service. | owner, 2026-10-01 | accepted |
| R42 | Users may choose cloud projects; cloud project access requires sign-in. | owner, 2026-10-01 | accepted |
| R43 | Local project data stays local unless the user explicitly chooses to transfer or share it. Automatic cloud upload is not part of local mode. | local-only requirement, 2026-10-01; [0006](../decisions/0006-local-and-cloud-projects.md) | accepted |
| R72 | In the desktop app, the user **chooses each project's task box**; any writable folder is allowed. | owner, 2026-10-04; [0010](../decisions/0010-project-data-folder.md) | accepted |
| R73 | The task box may be a **standalone folder** or a **subfolder of an existing project folder** (for example inside a code repository). | owner, 2026-10-04; [0010](../decisions/0010-project-data-folder.md) | accepted |
| R74 | A task box is **self-contained**: moving or copying it moves the project; rebuildable caches and indexes stay out of it or are marked as ignorable. | [0010](../decisions/0010-project-data-folder.md) | accepted 2026-10-04 |
| R75 | Inside an existing Git repository, glidebox **does not create a nested repository** and never commits or pushes on its own; one running desktop app writes to a folder at a time. | [0010](../decisions/0010-project-data-folder.md) | accepted 2026-10-04 |
| R76 | **All Git actions run the `git` command** installed on the user's machine (commit, push, pull, status, diff), so the user's Git configuration, credentials and repository hooks apply. If `git` is missing, the app asks the user to install it. | owner, 2026-10-04; [0010](../decisions/0010-project-data-folder.md) | accepted |
| R87 | A project's folder is called the **task box** everywhere: documents, screens and command messages. Its default name on disk inside a repository is **`.glidebox/`**. | owner, 2026-10-04; [0010](../decisions/0010-project-data-folder.md) | accepted |
| R88 | The command-line program is called **`glidebox`**, with **`gb`** as a short alias. | owner, 2026-10-04 | accepted |

Local tracking works offline. AI-provider calls and other external integrations may still need their own network access and authentication. Moving projects between modes and synchronization are not yet decided.

## Collaboration

| ID | Requirement | Source | Status |
|---|---|---|---|
| R20 | **Teammates collaborate online** in cloud projects with live updates. | owner, 2026-09-27 | accepted |
| R21 | Glidebox has **roles**; what a member can do depends on the role. | owner, 2026-09-27; [0004](../decisions/0004-team-accounts-and-roles.md) | accepted |
| R22 | For cloud projects, **an owner or admin adds a new member by email address**; no public sign-up for cloud membership. Local use requires no account. | owner, 2026-09-27; [0004](../decisions/0004-team-accounts-and-roles.md) | accepted |
| R23 | Role set: owner, admin, approver, contributor, viewer, agent; agents cannot approve. | [0004](../decisions/0004-team-accounts-and-roles.md) | proposed |
| R24 | A **"waiting on me"** inbox of checks and approvals owed by the current person (local identity or signed-in cloud member), with approvals recorded. | PLAN | proposed |
| R25 | Every change records **who** (person or agent session) and **when**. | PLAN | proposed |

## Agent sessions

| ID | Requirement | Source | Status |
|---|---|---|---|
| R30 | **Start a task in a specified AI agent** (Claude Code, Codex) from the app. | owner, 2026-09-27; [0005](../decisions/0005-start-and-open-agent-sessions.md) | accepted |
| R31 | **Jump into** an already-started task's session in that agent. | owner, 2026-09-27; [0005](../decisions/0005-start-and-open-agent-sessions.md) | accepted |
| R32 | Show whether a task's agent session is **running, waiting for input or exited**. | [0005](../decisions/0005-start-and-open-agent-sessions.md) | proposed |
| R33 | Only permitted members may start agents; runners accept only allowed projects, repositories and people; every start and open is recorded. | [0005](../decisions/0005-start-and-open-agent-sessions.md) | proposed |

## AI-generated project setup

| ID | Requirement | Source | Status |
|---|---|---|---|
| R50 | A team leader can ask an AI agent to generate project/task data and files using a prompt supplied by Glidebox. | owner, 2026-10-01 | accepted |
| R51 | Glidebox can read or transform that generated output into a usable project for its lists, charts and task workflow. | owner, 2026-10-01 | accepted |
| R52 | Supply a versioned generation prompt and file schema shared with the validator, with examples and explicit handling of unknown data. | [feature contract](./AI-GENERATED-PROJECTS.md) | proposed |
| R53 | Validate generated bundles and show an adoption/change preview with actionable errors before changing active project data. | [feature contract](./AI-GENERATED-PROJECTS.md) | proposed |
| R54 | Generated output cannot invent verified progress or authoritative approvals; missing completion history stays unknown. | [feature contract](./AI-GENERATED-PROJECTS.md) | proposed |
| R55 | Re-importing a generated bundle does not duplicate tasks or silently overwrite existing edits; show changes/conflicts. | [feature contract](./AI-GENERATED-PROJECTS.md) | proposed |

Workflow, storage boundaries and proposed acceptance examples: [AI-generated project setup](./AI-GENERATED-PROJECTS.md). Local use follows R40–R43; cloud publication remains an explicit signed-in action. The final file schema is not yet selected.

## Human and agent interfaces

| ID | Requirement | Source | Status |
|---|---|---|---|
| R60 | The app is well structured to provide interfaces for both people and AI agents. | owner, 2026-10-03 | accepted |
| R61 | Human UI supports desktop and mobile use. Packaging is set by R69–R71; desktop systems by R81; release order remains undecided. | owner, 2026-10-03; updated 2026-10-04 | accepted |
| R62 | Investigate and consider browser and human CLI interfaces alongside desktop/mobile; agent CLI/MCP remains required by R11. | owner, 2026-10-03 | accepted |
| R63 | Graphical clients, CLI, MCP and imports use shared task commands/queries with consistent validation, permissions and event recording. | [interface contract](./INTERFACES.md) | proposed |
| R64 | Agent interfaces expose discoverable capabilities, versioned structured inputs/results, bounded task context and actionable errors. | [interface contract](./INTERFACES.md) | proposed |
| R65 | Mutations support stale-revision detection and safe retries; interrupted long operations can be resumed without duplicate effects. | [interface contract](./INTERFACES.md) | proposed |
| R66 | Human review shows evidence and exact decision scope; responsible person, executing agent, verification and session state remain distinguishable. | [interface contract](./INTERFACES.md) | proposed |
| R67 | Graphical interfaces adapt to screen size and input method, with accessible text/controls and alternatives to drag-only actions. | [interface contract](./INTERFACES.md) | proposed |
| R68 | Clients show where local data lives, explain unavailable cross-device capabilities and support explicit transfer rather than automatic publication. | [interface contract](./INTERFACES.md) | proposed |
| R69 | The **desktop application provides the complete feature set** and is the hub for local projects, agents and companions. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R70 | **Web and mobile apps are companions** that connect to a running desktop application. The mobile companion is a **phone-friendly web page** served by the desktop app (decided 2026-10-04). | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R71 | Companions can **view** the desktop app's projects, including **progress and status** (task states, area progress, charts and burndown, waiting-on). | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R77 | Companions can also **act** (for example mark a check done, approve). Each action is a request **sent to the desktop app**, which checks it, applies it and records who did it. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R78 | Companions can connect **from outside** the local network, using a **free or very cheap** method. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R79 | **Pairing:** the companion user gives a valid email address and an alias; the desktop user approves the email; the desktop app sends a long verification code to that email; the code grants access. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R80 | Pairing codes are single-use and short-lived; the desktop user can remove a paired device at any time; pairing and companion actions are recorded. | [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | proposed |
| R81 | The desktop app ships for **macOS and Windows**. | owner, 2026-10-04; [0011](../decisions/0011-desktop-framework-and-shared-code.md) | accepted |
| R82 | The desktop window, browser companion and phone companion **share one interface source code**; only the desktop app adds native parts (Git, agent operations, MCP, task box, local server and other OS needs). | owner, 2026-10-04; [0011](../decisions/0011-desktop-framework-and-shared-code.md) | accepted |
| R83 | When the desktop app is not running, a companion tells the user to **start the desktop app on their computer**; **refreshing reconnects automatically** once it runs. No stale data is shown meanwhile. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R84 | The pairing code email is sent through a **free email-sending service**. Its access key is never committed or embedded in the shipped app. | owner, 2026-10-04; [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | accepted |
| R85 | macOS and Windows versions are **released together**; builds stay **unsigned until the first public release**. | owner, 2026-10-04; [0011](../decisions/0011-desktop-framework-and-shared-code.md) | accepted |
| R86 | The desktop app is built with **Tauri 2** (stable 2.x line); project rules live in Rust and are reused by the CLI and MCP server; the interface is TypeScript. | owner, 2026-10-04; [0011](../decisions/0011-desktop-framework-and-shared-code.md) | accepted |

Read [INTERFACES.md](./INTERFACES.md) for the accepted scope and proposed shared structure; [interface research](../research/interface-patterns.md) records primary sources, platform options and limitations. Detailed command/schema and packaging choices have not been accepted.

## Not yet decided

Remote-access method, and which free email service to use ([0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md)); UI languages; hosting target; one-time vs continuous import — see [TASK_BOARD § Waiting on the owner](../TASK_BOARD.md#waiting-on-the-owner).
