# Competitor investigation

Public-source research conducted 2026-09-30 and extended 2026-10-01; comparison consolidated 2026-10-01. Glidebox's column incorporates the owner's local/cloud requirement accepted 2026-10-01. This is research evidence and analysis, not an approval of a product direction or architecture.

## Finding

Glidebox overlaps substantially with existing project trackers and coding-agent workspaces. Boards, agent launches, collaboration and finish-date predictions are already available. Its most promising areas to validate are tracking verified delivery, recording exact decisions owed by people, and preserving task context across agent sessions.

For the original agent-workspace scope, Vibe Kanban is the closest task-board comparison; Linear is the strongest broad substitute; Conductor is both a session-management competitor and a possible integration target. These rankings are judgments based on the documented features below, not hands-on results. For the later Git-backed storage proposal, Backlog.md is the closest task-file comparison and git-bug the closest distributed-issue precedent.

## Method and limits

The investigation compared official vendor documentation with Glidebox's [plan](../product/PLAN.md) and [requirements](../product/REQUIREMENTS.md). It did not install or operate competitors, create accounts, or test workflows. Some vendor pages could not be retrieved; the readable documentation index or search extracts supported only limited claims. Pricing, license terms, adoption, security guarantees and API suitability were not comprehensively investigated.

**Legend:** ✓ = documented; **Partial** = related functionality documented but the full feature was not established; **?** = not established by this investigation, not evidence of absence. **Required** and **Proposed** describe Glidebox's planning commitments, not implemented capabilities. Source numbers identify entries in the source register below. Glidebox's Git storage and indexing cells reflect proposal 0007, not an accepted storage architecture.

## Feature comparison

| Feature | Vibe Kanban [1–3] | Conductor [4–7] | Linear [8–11] | GitHub Agent HQ [12] | Plane [13–14] | ActionableAgile [15] | Backlog.md / MrLesk [16] | git-bug [17–18] | backlog / veggiemonk [19] | Obsidian + Git [20] | **Glidebox** |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Structured task tracking with lists or boards | ✓ Issues, boards, lists | Partial — workspaces linked to issues | ✓ Issues and projects | ✓ Existing GitHub issue workflow | ✓ Work items and projects | Analytics focus | ✓ Browser and terminal boards | ✓ CLI, terminal, local web UI | Partial — task CLI | Partial — notes interface | **Required** |
| Workstream grouping and task hierarchy | ? Exact mapping not checked | ? Exact mapping not checked | Partial — projects and sub-issues | ? Exact mapping not checked | Partial — projects; exact hierarchy not checked | ? | Partial — milestones and dependencies; exact mapping not checked | ? | ✓ Hierarchical task IDs and parent fields | ? | **Required** — workstreams and hierarchical IDs |
| Online team collaboration | ✓ Live updates and invitations | ✓ Shared cloud workspaces | ✓ Team tracker | ✓ GitHub collaboration | ✓ Team project tracker | ? Task editing not checked | Partial — shared Git files; live hosted workflow not established | Partial — remote issue collaboration; public portal WIP | Partial — shared Git files | Partial — Git file sharing; managed team workflow not established | **Required** — cloud projects |
| Agent-accessible MCP / API | MCP listed; tool coverage not checked | ✓ Workspace and session tools | ✓ Tracker MCP | Partial — agent issue workflow; external control API not checked | ✓ Work-item and project MCP | ? | ✓ CLI and MCP | ✓ CLI and GraphQL; MCP not established | ✓ CLI; MCP not established | ? | **Required** — structured CLI/MCP updates |
| Start work in Claude Code or Codex | ✓ Both integrations listed | ✓ Both supported | Partial — coding sessions; exact provider options not checked | ✓ Both documented | ? | ? | ? | ? | ? | ? | **Required** — both providers |
| Return to existing agent work | ✓ Switch conversations | ✓ Sessions and workspace links | ✓ Active coding sessions | ✓ Session management | ? | ? | ? | ? | ? | ? | **Required** |
| Session activity and attention states | ✓ Running, Idle, Needs Attention | ✓ Session status API/MCP | ✓ Coding-session status | ✓ Progress and logs | ? | ? | ? | ? | ? | ? | **Proposed** |
| Project progress and scope charts | ? | ? | ✓ Scope and progress graph | ? Project analysis not checked | ? Relevant charts not checked | ✓ Flow analytics | ? | ? | ? | ? | **Required** — charts and burndown |
| Project finish-date prediction | ? | ? | ✓ Historical velocity | ? | ? | ✓ Monte Carlo forecasting | ? | ? | ? | ? | **Required** — date range |
| Probability-based throughput forecasting | ? | ? | Partial — velocity with date buffers | ? | ? | ✓ Simulation and probabilities | ? | ? | ? | ? | **Required** — 50% / 85% dates; simulation method proposed |
| Separate implementation and verification gates | ? Dedicated ladder not established | Partial — tests, review, checks | Partial — workflow and review | Partial — review and checks | ? Dedicated ladder not established | ? | ? | ? | ? | ? | **Proposed** — configurable verification ladder |
| Exact approvals and checks owed by a person | Partial — Needs Attention | Partial — agent permissions | Partial — priority inbox and conditional approvals | Partial — review workflow | ? | ? | ? | ? | ? | ? | **Proposed** — waiting-on-me inbox |
| Enforced one-task / one-holder claiming | ? | ? Workspace isolation is not a claim guarantee | ? | ? | ? | ? | ? | ? | ? | ? | **Proposed** — atomic claim |
| Task continuity across session changes | Partial — histories and shared files | Partial — workspace context and session records | Partial — issue context and sessions | Partial — issues, sessions, PRs | Partial — durable work-item context | ? | Partial — durable task specs, plans and notes | Partial — durable issue history | Partial — durable task files | Partial — durable notes; agent-session continuity not checked | **Intended outcome** — task and event model |
| Import the SSR Markdown board | ? Exact format not checked | ? | ? Exact format not checked | ? Exact format not checked | ? Exact format not checked | ? Exact format not checked | ? | ? | ? | ? | **Required** — dedicated importer |
| Local project/task storage without cloud sign-in | ? End-to-end condition not checked | Partial — local workspaces and local session data; complete tracker not checked | ? | ? | Partial — self-hosting is documented; account-free local use not established | ? | ✓ Account-free local use | ✓ Offline repository workflow | ✓ Local repo files | ✓ Local-file workflow | **Required** — R40–R43 |
| Optional cloud projects requiring sign-in | ✓ Cloud authentication documented in index | ✓ Cloud organizations and authenticated API | ✓ Cloud workspace and authenticated MCP | ✓ GitHub account workflow | ✓ Cloud and self-hosted options; exact mode selection not checked | ? | Partial — optional Git remote; managed cloud projects not established | Partial — remote authentication; managed cloud project mode not established | Partial — optional Git remote; managed cloud projects not established | Partial — authenticated Git remote; dedicated cloud task projects not established | **Required** — R42 |
| Canonical data as ordinary text files | ? | ? | ? | ? | ? | ? | ✓ Markdown task files | Different — Git-internal entities | ✓ Markdown with YAML metadata | ✓ Markdown notes | **Proposed** — JSON fields and Markdown narrative |
| Git-backed task / project synchronization | ? | ? | ? | ? | ? | ? | Partial — optional remote Git operations; conflict behavior not tested | ✓ Dedicated issue push/pull | ✓ Task files tracked through Git | ✓ Plugin commit/pull/push | **Proposed** — actual Git push/pull/merge |
| Local derived search / chart index | ? | ? | ? | ? | ? | ? | ? Implementation not examined | ✓ Local storage and search-index interfaces; rebuild details not checked | ? | ? | **Proposed** — rebuildable local index |
| Local indexing database excluded from synchronization | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | **Proposed** — local index not committed or pushed; cloud builds its own index |
| Managed cloud roles alongside local Git workflow | ? | ? | ? | ? | ? | ? | ? | Partial — public portal WIP; role combination not established | ? | ? | Cloud roles **required**; Git integration **proposed** |

## Competitor implications

### Vibe Kanban

Its documented session workflow has independently running conversations, agent selection, and an attention state for questions or approvals [1–2]. Team access includes email invitations and live updates; the documented roles are Admin and Member, with both able to manage tasks and projects [3]. This makes it the first product to test against Glidebox's proposed task and owner workflow. More granular roles could be useful, but that benefit is not yet validated.

### Linear

Linear combines project tracking, MCP, coding sessions and forecast graphs [8–10]. Its graph treats unestimated issues as one point, tracks scope, and predicts completion from weekly velocity. Its documented optimistic/pessimistic dates use approximately ±40% buffers [9]. Glidebox's proposed simulation percentiles differ, but that alone does not establish superior accuracy. Linear also documents a priority inbox and conditional tool approvals [10–11], so human attention and approval handling cannot be claimed as unique to Glidebox.

### Conductor

Conductor provides workspaces for Claude Code, Codex and other coding agents, review tools, cloud collaboration and programmable session operations [4–6]. Its deep links support opening or creating workspaces, including a Linear-issue workflow [7]. Glidebox could investigate integrating with this execution environment. API coverage, permissions and the ability to preserve Glidebox's task/approval rules need validation before selecting that approach.

### GitHub Agent HQ

GitHub documents assigning issues to Claude, Codex and Copilot, tracking sessions, and reviewing resulting proposed changes [12]. Teams already using GitHub have a strong alternative with little additional tracking infrastructure. Glidebox would need to justify another source of project state through a clearer delivery and owner-decision workflow.

### Plane and ActionableAgile

Plane offers tracker operations through MCP and self-hosting [13–14]. Self-hosting alone does not establish the new Glidebox requirement of local use without sign-in. ActionableAgile provides historical-throughput simulation and probability results [15]; it is a forecasting benchmark rather than a verified substitute for the full agent/task workflow.

## Opportunities to validate

These are hypotheses, not confirmed gaps in competing products.

1. **Verified delivery.** Record implemented, locally verified, deployed and verified, and owner verified as distinct gates, with who checked each gate and links to evidence.
2. **Exact owner decisions.** Put the requested operation, authorization scope, decision-maker and next delivery gate together so the owner can resolve waiting work quickly.
3. **Session continuity.** Preserve decisions, approvals, evidence and the next dependency when work moves between agents, people or machines.
4. **Delivery forecasts.** Explore forecasting implementation completion separately from owner-verified completion, making review delays visible. Reliable history and handling changing scope are prerequisites.
5. **Local autonomy with optional cloud collaboration.** Validate the full local workflow, including persistence, progress calculations and agent status updates, without a hosted service or sign-in. The owner accepted this requirement after the initial research; competitor coverage remains incomplete.

## Recommendation and next investigation

Before implementation, test Vibe Kanban, Linear and Conductor using the same small set of SSR-style tasks: one ready task, one awaiting deployment approval, and one interrupted by a session change. Use synthetic task text or explicitly approved data; the SSR reference repository remains read-only.

Check whether the owner can see what was verified and by whom, decide the exact next approval, and resume a task without reconstructing scattered messages. Compare forecast completion criteria and the amount of setup and manual bookkeeping. Add a separate local/cloud check: can tracking run with no cloud account or connection, where is data persisted, and does selecting cloud require explicit sign-in?

A positioning hypothesis to test is: **Glidebox helps an owner see what people and AI agents have actually verified, what needs a human decision next, and when the remaining work is likely to finish.**

Retain the structured event history, waiting-on fields and verification ladder in planning. Evaluate existing execution tools before committing to the full runner. The research does not establish that a standalone product is preferable to an integration or a narrower delivery-and-approval layer, and does not change accepted requirements or decide open architecture questions.

## Source register

Sources 1–15 were consulted 2026-09-30; sources 16–20 were consulted 2026-10-01. Consolidating the comparison does not represent a new live verification. Links can change.

1. [Vibe Kanban documentation index](https://vibekanban.com/docs/llms.txt) — agent integrations, cloud views, authentication, MCP and self-hosting pages; index-level evidence only where full pages were inaccessible.
2. [Vibe Kanban sessions](https://vibekanban.com/docs/workspaces/sessions) — conversations, shared files, agent selection and session states.
3. [Vibe Kanban team members](https://vibekanban.com/docs/cloud/team-members) — invitations, live updates and role permissions.
4. [Conductor first workspace](https://www.conductor.build/docs/first-workspace) — agents, isolation and review flow.
5. [Conductor cloud FAQ](https://www.conductor.build/docs/cloud/faq) — local/cloud distinction and team collaboration.
6. [Conductor MCP server](https://www.conductor.build/docs/api/mcp) and [API](https://www.conductor.build/docs/api) — authenticated workspace and session operations.
7. [Conductor deep links](https://www.conductor.build/docs/reference/deep-links) — workspace and Linear-issue navigation.
8. [Linear MCP](https://linear.app/docs/mcp) — agent access to tracker data.
9. [Linear project graph](https://linear.app/docs/project-graph) — scope, task counts and prediction method.
10. [Linear changelog](https://linear.app/changelog) — coding sessions, priority inbox and returning to sessions, through 2026-09-24.
11. [How Linear built its agent](https://linear.app/now/how-we-built-linear-agent) — contextual approvals.
12. [GitHub: Claude and Codex in Agent HQ](https://github.blog/news-insights/company-news/pick-your-agent-use-claude-and-codex-on-agent-hq/) — issue delegation, sessions and review. This dated announcement establishes capabilities, not current plan entitlements or pricing.
13. [Plane MCP server](https://developers.plane.so/dev-tools/mcp-server) — tracker operations; search-extract evidence when direct retrieval failed.
14. [Plane self-hosting](https://plane.so/self-hosted) — deployment choices; does not establish no-sign-in local use.
15. [ActionableAgile: Monte Carlo — How Many](https://55degrees.atlassian.net/wiki/spaces/ActionableAgile/pages/2407399482) — simulation, historical throughput and probability outputs. This vendor documentation page is archived; current behavior needs a live check. [Vendor site](https://actionableagile.com/) provides product context.

16. [Backlog.md maintainer README](https://github.com/MrLesk/Backlog.md/blob/main/README.md) — Markdown tasks, local boards, agent CLI/MCP and optional Git remotes.
17. [git-bug maintainer repository](https://github.com/git-bug/git-bug) — distributed offline issues, internal Git storage, issue push/pull and interfaces.
18. [git-bug storage/index interfaces](https://github.com/git-bug/git-bug/blob/trunk/repository/repo.go) — local storage and search-index operations; full rebuild/upload behavior not verified.
19. [backlog by veggiemonk](https://github.com/veggiemonk/backlog) — Markdown/YAML task files, hierarchical IDs and Git workflow.
20. [Obsidian Git plugin](https://github.com/tfney/obsidian-git) — commit/pull/push, history and diff interface for local notes.

## Follow-up: actual Git-backed tasks and local indexes

Investigated 2026-10-01 after the owner asked about actual Git with structured text files. These examples were missing from the initial product shortlist and materially strengthen the comparison. Evidence is maintainer documentation/source, not hands-on testing.

The products and storage features from this follow-up are incorporated in the [main feature comparison](#feature-comparison); there is one comparison table.

### Closest examples

**Backlog.md by MrLesk** is the strongest match for visible task files plus a local interface for people and agents. Its README documents Markdown tasks, acceptance criteria, milestones/dependencies, a local browser board, CLI/MCP and account-free local use. Do not confuse it with the separate `backlogmd/backlogmd` specification project from the first search. [Maintainer README](https://github.com/MrLesk/Backlog.md/blob/main/README.md).

**git-bug** is the stronger distributed-issue precedent: offline editing, Git remotes, issue push/pull and multiple interfaces. Its storage uses Git internals instead of adding ordinary files to the project. Its source exposes local storage under `.git/git-bug` and search-index operations. This supports the broad canonical-Git/local-index pattern; the exact index persistence, rebuild guarantees and transfer exclusions still need source inspection. [Maintainer repository](https://github.com/git-bug/git-bug), [storage/index interfaces](https://github.com/git-bug/git-bug/blob/trunk/repository/repo.go).

**backlog by veggiemonk** documents task files with YAML metadata, hierarchical IDs, status and parent/assignee fields, tracked in Git. It is useful prior art for schema and file organization. [Maintainer repository](https://github.com/veggiemonk/backlog).

**Obsidian with its community Git plugin** documents app-integrated commit/pull/push and history/diff views. It is relevant to hiding Git operations behind a familiar local-file interface, rather than a verified substitute for Glidebox's delivery tracking. [Plugin repository](https://github.com/tfney/obsidian-git).

### Implication

Actual Git-backed task storage is established prior art, not a unique product distinction. Test Backlog.md before building equivalent task-file and agent-access functionality; study git-bug for distributed history and derived indexes. No inspected source establishes the full Glidebox combination of verified-delivery gates, exact operation approvals, forecast dates, controlled cloud collaboration and session launch/resume. That is an evidence limit, not proof of a market gap. No local index upload policy was independently verified across these products.

## Follow-up: starting from open source and selling a service

Investigated 2026-10-01 after the owner said they may sell services through Glidebox. This records a possible commercial direction, not an approved pricing, source-license or upstream-base decision.

**Recommendation:** evaluate Backlog.md first, with the option to reuse selected components or integrate through its CLI rather than immediately forking the full app. It matches task files, local UI and agent access, and its TypeScript/React toolchain is adjacent to Glidebox's proposed stack. A full fork is justified only if a bounded evaluation shows it saves substantial work while accommodating the event model, workstreams, verification gates, approvals and cloud permissions. File-format similarity alone is insufficient. No architectural or dependency/license audit has yet been performed.

| Candidate | Verified top-level license | Implication for a possible paid Glidebox |
|---|---|---|
| Backlog.md / MrLesk | MIT | Allows use, modification, distribution, sublicensing and selling copies; preserve copyright and permission notices in copies/substantial portions. Does not itself require publishing Glidebox source. Dependencies and assets need their own review. |
| backlog / veggiemonk | Apache-2.0 | Permissive commercial reuse; distribution has license/notice and change-marking obligations, including NOTICE handling where applicable. Technical reuse value remains untested. |
| git-bug | GPLv3; README describes GPLv3-or-later | Commercial use is allowed. Distribution of a modified covered application brings corresponding-source and GPL obligations; whether a combined application is covered depends on integration. Server-only execution and distributing a local client are different cases. Prefer as reference unless choosing a compatible distribution model deliberately. |

Sources: [Backlog.md license](https://github.com/MrLesk/Backlog.md/blob/main/LICENSE), [backlog license](https://github.com/veggiemonk/backlog/blob/main/LICENSE), [git-bug license](https://github.com/git-bug/git-bug/blob/trunk/LICENSE), [GNU license FAQ](https://www.gnu.org/licenses/gpl-faq.en.html), [Backlog.md package manifest](https://github.com/MrLesk/Backlog.md/blob/main/package.json). Checked 2026-10-01. These are preliminary license observations; exact reused versions, dependencies, assets and distribution need review before a release. Calling a GPL program separately is not an automatic exemption for a tightly coupled combined product.

### Product and reuse boundaries to investigate

A paid service could provide managed Git hosting, team administration, authenticated approvals, backups, hosted agent execution and support while local project data remains portable. Those are business hypotheses, not features proven worth paying for. Charging for service does not require closing the local client's source; keep both open-client/paid-cloud and proprietary-client options available until the owner decides.

Suggested evaluation: pin a Backlog.md revision, use synthetic cards, check file identity and change history, test two independent clones with conflicting edits, inspect its programmatic boundaries, and map verification/approval fields without duplicating status. Report estimated work saved and the continuing cost of maintaining a fork. Compare this with a small Glidebox-owned Git/file core. No dependencies should be introduced or source copied merely on this recommendation.
