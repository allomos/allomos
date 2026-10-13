# Task-A01-28-forget-missing-last-folder

*Card type: coding. Area A01 (Tracking). Written 2026-10-13 at the owner's request, after the owner saw the warning in a build of main. **Not started: backlog until the owner approves it.***

## What gets built (plain words)

When Allomos starts without a folder given, it reopens the last folder this computer had open (`LAST_ROOT_KEY` in the window's storage, `apps/web/src/App.tsx`). If that folder has since been deleted or moved, a red bar says the folder doesn't exist, on the start page, though the person asked for nothing. Seen 2026-10-13: a test folder had been deleted after its card was finished.

After this card, a remembered folder that no longer exists is quietly forgotten: Allomos shows the start page with no warning, and doesn't try it again at the next start. A folder the person *gives* (on the command line, or picked) that doesn't exist is still reported, as today ([`A01-04`](./Task-A01-04-startup-folder-argument.md)).

## Details for the worker

- **Outcome.** Starting Allomos after the last folder was deleted shows the start page without a warning.
- **Decisive unknowns.** Whether the same folder should also leave File › Open Recent ([`A01-11`](./Task-A01-11-recent-folders-menu.md)). Proposed: yes, only when it's the remembered folder that failed at start; other recent entries are left alone, since a folder on a removable disk may come back. Owner to confirm.
- **Files owned.** `apps/web/src/App.tsx` (the start-up reopen), a small plain module and test if the decision needs one; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Checked by hand in a debug build: open a test folder, quit, delete the folder, start Allomos → start page, no red bar; start again → still none.
  - Starting with a missing folder given on the command line still shows the warning.
  - Typecheck, translation check and tests pass.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | Add the card ("ok") | owner (chat) |

## Notes

- 2026-10-13 — Asked for by the owner. Checked that no card covers it. Next: the owner answers the Open Recent question and approves the card.
