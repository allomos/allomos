# Task-A07-06-windows-installer-installs-alm

*Card type: coding. Area A07 (Platform & release), Iteration 2. Written 2026-10-11 at the owner's request ("I want it auto installed by the setup.exe") after an agent on Windows, asked by Allomos to set up a project, stopped because `alm` was not installed ([`A07-05`](./Task-A07-05-windows-agents-no-flash.md) notes).*

## What gets built (plain words)

Allomos asks AI agents to work with the `alm` command, but the Windows installer installs only the app, so on a new Windows computer the agent cannot find `alm`. After this card, running `Allomos_…_x64-setup.exe` also installs `alm.exe` and makes it available in every new terminal; uninstalling Allomos removes both.

| Piece | What it is | Where |
|---|---|---|
| `alm.exe` in the installer | The installer carries `alm.exe` and puts it next to `Allomos.exe` in the app's folder. | `apps/desktop/src-tauri/tauri.windows.conf.json`, `apps/desktop/scripts/build-windows.sh` |
| On the PATH | The installer adds the app's folder to the person's own PATH (no administrator rights), so `alm` works in any new terminal; the uninstaller takes it out again. | `apps/desktop/src-tauri/windows/hooks.nsh` |

**What you can see at the end:** on a Windows computer without `alm`, install Allomos, open a new PowerShell window, and `alm --version` answers. An agent asked by Allomos can run `alm brief`.

## Details for the worker

- **Outcome.** A Windows user who installs Allomos has `alm` too, with no manual step.
- **Decisive unknowns.**
  - Whether Tauri applies a Windows-only config file when the Windows build is made on the Mac: found by building.
  - PATH must never be cut short or lost: edited as the raw registry value, not through NSIS strings (limited to 1024 characters).
  - An agent or terminal already running keeps the old PATH until restarted: the owner's check opens a new one.
- **Files owned.** `apps/desktop/src-tauri/tauri.windows.conf.json` (new), `apps/desktop/src-tauri/windows/hooks.nsh` (new), `apps/desktop/scripts/build-windows.sh` (from [`A07-02`](./Task-A07-02-build-pipeline.md): builds `alm` before the installer), `.gitignore` (the copied `alm` binary), `docs/workflow/CODING_RULES.md` (`alm` install line), this card.
- **External operations.** None. The installer is built on the Mac and handed to the owner.
- **Done means mechanically.**
  - The macOS app build is unchanged (no `alm` requirement there); the six named checks pass.
  - The Windows installer contains `alm.exe` (listed from the built installer) and its hook script.
  - Owner check on Windows: after installing, a new PowerShell runs `alm --version`; after uninstalling, `alm` is gone and the rest of PATH is unchanged.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | Make the Windows setup.exe install `alm` automatically (start now) | owner (chat) |

## Notes

- 2026-10-11 — Card written and claimed by the agent.
- 2026-10-11 — Implemented. `tauri.windows.conf.json` (applied only to Windows builds, also when made on the Mac) adds `alm` as a bundled program, so the installer puts `alm.exe` beside the app (and the uninstaller deletes it), and adds `windows/hooks.nsh`: after installing, PowerShell appends the app's folder to the person's own PATH (`HKCU\Environment`, read raw so `%…%` entries stay as written; nothing written if already there); after uninstalling, only that entry is removed. Then Windows is told the environment changed. `build-windows.sh` builds `alm.exe` first and copies it to `apps/desktop/src-tauri/binaries/alm-x86_64-pc-windows-msvc.exe` (ignored by Git).
  Checks: PASS the six named checks; PASS macOS app build (`tauri build --debug --bundles app`; `Allomos.app` holds only the app, no `alm`); PASS `build-windows.sh`. Evidence from the generated `installer.nsi`: `File /a "/oname=alm.exe" …binaries/alm-x86_64-pc-windows-msvc.exe`, `Delete "$INSTDIR\alm.exe"`, hooks included and called after install and uninstall. The PowerShell commands, compiled into a test installer and read back, are the intended text. NOT RUN: the installer on Windows (owner check).
