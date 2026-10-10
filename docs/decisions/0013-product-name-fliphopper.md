# 0013 — The product is called FlipHopper; a project's data is its seabag (to be renamed taskcase)

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-06 | 2026-10-06 | will.w.hui |

## In plain words

The product was called **Glidebox**. It is now **FlipHopper**. The name says what the product is for: a person or a project **hops back and forth between AI agents** (Claude Code, Codex, …) and is **handed between people**, as often as needed. *Flip* is the English "flip-flop", switching sides again and again — the Chinese 反复横跳.

The folder that holds a project's tasks, formerly the **task box**, is now the **seabag**: a sailor's bag holding everything they own, carried from ship to ship, like a project's belongings carried from agent to agent and person to person. Chinese **水手袋**, Japanese **シーバッグ**.

## Decision

| What | Before | Now |
|---|---|---|
| Name in text | Glidebox | **FlipHopper** |
| Name where code needs one lower-case word | `glidebox` | **`fliphopper`** |
| Command and short alias (R88) | `glidebox` / `gb` | **`fliphopper`** / **`fh`** |
| A project's folder (R87) | task box (任务盒, タスクボックス) | **seabag** (水手袋, シーバッグ) |
| Where a new seabag goes | `.glidebox/` inside a folder with other files; an empty folder became the task box itself | **always `.fliphopper/seabag/`** inside the folder the user picks, also an empty one, so agents, `fh` and the app look in one place only. `.fliphopper/` leaves room for other FlipHopper files later. |
| Format tag in `project.json` | `glidebox/1` | **`fliphopper/1`** |
| App ID | `com.glidepace.glidebox` | **`io.fliphopper`** |
| GitHub repository | `mypals-ml/glidebox` | **`mypals-ml/fliphopper`** (GitHub redirects the old address) |
| Local folder | `/Users/wanghui/coding/glidepace/glidebox` | **`/Users/wanghui/coding/glidepace/fliphopper`** |

**No conversion of old data.** FlipHopper does not read `.glidebox/` folders or the `glidebox/1` format. The only seabags that existed were test projects made during Iteration 1.

## Amendment 2026-10-10: the seabag becomes the taskcase

**Decided by the owner, 2026-10-10.** The folder that holds a project's tasks is renamed from **seabag** to **taskcase**: Chinese **任务箱**, Japanese **タスクケース**. The word says what it is (a case full of tasks) and keeps the picture of carrying it from agent to agent.

- The folder on disk and the command option are renamed too: `.fliphopper/taskcase/`, `fh --taskcase`. FlipHopper keeps reading old `.fliphopper/seabag/` folders, so no project breaks.
- Other names considered: 手提箱 (the owner chose 任务箱 for Chinese), taskfolder (clashes with "FlipHopper folder" and "project folder"), shoulderbag, 背包, 双肩包.
- **Not built yet.** Until [`Task-A01-20`](../tasks/Task-A01-20-rename-seabag-to-taskcase.md) is done, the product and its documents still say "seabag". The owner checked a rename of the app itself to "Handoff" or similar and kept FlipHopper for now (2026-10-10, [`Task-A00-20`](../tasks/Task-A00-20-app-name-research.md)).

## Why this name

The owner compared several names after a name check ([`Task-A00-17`](../tasks/Task-A00-17-rename-to-crossjump.md#name-decision)). CrossJump, the first choice, is used by three game makers; CrossJumper is a person's name and social handles; CrossHopper reads two ways and sits near the Hopper and Grasshopper trademarks; SideHopper is a well-known Nintendo enemy. FlipHopper was free on crates.io, npm, GitHub and the `.io` and `.com` domains, and a web search found no product, company or trademark. This was not a legal trademark search; one is advised before selling under the name.

## Where the old name stays

**Current documents** use FlipHopper: the agent guides, README, board and handoff, glossary, product documents, workflow rules, decision records and open cards. **Dated history keeps "glidebox"**, because it records what was true then:

- discussion records ([discussions/](../discussions/)), research reports ([research/](../research/)) and the dated analysis ([analysis/](../analysis/));
- finished cards, and dated lines in any card's Notes;
- lines that state the old name on purpose, such as "renamed from `glidebox`".

Card file names never change ([CARDS.md](../workflow/CARDS.md#card-ids-and-files)), so `Task-A01-03-gb-command.md` and `Task-A00-17-rename-to-crossjump.md` keep their names.

## Consequences

- [0010](./0010-project-data-folder.md), [FILE-FORMAT.md](../product/FILE-FORMAT.md), [GLOSSARY.md](../GLOSSARY.md) and [REQUIREMENTS.md](../product/REQUIREMENTS.md) R87–R88 follow this record.
- The `fh` command ([`Task-A01-03`](../tasks/Task-A01-03-gb-command.md)) is built once, with the new name.
- A seabag made by an earlier build must be created again.
