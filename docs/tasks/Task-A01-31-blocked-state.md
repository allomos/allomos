# Task-A01-31-blocked-state

*Card type: decision, then coding. Area A01 (Tracking), Iteration 2. Written 2026-10-14 at the owner's request; replaces A01-30 (dropped).*

> **Approved for code.** Decision record [0016](../decisions/0016-waiting-and-blocked-states.md) was accepted by the owner on 2026-10-14 with the answers recorded there; where the details below differ from those answers, the record wins. The format goes to `allomos/4` through A01-32's guided migration, not a code step.

## What gets built (plain words)

Today a task that cannot go on is always **Waiting**, whether a person must act or another task must finish first. The owner read "Waiting" as "waiting for the AI". After this card the app and `alm` say plainly who or what a task waits on.

| Piece | What it is | Where |
|---|---|---|
| Decision record | The split, its names and open points, accepted by the owner | [docs/decisions/0016](../decisions/0016-waiting-and-blocked-states.md) |
| `waiting`, narrowed | A person (or an outside event) must act; says what, and optionally who | `crates/core`, `crates/cli` |
| `blocked`, new | Another task in the taskbag must finish first; lists the blocking task IDs | `crates/core`, `crates/cli` |
| `alm block` | `alm block <id> --by <task-id>…`; `alm list` / `show` / `next` show each blocker's current state, and say when all are done (alm tells, the agent checks) | `crates/cli`, `crates/store` |
| Format `allomos/4` | One upgrade step through the A01-25 path; existing `waiting` tasks stay `waiting`; older Allomos asks to be updated | `crates/store/src/versions.rs`, `docs/product/FILE-FORMAT.md` |
| App | Two states with their own words and colours in the list, the task pane, the state filters and the edit form, in every UI language | `apps/web`, `apps/desktop/src-tauri` |
| Docs | "Seven states" in `alm guide`, `alm brief` (the AGENTS.md section), CARDS, CODING_RULES | see Files owned |

**What you can see at the end:** in the app, one task "Waiting for you: approve the card" and another "Blocked by A01-17 (in progress)"; finish A01-17 and the second shows "(done)", and `alm next` says it can be unblocked.

## Details for the worker

- **Outcome.** Anyone reading the list, in the app or in a terminal, can tell at a glance whether a stopped task waits on a person or on another task, and which.
- **Decisive unknowns** (owner; listed in [0016 § Open for the owner](../decisions/0016-waiting-and-blocked-states.md#open-for-the-owner)):
  - state names (`waiting` + `blocked`, or e.g. `needs-person` + `blocked`);
  - whether a blocked task may also carry a waiting note;
  - whether a task moves by itself when all its blockers are done (proposed: no; `alm` only tells);
  - whether `blocked_by` may name a dropped task, and what that means;
  - whether `waiting` gets the optional `--who` now.
- **Files owned.** `crates/core/src/model.rs`, `ops.rs`, `validate.rs`, `problem.rs`, `event.rs`; `crates/store/src/versions.rs`, `lib.rs` (`alm brief`, `next`), `guide.rs`; `crates/cli/src/lib.rs`; `apps/desktop/src-tauri/src/lib.rs`; `apps/web/src/types.ts`, `messenger.ts`, `App.tsx`, `TaskPane.tsx`, `editFlow.ts`, `styles.css`, `i18n/*.json`; `docs/product/FILE-FORMAT.md`, `docs/workflow/CARDS.md`, `docs/workflow/CODING_RULES.md`, `AGENTS.md` (§ Tasks: Allomos, regenerated from `alm brief`); tests; this card. Shared: `docs/decisions/README.md` (the 0016 row's status).
- **External operations.** None.
- **Done means mechanically.**
  - Decision record 0016 accepted by the owner (status *Accepted*, open points answered).
  - The named checks pass (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`).
  - Tests: `alm block` refuses an unknown ID, the task itself and an empty list; a `blocked` task without `blocked_by` fails validation; `alm list` / `show` / `next` print each blocker's current state; `alm next` says when all blockers are done and changes nothing; `alm upgrade` from `allomos/3` adds only the format line and keeps `waiting` tasks `waiting`; an `allomos/4` taskbag read by an `allomos/3` build gives `taskbag_newer`; each change writes one event.
  - In the built app: the owner sees one waiting and one blocked task, each worded plainly, in each UI language; the state filter separates them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Owner request; card and decision record written (not approved) | owner (chat) |
| 2026-10-14 | Accept 0016 with its five answers; build A01-31 together with A01-32, version 4 through the guided migration (relayed by the owner's other session, confirmed in this session's chat) | owner (chat) |

## Notes

- 2026-10-14 — Card and decision record 0016 written by the agent (`claude/a01-31-blocked-state`), documents only. Next: the owner accepts 0016 and answers its open points; then the card is approved for code.
- 2026-10-14 — Owner accepted 0016; card approved for code. Version 4 is built on A01-32's migration machinery.
