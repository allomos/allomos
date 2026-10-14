# Task-A01-36-search

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-14. Builds on [`A01-15`](./Task-A01-15-iterations-view.md) (grouping by iteration), [`A01-16`](./Task-A01-16-task-pane.md) (task pane), [`A01-26`](./Task-A01-26-collapsible-sections.md) (collapsing groups) and [`A03-06`](./Task-A03-06-agent-handoff-rules.md) (handoffs in the app). Task text and owner decisions: `alm show A01-36`.*

## What gets built (plain words)

A project with many tasks is slow to scan by eye. After this card the main window has a **search box** above the task list. Typing in it shows only the tasks whose ID, title or description holds the typed words, and the handoffs (the notes sessions leave for whoever continues) that hold them. The words are marked in the results. Emptying the box, or pressing Esc in it, brings back the whole list exactly as it was, including which groups were collapsed.

| Piece | What it is | Where |
|---|---|---|
| What matches | Pure functions: the words of a search, whether a task or handoff holds them, where in the text (for marking), a one-line excerpt of a long text | `apps/web/src/searchFlow.ts` (tested in `apps/web/tests/searchFlow.test.ts`) |
| The box, marks, found handoffs | Search box with clear button, `<mark>` highlighting, the description excerpt under a task, the "Handoffs found" section | `apps/web/src/Search.tsx` |
| The task list | Filters the groups, opens collapsed groups that hold a match, "nothing found" message, Cmd/Ctrl+F | `apps/web/src/App.tsx` |
| The Handoffs tab | The same search filters the handoff list and marks its summaries | `apps/web/src/Handoffs.tsx` |
| Texts and look | 8 new texts in English, Chinese and Japanese; styles | `apps/web/src/i18n/*.json`, `apps/web/src/styles.css` |

**What you can see at the end:** type `搜索` in the box: only tasks holding 搜索 stay, in their area (or iteration) groups, with 搜索 marked; a collapsed group holding one opens; handoffs holding it appear below, labelled "Handoff"; clicking one opens it in the Handoffs tab. Type `xyz`: "Nothing found". Esc: the full list, collapsed groups collapsed again.

## Details for the worker

- **Outcome.** A person finds a task or handoff by a few words of its ID, title, description or text.
- **Decisive unknowns.** None. The owner decided on 2026-10-14: collapsed groups open by themselves when they hold a match; handoffs are searched too; the web version gets the same.
- **Choices** (routine, made by the worker under A00-29):
  - **Shortcut:** Cmd+F on a Mac, Ctrl+F elsewhere, anywhere in the window; it focuses the box and selects its text. Esc in the box empties it; Esc in an empty box leaves it. The box's placeholder shows the shortcut.
  - **Matching:** the search is split at spaces (also the full-width space `　`); a task matches when **every** word is in its ID, title or description (each word may be in a different one). A handoff matches the same way over its summary, text, writer and the task IDs it names. Case does not matter. Full-width and half-width forms compare alike (NFKC: `ＡＰＩ` = `api`, half-width `ｶﾞｲﾄﾞ` = `ガイド`), and a letter with its marks counts as one character. Hiragana and katakana are not treated as the same (`かな` does not find `カナ`), and Simplified and Traditional Chinese are not mapped to each other.
  - **Highlight:** found words are marked with `<mark>` in the accent colour's pale tint (readable in light and dark mode) in the task's ID and title, and in a one-line excerpt of the description ("In the description: …", about 40 characters each side of the first match) shown only when the description holds a word.
  - **Groups:** the search applies after the state filter ("Show"); groups with no match are hidden; each group's count and progress bar stay those of the whole group. Collapsed groups holding a match open while searching without changing what is remembered as collapsed, so clearing the search restores them. Collapse all / Expand all act on the groups shown. While searching, "New area / New iteration" and the empty-list hints are hidden.
  - **Handoffs:** in the By area / By iteration lists, matching handoffs come after the tasks in a section "Handoffs found", each row labelled "Handoff" with its date, writer, task IDs and an excerpt; clicking one switches to the Handoffs tab with it open. In the Handoffs tab the same search filters the list; "Latest only" is set aside while searching (older handoffs are searched too) and its box is greyed. Words are marked in the summary and in an excerpt shown under a closed handoff; inside an opened handoff's full text they are not marked.
  - **One search for the window:** the words stay when switching between By area, By iteration and Handoffs, and are emptied when another project opens.
  - **No new loading:** task descriptions already come with the taskbag (`open_taskbag` returns each task's `description`) and handoffs with their full text (`list_handoffs` returns `body`), both read again when the taskbag's files change. Searching reads nothing more from disk.
  - **Web version:** the search is in `apps/web`, the one interface the desktop app shows and the browser and phone companions will reuse (decision 0009; companions arrive in Iteration 9, they do not exist yet). No separate code is needed for them.
- **Files owned.** `apps/web/src/searchFlow.ts`, `apps/web/src/Search.tsx`, `apps/web/tests/searchFlow.test.ts` (new); `apps/web/src/App.tsx` (TaskbagView, TaskGroup), `apps/web/src/Handoffs.tsx` (HandoffsView), `apps/web/src/i18n/{en,zh-CN,ja}.json` (`search.*` keys), `apps/web/src/styles.css` (search section at the end); this card.
- **External operations.** None.
- **Done means mechanically.**
  - The named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`).
  - Tests: words split (also at the full-width space) and lower-cased; task found by ID, title, description; every word required; Chinese, Japanese, half-width katakana and full-width Latin found; found places given in the text as written and joined; marked and unmarked stretches; excerpt on one line with "…", never cutting a two-unit character; handoffs found by summary, text, writer, task ID; groups filtered with whole counts and empty ones left out; collapsed groups with a match open while searching and are collapsed again after.
  - A debug desktop app built, and a trial project with Chinese and Japanese text under `~/allomos-tests/a01-36-search/`.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Collapsed groups open by themselves on a match; handoffs searched too; web version too | owner | 2026-10-14 | recorded in the task (`alm show A01-36`) |

## Progress notes

- 2026-10-14 — **Built and locally verified** (session claude/a01-36-search).

  **In plain words.** The main window's tool row above the task list now has a search box. Typing filters the list as you type: only tasks whose ID, title or description hold every typed word stay, in their area or iteration groups; groups with none are hidden, and collapsed groups that hold a match open (what was collapsed comes back when the search is emptied). The words are marked in the ID and title, and a task found through its description shows a one-line excerpt with the words marked. Handoffs holding the words are listed after the tasks under "Handoffs found", each labelled "Handoff"; clicking one opens it in the Handoffs tab, which the same search also filters. When nothing matches, the list says so. Cmd+F (Ctrl+F on Windows) focuses the box; Esc empties it. Everything searched was already loaded with the taskbag, so searching reads nothing more from disk. The texts are in English, Chinese and Japanese.

  **Evidence.** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 2 + 44, core 40, store 64 + 4 + 4 + 5, desktop 24 + 1 ignored); `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (346 texts, 69 codes), `test` PASS (74, of which 10 new in `tests/searchFlow.test.ts`). Debug desktop app built: `npm --prefix apps/desktop run tauri build -- --debug --bundles app` → `target/debug/bundle/macos/Allomos.app`. Trial project made with this branch's `alm` at `~/allomos-tests/a01-36-search/` (3 areas, 6 tasks in Chinese, Japanese and English, one with full-width `ＡＰＩ`, one with half-width `ｶﾞｲﾄﾞ`, one with a long description hiding `NEEDLE`, one handoff in Chinese for A01-01 and A01-02). The app itself was not driven by the agent: the window's behaviour is NOT RUN until the owner's check below. Windows: untested (no Windows machine; Ctrl+F is the shortcut there). No GitHub workflow was started.

  **Human check (steps).** `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-36-search/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-36-search`, then:
  1. Collapse the group **A02 同期 Sync**. Press Cmd+F: the search box is focused.
  2. Type `搜索`: A01-01 stays (搜索 marked in its title), the other groups disappear, and "Handoffs found" lists the claude/trial handoff with 搜索 marked.
  3. Type `needle`: only A02-01 stays, its group A02 opened although collapsed, with "In the description: …NEEDLE…" marked.
  4. Type `ガイド` (finds A01-02 through half-width ｶﾞｲﾄﾞ in its description) and `api` (finds A03-01 through full-width ＡＰＩ).
  5. Type `xyz`: "Nothing found for “xyz” …".
  6. Press Esc: the full list comes back, with A02 collapsed again.
  7. Type `高亮`, click the handoff row: the Handoffs tab opens with that handoff open. Switch the language (globe menu) to English and Japanese: the box's placeholder and messages follow.
