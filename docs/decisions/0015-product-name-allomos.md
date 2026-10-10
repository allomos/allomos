# 0015 — The product is called Allomos (换肩); a project's data is its taskbag

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-10 | 2026-10-10 (rename started 2026-10-11) | will.w.hui |

Supersedes the name in [0013](./0013-product-name-fliphopper.md) (FlipHopper, seabag).

## In plain words

The product was called **FlipHopper**. It is now **Allomos**: Greek *állos* ("other") plus *ōmos* ("shoulder"), **the other shoulder**. Its Chinese name is **换肩** (huàn jiān): shifting a load to the other shoulder so the carrying goes on. That is what the product does: when one AI agent runs out or a teammate takes over, the project moves to another shoulder without losing its tasks, progress or notes. Japanese **アロモス**.

The folder that holds a project's tasks, the **seabag**, is now the **taskbag**: a bag is carried on the shoulder. Chinese **任务包**, Japanese **タスクバッグ**.

## Decision

| What | Before | Now |
|---|---|---|
| Name in text | FlipHopper | **Allomos**; Chinese screens **换肩**, Japanese screens **アロモス** |
| Lower-case name in code | `fliphopper` | **`allomos`** |
| Command and short alias | `fliphopper` / `fh` | **`allomos` / `alm`**; `fh` is not kept |
| A project's folder | seabag, `.fliphopper/seabag/` | **taskbag, `.allomos/taskbag/`** |
| Format tag in `project.json` | `fliphopper/1` | **`allomos/1`** |
| Environment settings | `FLIPHOPPER_*` | **`ALLOMOS_*`** |
| App ID and settings folder | `io.fliphopper` | **`io.allomos`** |
| Icon | the hopper | **kept** |
| GitHub repository, local folder | `mypals-ml/fliphopper`, `…/glidepace/fliphopper` | **`mypals-ml/allomos`, `…/glidepace/allomos`** (owner approved 2026-10-11) |

**An old FlipHopper project is treated like any other task system** (owner, 2026-10-11). Allomos does not open `.fliphopper/seabag/` or read `fliphopper/1`; a folder holding one gets a new, empty taskbag, and the open-folder flow offers its usual step: one AI agent finds the existing tasks (here, FlipHopper's files) and, after **Go ahead** in the app, moves them in with `alm` ([OPEN-FOLDER-FLOW](../product/OPEN-FOLDER-FLOW.md)). The old FlipHopper section in an `AGENTS.md` does not count as telling agents about Allomos, so the app offers to refresh it. Only the app's interface-language setting is read from FlipHopper's settings file until Allomos saves its own. FlipHopper's own repository moved its folder once by hand (`git mv`).

## Why this name

Checked on 2026-10-10 and 2026-10-11 ([research](../research/app-name-handoff.md), [`Task-A00-21`](../tasks/Task-A00-21-rename-to-allomos.md)):
- **free:** crates.io, npm and GitHub; `allomos.com`, `.io`, `.dev` and `.app` not registered;
- **no mark:** no ALLOMOS, 换肩 or アロモス mark in WIPO's Global Brand Database, which covers the US, Japan, the EU, the UK and Madrid marks;
- **similar marks:** ALLMOS (US, phone cases), ALTOMOS (Japan, medical supplies, broad list including software) and ALOMOS (South-East Asia), judged low risk by the agent;
- **still open with the owner:** China's register (CNIPA) and an attorney's view.

Names compared on the way: Handoff, HandoffHub, TagTeamHub, Batonhop, TaskHub, ProjectHub, OtherShoulder, ShoulderShift, Spallio, Ombroca, Cambro, Allom (an exact French mark for software), Allamos (reads as "Los Alamos"), Alagomos, Omalagi.

## Where the old name stays

As [0013](./0013-product-name-fliphopper.md#where-the-old-name-stays) did for "glidebox": dated history keeps "FlipHopper" and "seabag" (discussion records, research, analysis, finished cards, dated notes, decks), and so do file names, which never change. Current documents use Allomos.
