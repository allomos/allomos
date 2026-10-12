# Task-A01-26-collapsible-sections

*Card type: coding. Area A01 (Tracking). Written 2026-10-12 at the owner's request ("both desktop and web"). The handoff part needs [`A03-06`](./Task-A03-06-agent-handoff-rules.md)'s Handoffs tab. **Not started: A03-06 is done; waits for the owner's approval.***

## What gets built (plain words)

The task list shows one section per area, or one per iteration when it is grouped by iteration (`AreaSection` in `apps/web/src/App.tsx`, `group-by` switch). [`A03-06`](./Task-A03-06-agent-handoff-rules.md) adds a **Handoffs tab** after Iterations, listing every handoff in the project, newest first. Today none of these sections can be folded away, so a big project means a long scroll.

After this card, every area section, iteration section and handoff section in the Handoffs tab has a toggle in its header to collapse it (header only, with the count and progress bar still visible) or expand it again. Allomos remembers which ones are collapsed per project, so they stay that way after reopening. There's also "collapse all / expand all".

It works the same in the desktop app and in the browser: both use the same interface code in `apps/web`, so one change covers both. The web companion (area A04) gets it once it exists.

## Details for the worker

- **Outcome.** A person can collapse and expand each area, iteration and handoff section in the desktop app and in the browser, and the choice is remembered.
- **Decisive unknowns.**
  - What a collapsed handoff shows (probably the date, writer, tasks covered and first line the tab already lists) and whether older handoffs start collapsed. This follows A03-06's tab design.
  - Where the collapsed state is kept: browser storage per project (like `GROUP_BY_KEY` today) or per viewer in the taskbag. Proposed: browser storage, because it's a per-viewer convenience, not project data.
- **Decided.** "Handoff section" means the handoff view A03-06 adds, the Handoffs tab, not the handoff note in task rows or the task pane (owner, 2026-10-12).
  - Build it all in one go, after A03-06 is finished, not in two parts (owner, 2026-10-12).
- **Files owned.** `apps/web/src/App.tsx` (`AreaSection`, list header), the Handoffs tab's sections (`apps/web/src/Handoffs.tsx`), `apps/web/src/styles.css`, language files (en, zh-CN, ja), tests; this card.
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

## Notes

- 2026-10-12 — Asked for by the owner. Checked that no card covers it: the closest, [`A01-19`](./Task-A01-19-create-in-app.md) (create in app) and A04-02 (companion viewing), don't collapse sections, and no code does it yet. Owner confirmed the handoff section is A03-06's Handoffs tab. Owner chose to wait for A03-06 and build it all together. Next: A03-06 is finished, then the owner approves this card.
- 2026-10-12 — A03-06 is done (owner verified); its Handoffs tab is in `apps/web/src/Handoffs.tsx`. Next: the owner approves this card (`alm ready A01-26`).
