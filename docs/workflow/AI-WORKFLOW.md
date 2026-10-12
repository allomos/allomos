# AI workflow and handoff

How agent sessions (Claude Code, Codex) start, hand over and stop on Allomos. [AGENTS.md](../../AGENTS.md) holds the short, always-loaded version of these rules; this file holds the detail. Adapted from the owner's SSR migration project (`AI-WORKFLOW.md`, `AGENT_HANDOFF.md`; see [REFERENCES.md](../REFERENCES.md)).

## Roles and effort

- **Worker** (default): owns one card end to end — implementation, tests, docs, commits within the card's approval. Medium effort.
- **Focused reviewer / designer**: raised effort on the *same* model for one bounded hard question (architecture, security, data model, forecasting math, a risky diff). It returns a decision to the worker; it does not take over the card.
- Model and effort are separate controls. If an effort level cannot be set in the current tool, say so once and keep the actual setting. Never claim a model switch, an effort change or another model's review that did not happen.
- A worker given a delegated task receives: the card, exact file ownership, acceptance criteria, minimum checks and when to escalate. The delegate confirms its scope to the worker, not to the owner.

## Starting a session

1. Read [AGENTS.md](../../AGENTS.md) (loaded automatically).
2. Read [TASK_BOARD.md](../TASK_BOARD.md) (**Now**, **Waiting on the owner**), then run `alm list` against the taskbag at `../allomos-taskbag/.allomos/taskbag` ([CARDS.md § Where status lives](./CARDS.md#where-status-lives)).
3. Resuming a claimed card → open its card file and your block in [AGENT_HANDOFF.md § Active resume](../AGENT_HANDOFF.md#active-resume). Taking a new card → follow the [claiming protocol](./CARDS.md#claiming-protocol).
4. Read [AGENT_HANDOFF.md](../AGENT_HANDOFF.md) **Live state** and **Pitfalls** — they change what you do.
5. Check the actual state before trusting notes: `git status`, `git worktree list`, running ports (`lsof -iTCP -sTCP:LISTEN`).
6. Read only what the card and [AGENTS.md § Read by activity](../../AGENTS.md#read-by-activity) require.

## Handoff timing and contents

**When — continuously.** After every meaningful step — a task finished, a finding recorded, a commit, a process or container started or stopped — update your block in [AGENT_HANDOFF.md § Active resume](../AGENT_HANDOFF.md#active-resume) and commit it with the work. An interrupted session then loses at most one step.

**Usage limit.** Check remaining usage at step boundaries where the tool exposes it (Claude Code: `mcp__ccd_session_mgmt__get_usage`, 5-hour window). When close to **3% left**, start no new work: finish and commit the handoff, then stop and tell the owner.

**What — one block per claimed card, replaced in place, about 10 lines:**

- **Checkout:** worktree path, branch, last commit, pushed or not.
- **Card:** link; approvals still in force (and what they do *not* cover).
- **Uncommitted changes:** file by file, and why they are uncommitted.
- **Running processes:** what, port, how to restart, whether safe to stop.
- **Unrecorded findings:** anything learned that is not yet on the card or in evidence.
- **Next action:** the single next step.

**Keep it short:**

- Evidence goes on the card file, not in the handoff. Write "see card".
- Status goes in the taskbag only, with `alm`.
- When you release or finish a card, **delete your block** in the same commit. Nothing moves to a history file; git keeps it.
- Update **Live state** when you start or stop a server, create a hosted resource, or change a remote.
- Add to **Pitfalls** only a lesson that would change what the next agent does.
- Never write secrets. Credential *locations* go in **Current truth**.

## When to start a new session

The agent raises this itself; the owner should not have to ask.

- Remind the owner when context is **about 70% used** (Claude Code: `get_usage` → `context.percentUsed`; automatic summarising near the limit loses detail), or once past **half** at a natural break (card or task finished, waiting on the owner, nothing running, everything committed), or before a large new card when context is already heavy.
- **How:** update your handoff block and your task's status with `alm`, commit and push (agents push after every commit — [AGENTS.md § Working procedure](../../AGENTS.md#working-procedure)), then tell the owner briefly: why now, that the handoff is committed, what is still running — and give the exact prompt for the new session, e.g.:

  > Read AGENTS.md, then docs/TASK_BOARD.md, `alm show <card>` and your card, then docs/AGENT_HANDOFF.md (Active resume for `<card>`, Live state, Pitfalls), and continue with `<card>`: `<next action>`.

- Do not start the next task in the same message; the owner decides whether to switch.

## Switching provider (Claude ↔ Codex) or environment

- Both providers read the same [AGENTS.md](../../AGENTS.md); `CLAUDE.md` only imports it. Never maintain competing copies of rules.
- A running agent is not assumed to have reloaded edited rules; it rereads them at its next assignment or resume.
- Before the new environment writes anything, establish which files, worktree and running processes the old session still owns (from its handoff block and `git worktree list`).
- For **claude.ai (web) projects**: a local `CLAUDE.md` is not loaded automatically. Add `AGENTS.md`, this file and the current board as project knowledge, and paste an instruction to follow them. Never upload `.env` files, tokens or a whole working directory.
- A switch preserves decisions, approvals, evidence and consumed attempts; it does not authorize repeating operations.

## After Allomos can track itself (dogfooding)

**Decided by the owner, 2026-10-04:** Allomos's own tasks are Iteration 1's first real project. When Iteration 1 (v0.1, macOS) passes its exit check ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)), Allomos's own cards move from Markdown into a taskbag inside this repository. This workflow then changes: claims, labels and waiting-on are written with `alm` instead of editing the board, and `TASK_BOARD.md` is retired or generated. The move is the last v0.1 card. **Done 2026-10-06** ([`Task-A01-07`](../tasks/Task-A01-07-own-cards-in-seabag.md)), slightly ahead of the full exit check, at the owner's request: the taskbag is authoritative and the rules are in [CARDS.md](./CARDS.md#where-status-lives).
