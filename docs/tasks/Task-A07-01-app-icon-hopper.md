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

## What was built (2026-10-06)

**In plain words.** The app icon is now the hopper, a person hopping with a seabag, on a light tile in the deck's colours. The mark at the top left of the window is the same figure. In dark mode it switches to light lines.

- **Two drawings**, both kept next to the icons so the set can be rebuilt:
  - [`source.svg`](../../apps/desktop/src-tauri/icons/source.svg) is the full mark with its dashed hop lines, used from 48 px up.
  - [`source-small.svg`](../../apps/desktop/src-tauri/icons/source-small.svg) is for 16–32 px, where the dashes turned into grey noise. It drops them and draws the figure larger and bolder.
- **How the files were made**, with no new package: `rsvg-convert` (already installed with Homebrew) turned each drawing into PNGs. `npx tauri icon` made the full set from the large one. The 32 px PNG, the macOS `.icns` file (via `iconutil`) and the Windows `.ico` file (via ImageMagick) then got the small drawing at 16–32 px. Tauri's Android and iOS icons were removed, because the project has no phone app yet.
- **Why development mode shows no icon:** `npm … run dev` runs the bare program without an app bundle, so macOS has no icon to show there. Only the built `FlipHopper.app` shows one.

**Checks (2026-10-06):**

| Check | Result |
|---|---|
| `cargo fmt --check`, clippy, `cargo test --workspace` (36) | PASS |
| `npm --prefix apps/web run typecheck` / `build` / `check:i18n` | PASS |
| Icons looked at by the agent at 256, 32 and 16 px; the window mark rendered in light and dark | PASS |
| `tauri build --debug --bundles app`: the bundle's `icon.icns` is the new file | PASS |
| Dock, Finder and the window in the running app | owner check (the agent cannot see the screen) |

## Notes

- 2026-10-06 — Card written by the agent from the owner's request. The mark was drawn and used in the decks under A00-18; applying it to the app is this card.
