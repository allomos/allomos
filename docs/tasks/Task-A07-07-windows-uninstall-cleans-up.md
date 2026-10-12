# Task-A07-07-windows-uninstall-cleans-up

*Card type: coding. Area A07 (Platform & release). Written 2026-10-12 at the owner's request ("allow uninstall in the setup exe file, to clean up the env change, the installed files, etc."). Builds on [`A07-06`](./Task-A07-06-windows-installer-installs-alm.md), whose installer adds `alm.exe` and a PATH entry.*

## What gets built (plain words)

The Windows setup (`Allomos_…_x64-setup.exe`) changes the computer in several places: it copies the app and `alm.exe` into a folder, adds that folder to the person's PATH (the list of folders Windows searches for commands), makes Start menu and desktop shortcuts, registers itself in *Settings › Apps*, and the app later keeps its own settings and web cache under the person's AppData folder. After this card, a person can remove Allomos completely, both from *Settings › Apps* and by running the same `setup.exe` again and choosing **Uninstall**, and afterwards nothing Allomos added is left behind. Their projects and taskbags are never touched.

| Piece | What it is | Where |
|---|---|---|
| Uninstall from the setup | Running `setup.exe` on a computer that already has Allomos offers **Uninstall** (beside reinstall/update). | `apps/desktop/src-tauri/windows/hooks.nsh`, `tauri.windows.conf.json` |
| Full clean-up list | Everything the installer and the app create is listed once and removed on uninstall: app folder, `alm.exe`, PATH entry, shortcuts, uninstall registry entry, app data (settings, recent folders, WebView2 cache). | same, plus this card |
| Keep or delete my settings | The uninstaller asks whether to also delete Allomos's settings; the default keeps them, so a reinstall remembers recent folders. | Tauri NSIS "delete app data" option |

**What you can see at the end:** on Windows, install Allomos, open a project, then uninstall (once from *Settings › Apps*, once from `setup.exe`). Afterwards the app folder, shortcuts and PATH entry are gone, `alm` no longer runs in a new terminal, Allomos is not listed in *Settings › Apps*, and, if asked, its AppData folders are gone. The project folder and its taskbag are unchanged.

## Details for the worker

- **Outcome.** Uninstalling Allomos on Windows undoes everything the installer and the app added, and nothing else.
- **Decisive unknowns.**
  - What Tauri's NSIS installer already offers when run a second time, and whether an Uninstall choice can be added through hooks or needs a custom installer template: found by reading the generated `installer.nsi`.
  - The exact AppData folders the app writes (`io.allomos` under Roaming and Local, WebView2 `EBWebView`): found by listing them on Windows after use.
  - An agent or terminal already running keeps the old PATH until restarted: the owner's check opens a new one.
- **Never removed:** project folders, taskbags, anything outside Allomos's own install and data folders, and other entries in PATH.
- **Files owned.** `apps/desktop/src-tauri/windows/hooks.nsh`, `apps/desktop/src-tauri/tauri.windows.conf.json`, possibly a custom NSIS template if hooks cannot do it, this card.
- **External operations.** None. The installer is built on the Mac and handed to the owner.
- **Done means mechanically.**
  - The macOS app build is unchanged; the card's named checks pass.
  - The generated `installer.nsi` shows each item of the clean-up list being removed.
  - Owner check on Windows: install, use, uninstall both ways; the "what you can see" list above holds, and the rest of PATH is unchanged.

## Approvals

| Date | What was approved | By |
|---|---|---|
| | (awaiting the owner: approve the card, `ready`) | |

## Notes

- 2026-10-12 — Card written at the owner's request; task added in backlog.
