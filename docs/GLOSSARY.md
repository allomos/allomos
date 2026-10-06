# Glossary

Plain meanings of the words used in FlipHopper's documents and product.

| Term | Meaning |
|---|---|
| **Agent** | An AI coding assistant working on tasks — Claude Code or Codex. In FlipHopper also a project member type that acts through a token and cannot approve. |
| **Session** | One running conversation of an agent, usually in its own git worktree and branch (e.g. `claude/w01-core-model-3f2a`). |
| **Worktree** | A separate working folder of the same git repository, checked out on its own branch, so several sessions can work at once without disturbing each other. |
| **Card / task** | One unit of work with one observable outcome. In this repo a card is a file in `docs/tasks/`; in the product it is a task record. |
| **Area** | *What* part of the product a task belongs to, named after what users can do (FlipHopper's own: A00 Planning … A07 Platform & release; [CARDS](./workflow/CARDS.md#areas)). Each task has exactly one area. Formerly "workstream" — the SSR project's word, which FlipHopper maps to area. Other tools say epic, component or track. Renameable per project (R90). |
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
| **Runner** | A small FlipHopper program on a person's machine that can start and reopen agent sessions when asked from the app. |
| **Handoff** | The short notes a session leaves so the next session can continue: checkout, uncommitted work, running processes, next action. Lives in `docs/AGENT_HANDOFF.md`. |
| **Owner** | will.w.hui — decides, approves, and verifies finished work. Also the top project role in the product. |
| **Glidelines** | The owner's earlier Gantt dashboard app (`glidepace-vibe-cc`), with a task-file format and burndown code. FlipHopper is a separate product that learns from its ideas ([0003](./decisions/0003-relationship-to-glidelines.md)). |
| **SSR board** | The owner's Joyns SSR migration task board — the real multi-agent project that motivated FlipHopper. |
| **Local project** | A project stored only on the user's machine, used without signing in ([0006](./decisions/0006-local-and-cloud-projects.md)). |
| **Cloud project** | A project stored on a hosted service, which needs sign-in; used for team collaboration. |
| **Desktop hub** | The FlipHopper desktop app: it has every feature, stores local projects, and serves the companions ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). |
| **Companion** | The web or mobile app that connects to a running desktop hub to view its projects, including progress and status. |
| **Pairing** | Allowing a specific phone or browser to connect to your desktop hub: the companion gives an email and alias, the desktop user approves, and a long code sent by email grants access. |
| **CLI** | Command-line interface: commands typed in a terminal, used by people, scripts and agents. FlipHopper's is the **`fliphopper`** command, short alias **`fh`** (R88); for example `fh init` sets up a seabag. |
| **MCP** | Model Context Protocol: a standard way for AI agents to discover and call a program's tools. |
| **Command / query** | A *command* asks to change something; a *query* only reads. All interfaces use the same commands and queries. |
| **Revision** | A version number of a task. An update says which revision it was based on, so stale edits are caught instead of silently overwriting newer ones. |
| **Idempotent** | Safe to repeat: sending the same request twice has the same effect as sending it once. |
| **Push / pull / merge** | Git words: send your changes to a shared copy, fetch others' changes, and combine two sets of changes ([0007](./decisions/0007-git-like-project-sync.md)). |
| **Three-way merge** | Combining two edited versions by comparing both with the version they started from. |
| **Fork** | Copying another project's code to build on it. Ruled out for FlipHopper by [0008](./decisions/0008-build-our-own.md). |
| **OPFS** | Origin private file system: browser storage that belongs to one website, is hidden from the user's folders, and is lost when site data is cleared. |
| **Seabag** | Our name for the folder that holds one project's tasks and history (R87): like a sailor's bag holding everything they own, carried from ship to ship, a project's belongings are carried from agent to agent and person to person. Chinese **水手袋**, Japanese **シーバッグ**. The user picks a folder ([0010](./decisions/0010-project-data-folder.md)); the seabag always goes in `.fliphopper/seabag/` inside it, created by the app or `fh init`. Formerly "task box" (renamed 2026-10-06, [0013](./decisions/0013-product-name-fliphopper.md)). |
| **Electron / Tauri / Electrobun / Wails** | Frameworks for building desktop apps from a web interface. Their native sides are TypeScript (Electron, Electrobun), Rust (Tauri) or Go (Wails). See [0011](./decisions/0011-desktop-framework-and-shared-code.md). |
| **Code signing** | A digital signature that lets macOS and Windows confirm who published an app, so it installs without security warnings. Usually paid. |
| **Iteration** | *When* work is planned: one round of building that ends in a usable release. A task has one area and is planned into at most one iteration (R89). Other tools call this a sprint, cycle, milestone or release. FlipHopper's own: Iteration 1 = v0.1 (macOS) … Iteration 14 = v0.14, Iteration 15 = v1.0 ([PLAN § 9](./product/PLAN.md#9-roadmap--releases)). |
