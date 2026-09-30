# Glossary

Plain meanings of the words used in glidebox's documents and product.

| Term | Meaning |
|---|---|
| **Agent** | An AI coding assistant working on tasks — Claude Code or Codex. In glidebox also a project member type that acts through a token and cannot approve. |
| **Session** | One running conversation of an agent, usually in its own git worktree and branch (e.g. `claude/w01-core-model-3f2a`). |
| **Worktree** | A separate working folder of the same git repository, checked out on its own branch, so several sessions can work at once without disturbing each other. |
| **Card / task** | One unit of work with one observable outcome. In this repo a card is a file in `docs/tasks/`; in the product it is a task record. |
| **Workstream** | A group of related tasks (e.g. W01 core model). Tasks are shown and counted per workstream. |
| **State** | Where a task is in its life: backlog, ready, in progress, waiting, done, dropped. |
| **Label** | The last *proven* step of a task's progress, from a fixed ladder (e.g. implemented → locally verified → deployed and verified → owner verified). The only allowed progress claim. |
| **Claim** | A session or person taking a task, so no one else works on it at the same time. |
| **Waiting on** | What blocks a task's next step: an owner check, an approval, another task, or something external. |
| **Next dependency** | The single thing that must happen before a task can reach its next label. |
| **Approval** | The owner's explicit yes to one exact external operation (e.g. deploy to a named target). Approving a card covers only its local work. |
| **External operation** | Anything with an effect outside this machine's working copy: hosted databases, deployments, secrets, email, creating repositories, pushing to a new remote. |
| **Event** | An append-only record of one change (who, what, when). Burndown and timelines are computed from events. |
| **Burndown** | A chart of work remaining (and total scope) over time; where the line is heading suggests when work will finish. |
| **Throughput** | How many tasks were finished per week. |
| **Monte Carlo forecast** | Repeating a simulated future many times, drawing weekly throughput from past weeks, to get a range of likely finish dates (e.g. 50% and 85% confidence). |
| **Runner** | A small glidebox program on a person's machine that can start and reopen agent sessions when asked from the app. |
| **Handoff** | The short notes a session leaves so the next session can continue: checkout, uncommitted work, running processes, next action. Lives in `docs/AGENT_HANDOFF.md`. |
| **Owner** | will.w.hui — decides, approves, and verifies finished work. Also the top project role in the product. |
| **Glidelines** | The owner's earlier Gantt dashboard app (`glidepace-vibe-cc`), with a task-file format and burndown code glidebox may reuse. |
| **SSR board** | The owner's Joyns SSR migration task board — the real multi-agent project that motivated glidebox. |
