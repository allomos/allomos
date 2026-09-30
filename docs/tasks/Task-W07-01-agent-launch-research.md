# Task-W07-01-agent-launch-research

- **Outcome.** The owner and the W07 worker know, from vendor documentation and local trials, exactly how glidebox can **start** a task in Claude Code and in Codex and later **jump into** that session — for each surface (CLI, desktop app, cloud/web) — so the runner design in [decision 0005](../decisions/0005-start-and-open-agent-sessions.md) can be confirmed or corrected.
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

## Notes
