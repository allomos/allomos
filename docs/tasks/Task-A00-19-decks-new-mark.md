# Task-A00-19-decks-new-mark

*Card type: documents. Area A00 (Planning). Requested by the owner after [`Task-A07-01`](./Task-A07-01-app-icon-hopper.md) gave the app a new icon (2026-10-06).*

## What gets built (plain words)

The English and Chinese road show decks show the app's new mark instead of the old hopper drawing: two pairs of balls hopping through one spark, jade side to side and terracotta up and down, on sand. The decks and the app then look like the same product.

| Piece | What it is | Where |
|---|---|---|
| Deck mark | The top-bar mark on every slide, and any other place the hopper drawing appears, replaced with the new mark. It keeps its flat colours on the decks' dark and light slides. | [`docs/pitch/roadshow-en.html`](../pitch/roadshow-en.html), [`roadshow-zh.html`](../pitch/roadshow-zh.html) |
| Mark file | `hopper-mark.svg` replaced by the new mark, so the decks, the README and the app all point to one drawing. | [`docs/pitch/`](../pitch/) |
| README | The stale line "the app icon follows it" corrected: the app icon and the decks now share the new mark. | [`docs/pitch/README.md`](../pitch/README.md) |
| Published decks | Both decks republished at their **existing addresses**, as the README's "Updating it" steps describe. | claude.ai Artifacts (private) |

**What you can see at the end:** open either deck address. Every slide's top bar shows the new mark, in the same colours as the app icon.

## Details for the worker

- **Outcome.** The decks, the app icon and the app window all show the same mark.
- **Decisive unknowns.**
  - Whether the mark needs a lighter outline or tile behind it on the decks' darkest slides to stay visible. The worker checks every slide.
  - Whether the hero slide's seabag drawing should also change. Recommended: no. That slide tells the seabag story; only the brand mark changes.
- **Files owned.** `docs/pitch/roadshow-en.html`, `roadshow-zh.html`, `hopper-mark.svg` (replaced, or renamed with links updated), `README.md`; this card.
- **External operations requested.** Republish the two decks at their existing private addresses: `https://claude.ai/artifact/MGmzN64hzvRqWAMmvt1qyU` (English) and `https://claude.ai/artifact/XVe1UxXRUVt7WytcDgcABU` (中文). Only the mark changes; nothing is shared or made public. Cleanup: none, as the same pages are updated in place.
- **Done means mechanically.**
  - `git diff --check` passes; links in `docs/pitch/README.md` resolve.
  - Every slide of both decks has been looked at in a browser with the new mark visible; recorded on the card.
  - Both addresses show the new mark after republishing; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | A card to update the decks with the new mark ("ok") | owner (chat) |

The republish (external operation above) is **not yet approved**.

## Notes

- 2026-10-06 — Card written by the agent from the owner's request.
