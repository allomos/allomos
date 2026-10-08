# Task-A01-13-agent-instructions-on-init

*Card type: coding. Area A01 (Tracking), Iteration 1. Requested by the owner (2026-10-07) after asking how an AI agent in a new project knows to use FlipHopper.*

## What gets built (plain words)

An AI agent learns how to work in a project from the project's instruction files: Codex reads `AGENTS.md`, Claude Code reads `CLAUDE.md`. After this card, when `fh init` or the app creates a seabag, it also adds a short **"This project uses FlipHopper"** section to those files. Any agent that later opens the project then knows to run `fh next`, to claim with `fh claim … --push`, and so on.

| Piece | What it is | Where |
|---|---|---|
| The section | A few lines in English between `<!-- fliphopper:start -->` and `<!-- fliphopper:end -->` markers: tasks live in `.fliphopper/seabag/`; start with `fh next`; `fh claim <id> --as <your session> --push`; report with `fh label`, `fh wait`, `fh release --note`, `fh done`, each with `--push`; never edit the seabag's files by hand. | `crates/store` |
| `AGENTS.md` | Created with the section when missing; the section is appended when the file exists without it; left alone when the section is already there. Nothing already in the file is changed. | the project folder |
| `CLAUDE.md` | Created as one line, `@AGENTS.md` (Claude Code then reads AGENTS.md too), when missing. When it exists: left alone if it already imports `@AGENTS.md` or holds the section; otherwise the section is appended. | the project folder |
| `fh init` | Does this by default and lists the files it changed; `--no-agent-files` skips it. | `crates/cli` |
| The app | Does this when it creates a seabag, and the notice says which files it changed. The README for an empty folder (A01-05) is decided first, so these files do not stop it. | `apps/desktop`, `apps/web` |

**What you can see at the end:** run `fh init` in a new repository: `AGENTS.md` and `CLAUDE.md` appear, and a new Claude Code or Codex session there starts by running `fh next`.

## Details for the worker

- **Outcome.** An agent opening a project set up with FlipHopper uses `fh` without anyone telling it.
- **Owner's choices (2026-10-07).** The instruction files: yes, now. A start-of-session hook: no. With the MCP server (Iteration 3), `fh init` also switches the server on in `.mcp.json` and the server sends its own instructions: yes, then (recorded in [PLAN § 9](../product/PLAN.md#9-roadmap--releases), Iteration 3).
- **Decisive unknowns.** None.
- **Files owned.** `crates/store/src/lib.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/fh.rs`, `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/App.tsx`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `docs/workflow/CODING_RULES.md` (the `fh init` line), `docs/product/FILE-FORMAT.md` (files FlipHopper may write next to `.fliphopper/`), `docs/product/PLAN.md` (Iteration 3 line), this card.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - Tests: no files → both created; existing `AGENTS.md` → section appended once, earlier text kept; running twice adds nothing; `CLAUDE.md` with `@AGENTS.md` untouched; `--no-agent-files` writes neither; an empty folder in the app still gets its README.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-07 | "Now: accept this" (instruction files on init); "With MCP (Iteration 3): accept this"; "Optional now [hook]: no need this" | owner (chat) |

## Notes

- 2026-10-07 — Card written by the agent and worked in the card run (`claude/a01-run`).

## Result (2026-10-07)

**In plain words.** The existing `fh init` command and the app now set a project up for AI agents when they create its seabag. They add a short "Tasks: FlipHopper" section to the project's `AGENTS.md` (read by Codex), and make `CLAUDE.md` (read by Claude Code) point to it. The section says to start with `fh next` and to record every change with `fh … --push`. Existing text in those files is never changed; the section is added only once. `fh init --no-agent-files` skips it, and `fh init --commit` commits these files together with the new seabag. The app's notice names the files it changed.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (store: new rules test; command: new `init_tells_agents_about_fliphopper`, `init --commit` includes both files) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (82 texts) | PASS |
| Built app on a new empty folder: `.fliphopper/`, `README-fliphopper.md`, `AGENTS.md` (with the section), `CLAUDE.md` (`@AGENTS.md`); the window shows both notices | PASS |
| A new Claude Code or Codex session in such a project starts with `fh next` | not run (would start an agent) — owner check if wanted |
- 2026-10-08 — Superseded in part: `fh init` now asks the agent ([`A01-14`](./Task-A01-14-init-asks-the-agent.md)); the app asks the agents it finds ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)); the marker lines were removed at the owner's request. Direct writing remains for `fh init --write-agent-files` and the app's "Write the files for me".
- 2026-10-08 — **Closed** at the owner's word ("yes"): replaced by A01-14 and A03-03; its direct writing remains for `fh init --write-agent-files` and the panel's "Write the files for me", both owner checked.
