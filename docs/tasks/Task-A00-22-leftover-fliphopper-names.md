# Task-A00-22-leftover-fliphopper-names

*Card type: coding and docs. Area A00 (Planning). Requested by the owner (2026-10-11): "change the term fliphopper to current name" (on card A03-07's title) and "find other left over using of term fliphopper in project". Follows [`A00-21`](./Task-A00-21-rename-to-allomos.md) and [0015](../decisions/0015-product-name-allomos.md).*

## What gets built (plain words)

| Leftover | Fix |
|---|---|
| This repository's board is still named "FlipHopper" (project id `fliphopper`); open cards A03-07 and A07-03 have "FlipHopper" in their titles. `alm` could not change a title or the project's name. | New commands: **`alm title <id> "<title>"`** and **`alm project --name "<name>" --id <id>`**; then rename the board and the two titles. |
| The board folder's `.gitignore` comment says "FlipHopper's own working files". | Say Allomos. |
| Card A07-02 names the repository `mypals-ml/fliphopper`. | `mypals-ml/allomos`. |
| FILE-FORMAT, PLAN, REQUIREMENTS (R87, R88) and AGENT_HANDOFF point to 0013 for the current naming rules. | Point to 0015. |

**Kept on purpose:** history notes ("renamed from `fliphopper` 2026-10-11"), decisions 0013 and 0015, links to 0013's file (file names never change), the code that reads FlipHopper's old settings file, finished cards, research and decks.

**Owner to decide:** the test-folder path `~/fliphopper-tests/` (the owner's own rule); `scripts/move-board-to-seabag.py` (a one-off script from A01-07 that no longer runs).

## Details for the worker

- **Files owned.** `crates/core/src/ops.rs`, `crates/cli/src/lib.rs` and its tests, `docs/workflow/CODING_RULES.md` (the `alm` table), the documents named above, this card.
- **External operations.** None.
- **Done means mechanically.** Rust checks pass; tests cover both commands (holder rules, empty title, invalid id); `alm list` shows no "FlipHopper" in open cards or the project name; `git grep -i fliphopper` in current documents finds only the kept kinds above.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | The request itself | owner (chat) |

## Notes
