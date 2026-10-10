# Task-A05-02-convert-existing-project

*Card type: coding. Area A05 (Project setup & import). Requested by the owner (2026-10-08): the conversion of an existing project into a standard FlipHopper project is **a prompt given to an AI agent the person chooses**, and the app includes that prompt when opening or setting up a project folder that needs it. Studied on the SSR migration plan (`next-ssr/docs/migration-plan-ssr/`) and its repository-root `AGENTS.md`, read only.*

## What gets built (plain words)

Today, when a person sets up a folder, FlipHopper creates an empty seabag and then offers to ask their AI agents to add FlipHopper's instructions ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md), [`A03-05`](./Task-A03-05-agents-panel-refinements.md)). This card adds a second kind of request: **"move this project's tasks into FlipHopper"**.

When the folder the person opens or sets up **already tracks its tasks another way** — a Markdown task board, card files, agent rules about claiming and status — the panel says so and offers to ask **one** AI agent the person picks to do the conversion. FlipHopper does not read or convert the board itself; the agent does, following FlipHopper's conversion prompt, and writes the seabag only through the `fh` command, so every change passes FlipHopper's own checks.

| Piece | What it is | Where |
|---|---|---|
| Noticing | When a folder is opened or set up, a quick look for signs of another task system: a board file (`TASK_BOARD.md`, `*board*.md`, `TODO.md`), card files (`Task-*.md`, `docs/tasks/`), or an `AGENTS.md` / `CLAUDE.md` that talks about claiming, a session registry or task status. Names of what it found, nothing more. | `crates/store` (a `find_task_system` function), called from `apps/desktop/src-tauri/src/lib.rs` |
| When it is offered | (a) setting up a folder that has no seabag but shows those signs — FlipHopper then **does not create an empty seabag first**; the agent creates it; (b) opening a folder whose seabag is empty while those signs are there; (c) any time, from **File → Move Tasks Into FlipHopper…** | `lib.rs`, `apps/web/src/App.tsx` |
| The panel | The same agents panel, with the title "This project already tracks its tasks — move them into FlipHopper?", the list of what was found, and **one** agent to choose (two agents converting at once would add every task twice). The person may switch to the plain setup request instead. | `apps/web/src/AgentsPanel.tsx`, `i18n/*` |
| The conversion prompt | A versioned text, in the app's language, built like the setup request (`agent_setup_request_in`): the project folder, the seabag path, what was found, the steps below, and FlipHopper's agent section | `crates/store/src/lib.rs` (`agent_convert_request_in`) |
| Asking | Exactly as today: the chosen agent opens in the project folder with the prompt typed in; nothing runs until the person presses Enter | `agents.rs` (unchanged) |
| Seeing it land | The app already watches the folder; when the seabag appears and fills, the window shows the tasks | existing awareness line and reload |

**What you can see at the end:** open the SSR project folder (or a test copy of one) in the app; the panel says it found `TASK_BOARD.md`, 101 card files and claiming rules in `AGENTS.md`; pick Claude Code, press Ask, press Enter in the terminal; the agent first shows its plan and asks a few questions, then fills the seabag with `fh`; the app shows every card in its area with its state, label and holder; the agent shows the changes to `AGENTS.md` and a short report, and commits nothing until asked.

## The conversion prompt — what it asks the agent to do

Plain numbered steps any agent can follow, in the spirit of the setup request:

1. **Look first, change nothing.** Find where this project keeps its tasks, their status, who holds each one, their labels, areas and next steps, and which rules in `AGENTS.md` / `CLAUDE.md` describe them. Do not run the project's scripts or hooks.
2. **Show the plan and ask.** Tell the person, briefly: how the areas and task IDs will map (keep the project's own IDs, dropping only prefixes like `Task-` and the trailing description); which words become the **label ladder** (keep the project's own, in order); which words mean *ready*, *waiting*, *done*, *dropped*; what to do with planned work that has no card yet; and anything ambiguous. **Wait for the person's answer.**
3. **Work on a new branch** (`fliphopper/convert` unless the person names one). Never use `--push` during the conversion; commit only when the person asks.
4. **Create the seabag with `fh`:** `fh init --id … --labels …` with the agreed ladder, then `fh add area` for each area and `fh add task` for each task, with a description that links to its card file (or to `file.md#section` when a card is a section of a bigger file). **Never write the seabag's files by hand.**
5. **Copy status exactly as the files say it**, with `fh claim --as <holder>`, `fh label`, `fh wait --on`, `fh needs`, `fh ready`, `fh done`, `fh drop`. Copy only what is written: an unknown state, date or holder stays unknown and goes in the report; never invent progress, verification or approvals. Approvals and status prose stay in the card files.
6. **Write a short report** (`docs/fliphopper-conversion.md`): every card file and the task ID it became, nothing dropped or duplicated; every value that could not be copied, with file and line; planned work left out. Run `fh list --json` and confirm it reports no issues.
7. **Switch the instructions over.** In `AGENTS.md`, replace the rules about where status lives, how to claim and release, session registries and regenerating the board with FlipHopper's section (given below); **keep every other rule word for word**. Make sure `CLAUDE.md` contains `@AGENTS.md`. Show the diff.
8. **List, do not do, the cut-over.** Name what still holds the old status — status tables in card files, generated board sections, board-generator scripts, hooks and CI checks that enforce them — and ask the person whether and when to remove them. Leave them in place unless the person says so.
9. **Tell the person what changed** and what waits on them.

The prompt carries the same FlipHopper section that the setup request adds to `AGENTS.md` (`agent_section`), so both requests end in identical instructions.

## How this fits the SSR project (read 2026-10-08)

The prompt has to work on the real case without special code for it. What the agent will meet there, and where each lands:

| In the SSR project | Becomes |
|---|---|
| Areas `A00`–`A15`, one file each (`areas/A09-enterprise-ui.md`) | `fh add area A09 "Enterprise UI"` |
| 101 card files `areas/Task-A…-….md`; a few older cards are sections of an area file; IDs up to three levels (`A07-01-08`) | `fh add task A07-01-08 …` with a link to the card file or section |
| Each card's board-fields table (State, Done, Priority, Depends on, Waiting on, Feature) and status table (Label, Session, Next dependency) | state, label, holder, waiting on, next dependency; Done date, Priority, Depends on, Feature go in the report until FlipHopper can hold them (see Gaps) |
| Ladder: implemented → locally verified → connected → deployed and verified → owner verified | the seabag's ladder (`fh init --labels`) |
| Session registry in `TASK_BOARD.md`; `check-board.mjs` generating board sections, enforced by a pre-commit hook and CI | holders on the tasks; the generator, hook and CI check listed as cut-over steps for the owner |
| `AGENTS.md` § Working procedure: "Status has one home: the card", claiming protocol, card file rules | the status and claiming parts replaced by FlipHopper's section; the boundaries, five gates, card runs, reading rules and verification language kept |

Two traps the agent will have to report rather than guess: the board's Conventions block still says `Task-W{…}` while every file is `Task-A{…}`, and some board text still says `workstreams/` where the folder is now `areas/`.

## Gaps in `fh` this depends on (each a small card of its own in A01)

The agent can only copy what `fh` can record. Today it cannot record:

- **a done date** (`fh done --on <date>`): the SSR project has about 80 done cards with dates, which the burndown and forecast ([`A02-01`](./Task-A02-01-burndown-and-forecast.md)) need;
- **dependencies** between tasks (`Depends on`);
- **changing the ladder after `fh init`**, for a folder whose seabag already exists (case b above);
- optionally **priority** and **feature** (owner: fields, or description text).

Until they exist, the prompt tells the agent to put those values in the report, so nothing is lost and a later run can fill them in.

## Details for the worker

- **Outcome.** A person whose project already tracks tasks another way picks one AI agent in FlipHopper, presses Ask, and that agent moves the tasks into a seabag through `fh` and switches the project's agent instructions to FlipHopper, after showing its plan and asking.
- **Defaults chosen by the agent** (the owner may change them on review):
  1. One agent per conversion (the panel lets the person pick only one).
  2. Setting up a folder with signs of another task system does **not** create an empty seabag first; the agent creates it with the agreed ladder.
  3. The agent shows its plan and waits for the person before writing anything.
  4. The conversion happens on a new branch and nothing is pushed; old status tables, generators and hooks stay until the person asks.
  5. The prompt's steps follow the app's language; FlipHopper's section stays English (as in A03-05).
- **Decisive unknowns.**
  - Which signs reliably mean "another task system" without false alarms on ordinary repositories (a `TODO.md` alone may be too weak) — the worker tries the rule on this repository, Glidelines, the SSR project and a few plain repositories, all read only.
  - Whether a real agent follows the prompt on a project the size of SSR in one session (about 100 cards, several hundred `fh` calls) — worker, on a copy.
- **Files owned.** `crates/store/src/convert.rs` (new: the noticing function and the conversion prompt, with tests) and its line in `crates/store/src/lib.rs`, `crates/core/src/problem.rs` (one new code, `agent_not_found`), `apps/desktop/src-tauri/src/lib.rs` (when to offer; the menu item), `apps/web/src/AgentsPanel.tsx`, `apps/web/src/App.tsx`, `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, a synthetic test board under `crates/store/tests/fixtures/`, this card. [`A05-01`](./Task-A05-01-ssr-board-import.md) (a built-in SSR importer) is superseded by this card and should be dropped when the owner accepts it.
- **External operations.** None by FlipHopper. Trying the prompt on the **real SSR repository** writes a branch and files there, which is read-only to FlipHopper's agents — it needs the owner's exact approval; otherwise the trial runs on a copy in a scratch folder.
- **Done means mechanically.** The usual Rust and web checks pass; unit tests cover noticing (a synthetic board, a plain repository, an empty folder) and the prompt in three languages (folder, seabag path and findings present; section English); in the built app, setting up the synthetic board folder offers the conversion with one agent to choose, and Ask opens it with the prompt typed in; one real agent run on the synthetic board (and, if approved, a copy of SSR) ends with `fh list --json` reporting no issues and a report that accounts for every card.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Open-folder flow decisions accepted ("ok", "yes"): empty new seabag; "Not now" holds until the folder is opened again; Change… re-records the project folder; the AGENTS.md section names the seabag when it is outside the project folder; `fh labels`; conversion request version 3 | owner (chat) |
| 2026-10-09 | Open-folder flow specified (now [OPEN-FOLDER-FLOW.md](../product/OPEN-FOLDER-FLOW.md)); the popup lives in the app; the app no longer looks for boards | owner (chat) |
| 2026-10-08 | Card approved; "drop A05-01 and implement it in a work tree" | owner (chat) |
| 2026-10-08 | Direction: "i want this feature as a prompt to ai agent. user can choose an ai agent to do the transformation. and when openning/setting the project folder, the prompt should also be included in when needed." (replaces the first draft, a built-in converter) | owner (chat) |

## Notes

- 2026-10-08 — First draft proposed a built-in scan-and-convert feature; the owner chose an agent prompt instead. Rewritten the same day.
- 2026-10-08 — Read only, to write this card: `next-ssr/docs/migration-plan-ssr/` (TASK_BOARD.md, areas/, a sample card, an area file) and the SSR repository's `AGENTS.md`. 101 card files in `areas/`, 3 cards claimed, board sections generated by `check-board.mjs`. Recount at implementation time; never copy private board text into fixtures.

## Result (2026-10-08)

**In plain words.** When you set up a folder that already keeps its tasks another way, FlipHopper now notices this. The "No seabag in this folder yet" card lists what it found (the task board, how many card files and where, agent rules about claiming) and offers **Ask an AI agent to move them…** next to the usual "Create seabag". The agents panel then lets you pick **one** agent and opens it with the conversion request typed in. While the agent works, the window shows "Waiting for the AI agent" and opens the project when the seabag appears. The same offer appears when an open seabag has no tasks yet, and any time from **File → Move Tasks Into FlipHopper…**.

- **Noticing** (`find_task_system`): board files (`TASK_BOARD.md`, `task-board.md`, `BOARD.md`, `KANBAN.md`, `TASKS.md`); card files named `Task-<ID with a digit>….md` (so `TASK_REORDERING.md` is a note, not a card); `AGENTS.md` / `CLAUDE.md` that mention a task board, claiming protocol or session registry. Offered when there is a board or at least two cards. Hidden folders (including `.claude/worktrees` copies), `node_modules`, `target`, `build` and similar are skipped, at most 6 levels and 50,000 entries.
- **The request** (`agent_convert_request_in`, version 1, in English, Chinese and Japanese): the nine steps in this card, with every `fh` command run with `--seabag "<path>"`. FlipHopper's agent section stays English at the end.
- **Decisive unknowns answered.** False alarms: tried read only on the SSR repository (1 board, 101 cards, `AGENTS.md`, about 0.5 s), FlipHopper itself, Glidelines (13 `TASK_<n>_PLAN.md` files: offered, since they are per-task plans) and six plain projects (not offered, 0–45 ms). A real agent follows the request: see the trial below. The SSR-sized run was not tried, because it needs a copy of a private repository; the synthetic run shows the shape.

| Check | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (new: noticing a board, a plain project, the fixture, names; the request in three languages, with and without a seabag) | PASS |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n` (137 texts, 42 Rust codes) | PASS |
| **Real agent trial** — Claude Code, request version 1 in English, on a Git copy of `crates/store/tests/fixtures/markdown-board` in a scratch folder. Turn 1: it changed nothing, showed a mapping table and asked 5 questions. Turn 2 ("go", plus one change): branch `fliphopper/convert`, seabag with the project's ladder, 3 areas, 5 tasks (done, held by `claude/order-form` with label, waiting with next dependency, ready, backlog "no card yet"), report `docs/fliphopper-conversion.md`, `AGENTS.md` status and claiming rules replaced and boundaries kept, cut-over listed and left in place, nothing pushed. `fh list --json`: no issues. Task links resolve to the card files (one to an area-file section). | PASS |
| The panel in the built app | owner check |

## Change (2026-10-09, owner): the agent finds the tasks, and asks in the app before moving

**Owner's request:** "prompt no task here, and user can use one of his agents to detect the tasks and move. before moving, there will be a confirm popup, which is showed up by the agent to call the fh command. user can choose cancel in that popup." Answers: the popup is in the FlipHopper app; the app no longer looks for a board.

**In plain words.** A folder without a seabag now says **No tasks here yet**, with **Ask an AI agent to find and move tasks…** and **Create an empty seabag**. FlipHopper no longer looks for boards or card files itself; the agent does. Before the agent changes anything, it writes its plan to a file and runs `fh confirm`: the FlipHopper window shows the plan in a popup with **Go ahead** and **Cancel**, and `fh confirm` tells the agent the answer (exit 0, or exit 1 `confirm_declined`). If no window shows the question within 3 seconds, `fh` opens the app on the project; with no answer in 10 minutes it gives up (`confirm_timeout`) and the popup disappears.

- **Removed:** `find_task_system` and the "what was found" lists; the request no longer lists findings.
- **Added:** `crates/store/src/confirm.rs` (questions in `<project>/.fliphopper/requests/`, ignored by Git, each with a time limit), `fh confirm` in `crates/cli`, the desktop commands `confirm_requests` and `answer_confirm_request`, `apps/web/src/ConfirmDialog.tsx`, problem codes `confirm_declined`, `confirm_timeout`, `request_not_found`; the request is **version 2**.

| Check (2026-10-09) | Result |
|---|---|
| `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` (new: `fh confirm` answered Go ahead, Cancel and not at all; an unreadable plan; questions asked, seen, answered, expired and withdrawn; request v2 in three languages) | PASS |
| Web `typecheck`, `build`, `check:i18n` (155 texts, 47 codes) | PASS |
| The built app on `~/fliphopper-tests/a05-02-bakery` saw a test question from `fh confirm` within 2.5 s (it marked it seen); the question timed out as asked | PASS |
| The popup and the whole flow on screen | owner check |

## Change (2026-10-09, owner): the open-folder flow

**Owner's request:** a FlipHopper folder has a corresponding project folder; opening a folder walks: initialize the seabag → find or ask for the project folder → check the agent instructions (offer an agent to add FlipHopper's section) → if the seabag has no tasks, offer an agent to find existing tasks and, after the person agrees, move them with `fh`; the migration step is also in the File menu, only when the project folder is specified. Recorded as the contract [OPEN-FOLDER-FLOW.md](../product/OPEN-FOLDER-FLOW.md).

**In plain words.** The app no longer hands the seabag's creation to the agent. Opening a folder now goes step by step: FlipHopper creates an **empty** seabag (as before, asking first when the folder has other files); finds the project folder (the folder itself when `AGENTS.md` or `CLAUDE.md` is there, otherwise it asks "Which folder holds this project?") and records it in `project.json`; opens "Tell AI agents about FlipHopper?" if the instructions lack FlipHopper's section; and, if the seabag has no tasks, opens "Find this project's tasks and move them into FlipHopper?". **Not now** on either panel holds until the folder is opened again.

- **Decisions made by the agent, accepted by the owner 2026-10-09:** the app's new seabag is empty (example tasks would always count as "has tasks", so the migration step would never run); the project folder is stored relative to the FlipHopper folder (`"."`, `"../app"`); **Change…** in the agents panel re-records it; the section added to `AGENTS.md` names the seabag's place when it is not inside the project folder.
- **Added:** `project_folder` in `project.json` (core model), `crates/store/src/setup.rs` (`setup_state`, `record_project_folder`, `relative_path`), `fh labels` (the agent sets the project's own ladder on the existing seabag), desktop commands `project_setup`, `set_project_folder`, `set_convert_enabled`; the request is **version 3** (no `fh init`; card links computed from the recorded project folder).

## Change (2026-10-10, owner): the section comes from `fh brief`, details from `fh guide`

**Owner's request:** instruct the agent to get the brief from the FlipHopper command and write it into the FlipHopper section; the brief tells the agent to get details from `fh` when needed; tell agents these are the terms FlipHopper relies on.

**In plain words.** The FlipHopper section in `AGENTS.md` is now whatever `fh brief` prints: the same short how-to as before, plus the terms FlipHopper relies on (area, iteration, label, state) and where to read more: `fh guide terms`, `fh guide customize` (what a project may change: areas, iterations, ladder, task IDs; what is fixed: the six states, and the words "area" and "iteration" until R90), `fh guide commands`, `fh guide workflow`. Both requests tell the agent to run `fh brief` and use its output; a copy stays in the request for when `fh` cannot run where the agent works.

## Change (2026-10-10, owner): agents know how to add a task

**In plain words.** The brief in `AGENTS.md` now says: "Adding a task: read `fh guide tasks` first". That topic tells the agent to look at the existing areas and IDs, use an existing area, leave out the ID so `fh` picks the next free number, write the title as one outcome, write the description as *Outcome / Done means / Notes*, leave the task in backlog for the person to approve with `fh ready`, and plan an iteration only when asked. `fh add task` without an ID now takes the next free number in the area (A01-04 after A01-03).
