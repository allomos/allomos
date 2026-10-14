# Task-A03-18-human-check

*Card type: coding. Area A03 (Agents), Iteration 2. Written 2026-10-14 from the task's description in the taskbag and the owner's decisions of the same day (see Approvals). Replaces A03-17 (dropped).*

## What gets built (plain words)

Today a task an agent finished in a loop run, without the person trying it by hand, carries a note (`finished_without_owner`) that nobody is shown and that never goes away, so the list of such tasks only grows. And the word "owner" means two things: the person who tries the finished work, and the person who decides and approves. After this card:

- Everyone sees which tasks need a person: **To check** (finished without the human check) and **To answer** (waiting on a person), in one `alm` command and in the app.
- Each task to check carries the **human check steps** the agent wrote when it finished it (Markdown: the test folder, what to open, what to click, what you should see), so it is easy to pick up and test.
- Marking it **checked** gives it the top label and takes it off To check; the note and the steps stay in its history.
- "Owner" is split by meaning: the final check is the **human check** (top label *human verified*), and the person who decides is the **approver**.

| Piece | What it is | Where |
|---|---|---|
| `alm done --without-human-check "<why>" --steps <file.md>` | Records why and how to check; old `--without-owner` kept as an alias. Without steps, `alm` asks for them and shows how to write them (warning `human_check_steps_missing`), but does not refuse. On a task already done, only the human check is recorded again (no holder rule). | `crates/core/src/ops.rs` (`finish`), `crates/cli` |
| `alm human` | Two groups, To check (with why; `--steps` adds the steps) and To answer (waiting, with who and what); `--iteration`, `--area`, `--json` (always with the steps). Backlog is not listed. | `crates/cli` |
| `alm checked <id>` | A person checked a finished task: top label, note and steps cleared (the event keeps them). Refused for a task that is not done (`not_done`). Setting the top label on a done task does the same. | `crates/core/src/ops.rs` (`checked`, `set_label`), `crates/cli` |
| Task fields | `without_human_check` (was `finished_without_owner`) and `human_check_steps`, in `allomos/4` | `crates/core/src/model.rs`, `validate.rs`, `event.rs` |
| Migration | A03-18's part of the one 3 → 4 entry: the agent shows the ladder, proposes `alm rename label "owner verified" "human verified"`, asks for a customised ladder (never guesses), moves each old note with `alm done --without-human-check` or clears it with `alm checked`. Until then `alm list --json` reports `finished_without_owner_renamed` and `alm migrate --finish` is refused. | `crates/store/src/versions.rs` |
| App | "Not human-checked" badge beside the state in the area and iteration lists and in the task pane; the steps and a **Checked** button in the task pane; a **Needs a human (to check, to answer)** choice in the Show filter; in en, zh-CN, ja | `apps/web` |
| Words | human check / 人工检查 / 人による確認; human verified / 已人工验证 / 人が確認済み; "Not human-checked" / 待人工检查 / 人の確認待ち; approver / 审批人 / 承認者 | `apps/web/src/i18n`, `alm guide`, `alm brief`, docs |

**What you can see at the end:** open the trial taskbag; the six example tasks show "Not human-checked"; choose Show → Needs a human; click one, read its steps, click **Checked**: it gets the top label and leaves the list. `alm human` says the same in a terminal.

## Details for the worker

- **Outcome.** One `alm` command and one view in the app list every task that needs a person, each to-check task with its steps; checking one takes it off; "owner" becomes "human check" and "approver".
- **Files owned.** `crates/core/src/{model,ops,validate,problem,event}.rs`; `crates/store/src/{versions,guide,lib}.rs`, `crates/store/tests/renames.rs`; `crates/cli/src/lib.rs`, `crates/cli/tests/alm.rs`; `apps/web/src/{types.ts,stopFlow.ts,historyFlow.ts,App.tsx,TaskPane.tsx,styles.css,i18n/*}`, `apps/web/tests/{stopFlow,historyFlow,editFlow}.test.ts`; `AGENTS.md` (the Allomos section from `alm brief`, and the word "owner" in its rules), `README*.md` (one line each), `docs/GLOSSARY.md`, `docs/product/FILE-FORMAT.md`, `docs/workflow/{CARDS,CODING_RULES}.md`; this card.
- **External operations.** None. The project's real taskbag stays `allomos/3`; it is migrated by the owner with the new build.
- **Done means mechanically.**
  - The named checks in [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass.
  - Tests: `done --without-human-check` with and without steps (warning, not refusal), empty note and empty steps refused, steps without a note refused; the `--without-owner` alias for `done` and `list`; `alm human` groups, filters, `--steps`, `--json`, backlog left out; `alm checked` (top label, cleared, history keeps the note, not-done refused, no holder rule); top label on a done task clears; the whole 3 → 4 migration (blocked part and human-check part, old default ladder renamed, a customised ladder left as it is) through `alm migrate` / `--finish`; the app's filter and badge logic.
  - In the built app: the owner checks the view, the steps, the badge and the button with the six example tasks (A01-18, A01-19, A01-29, A03-14, A03-15, A03-16).

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Owner request: one task (view + checked step + rename); backlog cards not included; "who" is the owner now and the approving role in cloud projects later; `alm` asks for the steps but does not refuse without them | owner (chat) |
| 2026-10-14 | Command names `alm human` (To check + To answer; `--iteration`, `--area`, `--steps`, `--json`) and `alm checked <id>`, replacing "alm to-check"; the label rename and the field rename go through A01-32's guided migration in the same 3 → 4 entry (the agent plans, asks, applies with `alm`), not a code step; a customised ladder is shown to the person, never guessed; `alm done --without-human-check` with `--without-owner` kept as an alias; the terms in zh-CN and ja; To answer = `waiting` tasks; version 4 ships as one entry, no `allomos/5` | owner (chat 2026-10-14, relayed by the owner's other session "Task workflow open questions") |

## Notes

- Decided in the card (for the owner's review): the note keeps a flat field, `without_human_check`, beside `human_check_steps`, so the history shows each as its own line. On a task already `done`, `alm done --without-human-check` and `alm checked` do not apply the holder rule: a finished task's holder is history, the person checking is rarely that session, and the migration must move notes on tasks other sessions held. Waiting with no `who` now reads "waiting for the approver" in `alm` (the app keeps "Waiting for you"). In AGENTS.md and the docs, "owner" in the sense of the person who decides became "approver"; dated provenance notes such as "(owner, 2026-10-14)" and historical records were left as they are.
