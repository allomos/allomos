# Task-A01-26-collapsible-sections

*Card type: coding. Area A01 (Tracking). Written 2026-10-12 at the owner's request ("both desktop and web"). The handoff part needs [`A03-06`](./Task-A03-06-agent-handoff-rules.md)'s Handoffs tab. Approved by the owner 2026-10-12.*

## What gets built (plain words)

The task list shows one section per area, or one per iteration when it is grouped by iteration (`AreaSection` in `apps/web/src/App.tsx`, `group-by` switch). [`A03-06`](./Task-A03-06-agent-handoff-rules.md) adds a **Handoffs tab** after Iterations, listing every handoff in the project, newest first. Today none of these sections can be folded away, so a big project means a long scroll.

After this card, every area section, iteration section and handoff section in the Handoffs tab has a toggle in its header to collapse it (header only, with the count and progress bar still visible) or expand it again. Allomos remembers which ones are collapsed per project, so they stay that way after reopening. There's also "collapse all / expand all".

It works the same in the desktop app and in the browser: both use the same interface code in `apps/web`, so one change covers both. The web companion (area A04) gets it once it exists.

## Details for the worker

- **Outcome.** A person can collapse and expand each area, iteration and handoff section in the desktop app and in the browser, and the choice is remembered.
- **Decided.** "Handoff section" means the handoff view A03-06 adds, the Handoffs tab, not the handoff note in task rows or the task pane (owner, 2026-10-12).
  - Build it all in one go, after A03-06 is finished, not in two parts (owner, 2026-10-12).
  - Built as: collapsed groups and open handoffs are kept in the window's storage per project (`allomos.fold.groups:<folder>`, `allomos.fold.handoffs:<folder>`), because they're a viewer's convenience, not project data. Handoffs start collapsed, as A03-06 shows them (date, writer, first line, tasks); groups start expanded. Collapse all / Expand all act on the list shown now only.
- **Files owned.** `apps/web/src/App.tsx` (`TaskGroup`, list header), `apps/web/src/Handoffs.tsx`, `apps/web/src/foldFlow.ts`, `apps/web/src/useFold.ts`, `apps/web/src/styles.css`, language files (en, zh-CN, ja), tests; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: collapsing a section hides its tasks and keeps its header, count and progress bar; collapse all / expand all; the state survives a reload for the same project and doesn't leak to another project; the toggle can be used with the keyboard and announces its state (`aria-expanded`).
  - Language files pass the missing-text test.
  - Checked by hand in the desktop app and in a browser, grouped both by area and by iteration.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Add the card, for both desktop and web | owner (chat) |
| 2026-10-12 | "Handoff section" is the handoff view A03-06 will add | owner (chat) |
| 2026-10-12 | Wait for A03-06 and do everything together ("B") | owner (chat) |
| 2026-10-12 | Card approved ("approve") | owner (chat) |

## Notes

- 2026-10-12 — Asked for by the owner. Checked that no card covers it: the closest, [`A01-19`](./Task-A01-19-create-in-app.md) (create in app) and A04-02 (companion viewing), don't collapse sections, and no code does it yet. Owner confirmed the handoff section is A03-06's Handoffs tab. Owner chose to wait for A03-06 and build it all together. Next: A03-06 is finished, then the owner approves this card.
- 2026-10-12 — A03-06 is done (owner verified); its Handoffs tab is in `apps/web/src/Handoffs.tsx`. Next: the owner approves this card (`alm ready A01-26`).
- 2026-10-12 — Built and locally verified. A collapsed area or iteration keeps its header (name, count, progress bar) and hides its tasks; each handoff opens and closes by itself; Collapse all / Expand all sit at the right of the By area · By iteration · Handoffs buttons; the arrow keys skip tasks in collapsed groups; opening a task from a handoff opens its area. Next: the owner tries it.

  Evidence: `npm --prefix apps/web run typecheck` PASS; `check:i18n` PASS (251 texts); `npm --prefix apps/web test` PASS, 23 tests (6 new in `tests/foldFlow.test.ts`). Debug build (`tauri build --debug --bundles app`) on `~/allomos-tests/a01-26-collapse` (3 areas, 2 iterations, 6 tasks, 2 handoffs), checked by hand: collapsing I01; Collapse all by area; Expand all in Handoffs (both open); A02-01 from a handoff opens the collapsed A02 and selects it; after quitting and reopening, A01 and A03 still collapsed, A02 open, and I01 still collapsed in By iteration. NOT RUN: a separate browser check — there is no browser version yet (companions are Iteration 9); the desktop window runs the same `apps/web` code.
- 2026-10-12 — Owner checked it in the debug build: "passed". Merged into main; card done.
