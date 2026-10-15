# Task-A01-44-filter-banner

*Card type: coding. Area A01 (Tracking). Written 2026-10-15. Builds on [`A01-36`](./Task-A01-36-search.md) (search) and [`A01-42`](./Task-A01-42-filter-awareness.md) (what the filter hides). Task text: `alm show A01-44`.*

## What gets built (plain words)

With a **Show** filter on (for example *Needs a human*), the list used to keep every area and iteration that had any task, so a group whose tasks were all hidden still showed, with only its "hidden by the filter" line. After this card, a group with nothing left after the filter is not drawn at all, on By area, By iteration and Handoffs alike, and a **banner under the tabs** says what the filter shows: "**3 tasks** need a human (to check, to answer, to review), in 2 of 7 areas. The 5 areas with none are left out." It is tinted blue, apart from the plain search banner, and ends with **Show all**, which turns the filter off. When nothing matches, only the banner shows ("No task is blocked. All 12 iterations are left out."). With a search on, the search banner ("**2 tasks** found for “alm”.") stays on top and the filter banner sits under it ("**1 of them** needs a human, in 1 area. 1 more is listed below, hidden by the filter.").

| Piece | What it is | Where |
|---|---|---|
| Which groups are drawn, banner words | Pure functions: `filterGroups` (groups with no match left out, the whole list kept for the count), `handoffGroups` (a handoff as the group of the tasks it names), `taskCount`, `filterBanner`, `searchBanner` | `apps/web/src/stopFlow.ts` (tested in `apps/web/tests/stopFlow.test.ts`) |
| Banners | `ListBanners`: the search banner (neutral) on top, the filter banner (blue tint, Show all) under it | `apps/web/src/Search.tsx` |
| Task lists | By area and By iteration use `filterGroups`; the banners at the top of the list; when nothing matches, the New area / New iteration link is hidden too | `apps/web/src/App.tsx` |
| Handoffs tab | The Show box shows there too; a handoff naming no task the filter keeps is not drawn; the same banners | `apps/web/src/Handoffs.tsx`, `apps/web/src/App.tsx` |
| Texts and look | Banner texts in English, Chinese and Japanese; a blue tint (`--info`, `--info-bg`, light and dark) | `apps/web/src/i18n/*.json`, `apps/web/src/styles.css` |

## Details for the worker

- **Outcome.** A person who picks a Show filter sees only the groups holding a match, and a banner under the tabs says which filter is on, how many tasks it shows and how many groups it left out, with Show all.
- **Decisive unknowns.** None. The approver chose the mockups as drawn on 2026-10-15 (banner under the tabs, accent tint, search banner above it; the Handoffs tab follows the same rule).
- **Choices** (routine, made by the worker, each recorded with `alm ai-decided A01-44`):
  - The app had no search banner, so a plain one was added on top while searching ("2 tasks found for “alm”."; on the Handoffs tab "1 handoff found …"), so the filter banner's "1 of them" refers to something. It shows only when the search finds something; "Nothing found" stays as it was.
  - The Handoffs tab follows the Show filter, and the Show box now shows there too: a handoff is drawn when it names at least one task the filter keeps; one naming no task is left out under a filter; the banner counts each named task once.
  - The app's accent colour is amber, so the filter banner uses a new blue tint, as the approver's "accent (blue)" asked; the search banner stays neutral.
  - While searching, the filter banner does not count groups left out (the search already leaves groups out). *Changed by [`A01-48`](./Task-A01-48-filter-and-search.md):* it now says the list is the filter and the search together and how many matches the filter hides; nothing is listed below any more.
  - When nothing matches, the New area / New iteration link and the "no iterations" hint are hidden too, so only the banner shows. Unplanned and unknown-area groups count among the groups. The banner sits at the top of the list and scrolls with it.
  - Groups that stay keep A01-42's "N more hidden by the filter: … · Show all" line.
- **Files owned.** The files above; this card.
- **External operations.** None.
- **Done means mechanically.** `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| The mockups as drawn (banner under the tabs, accent tint, search banner above it; Handoffs follows the same rule); build it | approver | 2026-10-15 | task notes of A01-44 (`alm show A01-44`) |

## Progress notes

- 2026-10-15 — **Built and locally verified** (session claude/a01-44-filter-banner).

  **In plain words.** With a Show filter on, areas, iterations and handoffs with no matching task are no longer drawn, and a blue banner at the top of the list says which tasks the filter shows, in how many groups, and how many groups it left out, with a Show all button. When nothing matches, only that banner shows. While searching, a plain search banner ("2 tasks found for “alm”.") sits on top and the filter banner under it. The Show box now also shows on the Handoffs tab. Texts in English, Chinese and Japanese.

  **Evidence.** `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (438 texts, 71 codes), `test` PASS (95, of which 6 new in `tests/stopFlow.test.ts`: groups left out, handoffs as groups, banner counts, nothing matches, while searching, search banner). `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 2 + 46, core 42, desktop 26 + 1 ignored, store 65 + 5 + 4 + 5). `git diff --check` PASS. Chinese and Japanese banner texts checked by rendering them with the pure functions (for example "**3 个任务**需要人处理（…），分布在 7 个领域中的 2 个。其余 5 个领域没有匹配，已省略。"). Debug desktop app built (`target/debug/bundle/macos/Allomos.app` in this worktree) and opened on a copy of the taskbag at `~/allomos-tests/a01-44-filter-banner/trial-taskbag` (no `.git`, so nothing can be pushed from it; code folder linked as `~/allomos-tests/a01-44-filter-banner/allomos`). **The window itself was not looked at by the agent** (screen access was declined): NOT RUN, left to the human check. Windows: untested.

  **Human check (steps).** `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-44-filter-banner/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-44-filter-banner/trial-taskbag`, then:
  1. By area, set Show to *Needs a human*: only areas holding such a task show; a blue banner under the tabs reads like "**N tasks** need a human (…), in X of Y areas. The Z areas with none are left out." with Show all; the areas left keep their "N more hidden by the filter" line.
  2. Set Show to *Blocked* (or any state no task is in): only the banner shows, "No task is blocked. All N areas are left out."; By iteration says the same for iterations.
  3. Click Show all in the banner: the filter goes back to All states, the banner goes and every group is back.
  4. Set Show to *Waiting* and search `A01`: the plain search banner ("N tasks found for “A01”.") is on top, the blue one under it ("**K of them** are waiting, in … . M more are listed below, hidden by the filter."), and the hidden ones listed after the groups.
  5. Handoffs tab with Show = *In progress*: only handoffs naming an in-progress task show, with the banner counting handoffs.
  6. Switch the language to 中文 and 日本語: the banners follow.
