# Task-A00-24-decks-allomos-taskbag

*Card type: documents. Area A00 (Planning). Requested by the owner on 2026-10-12: the road show decks still said FlipHopper and seabag, though the rename to Allomos and the taskbag ([0015](../decisions/0015-product-name-allomos.md), [`A00-21`](./Task-A00-21-rename-to-allomos.md)) is done.*

## What gets built (plain words)

The English and Chinese road show decks use the product's current names: **Allomos** (Chinese **换肩**) instead of FlipHopper, and the **taskbag** in `.allomos/taskbag/` (Chinese **任务包**) instead of the seabag. The example commands use the real command, `alm`, instead of `fh`.

| Piece | What it is | Where |
|---|---|---|
| Decks | Every "FlipHopper", "seabag", "水手袋", `.fliphopper/seabag/`, `fliphopper` and `fh` replaced with the current name. The hero drawing's bag keeps its shape; its stencil reads TASKBAG / 任务包. | [`docs/pitch/roadshow-en.html`](../pitch/roadshow-en.html), [`roadshow-zh.html`](../pitch/roadshow-zh.html) |
| README | The pitch README names Allomos. | [`docs/pitch/README.md`](../pitch/README.md) |
| Published decks | Both decks republished at their **existing addresses**. | claude.ai Artifacts (private) |

**What you can see at the end:** open either deck address. No slide says FlipHopper, seabag or 水手袋; the bag on the first slide says TASKBAG (任务包) and `.allomos/taskbag/`.

## Details for the worker

- **Files owned.** `docs/pitch/roadshow-en.html`, `roadshow-zh.html`, `README.md`; this card.
- **External operations requested.** Republish the two decks at their existing private addresses: `https://claude.ai/artifact/MGmzN64hzvRqWAMmvt1qyU` (English) and `https://claude.ai/artifact/XVe1UxXRUVt7WytcDgcABU` (中文). Nothing is shared or made public.
- **Done means mechanically.** `grep -i "fliphopper\|seabag\|水手袋"` finds nothing in the two decks; `git diff --check` passes; both decks looked at in a browser; both addresses show the new names after republishing; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Update the decks to Allomos and taskbag, and republish them ("the should fit the new term of "Allomos" and "taskbag", not using fliphopper and seabag") | owner (chat) |

## Notes

- 2026-10-12 — Card written by the agent from the owner's request.
