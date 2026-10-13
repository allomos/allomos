# Task-A03-12-element-number-labels

*Card type: coding. Area A03 (Agents). Written 2026-10-13 at the owner's request ("add a short cut keys binding for the app …"), which also asked for it to be built.*

## What gets built (plain words)

When the owner talks to an AI agent about the app's screen, it is hard to say *which* button or row they mean. After this card, pressing **Ctrl+Shift+S** in an Allomos window puts a small orange label, one letter and one digit such as `A0` or `C3`, on every element they could point at: buttons, links, fields, headings, list rows and any piece of text. Pressing Ctrl+Shift+S again (or Esc) removes them. The owner can then take a screenshot or just say "C3" and the agent knows the element.

| Piece | What it is | Where |
|---|---|---|
| Label rules | Which key toggles; how labels are named (`A0` … `Z9`, then `AA0`, `BB0` … so they never repeat); reading order (top to bottom, left to right); an element sitting exactly on top of another gets no second label | `apps/web/src/labelFlow.ts` |
| Label layer | Finds the visible elements, skips ones covered (for example behind a dialog), draws the labels on each element's top edge, follows scrolling and resizing | `apps/web/src/ElementLabels.tsx`, styles in `apps/web/src/styles.css` |

The label layer has its own React root (`apps/web/src/main.tsx`), so it keeps working even if the rest of the window fails.

## Details for the worker

- **Outcome.** In an Allomos window, Ctrl+Shift+S shows a unique letter-digit label on every pointable element; pressing it again hides them.
- **Decisive unknowns.** None. Labels are given afresh each time they are shown (and on scroll or resize), so the same element may get a different label after the window changes; a label is meant for the screen the owner is looking at now.
- **Files owned.** The files above; `apps/web/tests/labelFlow.test.ts`; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: the shortcut in either case and not with other modifiers; label names unique over 1000 elements; reading order; an element in the same place as its parent gets no label.
  - Checked by hand: in the desktop app, Ctrl+Shift+S shows labels on the task list and task pane; pressing it again hides them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | Build the shortcut (the request itself) | owner (chat) |

## Notes

- 2026-10-13 — Built. Checked in a browser against sample elements (the full app only runs inside the desktop shell): labels `A0` … `A7` appeared in reading order on a heading, two buttons, a field, two list rows, a paragraph and a link; text inside a button was not labelled twice; a second press removed them all. Next: the owner tries it in the desktop app.

  Evidence: `typecheck` PASS; `check:i18n` PASS; `npm --prefix apps/web test` PASS, 35 tests (4 new in `tests/labelFlow.test.ts`); `npm --prefix apps/web run build` PASS. Desktop (Tauri) window: NOT RUN.
- 2026-10-13 — Debug build of this branch (`target/debug/bundle/macos/Allomos.app`) opened for the owner, who tried Ctrl+Shift+S in the desktop app: "passed". Merged into main; card done.
