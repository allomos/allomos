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
