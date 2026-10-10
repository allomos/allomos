# Task-A00-21-rename-to-allomos

*Card type: docs and coding. Area A00 (Planning). Prepared at the owner's request (2026-10-10): "prepare the card to change app name to Allomos, but don't proceed." **Approved by the owner the same day** (answers below); the rename itself waits until the owner says to start it.*

## What gets built (plain words)

The app is renamed from **FlipHopper** to **Allomos**. The name is Greek *állos* ("other") plus *ōmos* ("shoulder"): **the other shoulder**. That is the picture of 换肩 (huàn jiān): the load moves to the other shoulder and the carrying goes on, as a project moves to another AI agent or teammate without losing its plan. **Chinese name 换肩; Japanese アロモス.** Research: [research/app-name-handoff.md](../research/app-name-handoff.md) (A00-20).

| What | Now | After |
|---|---|---|
| Name in text | FlipHopper | **Allomos** (Chinese **换肩**, Japanese **アロモス**) |
| Lower-case name in code | `fliphopper` | **`allomos`** |
| Command and short alias | `fliphopper` / `fh` | **`allomos` / `alm`**; `fh` and `fliphopper` are **not kept** (owner) |
| Hidden folder in every project | `.fliphopper/` | **`.allomos/`**; old folders are moved in by an agent (owner, 2026-10-11) |
| Format tag in `project.json` | `fliphopper/1` | **`allomos/1`** |
| App ID | `io.fliphopper` | **`io.allomos`** (or the domain the owner registers) |
| Environment variables | `FLIPHOPPER_*` | **`ALLOMOS_*`** |
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
| 2026-10-11 | **External operations approved:** rename the GitHub repository `mypals-ml/fliphopper` to `mypals-ml/allomos` and update `origin`; rename the local folder `…/glidepace/fliphopper` to `…/glidepace/allomos` ("the project folder and the github repository name should also be changed") | owner (chat) |
| 2026-10-11 | Old FlipHopper projects: no move button; "treat it as standard need to be moved task system" | owner (chat) |
| 2026-10-10 | Rename approved; alias `alm`; `fh` not kept; owner registers domains; hopper icon kept; taskbag + 任务包 for the folder word | owner (chat) |

## Notes

- 2026-10-10 — Card prepared by the agent; **not started**. Name check: `allomos` free on crates.io, npm and GitHub; `allomos.com`, `.io`, `.dev` and `.app` not registered; no product found by web search (A00-20 research).
- 2026-10-10 — **Trademark search by the agent** in WIPO's Global Brand Database (89 registers, 77 million records, including **USPTO (US)** and **JPO (Japan)** to October 2026, EUIPO, UKIPO and Madrid international marks). **Not included: China's national register (CNIPA)**; its own search (wcjs.sbj.cnipa.gov.cn) needs an account sign-in, so the agent did not search it.
  - **No mark** named ALLOMOS (contains the word), 换肩 or 換肩 (exact), or アロモス (exact).
  - **Similar marks for software** (Nice class 9, downloadable software; class 42, software services), which a trademark examiner or an owner could raise:
    - **ALLMOS**, class 9, **registered in the USA** on July 28, 2026 (Littlesoho, Korea), and in Korea (another owner);
    - **ALOMOS**, classes 9 and 35, registered in Singapore, Malaysia and Indonesia (Pointstar Pte. Ltd.);
    - **ALTOMOS**, classes 9, 42 and others, **registered in Japan** (アルケア株式会社 / Alcare);
    - **allmos electronic**, class 9, Germany and an international registration across Europe.
  - Other similar marks are in unrelated classes (clothing, bags, finance).
  - **In plain words:** nobody owns "Allomos" or "换肩", but three names one letter away are registered for software in the US, Japan and South-East Asia. That does not block Allomos, but a trademark attorney should judge the risk of confusion before you file or sell, and a CNIPA search for China is still needed.
- 2026-10-11 — **What the close marks actually cover** (agent):
  - **ALLMOS, USA** (reg. 8363417, Littlesoho): "covers for smartphones; grip holders for smartphones" only. These are phone cases: a physical product, not software ([USPTO TSDR](https://tsdr.uspto.gov/#caseNumber=8363417&caseSearchType=US_APPLICATION&caseType=DEFAULT&searchType=statusSearch)).
  - **ALTOMOS, Japan** (reg. 6757026, アルケア株式会社 / Alcare, a medical-supplies maker): ostomy supplies, wound dressings and skin care, plus a broad defensive list that includes software (class 9) and SaaS/PaaS (class 42).
  - **ALOMOS, Singapore/Malaysia/Indonesia** (Pointstar Pte. Ltd.): class 9 (and 35), no product found on the web; not in the owner's main markets.
  - **Agent's view (not legal advice):** low risk in the US (different goods); low-to-moderate in Japan (Alcare's list includes software, but the names sound different, アルトモス against アロモス, and the businesses differ); low in South-East Asia. An attorney still decides.
- 2026-10-11 — **Owner: "then Allomos. start now"** (after comparing Allom, Allamos and the allagí + ōmos forms). Started before the owner's China search and attorney review, at the owner's choice. Claimed; branch `claude/a00-21-allomos`, done together with [`A01-20`](./Task-A01-20-rename-seabag-to-taskcase.md).
- 2026-10-11 — **Code renamed** (crates, apps): `allomos` / `alm`, `.allomos/taskbag/`, `allomos/1`, `ALLOMOS_*`, `io.allomos`; 换肩 in Chinese screens, アロモス in Japanese ones; 任务包 / タスクバッグ. **Compatibility:** `.fliphopper/seabag/` and `fliphopper/1` still read; `--seabag` and `FLIPHOPPER_*` still honoured; FlipHopper's settings file read until Allomos saves its own; `alm migrate` and the app's **Move to .allomos/taskbag/** button move an old project. Checks: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` (new test: an old project opens by folder, `--seabag` and `FLIPHOPPER_SEABAG`, and `alm migrate` moves it), web typecheck, build, `check:i18n` (160 texts, 48 codes): PASS.
- 2026-10-11 — **Current documents renamed**; new decision [0015](../decisions/0015-product-name-allomos.md) supersedes 0013's name. Kept on purpose: `mypals-ml/fliphopper`, `…/glidepace/fliphopper` (external operations, not approved yet), file names, and dated history.
- 2026-10-11 — Owner checked the built app: the name shows correctly ("passed"). **Compatibility removed** at the owner's word: `.fliphopper/seabag/`, `fliphopper/1`, `--seabag`, `FLIPHOPPER_*`, `alm migrate` and the move button are gone; an old FlipHopper project is moved in by an AI agent through the open-folder flow, like any other task system. Checks after the removal: Rust fmt, clippy, all tests; web typecheck, build, `check:i18n` (156 texts): PASS.
