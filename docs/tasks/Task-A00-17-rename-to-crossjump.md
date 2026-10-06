# Task-A00-17-rename-to-fliphopper

*Card type: decision + coding + docs. Area A00 (Planning). Requested by the owner, 2026-10-06.*

*The card's file name keeps "crossjump" because card files never move ([CARDS.md](../workflow/CARDS.md#card-ids-and-files)). After the name check the owner chose **FlipHopper** instead (2026-10-06, see [Name decision](#name-decision)).*

## Why

The owner wants to rename the product from **Glidebox** to **FlipHopper**. The name carries the idea of the product: a person or a project **hops back and forth between AI agents** (Claude Code, Codex, …) and is **handed between people**, as often as needed. The front word expresses the Chinese 横跳 / 反复横跳: *flip* means switching to the other side, and "flip-flop" is the everyday English phrase for switching sides again and again. "Glidebox" described a box of tasks; "FlipHopper" describes the movement across agents and people that the product is for.

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| Decision record | Records the new name, why, and the answers below. | `docs/decisions/0013-product-name-fliphopper.md` (new) |
| The app | Window title, app name (`FlipHopper.app`), app ID, and every interface text in English, Chinese and Japanese. | `apps/desktop/src-tauri` (`tauri.conf.json`, crate names), `apps/web` (texts, storage keys) |
| The code | Rust crate and library names (`glidebox-core` → `fliphopper-core`, …), the start-up variable `GLIDEBOX_OPEN` → `FLIPHOPPER_OPEN`, code names for "task box" (`TaskBox` → `Seabag`). | `Cargo.toml`, `crates/*`, `apps/desktop/src-tauri`, `Cargo.lock` |
| The data on disk | The project folder name and the format tag written into `project.json`. | `crates/core` (`FORMAT`), `crates/store` (`DEFAULT_DIR`), [FILE-FORMAT.md](../product/FILE-FORMAT.md), [GLOSSARY.md](../GLOSSARY.md) ("task box" → "seabag") |
| The command | R88's command name and short alias, before [`Task-A01-03`](./Task-A01-03-gb-command.md) builds it. | [REQUIREMENTS.md](../product/REQUIREMENTS.md) R88, the A01-03 card |
| Documents | Every current document and the agent guides use the new name. | `AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/**` |
| Outside this repository (needs approval) | The GitHub repository name, the local folder path, and the published pitch decks. | see External operations |

**What you can see at the end:** an app called FlipHopper that opens a seabag (`.fliphopper/seabag/`), documents and agent guides that say FlipHopper, and the `fliphopper` command name ready for A01-03.

## Owner answers (2026-10-06)

| # | Question | Answer |
|---|---|---|
| 1 | Spelling | **FlipHopper** in text; **`fliphopper`** in lower case where code needs one word. (Recommendation, accepted with the card's approval.) |
| 2 | Command name | **`fliphopper`**, short name **`fh`** (replaces R88's `glidebox` / `gb`). (Recommendation, accepted with the approval.) |
| 3 | The data | The term **"task box" becomes "seabag"**: a sailor's bag holding everything they own, carried from ship to ship, like a project's belongings carried from agent to agent and person to person. Chinese **水手袋**, Japanese **シーバッグ**. The default folder inside a project is **`.fliphopper/seabag/`** (was `.glidebox/`), which leaves `.fliphopper/` room for other FlipHopper files later. The format tag becomes **`fliphopper/1`**. No conversion of old folders. |
| 4 | App ID | **`io.fliphopper`** (the owner chose `io.crossjump` over `com.glidepace.crossjump`; the same form with the final name). |
| 5 | Old documents | Current documents change; dated history keeps "glidebox", with a note in the decision record. (Recommendation, accepted with the approval.) |
| 6 | Order | Before A01-03. (Recommendation, accepted with the approval.) |
| 7 | Backup name | CrossJumper (one p) was the backup for CrossJump. Superseded: the owner chose **FlipHopper** after the name check. |
| 8 | Where a new seabag goes | **Always `.fliphopper/seabag/`**, also when the user picks an empty folder, so agents, `fh` and the app look in one place only. This replaces today's rule that an empty folder becomes the task box itself ([FILE-FORMAT.md](../product/FILE-FORMAT.md) and [0010](../decisions/0010-project-data-folder.md) are updated to match). |

## Order of work (when the owner says start)

1. **Name check, first of all.** Done 2026-10-06; the owner chose FlipHopper over CrossJump (see [Name check](#name-check-2026-10-06-agent) and [Name decision](#name-decision)).
2. **GitHub repository.** Rename `mypals-ml/glidebox` → `mypals-ml/fliphopper`, then point `origin` at the new address and confirm a push lands.
3. **Local folder.** Rename `/Users/wanghui/coding/glidepace/glidebox` → `…/fliphopper`. Every session, worktree and the agents' saved memory depend on this path, so: the session commits and pushes everything, stops all its processes, and ends. The owner moves the folder (or lets the agent do it as that session's last action), then starts a **new session in the new folder** with the prompt the agent gives. The agents' memory folder is copied to the new path's name in the same step.
4. **The rename in the repository**: the decision record, code, data folder, interface texts in three languages, documents and agent guides (the table above).
5. **Pitch decks**: update the published decks ([`Task-A00-16`](./Task-A00-16-road-show-deck.md)) to the new name.
6. **The owner's check** of the built app.

## Details for the worker

- **Outcome.** Everything the owner, users and agents see says FlipHopper; the code and the files on disk use `fliphopper`; nothing says glidebox except dated history.
- **Decisive unknowns** (the worker finds out and records on the card):
  - **Is the name free to use?** Answered by the name check below.
  - **Every place the old name hides.** Search for `glidebox`, `Glidebox`, `GLIDEBOX`, `.glidebox` and `gb` across the repository, including i18n texts and test data.
- **Files owned.** Effectively the whole repository (a rename touches most files). **No other card may be in progress while this runs**: it is claimed when no other session holds a card, and other sessions pull after it lands.
- **External operations** (approved by the owner 2026-10-06; carried out in the order above, only after the owner says start): rename the GitHub repository and update `origin`; rename the local folder; update the published pitch decks.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - A search for `glidebox` (any case) finds only dated history and the decision record's note, listed on the card.
  - The built app is named FlipHopper, creates a `.fliphopper/seabag/` folder with format `fliphopper/1`, says "seabag" (水手袋, シーバッグ) where it said "task box", and shows FlipHopper in all three languages. The owner checks it.
  - All document links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | Card approved with the answers above (given for CrossJump; the final name FlipHopper replaces it in each), including the external operations (answers 7 and 8 confirmed the same day): rename the GitHub repository and update `origin`, rename the local folder, update the published pitch decks. **Work starts only when the owner says start**, beginning with the name check. | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
- 2026-10-06 — Owner answered and approved; "seabag" chosen for the data (over belongings, bindle, kitbag, dunnage).
- 2026-10-06 — Claimed; step 1 (name check) done. The owner compared names and chose **FlipHopper** (see Name decision).

## Name check (2026-10-06, agent)

**In plain words:** the name is free where it matters most for us: no software package, no developer tool, and no registered trademark turned up. "CrossJump" is used by a few **game makers**, so app-store and web searches for the word will show games next to ours. "CrossJumper" turned up nothing at all.

| Where | CrossJump | CrossJumper |
|---|---|---|
| crates.io (Rust packages) | free (`crossjump`, `crossjump-core`) | free |
| npm (JavaScript packages) | free | free |
| GitHub account name | taken: user `Crossjump`, created 2024, 1 empty-looking repository, inactive since 2024-08. Only matters if we want `github.com/crossjump`; the repository would be `mypals-ml/crossjump`. | free |
| `.io` domain (matches app ID `io.crossjump`) | **`crossjump.io` not registered** | `crossjumper.io` not registered |
| `.com` domain | `crossjump.com` registered since 2006, parked for sale at HugeDomains | `crossjumper.com` not registered |
| Products and companies | **CrossJump Studio**, a mobile-game studio in Lahore founded 2022, with games on the App Store and Google Play and the site `crossjump.studio`; **CrossJump Games**, a Roblox group (about 124,000 members); **Crossjump**, a small horror game on itch.io. All games; none is a productivity or developer tool. | nothing found |
| Registered trademarks (web search of USPTO / Justia listings) | none found | none found |
| Other meaning | "cross-jumping", a compiler optimization (GCC `-fcrossjumping`) | — |

**Limits of this check:** this is a web and registry search, not a legal trademark search. Trademark databases were only reached through search engines, and only US results were available. Before selling under the name, a proper search in the owner's markets (Japan, China, US) is advised.

Sources: [CrossJump Studio on the App Store](https://apps.apple.com/lr/developer/crossjump-studio/id1681824296), [on Google Play](https://play.google.com/store/apps/developer?id=CROSSJUMP+STUDIO&hl=en&gl=US), [crossjump.studio](https://crossjump.studio/), [LinkedIn](https://pk.linkedin.com/company/cross-jump), [CrossJump Games on Roblox](https://www.roblox.com/communities/51713578/CrossJump-Games#!/about), [Crossjump on itch.io](https://jackthecatgaming.itch.io/crossjump); crates.io, npm and GitHub APIs, `whois` and `dig` run by the agent.

## Name decision

**In plain words:** after the name check the owner compared other names and chose **FlipHopper** (2026-10-06). It keeps "hopper" (one who hops from place to place) and adds a front word for 横跳 / 反复横跳.

| Name | Why not chosen |
|---|---|
| CrossJump | used by three game makers; `.com` parked for sale |
| CrossJumper | "Cross Jumper" is a college baseball player's name and his X and Instagram handles |
| CrossHopper | reads two ways (cross-hopper / cros-*shopper*, a retail term); sounds like Grasshopper; near the Hopper and Grasshopper trademarks |
| ToggleHopper, ZigHopper, ShuttleHopper | free, but further from 反复横跳 (ShuttleHopper's `.com` is taken) |
| SideHopper | literal 横跳, but a well-known enemy in Nintendo's Metroid games |
| **FlipHopper** (chosen) | "flip-flop" is the English idiom for 反复横跳 |

**FlipHopper check (agent, 2026-10-06):** crates.io, npm and GitHub account names free; `fliphopper.io` and `fliphopper.com` not registered; a web search found no product, company or trademark. The command short name `fh` is not taken on the owner's Mac. Same limits as above: not a legal trademark search.

Sources: [Sidehopper, Metroid Wiki](https://metroid.fandom.com/wiki/Sidehopper), [Cross Hopper Road](https://www.amazon.com/Mkstudio-Cross-Hopper-Road/dp/B01G29AUL6), [Hopper Inc. trademarks](https://trademarks.justia.com/owners/hopper-inc-2594110/), [Mainsoft (GRASSHOPPER) trademarks](https://trademarks.justia.com/owners/mainsoft-corporation-2668817), [Cross Jumper, X](https://x.com/crossjumper?lang=en), [Flip-flop, Wikipedia](https://en.wikipedia.org/wiki/Flip-flop).
