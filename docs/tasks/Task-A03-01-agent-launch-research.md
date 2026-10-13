# Task-A03-01-agent-launch-research

- **Outcome.** The owner and the W07 worker know, from vendor documentation and local trials, exactly how Allomos can **start** a task in Claude Code and in Codex and later **jump into** that session — for each surface (CLI, desktop app, cloud/web) — so the runner design in [decision 0005](../decisions/0005-start-and-open-agent-sessions.md) can be confirmed or corrected.
- **Decisive unknowns** (worker finds out, from official docs and trying the installed tools):
  - How to launch each agent non-interactively with a prompt in a given directory, and get a **session id** back.
  - How to **resume** a given session (command, flags) and whether desktop/web apps offer a **link or URL** to a specific session.
  - Whether each vendor offers a supported interface to **create cloud sessions** programmatically, and its authentication and cost.
  - How a background process on macOS can open a terminal or app window for the user.
- **Files owned.** `docs/research/agent-launch-and-resume.md` (evidence: what was read and what actually ran), [decision 0005](../decisions/0005-start-and-open-agent-sessions.md) updates, this card.
- **External operations.** None beyond reading public documentation. Trials use the owner's locally installed CLIs in a throwaway directory, with no pushes, no cloud sessions created and no paid API calls unless separately approved.
- **Done means mechanically.** The research file has, per agent and surface: launch command, resume command, session-id source, link/URL support, cloud-creation support — each marked **verified locally**, **documented only**, or **not available**, with source links and dates; decision 0005's "Unknowns" section is updated to match.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Research read. Real trials (short prompts, after `brew reinstall codex`) are deferred: run them when iteration I08 starts, as part of its first card | owner (chat) |
| 2026-10-07 | "find some easy task to do for the next 6 hrs … find them and do them" — the agent picked this card (documentation and `--help` only; no agent runs) | owner (chat) |

## Notes
- 2026-10-07 — Done in the card run (`claude/a01-run`) from vendor documentation and the installed tools' `--help`. No agent was started: running even a short prompt uses the owner's plan, so the "verified locally" marks cover only what the help output shows.

## Result (2026-10-07)

**In plain words.** [agent-launch-and-resume.md](../research/agent-launch-and-resume.md) gives, for Claude Code and Codex and for each surface (terminal, desktop app, cloud), how Allomos can start a task, where the session ID comes from, how to reopen it, whether there is a link to it, and whether cloud sessions can be created. Both agents can be started by another program and report an ID that can be reopened later, so "Start in agent" and "Open session" need no screen automation. Decision 0005's unknowns are updated to match.

- **Found on the way:** the Homebrew Codex CLI on this Mac (0.130.0) does not run; its platform binary is missing. Claude Code is 2.1.191, older than some documented features (`--cloud`, resuming from another folder).

| Check | Result |
|---|---|
| Per agent and surface: launch, resume, session-ID source, link support, cloud creation, each marked verified locally / documented only / not available, with sources and dates | PASS: Claude Code 7 rows, Codex 5 rows; 9 sources read 2026-10-07 (one secondary, marked) |
| Decision 0005 "Unknowns" updated | PASS |
| Local trials | partly: `--help` only (no prompts sent); Codex could not run — listed under "To check before building" |
| `git diff --check`, relative links | PASS |
