# Task-A03-20-project-notes-standard

*Card type: coding. Area A03 (Agents). Written 2026-10-14 at the owner's request: everything an agent is told about Allomos comes from `alm`, and the taskbag's `project-notes.md` is a standard part of Allomos that is kept up to date while a project is being built.*

## What gets built (plain words)

Every taskbag already has `project-notes.md`: what every session must know whatever its task (what is running, pitfalls, where credentials live). `alm` creates it, but nothing `alm` gives an agent mentions it. Projects had to point to it with a line they wrote into AGENTS.md by hand; the Jobs project removed that line when it kept Allomos's instructions only in the generated section, and its notes stopped being read.

After this card:

| Piece | What it is | Where |
|---|---|---|
| The brief names the notes | Allomos's section in AGENTS.md (`alm brief`) says: read the taskbag's `project-notes.md` (Running, Pitfalls) at the start; keep it true while you work; it is the one taskbag file you edit directly | `crates/store/src/lib.rs` (`agent_section`), brief v7 |
| When to update the notes | The brief says: when you start or stop a server or process, create or change a hosted resource or a remote, learn a pitfall that would change what the next session does, or a credential's location changes; change lines in place and remove ones no longer true | same |
| `alm next` shows them | Below the ready tasks, the Running and Pitfalls sections and the date the notes last changed (facts only; `alm` judges nothing) | `crates/cli/src/lib.rs` (`next`) |
| `alm handoff` reminds | After giving the handoff file, one line: "If what is running or a pitfall changed, update `project-notes.md` too", with its last-changed date | `crates/cli/src/lib.rs` (`handoff`) |
| Every new card gets a task | The brief says new work gets its task with `alm add task` (after `alm guide tasks`) | `crates/store/src/lib.rs` |

Projects with the v6 section get the existing "section out of date" warning and update it with `alm brief`; no hand-written Allomos lines are needed.

## Details for the worker

- **Outcome.** An agent that reads only Allomos's generated section and `alm next` knows the project notes exist, reads them, and keeps them current during development.
- **Decisive unknowns.**
  - `alm next` and `alm handoff` printing the notes and the reminder: approved by the owner 2026-10-14 (adds output, not checks).
  - Exact brief wording, for the owner to confirm in review.
- **Files owned.** `crates/store/src/lib.rs` (`agent_section`), `crates/store/src/versions.rs` (`SECTION_VERSION` 6 → 7), `crates/store/src/guide.rs` (if it repeats the brief), `crates/cli/src/lib.rs` (`next`, `handoff`), `crates/cli/tests/alm.rs`, `docs/product/FILE-FORMAT.md` (`project-notes.md` row), this repository's `AGENTS.md` (regenerated section; the hand-written pointers outside it may then go), this card.
- **External operations.** None. Updating the Jobs project's AGENTS.md is that project's own work after this ships.
- **Done means mechanically.**
  - Tests: the brief mentions `project-notes.md`, when to update it, and adding a task per new card; a v6 section is reported out of date; `alm next` (text and `--json`) includes the notes' Running and Pitfalls and last-changed date; a missing notes file is reported plainly, not as an error; `alm handoff` prints the reminder.
  - Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
  - By hand: `alm brief`, `alm next` and `alm handoff` on this project's taskbag.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Allomos instructions come only from `alm`; `project-notes.md` is a standard part of Allomos and is maintained during development; open this card | owner (chat) |
| 2026-10-14 | The card as written, including `alm next` and `alm handoff` showing the notes and the reminder; made ready for the loop session | owner (chat) |

## Notes

- **2026-10-14 — drafted.** Found while reviewing the Jobs project's AGENTS.md cleanup: `alm` creates `project-notes.md` (`crates/store/src/lib.rs`, `DEFAULT_NOTES` in `crates/store/src/handoff.rs`) and the conversion request fills it, but the brief and every `alm` command are silent about it, and the brief's "never edit those files by hand" contradicts the conversion request's "you may edit this file directly".
- **2026-10-14 — implemented and locally verified** (claude/a03-20-project-notes). In plain words: Allomos's section in AGENTS.md (`alm brief`, now v7) tells agents about `project-notes.md`: read it at the start, it is the taskbag file (besides their own handoff) they edit directly, keep it true, and when to update it; it also says every new piece of work gets its own task. `alm next` now ends with the notes' Running and Pitfalls sections and the day the file last changed; `alm handoff` adds one line reminding to update the notes. A project with the v6 section gets the usual "section out of date" warning.
  - **Choices made (routine, A00-29):** "last changed" is the file's modification time on this computer, as a UTC day (`alm` never reads Git, so a fresh clone shows the clone's day). A section is the first `##` heading that starts with "Running" / "Pitfalls" (any case), up to the next `#`/`##` heading, printed as written and indented two spaces; other sections (credentials, current truth) are not printed. A missing file: `Project notes: this taskbag has no project-notes.md yet (<path>).` and `"found": false`; a missing section: `Running: no such section.` Both commands add `project_notes` to `--json` (`file`, `path`, `found`, `changed`, `running`, `pitfalls`). The brief's first line now says "never edit those files by hand, except the project notes (below) and your own handoff file", ending the contradiction with the conversion request. `alm guide` (terms, customize, commands, workflow) says the same. In this repository's AGENTS.md the section was regenerated from `alm brief`, and the Handoff section's hand-written sentence pointing to `project-notes.md` was removed (the brief now says it); the always-on credentials rule and the "Read by activity" row stay. `crates/store/src/convert.rs` step 6 still says "never edit its files by hand" before step 7 allows the notes: not in this card's files, left as is.
  - **Evidence:** `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (new: `handoff::tests::project_notes_give_running_pitfalls_and_the_day_they_changed`, `project_notes_are_in_the_brief_next_and_handoff`; `an_out_of_date_section_warns_the_agent_on_every_call` now expects v7 and checks a v6 section warns). By hand on this project's taskbag with the worktree's `target/debug/alm`: `alm brief` prints v7; `alm next` printed Running and Pitfalls with "last changed 2026-10-14" (and the expected v7 out-of-date warning against main's AGENTS.md); `alm handoff --tasks A03-20 --from …` printed the reminder before the Git commands. Not done: the owner's review of the exact brief wording.
  - **Seen, not changed:** the real `project-notes.md` still has the stale pitfall "Use `--push` on every `alm` change" (removed by decision 0017); for the owner or a session to correct in place.
