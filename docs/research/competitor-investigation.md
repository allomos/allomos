# Competitor investigation

Public-source research conducted 2026-09-30; saved 2026-10-01. Glidebox's column incorporates the owner's local/cloud requirement accepted 2026-10-01. This is research evidence and analysis, not an approval of a product direction or architecture.

## Finding

Glidebox overlaps substantially with existing project trackers and coding-agent workspaces. Boards, agent launches, collaboration and finish-date predictions are already available. Its most promising areas to validate are tracking verified delivery, recording exact decisions owed by people, and preserving task context across agent sessions.

Vibe Kanban is the closest task-board comparison; Linear is the strongest broad substitute; Conductor is both a session-management competitor and a possible integration target. These rankings are judgments based on the documented features below, not hands-on results.

## Method and limits

The investigation compared official vendor documentation with Glidebox's [plan](../product/PLAN.md) and [requirements](../product/REQUIREMENTS.md). It did not install or operate competitors, create accounts, or test workflows. Some vendor pages could not be retrieved; the readable documentation index or search extracts supported only limited claims. Pricing, license terms, adoption, security guarantees and API suitability were not comprehensively investigated.

**Legend:** ✓ = documented; **Partial** = related functionality documented but the full feature was not established; **?** = not established by this investigation, not evidence of absence. **Required** and **Proposed** describe Glidebox's planning commitments, not implemented capabilities. Source numbers link to the source register below.

## Feature comparison

| Feature | Vibe Kanban [1–3] | Conductor [4–7] | Linear [8–11] | GitHub Agent HQ [12] | Plane [13–14] | ActionableAgile [15] | **Glidebox** |
|---|---|---|---|---|---|---|---|
| Structured task tracking with lists or boards | ✓ Issues, boards, lists | Partial — workspaces linked to issues | ✓ Issues and projects | ✓ Existing GitHub issue workflow | ✓ Work items and projects | Analytics focus | **Required** |
| Workstream grouping and task hierarchy | ? Exact mapping not checked | ? Exact mapping not checked | Partial — projects and sub-issues | ? Exact mapping not checked | Partial — projects; exact hierarchy not checked | ? | **Required** — workstreams and hierarchical IDs |
| Online team collaboration | ✓ Live updates and invitations | ✓ Shared cloud workspaces | ✓ Team tracker | ✓ GitHub collaboration | ✓ Team project tracker | ? Task editing not checked | **Required** — cloud projects |
| Agent-accessible MCP / API | MCP listed; tool coverage not checked | ✓ Workspace and session tools | ✓ Tracker MCP | Partial — agent issue workflow; external control API not checked | ✓ Work-item and project MCP | ? | **Required** — structured CLI/MCP updates |
| Start work in Claude Code or Codex | ✓ Both integrations listed | ✓ Both supported | Partial — coding sessions; exact provider options not checked | ✓ Both documented | ? | ? | **Required** — both providers |
| Return to existing agent work | ✓ Switch conversations | ✓ Sessions and workspace links | ✓ Active coding sessions | ✓ Session management | ? | ? | **Required** |
| Session activity and attention states | ✓ Running, Idle, Needs Attention | ✓ Session status API/MCP | ✓ Coding-session status | ✓ Progress and logs | ? | ? | **Proposed** |
| Project progress and scope charts | ? | ? | ✓ Scope and progress graph | ? Project analysis not checked | ? Relevant charts not checked | ✓ Flow analytics | **Required** — charts and burndown |
| Project finish-date prediction | ? | ? | ✓ Historical velocity | ? | ? | ✓ Monte Carlo forecasting | **Required** — date range |
| Probability-based throughput forecasting | ? | ? | Partial — velocity with date buffers | ? | ? | ✓ Simulation and probabilities | **Required** — 50% / 85% dates; simulation method proposed |
| Separate implementation and verification gates | ? Dedicated ladder not established | Partial — tests, review, checks | Partial — workflow and review | Partial — review and checks | ? Dedicated ladder not established | ? | **Proposed** — configurable verification ladder |
| Exact approvals and checks owed by a person | Partial — Needs Attention | Partial — agent permissions | Partial — priority inbox and conditional approvals | Partial — review workflow | ? | ? | **Proposed** — waiting-on-me inbox |
| Enforced one-task / one-holder claiming | ? | ? Workspace isolation is not a claim guarantee | ? | ? | ? | ? | **Proposed** — atomic claim |
| Task continuity across session changes | Partial — histories and shared files | Partial — workspace context and session records | Partial — issue context and sessions | Partial — issues, sessions, PRs | Partial — durable work-item context | ? | **Intended outcome** — task and event model |
| Import the SSR Markdown board | ? Exact format not checked | ? | ? Exact format not checked | ? Exact format not checked | ? Exact format not checked | ? Exact format not checked | **Required** — dedicated importer |
| Local project/task storage without cloud sign-in | ? End-to-end condition not checked | Partial — local workspaces and local session data; complete tracker not checked | ? | ? | Partial — self-hosting is documented; account-free local use not established | ? | **Required** — R40–R43 |
| Optional cloud projects requiring sign-in | ✓ Cloud authentication documented in index | ✓ Cloud organizations and authenticated API | ✓ Cloud workspace and authenticated MCP | ✓ GitHub account workflow | ✓ Cloud and self-hosted options; exact mode selection not checked | ? | **Required** — R42 |

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

Sources were consulted during the 2026-09-30 investigation; saving this document does not represent a new live verification. Links can change.

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
