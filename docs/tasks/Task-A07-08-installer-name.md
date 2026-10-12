# Task-A07-08-installer-name

*Card type: coding. Area A07 (Platform & release). Written 2026-10-12 at the owner's request: "make the exe name the {full app name}_setup.exe".*

## What gets built (plain words)

The Windows build script (`apps/desktop/scripts/build-windows.sh`, [`A07-02`](./Task-A07-02-build-pipeline.md)) leaves the installer in `target/windows/` under Tauri's own name, `Allomos_0.1.0_x64-setup.exe`. After this card it is named after the app alone: **`Allomos_setup.exe`**. The name comes from the app's `productName` in `apps/desktop/src-tauri/tauri.conf.json`, so a renamed app gets a matching installer name. `alm.exe` keeps its name.

## Details for the worker

- **Outcome.** `bash apps/desktop/scripts/build-windows.sh` leaves `target/windows/Allomos_setup.exe` and `target/windows/alm.exe`.
- **Decisive unknowns.** None. (Version and `x64` are no longer in the file name; the installer itself still records the version.)
- **Files owned.** `apps/desktop/scripts/build-windows.sh`; the build section of `docs/workflow/CODING_RULES.md`.
- **External operations.** None.
- **Done means mechanically.** The script runs from this branch with exit 0 and `target/windows/` holds exactly `Allomos_setup.exe` (a "Nullsoft Installer self-extracting archive") and `alm.exe`.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Name the Windows installer `{full app name}_setup.exe` | owner (chat) |

## Notes

- 2026-10-12 — Card written from the owner's request.
