# Task-A03-16-agents-use-the-apps-own-alm

*Card type: coding.*

## What gets built (plain words)

When Allomos opens an AI agent in a terminal window, the agent's `alm` is the one that came with that same copy of Allomos. Today the terminal finds whatever `alm` the person's shell finds first, usually the installed one in `/usr/local/bin`.

| Piece | In everyday words | Where |
|---|---|---|
| The app's own `alm` | The `alm` next to the app's program: inside `Allomos.app` when installed, `target/debug/alm` when run with `tauri dev` | `apps/desktop/src-tauri/src/alm_command.rs` already finds it |
| Terminal window for an agent | Before starting `claude` or `codex`, the window puts the folder of the app's own `alm` first in `PATH`, for that window only | `apps/desktop/src-tauri/src/agents.rs` (`ask`) |
| Request sent through a link | For an agent opened in the Claude desktop app, the request itself names the app's own `alm` by full path and asks the agent to use it | where the app writes its requests to agents |

What the owner can see at the end: an app built from a task's worktree opens an agent, and `which alm` in that window names the worktree's `alm`; the installed app opens one, and it names the installed app's `alm`.

## Details for the worker

- **Outcome.** An agent the app opens in a terminal window uses the `alm` that came with that app, so testing an in-development app also tests its in-development `alm`, with nothing set by hand.
- **Decisive unknowns.**
  - Agents opened through a link (the Claude desktop app, `how == "link"`) start outside any terminal window the app controls, so `PATH` cannot be set for them. Instead the request the app sends names the app's own `alm` by its full path and asks the agent to use it (owner, 2026-10-14). The request names it only when it differs from the `alm` the person's shell finds (owner, 2026-10-14); `alm_command.rs` already finds both (the app's own, and the shell's `command -v alm`, fully resolved), so reuse that. Start from this wording (owner, 2026-10-14): "In this task, run `alm` as `<full path>`, every time, never as plain `alm`." Try it with Claude and Codex; strengthen it only if an agent goes back to plain `alm`.
  - Windows (PowerShell window): the same change, `$env:PATH` set before starting the agent, where `alm.exe` sits beside the app. Check on Windows if a Windows computer is available; otherwise mark it untested.
  - When the app has no `alm` beside it (for example `tauri dev` before `cargo build -p allomos-cli`): leave `PATH` unchanged rather than fail.
- **Files owned.** `apps/desktop/src-tauri/src/agents.rs`; reuse the finder in `apps/desktop/src-tauri/src/alm_command.rs`; one line in `docs/workflow/CODING_RULES.md` (Install on a Mac / testing).
- **External operations.** None.
- **Done means mechanically.**
  - Unit test: the terminal command the app builds puts the app's `alm` folder first, and is unchanged when there is no `alm` beside the app.
  - Manual check: from a worktree's `tauri dev`, ask Claude Code in a test project under `~/allomos-tests/a03-16-own-alm/`; in that window `which alm` names `<worktree>/target/debug/alm`. With the installed app it names `/Applications/Allomos.app/Contents/MacOS/alm`.
  - Unit test: a request for an agent opened through a link names the app's own `alm` by full path when the shell's `alm` is a different one, and does not when they are the same.
  - Manual check: from a worktree's `tauri dev`, ask Claude Code through the Claude desktop app; the agent runs the worktree's `alm`.
  - The person's own terminal windows are unchanged.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Write this card and approve it (ready); local work only | owner (chat) |
| 2026-10-14 | Agents opened through a link: the request names the `alm` to use, only when the shell would find a different one | owner (chat) |

## Notes

- 2026-10-14: Found while installing a newer `alm`: an app run from a worktree still sends its agents to the installed `alm`, so a task's new `alm` commands cannot be tried through the app. Until this is done, start the agent by hand with `PATH="<worktree>/target/debug:$PATH"`, in a test project, never on the real taskbag.
- 2026-10-14 (claude/a03-16-own-alm): **Built; locally verified.** A Terminal window the app opens for Claude Code or Codex now first runs `export PATH='<folder of the app's own alm>':"$PATH"`, so that window, and the agent started in it, find the app's own `alm` first; other Terminal windows are untouched. With no `alm` beside the app the window types exactly what it did before. On Windows the PowerShell window sets `$env:PATH` the same way (**untested on Windows**: no Windows computer). For agents opened through a link, the request starts with the owner's line "In this task, run `alm` as `<full path>`, every time, never as plain `alm`." only when the shell's `alm` (resolved) is a different file; the line is in English in every language, as the owner wrote it. The finder is `alm_command::own()` (the `alm` beside the app's program, also in development) and `alm_command::shell_alm()`.
  - Evidence: unit tests `terminal_windows_put_the_apps_own_alm_first` and `link_requests_name_the_apps_alm_only_when_the_shell_finds_another` pass. Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`: all PASS. Headless: the exact text the window types, run in a clean login `zsh -il` in `~/allomos-tests/a03-16-own-alm/project`, gave `which alm` = `<worktree>/target/debug/alm`, also in a nested `zsh -i` (as an agent's command tool starts one), and `alm --version` ran; without the line it gave `/usr/local/bin/alm`.
  - Not run (owner): the GUI checks with `tauri dev`, the installed app, the Claude desktop app and Codex; see the steps in the report.
