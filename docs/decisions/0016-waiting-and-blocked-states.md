# 0016 — Waiting on a person and blocked by another task are two different states

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Proposed** | 2026-10-14 | — | will.w.hui |

Card: [`Task-A01-31`](../tasks/Task-A01-31-blocked-state.md). No code is written until the owner accepts this record.

## In plain words

A task that cannot go on is today always **waiting**. That one word covers two very different situations:

- **a person must act** — the owner must approve a card, someone must answer a question, an attorney must reply;
- **another task must finish first** — A01-31 cannot start until A01-17 is done.

The app shows only "Waiting", and the owner read it as "waiting for the AI to go on". Nobody can tell from the list whether *they* are the one holding things up.

**Proposal:** split it into two states.

| State | Set with | Must say | App shows |
|---|---|---|---|
| `waiting` (kept, narrowed) | `alm wait <id> --on "<what>" [--who "<who>"]` | what a person must do; optionally who | "Waiting for you: approve the card" / "Waiting for Ann: reply on licence" |
| `blocked` (new) | `alm block <id> --by <task-id> [<task-id>…]` | one or more blocking task IDs in this taskbag | "Blocked by A01-17 (in progress)" — each blocker with its **current** state |

Anything outside the taskbag — an attorney's reply, a vendor's release — stays `waiting`: no task can finish to free it, so someone must notice and resume.

## Context

- Allomos has six fixed states: `backlog`, `ready`, `in_progress`, `waiting`, `done`, `dropped` ([FILE-FORMAT § Tasks](../product/FILE-FORMAT.md), `alm guide terms`, `alm guide customize`: "The six states"). States are core terms written into every project's AGENTS.md by `alm brief`, so a new one is a change of the contract, not a detail.
- `waiting` needs a free-text `waiting_on` (`alm wait <id> --on "<what>"`, problem `waiting_on_missing`). Whether it waits on a person or on a task is only in that text.
- "Blocked by a task" lives today in free text (`waiting_on` or `next_dependency`); the loop that picks the next task has to read the words to skip such tasks, so a task that only waits for another task looks the same as one waiting for the owner.
- Task A01-30 (dropped, no card file) proposed only new wording for `waiting`; A01-31 and this record replace it.

## Proposal in detail

1. **`waiting` is narrowed** to "a person or an outside event must act". `--on "<what>"` stays required. A new optional `--who "<name>"` says who; with no `who`, the app says "Waiting for you" when the reader is the owner, otherwise "Waiting on: <what>".
2. **`blocked` is new.** `alm block <id> --by A01-17 [A01-20 …]` sets the state and a new task field `blocked_by` (a list of task IDs). Each ID must be a task in the same taskbag, not the task itself. `alm unblock <id>` (or `alm ready <id>`) moves it on.
3. **`alm` tells, the agent checks.** `alm list`, `show` and `next` print each blocker with its current state ("A01-17 (done)"). When every blocker is done, `alm next` says so ("A01-31: all blockers are done — unblock it?"); it does **not** move the task by itself (see Open point 3).
4. **The app** shows the two states with different words and colours, in the task pane, the list, and the state filters, in every UI language (en, zh-CN, ja).
5. **Format** goes from `allomos/3` to `allomos/4` through the guided migration of [A01-32](../tasks/Task-A01-32-guided-migration.md), not a code step: version 4's migration entry says what changed (the `blocked` state, the optional `who` and `blocked_by` fields), and `alm migrate --finish` sets the new version only when the taskbag has no issues; older Allomos builds see `taskbag_newer` and ask to be updated. **Existing `waiting` tasks stay `waiting`**: the entry does not guess which ones are really blocked by a task; an agent or the owner may move them with `alm block` afterwards. A03-18's human-check changes land in the same version 4.

## Alternatives considered

| Option | What it is | Why not (proposed) |
|---|---|---|
| **A. Two states** (proposed) | `waiting` + `blocked`, as above | — A state is what filters, colours and the loop already use; a second state is the plainest signal. |
| B. One state, with a kind | Keep `waiting`; add `waiting_kind: person \| task` and an optional task list | No new state, so less churn; but every reader (filters, loop, counts, burndown's "waiting on the owner") must look at two fields, and "Waiting" still reads the same at a glance. |
| C. Other names | `needs-person` (or `needs-you`) + `blocked` | Clearer about *who* than "waiting", but renames an existing state: every taskbag must be rewritten on upgrade, and every AGENTS.md and agent habit (`alm wait`) changes. |
| D. Wording only | Keep one state; app shows "Waiting on: <text>" | What A01-30 tried; does not let the app or loop tell the two apart. |

## Consequences

- **Format:** `allomos/3` → `allomos/4`, through a migration entry in A01-32's guided migration (no code step; `alm upgrade` is replaced by `alm migrate`); FILE-FORMAT gains the `blocked` state, `blocked_by` and `who`, and validation rules (a `blocked` task has at least one `blocked_by`; each is a known task ID; not itself).
- **Existing data:** nothing moves on upgrade; `waiting` tasks stay `waiting`.
- **Contract text:** "the six states" becomes seven in `alm guide terms` / `customize`, the `alm brief` section copied into every project's AGENTS.md (this repository's [Tasks: Allomos](../../AGENTS.md#tasks-allomos-v2) included), [CARDS § States and labels](../workflow/CARDS.md#states-and-labels) and [CODING_RULES](../workflow/CODING_RULES.md). Projects with an older AGENTS.md section learn of it through `alm`'s renewed instructions ([A03-07](../tasks/Task-A03-07-renew-agent-instructions.md)).
- **Loop and `alm next`:** a `blocked` task is skipped by state, not by reading text.
- **Progress views:** "what waits on the owner" counts only `waiting` tasks; `blocked` is shown apart.
- **App:** new state name, colour and filter; the pane's edit form gains "Blocked by" with a task picker; texts in all three languages.

## Open for the owner

1. **State names.** `waiting` + `blocked` (proposed), or rename `waiting` (e.g. `needs-person` / `needs-you`) at the cost of rewriting existing taskbags?
2. **Can a blocked task also carry a waiting note?** E.g. blocked by A01-17 *and* waiting for the owner's choice. Proposed: no — one state at a time; the note goes in `next_dependency`.
3. **When every blocker is done**, does the task move by itself (to `ready`, or back to `in_progress` if held), or does `alm` only tell and the agent move it? Proposed: only tell ("alm tells, the agent checks"); an automatic move is a check inside `alm` and needs your approval.
4. **Dropped blockers.** May `blocked_by` point at a dropped task, and does a dropped blocker count as "finished"? Proposed: allowed but flagged ("A01-30 (dropped) — still blocked?"), never counted as done.
5. **`--who`.** Add the optional "who" to `waiting` now, or leave `waiting` exactly as it is and change only the app's wording?

## Owner's answers (2026-10-14)

Given in chat and relayed to this session by the owner's other session ("Task workflow open questions"). The record as a whole is **not yet accepted**: the owner still says "accept the record" before it is marked Accepted and code starts.

1. **Names:** keep `waiting`, add `blocked`.
2. **Blocked and waiting on a person:** one state only, `blocked`; the person part goes in the note (`next_dependency`).
3. **When every blocker is done:** `alm` only tells; it never moves the task by itself.
4. **Dropped blocker:** allowed but flagged, never counted as done.
5. **`--who`:** add the optional `--who` to `alm wait`.
6. **Format:** version 4 goes through A01-32's guided migration, not a code step; A01-31 is worked together with A01-32, and A03-18 also lands in version 4.
