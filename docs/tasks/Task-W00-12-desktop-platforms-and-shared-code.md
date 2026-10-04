# Task-W00-12-desktop-platforms-and-shared-code

- **Outcome.** The owner's requirements are recorded: the desktop app ships for **macOS and Windows**, and the desktop and browser apps **share the same interface source code**. Only the desktop adds native parts: Git, agent operations, MCP, the data folder, and the other necessary pieces. The framework choice is written up for the owner to decide.
- **Decisive unknowns.** The owner's choice of desktop framework: Electron (recommended) or Tauri.
- **Files owned.** [decisions/0011](../decisions/0011-desktop-framework-and-shared-code.md), REQUIREMENTS R81–R82, related edits to 0009, PLAN, the board and the discussion record.
- **External operations.** None.
- **Done means mechanically.** The requirements and decision record are written; links and tables check clean; pushed.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-04 | "Desktop app and browser app share the same source code, except the desktop app has special native code for git, agent operation, MCP and other necessary parts. Desktop app for macOS and Windows." | owner (chat) |
| 2026-10-04 | Release together; unsigned until first public release; a second language is acceptable | owner (chat) |

## Notes

- 2026-10-04 — Owner: a second language is not a problem; release macOS and Windows together; unsigned builds until the first public release (R85). Compared four frameworks from current sources; the recommendation changed from Electron to **Tauri 2**. If chosen, `Task-W01-01` would be written in Rust.
