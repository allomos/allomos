# Cards, labels and claiming

A **contract** for how work is recorded on FlipHopper. Since [`Task-A01-07`](../tasks/Task-A01-07-own-cards-in-seabag.md) (2026-10-06) FlipHopper tracks itself: **status lives in this repository's seabag** (`.fliphopper/seabag/` on `main`), changed with the `fh` command ([CODING_RULES § The `fh` command](./CODING_RULES.md#the-fh-command)). Each card's detail is its own file in [docs/tasks/](../tasks/). [TASK_BOARD.md](../TASK_BOARD.md) keeps only **Now** and **Waiting on the owner**, until it is removed or generated from the seabag (owner, 2026-10-06).

## Areas

An **area** is *what part of the product* a card belongs to. Areas are named after what users can do, not after technical layers (owner, 2026-10-04). An **iteration** is *when* a card is planned ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). Every card has exactly one area.

| ID | Area | What users can do | Iterations |
|---|---|---|---|
| A00 | Planning | Read the plan, requirements, decisions and research | all |
| A01 | Tracking | Keep tasks in a seabag, view and edit them, use the `fliphopper` / `fh` command, commit and push from the app | 1, 2, 6 |
| A02 | Progress & forecast | See progress, what waits on them, burndown, finish-date forecast, timeline; export a report | 4, 5, 12 |
| A03 | Agents | Let Claude Code and Codex work through MCP; start and open agent sessions from the app | 3 (MCP), 8 (start/open agents) |
| A04 | Companions | View progress and approve from a browser or phone, at home or away | 9, 10 |
| A05 | Project setup & import | Import the SSR board; set up a project with an AI agent | 7, 11 |
| A06 | Cloud & team | Keep a project in the cloud; invite teammates with roles | 13, 14 |
| A07 | Platform & release | Use FlipHopper on Windows as well as macOS; install signed builds that update themselves | 2, 15 |

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

**Only in the seabag on `main`**: each card is a task with the card's number as its ID (`A01-07`), and its state, label, holder and waiting-on are that task's fields. The card file holds the outcome, scope, approvals and dated progress notes — never a copy of the state. The task's `description.md` links to the card file.

- **Change status only with `fh`, only on `main`.** Card branches hold an old copy of the seabag; never change it there. From a card's worktree, point `fh` at the main checkout: `export FLIPHOPPER_SEABAG=<main checkout>` (for example `/Users/wanghui/coding/glidepace/fliphopper`). Then commit only the seabag change on `main` (`git -C <main checkout> commit .fliphopper -m "fh: label A01-05 locally verified"`) and push it.
- **Who you are:** `export FLIPHOPPER_HOLDER=<your branch>` (for example `claude/a01-05-readme`), or `--as <branch>` on each command.
- **Next dependency:** `fh needs <ID> "<the one thing needed for the next label>"`. For a waiting task, its *waiting on* says it.
- **Seeing status:** `fh list` (all, by area), `fh list --state in_progress` (who holds what — this replaces the old Sessions table), `fh list --state done`, `fh show <ID>`, `fh next`. The app shows the same when it opens this repository.

## States and labels

- **State:** `backlog` → `ready` (owner reviewed the card) → `in progress` → `waiting` (on the owner or another card) → `done`; or `dropped`.
- **Label**, the only progress claim, for work that ships:
  0. **drafted** — a document or decision card is written for review
  1. **implemented** — code written, may not run end to end
  2. **locally verified** — the card's named checks pass locally, in the complete local app
  3. **deployed and verified** — running on the approved hosted target, hosted checks pass (only once a hosting target exists)
  4. **owner verified** — the owner checked it; the card may move to Done
- Documentation or decision cards use **drafted** → **owner verified** (the owner accepted it). This is one ladder for every card, set in the seabag's `project.json` (owner, 2026-10-06).
- Every open card names its **single next dependency** (what blocks the next label) and who it waits on.
- Passing test counts support a label; they are never a label.

## Approvals

- The owner approving a card (`ready`) authorizes all its **local** work: source, tests, docs, commits.
- Every **external operation** needs its own exact approval, recorded in the card's Approvals section: target, operation, cleanup. See [AGENTS.md § Always-on boundaries](../../AGENTS.md#always-on-boundaries).

## Claiming protocol

One card is worked by exactly one session at a time. The seabag on `main`, not memory or chat, records who holds what. Every step below is run against the main checkout's seabag (see [Where status lives](#where-status-lives)).

1. **Pull before claiming:** `git -C <main checkout> pull --ff-only`, then `fh next` (or `fh list --state ready`).
2. **Claim in a commit of its own.** `fh claim <ID> --as <branch>`; commit only the seabag change on `main` (`fh: claim <ID>`), push it at once, then start work. `fh claim` refuses a task someone else holds (`task_held`), and the lock stops two sessions on this computer from both winning.
3. **A card held by another session is read-only to you.** If its card file holds a stale fact, correct only that fact. `fh claim --take` is only for a session whose holder has stopped (for example its quota ran out) and says so in the handoff.
4. **Two claims from different computers:** the second to push gets a merge conflict in that task's `task.json` — keep whichever landed first and claim a different card.
5. **Labels and waiting:** `fh label <ID> "<label>"` and `fh wait <ID> --on "<what>"` on `main`, each committed and pushed as it happens.
6. **Finish** after the owner's check: `fh label <ID> "owner verified"`, `fh done <ID>`, `fh release <ID>`; commit with the merge of the card's branch.
7. **Release** a card you stop short of Done: `fh release <ID> --note "<what the next session needs>"`, remove your handoff block, commit and push.
8. **Clean up after a merge.** Once your card's branch is merged into `main` and pushed, and the owner no longer needs the app built in it for a check: stop anything still running from the worktree, then remove **your own** worktree (`git worktree remove .claude/worktrees/<name>`, which also deletes its `target/`) and delete the merged local branch (`git branch -d <branch>`). Never remove another session's worktree. The branch on GitHub stays unless the owner asks. Rules for build output: [CODING_RULES.md § Build output](./CODING_RULES.md#build-output-target).
9. **A new card:** write its file in `docs/tasks/`, then `fh add task <ID> --area <area> --title "<title>" --description "Card: [docs/tasks/<file>](../../../../docs/tasks/<file>)"` on `main`; it starts in `backlog`. When the owner approves it: `fh ready <ID>`. A card that will not be done: `fh drop <ID>`.
10. **Now** is shared text: rewrite it only when a card is claimed or finished, never for mid-flight progress.

## Now — rules

**Now** answers two questions only: *what is true of the product today* and *what is next*. About 300 words.

- Finishing a card removes its paragraph from Now in the same commit. (`fh list --state done` replaces the old Recently finished list.)
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
