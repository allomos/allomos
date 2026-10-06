# Task-A00-17-rename-to-crossjump

*Card type: decision + coding + docs. Area A00 (Planning). Requested by the owner, 2026-10-06.*

## Why

The owner wants to rename the product from **Glidebox** to **CrossJump**. The name carries the idea of the product: a person or a project can **jump between AI agents** (Claude Code, Codex, …), and a project can be **handed between people**. "Glidebox" described a box of tasks; "CrossJump" describes the movement across agents and people that the product is for.

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| Decision record | Records the new name, why, and the answers below. | `docs/decisions/0013-product-name-crossjump.md` (new) |
| The app | Window title, app name (`CrossJump.app`), app ID, and every interface text in English, Chinese and Japanese. | `apps/desktop/src-tauri` (`tauri.conf.json`, crate names), `apps/web` (texts, storage keys) |
| The code | Rust crate and library names (`glidebox-core` → `crossjump-core`, …), the start-up variable `GLIDEBOX_OPEN`. | `Cargo.toml`, `crates/*`, `apps/desktop/src-tauri`, `Cargo.lock` |
| The data on disk | The project folder name and the format tag written into `project.json`. | `crates/core` (`FORMAT`), `crates/store` (`DEFAULT_DIR`), [FILE-FORMAT.md](../product/FILE-FORMAT.md) |
| The command | R88's command name and short alias, before [`Task-A01-03`](./Task-A01-03-gb-command.md) builds it. | [REQUIREMENTS.md](../product/REQUIREMENTS.md) R88, the A01-03 card |
| Documents | Every current document and the agent guides use the new name. | `AGENTS.md`, `CLAUDE.md`, `README.md`, `docs/**` |
| Outside this repository (needs approval) | The GitHub repository name, the local folder path, and the published pitch decks. | see External operations |

**What you can see at the end:** an app called CrossJump that opens a CrossJump project folder, documents and agent guides that say CrossJump, and the `crossjump` command name ready for A01-03.

## Questions for the owner

Each has a recommendation; the decision record keeps the answers.

1. **Spelling.** Product name **CrossJump** in text; **`crossjump`** in lower case wherever code needs one word (crates, folder, format tag, command). *Recommended.*
2. **Short command name.** R88 says `glidebox` with `gb`. Proposal: **`crossjump`** with **`cj`**.
3. **Data folder and format tag.** `.glidebox/` → **`.crossjump/`**, and `glidebox/1` → **`crossjump/1`**. No released version has written real data yet, so no conversion of old folders is built. The owner's own test folders are recreated. *Recommended.*
4. **App ID.** `com.glidepace.glidebox` → **`com.glidepace.crossjump`** (Glidepace stays the maker's name). The app's saved settings (language choice, last folder) start empty once. *Recommended.*
5. **Old documents.** Current documents change to CrossJump. Dated records (discussions, evidence in finished cards, quotes) keep "glidebox" where it is history, with one line in the decision record explaining the old name. *Recommended.*
6. **Order.** Do this card **before A01-03**, so the command is built once under its final name. *Recommended.*

## Details for the worker

- **Outcome.** Everything the owner, users and agents see says CrossJump; the code and the files on disk use `crossjump`; nothing says glidebox except dated history.
- **Decisive unknowns** (the worker finds out and records on the card):
  - **Is the name free to use?** Search for existing products, apps and trademarks named CrossJump, and whether `crossjump` is taken on crates.io and npm. Note: "cross-jumping" is also the name of a compiler optimization (GCC's `-fcrossjumping`). That is unlikely to matter for a product name, but searches will find it. Report what was found; the owner decides.
  - **Every place the old name hides.** Search for `glidebox`, `Glidebox`, `GLIDEBOX`, `.glidebox` and `gb` across the repository, including i18n texts and test data.
- **Files owned.** Effectively the whole repository (a rename touches most files). **No other card may be in progress while this runs**: it is claimed when no other session holds a card, and other sessions pull after it lands.
- **External operations** (each needs the owner's exact approval, recorded on this card):
  - Rename the GitHub repository `mypals-ml/glidebox` → `mypals-ml/crossjump`, then update `origin` locally. GitHub redirects the old address, but the owner may prefer to do the rename in GitHub's settings themselves.
  - Rename the local folder `/Users/wanghui/coding/glidepace/glidebox` → `…/crossjump`. Done by the owner with no session running, because every session, worktree path and the agents' saved memory depend on it.
  - Update or replace the published pitch decks ([`Task-A00-16`](./Task-A00-16-road-show-deck.md)), or leave them as they are.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - A search for `glidebox` (any case) finds only dated history and the decision record's note, listed on the card.
  - The built app is named CrossJump, creates a `.crossjump/` folder with format `crossjump/1`, and shows CrossJump in all three languages. The owner checks it.
  - All document links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
