# Task-A03-03-app-asks-agents-to-set-up

*Card type: coding. Area A03 (Agents). Requested by the owner (2026-10-07): when the app creates a seabag, it should find the AI agents on this computer and ask them to add FlipHopper's instructions, instead of writing the files itself. Builds on the research in [`Task-A03-01`](./Task-A03-01-agent-launch-research.md) and the request text from [`Task-A01-14`](./Task-A01-14-init-asks-the-agent.md).*

## What gets built (plain words)

After the app creates a seabag, a small panel says "Tell AI agents about FlipHopper?" and lists the agents it found on this computer, as **apps** and as **command-line tools**. The person ticks one or more and presses **Ask**. Each ticked agent opens in this project with the request already typed in; the person reads it and sends it. "Not now" closes the panel, and it can be opened again later.

| Piece | What it is | Where |
|---|---|---|
| Finding agents | Looks for: **Claude Code (terminal)** — the `claude` command; **Claude (app)** — Claude.app; **Codex (terminal)** — the `codex` command, only if it actually runs; **Codex (app)** — the app that opens `codex://` links. Each found one shows; missing ones do not. | `apps/desktop` (Rust) |
| The panel | A list with tick boxes, one line per agent and what will happen ("opens a terminal with the request typed in", "opens the app with the request typed in"), then Ask and Not now. In three languages. | `apps/web` |
| Asking | Claude Code terminal: the `claude-cli://open?cwd=…&q=…` link. Claude app: `claude://code/new?folder=…&q=…`. Codex app: `codex://threads/new?path=…&prompt=…`. Codex terminal: a new Terminal window running `codex "<request>"`. Nothing is sent by FlipHopper: the person presses Enter in the agent. | `apps/desktop` |
| Several agents | Each agent is told which file is its own (Claude: `CLAUDE.md`; Codex: `AGENTS.md`) and to read what is already there first, so two agents do not add the same text twice. | the request text |
| No agent found | The panel says so and offers the old way: "Write the files for me" (A01-13). | `apps/web` |

**What you can see at the end:** choose a new folder in the app; the panel lists Claude Code and the Claude app (and Codex once it works); tick Claude Code, press Ask: a terminal opens in that folder with the request typed in; press Enter and Claude adds the section.

## Details for the worker

- **Outcome.** A person sets up a project for AI agents in one click, and the agent writes the instructions in a way that fits the project.
- **Defaults chosen by the agent** (owner may change them on review):
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
| 2026-10-07 | "let the app try to find the agents and ask the agents to do the job. if there are multiple agents, ask user to choose one or many. should consider the situation of agent apps and CLIs. show a UI to the user to do select." | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent from the owner's request; the two defaults above wait for the owner's yes.
