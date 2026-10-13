# Task-A07-11-github-builds-releases

*Card type: coding. Area A07 (Platform & release). Written 2026-10-13 at the owner's request. **Not started.***

## What gets built (plain words)

Today a GitHub release of Allomos (`v0.1.0`, `v0.1.1`) is only a name on the code's history: no files to download. The installers are built only on the owner's Mac ([`A07-02`](./Task-A07-02-build-pipeline.md)), because the repository was private then and GitHub's build time was limited. The repository is public now, and GitHub does not charge build time for public repositories.

| Piece | What it is in everyday words | Where |
|---|---|---|
| Release workflow | When a new version tag such as `v0.2.0` is pushed, GitHub builds Allomos on its own Mac and Windows computers and attaches the results to that version's release page | `.github/workflows/release.yml` |
| macOS download | `Allomos_<version>_aarch64.dmg`, for Apple Silicon Macs only (owner, 2026-10-13), unsigned | release page |
| Windows download | `Allomos_<version>_x64-setup.exe`, unsigned, with `alm` inside (as the Mac script builds it today) | release page |
| `alm` downloads | `alm` for macOS and `alm.exe` for Windows, for people who use only the command | release page |
| Guide | "Get the app" links to the newest downloads and says how to open an unsigned app (macOS: right-click › Open; Windows: More info › Run anyway) | `docs/guide/` (three languages) |

At the end the owner pushes a tag, and a few minutes later the release page has the files. The app's **Download the newest Allomos** button ([`A01-25`](./Task-A01-25-taskbag-format-upgrade.md)) then leads somewhere useful.

## Details for the worker

- **Outcome.** Anyone can download and install the newest Allomos from its GitHub release page, without building it.
- **Decisive unknowns.**
  - Whether builds start on a pushed tag only, or also by hand (`workflow_dispatch`) — owner; assume both.
  - *Decided (owner, 2026-10-13):* `v0.1.0` and `v0.1.1` stay without files; only new versions get them.
  - *Decided (owner, 2026-10-13):* Apple Silicon Macs only; no Intel build.
  - *Decided (owner, 2026-10-13):* the Mac script `build-windows.sh` stays as a backup.
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
| 2026-10-13 | The card (local work only; merging the workflow, pushing a tag and publishing files still need their own approval) | owner |
| 2026-10-13 | Version 0.1.2; merge `claude/a07-11-release-builds` into `main`; push tag `v0.1.2`, which runs the Release workflow and publishes its four files on the `v0.1.2` release. Cleanup: delete the tag and its release | owner |

## Notes

- 2026-10-13 — Card written after the owner asked why the `v0.1.0` release had no files, and whether GitHub can build releases.
- 2026-10-13 — Owner approved the card and decided: no files for `v0.1.0` and `v0.1.1`; Apple Silicon only; keep `build-windows.sh` as a backup.
- 2026-10-13 — Built locally, in branch `claude/a07-11-release-builds`. Plain words: the GitHub build script is written, its Mac step works on the owner's Mac and makes a `.dmg` that opens correctly, and the guide's "Get the app" step now says where to download and how to open an unsigned app, in all three languages. Nothing has run on GitHub yet.
  Evidence: `CI=true npm --prefix apps/desktop run tauri build -- --bundles dmg --config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'` → `Allomos_0.1.0_aarch64.dmg` (3.7 MB); mounted: `Allomos.app` + `Applications` link, `codesign --verify --deep --strict` passes (ad hoc), arm64 only. Without `CI=true` the `.dmg` step fails on this Mac, because arranging the window needs Finder automation; GitHub sets `CI=true`. The Windows job is not tried locally (it runs only on GitHub). `npm --prefix docs/guide run build`: 10 pages, no warnings. Workflow YAML parses (jobs: release, macos, windows).
  The app's version is still `0.1.0` while tags `v0.1.0` and `v0.1.1` exist, so the first run needs a new version in `tauri.conf.json` and `Cargo.toml` with a matching tag.
- 2026-10-13 — Owner chose version 0.1.2 and approved publishing. Version set in `Cargo.toml`, `Cargo.lock`, `tauri.conf.json` and both `package.json` / `package-lock.json`; the `alm --version` test now compares with the package version instead of a fixed "0.1.0". `cargo test --workspace`: all pass; `cargo fmt --check`: pass.
- 2026-10-13 — Published. Plain words: tag `v0.1.2` made GitHub build Allomos for Mac and Windows and put the four files on https://github.com/mypals-ml/allomos/releases/tag/v0.1.2; the "latest release" link the guide uses now leads there. The Mac files were downloaded back and work; the Windows installer still needs a check on a Windows computer.
  Evidence: Release run 38329616669 — release, macos, windows jobs all success; User guide run 38329614505 success. Files: `Allomos_0.1.2_aarch64.dmg` (3.7 MB), `Allomos_0.1.2_x64-setup.exe` (2.7 MB), `alm_0.1.2_macos_arm64.tar.gz` (0.8 MB), `alm_0.1.2_windows_x64.exe` (2.0 MB); not marked pre-release. `/releases/latest` → `v0.1.2`. Downloaded `.dmg`: `codesign --verify --deep --strict` passes, bundle version 0.1.2; downloaded `alm --version` → `alm 0.1.2`.
- 2026-10-13 — **Done, owner verified.**
