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

**In plain words.** The owner tried the first hopper drawing in the Dock and asked for something more abstract and artistic. After several rounds of sketches the owner chose **layout Y2 with colours D4**. Two pairs of balls cross on a sand tile:
- **Jade, side to side:** a ring on the left marks where a ball was, and a solid ball on the right marks where it arrived. This is hopping between AI agents.
- **Terracotta, up and down:** the same, for hopping between teammates.
- **Mustard spark:** the jump itself, in the middle.

All colours are flat; the owner asked for no gradients. The mark at the top left of the window is the same drawing, and it keeps these colours in light and dark mode.

**How the design was chosen** (owner, 2026-10-06; sketches kept outside the repository):
1. The hopper figure (built, then judged too plain).
2. Abstract directions A, B and C. B was chosen.
3. B as brush strokes, then as a jump with the feet together.
4. "Two balls side by side that feel like 瞬移 (teleporting)", with no gradients.
5. Eight bolder options for small sizes (V1–V8). V5 and V7 were taken further.
6. "Four balls, a pair side by side and a pair stacked" (X1–X4, Y1–Y4). Y2 was chosen.
7. Thirty-three colour schemes. D4 was chosen.

**Files:**
- [`icons/source.svg`](../../apps/desktop/src-tauri/icons/source.svg) is the drawing used from 48 px up.
- [`icons/source-small.svg`](../../apps/desktop/src-tauri/icons/source-small.svg) is used at 16–32 px, with bigger balls, thicker rings and a bigger spark, so the small icon stays clear.
- [`apps/desktop/scripts/make-icons.sh`](../../apps/desktop/scripts/make-icons.sh) rebuilds every icon file from those two drawings. It uses `rsvg-convert`, ImageMagick, `iconutil` and `tauri icon`, and adds nothing to the project's packages.
- The decks still use the old hopper drawing ([`hopper-mark.svg`](../pitch/hopper-mark.svg)); updating them is not part of this card.

**Why development mode shows no icon:** `npm … run dev` runs the bare program without an app bundle, so macOS has no icon to show there. Only the built `FlipHopper.app` shows one.

**Checks (2026-10-06):**

| Check | Result |
|---|---|
| `cargo fmt --check`, clippy, `cargo test --workspace` (36) | PASS |
| `npm --prefix apps/web run typecheck` / `build` / `check:i18n` | PASS |
| `.icns` looked at by the agent at 256, 32 and 16 px | PASS |
| `tauri build --debug --bundles app`: the bundle's `icon.icns` is the new file | PASS |
| Owner check of the first (hopper) icon in the Dock, Finder and window | PASS (owner, 2026-10-06), then redesigned |
| Owner check of the Y2/D4 icon in the Dock, Finder and window | **PASS** (owner, 2026-10-06) |

## Notes

- 2026-10-06 — Card written by the agent from the owner's request. The mark was drawn and used in the decks under A00-18; applying it to the app is this card.
- 2026-10-06 — Owner verified the Y2/D4 icon; merged into `main`. **Done.** Later sketches (V5 with a star in the centre, S1–S8) were drawn but not chosen.
