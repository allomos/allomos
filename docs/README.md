# Allomos documentation

**Agents:** start at [../AGENTS.md](../AGENTS.md). **People:** start with [product/PLAN.md](./product/PLAN.md) and [TASK_BOARD.md](./TASK_BOARD.md).

## Map

| Where | What | Kind |
|---|---|---|
| [TASK_BOARD.md](./TASK_BOARD.md) | Now and what waits on the owner. Card status lives in the taskbag (`../allomos-taskbag/.allomos/taskbag/`, read with `alm list` or the app). | status |
| taskbag `handoffs/` and `project-notes.md` (in `../allomos-taskbag/.allomos/taskbag/`, outside this repository) | Each session's handoff (`alm handoffs`), and what every session must know: running, pitfalls, where credentials live, current truth. Replaced `AGENT_HANDOFF.md` on 2026-10-12 | handoff |
| [guide/](./guide/) | The user guide in English, Chinese and Japanese, built with Astro Starlight; published at https://allomos.github.io/allomos/ once approved (`A07-10`) | user guide |
| taskbag `tasks/<id>/description.md` (outside this repository) | Each task's full detail: outcome, details, rules, approvals, and evidence under an **Evidence** heading (`alm show <id>`, written with `alm describe`). Replaced the card files in `docs/tasks/` on 2026-10-16 (`A00-30`); those card files now sit beside their tasks as `tasks/<id>/card.md`, linked from the description (2026-10-17) | tasks |
| **product/** | | |
| [product/PLAN.md](./product/PLAN.md) | Problem, goals, data model, views, burndown, architecture, milestones | plan |
| [product/REQUIREMENTS.md](./product/REQUIREMENTS.md) | Numbered requirements (R1…) cited by cards and tests | contract |
| [product/AI-GENERATED-PROJECTS.md](./product/AI-GENERATED-PROJECTS.md) | Leader → supplied prompt → external agent → generated files → Allomos project; proposed validation/adoption rules | contract |
| [product/INTERFACES.md](./product/INTERFACES.md) | Human desktop/mobile scope, browser/CLI consideration and proposed shared task operations for GUI, agents and imports | contract |
| **workflow/** | | |
| [workflow/CODING_RULES.md](./workflow/CODING_RULES.md) | Rules for this repository's code: layout, versions, checks, how `alm` behaves, running and building the app. How agents work is told by `alm` and [AGENTS.md](../AGENTS.md) | contract |
| [decisions/](./decisions/README.md) | One record per decision, with status (open / proposed / accepted) | decisions |
| [discussions/](./discussions/) | Dated records of planning discussions with the owner | history |
| [analysis/](./analysis/) | Analyses of reference material | analysis |
| [research/competitor-investigation.md](./research/competitor-investigation.md) | Competitor feature matrix, sources, limitations and product hypotheses | evidence |
| [research/interface-patterns.md](./research/interface-patterns.md) | Internet evidence for human/agent interfaces, adaptive layouts, CLI/MCP, local access and platform packaging options | evidence |
| [research/desktop-frameworks.md](./research/desktop-frameworks.md) | Detailed comparison of six desktop frameworks for decision 0011 (2026-10-04) | evidence |
| [research/remote-access.md](./research/remote-access.md) | How companions reach the desktop from outside, and which service sends the pairing email (2026-10-07) | evidence |
| [research/agent-launch-and-resume.md](./research/agent-launch-and-resume.md) | How Allomos can start Claude Code and Codex sessions and reopen them, per surface (2026-10-07) | evidence |
| [pitch/](./pitch/README.md) | Road show deck in English and Chinese: sources and published addresses | pitch |
| [REFERENCES.md](./REFERENCES.md) | External projects and files to learn from (read-only) | reference |
| [GLOSSARY.md](./GLOSSARY.md) | Plain meanings of terms | reference |

## Discussions

- [2026-09-27 — Kickoff](./discussions/2026-09-27-kickoff.md): problem, requirements, decisions 0001–0005
- [2026-09-30 — Documents and agent workflow](./discussions/2026-09-30-docs-and-agent-workflow.md): documentation structure and handoff rules
- [2026-10-01 to 10-03 — Owner requests](./discussions/2026-10-01-to-03-owner-requests.md): local/cloud projects, competitors, Git sync, selling a service, AI-generated projects, interfaces (reconstructed from card notes)
- [2026-10-03 — Agent handoff](./discussions/2026-10-03-agent-handoff.md): the owner asked for a handoff to the next agent
- [2026-10-04 — Documentation review](./discussions/2026-10-04-docs-review.md): review findings, "build our own", desktop hub with web and mobile companions

## Rules for documents

- A new file only for a **contract**, **evidence**, a **decision record**, a **discussion record**, or material the owner asks for, such as the [road show deck](./pitch/README.md) ([AGENTS.md § Working procedure](../AGENTS.md#working-procedure)). Status goes in the taskbag, with `alm`.
- Supersede by editing the stale line in place; do not add a companion document.
- Evidence of what actually ran goes under `docs/research/` or `docs/evidence/`, linked from the task's description.
