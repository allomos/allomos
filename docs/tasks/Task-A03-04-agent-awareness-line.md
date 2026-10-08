# Task-A03-04-agent-awareness-line

*Card type: coding. Area A03 (Agents). Requested by the owner (2026-10-08): the app should show whether the AI agents know about this project's seabag, without asking agents to write a flag.*

## What gets built (plain words)

One line under the window's top bar answers "do the AI agents know about this project?". FlipHopper works it out itself, from two things it can see:

1. **Are the instructions there?** It reads the project's `AGENTS.md` (read by Codex) and `CLAUDE.md` (read by Claude Code) and checks whether they tell agents about FlipHopper.
2. **Do agents use it?** It looks at who last changed the seabag with `fh` (the `Agent:` line that `fh --push` / `--commit` writes into each commit, and the holders of tasks in progress).

| State | What the line shows |
|---|---|
| Not set up | "AI agents: not told about this project yet" (and which agent is missing, e.g. "Claude Code ✗") → **Tell AI agents…** (opens the A03-03 panel) |
| Set up, not used yet | "AI agents: Codex ✓ Claude Code ✓ — no agent has used fh here yet" |
| In use | "AI agents: Codex ✓ Claude Code ✓ — last used by claude/a01-run, 14:02 today" |

| Piece | What it is | Where |
|---|---|---|
| The check | A file counts as telling agents about FlipHopper when a Markdown heading names FlipHopper, or its text mentions `fh next` or `fh claim`, or `.fliphopper/seabag`. Codex: `AGENTS.md` passes. Claude Code: `CLAUDE.md` passes, or it imports `@AGENTS.md` and `AGENTS.md` passes. The same check stops "Write the files for me" from adding a second copy. | `crates/store` |
| Last use | The newest commit touching the seabag whose message has an `Agent:` line (name and time), from `git log`; else a task's current holder. No Git: holders only. | `apps/desktop` (runs `git`, read-only) |
| The line | In the window, refreshed when the seabag or those files change (about every few seconds), in three languages. | `apps/web` |

**What you can see at the end:** open a project without the instructions: the line says so and offers the button; after an agent adds them, the line turns to ✓ within seconds; after an agent runs `fh … --push`, it shows who and when.

## Details for the worker

- **Outcome.** The owner sees at a glance whether AI agents know about a project and are using FlipHopper there.
- **Owner's choices (2026-10-08).** No flag written by agents; FlipHopper checks the files and the activity itself.
- **Decisive unknowns.** None.
- **Files owned.** `crates/store/src/lib.rs`, `apps/desktop/src-tauri/src/lib.rs` (or `agents.rs`), `apps/web/src/App.tsx` (or a small component), `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/web/src/styles.css`, this card.
- **External operations.** None.
- **Done means mechanically.** The usual Rust and web checks pass; tests cover the check (heading, `fh next` in other words, `.fliphopper/seabag`, CLAUDE.md importing AGENTS.md, none) and reading the last `Agent:` commit in a temporary Git repository; in the built app, this repository shows "in use" with the last agent, and a new folder shows "not set up". Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | "ok. yes" to the agent's recommendation: check the instruction files and show agent activity, no flag | owner (chat) |

## Notes

- 2026-10-08 — Card written by the agent and worked in the card run (`claude/a01-run`).

## Result (2026-10-08)

**In plain words.** Under the top bar, the window now says whether AI agents know about the open project, worked out by FlipHopper itself (no flag written by agents). It checks `AGENTS.md` (Codex) and `CLAUDE.md` (Claude Code, also through `@AGENTS.md`) about every 4 seconds, and reads the seabag's Git history for the newest commit with an `Agent:` line (written by `fh --push` / `--commit`); with no such commit it names whoever holds a task in progress. When an agent is missing, a **Tell AI agents…** button opens the A03-03 panel. The same check now stops "Write the files for me" from adding a second copy after an agent merged the rules in its own words.

- **The check:** a file tells agents about FlipHopper when a Markdown heading names FlipHopper, or it mentions `fh next` or `fh claim`, or `.fliphopper/seabag` (`tells_agents`, `agent_setup` in `crates/store`).
- **Where:** `agent_status` in `apps/desktop/src-tauri/src/lib.rs`; `last_agent_use` in `agents.rs` (runs `git log`, read-only); `AgentLine.tsx` (new); six texts in three languages.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (new: the check's cases, CLAUDE.md through AGENTS.md; parsing the log; a real temporary Git repository) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (105 texts) | PASS |
| Built app on this repository | PASS: "AI agents: Codex ✓ Claude Code ✓ — last used by claude/a01-run, <date>" |
| Built app on `~/fliphopper-check3` (no instructions); then an `AGENTS.md` mentioning `fh next` added by hand | PASS: "not told about this project yet", then within seconds "Codex ✓ Claude Code ✗ — no agent has used fh here yet" (test file removed afterwards) |
- 2026-10-08 — Owner: the Tell AI agents… button was not vertically aligned. Cause: link-style buttons were pinned to the top of their row (`align-self: start`) with a smaller font. Fixed for this line and the panel's Change… row (centred, same font size); measured in the browser pane: button and text middles equal (67/67, 351/351; before 26/29 and 309/350).
