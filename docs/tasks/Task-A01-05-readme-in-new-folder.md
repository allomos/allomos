# Task-A01-05-readme-in-new-folder

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner while checking [`Task-A00-17`](./Task-A00-17-rename-to-crossjump.md) (2026-10-06).*

## What gets built (plain words)

When FlipHopper creates a seabag in a folder that has **no visible files or folders**, it also writes a short `README-fliphopper.md` in that folder. A person who later opens the folder in Finder or on GitHub then sees what the folder is, instead of an empty-looking folder whose content is hidden in `.fliphopper/`.

| Piece | What it is | Where |
|---|---|---|
| "Looks empty" rule | A folder looks empty when every entry's name starts with `.` (for example `.DS_Store` or `.git`) or it has no entries. Such a folder is treated like an empty one: the seabag is created straight away, without asking. Today a folder holding only `.DS_Store` counts as "has other files" and asks first. | `crates/store` (`inspect`) |
| The README | `README-fliphopper.md`, written next to `.fliphopper/`, only in a folder that looks empty, and never over an existing file. A few lines: this folder is a FlipHopper project; the tasks are in `.fliphopper/seabag/`; open the folder in the FlipHopper app (or with `fh` once it exists); the files are plain JSON and Markdown that Git can track. | `crates/store` (`create_starter` or a new function), [FILE-FORMAT.md](../product/FILE-FORMAT.md) |
| Its language | The README is written in the language the app is showing (English, Simplified Chinese or Japanese); the interface passes it to Rust. | `apps/web/src/App.tsx`, `messenger.ts`, `apps/desktop/src-tauri/src/lib.rs` |
| The notice | The "created" notice also says that `README-fliphopper.md` was added. | `apps/web/src/i18n/*.json` |

**What you can see at the end:** pick an empty folder (or one with only `.DS_Store`): the seabag is created and the folder shows a `README-fliphopper.md` that introduces FlipHopper in the app's language. Pick a code repository: nothing changes from today; no README is written.

## Details for the worker

- **Outcome.** A folder that FlipHopper fills from (visibly) empty explains itself to a person who opens it.
- **File name (owner, 2026-10-06): `README-fliphopper.md`.** It never takes the name `README.md`, so it cannot clash with a README the user adds later, and it sorts next to one. GitHub shows only `README.md` on a folder's front page, so this file appears in the list but not there.
- **Defaults chosen by the agent** (the owner may change them on review): it is written in the app's language; a folder holding only hidden entries is created without asking.
- **Decisive unknowns.** Whether a hidden-only folder that is a Git repository's root (only `.git`) should get the README straight away; recommended yes, since that is a fresh repository.
- **Files owned.** `crates/store/src/lib.rs`; `apps/desktop/src-tauri/src/lib.rs`; `apps/web/src/App.tsx`, `messenger.ts`, `types.ts`; `apps/web/src/i18n/en.json`, `zh-CN.json`, `ja.json`; `docs/product/FILE-FORMAT.md` (where the README may appear); this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - Rust tests cover: empty folder gets README and seabag; folder with only `.DS_Store` the same; a folder with a visible file gets no README; an existing `README-fliphopper.md` is never overwritten; README text for each of the three languages.
  - In the built app, the "What you can see at the end" checks pass. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | "If a folder has no visible files or dirs, then add a readme.md at that folder to introduce fliphopper briefly." | owner (chat, while checking A00-17) |
| 2026-10-06 | File name `README-fliphopper.md` | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent from the owner's request. Not started: one card per session; it follows A00-17.
