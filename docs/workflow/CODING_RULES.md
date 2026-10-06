# Coding rules

A **contract** for code in this repository. Created by [`Task-A01-01`](../tasks/Task-A01-01-walking-skeleton.md). Extend it in the card that needs a new rule.

## Layout

| Where | What | Language |
|---|---|---|
| `crates/core` | Project rules: model and validation. **No disk, network or UI code.** | Rust |
| `crates/store` | Reading and writing task boxes on disk. Never runs `git` by itself. | Rust |
| `apps/desktop/src-tauri` | The Tauri desktop app: thin commands that call the crates. | Rust |
| `apps/web` | The interface. Talks to Rust **only through `src/messenger.ts`**. | TypeScript + React |

Project rules live in Rust. The interface displays results and sends requests; it never decides whether a change is valid.

## Versions

- **Rust 1.85.1**, pinned in `rust-toolchain.toml`. **Tauri 2.11.x**: Tauri 2.12 needs Rust 1.90, so moving up is its own card.
- Cargo uses resolver 3 with `rust-version = "1.85"`, so dependencies are chosen to fit that Rust. `Cargo.lock` is committed.
- JavaScript packages are pinned to exact versions. `@tauri-apps/api` must match the Tauri crate's minor version (2.11), and plugin packages must match their crates. npm is used; there is no pnpm.
- Never upgrade a dependency on the side. Upgrade it on purpose, in a card, with its licence noted.

## Checks before a commit

From the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
npm --prefix apps/web run check:i18n
```

All must pass. Report anything not run as NOT RUN.

## Writing code

- **Problems are values, not crashes.** Reading a task box returns what could be read plus a list of problems. Each problem names the file and field, and explains it in plain words: what is wrong and how to fix it.
- **Never overwrite user content.** Creating a task box into a non-empty folder is refused. Writing files keeps the format in [FILE-FORMAT.md](../product/FILE-FORMAT.md).
- **Tests use temporary folders and synthetic data only**, never the owner's real projects.
- **Keep interface text short and plain**, and keep its terms consistent with the [glossary](../GLOSSARY.md): task box, area, state, label.

## Interface languages

The interface speaks English, Simplified Chinese and Japanese (R94). English is the reference.

- **No interface text in the code.** Every text the interface shows is a key in `apps/web/src/i18n/en.json`, `zh-CN.json` and `ja.json`, shown with `t()` (or `rich()` when a placeholder holds an element). The user's own data, file paths and field names are not translated.
- **Write English first, and add Chinese and Japanese in the same edit.** Keep each text short and plain, and keep the same `{placeholders}` in every language.
- **Rust sends codes, not sentences.** A problem is a `Problem` in `crates/core/src/problem.rs`: a code, its details, and one English wording for logs and `gb`. After adding or changing a code, run `GLIDEBOX_WRITE_CODES=1 cargo test -p glidebox-core` to update `apps/web/src/i18n/codes.json`, then add `problem.<code>` to every language file.
- **The checks catch slips:** `typecheck` fails when a language file misses a key; `check:i18n` also fails on extra or empty texts, different placeholders, and Rust codes without words; `cargo test` fails when `codes.json` is out of date.
- The computer's language is read in Rust (`sys-locale` 0.3.2, MIT or Apache-2.0), so it does not depend on what the macOS window reports. The user's own choice is kept in the window's storage.
- **Formatting:** `cargo fmt` defaults for Rust; two-space indentation and double quotes for TypeScript.

## Build output (`target/`)

Rust puts everything it builds in `target/`, one per worktree. A Tauri app's `target/` reaches several gigabytes, so these rules keep disk use to about one `target/` per active card (owner, 2026-10-06).

- **Development builds keep line numbers only.** The workspace `Cargo.toml` sets `[profile.dev] debug = "line-tables-only"`: crash reports and test failures still name the file and line, but the full information a debugger needs is not kept. When you really need a debugger (`lldb`), turn it on for that one build only, `CARGO_PROFILE_DEV_DEBUG=true cargo build`, and do not commit a change to the setting.
- **One `target/` per worktree.** Do not set a shared `CARGO_TARGET_DIR`: parallel sessions would wait on each other's builds and keep rebuilding over each other's changes.
- **No routine `cargo clean`.** It throws away work that the next build must redo. Run it only in your own worktree, and only when builds behave strangely after a Rust or Tauri upgrade, or when the disk is short of space.
- **Remove a worktree when its card is merged.** This deletes its `target/` with it. See [CARDS.md § Claiming protocol](./CARDS.md#claiming-protocol).
- Do not build in the main checkout; build in your card's worktree.

## Running the app

```bash
npm --prefix apps/web install
npm --prefix apps/desktop install
npm --prefix apps/desktop run dev
```

`npm --prefix apps/desktop run dev` starts the interface and the desktop window together and reloads on changes. To open a folder directly, pass it at start-up: `GLIDEBOX_OPEN=/path/to/folder npm --prefix apps/desktop run dev`, or `open Glidebox.app --args /path/to/folder` for a built app.

To build a local, unsigned app: `npx --prefix apps/desktop tauri build --debug --bundles app`. The result is `target/debug/bundle/macos/Glidebox.app`.
