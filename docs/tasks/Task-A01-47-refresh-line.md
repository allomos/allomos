# Task-A01-47-refresh-line

*Card type: coding. Area A01 (Tracking). Written 2026-10-15. Builds on [`A01-03`](./Task-A01-03-gb-command.md) (the app follows changes once a second) and [`A01-44`](./Task-A01-44-filter-banner.md) (the banners under the tabs). Task text: `alm show A01-47`.*

## What gets built (plain words)

The task list already follows changes by itself: once a second the app checks whether the taskbag files changed and, if they did, loads them again; the filter and the search are worked out again from the new tasks. But nothing on screen says so, and the person cannot ask for it. After this card, a quiet grey line at the top of the list says when the list was last refreshed and how, with an orange **Refresh now** link:

- "Refreshed automatically at 14:32:05 · the taskbag files changed"
- "Refreshed by you at 14:35:12 · nothing changed" / "· 1 task changed (A01-33)"
- "Loaded at 14:20:03, when the taskbag was opened"

It sits under the search banner and the filter banner when they show, alone when they don't, on By area, By iteration and Handoffs alike.

| Piece | What it is | Where |
|---|---|---|
| Message and what changed | Pure functions: which message to show (opened / automatic / by you), and which tasks differ between two loads | `apps/web/src/` (a small flow module, tested in `apps/web/tests/`) |
| The line | Grey text, no box, **Refresh now** as a link styled like *Collapse all*; under `ListBanners` and in the Handoffs tab | `apps/web/src/Search.tsx`, `apps/web/src/Handoffs.tsx`, `apps/web/src/App.tsx` |
| Reload | The once-a-second follow records when it reloads; **Refresh now** reloads even when the fingerprint is unchanged | `apps/web/src/App.tsx` |
| Texts and look | en, zh-CN, ja | `apps/web/src/i18n/*.json`, `apps/web/src/styles.css` |

## Details for the worker

- **Outcome.** A person sees when the list was last refreshed, automatically or by them, and can refresh it themselves.
- **Decisive unknowns.** None. The approver chose mockups 1–3 on 2026-10-15: the line under the banners (not folded into the search banner), alone when nothing is filtered, and the three messages above.
- **Rules.**
  - "Automatically" is stamped only when the check finds a change and reloads, not on every one-second check (so the time can be old in a quiet hour; that is intended).
  - **Refresh now** always reloads, then reports what differed from the list before it.
  - The selected task stays selected; the filter and the search follow the reload as now.
- **Choices** (routine, made by the worker in a quiet loop run, each recorded with `alm ai-decided A01-47`):
  - Decided: when Refresh now finds no task changed but areas, iterations or the project did, the line says so ("no task changed; areas, iterations or the project did") instead of "nothing changed", so it is never false.
  - Decided: the changed tasks are named up to five, then "and N more"; added and removed tasks count as changed, and so does a new AI decision.
  - Decided: a change made in this window (task pane, new task, edit) reads the taskbag again as before but does not stamp the line "automatically": the window notes the new stamp itself, so the once-a-second follow does not read it a second time and suggest someone else changed it. Refresh now notes the stamp too, so the follow cannot replace its "by you" message a second later.
  - Decided: Refresh now is greyed out while it reads; if the read fails, the usual error banner shows and the line stays as it was.
  - The pure functions are in `apps/web/src/refreshFlow.ts` (`changedTasks`, `whatChanged`, `clock`, `refreshText`), the line is `RefreshLine` in `Search.tsx`; the time is the computer's local time, `HH:MM:SS`.
- **Not included** (a card each, if wanted): a keyboard shortcut for Refresh; a notice when the automatic check keeps failing.
- **Files owned.** The files above; this card.
- **External operations.** None.
- **Done means mechanically.** `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Mockups 1–3 (line under the banners, Refresh now link, the three messages); make the card ready | approver | 2026-10-15 | chat; task note of A01-47 |

## Progress notes

- 2026-10-16 — **Built and locally verified** (session claude/a01-47-refresh-line, quiet loop run).

  **In plain words.** A quiet grey line now sits at the top of the list, under the search and filter banners (alone when they do not show), on By area, By iteration and Handoffs. It says when the list was last read and how: "Loaded at …, when the taskbag was opened", "Refreshed automatically at … · the taskbag files changed" (only when the once-a-second check found a change), or, after the orange **Refresh now** link, "Refreshed by you at … · nothing changed" / "· 1 task changed (A01-33)". Refresh now always reads the taskbag again and compares it with the list before. The selected task stays selected, and the filter and search follow the new tasks as before. Texts in English, Chinese and Japanese.

  **Evidence.** `npm --prefix apps/web run typecheck` PASS, `build` PASS, `check:i18n` PASS (450 texts, 68 codes), `test` PASS (104, of which 6 new in `tests/refreshFlow.test.ts`: clock, the three messages, counts and the five-ID cut, other changes, changed/added/removed tasks with field order ignored, `whatChanged`). `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (cli 46, core 42, desktop 27 + 1 ignored, store 62 + 5 + 4 + 5). Debug desktop app built (`target/debug/bundle/macos/Allomos.app` in this worktree). Trial taskbag copied to `~/allomos-tests/a01-47-refresh-line/trial/.allomos/taskbag` (no `.git`). **The window itself was not looked at by the agent**: NOT RUN, left to the human check. Windows: untested.

  **Human check (steps).** `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-47-refresh-line/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-47-refresh-line/trial`, then:
  1. At the top of By area: a grey line "Loaded at HH:MM:SS, when the taskbag was opened" with an orange **Refresh now**.
  2. Click a task (for example A01-33) so the pane opens. In a terminal: `/Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-47-refresh-line/target/debug/alm --taskbag ~/allomos-tests/a01-47-refresh-line/trial/.allomos/taskbag --as trial note A01-33 "trial note"`. Within a second the line reads "Refreshed automatically at … · the taskbag files changed", the pane shows the new handoff note, and A01-33 stays selected.
  3. Click **Refresh now** with nothing changed: "Refreshed by you at … · nothing changed".
  4. Run the same command with another text and click **Refresh now** at once: either message is right (the automatic check may get there first); when Refresh now gets there first it reads "· 1 task changed (A01-33)".
  5. Set Show to *Waiting* and search a word: the line sits under both banners; turn both off: it is alone at the top. Same on By iteration and Handoffs.
  6. Change something with the task pane: the list updates and the line does not say "automatically".
  7. Switch the language to 中文 and 日本語: the line follows.
