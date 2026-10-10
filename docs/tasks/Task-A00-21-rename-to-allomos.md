# Task-A00-21-rename-to-allomos

*Card type: docs and coding. Area A00 (Planning). Prepared at the owner's request (2026-10-10): "prepare the card to change app name to Allomos, but don't proceed." **Not approved: nothing here may start until the owner moves this card to ready.***

## What gets built (plain words)

The app is renamed from **FlipHopper** to **Allomos**. The name is Greek *állos* ("other") plus *ōmos* ("shoulder"): **the other shoulder**. That is the picture of 换肩 (huàn jiān): the load moves to the other shoulder and the carrying goes on, as a project moves to another AI agent or teammate without losing its plan. **Chinese name 换肩; Japanese アロモス.** Research: [research/app-name-handoff.md](../research/app-name-handoff.md) (A00-20).

| What | Now | After |
|---|---|---|
| Name in text | FlipHopper | **Allomos** (Chinese **换肩**, Japanese **アロモス**) |
| Lower-case name in code | `fliphopper` | **`allomos`** |
| Command and short alias | `fliphopper` / `fh` | **`allomos` / short alias to be chosen** (free on this Mac: `am`, `al`, `alm`) |
| Hidden folder in every project | `.fliphopper/` | **`.allomos/`**, with the old folder still read |
| Format tag in `project.json` | `fliphopper/1` | **`allomos/1`**, with `fliphopper/1` still read |
| App ID | `io.fliphopper` | **`io.allomos`** (or the domain the owner registers) |
| Environment variables | `FLIPHOPPER_*` | **`ALLOMOS_*`**, old names still read |
| GitHub repository | `mypals-ml/fliphopper` | **`mypals-ml/allomos`** (GitHub redirects the old address) |
| Local folder | `…/glidepace/fliphopper` | **`…/glidepace/allomos`** |
| App icon and mark | the hopper | to be decided: keep, or a new mark for "the other shoulder" |
| Decks and published pages | FlipHopper | Allomos |

**Done together with the taskbag/taskcase rename ([`A01-20`](./Task-A01-20-rename-seabag-to-taskcase.md))**, so every project moves only once: `.fliphopper/seabag/` becomes `.allomos/<new folder word>/` in one step.

## Details for the worker

- **Outcome.** People and agents see Allomos (换肩 in Chinese) everywhere; every existing project keeps working and moves its folder once; the `allomos` command works with the old `fh` still accepted for a while.
- **Decisive unknowns (owner).**
  1. Go ahead with the rename at all (this card is prepared, not approved).
  2. The short command alias: `am`, `al` or `alm` (all free on this Mac), or another.
  3. Keep `fh` as a second alias for a transition period, and how long.
  4. Register `allomos.com` (and `.io` / `.dev` / `.app`): free on 2026-10-10. Buying a domain is the owner's purchase; the agent cannot do it.
  5. Keep the hopper icon, or a new mark.
  6. A proper trademark search in Japan, China and the US before selling under the name (the A00-20 check was registries and web search only).
- **Decisive unknowns (worker).** Every place the old name lives: run `grep -ri fliphopper` and list them on this card before changing any; decide with the owner which dated history keeps "FlipHopper" (as [0013](../decisions/0013-product-name-fliphopper.md#where-the-old-name-stays) did for "glidebox").
- **Files owned.** The whole repository's current documents, code, app metadata and texts (one-time rename); a new decision record superseding 0013's name, or an amendment; this card.
- **External operations (each needs the owner's exact approval, recorded below).** Rename the GitHub repository and update `origin`; rename the local folder (every session's worktrees move with it); republish the decks and any published pages; register domains (owner's own purchase).
- **Done means mechanically.** The usual checks pass; a project made by FlipHopper (`.fliphopper/seabag/`, `fliphopper/1`) opens in Allomos and moves to the new folder once; `allomos` and the chosen alias work, and `fh` still works with a note during the transition; the app shows Allomos / 换肩 / アロモス; a name check of the final words is recorded.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-10 — Card prepared by the agent; **not started**. Name check: `allomos` free on crates.io, npm and GitHub; `allomos.com`, `.io`, `.dev` and `.app` not registered; no product found by web search (A00-20 research).
