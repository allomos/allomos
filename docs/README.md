# Glidebox documentation

**Agents:** start at [../AGENTS.md](../AGENTS.md). **People:** start with [product/PLAN.md](./product/PLAN.md) and [TASK_BOARD.md](./TASK_BOARD.md).

## Map

| Where | What | Kind |
|---|---|---|
| [TASK_BOARD.md](./TASK_BOARD.md) | What is being worked on, by whom, and what waits on the owner. **The only home of status.** | status |
| [AGENT_HANDOFF.md](./AGENT_HANDOFF.md) | Restart mechanics for the next agent session: active resume, live state, pitfalls, current truth | handoff |
| [tasks/](./tasks/) | One file per card: outcome, scope, approvals, notes | cards |
| **product/** | | |
| [product/PLAN.md](./product/PLAN.md) | Problem, goals, data model, views, burndown, architecture, milestones | plan |
| [product/REQUIREMENTS.md](./product/REQUIREMENTS.md) | Numbered requirements (R1…) cited by cards and tests | contract |
| [product/AI-GENERATED-PROJECTS.md](./product/AI-GENERATED-PROJECTS.md) | Leader → supplied prompt → external agent → generated files → Glidebox project; proposed validation/adoption rules | contract |
| **workflow/** | | |
| [workflow/AI-WORKFLOW.md](./workflow/AI-WORKFLOW.md) | Agent roles and effort, starting a session, handoff timing and contents, new-session rule, switching providers | contract |
| [workflow/CARDS.md](./workflow/CARDS.md) | Workstreams, card IDs and template, states and labels, approvals, claiming protocol, Now rules | contract |
| [decisions/](./decisions/README.md) | One record per decision, with status (open / proposed / accepted) | decisions |
| [discussions/](./discussions/) | Dated records of planning discussions with the owner | history |
| [analysis/](./analysis/) | Analyses of reference material | analysis |
| [research/competitor-investigation.md](./research/competitor-investigation.md) | Competitor feature matrix, sources, limitations and product hypotheses | evidence |
| [REFERENCES.md](./REFERENCES.md) | External projects and files to learn from (read-only) | reference |
| [GLOSSARY.md](./GLOSSARY.md) | Plain meanings of terms | reference |

## Discussions

- [2026-09-27 — Kickoff](./discussions/2026-09-27-kickoff.md): problem, requirements, decisions 0001–0005
- [2026-09-30 — Documents and agent workflow](./discussions/2026-09-30-docs-and-agent-workflow.md): documentation structure and handoff rules

## Rules for documents

- A new file only for a **contract**, **evidence**, a **decision record** or a **discussion record** ([AGENTS.md § Working procedure](../AGENTS.md#working-procedure)). Status goes on the board.
- Supersede by editing the stale line in place; do not add a companion document.
- Evidence of what actually ran goes under `docs/research/` or `docs/evidence/`, linked from the card.
