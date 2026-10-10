# Task-A00-21-rename-to-allomos

*Card type: docs and coding. Area A00 (Planning). Prepared at the owner's request (2026-10-10): "prepare the card to change app name to Allomos, but don't proceed." **Approved by the owner the same day** (answers below); the rename itself waits until the owner says to start it.*

## What gets built (plain words)

The app is renamed from **FlipHopper** to **Allomos**. The name is Greek *állos* ("other") plus *ōmos* ("shoulder"): **the other shoulder**. That is the picture of 换肩 (huàn jiān): the load moves to the other shoulder and the carrying goes on, as a project moves to another AI agent or teammate without losing its plan. **Chinese name 换肩; Japanese アロモス.** Research: [research/app-name-handoff.md](../research/app-name-handoff.md) (A00-20).

| What | Now | After |
|---|---|---|
| Name in text | FlipHopper | **Allomos** (Chinese **换肩**, Japanese **アロモス**) |
| Lower-case name in code | `fliphopper` | **`allomos`** |
| Command and short alias | `fliphopper` / `fh` | **`allomos` / `alm`**; `fh` and `fliphopper` are **not kept** (owner) |
| Hidden folder in every project | `.fliphopper/` | **`.allomos/`**, with the old folder still read |
| Format tag in `project.json` | `fliphopper/1` | **`allomos/1`**, with `fliphopper/1` still read |
| App ID | `io.fliphopper` | **`io.allomos`** (or the domain the owner registers) |
| Environment variables | `FLIPHOPPER_*` | **`ALLOMOS_*`**, old names still read |
| GitHub repository | `mypals-ml/fliphopper` | **`mypals-ml/allomos`** (GitHub redirects the old address) |
| Local folder | `…/glidepace/fliphopper` | **`…/glidepace/allomos`** |
| App icon and mark | the hopper | **kept** (owner) |
| Decks and published pages | FlipHopper | Allomos |

**Done together with the taskbag rename ([`A01-20`](./Task-A01-20-rename-seabag-to-taskcase.md))**, so every project moves only once: `.fliphopper/seabag/` becomes **`.allomos/taskbag/`** (Chinese 任务包, Japanese タスクバッグ) in one step.

## Details for the worker

- **Outcome.** People and agents see Allomos (换肩 in Chinese) everywhere; every existing project keeps working and moves its folder once; the `allomos` command works with the old `fh` still accepted for a while.
- **Decisive unknowns (owner).**
  1. ~~Go ahead with the rename at all~~ — **approved** 2026-10-10.
  2. ~~The short command alias~~ — **`alm`**.
  3. ~~Keep `fh` for a transition~~ — **no**: `fh` is not kept. Agents' `AGENTS.md` sections must be refreshed (`alm brief`) as part of the move.
  4. ~~Register the domains~~ — **the owner does it**.
  5. ~~Keep the hopper icon~~ — **kept**.
  6. Trademark search in Japan, China and the US: the agent searches the public databases (see Notes); a professional clearance before selling stays advised.
- **Decisive unknowns (worker).** Every place the old name lives: run `grep -ri fliphopper` and list them on this card before changing any; decide with the owner which dated history keeps "FlipHopper" (as [0013](../decisions/0013-product-name-fliphopper.md#where-the-old-name-stays) did for "glidebox").
- **Files owned.** The whole repository's current documents, code, app metadata and texts (one-time rename); a new decision record superseding 0013's name, or an amendment; this card.
- **External operations (each needs the owner's exact approval, recorded below).** Rename the GitHub repository and update `origin`; rename the local folder (every session's worktrees move with it); republish the decks and any published pages; register domains (owner's own purchase).
- **Done means mechanically.** The usual checks pass; a project made by FlipHopper (`.fliphopper/seabag/`, `fliphopper/1`) opens in Allomos and moves to the new folder once; `allomos` and the chosen alias work, and `fh` still works with a note during the transition; the app shows Allomos / 换肩 / アロモス; a name check of the final words is recorded.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-10 | Rename approved; alias `alm`; `fh` not kept; owner registers domains; hopper icon kept; taskbag + 任务包 for the folder word | owner (chat) |

## Notes

- 2026-10-10 — Card prepared by the agent; **not started**. Name check: `allomos` free on crates.io, npm and GitHub; `allomos.com`, `.io`, `.dev` and `.app` not registered; no product found by web search (A00-20 research).
