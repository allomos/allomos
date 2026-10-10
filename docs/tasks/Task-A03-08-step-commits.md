# Task-A03-08-step-commits

*Card type: coding. Area A03 (Agents). Requested by the owner (2026-10-11), after a trial in `~/fliphopper-tests/a00-21-old-project` (session "Migrate project tasks to Allomos"): "the create taskbag step and the tell agent step should have their own commit instructions to the agent, because the modified files blocked the find tasks step."*

## What gets built (plain words)

When a person opens a folder, Allomos creates a taskbag (step 1 of [OPEN-FOLDER-FLOW](../product/OPEN-FOLDER-FLOW.md)) and asks an agent to add its section to AGENTS.md (step 3). Neither was committed, so in step 5 the agent that moves the tasks found uncommitted files and stopped, three times, until the person said "commit them".

| Request | New instruction to the agent |
|---|---|
| Tell AI agents about Allomos | Commit the new taskbag on its own first (message "Add Allomos taskbag"); after editing, commit only AGENTS.md and CLAUDE.md (message "Tell AI agents about Allomos"). Never other files; never push unless asked. |
| Find the tasks and move them (request version 5) | Before branching, commit leftovers from the earlier steps, each on its own: the taskbag, then AGENTS.md and CLAUDE.md. Stop and ask only when *other* files it will change have uncommitted changes. |

The app itself still never commits (decision [0010](../decisions/0010-project-data-folder.md)): the agent does, as part of the request the person sends.

## Details for the worker

- **Outcome.** The person goes from "Create taskbag" through "Tell AI agents" to "Move tasks" without being asked to commit by hand.
- **Files owned.** `crates/store/src/lib.rs` (setup request), `crates/store/src/convert.rs` (request v5), their tests, this card.
- **External operations.** None.
- **Done means mechanically.** Rust checks pass; tests show both commit instructions in all three languages, with paths limited (`git commit ... -- <paths>`); a real agent run on a fresh test folder goes from the setup request to the move without stopping for uncommitted files.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | The request itself ("should have their own commit instructions") | owner (chat) |

## Notes
- 2026-10-11 — Built: both requests carry the commit instructions (three languages; tests check the exact commands and that only the step's own paths are committed). Rust fmt, clippy, all tests: PASS. App rebuilt from this branch and `alm` installed from it. Test project for the owner: `~/fliphopper-tests/a03-08-old-project`, a committed FlipHopper project (`.fliphopper/seabag/`, `fliphopper/1`; A01-01 done, A02-01 held by claude/order-form, A02-02 waiting) with an AGENTS.md that still mentions `fh`. Waiting on the owner's run through the flow.
