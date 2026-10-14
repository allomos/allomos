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
| Format `allomos/4` | One migration entry 3 → 4 in A01-32's guided migration (data only, no code step); existing `waiting` tasks stay `waiting`; older Allomos asks to be updated. The entry is held for A03-18 too: **A03-18's part of this entry is added before version 4 is merged** (owner, 2026-10-14: one entry 3 → 4, not 4 and 5) | `crates/store/src/versions.rs`, `docs/product/FILE-FORMAT.md` |
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
  - Tests: `alm block` refuses an unknown ID, the task itself and an empty list; a `blocked` task without `blocked_by` fails validation; `alm list` / `show` / `next` print each blocker's current state; `alm next` says when all blockers are done and changes nothing; `alm migrate` / `alm migrate --finish` from `allomos/3` change only the format line and keep `waiting` tasks `waiting` unless the agent moves one with `alm block`; an `allomos/4` taskbag read by an `allomos/3` build gives `taskbag_newer`; each change writes one event.
  - In the built app: the owner sees one waiting and one blocked task, each worded plainly, in each UI language; the state filter separates them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Owner request; card and decision record written (not approved) | owner (chat) |
| 2026-10-14 | Accept 0016 with its five answers; build A01-31 together with A01-32, version 4 through the guided migration (relayed by the owner's other session, confirmed in this session's chat) | owner (chat) |
| 2026-10-14 | `allomos/4` is held for A03-18: A01-31's blocked state and A03-18's human-check rename ship in **one** 3 → 4 migration entry. A01-31 builds the format bump and its part of the entry, written in separate parts so A03-18's part is added to the same entry before version 4 is merged | owner (relayed by the coordinator) |

## Notes

- 2026-10-14 — Card and decision record 0016 written by the agent (`claude/a01-31-blocked-state`), documents only. Next: the owner accepts 0016 and answers its open points; then the card is approved for code.
- 2026-10-14 — Owner accepted 0016; card approved for code. Version 4 is built on A01-32's migration machinery.
- 2026-10-14 — **Built (claude/a01-31-blocked-state), locally verified.** In plain words: a stopped task now says plainly why. **Waiting** means a person (or an outside event) must act: `alm wait <id> --on "<what>" [--who "<who>"]`; the app says "Waiting for you: approve the card" or "Waiting for Ann: reply on the licence". **Blocked** is a new, seventh state: another task must finish first: `alm block <id> --by <task-id>…`; the app and `alm list` / `show` / `next` say "Blocked by A01-17 (in progress)", each blocker with its state now. When every blocker is done, `alm next` (and the app) only say so ("unblock it?"); nothing moves the task until someone runs `alm unblock <id>` (ready, or in progress when held) or `alm ready <id>`. A dropped blocker is allowed with a warning, shown as "(dropped; still blocked?)", and never counts as done. `alm next` and the loop skip blocked and waiting tasks by their state. The app has a state filter (All states, or any one of the seven), the pane's state editor offers Blocked with a task picker and Waiting with "who", and a "Blocked by" field; in en, zh-CN and ja. The format is `allomos/4`, reached through one migration entry 3 → 4 (`MIGRATIONS` in `crates/store/src/versions.rs`): nothing to add; existing waiting tasks stay waiting unless the agent finds that another task must finish first, then moves them with `alm block` after the person's answer; commands `alm block`, `alm wait`, `alm needs`; check with `alm list --json` and `--state blocked|waiting`. Its texts start with `[A01-31]`, and it says **A03-18's part of this entry is added before version 4 is merged** (owner, 2026-10-14). The brief (`## Tasks: Allomos (v4)`), `alm guide` ("The seven states"), FILE-FORMAT, CARDS, CODING_RULES and this repository's AGENTS.md section (from the new `alm brief`) are updated.
  - Task fields: `who` (waiting only), `blocked_by` (list, left out when empty); events list both. New problem codes: `blocked_by_missing`, `blocker_unknown`, `blocked_by_self`, `not_blocked`, `blocker_dropped` (warning); `cannot_make_ready` now names blocked too. Validation: a blocked task names at least one task, each known, not itself.
  - Checks: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (new: block/unblock rules in core; validation of `blocked_by`; app edits Blocked / Blocked by / Waiting with who, one event each; `alm wait --who`; `alm block` refusals (self, unknown, empty, missing `--by`), blockers' states in list/show/next, dropped blocker flagged and never done, `alm next` tells when all are done and writes nothing, `alm unblock`, one event per change; the real 3 → 4 migration in `crates/store` and through `alm migrate` / `--finish` with a waiting task kept waiting and one moved with `alm block`, mid-migration `claim` refused; a newer format refused with `taskbag_newer`; the test-only migration tests moved to 4 → 5); `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS (337 texts, 69 codes); `npm --prefix apps/web test` PASS (60).
  - Trial folders: `~/allomos-tests/a01-31-blocked-state` (format 4: one task waiting for you, one waiting for Ann, one blocked by an in-progress task, one blocked by a dropped task) and `~/allomos-tests/a01-31-blocked-state-format3` (format 3, two waiting tasks, one of which waits for another task) for **Migrate the taskbag…**. The project's real taskbag was not touched; it stays `allomos/3` until the owner installs the new build and migrates it. Delete both folders when the card is done.
  - Owner's trial: `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-31-blocked-state/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-31-blocked-state` (switch the language in the app for zh-CN and ja; use the **Show** filter), and `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-31-blocked-state/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-31-blocked-state-format3` for **Migrate the taskbag…**.
  - Not done here: A03-18's part of the 3 → 4 entry; the owner's in-app check.
