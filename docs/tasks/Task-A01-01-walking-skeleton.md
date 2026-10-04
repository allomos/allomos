# Task-A01-01-walking-skeleton

*First card of **Iteration 1 — v0.1 macOS local tracker** ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). This is a **coding** card: it produces runnable software.*

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| Core rules (Rust) | What a project, an area and a task are, and the checks that say whether their files are valid | `crates/core` |
| Task box reader (Rust) | Opens a task box folder, reads its files, or creates a small starter project in an empty folder | `crates/store` |
| Desktop app | The macOS app you start; a Tauri 2 window that hosts the interface and talks to the Rust code | `apps/desktop` |
| Interface | A folder picker, and a list of the task box's tasks grouped by area, showing each task's state | `apps/web` |
| File format v1 | A short contract describing the files inside `.glidebox/`, so agents and later code write them correctly | `docs/product/FILE-FORMAT.md` |
| Coding rules v1 | A short contract for how code in this repository is written and tested | `docs/workflow/CODING_RULES.md` |
| Tests | Automatic checks of the rules and the reader; must pass before the card is done | inside each piece |

**What you can do at the end:** start glidebox on your Mac, pick a folder, and see its tasks grouped by area. It is view-only: creating and editing tasks come later (the `gb` command next, in-app editing in Iteration 2).

## Details for the worker

- **Outcome.** The owner installs a development build of glidebox on **macOS**, picks a task box (a new standalone folder, or a subfolder of an existing Git repository), and sees the tasks stored there, grouped by area, with each task's state. If the folder is empty, glidebox creates a small starter project in it. This is the thinnest end-to-end version: the interface, the Rust rules and the files all connected.
- **Decisive unknowns** (the worker finds out):
  - The first, minimal file layout for a project. Use the default folder name `.glidebox/` inside a repository (owner, 2026-10-04). It follows [0007](../decisions/0007-git-like-project-sync.md): `project.json`, `areas/<id>.json`, `tasks/<id>/task.json` and `description.md`. The worker writes it down as the first version of the file contract.
  - How the TypeScript interface talks to the Rust side inside the Tauri window, so that the same interface code can later talk to the desktop's local server from a browser (proposal in [0011](../decisions/0011-desktop-framework-and-shared-code.md)).
  - The pinned versions of the toolchain: Rust, Tauri 2.x, Node and the web build tools.
- **Files owned.**
  - `Cargo.toml` (workspace), `rust-toolchain.toml`;
  - `crates/core/**` (minimal: project, area and task types, and reading their validation rules);
  - `crates/store/**` (minimal: open, create and read a task box);
  - `apps/desktop/**` (Tauri 2 app);
  - `apps/web/**` (interface: folder picker, task list grouped by area);
  - `docs/product/FILE-FORMAT.md` (the first file contract);
  - `docs/workflow/CODING_RULES.md` (the first coding rules);
  - this card.
- **External operations.** None. Installing toolchains and packages locally is local work. No signing, publishing or hosted services.
- **Done means mechanically.**
  - The app builds and starts on **macOS** (development build, unsigned). Windows is Iteration 2.
  - Choosing an empty folder creates the starter project; choosing a subfolder of a Git repository works and creates no nested repository.
  - The task list shows the starter project's areas and tasks.
  - Invalid files show a clear error naming the file and field, instead of crashing.
  - These pass: `cargo test`, `cargo clippy`, `cargo fmt --check`, plus the interface's build and type check.
  - The owner sees it running.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-04 | Card approved: all local work (source, tests, docs, commits, pushes; installing toolchains and packages locally). No external operations. | owner (chat) |

## Notes

- 2026-10-04 — **Built.** Rust workspace (`crates/core`, `crates/store`); Tauri 2.11.6 desktop app; React interface talking to Rust only through `apps/web/src/messenger.ts` (point 1 of [PLAN § 8](../product/PLAN.md#at-a-glance-updated-2026-10-04)). Added start-up folder support (`glidebox <folder>` or `GLIDEBOX_OPEN`). Wrote [FILE-FORMAT.md](../product/FILE-FORMAT.md) and [CODING_RULES.md](../workflow/CODING_RULES.md).
  - **Versions:** Rust 1.85.1 is what is installed (Homebrew). Tauri 2.12 needs Rust 1.90, so Tauri is pinned to 2.11.x and the JavaScript packages match it (`@tauri-apps/api` 2.11.1, `plugin-dialog` 2.7.3, CLI 2.11.5).
  - **Checks PASS:** `cargo fmt --check`; `cargo clippy --workspace --all-targets -D warnings`; `cargo test --workspace` (13 tests: core 7, store 6); web `typecheck` and `build`; `tauri build --debug --bundles app`.
  - **Real entry path:** the app was started on an empty folder. The interface called Rust, which created the starter project files (`project.json`, `areas/`, `tasks/`).
  - **NOT RUN by the agent:** looking at the window. The owner declined screen access, so the visual check is the owner's.
  - **Bug found and fixed:** the starter writer wrote the description into `task.json`. The reader rejected it, which the test caught.

- Next Iteration 1 cards, written when this one is done:
  - the `glidebox` command (alias `gb`, R88) for agents. It must decide how `gb` and the running desktop app avoid writing the same task box at once (R75; [PLAN § 8 open point 2](../product/PLAN.md#at-a-glance-updated-2026-10-04)). Commands: `gb init` to create the task box; add, list, show, claim, label, wait, done, release);
  - moving glidebox's own cards from Markdown into a task box in this repository (dogfooding, decided 2026-10-04), and switching the agent workflow to `gb`;
  - the Iteration 1 exit check on macOS. Windows, and creating and editing tasks in the app, follow in Iteration 2.
