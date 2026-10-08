# Task-A05-02-convert-existing-project

*Card type: decision (a proposal for the owner). Area A05 (Project setup & import). Requested by the owner (2026-10-08): look at the SSR migration plan (`next-ssr/docs/migration-plan-ssr/`) and the `AGENTS.md` that governs it, and propose how a person uses the FlipHopper app to turn such a project into a standard FlipHopper project. Read-only observation of the SSR repository on 2026-10-08; nothing there was changed.*

## What gets built (plain words)

A person opens an existing project folder in FlipHopper and chooses **File → Convert This Project…**. FlipHopper reads the folder without changing it and says what it found: "a task board, 101 card files in 16 areas, a handoff file, agent instructions that say how to claim cards". It then shows **what the project would look like in FlipHopper** — every area and task, its state, label, holder and next dependency — next to a list of everything it could not place. The person corrects the few mappings it asks about (for example, which of their words means *done*). Pressing **Convert** writes the seabag on a new Git branch, and asks the person's AI agent to rewrite the project's own instructions so agents use `fh` from then on. Nothing is committed, merged or pushed until the person says so.

| Step | What the person sees | What happens underneath |
|---|---|---|
| 1. Look | "This folder has a Markdown task board" and a short inventory | A read-only **scan** finds the board, card files, area files, handoff file and agent instructions |
| 2. Preview | Areas and tasks as FlipHopper will show them, plus **"Not placed"** and **"Needs your answer"** lists | A **reader** for the recognised board shape (or the person's AI agent, for shapes FlipHopper does not know) produces a *proposed* seabag in a staging folder; FlipHopper validates it |
| 3. Answer | A few questions: which label words form the ladder, which states mean done or waiting, what to call areas | The answers are saved with the conversion, so running it again gives the same result |
| 4. Convert | A new branch `fliphopper/convert` with `.fliphopper/seabag/` and a reconciliation report | Card files stay where they are; each task links to its card file, as FlipHopper's own tasks do |
| 5. Switch the agents over | The existing "Tell AI agents about this project" panel ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)), with a conversion request | The person's agent edits `AGENTS.md` / `CLAUDE.md` in place: status and claiming move to `fh`; every other rule stays |
| 6. Retire the old status | A checklist: board-fields tables, the generated board sections, the board generator and its pre-commit hook | Done by the person's agent on the conversion branch, only after the person agrees, because it changes their repository's rules |

**What you can see at the end:** the SSR project opens in FlipHopper with all its cards in their areas, the three claimed cards held by their sessions, the done cards with their done dates (so a burndown has real history), and its `AGENTS.md` telling agents to `fh claim` instead of editing a Session field.

## Why this shape

**Two halves, two different kinds of work.** Moving *status* (state, label, holder, dates) is mechanical and must be exact: a dropped or duplicated card is a lie on the board. Rewriting *instructions* (`AGENTS.md`) is judgement: it is the owner's rule book, written in their words, mixing status rules with safety boundaries that must survive untouched. So FlipHopper does the first itself, deterministically, and asks an agent to do the second, with the person pressing Enter — the same split the app already uses when it asks agents to add FlipHopper's instructions ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)).

**Status is copied, narrative is linked.** FlipHopper already keeps "the narrative stays in the repo" ([analysis](../analysis/ssr-task-board.md#what-glidebox-should-keep)): a task's `description.md` points at its card file. Converting therefore moves only the fields FlipHopper owns, and leaves the outcome, unknowns, approvals and progress notes where the team reads them.

**Preview before anything changes.** This is the accepted rule for generated projects (R53–R55, [AI-GENERATED-PROJECTS](../product/AI-GENERATED-PROJECTS.md#validation-and-adoption-rules--proposed)): validate, show, then adopt; never adopt half a project.

## What the SSR project looks like today (observed 2026-10-08)

| Thing | Where | FlipHopper equivalent |
|---|---|---|
| **Areas** A00–A15 (one file each, e.g. `areas/A09-enterprise-ui.md`, with a `## Tasks` table) | `areas/` | areas `A00`…`A15`; name from the file's `# A9 — Enterprise UI` heading |
| **Cards**: 101 `areas/Task-A…-….md` files; a few older cards are *sections* inside an area file (`A09-enterprise-ui.md#task-a09-01-…`); one sub-folder of supporting documents (`areas/A03-04/`) that is not a card | `areas/` | one task each; ID without `Task-` and description (`A09-05`, `A07-01-08`); three-level IDs are already valid in FlipHopper |
| **Board-fields table** on each card: State, Done, Priority, Depends on, Waiting on, Feature | top of each card | `state`, done date, priority, dependencies, `waiting_on`, feature — **the last four are not yet in FlipHopper's task model** (see Gaps) |
| **Status table** on each card: Status (prose), Label, Approval, Session, Next dependency | top of each card | `label`, `holder` (Session), `next_dependency`; Approval and Status prose stay in the card |
| **Label ladder**: implemented → locally verified → connected → deployed and verified (+ owner verified) | `TASK_BOARD.md` Conventions, `AGENTS.md` gate 3 | the project's own ladder in `project.json` — FlipHopper already allows one per project |
| **Session registry** (who holds what) | `TASK_BOARD.md` | `holder` on each task; the registry disappears (`fh list --state in_progress`) |
| **Generated board sections** (`<!-- board:open/done/recent -->`) and counts, from `next-ssr/src/tools/board/check-board.mjs`, enforced by a pre-commit hook and CI | `TASK_BOARD.md`, the hook | the app's views; the generator and hook are retired or pointed at the seabag at cut-over |
| **Now**, Waiting on the owner, Timeline (hand-drawn gantt), Deferred | `TASK_BOARD.md` | Now stays as text (as FlipHopper's own board does); the gantt is replaced by the burndown and forecast ([`A02-01`](./Task-A02-01-burndown-and-forecast.md)) |
| **Roadmap work not yet carded** (rows in area `## Tasks` tables without a card file) | area files | backlog tasks, marked "no card yet" — or left out; the person chooses |
| **Handoff** (`AGENT_HANDOFF.md`, active resume blocks) | | stays as it is; `fh release --note` covers only the one-line per-task note |
| **Estimates** per area in person-weeks | `ESTIMATE.md` §4 | not converted now; a later forecasting card |
| **Agent rules** on status: "Status has one home: the card", the claiming protocol, the session registry, "run check-board.mjs after changing a card" | `AGENTS.md` § Working procedure, `TASK_BOARD.md` Conventions | replaced by FlipHopper's short `fh` section; everything else (boundaries, five gates, card runs, reading rules, verification language) is kept word for word |

Two things the scan must expect, because they are true here: the Conventions block still says `Task-W{…}` while every file is `Task-A{…}` (old IDs need an alias list, not a guess), and the board says `workstreams/` in places where the folder is now `areas/`.

## The pieces

1. **Scan** (Rust, read-only). Walks the chosen folder for agent instruction files, a board file, card-like files (`Task-*.md` or a configurable pattern), area files and a handoff file. Produces an inventory. Never runs the project's scripts or hooks.
2. **Readers.** One per board shape FlipHopper recognises. The first is the **card-file board with a board-fields table** — the SSR shape — and is exactly the importer of [`A05-01`](./Task-A05-01-ssr-board-import.md), which this card generalises: A05-01 becomes "the first reader". A reader turns files into a proposed seabag plus a list of what it could not read; it never guesses (an unknown state word is a question, not a default).
3. **Agent fallback for unknown shapes.** If no reader recognises the folder, the app offers the AI-generated route (R50–R52): it opens the person's agent with a versioned conversion prompt that says *write a seabag into `.fliphopper/staging/`, copy only what the files say, leave unknown dates unknown, list everything you skipped*. FlipHopper then validates that staging folder exactly as it validates a reader's output. An agent never writes the live seabag directly.
4. **Preview and questions** (app). Areas and tasks, counts per state, a "Not placed" list (file, line, why) and a "Needs your answer" list. The answers — label ladder, state mapping, area word (R90: SSR says "area" now, earlier "workstream"), what to do with uncarded roadmap rows — are saved in `.fliphopper/convert.json` on the branch, so a second run (for example after the team kept working for a week) gives the same mapping and shows only what changed.
5. **Convert.** On a new branch (or worktree) the person names: writes `.fliphopper/seabag/` and `docs/fliphopper-conversion-report.md` (every card file → task ID, nothing dropped or duplicated; every unread value). Uses the real `git` ([0010](../decisions/0010-project-data-folder.md)); never commits unless asked; never runs or bypasses the project's hooks.
6. **Switch the agents over.** The conversion request sent through the A03-03 panel: *in `AGENTS.md` (Codex) / `CLAUDE.md` (Claude), replace the rules that say where status lives and how to claim with FlipHopper's `fh` section; keep every other rule word for word; show the diff.* The request names the exact sections the scan found.
7. **Cut over.** A checklist the person ticks, each item done by their agent on the conversion branch: remove the board-fields tables from cards (or leave them and mark them stale — the person chooses), remove the generated board sections and session registry, retire or rewrite `check-board.mjs` and its hook. Until cut-over, the cards stay the source of status and the seabag is a read-only **mirror** the app regenerates on demand; after it, the seabag is the source and the cards hold only narrative.

## Proposed defaults (owner to accept or change)

1. **Mirror first, then cut over.** The first conversion is a read-only mirror the team can look at in the app while still working the old way; cut-over is a separate, explicit step. This answers the open board question *"one-time import or keep syncing during the transition?"* with "sync one way (cards → seabag) until cut-over, then one-time switch".
2. **The project's own ladder is kept** (here: implemented, locally verified, connected, deployed and verified, owner verified), not FlipHopper's.
3. **Task IDs drop only the `Task-` prefix and the description**: `Task-A07-01-08-seeker-application-list` → `A07-01-08`. Card files keep their names and places.
4. **Uncarded roadmap rows become backlog tasks** with "no card yet" in their description, so the forecast knows the full scope.
5. **Status prose and approvals stay in the card file**, never parsed into fields (R54: a converted record must not invent an approval).
6. **FlipHopper itself never edits the project's agent instructions or hooks**; it asks the person's agent, who shows a diff.

## Gaps this exposes in FlipHopper (each its own card when this is accepted)

| Gap | Why the SSR project needs it | Proposed card |
|---|---|---|
| Task **dependencies** (`Depends on`) | cards name the cards they wait for | A01: dependencies on tasks |
| **Done date** and state history | burndown and forecast need when each card finished; SSR has about 80 done cards with dates | A02-01 (burndown) reads it; the model gains `done_on` / an event log |
| **Priority** and **Feature** fields | used on SSR cards | A01: optional extra fields, or kept as description text (owner) |
| Card file **sections** as tasks | some old cards live inside an area file | handled by the reader (description links to `file.md#anchor`) |
| **Convert** menu, scan, preview | this outcome | A05-02-01 scan and preview (mirror); A05-02-02 cut-over and agent switch |

## Details for the worker

- **Outcome.** A person converts an existing Markdown-board project into a standard FlipHopper project from the app: a read-only preview first, then a seabag on a branch, then their agents switched to `fh`, without losing any card or rule.
- **Decisive unknowns.**
  - The owner accepts or changes the six defaults above (owner).
  - Whether Priority and Feature become task fields or stay description text (owner).
  - Whether the real SSR conversion is wanted as the first real use — it writes to the SSR repository, which is read-only to FlipHopper's agents, so it needs the owner's exact approval (owner).
- **Files owned (this card).** This card only. The implementation cards named in Gaps own code.
- **External operations.** None for this proposal. A conversion of the real SSR repository (a branch and files there) needs its own exact approval on the implementing card.
- **Done means mechanically.** The owner accepts or changes the defaults; accepted pieces are opened as implementation cards (scan and mirror first, then cut-over); A05-01 is re-scoped as the first reader.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-08 — Proposal written from a read-only look at `next-ssr/docs/migration-plan-ssr/` (TASK_BOARD.md, areas/, a sample card, an area file) and the repository-root `AGENTS.md`. Counts: 101 card files in `areas/` across A00–A15, 3 cards claimed, board sections generated by `check-board.mjs`. Recounted at implementation time; never copy private board text into fixtures.
