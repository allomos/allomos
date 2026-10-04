# 0005 — Start a task in an AI agent, and jump into it, from glidebox

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; design proposed** | 2026-09-27 | 2026-09-27 (requirement) | will.w.hui |

## Requirement (owner, 2026-09-27)

1. **Start** a task in a **specified AI agent** (e.g. Claude Code or Codex) from the glidebox app.
2. **Jump into** a task that has already started, in that agent's session.

This reverses the earlier non-goal "glidebox tracks work; it does not start it".

## The core problem

Glidebox is an online web app; the agents usually run **on a person's own machine** (Claude Code / Codex CLI or desktop app, inside a git worktree) or **in a vendor's cloud** (web sessions). A browser page cannot launch programs on a computer by itself. So glidebox needs a way to reach where the agent runs.

## Proposal

### Where an agent can run — "runners"

| Runner type | Start | Jump in |
|---|---|---|
| **Local runner** — a small glidebox daemon on a person's machine (part of the `gb` CLI: `gb runner`), registered to a project | The daemon receives a start request from glidebox (realtime subscription), creates a git worktree and branch for the task, and launches the chosen agent there with a prompt built from the task | Glidebox shows the session's resume command and, on the runner's own machine, asks the daemon to open it (terminal or desktop app) |
| **Cloud agent** — a vendor-hosted session (Claude Code on the web, Codex cloud, or an API-hosted agent) | Glidebox (or the runner) creates the cloud session through the vendor's supported interface | Open the session's web URL |

A person may register several runners (laptop, desktop, a shared build machine). When starting a task, the user picks **agent** (Claude / Codex), **runner** (where), and optionally model / effort.

### What happens on "Start"

1. User clicks **Start in agent** on a task → chooses agent, runner, repository and base branch.
2. Glidebox records a **dispatch** event and sends the request to the runner.
3. The runner creates the worktree/branch (named from the task id, e.g. `claude/W07-01-09-seeker-resume-list`), writes the task brief (outcome, done-when, links, approval limits) into the prompt, and starts the agent.
4. The runner reports back the **session id**, worktree path, branch and how to resume; glidebox **claims** the task for that session automatically.
5. The agent then reports progress through the `gb` CLI / MCP, as for any session.

### What happens on "Open session"

- Cloud session → open its URL in a new tab.
- Local session on this machine → the runner opens it (e.g. a terminal in the worktree running the agent's resume command).
- Local session on another machine → glidebox shows where it runs and the exact command to resume there; it cannot open it remotely.

### Session status

The runner reports liveness (running / waiting for input / exited) so the task view can show whether its agent is still working. This also covers the reference project's "session registry".

## Local projects (added 2026-10-04)

Under [0006](./0006-local-and-cloud-projects.md), a local project has no accounts, so the role rules below apply to **cloud projects**. In a local project, the person using the desktop app ([0009](./0009-desktop-hub-with-web-and-mobile-companions.md)) is the authority on that machine. The desktop app is also the natural home of the local runner. Two things remain open: how a local project records who did what, and whether a paired companion may start agents.

## Security

Starting an agent runs code on a machine, so:

- In cloud projects, only **owner / admin / contributor** may start agents; **agents cannot start agents** (at least initially). Viewers cannot.
- A runner accepts requests only for projects and repositories its machine owner has allowed, and only from members that owner allows (default: only the machine owner).
- The runner never receives secrets from glidebox; it uses the credentials already on that machine.
- Every start and every open is recorded as an event (who, which runner, which agent, which task).
- Prompts are built from task fields; task text is data for the agent, and the agent keeps its own permission settings (glidebox does not bypass them).

## Unknowns to verify before building (research spike)

- The exact, supported ways to **launch and resume** each agent non-interactively and to obtain a **session id**: Claude Code (CLI, desktop app, web) and Codex (CLI, desktop / IDE, cloud).
- Whether each agent's desktop app or web version offers a **link or URL** to open a specific session.
- Whether each vendor offers an interface to **create cloud sessions** programmatically, and its authentication and cost model.
- How to open a terminal/app window reliably on macOS (and later Windows/Linux) from a background daemon.

## Open questions for the owner

1. Which agent surfaces matter first: local CLI, desktop app, or cloud/web sessions?
2. Runners on your machine only, or also on teammates' machines? May a teammate start an agent on *your* runner?
3. Should starting a task also create the git worktree/branch automatically (matching your current `codex/…` / `claude/…` naming)?
4. Should glidebox choose model and effort per task (as in your `AI-WORKFLOW.md`), or leave that to the agent's defaults?
