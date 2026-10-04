# Task-W04-01-walking-skeleton

*First card of release **v0.1 — Local tracker** ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)).*

- **Outcome.** The owner installs a development build of glidebox on macOS or Windows, picks a data folder (a new standalone folder, or a subfolder of an existing Git repository), and sees the tasks stored there, grouped by workstream, with each task's state. If the folder is empty, glidebox creates a small starter project in it. This is the thinnest end-to-end version: the interface, the Rust rules and the files all connected.
- **Decisive unknowns** (the worker finds out):
  - The first, minimal file layout for a project. Use the proposed default folder name `.glidebox/` inside a repository ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)), unless the owner chooses another. It follows [0007](../decisions/0007-git-like-project-sync.md): `project.json`, `workstreams/<id>.json`, `tasks/<id>/task.json` and `description.md`. The worker writes it down as the first version of the file contract.
  - How the TypeScript interface talks to the Rust side inside the Tauri window, so that the same interface code can later talk to the desktop's local server from a browser (proposal in [0011](../decisions/0011-desktop-framework-and-shared-code.md)).
  - The pinned versions of the toolchain: Rust, Tauri 2.x, Node and the web build tools.
- **Files owned.**
  - `Cargo.toml` (workspace), `rust-toolchain.toml`;
  - `crates/core/**` (minimal: project, workstream and task types, and reading their validation rules);
  - `crates/store/**` (minimal: open, create and read a data folder);
  - `apps/desktop/**` (Tauri 2 app);
  - `apps/web/**` (interface: folder picker, task list grouped by workstream);
  - `docs/product/FILE-FORMAT.md` (the first file contract);
  - `docs/workflow/CODING_RULES.md` (the first coding rules);
  - this card.
- **External operations.** None. Installing toolchains and packages locally is local work. No signing, publishing or hosted services.
- **Done means mechanically.**
  - The app builds and starts on **macOS** and on **Windows**: a development build, unsigned. If no Windows machine is available, this is recorded as **NOT RUN** and the label stops at "locally verified on macOS".
  - Choosing an empty folder creates the starter project; choosing a subfolder of a Git repository works and creates no nested repository.
  - The task list shows the starter project's workstreams and tasks.
  - Invalid files show a clear error naming the file and field, instead of crashing.
  - These pass: `cargo test`, `cargo clippy`, `cargo fmt --check`, plus the interface's build and type check.
  - The owner sees it running.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- Next v0.1 cards, written when this one is done:
  - creating and editing tasks in the app, with every change recorded as an event;
  - the `gb` command for agents (`gb init` to create the tasks folder; add, list, show, claim, label, wait, done, release);
  - moving glidebox's own cards from Markdown into a data folder in this repository (dogfooding, decided 2026-10-04), and switching the agent workflow to `gb`;
  - the v0.1 exit check on both systems.
