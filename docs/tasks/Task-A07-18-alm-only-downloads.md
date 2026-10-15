# Task-A07-18-alm-only-downloads

*Card type: coding. Area A07 (Platform & release). Written 2026-10-15 at the owner's request, after the owner double-clicked `alm_0.1.5_windows_x64.exe` on Windows instead of the installer: a console program opens and closes at once, which looked like a crash.*

## What gets built (plain words)

A release page shows one obvious installer per system. The `alm` command on its own, for a computer without the app, stays available, but zipped and named `alm-only_…`, so nobody mistakes it for the installer. The release notes say most people want the installer.

| File on the release | What it is |
|---|---|
| `Allomos_<v>_x64-setup.exe` | Windows installer (also installs `alm`); the only `.exe` |
| `Allomos_<v>_aarch64.pkg`, `Allomos_<v>_aarch64.dmg` | Mac installers (unchanged) |
| `alm-only_<v>_windows_x64.zip` | holds `alm.exe` alone (was `alm_<v>_windows_x64.exe`) |
| `alm-only_<v>_macos_arm64.tar.gz` | holds `alm` alone (was `alm_<v>_macos_arm64.tar.gz`) |

## Details for the worker

- **Outcome.** The release page's only `.exe` is the Windows installer; `alm` alone is zipped, named `alm-only`, and the notes say which file to pick.
- **Decisive unknowns.** None: keep `alm` alone, zip and rename it (owner, chat 2026-10-15).
- **Files owned.** `.github/workflows/release.yml` (the two "Attach to the release" steps and the release notes); `docs/guide/src/content/docs/start/getting-started.md` and its `ja` and `zh-cn` copies (the `alm_…` file names); this card.
- **External operations.** One, approved (below): when this is implemented and merged, bump the version (patch) and push its tag, so GitHub builds and publishes a new release. Nothing else.
- **Not affected.** A07-16's update notice does not read these file names.
- **Done means mechanically.**
  - `release.yml`: Windows zips `alm.exe` into `alm-only_<v>_windows_x64.zip` (PowerShell `Compress-Archive` or `7z`, both on GitHub's Windows runner); Mac names its archive `alm-only_<v>_macos_arm64.tar.gz`; the notes list the installers first and say `alm-only` is for computers without the app.
  - The guide pages (en, ja, zh-CN) name the new files and say to unpack, then run `alm` from a terminal.
  - After the release tag: the release lists exactly one `.exe`; the zip opens to `alm.exe`; the guide site builds.
  - `git diff --check`; links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | Make the task ready; when it is implemented, make GitHub create a new release (bump the version, push the tag) | owner (chat) |
| 2026-10-15 | Open this card: keep `alm` alone, zip it and name it `alm-only` (not started; backlog) | owner (chat) |

## Notes

- 2026-10-15: the 0.1.5 Windows installer itself installed and worked on the owner's PC; only the file mix-up led here.
