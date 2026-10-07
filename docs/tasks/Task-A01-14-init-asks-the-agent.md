# Task-A01-14-init-asks-the-agent

*Card type: coding. Area A01 (Tracking), Iteration 1. Changes [`Task-A01-13`](./Task-A01-13-agent-instructions-on-init.md) at the owner's request (2026-10-07).*

## What gets built (plain words)

`fh init` is usually run by an AI agent. Instead of writing `AGENTS.md` and `CLAUDE.md` itself, it now **asks that agent to do it**, by printing clear numbered steps and the exact section to add. The agent can then merge the section with any task-tracking rules already in the file, so the file does not end up with two conflicting rules.

| Piece | What it is | Where |
|---|---|---|
| The request | Numbered steps plus the section: open or create `AGENTS.md`; add or replace the section between the FlipHopper markers; point any older task-tracking rule to FlipHopper; make `CLAUDE.md` contain `@AGENTS.md`; show the user the changes. One text, used by `fh init` and later by the app ([`Task-A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)). | `crates/store` |
| `fh init` | Prints the request after creating the seabag (in `--json`: the field `agent_setup`). It writes neither file. | `crates/cli` |
| `--write-agent-files` | Keeps A01-13's direct writing, for scripts with no agent. | `crates/cli` |
| The app | Unchanged for now: it still writes the files directly, until A03-03 lets it ask an agent instead. | — |

**What you can see at the end:** an agent runs `fh init`; its output ends with the request; the agent then edits `AGENTS.md` and `CLAUDE.md` and shows you the change.

## Details for the worker

- **Outcome.** An agent setting up FlipHopper in a project also adds the instructions for future agents, merged with what the project already says.
- **Decisive unknowns.** None.
- **Files owned.** `crates/store/src/lib.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/fh.rs`, `docs/workflow/CODING_RULES.md`, `docs/product/FILE-FORMAT.md`, this card.
- **External operations.** None.
- **Done means mechanically.** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass. Tests: `fh init` writes no `AGENTS.md` / `CLAUDE.md` and prints the request (text and `agent_setup`); `--write-agent-files` writes them as before.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "directly write these files? what about tell the agent to do it, then we can prevent duplicated contents writing?" — the agent's proposal (fh init asks; the app decided separately) | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent and worked in the card run (`claude/a01-run`).

## Result (2026-10-07)

**In plain words.** `fh init` no longer writes `AGENTS.md` or `CLAUDE.md`. After creating the seabag it prints "Next step for the AI agent running this command:", five numbered steps and the exact section. The agent reads that output and edits the files itself, merging with any rules already there. `fh init --write-agent-files` still writes the section directly, for scripts. The app still writes directly until [`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md).

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (new: `init_asks_the_agent_or_writes_when_told`; store: `agent_setup_request_carries_the_section_and_steps`) | PASS |
| `fh init` in a new folder: prints the request; no `AGENTS.md` / `CLAUDE.md` written | PASS |
| An agent following the request | not run (would start an agent) — owner check if wanted |
- 2026-10-07 — Owner review: step 2 must not rely on the markers ("it is possible has this section but no markers"), and the request should name the seabag. Changed: step 2 now says "FlipHopper instructions (with or without the marker lines) … replace them", and the request starts with the absolute **project folder** and **seabag** paths. Tests updated; PASS.
