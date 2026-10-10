# Task-A01-20-rename-seabag-to-taskcase

*Card type: coding. Area A01 (Tracking). **Changed by the owner later on 2026-10-10: the seabag becomes the taskbag** (Chinese **任务包**, Japanese **タスクバッグ**), not the taskcase, and the app becomes Allomos, so the folder is `.allomos/taskbag/` ([0013](../decisions/0013-product-name-fliphopper.md#amendment-2026-10-10-later-the-same-day-allomos-and-the-taskbag)). Do this card together with [`A00-21`](./Task-A00-21-rename-to-allomos.md). The card's file name keeps "taskcase" because card files never move. Below, read "taskbag / 任务包 / タスクバッグ" for "taskcase / 任务箱 / タスクケース", `.allomos/taskbag/` for `.fliphopper/taskcase/`, and `alm --taskbag` / `ALLOMOS_TASKBAG` for `fh --taskcase` / `FLIPHOPPER_TASKCASE`.*

*First decision (2026-10-10): the **seabag** is renamed **taskcase**: Chinese **任务箱**, Japanese **タスクケース**. The folder on disk and the command option are renamed too, and FlipHopper keeps reading old `.fliphopper/seabag/` folders. Recorded in [0013](../decisions/0013-product-name-fliphopper.md#amendment-2026-10-10-the-seabag-becomes-the-taskcase).*

## What gets built (plain words)

| Piece | Before | After |
|---|---|---|
| The word, in every screen, message and document | seabag / 水手袋 / シーバッグ | **taskcase / 任务箱 / タスクケース** |
| A new project's folder | `.fliphopper/seabag/` | **`.fliphopper/taskcase/`** (the folder name follows the app's name if [`A00-20`](./Task-A00-20-app-name-research.md) changes it) |
| Opening an old project | — | `.fliphopper/seabag/` still opens; FlipHopper offers to rename the folder to `taskcase` (one Git-visible rename; the person agrees) |
| Command option and setting | `fh --seabag`, `FLIPHOPPER_SEABAG` | **`fh --taskcase`, `FLIPHOPPER_TASKCASE`**; the old ones keep working, with a note to switch |
| The section in AGENTS.md (`fh brief`) and `fh guide` | "seabag" | "taskcase"; the section also says that `.fliphopper/seabag/` is the old name |

**What you can see at the end:** the app and `fh` say "taskcase" (任务箱 in Chinese); a new project gets `.fliphopper/taskcase/`; FlipHopper's own repository, still on `.fliphopper/seabag/`, keeps working until its folder is renamed.

## Details for the worker

- **Outcome.** Everyone sees the new word, new projects use the new folder, and no existing project breaks.
- **Decisive unknowns.**
  - ~~Whether the app's name changes too~~ — answered 2026-10-10: **keep FlipHopper for now** ([`A00-20`](./Task-A00-20-app-name-research.md)), so the folder is `.fliphopper/taskcase/`. **Superseded later that day: the app becomes Allomos ([`A00-21`](./Task-A00-21-rename-to-allomos.md)); the folder is `.allomos/taskbag/`.**
  - ~~Whether FlipHopper's own `.fliphopper/seabag/` is renamed~~ — answered 2026-10-10: **yes**, as the last step of this card, after the new `fh` is merged and installed (an older `fh` would not find `taskcase`).
- **Files owned.** `crates/*` (paths, option, messages, guide, brief), `apps/*` (texts in three languages), `docs/` (current documents; dated history keeps "seabag", as 0013 says for "glidebox"), this card.
- **External operations.** None.
- **Done means mechanically.** The usual checks pass; tests open an old `seabag` folder and a new `taskcase` folder; `fh --seabag` still works with a note; the app shows 任务箱 in Chinese.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-10 | Rename FlipHopper's own `.fliphopper/seabag/` too: "yes" | owner (chat) |
| 2026-10-10 | "taskcase + 任务箱"; rename the folder and the command option too ("yes", keeping old folders readable) | owner (chat) |
| 2026-10-10 | Taskbag instead of taskcase, Chinese "任务包"; folder `.allomos/taskbag/` with the app rename ("ok") | owner (chat) |

## Notes
