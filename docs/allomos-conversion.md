# Moving Allomos's tasks into the new taskbag (2026-10-22)

**What happened, in plain words:** Allomos 0.1.20 writes a newer taskbag format (`allomos/5`) and no longer reads the old one (`allomos/4`). The app set the old taskbag aside as `../allomos-taskbag/.allomos/taskbag-allomos-4` and made a new, empty taskbag in its place, `../allomos-taskbag/.allomos/taskbag`. Everything in the old one was copied into the new one with the `alm` command, keeping every ID and the order. No taskbag file was edited by hand except `project-notes.md`, which is meant to be edited directly. The approver approved the plan in the Allomos app (`alm confirm`, "Move tasks into Allomos", plan fingerprint `fbc6da29c3283c52`). The old folder was then deleted; its content is in the taskbag repository's Git history.

The previous move (2026-10-12, from a taskbag inside this repository) is described in this file's Git history (`git log -- docs/allomos-conversion.md`).

## What was copied

- **Settings:** the label ladder drafted → implemented → locally verified → deployed and verified → human verified (the new taskbag lacked "drafted"; set with `alm labels`; the top step was already called "human verified"); task language `ui` (unchanged); handoff rules: context gate 70% and when to write (`alm handoff-rules`).
- **8 areas** (A00–A07) and **15 iterations** (I01–I15): same IDs, names, order and releases.
- **194 tasks**, same IDs, areas, iterations and descriptions. State, label, holder, next dependency, handoff note, waiting-on, the reason for finishing without the human check and the human check steps were compared field by field with the old `task.json` after copying: **0 differences** (titles compared against their translations).
- **Titles:** the task language is zh-CN, so the 146 titles written in English were translated into Chinese; titles already in Chinese are unchanged. Descriptions and cards are not translated.
- **115 card files**, each moved beside its own task with `alm card` (table below); every copy is byte for byte the same as the old `card.md`. Their relative links work as before, because a card sits at the same place inside the taskbag. 14 tasks had a `card.md` but no `Card:` line at the top of their description (in some the link appeared only further down): `alm card` added the line `Card: [card.md](card.md)` at the top (A01-25, A01-26, A01-27, A01-28, A01-29, A01-47, A03-02, A03-09, A03-11, A03-12, A03-20, A07-07, A07-16, A07-18); the rest of those descriptions is unchanged.
- **No card files in this project folder** (`docs/tasks/` was removed in A00-30, and no file is named after a task ID; `crates/store/tests/fixtures/` holds test samples, not cards). So nothing was removed from this repository, and there are no card files without a task.
- **12 handoffs**, each with its writer and tasks, the text unchanged (`alm handoff --from`; `alm` adds one blank line after the header).
- **Project notes:** the old `project-notes.md` in full, plus a **Moved in** section; the pitfall about the old format was rewritten in place, because it is no longer true.
- **AGENTS.md:** its Allomos section is already exactly what `alm brief` prints for the new taskbag, so it did not change. `CLAUDE.md` already imports `@AGENTS.md`.
- **Checks:** `alm list --json` reports `issues: []` and `cards_outside: []`; `alm cards` lists none.

## Values that could not be copied

| Value | Where it was | What happened |
|---|---|---|
| Change history: 1692 event files | `taskbag-allomos-4/events/` | Not copied: `alm` records only its own changes. They stay in the taskbag repository's Git history; the last commit with the whole old taskbag is `868c0696710491eafd67ecdaf09784a2e0fc333a` (path `.allomos/taskbag/`). The new tasks' history starts with today's copy. |
| Handoff written / updated times: 2026-10-15T13:16Z / 2026-10-15T14:56Z | `taskbag-allomos-4/handoffs/2026-10-15-1316-claude-loop-i02.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-15T14:58Z / 2026-10-16T09:57Z | `taskbag-allomos-4/handoffs/2026-10-15-1458-claude-loop-ready.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-16T01:14Z / 2026-10-16T01:14Z | `taskbag-allomos-4/handoffs/2026-10-16-0114-claude-a00-30-detail-in-taskbag.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-16T09:27Z / 2026-10-16T09:27Z | `taskbag-allomos-4/handoffs/2026-10-16-0927-claude-a00-30-cards-in.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-16T10:17Z / 2026-10-17T02:10Z | `taskbag-allomos-4/handoffs/2026-10-16-1017-claude-a01-51-auto-notice.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-18T08:36Z / 2026-10-20T03:45Z | `taskbag-allomos-4/handoffs/2026-10-18-0836-claude-website.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-19T10:25Z / 2026-10-19T10:25Z | `taskbag-allomos-4/handoffs/2026-10-19-1025-claude-i02-loop.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-20T10:31Z / 2026-10-20T12:00Z | `taskbag-allomos-4/handoffs/2026-10-20-1031-claude-i02-loop-2.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-20T16:57Z / 2026-10-20T16:57Z | `taskbag-allomos-4/handoffs/2026-10-20-1657-claude-a05-11-welcome-fit.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-21T07:18Z / 2026-10-22T10:45Z | `taskbag-allomos-4/handoffs/2026-10-21-0718-claude-i02-loop-2026-10-21.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-21T14:22Z / 2026-10-21T14:22Z | `taskbag-allomos-4/handoffs/2026-10-21-1422-claude-a05-gusto837-welcome-balance.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Handoff written / updated times: 2026-10-22T12:16Z / 2026-10-22T12:16Z | `taskbag-allomos-4/handoffs/2026-10-22-1216-claude-a07-apex359-faster.md`, lines 4–5 | `alm handoff` set both to the time of the copy (2026-10-22T13:42Z) and named the file `2026-10-22-1342-…`. |
| Base branch of each task | (none: `allomos/4` did not record one) | Every task's base is `main`, this project's base branch (decided in the plan). |
| Working branch of held tasks | (none recorded) | Left unknown: holders were copied with `alm claim` without a working branch. |
| Quota gate for handoffs | `taskbag-allomos-4/project.json`, lines 14–17 (`handoff` has only `context_gate` and `write_when`) | Not set in the old file; the new taskbag shows `alm`'s default (3% of the 5-hour quota). |
| Iteration dates | `taskbag-allomos-4/iterations/I01.json` … `I15.json` | None were recorded, so none were written. |
| A03-tide388's latest changes | taskbag repository commit `905f155` ("alm: A03-tide388 evidence, AI decision, label, done to check", by `claude/a03-tide388-size-reminder`) | That session's `alm` changes reached neither taskbag (its commit only deleted old files). A03-tide388 was copied as the old folder had it: in progress, held by `claude/a03-tide388-size-reminder`, no label. That session must redo its evidence, AI decision, label and done in the new taskbag. |
| Empty `.keep` files | `taskbag-allomos-4/tasks/.keep`, `handoffs/.keep` | Not needed in the new taskbag. |

## Card files and their tasks

Each old `taskbag-allomos-4/tasks/<id>/card.md` became `tasks/<id>/card.md` in the new taskbag, for the same task. No card was left out or copied twice.

| Old card file | Task |
|---|---|
| `tasks/A00-01/card.md` | `A00-01` |
| `tasks/A00-02/card.md` | `A00-02` |
| `tasks/A00-03/card.md` | `A00-03` |
| `tasks/A00-04/card.md` | `A00-04` |
| `tasks/A00-05/card.md` | `A00-05` |
| `tasks/A00-06/card.md` | `A00-06` |
| `tasks/A00-07/card.md` | `A00-07` |
| `tasks/A00-08/card.md` | `A00-08` |
| `tasks/A00-09/card.md` | `A00-09` |
| `tasks/A00-10/card.md` | `A00-10` |
| `tasks/A00-11/card.md` | `A00-11` |
| `tasks/A00-12/card.md` | `A00-12` |
| `tasks/A00-13/card.md` | `A00-13` |
| `tasks/A00-14/card.md` | `A00-14` |
| `tasks/A00-15/card.md` | `A00-15` |
| `tasks/A00-16/card.md` | `A00-16` |
| `tasks/A00-17/card.md` | `A00-17` |
| `tasks/A00-18/card.md` | `A00-18` |
| `tasks/A00-19/card.md` | `A00-19` |
| `tasks/A00-20/card.md` | `A00-20` |
| `tasks/A00-21/card.md` | `A00-21` |
| `tasks/A00-22/card.md` | `A00-22` |
| `tasks/A00-23/card.md` | `A00-23` |
| `tasks/A00-24/card.md` | `A00-24` |
| `tasks/A00-25/card.md` | `A00-25` |
| `tasks/A00-28/card.md` | `A00-28` |
| `tasks/A00-29/card.md` | `A00-29` |
| `tasks/A01-01/card.md` | `A01-01` |
| `tasks/A01-02/card.md` | `A01-02` |
| `tasks/A01-03/card.md` | `A01-03` |
| `tasks/A01-04/card.md` | `A01-04` |
| `tasks/A01-05/card.md` | `A01-05` |
| `tasks/A01-06/card.md` | `A01-06` |
| `tasks/A01-07/card.md` | `A01-07` |
| `tasks/A01-08/card.md` | `A01-08` |
| `tasks/A01-09/card.md` | `A01-09` |
| `tasks/A01-10/card.md` | `A01-10` |
| `tasks/A01-11/card.md` | `A01-11` |
| `tasks/A01-12/card.md` | `A01-12` |
| `tasks/A01-13/card.md` | `A01-13` |
| `tasks/A01-14/card.md` | `A01-14` |
| `tasks/A01-15/card.md` | `A01-15` |
| `tasks/A01-16/card.md` | `A01-16` |
| `tasks/A01-17/card.md` | `A01-17` |
| `tasks/A01-18/card.md` | `A01-18` |
| `tasks/A01-19/card.md` | `A01-19` |
| `tasks/A01-20/card.md` | `A01-20` |
| `tasks/A01-21/card.md` | `A01-21` |
| `tasks/A01-24/card.md` | `A01-24` |
| `tasks/A01-25/card.md` | `A01-25` |
| `tasks/A01-26/card.md` | `A01-26` |
| `tasks/A01-27/card.md` | `A01-27` |
| `tasks/A01-28/card.md` | `A01-28` |
| `tasks/A01-29/card.md` | `A01-29` |
| `tasks/A01-31/card.md` | `A01-31` |
| `tasks/A01-32/card.md` | `A01-32` |
| `tasks/A01-34/card.md` | `A01-34` |
| `tasks/A01-36/card.md` | `A01-36` |
| `tasks/A01-37/card.md` | `A01-37` |
| `tasks/A01-38/card.md` | `A01-38` |
| `tasks/A01-39/card.md` | `A01-39` |
| `tasks/A01-41/card.md` | `A01-41` |
| `tasks/A01-42/card.md` | `A01-42` |
| `tasks/A01-43/card.md` | `A01-43` |
| `tasks/A01-44/card.md` | `A01-44` |
| `tasks/A01-45/card.md` | `A01-45` |
| `tasks/A01-46/card.md` | `A01-46` |
| `tasks/A01-47/card.md` | `A01-47` |
| `tasks/A01-48/card.md` | `A01-48` |
| `tasks/A02-01/card.md` | `A02-01` |
| `tasks/A03-01/card.md` | `A03-01` |
| `tasks/A03-02/card.md` | `A03-02` |
| `tasks/A03-03/card.md` | `A03-03` |
| `tasks/A03-04/card.md` | `A03-04` |
| `tasks/A03-05/card.md` | `A03-05` |
| `tasks/A03-06/card.md` | `A03-06` |
| `tasks/A03-07/card.md` | `A03-07` |
| `tasks/A03-08/card.md` | `A03-08` |
| `tasks/A03-09/card.md` | `A03-09` |
| `tasks/A03-10/card.md` | `A03-10` |
| `tasks/A03-11/card.md` | `A03-11` |
| `tasks/A03-12/card.md` | `A03-12` |
| `tasks/A03-13/card.md` | `A03-13` |
| `tasks/A03-14/card.md` | `A03-14` |
| `tasks/A03-15/card.md` | `A03-15` |
| `tasks/A03-16/card.md` | `A03-16` |
| `tasks/A03-18/card.md` | `A03-18` |
| `tasks/A03-19/card.md` | `A03-19` |
| `tasks/A03-20/card.md` | `A03-20` |
| `tasks/A03-21/card.md` | `A03-21` |
| `tasks/A03-22/card.md` | `A03-22` |
| `tasks/A04-01/card.md` | `A04-01` |
| `tasks/A05-01/card.md` | `A05-01` |
| `tasks/A05-02/card.md` | `A05-02` |
| `tasks/A05-03/card.md` | `A05-03` |
| `tasks/A05-05/card.md` | `A05-05` |
| `tasks/A05-06/card.md` | `A05-06` |
| `tasks/A07-01/card.md` | `A07-01` |
| `tasks/A07-02/card.md` | `A07-02` |
| `tasks/A07-03/card.md` | `A07-03` |
| `tasks/A07-04/card.md` | `A07-04` |
| `tasks/A07-05/card.md` | `A07-05` |
| `tasks/A07-06/card.md` | `A07-06` |
| `tasks/A07-07/card.md` | `A07-07` |
| `tasks/A07-08/card.md` | `A07-08` |
| `tasks/A07-09/card.md` | `A07-09` |
| `tasks/A07-10/card.md` | `A07-10` |
| `tasks/A07-11/card.md` | `A07-11` |
| `tasks/A07-12/card.md` | `A07-12` |
| `tasks/A07-13/card.md` | `A07-13` |
| `tasks/A07-14/card.md` | `A07-14` |
| `tasks/A07-15/card.md` | `A07-15` |
| `tasks/A07-16/card.md` | `A07-16` |
| `tasks/A07-17/card.md` | `A07-17` |
| `tasks/A07-18/card.md` | `A07-18` |

115 card files, 115 tasks with a card.
