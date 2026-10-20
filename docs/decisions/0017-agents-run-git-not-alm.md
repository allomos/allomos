# 0017 — `alm` and the app never run Git; each change tells the agent the exact Git commands

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-14 | 2026-10-14 | will.w.hui |

Card: task `A01-38`. Accepted by the owner on 2026-10-14 ("accept", in chat with session claude/a01-38-no-git-in-alm); the code may start.

## In plain words

Today `alm` commits and pushes the task list by itself (`--commit`, `--push`), and the setup wizard runs `git init` and commits without asking. That makes Git happen behind the agent's back. On 2026-10-14 it went wrong: one session's `alm wait A07-03 --push` committed **every** changed file in the task list, so another agent's whole migration (154 files, waiting for the person's OK) and an unrelated new task were uploaded under the wrong name.

**Proposal:** Allomos stops running Git altogether. After each change, `alm` prints the exact Git commands for **that change's files only**, and the agent runs them, the same way it commits its code. This follows the project's rule "`alm` tells; the agent checks".

## Context

- `--commit` / `--push` on every changing command: `share()` in `crates/cli/src/lib.rs` runs `git add --all -- <taskbag>`, `git commit` ("alm: <command> <id>" plus an `Agent:` line) and `git push`. It commits the whole taskbag folder, not only what the command wrote.
- `alm init --commit` also commits the AGENTS.md / CLAUDE.md that `init` wrote. `alm init` reads Git to find the repository's top folder, where it puts the taskbag.
- The setup wizard (A05-03, `keep_taskbag_in_git` in `apps/desktop/src-tauri/src/lib.rs`) runs `git init` when the folder is not a repository, then commits the taskbag and `README-allomos.md` (step 2) and the `project_folder` change (step 3). It never pushes.
- The app's "last used by … at …" line (A03-04, `last_agent_use` in `agents.rs`) reads Git history for the newest `Agent:` line.
- Changes made in the app (task pane, new tasks, areas, iterations) are never committed; the next `alm … --push` sweeps them in.
- The AGENTS.md section (`alm brief`), `alm guide`, `alm`'s hints and the setup, conversion and migration prompts all mention `--push` / `--commit`.

## Decision (every row agreed with the owner in chat, 2026-10-14)

| # | Use case | Today | After |
|---|---|---|---|
| 1 | Everyday task changes (`label`, `wait`, `block`, `done`, `release`, `add task`, `ready`, `plan` …) | `alm` commits the whole taskbag folder; `--push` pushes | `alm` makes the change, then prints "Commit and push this change:" and the **exact `git add / commit / push` commands** for that change's files only, plus "commit only these files; leave other changes as they are". The agent runs them. `--json` gives `changed_files` and `git_commands`. |
| 2 | `alm claim` (so two sessions don't take one task) | automatic through `--push` | `alm claim` prints: **"Commit and push this change now, before you start the work. Until it is pushed, other sessions do not see your claim and may take this task too"**, then the commands. The AGENTS.md rule repeats it and says to pull before claiming. |
| 3 | `alm init --commit` (also AGENTS.md / CLAUDE.md) | commits them | **No Git and no Git commands printed**: no agent is connected yet. |
| 4 | Handoffs (`alm handoff`, `--remove`) | commit and push | As 1: the commands for the handoff file. |
| 5 | The `Agent:` line | written by `alm` | Part of the printed `git commit` command (`-m "Agent: <you>"`). Nothing depends on it any more (see 8). |
| — | `--commit` / `--push` | on every changing command | **Removed outright**, no transition period. |
| — | `alm init` finding the repository | reads Git | **Removed.** The taskbag goes in the current folder or where `--taskbag` says. `alm` never reads or runs Git. |
| 6 | Setup wizard step 2 | `git init` without asking; commits taskbag and README | **No Git.** The step 4 prompt to the connected agent gains a step: "if this folder isn't in Git, ask the person whether to set it up; then commit the taskbag and README". |
| 7 | Setup wizard step 3 (`project_folder` in `project.json`) | commits that change | **No Git.** Committed by the agent in step 4, with 6. |
| 8 | "Last used by … at …" in the app | reads Git for the `Agent:` line | **Reads the change history** (`events/`: who and when for every change). Works without commits and covers app changes. |
| 9 | Changes made in the app | written, never committed; swept in by the next `--push` | **Now:** the app shows "N changes not shared yet", and the next agent's `alm` output lists them with the commands to commit them as the person's. **Later:** an **Ask an agent to share…** button. |
| 10 | AGENTS.md section (`alm brief`), `alm guide` | `--push` on every command | No `--push`. Rule: **pull before claiming; after every `alm` change, run the Git commands it prints**; a claim is pushed before the work starts. |
| 11 | Hints `alm` prints (`unblock`, `checked`, handoff clean-up) | include `--push` | No `--push`; each change prints its own commands. |
| 12 | A03-08, "each step tells the agent what to commit" | the setup request names what to commit | Kept, and it becomes the general rule. |
| 13 | Conversion prompt (A05-02) | never `--push`; commit only when asked | **When the conversion is done, the agent commits** the taskbag, the AGENTS.md change and the report. **Commit only; no push.** |
| 14 | Migration prompt (A01-32) | no `--commit` / `--push`; commit only when asked | **When `alm migrate --finish` succeeds, the agent commits** the taskbag's files. **Commit only; no push.** |
| 15 | "Migrate the taskbag…" in the app | commit those files; push only if asked | As 14. |

## Alternatives considered

| Option | Why not |
|---|---|
| Keep `--push`, but commit only the files the command wrote | Fixes the sweeping, but Git still runs behind the agent's back, and the wizard still runs `git init` unasked. |
| Keep `--commit` / `--push` for a transition version with a notice | Owner: no need. |
| `alm` runs Git, but only after asking | `alm` cannot ask; the agent can. Printing the commands gives the same control with less machinery. |

## Consequences

- **A claim reaches others only if the agent runs the printed commands.** Row 2 says so at the moment it matters; `alm` cannot force it.
- **Every existing AGENTS.md section** that says `--push` must be renewed (the section's version goes up, so `alm` and the app ask for it), including this repository's.
- **Tests** that check commits made by `alm` and the wizard are replaced by tests of the printed files and commands (exactly the command's files, nothing else; claim's wording; no commands for `init`).
- **The wizard** no longer needs Git to finish; a folder outside Git works until the agent sets it up.
- **Exception: a quiet goal run** (approver, 2026-10-19, task `A03-35`). In **Build a new app…** with quiet mode, the agent runs `git init` itself when the folder is not a repository, and records it as an AI decision on the goal task, instead of asking (row 6). Connecting a project and normal goal runs still ask first.
- **The "last used by" line** depends on the change history instead of Git.
- **Errors** `commit_failed` / `push_failed` / `git_init_failed` / `git_missing` disappear from `alm` and the wizard.

## Open for the owner

None known; the rows above are the answers given in chat on 2026-10-14. Accepting this record approves the code.
