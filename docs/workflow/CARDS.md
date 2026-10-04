# Cards, labels and claiming

A **contract** for how work is recorded on glidebox while the board is Markdown. [TASK_BOARD.md](../TASK_BOARD.md) is the index; each card's detail is its own file in [docs/tasks/](../tasks/).

## Workstreams

| ID | Workstream | Milestone |
|---|---|---|
| W00 | Planning, docs, decisions and research | M0 |
| W01 | Core model, burndown and forecast (`packages/core`) | M1 |
| W02 | Importer (Markdown boards → glidebox) | M1 |
| W03 | Storage and backend: local store, cloud schema, auth, RLS, realtime | M2, M4 |
| W04 | Desktop hub and UI: lists, charts, burndown; local server for companions | M2 |
| W05 | Agent write path: CLI and MCP | M3 |
| W06 | Collaboration: roles, invitations, inbox, approvals | M4 |
| W07 | Agent runner: start and open agent sessions | M5 |
| W08 | Polish: timeline, export, size-weighted burndown | M6 |
| W09 | AI-generated project setup: prompt, schema, validation, adoption | proposed with M2 |
| W10 | Companions and other interfaces: web and mobile companions, pairing, human CLI | M2b |

Add a workstream by editing this table and the board's card index in the same commit.

## Card IDs and files

- **ID:** `Task-W{2-digit workstream}-{2-digit task}-{short-description}`, e.g. `Task-W01-01-core-model`. A child extends its parent with another two-digit number: `Task-W01-01-02-forecast-monte-carlo`.
- **One card = one observable outcome** — a sentence about what a person (or agent) can do afterwards, not a layer or component. Split by user-visible step, never by layer. If more than about four documents must be read before the first line of code, split.
- **File:** `docs/tasks/Task-W…-….md`, created from [the template](#card-template). The file never moves; only its board row changes.

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
6. **Now** is shared text: rewrite it only when a card is claimed or finished, never for mid-flight progress.

## Now — rules

**Now** answers two questions only: *what is true of the product today* and *what is next*. About 300 words.

- Finishing a card removes its paragraph from Now in the same commit; one row goes to **Recently finished**.
- Now never repeats what a card says; every line names a card that exists.
- Headings name states (Now, Waiting on the owner, Blocked, Done), never topics.
- Before committing a board edit, read Now from the top; if two lines disagree, correct the older one in place.

## Card template

```markdown
# Task-W00-00-short-description

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
