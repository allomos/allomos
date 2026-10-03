# 2026-10-03 — handoff to the next agent

This restart snapshot records facts at handoff time. [The task board](../TASK_BOARD.md) remains the only home of card status.

## Start here

1. Read [AGENTS.md](../../AGENTS.md), then [the board](../TASK_BOARD.md): Now, Sessions and Waiting on the owner.
2. Read [AGENT_HANDOFF.md](../AGENT_HANDOFF.md): Live state, Pitfalls and Current truth.
3. Read the card you claim before editing. Claim an unclaimed `ready` card in a commit of its own. The board has no claimed card after this handoff.
4. Use the existing worktree or create an isolated worktree for the selected card. Reference projects named in [REFERENCES.md](../REFERENCES.md) are read-only.

## Product state

Glidebox is a planning-only repository: no application code, dependencies, processes, hosted resources, credentials or Git remote exist. The main checkout is `/Users/wanghui/coding/glidepace/glidebox` on `main`. The reusable worktree is `/Users/wanghui/coding/glidepace/glidebox-local-cloud` on `codex/local-cloud-requirements`. Both point to the latest local documentation commit at this handoff. There is nothing to push.

The owner accepted these needs: local-only projects without sign-in, a cloud-project option that requires sign-in, first-class interfaces for people and AI agents, desktop/mobile human use with browser/human CLI considered, structured agent updates by CLI/MCP, and generation of project/task files by an external agent from a Glidebox-supplied prompt. Glidebox also tracks workstreams, tasks, events, progress, charts, burndown/forecast, roles/invitations and agent sessions.

Read [requirements](../product/REQUIREMENTS.md) rather than relying only on this summary. Requirements marked **accepted** are commitments; **proposed** entries await owner review.

## Recent work worth reading

| Topic | Read this | What it establishes |
|---|---|---|
| Human and agent interfaces | [interface contract](../product/INTERFACES.md), [internet investigation](../research/interface-patterns.md) | Desktop/mobile scope and browser/CLI consideration are accepted. Shared commands, adaptive layouts, tool shapes, error/retry behavior and packaging are proposed. Public-source research only; competitors were not operated. |
| Local/cloud and Git question | [0006](../decisions/0006-local-and-cloud-projects.md), [0007](../decisions/0007-git-like-project-sync.md) | Local/cloud requirements are accepted. Actual Git plus structured task files is a candidate only; it does not supersede the database decision. |
| AI-generated project setup | [feature contract](../product/AI-GENERATED-PROJECTS.md) | The owner-requested workflow is accepted. Prompt/schema, validator, preview and adoption details are proposed. |
| Competitors and possible reuse | [competitor investigation](../research/competitor-investigation.md) | Feature-first comparison with Glidebox in the last column. Includes actual-Git examples, preliminary license evidence and human-platform rows. Treat unknown cells as unknown. |
| Overall plan | [plan](../product/PLAN.md) | Product goals, M0–M6 and the currently proposed architecture. |

## Decisions and boundaries

Decision 0001, database as source of truth, remains accepted until the owner explicitly supersedes it. Decision 0007 must not be implemented as accepted. Decision 0003, the relationship to Glidelines, remains open. Do not choose cloud hosting, authentication details, interface packaging/release order, a mobile runtime, Git storage, or an open-source base without an owner decision.

Do not make external changes without the owner's exact approval recorded on a card: services, databases, deployments, secrets, email, new remotes and pushes. Preserve unrelated work. Do not print or commit credentials.

## Practical next step

The planning documents await owner review under [`Task-W00-01`](../tasks/Task-W00-01-planning-docs.md). Once the owner reviews the first core-model card, [`Task-W01-01`](../tasks/Task-W01-01-core-model.md) is safe under every option in open decision 0003. It must still be claimed before work begins.

If the owner instead asks to continue interface work, create a bounded card first: for example, a common task-command/schema contract, a local runtime feasibility spike, or a desktop/mobile task-detail prototype. Do not build all clients at once or treat proposed architecture as accepted scope.

## Handoff checks

Documentation checks passed: relative paths, table columns, selected section anchors, requirement-ID uniqueness and whitespace. Runtime, prototype, competitor-operation and accessibility tests were not run because the repository contains documentation only. Verify the working tree and board again before changing anything; both are live state.
