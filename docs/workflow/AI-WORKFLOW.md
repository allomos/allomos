# AI workflow and handoff

How agent sessions (Claude Code, Codex) start, hand over and stop on Allomos. [AGENTS.md](../../AGENTS.md) holds the short, always-loaded version of these rules; this file holds the detail. Adapted from the owner's SSR migration project (`AI-WORKFLOW.md`, `AGENT_HANDOFF.md`; see [REFERENCES.md](../REFERENCES.md)). Since 2026-10-12 handoffs live in the taskbag ([`Task-A03-06`](../tasks/Task-A03-06-agent-handoff-rules.md)); `docs/AGENT_HANDOFF.md` is retired and its history stays in Git.

## Roles and effort

- **Worker** (default): owns one card end to end — implementation, tests, docs, commits within the card's approval. Medium effort.
- **Focused reviewer / designer**: raised effort on the *same* model for one bounded hard question (architecture, security, data model, forecasting math, a risky diff). It returns a decision to the worker; it does not take over the card.
- Model and effort are separate controls. If an effort level cannot be set in the current tool, say so once and keep the actual setting. Never claim a model switch, an effort change or another model's review that did not happen.
- A worker given a delegated task receives: the card, exact file ownership, acceptance criteria, minimum checks and when to escalate. The delegate confirms its scope to the worker, not to the owner.

## Starting a session

1. Read [AGENTS.md](../../AGENTS.md) (loaded automatically).
2. Read [TASK_BOARD.md](../TASK_BOARD.md) (**Now**, **Waiting on the owner**), then run `alm list` against the taskbag at `../allomos-taskbag/.allomos/taskbag` ([CARDS.md § Where status lives](./CARDS.md#where-status-lives)).
3. Resuming a card → open its card file and its latest handoff (`alm show <id>`, or `alm handoffs --task <id>`). Taking a new card → follow the [claiming protocol](./CARDS.md#claiming-protocol).
4. Read the taskbag's `project-notes.md` (**Running** and **Pitfalls**) — they change what you do.
5. Check the actual state before trusting notes: `git status`, `git worktree list`, running ports (`lsof -iTCP -sTCP:LISTEN`).
6. Read only what the card and [AGENTS.md § Read by activity](../../AGENTS.md#read-by-activity) require.

## Handoff timing and contents

**When.** `alm handoff-rules` gives the project's rules; `alm next` repeats them for you at the start of a session (with any override for your agent or account). For Allomos: after every meaningful step — a task finished, a finding recorded, a commit, a process or container started or stopped — so an interrupted session loses at most one step; near **3% left** in the 5-hour window (Claude Code: `mcp__ccd_session_mgmt__get_usage`), start no new work, finish the handoff and stop; at about **70% context used**, see [When to start a new session](#when-to-start-a-new-session).

**Where.** `alm handoff --as <your session name> --tasks <id,id> --push`: the first call makes this session's file in the taskbag's `handoffs/` from the template and prints its path; later calls update the same file (`--tasks` changes the tasks it covers, `--from <file>` its text). Fill it in directly: the one file in the taskbag you edit by hand. Push it with your next `alm … --push` or `git -C ../allomos-taskbag` commit.

**What.** The template, in two parts, written for any reader (you do not know who takes over, an agent or a person):

- **For anyone, in plain words:** what was being done, where it stopped, what is verified and what is not, what waits on whom, the single next step.
- **Restart details:** checkout (worktree, branch, last commit, pushed or not), uncommitted changes and why, what is running (port, how to restart, safe to stop), approvals still in force (and what they do *not* cover).

**Keep it short:**

- Evidence goes on the card file, not in the handoff. Write "see card".
- Status goes in the taskbag's tasks, with `alm`; a one-line `alm release --note` still points the next holder at the handoff.
- What every session must know, whatever its task, goes in the taskbag's `project-notes.md`: **Running** when you start or stop a server, create a hosted resource or change a remote; **Pitfalls** only for a lesson that would change what the next agent does; **Where credentials live**, never their values. Change lines in place.

## When to start a new session

The agent raises this itself; the owner should not have to ask.

- Remind the owner when context is **about 70% used** (Claude Code: `get_usage` → `context.percentUsed`; automatic summarising near the limit loses detail), or once past **half** at a natural break (card or task finished, waiting on the owner, nothing running, everything committed), or before a large new card when context is already heavy.
- **How:** update your handoff (`alm handoff`) and your task's status with `alm`, commit and push (agents push after every commit — [AGENTS.md § Working procedure](../../AGENTS.md#working-procedure)), then tell the owner briefly: why now, that the handoff is committed, what is still running — and give the exact prompt for the new session, e.g.:

  > Read AGENTS.md, then docs/TASK_BOARD.md, `alm show <card>` (its latest handoff) and your card, then the taskbag's project-notes.md (Running, Pitfalls), and continue with `<card>`: `<next action>`.

- Do not start the next task in the same message; the owner decides whether to switch.

## Switching provider (Claude ↔ Codex) or environment

- Both providers read the same [AGENTS.md](../../AGENTS.md); `CLAUDE.md` only imports it. Never maintain competing copies of rules.
- A running agent is not assumed to have reloaded edited rules; it rereads them at its next assignment or resume.
- Before the new environment writes anything, establish which files, worktree and running processes the old session still owns (from its handoff and `git worktree list`).
- For **claude.ai (web) projects**: a local `CLAUDE.md` is not loaded automatically. Add `AGENTS.md`, this file and the current board as project knowledge, and paste an instruction to follow them. Never upload `.env` files, tokens or a whole working directory.
- A switch preserves decisions, approvals, evidence and consumed attempts; it does not authorize repeating operations.

## After Allomos can track itself (dogfooding)

**Decided by the owner, 2026-10-04:** Allomos's own tasks are Iteration 1's first real project. When Iteration 1 (v0.1, macOS) passes its exit check ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)), Allomos's own cards move from Markdown into a taskbag inside this repository. This workflow then changes: claims, labels and waiting-on are written with `alm` instead of editing the board, and `TASK_BOARD.md` is retired or generated. The move is the last v0.1 card. **Done 2026-10-06** ([`Task-A01-07`](../tasks/Task-A01-07-own-cards-in-seabag.md)), slightly ahead of the full exit check, at the owner's request: the taskbag is authoritative and the rules are in [CARDS.md](./CARDS.md#where-status-lives).
