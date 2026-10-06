# Task-A00-17-rename-to-crossjump

*Card type: decision + coding + docs. Area A00 (Planning). Requested by the owner, 2026-10-06.*

## Why

The owner wants to rename the product from **Glidebox** to **CrossJump**. The name carries the idea of the product: a person or a project can **jump between AI agents** (Claude Code, Codex, …), and a project can be **handed between people**. "Glidebox" described a box of tasks; "CrossJump" describes the movement across agents and people that the product is for.

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| Decision record | Records the new name, why, and the answers below. | `docs/decisions/0013-product-name-crossjump.md` (new) |
| The app | Window title, app name (`CrossJump.app`), app ID, and every interface text in English, Chinese and Japanese. | `apps/desktop/src-tauri` (`tauri.conf.json`, crate names), `apps/web` (texts, storage keys) |
| The code | Rust crate and library names (`glidebox-core` → `crossjump-core`, …), the start-up variable `GLIDEBOX_OPEN` → `CROSSJUMP_OPEN`, code names for "task box" (`TaskBox` → `Seabag`). | `Cargo.toml`, `crates/*`, `apps/desktop/src-tauri`, `Cargo.lock` |
| The data on disk | The project folder name and the format tag written into `project.json`. | `crates/core` (`FORMAT`), `crates/store` (`DEFAULT_DIR`), [FILE-FORMAT.md](../product/FILE-FORMAT.md), [GLOSSARY.md](../GLOSSARY.md) ("task box" → "seabag") |
| The command | R88's command name and short alias, before [`Task-A01-03`](./Task-A01-03-gb-command.md) builds it. | [REQUIREMENTS.md](../product/REQUIREMENTS.md) R88, the A01-03 card |
| Documents | Every current document and the agent guides use the new name. | `AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/**` |
| Outside this repository (needs approval) | The GitHub repository name, the local folder path, and the published pitch decks. | see External operations |

**What you can see at the end:** an app called CrossJump that opens a seabag (`.crossjump/seabag/`), documents and agent guides that say CrossJump, and the `crossjump` command name ready for A01-03.

## Owner answers (2026-10-06)

| # | Question | Answer |
|---|---|---|
| 1 | Spelling | **CrossJump** in text; **`crossjump`** in lower case where code needs one word. (Recommendation, accepted with the card's approval.) |
| 2 | Command name | **`crossjump`**, short name **`cj`** (replaces R88's `glidebox` / `gb`). (Recommendation, accepted with the approval.) |
| 3 | The data | The term **"task box" becomes "seabag"**: a sailor's bag holding everything they own, carried from ship to ship, like a project's belongings carried from agent to agent and person to person. Chinese **水手袋**, Japanese **シーバッグ**. The default folder inside a project is **`.crossjump/seabag/`** (was `.glidebox/`), which leaves `.crossjump/` room for other CrossJump files later. The format tag becomes **`crossjump/1`**. No conversion of old folders. |
| 4 | App ID | **`io.crossjump`** (not `com.glidepace.crossjump`). |
| 5 | Old documents | Current documents change; dated history keeps "glidebox", with a note in the decision record. (Recommendation, accepted with the approval.) |
| 6 | Order | Before A01-03. (Recommendation, accepted with the approval.) |
| 7 | Backup name | If the name check finds CrossJump unusable: **CrossJumper** (one p; owner confirmed). |
| 8 | Where a new seabag goes | **Always `.crossjump/seabag/`**, also when the user picks an empty folder, so agents, `cj` and the app look in one place only. This replaces today's rule that an empty folder becomes the task box itself ([FILE-FORMAT.md](../product/FILE-FORMAT.md) and [0010](../decisions/0010-project-data-folder.md) are updated to match). |

## Order of work (when the owner says start)

1. **Name check, first of all.** Search for products, apps and trademarks named CrossJump (and CrossJumper), and whether `crossjump` is taken on crates.io and npm. Report to the owner; the owner confirms the name. Nothing else starts until then.
2. **GitHub repository.** Rename `mypals-ml/glidebox` → `mypals-ml/crossjump`, then point `origin` at the new address and confirm a push lands.
3. **Local folder.** Rename `/Users/wanghui/coding/glidepace/glidebox` → `…/crossjump`. Every session, worktree and the agents' saved memory depend on this path, so: the session commits and pushes everything, stops all its processes, and ends. The owner moves the folder (or lets the agent do it as that session's last action), then starts a **new session in the new folder** with the prompt the agent gives. The agents' memory folder is copied to the new path's name in the same step.
4. **The rename in the repository**: the decision record, code, data folder, interface texts in three languages, documents and agent guides (the table above).
5. **Pitch decks**: update the published decks ([`Task-A00-16`](./Task-A00-16-road-show-deck.md)) to the new name.
6. **The owner's check** of the built app.

## Details for the worker

- **Outcome.** Everything the owner, users and agents see says CrossJump; the code and the files on disk use `crossjump`; nothing says glidebox except dated history.
- **Decisive unknowns** (the worker finds out and records on the card):
  - **Is the name free to use?** Search for existing products, apps and trademarks named CrossJump, and whether `crossjump` is taken on crates.io and npm. Note: "cross-jumping" is also the name of a compiler optimization (GCC's `-fcrossjumping`). That is unlikely to matter for a product name, but searches will find it. Report what was found; the owner decides.
  - **Every place the old name hides.** Search for `glidebox`, `Glidebox`, `GLIDEBOX`, `.glidebox` and `gb` across the repository, including i18n texts and test data.
- **Files owned.** Effectively the whole repository (a rename touches most files). **No other card may be in progress while this runs**: it is claimed when no other session holds a card, and other sessions pull after it lands.
- **External operations** (approved by the owner 2026-10-06; carried out in the order above, only after the owner says start): rename the GitHub repository and update `origin`; rename the local folder; update the published pitch decks.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - A search for `glidebox` (any case) finds only dated history and the decision record's note, listed on the card.
  - The built app is named CrossJump, creates a `.crossjump/seabag/` folder with format `crossjump/1`, says "seabag" (水手袋, シーバッグ) where it said "task box", and shows CrossJump in all three languages. The owner checks it.
  - All document links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | Card approved with the answers above, including the external operations (answers 7 and 8 confirmed the same day): rename the GitHub repository and update `origin`, rename the local folder, update the published pitch decks. **Work starts only when the owner says start**, beginning with the name check. | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
- 2026-10-06 — Owner answered and approved; "seabag" chosen for the data (over belongings, bindle, kitbag, dunnage).
