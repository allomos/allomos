# Task-A03-09-session-handoffs

*Card type: coding. Area A03 (Agents). Set: main set, switching ([0014](../decisions/0014-main-purpose-and-three-sets.md)). Written 2026-10-12 at the owner's request, split from [`A03-06`](./Task-A03-06-agent-handoff-rules.md), which sets the rules and the template.*

## What gets built (plain words)

Today a handoff (the short note a session leaves so the next one can continue) lives in one hand-kept file per project, `docs/AGENT_HANDOFF.md`. In the SSR project that file grew to hundreds of lines because every session added to it and nobody removed old notes; the next agent had to dig to find what mattered. After this card, every session, by an agent or a person, writes **its own small handoff file** into the taskbag, the file names the tasks it covers, and the app shows handoffs in their own tab and beside each task. Only the latest are shown by default, so old ones never get in the way.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Handoff files | One file per session, from the template of A03-06. A short header lists the tasks it covers (`tasks: A07-01-27, A07-01-28`), because one session may work several tasks in a row; a session with no task (for example a planning discussion) leaves the list empty | `.allomos/taskbag/handoffs/<date-time>-<writer>.md`, next to `tasks/` |
| Project notes | What every session must know whatever its task: what is running, pitfalls, where credentials live (never their values) | `.allomos/taskbag/project-notes.md` |
| Writing a handoff | `alm handoff --tasks <ids>` creates the session's file from the template (or updates it if this session already has one) and prints its path; `alm handoff --push` commits and pushes it | `alm` |
| Finding handoffs | `alm next` and `alm show <id>` show the latest handoff naming that task; `alm handoffs` lists them, newest first | `alm` |
| Handoffs tab | A tab after **Iterations**: every handoff in the project, newest first, with date, writer (agent, account or person), tasks covered and first line. Opening one shows it in full; its tasks are links. Filters: by task, by writer, only the latest | `apps/web`, in en, zh-CN and ja (R94) |
| Task pane link | The task pane shows the selected task's latest handoff, with a link to all of them | `apps/web` |
| Writing as a person | A simple form in the app: fill in part 1 (for anyone, in plain words); part 2 (restart details) is optional | `apps/web`, `apps/desktop` |

At the end the owner opens the Handoffs tab, sees the handoffs of the last sessions, clicks a task in one, and sees the same handoff in the task pane; a new agent session started on that task reads it through `alm next`.

## Details for the worker

- **Outcome.** Handoffs are stored in the taskbag, one file per session, linked to the tasks they cover, and anyone (agent or person) can write and find them in `alm` and the app.
- **Decided (owner, 2026-10-12).**
  - Handoffs live in the taskbag, in a `handoffs` folder **beside** `tasks`, not inside a task's folder, because one session may run several tasks.
  - A **Handoffs tab after the Iterations tab** shows the project's handoffs.
  - Built in, not left as advice: one small file per session; short template fields with no room for evidence (evidence goes on the card); only the latest shown by default, older ones kept as history.
  - Handoffs are written for any reader: the writer does not know who takes over (agents or people).
- **Decisive unknowns.**
  - How a session is identified for "update this session's file instead of making a new one": the claim name used with `alm claim --as`, or a session name given to `alm handoff`.
  - How this project moves off `docs/AGENT_HANDOFF.md` (its Live state, Pitfalls and Current truth sections become `project-notes.md`); the old file is then retired. A step at the end of this card, approved by the owner.
  - The task's existing `handoff_note` (set by `alm release --note`, [FILE-FORMAT.md](../product/FILE-FORMAT.md)): keep it as a one-line pointer, or fold it into handoff files. Proposal: keep it; the release note stays a one-liner and the handoff file holds the detail.
- **Depends on** [`A03-06`](./Task-A03-06-agent-handoff-rules.md) for the template file.
- **Files owned.** The `alm` crate's `handoff` and `handoffs` commands and the handoff part of `next` and `show`; the taskbag format document's new `handoffs/` folder and `project-notes.md`; the Handoffs tab, task-pane link and handoff form in `apps/web` (and any `apps/desktop` commands they need); language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: `alm handoff` creates a file from the template with the right header; a second call in the same session updates it; `alm show <id>` and `alm next` find the latest handoff naming the task, including one that covers several tasks; a handoff with no tasks is listed but linked to none; `alm list --json` reports no `issues` with handoffs present.
  - The app shows the Handoffs tab after Iterations with its filters, the task pane's latest handoff, and the form for a person; language files pass the missing-text test.
  - A trial in `~/allomos-tests/A03-09-session-handoffs/`: one agent session works two tasks and writes a handoff; a second session started on the second task finds and follows it; the owner sees both in the Handoffs tab.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Handoffs stored under the taskbag, in a folder beside `tasks` ("why not … the same level of the tasks? … a session run multiple tasks"); a Handoffs tab after the Iterations tab; built-in short handoffs ("i accept these designs"); no "from → to" | owner (chat) |

## Notes

- 2026-10-12 — Asked for by the owner while discussing [`A03-06`](./Task-A03-06-agent-handoff-rules.md), after reading the SSR project's `AGENT_HANDOFF.md` and its 722-line history file. Checked that no other card covers storing or showing handoffs. Next: the owner approves the card.
