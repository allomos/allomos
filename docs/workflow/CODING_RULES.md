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
```

All must pass. Report anything not run as NOT RUN.

## Writing code

- **Problems are values, not crashes.** Reading a task box returns what could be read plus a list of problems. Each problem names the file and field, and explains it in plain words: what is wrong and how to fix it.
- **Never overwrite user content.** Creating a task box into a non-empty folder is refused. Writing files keeps the format in [FILE-FORMAT.md](../product/FILE-FORMAT.md).
- **Tests use temporary folders and synthetic data only**, never the owner's real projects.
- **Keep interface text short and plain**, and keep its terms consistent with the [glossary](../GLOSSARY.md): task box, area, state, label.
- **Formatting:** `cargo fmt` defaults for Rust; two-space indentation and double quotes for TypeScript.

## Running the app

```bash
npm --prefix apps/web install
npm --prefix apps/desktop install
npm --prefix apps/desktop run dev
```

`npm --prefix apps/desktop run dev` starts the interface and the desktop window together and reloads on changes. To open a folder directly, pass it at start-up: `GLIDEBOX_OPEN=/path/to/folder npm --prefix apps/desktop run dev`, or `open Glidebox.app --args /path/to/folder` for a built app.

To build a local, unsigned app: `npx --prefix apps/desktop tauri build --debug --bundles app`. The result is `target/debug/bundle/macos/Glidebox.app`.
