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
| 2026-10-14 | Approve the card (state `ready`). The manual Windows GitHub workflow still needs its own exact approval before it is added | owner (chat) |
| 2026-10-09 | Write the card (not start it) | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card (`ready`); it starts after A07-02.
- 2026-10-14 — Owner approved the card. A07-02 is already done, so a session can claim it now.
