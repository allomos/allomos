# Task-A01-46-pane-stays

*Card type: coding. Area A01 (Tracking). Written 2026-10-16. Builds on [`A01-16`](./Task-A01-16-task-pane.md) (the task pane). Task text: `alm show A01-46`.*

## What gets built (plain words)

Clicking a task in the list opens its details in the task pane beside the list. Until now, clicking the same task a second time closed the pane, which surprised people who clicked again just to be sure. After this card, clicking the task already shown leaves the pane open; clicking another task switches the pane to it, as before. The pane still closes with its own × button and with Esc.

| Piece | What it is | Where |
|---|---|---|
| Which task the pane shows after a click | A pure function `afterClick(shown, clicked)`: always the clicked task | `apps/web/src/paneFlow.ts`, tested in `apps/web/tests/paneFlow.test.ts` |
| The list's click | By area, By iteration and the "hidden by the filter" hits use `afterClick` instead of toggling | `apps/web/src/App.tsx` |

## Details for the worker

- **Outcome.** Clicking the task already shown keeps the pane open; clicking another switches; × and Esc still close it.
- **Rules.** Desktop and web share the same interface code (`apps/web`), so both change together. The Handoffs tab's task links already opened the pane without toggling (they switch to By area and select the task); they are unchanged.
- **Not included.** Any change to Esc, the × button or the arrow keys.
- **Files owned.** The files above; this card; the one line in [`A01-16`](./Task-A01-16-task-pane.md) that described the old toggle (superseded in place).
- **External operations.** None.
- **Done means mechanically.** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test` pass; a debug desktop app built.

## Approvals

| What | By | When | Source |
|---|---|---|---|
| The outcome: a second click keeps the pane open | approver | 2026-10-15 | chat; task note of A01-46 |

## Decisions made while working

- Decided: the click logic lives in a tiny pure module (`paneFlow.ts`) rather than inline, so the rule is tested, as CODING_RULES asks for interface decisions.
- Decided: the search's "hidden by the filter" hits behave like the list (they used the same toggle), so every task list in the window acts alike.

## Evidence

**2026-10-16, claude/a01-46-pane-stays.** Checks on branch `claude/a01-46-pane-stays`: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS; `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS; `test` PASS (98 tests, 0 failed, including 3 new in `paneFlow.test.ts`). Debug app built: `target/debug/bundle/macos/Allomos.app`. Manual check in the app: NOT RUN by the agent (for the approver).

**Manual check.** Trial project: `~/allomos-tests/a01-46-pane-stays/` (area A01 Kitchen, tasks A01-01, A01-02).

```bash
open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a01-46-pane-stays/target/debug/bundle/macos/Allomos.app
```

1. Open the folder `~/allomos-tests/a01-46-pane-stays`.
2. By area: click A01-01; the pane opens. Click A01-01 again; the pane stays open.
3. Click A01-02; the pane switches to it. Press Esc; it closes. Open it again and click ×; it closes.
4. By iteration: repeat step 2.
