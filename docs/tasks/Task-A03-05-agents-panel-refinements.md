# Task-A03-05-agents-panel-refinements

*Card type: coding. Area A03 (Agents). Requested by the owner while checking [`Task-A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md) (2026-10-08).*

## What gets built (plain words)

Three changes to the "Tell AI agents about FlipHopper?" panel:

| Piece | What it is | Where |
|---|---|---|
| Progress | If looking for the AI agents takes longer than **200 ms**, the panel shows a small spinner and "Looking for AI agents…" until the list is ready. | `apps/web/src/AgentsPanel.tsx`, `styles.css`, `i18n/*` |
| One group per agent | Tools of the same agent are grouped: **Claude** (terminal, app) and **Codex** (terminal, app). Each group has its own tick box; inside a group the person picks **one** tool (round buttons), so the same agent is never asked twice. A group with one tool shows just that tool. | `AgentsPanel.tsx` |
| The request's language | The request typed into the agent follows the app's language (English, Simplified Chinese or Japanese): the steps and the note to several agents. The section the agent adds to `AGENTS.md` stays in English, so every agent and teammate reads the same rules. `fh init` (no app language) stays English. | `crates/store` (`agent_setup_request_in`), `apps/desktop` (`ask_agents` takes the language), `agents.rs` |

**What you can see at the end:** the panel shows "Claude" with Terminal / App to choose one, and "Codex" with App; Ask opens the chosen tool with the request in the app's language.

## Details for the worker

- **Outcome.** The panel is clearer to use and speaks the person's language.
- **Defaults chosen by the agent** (the owner may change them on review): the section added to `AGENTS.md` stays English; in each group the terminal tool is chosen first when both are found; only the first group is ticked at the start, as before.
- **Decisive unknowns.** None.
- **Files owned.** `apps/web/src/AgentsPanel.tsx`, `apps/web/src/styles.css`, `apps/web/src/messenger.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/agents.rs`, `crates/store/src/lib.rs`, this card.
- **External operations.** None.
- **Done means mechanically.** The usual Rust and web checks pass; tests cover the request in three languages (steps translated, section English, paths present) and the translated shared note; in the built app the groups show, one tool per group can be chosen, and the spinner appears when detection is slow (checked by delaying detection in a test build or by reading the code path). Recorded on the card; then the owner checks.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-08 | Owner check passed: spinner, grouped choice, request in the app's language ("passed") | owner (chat) |
| 2026-10-08 | "there should be a progress indicator if searching for the ai agents took more than 200 ms"; "group the ai tools of the same ai agent … user can only select one of them in one group at one time"; "use the corresponding language to the ui language" | owner (chat) |

## Notes

- 2026-10-08 — Card written by the agent from the owner's requests and worked in the card run (`claude/a01-run`).

## Result (2026-10-08)

**In plain words.** The panel now groups tools by agent: **Claude** (Claude Code in the terminal, the Claude app) and **Codex** (the Codex terminal tool, the Codex app). Each group has one tick box; inside it, round buttons choose exactly one tool, so the same agent is never asked twice (a group with one tool shows no round button). If finding the agents takes longer than 200 ms, a small spinner and "Looking for AI agents…" show until the list is ready. The request typed into the agent, and the note sent when several agents are asked, follow the app's language; the section the agent adds to `AGENTS.md` stays English (the request says so).

- **Found on the way:** detection took about 1.9 s, almost all of it reading every app's `Info.plist`. It now checks the known apps first (Claude.app, Claude Code URL Handler.app, Codex.app, ChatGPT.app) and scans everything only when none of them answers: 0.6–0.9 s on this Mac, the rest mostly the broken Codex CLI failing its check twice. The spinner still shows briefly.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (new: request in zh-CN / ja with the section in English; shared note in three languages) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (106 texts) | PASS |
| Built app (Chinese), new folder: group Claude ticked with Claude Code（终端） chosen and Claude（应用） as the other choice; group Codex unticked with Codex（应用） (ChatGPT) alone | PASS (read through macOS accessibility) |
| Detection time on this Mac | 1.8–1.95 s before; 0.6–0.9 s after |
| Spinner on screen; the request in Chinese / Japanese inside the agent | not seen by the agent — owner check |
- 2026-10-08 — **Owner verified** in the built app ("passed").
