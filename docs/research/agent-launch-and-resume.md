# Starting and reopening agent sessions

**Evidence collected 2026-10-07** for [`Task-A03-01`](../tasks/Task-A03-01-agent-launch-research.md) and [decision 0005](../decisions/0005-start-and-open-agent-sessions.md). Sources: the vendors' own documentation (read 2026-10-07), plus `--help` output of the tools installed on the owner's Mac. **No agent was started**: no prompt was sent, no session or cloud task was created, nothing was spent.

**Status marks:** **verified locally** — seen in the installed tool's own help on this Mac; **documented only** — in vendor documentation, not tried; **not available** — the documentation offers no such thing.

Installed on this Mac: Claude Code **2.1.191** (`/Users/wanghui/.local/bin/claude`). Codex CLI **0.130.0** from Homebrew (`/opt/homebrew/bin/codex`), but **it does not run**: its platform binary is missing (`spawn …/codex-darwin-arm64/vendor/…/codex ENOENT`), so nothing about Codex could be checked locally. The Claude desktop app, the ChatGPT app and the Claude Code URL handler are installed.

## In plain words

Both agents can be **started by another program** with a prompt in a chosen folder. Both report a **session ID** that FlipHopper can store on the task, and both can be **reopened** later from that ID.

- **Claude Code** has the richest set of tools for this:
  - `claude -p` runs one prompt and prints the session ID as JSON;
  - `claude --bg` starts a background session that you can list, open and stop later;
  - a `claude-cli://` link opens a new terminal already in the right folder with the prompt typed in;
  - `claude --cloud` starts a cloud session that has its own claude.ai web address.
- **Codex**:
  - `codex exec --json` runs a prompt and reports a *thread ID*;
  - `codex resume <id>` reopens that thread in the terminal;
  - a `codex://threads/<id>` link opens it in the desktop app;
  - `codex cloud exec` starts a cloud task.

So FlipHopper's **"Start in agent"** can use each agent's own non-interactive start, and **"Open session"** can use each one's resume command or link. No screen automation is needed.

## Claude Code

| What | How | Status |
|---|---|---|
| Start, non-interactive, in a folder | run in the folder: `claude -p "<prompt>" --output-format json` (also `--model`, `--effort`, `--permission-mode`, `-n <name>`) | flags **verified locally** (`--help`); behaviour **documented only** [C1] |
| Session ID | `session_id` field of the JSON result; or choose it up front with `--session-id <uuid>` | `--session-id` **verified locally**; JSON field **documented only** [C1] |
| Resume | `claude --resume <session-id>` (interactive) or `claude -p --resume <id> "<prompt>"`; works from any folder since 2.1.223 | `--resume` **verified locally**; cross-folder lookup **documented only** [C2] (installed 2.1.191 is older) |
| Background session (start now, look later) | `claude --bg --name <name> "<prompt>"` prints a short ID; `claude agents --json` lists sessions with `state` (`working`, `blocked`, `done`, `failed`, `stopped`) and `sessionId`; `claude attach <id>`, `claude logs <id>`, `claude stop <id>` | `--bg`, `agents`, `attach` **verified locally**; `--json` fields **documented only** [C3] |
| Open a terminal window for the user | `open "claude-cli://open?cwd=<abs path>&q=<prompt>"` (macOS; Windows `start "" "<link>"`). Opens a **new** session with the prompt filled in, not sent; the handler registers after the first interactive prompt | handler **verified locally** (`~/Applications/Claude Code URL Handler.app`); link **documented only** [C4] |
| Link to a specific session | desktop app: only `claude://code/new?q=…&folder=…` (new session) is documented, no link to an existing Code session [C5]; cloud sessions have `https://claude.ai/code/<session-id>` [C6] | **documented only**; existing local session link **not available** |
| Cloud session | `claude --cloud "<task>"` starts a cloud session for the current repository's GitHub remote; follow-ups with `claude -p "<msg>" --cloud <session-id>` (JSON `{ok, session_id, url}`); `claude --teleport <id>` pulls it into a terminal. Needs a claude.ai Pro/Max/Team/Enterprise sign-in, not an API key; uses the account's normal limits, no separate compute charge [C6] | **documented only** (`--cloud` is not in 2.1.191's help; the older alias `--remote` is documented) |

## Codex

| What | How | Status |
|---|---|---|
| Start, non-interactive, in a folder | `codex exec --json -C <folder> "<prompt>"` (also `-m <model>`, `--config key=value` for reasoning effort) [X1] | **documented only** (local Codex broken) |
| Session ID | first JSON line `{"type":"thread.started","thread_id":"…"}` [X1; X2 *secondary*] | **documented only** |
| Resume | `codex resume <SESSION_ID>` (interactive); `codex exec resume <SESSION_ID> "<prompt>"`; `--last` (current folder) or `--all` [X1] | **documented only** |
| Link to a specific session | desktop app: `codex://threads/<thread-id>`; new: `codex://threads/new?prompt=…&path=<abs path>` [X3] | **documented only** |
| Cloud task | `codex cloud exec --env <ENV_ID> "<prompt>"` (`--attempts 1-4`); `codex cloud list` [X1] | **documented only** |

## Opening a window from a background program (macOS, Windows)

- **Claude Code:** the `claude-cli://` link does it: macOS reuses the terminal of the last interactive session (Terminal.app, iTerm2, Ghostty, kitty, Alacritty, WezTerm); Windows prefers Windows Terminal, then PowerShell, then `cmd.exe` [C4].
- **Codex:** the `codex://threads/<id>` link opens the desktop app [X3]. To open a terminal instead, FlipHopper must start one itself; on macOS the standard ways are `open -a Terminal <script>.command` or AppleScript `tell application "Terminal" to do script "…"`, and on Windows `wt.exe` or `start`. *Not checked against vendor documentation in this card.*

## What this means for decision 0005

- **Start in agent (Iteration 8):**
  - Claude Code: `claude --bg` in the task's worktree. It survives FlipHopper closing, reports state through `claude agents --json`, and gives a session ID to store on the task.
  - Codex: `codex exec --json -C <worktree>`, run and watched by FlipHopper itself, reading `thread_id` from the first line.
- **Open session:**
  - Claude Code: `claude attach <id>` or `claude --resume <session-id>`, in a terminal FlipHopper opens.
  - Codex: `codex://threads/<id>`, or `codex resume <id>` in a terminal.
- **Status (running / waiting / exited):** Claude Code reports it itself (`state` and `waitingFor`). For Codex, FlipHopper watches its own child process and the JSON events (`turn.completed`, `turn.failed`).
- **Cloud:** both agents can start cloud work from their command line with the user's own sign-in. No separate API is needed, but it uses the user's plan. Treat it as an explicit, per-task choice.
- **Model and effort per task:** both accept them at start (`--model` / `--effort`; `-m` / `--config`).

## To check before building (Iteration 8 card)

1. Repair or reinstall Codex on the test Mac, then run the Codex rows for real: one short prompt with the owner's approval of its usage.
2. Update Claude Code to the documented version, then confirm `--bg`, `agents --json`, `--resume` from another folder and `--cloud` with one short prompt each.
3. Whether the Claude desktop app can open an existing local Code session by link (not documented today).

## Sources (read 2026-10-07)

- C1. Claude Code, "Run Claude Code programmatically" — <https://code.claude.com/docs/en/headless>
- C2. Claude Code, "Manage sessions" — <https://code.claude.com/docs/en/sessions>
- C3. Claude Code, agent view and background sessions — <https://code.claude.com/docs/en/agent-view>
- C4. Claude Code, "Launch sessions from links" — <https://code.claude.com/docs/en/deep-links>
- C5. Claude Help Center, "Open Claude Desktop with a link" — <https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link>
- C6. Claude Code, "Use Claude Code in the cloud" — <https://code.claude.com/docs/en/claude-code-on-the-web>
- X1. OpenAI Codex, developer commands (CLI) — <https://learn.chatgpt.com/docs/developer-commands?surface=cli>
- X2. *Secondary:* "codex exec JSONL Reference" — <https://codex.danielvaughan.com/2026/04/08/codex-exec-jsonl-reference/>
- X3. OpenAI Codex app, deep links — <https://learn.chatgpt.com/docs/reference/commands>
