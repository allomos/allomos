# Task-A03-03-app-asks-agents-to-set-up

*Card type: coding. Area A03 (Agents). Requested by the owner (2026-10-07): when the app creates a seabag, it should find the AI agents on this computer and ask them to add FlipHopper's instructions, instead of writing the files itself. Builds on the research in [`Task-A03-01`](./Task-A03-01-agent-launch-research.md) and the request text from [`Task-A01-14`](./Task-A01-14-init-asks-the-agent.md).*

## What gets built (plain words)

After the app creates a seabag, a small panel says "Tell AI agents about FlipHopper?" and lists the agents it found on this computer, as **apps** and as **command-line tools**. The person ticks one or more and presses **Ask**. Each ticked agent opens in this project with the request already typed in; the person reads it and sends it. "Not now" closes the panel, and it can be opened again later.

| Piece | What it is | Where |
|---|---|---|
| Project folder | The panel shows **Project folder: <path> [Change…]**. It starts as the folder the seabag was created in; the person may pick another folder (for example the repository's root above it). Agents open in that folder, and the request names it and the seabag's absolute path. |  `apps/web`, `apps/desktop` |
| Finding agents | Looks for: **Claude Code (terminal)** — the `claude` command; **Claude (app)** — Claude.app; **Codex (terminal)** — the `codex` command, only if it actually runs; **Codex (app)** — the app that opens `codex://` links. Each found one shows; missing ones do not. | `apps/desktop` (Rust) |
| The panel | A list with tick boxes, one line per agent and what will happen ("opens a terminal with the request typed in", "opens the app with the request typed in"), then Ask and Not now. In three languages. | `apps/web` |
| Asking | Claude Code terminal: the `claude-cli://open?cwd=…&q=…` link. Claude app: `claude://code/new?folder=…&q=…`. Codex app: `codex://threads/new?path=…&prompt=…`. Codex terminal: a new Terminal window running `codex "<request>"`. Nothing is sent by FlipHopper: the person presses Enter in the agent. | `apps/desktop` |
| Several agents | Each agent is told which file is its own (Claude: `CLAUDE.md`; Codex: `AGENTS.md`) and to read what is already there first, so two agents do not add the same text twice. | the request text |
| No agent found | The panel says so and offers the old way: "Write the files for me" (A01-13). | `apps/web` |

**What you can see at the end:** choose a new folder in the app; the panel lists Claude Code and the Claude app (and Codex once it works); tick Claude Code, press Ask: a terminal opens in that folder with the request typed in; press Enter and Claude adds the section.

## Details for the worker

- **Outcome.** A person sets up a project for AI agents in one click, and the agent writes the instructions in a way that fits the project.
- **Owner's answers (2026-10-07):** both defaults below: yes. Add a project folder the person can change, and name the seabag's path in the request.
- **Defaults** (accepted):
  1. FlipHopper only **opens** the agent with the request typed in; it never sends it or runs an agent in the background, so nothing runs or spends the person's plan without them pressing Enter.
  2. The panel shows after a seabag is created, and from a menu item (File → Tell AI Agents About This Project…).
- **Decisive unknowns.**
  - Whether the `claude-cli://` handler is registered on a given Mac (it registers after the first interactive Claude Code prompt) — the worker checks for `~/Applications/Claude Code URL Handler.app` and otherwise opens Terminal with `claude "<request>"`.
  - How a macOS app finds command-line tools without the person's shell path — the worker checks the usual places (`~/.local/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, npm's global folder) and asks the login shell.
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs` (or a new `agents.rs`), `apps/web/src/App.tsx` (or a new panel component), `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `docs/decisions/0005-start-and-open-agent-sessions.md` (what this decides), this card.
- **External operations.** None by FlipHopper. The person's own agent, once they press Enter, uses their plan.
- **Done means mechanically.** The usual Rust and web checks pass; unit tests cover agent detection with made-up folders and the link building (encoding of folder and request); in the built app on this Mac the panel lists what is installed and Ask opens each ticked agent with the request. Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Owner check passed: the panel and Ask ("passed") | owner (chat) |
| 2026-10-07 | Defaults 1 and 2: "yes", "yes"; "did you specify the tracking seabag path? and maybe we should let user select the working project path?" | owner (chat) |
| 2026-10-07 | "let the app try to find the agents and ask the agents to do the job. if there are multiple agents, ask user to choose one or many. should consider the situation of agent apps and CLIs. show a UI to the user to do select." | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent from the owner's request; the two defaults above wait for the owner's yes.

## Result (2026-10-07)

**In plain words.** After the app creates a seabag, a panel "Tell AI agents about FlipHopper?" shows the project folder (with **Change…**), the seabag's path, and the AI agents found on this computer, each with a tick box and what will happen. **Ask** opens every ticked agent in the project folder with the request from A01-14 typed in (naming both paths); the person reads and sends it. With several agents, the request starts with a note telling them about each other, so they check before writing. If no agent is found, **Write the files for me** writes the section directly (A01-13). **Not now** closes the panel; **File → Tell AI Agents About This Project…** opens it again. The app no longer writes `AGENTS.md` / `CLAUDE.md` on its own.

- **How agents are found:** command-line tools in the usual folders (a macOS app does not get the person's shell path) that answer `--version` — so a broken install, like this Mac's Codex CLI, is not offered; apps by the link types their `Info.plist` declares. On this Mac: Claude Code (`~/.local/bin/claude`, link handler present), Claude (app), and the ChatGPT app (it opens `codex://` links).
- **How they are asked:** `claude-cli://open?cwd=…&q=…`, `claude://code/new?folder=…&q=…`, `codex://threads/new?path=…&prompt=…`; a command-line tool without a link opens in a new Terminal window in the project folder, with the request on the clipboard to paste. FlipHopper never sends the request.
- **Decisive unknowns answered:** the Claude Code link handler is detected by its app (`~/Applications/Claude Code URL Handler.app` declares `claude-cli`); tools are found in `PATH` plus `~/.local/bin`, `~/.claude/local`, `~/.npm-global/bin`, `~/.bun/bin`, `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`. One detection run missed `claude` once; it now asks twice.
- **Where:** `apps/desktop/src-tauri/src/agents.rs` (new), commands `find_agents`, `ask_agents`, `write_agent_files` and the menu item in `lib.rs`; `apps/web/src/AgentsPanel.tsx` (new); `App.tsx`, `messenger.ts`, `types.ts`; 17 texts in three languages. Windows: not yet (Iteration 2).

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (new: link encoding, the shared note, broken tools not offered, quoting) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (99 texts) | PASS |
| Detection on this Mac with an app-like bare `PATH` (three runs) | PASS: Claude Code (terminal, link), Claude (app), Codex (app: ChatGPT); Codex CLI not offered |
| Built app on a new folder: seabag and README created; File menu shows the new item (ファイル in Japanese) | PASS |
| The panel on screen, and Ask opening each agent with the request typed in | not seen: the agent's accessibility tool could not reach the window during this test — owner check |

- 2026-10-08 — **Owner verified** in the built app ("passed").
- 2026-10-08 — Owner checked again end to end: the panel, Ask with Claude Code (terminal), and the agent line turning to ✓ ("passed"). Follow-up requests are in [`A03-05`](./Task-A03-05-agents-panel-refinements.md).
