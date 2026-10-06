# Cards, labels and claiming

A **contract** for how work is recorded on glidebox while the board is Markdown. [TASK_BOARD.md](../TASK_BOARD.md) is the index; each card's detail is its own file in [docs/tasks/](../tasks/).

## Areas

An **area** is *what part of the product* a card belongs to. Areas are named after what users can do, not after technical layers (owner, 2026-10-04). An **iteration** is *when* a card is planned ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). Every card has exactly one area.

| ID | Area | What users can do | Iterations |
|---|---|---|---|
| A00 | Planning | Read the plan, requirements, decisions and research | all |
| A01 | Tracking | Keep tasks in a task box, view and edit them, use the `glidebox` / `gb` command, commit and push from the app | 1, 2, 6 |
| A02 | Progress & forecast | See progress, what waits on them, burndown, finish-date forecast, timeline; export a report | 4, 5, 12 |
| A03 | Agents | Let Claude Code and Codex work through MCP; start and open agent sessions from the app | 3 (MCP), 8 (start/open agents) |
| A04 | Companions | View progress and approve from a browser or phone, at home or away | 9, 10 |
| A05 | Project setup & import | Import the SSR board; set up a project with an AI agent | 7, 11 |
| A06 | Cloud & team | Keep a project in the cloud; invite teammates with roles | 13, 14 |
| A07 | Platform & release | Use glidebox on Windows as well as macOS; install signed builds that update themselves | 2, 15 |

A card that touches several layers (for example interface, Rust rules and files) still has **one** area: the capability it delivers.

Add an area by editing this table and the board in the same commit.

**Renamed 2026-10-04.** Until then, "workstream" (W00–W10, grouped by technical layer) was used:

| Old card ID | New card ID |
|---|---|
| `Task-W00-01` … `Task-W00-14` | `Task-A00-01` … `Task-A00-14` (same numbers) |
| `Task-W04-01-walking-skeleton` | `Task-A01-01-walking-skeleton` |
| `Task-W01-01-core-model` | `Task-A02-01-burndown-and-forecast` |
| `Task-W07-01-agent-launch-research` | `Task-A03-01-agent-launch-research` |
| `Task-W10-01-remote-access-research` | `Task-A04-01-remote-access-research` |
| `Task-W02-01-ssr-board-import` | `Task-A05-01-ssr-board-import` |

## Card IDs and files

- **ID:** `Task-A{2-digit area}-{2-digit task}-{short-description}`, e.g. `Task-A02-01-burndown-and-forecast`. A child extends its parent with another two-digit number: `Task-A02-01-02-forecast-monte-carlo`.
- **One card = one observable outcome** — a sentence about what a person (or agent) can do afterwards, not a layer or component. Split by user-visible step, never by layer. If more than about four documents must be read before the first line of code, split.
- **File:** `docs/tasks/Task-A…-….md`, created from [the template](#card-template). The file never moves; only its board row changes.

## Where status lives

**Only on the board row** (State, Label, Session, Next dependency). The card file holds the outcome, scope, approvals and dated progress notes — never a copy of the state. When state changes, edit the board row; the card gets a dated note only if there is something to explain.

## States and labels

- **State:** `backlog` → `ready` (owner reviewed the card) → `in progress` → `waiting` (on the owner or another card) → `done`; or `dropped`.
- **Label**, the only progress claim, for work that ships:
  1. **implemented** — code written, may not run end to end
  2. **locally verified** — the card's named checks pass locally, in the complete local app
  3. **deployed and verified** — running on the approved hosted target, hosted checks pass (only once a hosting target exists)
  4. **owner verified** — the owner checked it; the card may move to Done
- Documentation or decision cards use: **drafted** → **owner accepted**.
- Every open card names its **single next dependency** (what blocks the next label) and who it waits on.
- Passing test counts support a label; they are never a label.

## Approvals

- The owner approving a card (`ready`) authorizes all its **local** work: source, tests, docs, commits.
- Every **external operation** needs its own exact approval, recorded in the card's Approvals section: target, operation, cleanup. See [AGENTS.md § Always-on boundaries](../../AGENTS.md#always-on-boundaries).

## Claiming protocol

One card is worked by exactly one session at a time. The board, not memory or chat, records who holds what.

1. **Pull before claiming** (when a remote exists) and read the latest board.
2. **Claim in a commit of its own.** Pick a `ready` card with no session. Write your session name (branch or worktree name, e.g. `claude/w01-core-model-3f2a`) into the card's board row and the **Sessions** table, commit only that change, then start work.
3. **A card claimed by another session is read-only to you.** If it holds a stale fact, correct only that fact and leave the Session untouched.
4. **Two claims at once:** the second to land gets a merge conflict on that row — keep whichever landed first and claim a different card.
5. **Release** a card you stop short of Done: clear its Session to `—`, remove your Sessions row and your handoff block, and set State/Next dependency so the next session knows what it is picking up.
6. **Clean up after a merge.** Once your card's branch is merged into `main` and pushed, and the owner no longer needs the app built in it for a check: stop anything still running from the worktree, then remove **your own** worktree (`git worktree remove .claude/worktrees/<name>`, which also deletes its `target/`) and delete the merged local branch (`git branch -d <branch>`). Never remove another session's worktree. The branch on GitHub stays unless the owner asks. Rules for build output: [CODING_RULES.md § Build output](./CODING_RULES.md#build-output-target).
7. **Now** is shared text: rewrite it only when a card is claimed or finished, never for mid-flight progress.

## Now — rules

**Now** answers two questions only: *what is true of the product today* and *what is next*. About 300 words.

- Finishing a card removes its paragraph from Now in the same commit; one row goes to **Recently finished**.
- Now never repeats what a card says; every line names a card that exists.
- Headings name states (Now, Waiting on the owner, Blocked, Done), never topics.
- Before committing a board edit, read Now from the top; if two lines disagree, correct the older one in place.

## Card template

```markdown
# Task-A00-00-short-description

*Card type: coding / research / decision / docs.*

## What gets built (plain words)

A short table or list: each piece, what it is in everyday words, and where it lives. What the owner can do or see at the end.

## Details for the worker

- **Outcome.** One or two plain sentences: what a person or agent can do when this is done.
- **Decisive unknowns.** Facts to find out (not design), each with who can answer.
- **Files owned.** Paths this card creates or changes; shared files named explicitly.
- **External operations.** Exact targets, operations and cleanup that will need approval — or "none".
- **Done means mechanically.** The checks and demonstrations that prove the outcome.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

Dated, plain-language progress notes and evidence links. Newest last.
```
