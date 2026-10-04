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
| 2026-10-04 | Framework: Tauri 2 | owner (chat) |
| 2026-10-04 | Release together; unsigned until first public release; a second language is acceptable | owner (chat) |

## Notes

- 2026-10-04 — **Owner chose Tauri 2.** 0011 accepted; R86 added; PLAN architecture rewritten (Rust crates and TypeScript interface); `Task-W01-01` rewritten for Rust.

- 2026-10-04 — At the owner's request, wrote a [detailed comparison](../research/desktop-frameworks.md) of Tauri 2, Electron, Electrobun 2, Wails 2, Neutralinojs and NW.js from current sources. The recommendation stays Tauri 2. New findings: Electrobun has no Intel Mac build; Tauri 3 is in alpha.

- 2026-10-04 — Owner: a second language is not a problem; release macOS and Windows together; unsigned builds until the first public release (R85). Compared four frameworks from current sources; the recommendation changed from Electron to **Tauri 2**. If chosen, `Task-W01-01` would be written in Rust.
