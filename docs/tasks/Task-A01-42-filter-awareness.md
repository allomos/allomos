# Task-A01-42-filter-awareness

*Card type: coding. Area A01 (Tracking). Written 2026-10-14. Builds on [`A01-36`](./Task-A01-36-search.md) (search) and the state filter of decision [0016](../decisions/0016-waiting-and-blocked-states.md) and [`A03-18`](./Task-A03-18-human-check.md). Task text: `alm show A01-42`.*

## What gets built (plain words)

The **Show** filter above the task list is remembered between launches. A filter left on (for example *Backlog*) quietly hides every other task, so a task you are looking for seems missing; the approver ran into this several times, most recently with A01-41. After this card the list says what the filter hides, where you are looking (option B from the mockups), and a search finds a hidden task too (option C).

| Piece | What it is | Where |
|---|---|---|
| Hidden counts | Pure functions: how many tasks a filter hides in a group and in which states; the line's words; the filter's name | `apps/web/src/stopFlow.ts` (tested in `apps/web/tests/stopFlow.test.ts`) |
| In each group (B) | Under a group's tasks: "38 more hidden by the filter: 4 Ready, 1 Waiting, 33 Done · Show all". A group whose tasks are all hidden still shows, with only that line | `apps/web/src/App.tsx` (TaskbagView, TaskGroup) |
| In a search (C) | Tasks the search finds that the filter hides, after the groups, under "⊘ Hidden by the “Backlog” filter", with Show all; clicking one opens it in the task pane | `apps/web/src/Search.tsx` (FilterHits), `apps/web/src/App.tsx` |
| Texts and look | 5 texts in English, Chinese and Japanese; styles | `apps/web/src/i18n/*.json`, `apps/web/src/styles.css` |

## Details for the worker

- **Outcome.** A person with a state filter on sees how many tasks it hides, and a search still finds them.
- **Decisive unknowns.** None. The approver chose B + C on 2026-10-14 (not A, the strip above the list; not D, resetting the filter).
- **Choices** (routine, made by the worker):
  - A group with no tasks at all is still left out under a filter; a group whose tasks are all hidden is shown with only its "hidden" line, so no area or iteration disappears without a word.
  - While searching, the in-group line is not shown (the search section C says it instead).
  - States in the line follow the Show box's order (Backlog, Ready, In progress, Waiting, Blocked, Done, Dropped). "Show all" sets the filter to *All states*.
  - Hidden search hits are not in the arrow-key order of the list; they are reached by clicking or Tab.
- **Files owned.** The files above; this card.
- **External operations.** None.
- **Done means mechanically.** `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Options B and C from the mockups; open the card and build it | approver | 2026-10-14 | chat with session claude/a01-42-filter-awareness |

## Progress notes

- 2026-10-14 — **Built and locally verified** (session claude/a01-42-filter-awareness).

  **In plain words.** With a Show filter on, each group now ends with a line saying how many of its tasks the filter hides and in which states, with a Show all button; a group whose tasks are all hidden still appears with just that line. A search under a filter also lists the tasks it finds that the filter hides, after the groups, under "⊘ Hidden by the “Waiting” filter"; clicking one opens it in the task pane. Texts in English, Chinese and Japanese.

  **Evidence.** `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (367 texts, 69 codes), `test` PASS (81, of which 2 new in `tests/stopFlow.test.ts`). `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 2 + 44, core 40, desktop 26 + 1 ignored, store 64 + 4 + 4 + 5). `git diff --check` PASS. Debug desktop app built (`target/debug/bundle/macos/Allomos.app`) and looked at by the agent on a copy of the taskbag at `~/allomos-tests/a01-42-filter/trial-taskbag` (code folder linked as `~/allomos-tests/a01-42-filter/allomos`): with Show = Waiting, A00 showed A00-23 and "28 more hidden by the filter: 2 Backlog, 26 Done · Show all"; A01 showed only "42 more hidden by the filter: 4 Backlog, 3 Ready, 1 In progress, 33 Done, 1 Dropped · Show all"; searching `A01-41` listed A01-41 (Done) and A01-42 (In progress) under "Hidden by the “Waiting” filter". Chinese and Japanese texts: not looked at in the window. Windows: untested.

  **Human check (steps).** `open -n /Users/wanghui/coding/glidepace/allomos/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-42-filter/trial-taskbag`, then:
  1. Set Show to Backlog: each group ends with "N more hidden by the filter: …"; groups with no backlog task show only that line.
  2. Click Show all on one: the filter goes back to All states and the lines disappear.
  3. Set Show to Backlog again and search `A01-41`: it appears under "Hidden by the “Backlog” filter"; click it: the task pane opens.
  4. Switch the language to 中文 and 日本語: the lines follow.
