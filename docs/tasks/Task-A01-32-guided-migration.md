# Task-A01-32-guided-migration

*Card type: coding. Area A01 (Tracking), Iteration 2. Written 2026-10-14 from the task's description in the taskbag (the owner's decisions of 2026-10-14). Replaces the code steps of [`A01-25`](./Task-A01-25-taskbag-format-upgrade.md).*

## What gets built (plain words)

Today, when Allomos finds a taskbag written by an older Allomos, `alm upgrade` rewrites it by itself, one code step per version. That cannot work when several versions are skipped at once: a step written for one version cannot know what a later version changes, and some changes need a decision per task. After this card, updating an older taskbag is a **guided migration**, like moving another task system into Allomos ([`A05-02`](./Task-A05-02-convert-existing-project.md)): Allomos gives an AI agent one prompt, the agent follows it and changes the taskbag only through `alm`, and `alm` marks the taskbag as the new version only when nothing is left to fix.

| Piece | What it is | Where |
|---|---|---|
| A migration entry per version | For each format change: what changed; what to add; what to decide task by task, with the rule; the `alm` commands to use; how to check. Adding a version later is data only. | `crates/store/src/versions.rs` (`MIGRATIONS`) |
| One prompt | The entries from the taskbag's version to the newest, joined, with the project's paths, in the app's language. It tells the agent: look and change nothing; show the plan and wait for the person; apply only through `alm`; finish with `alm migrate --finish`; report what changed and what could not be decided; commit only the taskbag's files, and only when asked. | `crates/store/src/versions.rs` |
| `alm migrate` | Prints that prompt. `alm migrate --finish` sets the new version, and is refused while `alm list --json` reports issues, listing them. | `crates/cli` |
| Read-only while updating | Until finished, reading commands, `alm migrate`, `alm handoff` and the commands the entries name work; commands that start or change work (claim, label, done, add …) are refused, with what is left to fix. The app's changes are refused the same way. | `crates/cli`, `crates/store` |
| Too old | `allomos/3` is the oldest format this Allomos updates. An older taskbag is told it is too old (`taskbag_too_old`). The code steps 1 → 2 and 2 → 3 and their tests are gone. | `crates/core`, `crates/store` |
| Rename a label | `alm rename label "<old>" "<new>"` renames a step of the ladder together with every task that carries it (same shape as `alm rename area`). | `crates/core/src/ops.rs`, `crates/store`, `crates/cli` |
| The app | **Migrate the taskbag…** (was **Update the taskbag…**, in every interface language) opens the chosen agent with this prompt typed in (plus the instruction-file renewal when that is out of date, as before). | `crates/store/src/versions.rs` (`agent_update_request_in`), `apps/web` |
| Same process as moving another task system in | An older Allomos taskbag is one more task system to migrate: the prompt shares the conversion request's opening (paths, how to run `alm`) and its one question (`alm confirm`, Go ahead or change nothing). | `crates/store/src/convert.rs` (`migration_header_in`, `ask_once_in`) |

**What you can see at the end:** nothing changes for today's `allomos/3` taskbags. A taskbag on `allomos/2` or older shows "too old" in `alm` and the app. The first real entry (3 → 4) is added by the cards that change the format; until then the flow is shown by tests with a test-only entry.

## Details for the worker

- **Outcome.** An agent brings an older taskbag up to the newest format by following one guided prompt, and `alm` marks it new only when nothing is left to fix.
- **Decisive unknowns.** None left: the owner decided on 2026-10-14 (see Approvals).
- **Boundary.** The first real entry (3 → 4) depends on A01-31 (decision record 0016, still *Proposed*) and A03-18 (backlog). This card does **not** raise the format to `allomos/4` and does **not** write the 3 → 4 entry; it builds the machinery so that entry is data only.
- **Files owned.** `crates/core/src/{model,lib,validate,problem,ops}.rs`, `crates/store/src/{versions,lib,setup,handoff,events,convert}.rs`, `crates/store/tests/edits.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/alm.rs`, `crates/store/src/guide.rs`, `apps/desktop/src-tauri/src/lib.rs` (tests), `apps/web/src/{App.tsx,types.ts,versionFlow.ts}`, `apps/web/src/i18n/*`, `apps/web/tests/versionFlow.test.ts`, `docs/product/FILE-FORMAT.md`, `docs/product/OPEN-FOLDER-FLOW.md`, `docs/workflow/CODING_RULES.md`, this card.
- **External operations.** None.
- **Done means mechanically.**
  - The named checks in [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass.
  - Tests: an update from 3 to 4 on a test taskbag (with a test-only entry, not shipped); `--finish` refused while issues remain, and nothing written; the read-only behaviour mid-update (reading and migration commands work, claim/label/add refused with what is left); the "too old" message for version 2; renaming a label with the tasks that carry it.
  - A test fails if the format is raised without an entry for every version from the oldest supported one.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Every format change goes through the guided prompt, even one that only adds a folder (several versions may be skipped at once). | owner (chat) |
| 2026-10-14 | Drop the code steps 1 → 2 and 2 → 3 and the test `upgrade_from_2_adds_nothing_but_the_format`; `allomos/3` is the oldest supported format; an older taskbag is told it is too old. | owner (chat) |
| 2026-10-14 | Migration replaces the update process entirely: `alm upgrade` is removed, replaced by `alm migrate` (prints the prompt, writes nothing) and `alm migrate --finish`; the app's **Update the taskbag…** becomes **Migrate the taskbag…** in every interface language; the older-format message says to run `alm migrate`; it is the same migration process as A05-02's conversion (an older Allomos taskbag is one more task system to migrate), sharing that prompt path. A01-31 and A03-18 supply the 3 → 4 entry later. | owner (relayed by the owner's other session through the coordinator) |
| 2026-10-14 | The finish gate: `alm` refuses to write the new version while `alm list --json` reports issues (a check inside `alm`, approved). Until finished, reading and migration commands work; commands that start or change work are refused with what is left to fix. | owner (chat) |

## Notes

- Known limit (from the task): another machine still running the old Allomos sees version 3 and keeps writing during the update; people are told to install the new Allomos first.
- 2026-10-14 — **Built (claude/a01-32-guided-migration), locally verified.** In plain words: `alm upgrade` and its code steps are gone. An older taskbag (from `allomos/3` on) is now read but locked for work until an agent migrates it: `alm migrate` prints the guided prompt, built from per-version migration entries; `alm migrate --finish` writes the new format only when `alm list --json` reports nothing. A taskbag on `allomos/2` or older is told it is too old. `alm rename label "<old>" "<new>"` renames a ladder step and every task that carries it (named like `alm rename area`). The app's button is now **Migrate the taskbag…** in all three languages, and the prompt opens the way the conversion request (A05-02) does, with the same one question through `alm confirm`. `MIGRATIONS` is still empty and the format stays `allomos/3`: the 3 → 4 entry waits on A01-31 (record 0016, *Proposed*) and A03-18.
  - New problem codes: `taskbag_too_old`, `migration_missing` (replaces `upgrade_missing`), `migration_unfinished`, `label_exists`, `label_name_missing`; `taskbag_older` now carries `left` (what is left to fix).
  - Checks: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (new: 3 → 4 with a test-only entry, in `crates/store` and in-process through `alm`; finish refused while an issue remains and nothing written; read-only mid-migration: reading, `migrate`, `handoff` and `rename label` work, claim/label/done/add/labels/plan/rename area refused with what is left; version 2 too old; `rename label`; a test that fails if the format is raised without an entry; the prompt in three languages sharing the conversion request's opening); `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS (316 texts, 64 codes); `npm --prefix apps/web test` PASS (56).
  - Manual: `~/allomos-tests/a01-32-too-old` (format set to `allomos/2`): `alm list` and `alm migrate` exit 1 with `taskbag_too_old`, nothing changed. `~/allomos-tests/a01-32-current`: `alm migrate` and `alm migrate --finish` say there is nothing to do. Delete both folders when the card is done.
  - Not tried: a real agent following a real 3 → 4 prompt (no real entry yet), and **Migrate the taskbag…** on screen (needs an older taskbag, which today's Allomos cannot have): owner check once the first entry exists.
