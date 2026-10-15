# Task-A01-48-filter-and-search

*Card type: coding. Area A01 (Tracking), Iteration I02. Written 2026-10-16. Builds on [`A01-36`](./Task-A01-36-search.md) (search), [`A01-42`](./Task-A01-42-filter-awareness.md) (what the filter hides), [`A01-44`](./Task-A01-44-filter-banner.md) (the banners under the tabs) and keeps [`A01-47`](./Task-A01-47-refresh-line.md) (the refresh line). Task text: `alm show A01-48`.*

## What gets built (plain words)

Until now, with a **Show** filter on (for example *Waiting*), a search listed the tasks passing both, and then, after the groups, every other task it found that the filter hides, under "⊘ Hidden by the “Waiting” filter" (A01-42's option C). After this card the filter and the search simply narrow the list together: only tasks that pass the filter **and** hold the search words are listed, and areas or iterations with none of them are left out. The banners say so, with counts that match what is shown:

- "**2 tasks** found for “alm” with the “Waiting” filter on." (neutral, on top)
- "Filter and search together: only tasks that are waiting and hold “alm” are listed, in 1 area. 3 more matches are hidden by the filter." (blue, with **Show all**)
- When every match is hidden: only the blue banner, "None of the tasks found for “alm” is waiting. All 3 matches are hidden by the filter."

Clearing the search gives back the filter alone (A01-44's banner); turning the filter off gives back the search alone. A01-47's refresh line stays under the banners on every tab.

| Piece | What it is | Where |
|---|---|---|
| Banner words | Pure functions: `bothBanner` (the filter banner while searching), `searchBanner` with the filter's name, `taskListBanners` (both banners of By area / By iteration from the groups, the filtered groups, the found groups and the number of matches); `filterBanner` loses its old while-searching branch | `apps/web/src/stopFlow.ts` (tested in `apps/web/tests/stopFlow.test.ts`) |
| Task list | The search narrows the filtered groups only; the "Hidden by the filter" section (`FilterHits`) is removed; "Nothing found" only when the search finds no task and no handoff at all | `apps/web/src/App.tsx`, `apps/web/src/Search.tsx` |
| Handoffs tab | Already narrowed by both; its search banner now counts the handoffs shown and names the filter | `apps/web/src/Handoffs.tsx` |
| Texts and look | 7 new texts and 6 removed, in English, Chinese and Japanese; the `.search-hidden` style removed | `apps/web/src/i18n/*.json`, `apps/web/src/styles.css` |

## Details for the worker

- **Outcome.** With a Show filter on and a search typed, the list shows only tasks passing both, and the banners say it is the combined result.
- **Decisive unknowns.** None. The approver asked for this on 2026-10-15; whether to still mention the matches the filter hides was left to the worker.
- **Choices** (routine, made by the worker in a quiet loop run, each recorded with `alm ai-decided A01-48`):
  - Decided: the blue banner still **counts** the matches the filter hides ("3 more matches are hidden by the filter.", with Show all) but no longer lists them, so a filter left on cannot make a task look missing (A01-42's reason) without breaking the both-together list.
  - Decided: with both on, the search banner counts only the tasks listed and names the filter; when every match is hidden the search banner is left out, only the blue banner shows, and "Nothing found" is not shown, since it would be false.
  - Decided: the Handoffs tab already listed only handoffs passing both; only its search banner changed (the handoffs shown, the filter named). Its blue banner stays A01-44's.
  - While searching, the in-group "N more hidden by the filter" line stays off, as before.
- **Not changed.** A01-47's refresh line and Refresh now: same place (under the banners, on every tab), same behaviour; the combined list is worked out again from the tasks after every reload, and the selected task stays selected.
- **Files owned.** The files above; this card; the lines of the A01-42 and A01-44 cards that described the old behaviour (corrected in place).
- **External operations.** None.
- **Done means mechanically.** `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Filter and search narrow the list together; keep A01-47's refresh line; the hidden-matches mention left to the worker | approver | 2026-10-15 | chat; task notes of A01-48 (`alm show A01-48`) |

## Progress notes

- 2026-10-16 — **Built and locally verified** (session claude/a01-48-filter-and-search, quiet loop run).

  **In plain words.** With a Show filter on, a search now lists only the tasks that pass the filter and hold the words; groups with none are left out. The grey search banner counts those tasks and names the filter, and the blue banner says the list is the filter and the search together and how many more matches the filter hides, with Show all. The old "Hidden by the filter" list after the groups is gone. Clearing either one falls back to the other. The refresh line is unchanged. Texts in English, Chinese and Japanese.

  **Evidence.** `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (451 texts, 68 codes), `test` PASS (105: the A01-44 while-searching test replaced by 2 new ones in `tests/stopFlow.test.ts`, both together with hidden counts and none left, and the fallbacks of `taskListBanners`; the search banner test gains the filtered form). `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 46, core 42, desktop 27 + 1 ignored, store 62 + 5 + 4 + 5). `git diff --check` PASS. Chinese and Japanese texts checked by rendering them with the pure functions (for example "筛选与搜索同时生效：只列出在等待中且含有「alm」的任务，分布在 1 个领域。另有 3 个匹配被筛选隐藏。"). Debug desktop app built (`target/debug/bundle/macos/Allomos.app` in this worktree). Trial taskbag copied to `~/allomos-tests/a01-48-filter-and-search/trial/.allomos/taskbag` (no `.git`). **The window itself was not looked at by the agent**: NOT RUN, left to the human check. Windows: untested.

  **Human check (steps).** `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-48-filter-and-search/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-48-filter-and-search/trial`, then:
  1. By area, set Show to *Waiting* and search `search`: only waiting tasks holding the word are listed (A00-23 in the trial copy), other areas are left out; the grey banner reads "**1 task** found for “search” with the “Waiting” filter on.", the blue one "Filter and search together: only tasks that are waiting and hold “search” are listed, in 1 area. N more matches are hidden by the filter."; no "Hidden by the filter" list after the groups.
  2. Set Show to *Blocked*: only the blue banner, "None of the tasks found for “search” is blocked. All N matches are hidden by the filter.", and no "Nothing found".
  3. Clear the search (Esc): the filter banner alone, as A01-44. Search again and set Show to *All states*: the search banner alone, counting every match.
  4. Click **Show all** in the blue banner while searching: the filter goes off and every match is listed.
  5. With both on, the refresh line sits under the two banners; click a listed task, then **Refresh now**: the line updates and the task stays selected. Same on By iteration and Handoffs (there the grey banner counts handoffs and names the filter).
  6. Switch the language to 中文 and 日本語: the banners follow.
