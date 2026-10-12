# Task-A07-11-github-builds-releases

*Card type: coding. Area A07 (Platform & release). Written 2026-10-13 at the owner's request. **Not started.***

## What gets built (plain words)

Today a GitHub release of Allomos (`v0.1.0`, `v0.1.1`) is only a name on the code's history: no files to download. The installers are built only on the owner's Mac ([`A07-02`](./Task-A07-02-build-pipeline.md)), because the repository was private then and GitHub's build time was limited. The repository is public now, and GitHub does not charge build time for public repositories.

| Piece | What it is in everyday words | Where |
|---|---|---|
| Release workflow | When a new version tag such as `v0.2.0` is pushed, GitHub builds Allomos on its own Mac and Windows computers and attaches the results to that version's release page | `.github/workflows/release.yml` |
| macOS download | `Allomos_<version>_aarch64.dmg` (Apple Silicon), unsigned; Intel Macs: worker checks whether a universal build is wanted | release page |
| Windows download | `Allomos_<version>_x64-setup.exe`, unsigned, with `alm` inside (as the Mac script builds it today) | release page |
| `alm` downloads | `alm` for macOS and `alm.exe` for Windows, for people who use only the command | release page |
| Guide | "Get the app" links to the newest downloads and says how to open an unsigned app (macOS: right-click › Open; Windows: More info › Run anyway) | `docs/guide/` (three languages) |

At the end the owner pushes a tag, and a few minutes later the release page has the files. The app's **Download the newest Allomos** button ([`A01-25`](./Task-A01-25-taskbag-format-upgrade.md)) then leads somewhere useful.

## Details for the worker

- **Outcome.** Anyone can download and install the newest Allomos from its GitHub release page, without building it.
- **Decisive unknowns.**
  - Whether builds start on a pushed tag only, or also by hand (`workflow_dispatch`) — owner; assume both.
  - Whether `v0.1.0` and `v0.1.1` get files too (built from their tags) — owner.
  - Apple Silicon only, or also Intel (a universal `.dmg`) — owner.
  - Whether the Mac build script (`build-windows.sh`) stays as a fallback — owner; assume yes.
- **Files owned.** `.github/workflows/release.yml`; `docs/guide/src/content/docs/**/getting-started.md`; the "Building for Windows" section of `docs/workflow/CODING_RULES.md`; `apps/desktop/src-tauri/tauri.conf.json` only if the bundle targets must change (`dmg`).
- **External operations** (each needs the owner's exact approval below):
  - Merging the workflow into `main`, and each workflow run that attaches files to a release (publishing downloads). Cleanup: delete the files from the release, delete the workflow.
  - Pushing a new version tag to start the first run. Cleanup: delete the tag and its release.
- **Done means mechanically.**
  - A run of the workflow succeeds on GitHub for a test tag, and the release page has the `.dmg`, the `-setup.exe` and both `alm` files.
  - The owner downloads both installers, opens Allomos on macOS and Windows, and `alm --version` works on Windows after installing.
  - The guide's links in all three languages lead to the files.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-13 — Card written after the owner asked why the `v0.1.0` release had no files, and whether GitHub can build releases.
