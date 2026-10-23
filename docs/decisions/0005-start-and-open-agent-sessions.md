# 0005 — Start a task in an AI agent, and jump into it, from Allomos

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; design proposed** | 2026-09-27 | 2026-09-27 (requirement) | will.w.hui |

## Requirement (owner, 2026-09-27)

1. **Start** a task in a **specified AI agent** (e.g. Claude Code or Codex) from the Allomos app.
2. **Jump into** a task that has already started, in that agent's session.

This reverses the earlier non-goal "Allomos tracks work; it does not start it".

## The core problem

Allomos is an online web app; the agents usually run **on a person's own machine** (Claude Code / Codex CLI or desktop app, inside a git worktree) or **in a vendor's cloud** (web sessions). A browser page cannot launch programs on a computer by itself. So Allomos needs a way to reach where the agent runs.

## Proposal

### Where an agent can run — "runners"

| Runner type | Start | Jump in |
|---|---|---|
| **Local runner** — a small Allomos daemon on a person's machine (part of the `alm` CLI: `alm runner`), registered to a project | The daemon receives a start request from Allomos (realtime subscription), creates a git worktree and branch for the task, and launches the chosen agent there with a prompt built from the task | Allomos shows the session's resume command and, on the runner's own machine, asks the daemon to open it (terminal or desktop app) |
| **Cloud agent** — a vendor-hosted session (Claude Code on the web, Codex cloud, or an API-hosted agent) | Allomos (or the runner) creates the cloud session through the vendor's supported interface | Open the session's web URL |

A person may register several runners (laptop, desktop, a shared build machine). When starting a task, the user picks **agent** (Claude / Codex), **runner** (where), and optionally model / effort.

### What happens on "Start"

1. User clicks **Start in agent** on a task → chooses agent, runner, repository and base branch.
2. Allomos records a **dispatch** event and sends the request to the runner.
3. The runner creates the worktree/branch (named from the task id, e.g. `claude/A01-01-walking-skeleton`), writes the task brief (outcome, done-when, links, approval limits) into the prompt, and starts the agent.
4. The runner reports back the **session id**, worktree path, branch and how to resume; Allomos **claims** the task for that session automatically.
5. The agent then reports progress through the `alm` CLI, as for any session (MCP deferred, [0018](./0018-mcp-deferred-agents-use-alm.md)).

### What happens on "Open session"

- Cloud session → open its URL in a new tab.
- Local session on this machine → the runner opens it (e.g. a terminal in the worktree running the agent's resume command).
- Local session on another machine → Allomos shows where it runs and the exact command to resume there; it cannot open it remotely.

### Session status

The runner reports liveness (running / waiting for input / exited) so the task view can show whether its agent is still working. This also covers the reference project's "session registry".

## Local projects (added 2026-10-04)

Under [0006](./0006-local-and-cloud-projects.md), a local project has no accounts, so the role rules below apply to **cloud projects**. In a local project, the person using the desktop app ([0009](./0009-desktop-hub-with-web-and-mobile-companions.md)) is the authority on that machine. The desktop app is also the natural home of the local runner. Two things remain open: how a local project records who did what, and whether a paired companion may start agents.

## Security

Starting an agent runs code on a machine, so:

- In cloud projects, only **owner / admin / contributor** may start agents; **agents cannot start agents** (at least initially). Viewers cannot.
- A runner accepts requests only for projects and repositories its machine owner has allowed, and only from members that owner allows (default: only the machine owner).
- The runner never receives secrets from Allomos; it uses the credentials already on that machine.
- Every start and every open is recorded as an event (who, which runner, which agent, which task).
- Prompts are built from task fields; task text is data for the agent, and the agent keeps its own permission settings (Allomos does not bypass them).

## Unknowns to verify before building (research spike)

*Researched 2026-10-07* ([agent-launch-and-resume.md](../research/agent-launch-and-resume.md), from vendor documentation and the installed tools' help; no agent was run):

- **Launch and session ID — answered.** Claude Code: `claude -p … --output-format json` (`session_id`), or `claude --bg` for a background session listed by `claude agents --json`. Codex: `codex exec --json -C <folder>` (`thread_id` in the first event).
- **Resume — answered.** `claude --resume <id>` / `claude attach <id>`; `codex resume <id>` / `codex exec resume <id>`.
- **Link to a session — partly.** Codex desktop: `codex://threads/<id>`. Claude: cloud sessions have a `claude.ai/code/<id>` address; a link to an existing *local* Code session in the desktop app is not documented. `claude-cli://open?cwd=…&q=…` opens a *new* terminal session.
- **Create cloud sessions — answered.** `claude --cloud "<task>"` and `codex cloud exec --env <id> "<task>"`, both with the user's own sign-in and plan; no separate API needed.
- **Open a window from a background program — answered for Claude** (the `claude-cli://` handler picks the terminal); for Codex, the `codex://` link opens the app, and a terminal needs Allomos's own launcher.
- **Still to check by running:** the Codex rows (the local Codex install is broken), and Claude features newer than the installed 2.1.191 (`--cloud`, cross-folder `--resume`).

## Open questions for the owner

1. Which agent surfaces matter first: local CLI, desktop app, or cloud/web sessions?
2. Runners on your machine only, or also on teammates' machines? May a teammate start an agent on *your* runner?
3. Should starting a task also create the git worktree/branch automatically (matching your current `codex/…` / `claude/…` naming)?
4. Should Allomos choose model and effort per task (as in your `AI-WORKFLOW.md`), or leave that to the agent's defaults?
