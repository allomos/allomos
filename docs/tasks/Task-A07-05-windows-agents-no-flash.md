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
