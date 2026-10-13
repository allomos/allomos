# Task-A01-28-forget-missing-last-folder

*Card type: coding. Area A01 (Tracking). Written 2026-10-13 at the owner's request, after the owner saw the warning in a build of main. Approved by the owner 2026-10-13.*

## What gets built (plain words)

When Allomos starts without a folder given, it reopens the last folder this computer had open (`LAST_ROOT_KEY` in the window's storage, `apps/web/src/App.tsx`). If that folder has since been deleted or moved, a red bar says the folder doesn't exist, on the start page, though the person asked for nothing. Seen 2026-10-13: a test folder had been deleted after its card was finished.

After this card, a remembered folder that no longer exists is quietly forgotten: Allomos shows the start page with no warning, and doesn't try it again at the next start. A folder the person *gives* (on the command line, or picked) that doesn't exist is still reported, as today ([`A01-04`](./Task-A01-04-startup-folder-argument.md)).

## Details for the worker

- **Outcome.** Starting Allomos after the last folder was deleted shows the start page without a warning.
- **Decided.** The folder also leaves File › Open Recent ([`A01-11`](./Task-A01-11-recent-folders-menu.md)), but only the remembered one that failed at start, and only when its project folder is gone too; other recent entries are left alone, since a folder on a removable disk may come back (owner, 2026-10-13: "yes").
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
| 2026-10-13 | Remove it from Open Recent too ("yes"); card approved ("approve") | owner (chat) |

## Notes

- 2026-10-13 — Asked for by the owner. Checked that no card covers it. Next: the owner answers the Open Recent question and approves the card.
- 2026-10-13 — Built and locally verified. At start, a remembered folder that is gone is forgotten before anything is opened, so no error is set and the start page shows. Next: the owner tries it.

  Evidence: `typecheck` PASS; `check:i18n` PASS; `npm --prefix apps/web test` PASS (31; the change is start-up I/O, no new decision module). Debug build, checked through the window's storage (`allomos.lastRoot`, `allomos.recentFolders`): opened `~/allomos-tests/a01-28-missing-folder` → remembered and first in Open Recent; quit, deleted it, started without a folder → `lastRoot` gone, the folder gone from Open Recent, the other nine entries kept; started again → nothing remembered, nothing tried. NOT RUN by the agent: looking at the window (the red bar's absence, and the warning for a missing folder given on the command line, whose code is unchanged) — the screen tool reaches only the installed Allomos, which has the same app ID, and the owner asked not to take over the screen. Found on the way: test builds share the window's storage with the installed Allomos, so test folders appear in its Open Recent.
- 2026-10-13 — Owner checked it in the debug build (open a folder, quit, delete it, start again): "passed" — start page, no red bar. Merged into main; card done.
