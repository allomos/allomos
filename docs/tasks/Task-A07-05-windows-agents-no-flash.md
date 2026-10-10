# Task-A07-05-windows-agents-no-flash

*Card type: coding. Area A07 (Platform & release), Iteration 2. Written 2026-10-11 after the owner ran a Mac-built Windows installer on a Windows computer and reported two problems. Split from [`A07-03`](./Task-A07-03-windows-works.md), which keeps paths, line endings and file locking.*

## What gets built (plain words)

On Windows, Allomos (1) finds no AI agents, because the search was written for macOS only, and (2) flashes a black command window every few seconds, because each time the app runs `git` (every 4 seconds, to show which agent last changed the taskbag) Windows gives it a console window. After this card, the Windows app runs its helper programs invisibly and finds and asks Claude and Codex as the Mac app does.

| Piece | What it is | Where |
|---|---|---|
| No flashing | Every program the app starts on Windows (`git`, the agents' `--version` check, `reg`, the clipboard) is started without a window. | `apps/desktop/src-tauri/src/agents.rs`, `lib.rs` |
| Finding the terminal agents | `claude` and `codex` looked for as `.exe` / `.cmd` in `PATH` and where Windows installers put them (`%USERPROFILE%\.local\bin`, `%APPDATA%\npm`, …). | `agents.rs` |
| Finding the apps | The Claude and Codex apps found by the `claude://`, `claude-cli://` and `codex://` links they register with Windows. | `agents.rs` |
| Asking | A link opens with Windows' own opener; a terminal agent opens in a new PowerShell window in the project folder with the request on the clipboard. | `agents.rs` |

**What you can see at the end:** on Windows, no window flashes; the agents panel lists the Claude and Codex installed there, and asking one opens it in the project with the request ready.

## Details for the worker

- **Outcome.** A Windows user sees no console flashes and can ask Claude or Codex to set up a project, as on the Mac.
- **Decisive unknowns.**
  - Which links the Windows Claude and Codex apps register, and where their terminal tools are installed: answered on the owner's Windows computer with the new build (the app shows what it found).
- **Files owned.** `apps/desktop/src-tauri/src/agents.rs`, `apps/desktop/src-tauri/src/lib.rs` (starting programs), this card.
- **External operations.** None. The test build is made on the Mac and handed to the owner.
- **Done means mechanically.**
  - The six named checks pass on the Mac; `cargo xwin clippy` for `x86_64-pc-windows-msvc` passes with `-D warnings`.
  - New unit tests: Windows tool names, reading a link registration, the clipboard's text encoding, PowerShell quoting.
  - Owner check on Windows: no flashing window; the agents panel lists the agents installed; asking one opens it.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | Open the card and start it now; cross-build a new installer to try | owner (chat) |

## Notes

- 2026-10-11 — Card written and claimed by the agent.
- 2026-10-11 — Implemented. Every program the app starts now goes through `agents::command`, which on Windows sets `CREATE_NO_WINDOW` (the 4-second `git log` check was the flashing window). On Windows, `claude` / `codex` are looked for as `.exe`, `.cmd`, `.bat` in `PATH`, `~\.local\bin`, `%APPDATA%\npm`, `%LOCALAPPDATA%\pnpm` and WinGet's links folder; the apps are found by their `claude://`, `claude-cli://`, `codex://` registrations (`reg query HKCR\<scheme>\shell\open\command`); links open with `rundll32 url.dll,FileProtocolHandler`; a terminal agent opens in a new PowerShell window in the project, with the request put on the clipboard by `clip` (UTF-16 so Chinese and Japanese survive). macOS behaviour unchanged (probe on this Mac still finds Claude Code, Claude and ChatGPT).
  Checks: PASS `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, web `typecheck`, `build`, `check:i18n`; PASS `cargo xwin clippy --target x86_64-pc-windows-msvc --workspace --all-targets -- -D warnings`; `git diff --check` clean. NOT RUN: tests on Windows (no Windows computer here; A07-02). Installer for the owner: `Allomos_0.1.0_x64-setup.exe` (2.2 MB), cross-built from this branch. Next: owner check on Windows.
- 2026-10-11 — Owner's first Windows run: Claude was found and opened in the project (`C:\coding\selective-rendering`) with the request. It stopped because `alm` is not installed on that computer: the installer does not ship `alm`, and nothing in v0.2 does yet (CODING_RULES: "Bundling with the app is a later card"). Not this card's outcome; proposed as a new card. Workaround given: `alm.exe` cross-built from this branch, copied by the owner into `%USERPROFILE%\.local\bin`.
- 2026-10-11 — **Owner verified** on Windows ("checked passed") with the trial installer that held both A07-05 and A07-06: no flashing window, the agent found and opened in the project, `alm` installed by the setup and usable by the agent. Merged into `main`.
