# Task-A00-28-worktree-default

*Card type: docs. Area A00 (Planning). Written 2026-10-14 by `claude/a00-28-worktree-default`.*

## What gets built (plain words)

Until now the project's instructions told every agent session to work in its own git worktree (a second checkout of the repository in its own folder, on its own branch). That costs a full build folder per session even when only one session is working.

After this card the instructions say: a session works in the current checkout by default. A worktree is created only when a task needs its own checkout, for example when several sessions work at once; it then goes under `.claude/worktrees/<short-name>`, on a branch (`claude/…`, `codex/…`) cut from `main`.

## Details for the worker

- **Outcome.** An agent reading `AGENTS.md`, `docs/workflow/CARDS.md` or `docs/workflow/CODING_RULES.md` learns the default above, and no line says otherwise.
- **Files owned.** `AGENTS.md`, `docs/workflow/CARDS.md`, `docs/workflow/CODING_RULES.md`, this card.
- **External operations.** None.
- **Done means mechanically.** `grep -rni worktree` over `AGENTS.md`, `docs/workflow/` and `crates/` finds no line saying every session needs a worktree; links valid; `git diff --check` clean.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-14 — Changed (routine choices, owner rule A00-29):
  - `AGENTS.md` working procedure 2: "Several sessions may work at once, each in its own git worktree and branch …" → "A session works in the current checkout by default, holding one card. A worktree is created only when a task needs its own checkout, for example when several sessions work at once: each then has its own git worktree under `.claude/worktrees/<short-name>`, on its own branch (`claude/…`, `codex/…`) cut from `main`."
  - `docs/workflow/CARDS.md` claiming protocol: the same rule added to its opening paragraph; step 8 (clean up) now starts "If you worked in a worktree".
  - `docs/workflow/CODING_RULES.md` § Build output: "one `target/` per worktree" → "per checkout"; "Do not build in the main checkout; build in your card's worktree" → "Build in the checkout you work in: the main checkout by default, or your card's worktree when the task has one".
  - `alm guide` / `alm brief` texts (`crates/store/src/guide.rs`) and the handoff template say nothing about worktrees per session, so the brief and its version are unchanged. `docs/workflow/AI-WORKFLOW.md` only mentions worktrees as something to check or record when one exists; unchanged.
- 2026-10-14 — Checks: `grep -rni "worktree\|main checkout"` over `AGENTS.md`, `CLAUDE.md`, `docs/workflow/`, `crates/` — remaining lines are conditional ("when the task has one", clean-up, `git worktree list`) — PASS. Link target `CARDS.md#claiming-protocol` exists — PASS. `git diff --check` — PASS. No Rust changed; Rust checks NOT RUN (not needed).
