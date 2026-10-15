# Task-A03-19-ai-decided

*Card type: coding. Area A03 (Agents), Iteration 2. Written 2026-10-15 from the task's description in the taskbag and the owner's decisions of 2026-10-14 (see Approvals).*

## What gets built (plain words)

Under the owner's rule A00-29, an agent decides routine choices inside a task itself (a name, a file layout, a small detail with an obvious default) instead of stopping to ask. Until now such a choice was only written on the card, where the owner had to go looking for it. After this card:

- The agent records each choice with **`alm ai-decided <id> "<what and why>"`**. It goes into the task's history.
- The task shows an **AI decided** badge in the app's task lists, and its task pane lists the decisions with who made them and when.
- **`alm human`** gets a third group, **To review: AI decisions**, beside To check and To answer, each task with its decisions; `--json` carries the same.
- The owner marks them reviewed with the task pane's **Reviewed** button, or by telling an agent to run **`alm reviewed <id>`**. The badge and the group entry go away; the history keeps the decisions and the review. Overturning a decision is an ordinary change to the task.

| Piece | What it is | Where |
|---|---|---|
| Decisions as events | `ai-decided` event with one change `ai_decision` (`to`: the text); `reviewed` event with one change per decision (`from` only). Decisions to review are worked out from the events: no new task field, no format change | `crates/core/src/decision.rs`, `crates/store/src/lib.rs` |
| `alm ai-decided`, `alm reviewed` | Record / review; `alm human` third group; `alm show` lists them | `crates/cli/src/lib.rs` |
| App | "AI decided" badge beside the state in the lists; the decisions and a **Reviewed** button in the task pane; the **Needs a human** filter keeps these tasks too; History names the field "AI decision"; en, zh-CN, ja | `apps/web/src`, `apps/desktop/src-tauri/src/lib.rs` |
| Agent instructions | `alm brief` (section v8), `alm guide` (terms, commands, workflow) point agents to `alm ai-decided` | `crates/store/src/{lib,guide,versions}.rs`, `AGENTS.md` |

**What you can see at the end:** open the trial taskbag; three tasks show "AI decided"; click one, read its decisions, click **Reviewed**: the badge goes, and History shows the review. `alm human` says the same in a terminal.

## Details for the worker

- **Outcome.** An agent's own decisions are recorded with `alm`, shown to the owner in the app and in `alm human`, and leave once reviewed.
- **Files owned.** `crates/core/src/{decision,lib,problem}.rs`; `crates/store/src/{lib,guide,versions}.rs`; `crates/cli/src/lib.rs`, `crates/cli/tests/alm.rs`; `apps/desktop/src-tauri/src/lib.rs`; `apps/web/src/{types.ts,messenger.ts,stopFlow.ts,historyFlow.ts,App.tsx,TaskPane.tsx,styles.css,i18n/*}`, `apps/web/tests/{stopFlow,historyFlow}.test.ts`; `AGENTS.md` (the Allomos section from `alm brief`); `docs/product/FILE-FORMAT.md`, `docs/workflow/CODING_RULES.md`, `docs/GLOSSARY.md`; this card.
- **External operations.** None. The real taskbag needs no migration (no format change).
- **Done means mechanically.**
  - The named checks in [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass.
  - Tests: recording a decision (history event, task files untouched, empty text and unknown task refused); `alm human` group and `--json` (`to_review` with `ai_decisions`, area filter); `alm show`; `alm checked` does not review; `alm reviewed` (group entry gone, history keeps each decision, nothing to review refused); the app's Reviewed path (name required, event as the person's name, decisions gone on reopen); the app's badge/filter logic and History wording.
  - In the built app: the owner sees the badge, the decisions and the button work on the trial taskbag.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | Owner request, following A00-29 (agents decide routine choices and note them on the card): an `alm` command records a decision; a badge in the app; an `alm human` group; the owner marks them reviewed in the app and with an `alm` command run only when the owner says so; A00-29's documents point agents to the command once it exists | owner (chat) |
| 2026-10-14 | Names: the command is `alm ai-decided`; the badge is "AI decided"; the `alm human` group is "To review: AI decisions". Whether `checked` also counts as reviewing is the worker's choice | owner (chat) |

## Notes

- **Decided (routine, A00-29), for the owner's review:**
  - **Stored only as history events**, not as a `task.json` field. Reason: older Allomos builds refuse unknown task fields, so a field would have needed format `allomos/5` and a migration; events with a new command and field name are read by older builds as they are (checked with the installed `alm` 0.1.4: `alm history` shows them, `alm list --json` reports no issues).
  - **Review command: `alm reviewed <id>`**, matching `alm checked <id>`. It reviews all of a task's decisions at once, as the Reviewed button does. Event field `ai_decision`; commands `ai-decided` and `reviewed`.
  - **`alm checked` does not count as reviewing.** Reason: checking means "it works when I try it"; reviewing means "I accept the choices the agent made". A person can do one without the other, and keeping them apart means a decision never disappears unread.
  - **Any task in any state can carry decisions, and To review lists them all** (including backlog). Reason: a decision made while drafting a task needs review as much as one made while building it. No holder rule for either command: recording changes nothing in the task, and the owner reviews tasks an agent holds.
  - Badge colour: the calm teal of "ok" (not the amber of "Not human-checked"), dashed like it: it asks for a look, not a fix.
  - Brief wording (v8): routine choices are the agent's; record each with `alm ai-decided`; what users will see and get used to, or anything hard to undo, goes to the approver first.
- Not done here: A00-29 itself (the dividing line in AGENTS.md § Always-on boundaries and CARDS.md) is still in backlog; when it is done, its texts should name `alm ai-decided`.
