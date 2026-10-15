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

## What was done (plain words)

The release workflow now puts `alm` alone into archives named `alm-only_…`, so the Windows installer is the only `.exe` on the release page. The release notes list the installers first and say the `alm-only` files are for computers without the app. The getting-started guide (English, Japanese, Chinese) names the new files, says to unpack them and run `alm` from a terminal, and warns that double-clicking `alm.exe` only flashes a window. The version is now 0.1.6, ready for the coordinating session to push tag `v0.1.6` after the merge.

- Decided: the Windows zip is made with PowerShell `Compress-Archive` called as `pwsh` from the step's bash shell, because the same command runs on the Mac (pwsh installed), so the exact step could be dry-run here; `7z` is not on this Mac.
- Decided: the notes are a short multi-line text (a list of installers, then one line on `alm-only`) instead of one long sentence, so the installers read first.
- Decided: the guide shows both `alm-only` files as a two-item list (Mac and Windows), since before only the Mac file was named.
- Decided: the version bump touches exactly the seven files of the 0.1.5 bump (7d5b46f); locks updated with `npm version --no-git-tag-version` and `cargo update --workspace --offline`, which changed only our own four crates' versions.

## Evidence (2026-10-16, worker `claude/a07-18-alm-only`)

- Workflow file parses (Ruby YAML): jobs `release`, `macos`, `windows`; every `run` step passes `bash -n`. actionlint: NOT RUN (not on this Mac; no new global tools installed).
- Version check step simulated with `TAG=v0.1.6`: tauri.conf.json 0.1.6 = Cargo.toml 0.1.6 → PASS.
- Mac packaging dry-run with a release `alm` build: `tar -czf alm-only_0.1.6_macos_arm64.tar.gz -C target/release alm` → archive lists exactly `alm` (2713680 bytes); unpacked `alm --version` prints `alm 0.1.6`. PASS.
- Windows packaging dry-run on the Mac (the release `alm` copied as `alm.exe`, same command as the workflow): `pwsh -NoProfile -Command "Compress-Archive -Path target/release/alm.exe -DestinationPath alm-only_0.1.6_windows_x64.zip -Force"` → zip lists exactly `alm.exe`; unpacked file is byte-identical. No `.exe` left beside the zip. PASS. Not exercised: the step on GitHub's Windows runner.
- Guide site: `npm ci` + `npm run build` in `docs/guide` → 10 pages built; each getting-started page (en, ja, zh-cn) contains `alm-only`; no `alm_` names left in the guide sources. PASS.
- Coding-rules checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, web `typecheck`, `build`, `check:i18n`, `test` → all PASS.
- `git diff --check` PASS.
- Still to verify after tag `v0.1.6` (coordinating session / owner): the release page lists exactly one `.exe` (`Allomos_0.1.6_x64-setup.exe`), plus `.pkg`, `.dmg`, `alm-only_0.1.6_macos_arm64.tar.gz`, `alm-only_0.1.6_windows_x64.zip`; the zip opens to `alm.exe`; the notes read installers first.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | Make the task ready; when it is implemented, make GitHub create a new release (bump the version, push the tag) | owner (chat) |
| 2026-10-15 | Open this card: keep `alm` alone, zip it and name it `alm-only` (not started; backlog) | owner (chat) |

## Notes

- 2026-10-15: the 0.1.5 Windows installer itself installed and worked on the owner's PC; only the file mix-up led here.
