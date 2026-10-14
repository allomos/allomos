# Task-A07-03-windows-works

*Card type: coding. Area A07 (Platform & release), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2 ([PLAN § 9](../product/PLAN.md#9-roadmap--releases): "fixes for Windows differences (paths, line endings, the WebView2 engine)"; [R81](../product/REQUIREMENTS.md)). Needs a Windows computer to test on: since 2026-10-11 [`A07-02`](./Task-A07-02-build-pipeline.md) only builds Windows on the Mac (no GitHub minutes on the Free plan), so this card adds a GitHub job that runs only when started by hand.*

## What gets built (plain words)

Allomos was written and tested only on macOS. Windows differs in ways that commonly break such apps: folder paths use `\`, Git may turn line endings into `\r\n`, files cannot always be replaced while another program reads them, and the window's web engine (WebView2) may be missing. After this card, everything Allomos does on a Mac also works on Windows: the app, `alm`, the agents panel and opening files and links.

| Piece | What it is | Where |
|---|---|---|
| 2026-10-14 | Add the manual (`workflow_dispatch`) Windows-only GitHub Actions workflow to `mypals-ml/allomos`, builds kept 14 days; cleanup: delete the workflow file and stored builds. Repository is now public | owner (chat) |
| Paths | Taskbag paths shown and stored correctly with `\`; the project folder found from a taskbag path on both systems. | `crates/store`, `apps/web` |
| Line endings | Files read correctly whether Git checked them out with `\n` or `\r\n`; Allomos always writes `\n`. A `.gitattributes` rule for taskbag files. | `crates/store`, `.allomos/taskbag` template |
| Writing files | Replacing a file while the app reads it does not fail on Windows (retry briefly, as the lock rules allow). | `crates/store` |
| WebView2 | The installer makes sure the web engine is there. | `apps/desktop/src-tauri/tauri.conf.json` |
| Opening things | "Show in Explorer", opening a card link, finding Claude Code and Codex on Windows, `alm` installed as `alm.exe`. | `apps/desktop/src-tauri`, `crates/cli` |
| Tests on Windows | The whole test suite runs on the pipeline's Windows job, plus new Windows-specific tests. | `.github/workflows/build.yml` |

**What you can see at the end:** the pipeline's Windows job passes every test, and the Windows build opens Allomos's own repository with the same list, iterations and task pane as on the Mac.

## Details for the worker

- **Outcome.** A Windows user can use every v0.2 feature.
- **Decisive unknowns.**
  - Which of the differences above actually break: found by running the suite on the Windows job first, then fixing what fails.
  - How the agent sees the Windows app working without a Windows computer: the pipeline's tests, plus the workmate's check in [`A07-04`](./Task-A07-04-v0-2-exit-check.md).
- **Files owned.** `crates/store`, `crates/cli`, `apps/desktop/src-tauri` (`lib.rs`, `agents.rs`, `tauri.conf.json`), `apps/web/src/App.tsx` (path handling), `.github/workflows/build.yml` (test steps), tests, `docs/product/FILE-FORMAT.md` (line endings), this card.
- **External operations (need the owner's exact approval).** Add a GitHub Actions workflow to `mypals-ml/allomos` that runs only when started by hand (`workflow_dispatch`), Windows job only; keep its builds 14 days. The repository is public (owner, 2026-10-14), so GitHub's standard runners cost no minutes. Cleanup: delete the workflow file and stored builds.
- **Done means mechanically.**
  - The six named checks pass on macOS and on the Windows job.
  - New tests: a taskbag checked out with `\r\n` loads; a path with `\` finds its project; replacing a file that is open for reading succeeds.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Add and run `.github/workflows/windows-tests.yml` on mypals-ml/allomos (manual start, Windows only, builds kept 14 days); the owner reports the Windows version works by hand | owner (chat, relayed by the owner's other session and confirmed in the coordinating session's chat) |
| 2026-10-14 | Approve the card (state `ready`). The manual Windows GitHub workflow still needs its own exact approval before it is added | owner (chat) |
| 2026-10-09 | Write the card (not start it) | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A07-02.
- 2026-10-14 — Owner approved the card. A07-02 is already done, so a session can claim it now.
- 2026-10-14 — Local work done on the Mac by `claude/a07-03-windows`; **the Windows job has not run yet**: it waits on the owner's exact approval to add the workflow. What changed, in plain words:
  - **Line endings.** Allomos now reads every taskbag file (JSON, descriptions, handoffs, the template, files given to `alm --from`/`--set`/`--plan`) the same whether Git wrote `\r\n` or `\n`, and always writes `\n` itself. A new taskbag gets a `.gitattributes` (`*.json`, `*.md` with `text eol=lf`); [FILE-FORMAT](../product/FILE-FORMAT.md#writing-rules-how-writers-take-turns) says so. A handoff checked out with `\r\n` used to lose its header (writer, tasks); it is read correctly now.
  - **Paths.** The taskbag's path is built part by part (`default_root`), so Windows shows `C:\p\.allomos\taskbag` instead of `C:\p\.allomos/taskbag`. A relative project folder is stored with `/` (`../app`) and read with either separator, so a taskbag works on both systems; the taskbag's place written into `AGENTS.md` uses `/` too. The interface's `projectFolder` moved to `apps/web/src/paths.ts` so it can be tested.
  - **Replacing files.** Renaming the new file over the old one is tried again for up to one second when Windows says the file is in use (another program has it open), while the lock is still held (well within the 5 seconds another writer waits).
  - **WebView2.** The installer carries Microsoft's small WebView2 bootstrapper (`embedBootstrapper`, silent, about 1.8 MB) and runs it only when the engine is missing. Chosen over the default `downloadBootstrapper` because the check then never depends on first fetching the bootstrapper itself; the offline installer (about 127 MB) is too large for what nearly every Windows 10/11 computer already has.
  - **Opening things.** Explorer gets the path with `\` throughout and without the `\\?\` prefix that resolving a card link adds (Explorer does not open such paths). `alm confirm` on Windows now opens the app installed beside `alm.exe` (`Allomos.exe`, else `allomos-desktop.exe`). Finding Claude Code and Codex (`.exe`, `.cmd`, `.bat` in PATH, `%APPDATA%\npm`, pnpm, WinGet, `~/.local/bin`) and opening them were reviewed and left as they are (Task-A07-05).
  - **Only Windows can confirm:** that Explorer opens the folder and card files; that a rename retry is enough when an editor or virus scanner holds a file; that the installer installs WebView2 where it is missing; the app's exact `.exe` name for `alm confirm`; finding and opening Claude Code and Codex; and the whole suite on a `\r\n` checkout (the repository itself has no root `.gitattributes`, so tests that compare files byte for byte may show up there).
  - **Evidence (macOS, 2026-10-14):** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (133 tests; new: `a_taskbag_checked_out_with_crlf_loads_and_is_written_back_with_lf`, `a_path_with_backslashes_finds_its_project`, `replacing_a_file_that_is_open_for_reading_succeeds`, `replacing_retries_while_the_file_is_in_use_then_gives_up`, `windows_paths_are_given_to_explorer_plainly`, `a_new_taskbag_keeps_lf_line_endings_in_git`); `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS; `test` PASS (37 tests, new `tests/paths.test.ts`). Windows job: NOT RUN. The workflow is drafted at `~/allomos-tests/a07-03-windows/windows-tests.yml` (manual `workflow_dispatch`, Windows only, the named checks, installer and logs kept 14 days), ready to become `.github/workflows/windows-tests.yml` once approved. Next: the owner approves adding it; then run it and fix what fails.
- 2026-10-14 — Owner approved adding and running the Windows workflow. Branch brought up to date with main (A01-32's guided migration kept; `\r\n` reading re-applied in `set_format` and history events). Workflow added as `.github/workflows/windows-tests.yml`. Next: merge, start the run on GitHub, finish at the top label if it passes.
