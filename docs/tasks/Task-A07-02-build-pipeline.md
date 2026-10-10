# Task-A07-02-build-pipeline

*Card type: coding. Area A07 (Platform & release), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2 ([PLAN § 9](../product/PLAN.md#9-roadmap--releases): "one build pipeline producing both systems' builds"; [R85](../product/REQUIREMENTS.md): macOS and Windows released together, unsigned until v1.0).*

## What gets built (plain words)

Today the app and `alm` are built by hand on the owner's Mac, and nothing is ever built or tested on Windows. After this card, one **build pipeline** (an automatic job on GitHub) builds Allomos for **both** macOS and Windows from the same source whenever a branch is pushed: it runs all the checks on both systems and keeps the unsigned builds for download.

| Piece | What it is | Where |
|---|---|---|
| The pipeline | A GitHub Actions workflow with one macOS and one Windows job. Each runs the six named checks, then builds the app and `alm`. | `.github/workflows/build.yml` |
| macOS build | The unsigned app (`Allomos.app` in a `.dmg`) and `alm`. | download from the pipeline's run |
| Windows build | The unsigned installer (`.msi` or `.exe`) and `alm.exe`. Windows will warn that the publisher is unknown until v1.0. | download from the pipeline's run |
| How to get a build | A short section: where to download, how long builds are kept, the expected security warnings. | `docs/workflow/CODING_RULES.md` |

**What you can see at the end:** push a branch; on GitHub, the run shows both systems' checks passing and offers a macOS and a Windows download.

## Details for the worker

- **Outcome.** Every change is checked on Windows as well as macOS, and a Windows build exists without a Windows computer.
- **Decisive unknowns.**
  - Whether this repository's GitHub plan covers the pipeline's minutes (macOS minutes cost more): the owner checks the plan, or the worker reads it from GitHub's settings with permission.
  - Installer type for Windows (`.msi` or NSIS `.exe`): the worker proposes one; recommended NSIS, which does not need administrator rights.
  - How long builds are kept (GitHub default 90 days; proposed 14).
- **Files owned.** `.github/workflows/build.yml` (new), `apps/desktop/src-tauri/tauri.conf.json` (Windows bundle settings), `docs/workflow/CODING_RULES.md`, this card.
- **External operations (need the owner's exact approval).**
  - Target: GitHub repository `mypals-ml/fliphopper`, GitHub Actions.
  - Operations: add the workflow (it then runs on GitHub's computers on every push), store build files on GitHub for the chosen number of days.
  - Cleanup: delete the workflow file and the stored builds.
  - No secrets, no signing, no publishing to a public download page.
- **Done means mechanically.**
  - A pushed branch runs green on both systems; both builds can be downloaded.
  - A deliberately failing test on Windows only makes the Windows job fail (proves Windows really runs the tests).
  - `git diff --check`; the docs' links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |

The external operations above are **not yet approved**.

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card and its external operations.
