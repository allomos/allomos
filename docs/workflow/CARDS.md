# Cards, labels and claiming

A **contract** for how work is recorded on Allomos. Since [`Task-A01-07`](../tasks/Task-A01-07-own-cards-in-seabag.md) (2026-10-06) Allomos tracks itself: **status lives in this repository's taskbag** (`.allomos/taskbag/` on `main`), changed with the `alm` command ([CODING_RULES § The `alm` command](./CODING_RULES.md#the-fh-command)). Each card's detail is its own file in [docs/tasks/](../tasks/). [TASK_BOARD.md](../TASK_BOARD.md) keeps only **Now** and **Waiting on the owner**, until it is removed or generated from the taskbag (owner, 2026-10-06).

## Areas

An **area** is *what part of the product* a card belongs to. Areas are named after what users can do, not after technical layers (owner, 2026-10-04). An **iteration** is *when* a card is planned ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). Every card has exactly one area.

| ID | Area | What users can do | Iterations |
|---|---|---|---|
| A00 | Planning | Read the plan, requirements, decisions and research | all |
| A01 | Tracking | Keep tasks in a taskbag, view and edit them, use the `allomos` / `alm` command, commit and push from the app | 1, 2, 6 |
| A02 | Progress & forecast | See progress, what waits on them, burndown, finish-date forecast, timeline; export a report | 4, 5, 12 |
| A03 | Agents | Let Claude Code and Codex work through MCP; start and open agent sessions from the app | 3 (MCP), 8 (start/open agents) |
| A04 | Companions | View progress and approve from a browser or phone, at home or away | 9, 10 |
| A05 | Project setup & import | Import the SSR board; set up a project with an AI agent | 7, 11 |
| A06 | Cloud & team | Keep a project in the cloud; invite teammates with roles | 13, 14 |
| A07 | Platform & release | Use Allomos on Windows as well as macOS; install signed builds that update themselves | 2, 15 |

A card that touches several layers (for example interface, Rust rules and files) still has **one** area: the capability it delivers.

**Iterations in the taskbag** ([`Task-A01-15`](../tasks/Task-A01-15-iterations-view.md), 2026-10-08): Allomos's own taskbag holds the 15 iterations of [PLAN § 9](../product/PLAN.md#9-roadmap--releases) as `I01` … `I15` (Iteration 15 produces v1.0). When a card is added to the roadmap, plan its task too: `alm plan <ID> I02 --push`. Ideas not yet on the roadmap stay unplanned.

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

**Only in the taskbag on `main`**: each card is a task with the card's number as its ID (`A01-07`), and its state, label, holder and waiting-on are that task's fields. The card file holds the outcome, scope, approvals and dated progress notes — never a copy of the state. The task's `description.md` links to the card file.

- **Change status only with `alm`, only on `main`.** Card branches hold an old copy of the taskbag; never change it there. From a card's worktree, point `alm` at the main checkout: `export ALLOMOS_TASKBAG=<main checkout>` (for example `/Users/wanghui/coding/glidepace/fliphopper`). Add `--push` to every status change: `alm` then commits only the taskbag's files on `main` and pushes them (`alm label A01-05 "locally verified" --push`). Check the output for a `commit_failed` or `push_failed` warning; if one appears, commit or push by hand.
- **Who you are:** `export ALLOMOS_HOLDER=<your branch>` (for example `claude/a01-05-readme`), or `--as <branch>` on each command.
- **Next dependency:** `alm needs <ID> "<the one thing needed for the next label>"`. For a waiting task, its *waiting on* says it.
- **Seeing status:** `alm list` (all, by area), `alm list --state in_progress` (who holds what — this replaces the old Sessions table), `alm list --state done`, `alm show <ID>`, `alm next`. The app shows the same when it opens this repository.

## States and labels

- **State:** `backlog` → `ready` (owner reviewed the card) → `in progress` → `waiting` (on the owner or another card) → `done`; or `dropped`.
- **Label**, the only progress claim, for work that ships:
  0. **drafted** — a document or decision card is written for review
  1. **implemented** — code written, may not run end to end
  2. **locally verified** — the card's named checks pass locally, in the complete local app
  3. **deployed and verified** — running on the approved hosted target, hosted checks pass (only once a hosting target exists)
  4. **owner verified** — the owner checked it; the card may move to Done
- Documentation or decision cards use **drafted** → **owner verified** (the owner accepted it). This is one ladder for every card, set in the taskbag's `project.json` (owner, 2026-10-06).
- Every open card names its **single next dependency** (what blocks the next label) and who it waits on.
- Passing test counts support a label; they are never a label.

## Approvals

- The owner approving a card (`ready`) authorizes all its **local** work: source, tests, docs, commits.
- Every **external operation** needs its own exact approval, recorded in the card's Approvals section: target, operation, cleanup. See [AGENTS.md § Always-on boundaries](../../AGENTS.md#always-on-boundaries).

## Claiming protocol

One card is worked by exactly one session at a time. The taskbag on `main`, not memory or chat, records who holds what. Every step below is run against the main checkout's taskbag (see [Where status lives](#where-status-lives)).

1. **Pull before claiming:** `git -C <main checkout> pull --ff-only`, then `alm next` (or `alm list --state ready`).
2. **Claim in a commit of its own.** `alm claim <ID> --as <branch> --push`, then start work. `alm claim` refuses a task someone else holds (`task_held`), and the lock stops two sessions on this computer from both winning.
3. **A card held by another session is read-only to you.** If its card file holds a stale fact, correct only that fact. `alm claim --take` is only for a session whose holder has stopped (for example its quota ran out) and says so in the handoff.
4. **Two claims from different computers:** the second to push gets a merge conflict in that task's `task.json` — keep whichever landed first and claim a different card.
5. **Labels and waiting:** `alm label <ID> "<label>" --push` and `alm wait <ID> --on "<what>" --push` on `main`.
6. **Finish** after the owner's check: `alm label <ID> "owner verified" --push`, `alm done <ID> --push`.
7. **Release** a card you stop short of Done: `alm release <ID> --note "<what the next session needs>" --push`, then remove your handoff block, commit and push.
8. **Clean up after a merge.** Once your card's branch is merged into `main` and pushed, and the owner no longer needs the app built in it for a check: stop anything still running from the worktree, then remove **your own** worktree (`git worktree remove .claude/worktrees/<name>`, which also deletes its `target/`) and delete the merged local branch (`git branch -d <branch>`). Never remove another session's worktree. The branch on GitHub stays unless the owner asks. Rules for build output: [CODING_RULES.md § Build output](./CODING_RULES.md#build-output-target).
9. **A new card:** write its file in `docs/tasks/`, then `alm add task <ID> --area <area> --title "<title>" --description "Card: [docs/tasks/<file>](../../../../docs/tasks/<file>)"` on `main`; it starts in `backlog`. When the owner approves it: `alm ready <ID>`. A card that will not be done: `alm drop <ID>`.
10. **Now** is shared text: rewrite it only when a card is claimed or finished, never for mid-flight progress.

## Now — rules

**Now** answers two questions only: *what is true of the product today* and *what is next*. About 300 words.

- Finishing a card removes its paragraph from Now in the same commit. (`alm list --state done` replaces the old Recently finished list.)
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
