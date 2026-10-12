# Task-A03-11-hide-template-hints

*Card type: coding. Area A03 (Agents). Written 2026-10-12 at the owner's request, from what was seen while checking [`A01-26`](./Task-A01-26-collapsible-sections.md). Approved by the owner 2026-10-12.*

## What gets built (plain words)

The handoff template ([`A03-06`](./Task-A03-06-agent-handoff-rules.md)) has hints for whoever fills it in, written between `<!--` and `-->`, for example `- **Checkout:** <!-- worktree, branch, last commit, pushed or not -->`. In Markdown such text is a *comment*: it is meant to stay hidden when the text is shown.

Allomos's own Markdown display (`apps/web/src/Markdown.tsx`) doesn't recognise comments, so when a handoff is opened in the Handoffs tab the hints show as ordinary text, mixed in with the real content. A handoff that is not filled in yet shows little else. Seen on 2026-10-12.

After this card, comments are hidden wherever Allomos shows Markdown: handoffs in the Handoffs tab, task descriptions in the task pane, and plans shown with `alm confirm`. The files themselves are not changed.

## Details for the worker

- **Outcome.** Opening a handoff, a task description or a plan in Allomos shows no `<!-- … -->` text.
- **Decisive unknowns.** None known. A comment inside a code block or inline code stays visible, as Markdown does.
- **Files owned.** `apps/web/src/Markdown.tsx`, `apps/web/src/markdownComments.ts` (comments removed before the text is read), `apps/web/tests/markdownComments.test.ts`; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: a comment on its own line, at the end of a line, spanning several lines, and two on one line are hidden; text around them is kept; a comment inside a code block or inline code stays visible; an unclosed `<!--` doesn't hide the rest of the text silently (shown as written).
  - Checked by hand: a fresh handoff from `alm handoff` opened in the Handoffs tab shows the template's headings without the hints.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Add the card ("yes") | owner (chat) |
| 2026-10-12 | Card approved ("approve") | owner (chat) |

## Notes

- 2026-10-12 — Asked for by the owner. Checked that no card covers it. Next: the owner approves the card.
- 2026-10-12 — Built and locally verified. Comments are removed before Markdown is shown; a line that held only a comment is dropped so a list stays one list. Next: the owner tries it.

  Evidence: `typecheck` PASS; `check:i18n` PASS; `npm --prefix apps/web test` PASS, 31 tests (8 new in `tests/markdownComments.test.ts`). Debug build on `~/allomos-tests/a03-11-hide-hints`, checked by hand: a fresh `alm handoff` opened in the Handoffs tab shows the template's headings and field names without hints; task A01-01's description hides its comment and still shows `<!-- inline code stays -->` in inline code.
