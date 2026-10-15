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
- **Not included** (a card each, if wanted): a keyboard shortcut for Refresh; a notice when the automatic check keeps failing.
- **Files owned.** The files above; this card.
- **External operations.** None.
- **Done means mechanically.** `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| Mockups 1–3 (line under the banners, Refresh now link, the three messages); make the card ready | approver | 2026-10-15 | chat; task note of A01-47 |
