# Task-A07-01-app-icon-hopper

*Card type: coding. Area A07 (Platform & release). Requested by the owner, 2026-10-06, while reviewing the deck ([`Task-A00-18`](./Task-A00-18-positioning-and-deck.md)).*

## What gets built (plain words)

The FlipHopper app gets a new icon: **a person hopping with a seabag over the shoulder**, the idea behind the name (hopping between AI agents and teammates, carrying the project). The drawing already exists as [hopper-mark.svg](../pitch/hopper-mark.svg) and is used in the road show decks.

| Piece | What it is | Where |
|---|---|---|
| App icon | A square app icon built from the mark: the mark on a plain tile that reads at Dock and Finder sizes (16 px to 1024 px), in the deck's colours. | `apps/desktop/src-tauri/icons/` (all sizes regenerated from one 1024 px source with `tauri icon`) |
| Mark inside the app | The top-left mark in the window (today a box with a line, from the Glidebox days) becomes the hopper. | `apps/web/src/App.tsx`, `apps/web/src/styles.css` |

**What you can see at the end:** the built `FlipHopper.app` shows the hopper in the Dock, in Finder and in the window's top-left corner.

## Details for the worker

- **Outcome.** The app's icon and in-app mark show the person hopping with a seabag.
- **Decisive unknowns.** How to turn the SVG into the 1024 px PNG without adding a dependency (for example macOS `qlmanage` or `sips`, or a pinned tool already present); whether the thin dashed hop lines survive at 16–32 px, or the small sizes drop them.
- **Files owned.** `apps/desktop/src-tauri/icons/*`, `apps/web/src/App.tsx`, `apps/web/src/styles.css`, `docs/pitch/hopper-mark.svg` (if the drawing needs a small-size variant), this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - The built app shows the new icon in the Dock and Finder and the new mark in the window, in light and dark mode; screenshots recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | "Change the app icon into something that expresses a person hopping with that seabag." | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent from the owner's request. The mark was drawn and used in the decks under A00-18; applying it to the app is this card.
