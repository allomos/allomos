# Task-A01-29-rename-delete-areas-iterations

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-14 at the owner's request, split out of [`A01-19`](./Task-A01-19-create-in-app.md) (creating them in the app).*

## What gets built (plain words)

After [`A01-19`](./Task-A01-19-create-in-app.md) the owner can create areas and iterations, but a typo in a name, a moved date or a plan that changed cannot be fixed: neither the app nor `alm` can change or delete them. After this card, both can, under the same rules.

| Piece | What it is | Where |
|---|---|---|
| Rename an area | Change an area's name (its ID stays). | `crates/core`, `crates/cli`, `apps/web` |
| Change an iteration | Change an iteration's name, release, start date and end date (its ID stays). Dates are checked like `alm add iteration`. | `crates/core`, `crates/cli`, `apps/web` |
| Delete an area | Refused while any task is in the area. | `crates/core`, `crates/cli`, `apps/web` |
| Delete an iteration | Refused while any task is planned in it; move those tasks out first. | `crates/core`, `crates/cli`, `apps/web` |
| Same rules, recorded | The app calls the same Rust rules as the new `alm` commands; each change is recorded in history ([`A01-17`](./Task-A01-17-change-history.md)). | `apps/desktop/src-tauri` |

**What you can see at the end:** in the app, rename an area, change an iteration's end date, delete an empty iteration, and try to delete an area that still has tasks (refused, in plain words); `alm list` agrees.

## Details for the worker

- **Outcome.** The owner keeps areas and iterations correct as plans change, in the app or in a terminal.
- **Decisive unknowns.**
  - Decided (owner, 2026-10-14): deleting an area or an iteration is refused unless it has no tasks. These two refusals are checks inside `alm`, approved by the owner here.
  - Decided (owner, 2026-10-14): command names `alm rename area <id> "<name>"`, `alm change iteration <id> [--name] [--release] [--start] [--end]`, `alm delete area <id>`, `alm delete iteration <id>`.
  - Decided (owner, 2026-10-14): a done or dropped task counts as "in" its area or iteration, so history never points at one that is gone.
  - Decided (owner, 2026-10-14): planned in Iteration 2 (I02).
- **Files owned.** `crates/core/src/ops.rs`, `crates/cli`, `apps/web/src/App.tsx` (or a new component), `apps/web/src/messenger.ts`, `apps/web/src/types.ts`, `apps/web/src/i18n/*`, `apps/desktop/src-tauri/src/lib.rs`, the `alm` guide text for the new commands, tests, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The six named checks pass.
  - Tests: each change writes one event; deleting a non-empty area or iteration is refused and changes no file; a bad date is refused with the same code as `alm add iteration`.
  - In the built app: each of the four actions works; `alm list` agrees.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Write the card (not start it); delete only when empty, for both areas and iterations | owner (chat) |
| 2026-10-14 | Approve the card (ready) with the proposed command names and done/dropped tasks counting | owner (chat) |

## Notes

- 2026-10-14 — Card written by the agent. Next: the owner approves the card (`ready`) and picks its iteration; it starts after A01-19.
- 2026-10-14 — Owner approved the card; ready. Starts after A01-19; iteration still to be picked.
- 2026-10-14 — Owner planned it in I02.
- 2026-10-14 — Built in `claude/a01-29-rename-delete`; locally verified. Next: the owner tries it in the built app (below).

## Result (2026-10-14)

**In plain words.** Each area group and each iteration group in the list now has an **Edit** link next to **+ New task**. For an area it opens a small form with its name; for an iteration, its name, release and two dates. **Save** changes them (the ID stays); **Delete…** asks once more, then deletes it. A group that still has a task in it, even a done or dropped one, is not deleted: the form says so in plain words, and nothing changes. The same four changes exist in a terminal: `alm rename area <id> "<name>"`, `alm change iteration <id> [--name] [--release] [--start] [--end]` (what is not given stays; an empty value such as `--release ""` clears a release or date), `alm delete area <id>` and `alm delete iteration <id>`.

The app decides nothing itself: each form goes to the same Rust code as the `alm` command (`allomos_store::rename_area`, `change_iteration`, `delete_area`, `delete_iteration`: format check → lock, waiting up to 5 s → load → `crates/core` rule → event + file), as A01-19's adds do. Each change is one event in `events/` under the person's name (`via` app) or the `--as` name (`via` alm), with the commands `rename area`, `change iteration`, `delete area`, `delete iteration` and the kinds `area` / `iteration` that A01-17's format already has; a deleted item's event lists every field it had, with only `from` (no format change; one sentence added to FILE-FORMAT § Events). A change that changes nothing writes no event. Two new problem codes, `area_not_empty` and `iteration_not_empty` (the owner-approved refusals), worded in all three languages; an unknown ID reuses `unknown_area` / `unknown_iteration`, an empty name `area_name_missing` / `iteration_name_missing`, a bad date `invalid_date`.

Outside the files listed above: `crates/core/src/event.rs` (`removed`, the deleted item's changes), `crates/core/src/problem.rs` (two codes), `crates/store/src/lib.rs` (the shared write path), `crates/store/src/guide.rs` and `docs/workflow/CODING_RULES.md` (the new commands), `docs/product/FILE-FORMAT.md` (one sentence: what a delete event lists), `apps/web/src/styles.css` (the Edit link and the delete confirmation); new: `apps/web/src/GroupEditForm.tsx`, `apps/web/src/groupEditFlow.ts` (the form's choices, tested as `CODING_RULES` asks), `crates/store/tests/renames.rs`.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (core 36, store 55 + adds 4 + edits 4 + renames 5, cli 36, desktop 25 + 1 ignored as before) |
| `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` | PASS (55 web tests, 311 texts, 60 Rust codes) |
| Each change writes one event | PASS: `each_change_writes_one_event` (store: `rename area`, `change iteration`, `delete iteration`, `delete area`, one event each, by Will via app; the rename's change is `name` from → to; a repeated change writes none); `areas_and_iterations_are_renamed_changed_and_deleted` (cli: the same through `alm`, counted in `events/`) |
| Deleting a non-empty area or iteration is refused and changes no file, done/dropped tasks counting | PASS: `deleting_a_non_empty_area_or_iteration_is_refused_and_changes_no_file` (store: A01/I01 holding only a done task, A02/I02 only a dropped one → `area_not_empty` / `iteration_not_empty` with count 1; also unknown IDs and empty names; every file compared before and after: unchanged); cli: the same codes, no event, files still there |
| A bad date is refused with the same code as `alm add iteration` | PASS: `a_bad_date_is_refused_with_the_same_code_as_alm_add_iteration` (both `invalid_date` for `31/10/2026`, nothing written); cli `change iteration --start soon` → `invalid_date` |
| The app waits for `alm`'s lock | PASS: `changes_wait_for_alms_lock` |
| The Tauri commands, with the arguments as the interface sends them | PASS: `the_list_renames_changes_and_deletes_under_the_persons_name` (no name → `holder_missing`; `{name, release: "", start: "", end}` as `messenger.changeIteration` sends it; non-empty refused; each change one event by Will via app) |
| Built app | Built: `npm --prefix apps/desktop run tauri build -- --debug --bundles app` → `target/debug/bundle/macos/Allomos.app` in the worktree. Not driven by the agent (no screen access) |
| In the built app: each of the four actions works; `alm list` agrees | NOT RUN — needs the owner's eyes |

**For the owner's check.** A throwaway project is at `~/allomos-tests/a01-29-rename-delete/` (areas A01 Tracking, A02 Charts, A03 Spare; iterations I01 First (v0.1, 2026-10-01 to 2026-10-15), I02 Second, I03 Empty; task A01-01 planned in I01, task A02-01 planned in I02 and dropped; made with this branch's `alm`). Open it with the branch's build:

```bash
open /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-29-rename-delete/target/debug/bundle/macos/Allomos.app --args ~/allomos-tests/a01-29-rename-delete
```

Then: **By area** → A01's **Edit** → rename it → Save. A02's **Edit** → **Delete…** → Delete: refused in plain words (one task, counting dropped ones). A03's **Edit** → **Delete…** → Delete: it goes. **By iteration** → I01's **Edit** → change the last day → Save. I03's **Edit** → **Delete…** → Delete: it goes. Then compare in Terminal:

```bash
/Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-29-rename-delete/target/debug/alm list --taskbag ~/allomos-tests/a01-29-rename-delete
/Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-29-rename-delete/target/debug/alm list --by iteration --taskbag ~/allomos-tests/a01-29-rename-delete
```

(`alm list --by iteration` shows only iterations with tasks in them; `ls ~/allomos-tests/a01-29-rename-delete/.allomos/taskbag/iterations` shows the files.) Delete the folder once the card is done.
