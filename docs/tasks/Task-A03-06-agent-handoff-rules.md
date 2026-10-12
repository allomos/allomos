# Task-A03-06-agent-handoff-rules

*Card type: coding. Area A03 (Agents). Set: main set, switching ([0014](../decisions/0014-main-purpose-and-three-sets.md)). Recorded for the owner, 2026-10-10; scope revised with the owner 2026-10-12, when [`A03-09`](./Task-A03-09-session-handoffs.md) (storing and showing handoffs) was merged into this card at the owner's request.*

## What gets built (plain words)

An AI agent's session can end without warning when its quota (the usage allowance of its account) runs out or its context (how much of the conversation it can hold) fills up. A person can also stop mid-task. If nobody wrote down where the work stopped, whoever continues starts blind. Today the note left behind, the **handoff**, lives in one hand-kept file per project (`docs/AGENT_HANDOFF.md`); in the SSR project that file grew to hundreds of lines because every session added to it and nobody removed old notes, so the next agent had to dig to find what mattered.

After this card:

- a person decides, per project, **when** a handoff must be written and **what** it contains, in a settings page in the app or with `alm`, and Allomos tells every agent that works the project;
- every session, by an agent or a person, writes **its own small handoff file** into the taskbag, naming the tasks it covers;
- the app shows handoffs in a **Handoffs tab** and beside each task; only the latest are shown by default, so old ones never get in the way.

### Rules and template

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Handoff rules | The **quota gate** (default: with 3% of the 5-hour quota left, start nothing new; write the handoff and stop), the **context gate** (default: at 90% context used, finish the current step, write the handoff, commit, and give the owner the exact prompt for a new session), and **when else to write** (default: after every meaningful step: a task finished, a finding, a commit, a process started or stopped) | `project.json` in the taskbag |
| Overrides | An agent or account may have its own gates, which win over the project's (for example "Codex: context 80%") | `project.json` |
| Handoff template | The outline every handoff follows, written for **any** reader, because the writer does not know who will take over. Part 1, *for anyone, in plain words*: what was being done, where it stopped, what is verified and what is not, what waits on whom, the single next step. Part 2, *restart details*: checkout and branch, uncommitted changes, what is running and how to restart it, approvals still in force. Short fields with no room for evidence (that goes on the task's card). A project can edit or replace it | `handoff-template.md` in the taskbag, created by `alm init` |
| Telling the agent | `alm brief` (the short section agents read first) states the gates (with any override for the asking agent) and how to write a handoff; `alm guide` explains them in full | `alm` |
| Settings in `alm` | `alm handoff-rules` shows the rules; `alm handoff-rules --quota 5 --context 80` changes them; `--agent <agent or account>` sets an override; `alm handoff-template` shows or replaces the template. Invalid values are refused | `alm` |
| Settings page in the app | A **Handoff** page in project settings: the two gates, when else to write, a button that opens the template, and the list of agent or account overrides | `apps/web`, in en, zh-CN and ja (R94) |

### Storing and showing handoffs

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Handoff files | One file per session, from the template. A short header lists the tasks it covers (`tasks: A07-01-27, A07-01-28`), because one session may work several tasks in a row; a session with no task (for example a planning discussion) leaves the list empty | `.allomos/taskbag/handoffs/<date-time>-<writer>.md`, beside `tasks/` |
| Project notes | What every session must know whatever its task: what is running, pitfalls, where credentials live (never their values) | `.allomos/taskbag/project-notes.md` |
| Writing a handoff | `alm handoff --tasks <ids>` creates this session's file from the template (or updates it if the session already has one) and prints its path; `--push` commits and pushes it | `alm` |
| Finding handoffs | `alm next` and `alm show <id>` show the latest handoff naming that task; `alm handoffs` lists them, newest first | `alm` |
| Handoffs tab | A tab after **Iterations**: every handoff in the project, newest first, with date, writer (agent, account or person), tasks covered and first line. Opening one shows it in full; its tasks are links. Filters: by task, by writer, only the latest | `apps/web`, in en, zh-CN and ja |
| Task pane link | The task pane shows the selected task's latest handoff, with a link to all of them | `apps/web` |
| Writing as a person | A simple form in the app: part 1 (for anyone) is required; part 2 (restart details) is optional | `apps/web`, `apps/desktop` |

At the end the owner opens a project, sets "quota 5%, context 80%" in the app (or with `alm`), and sees a newly started Claude Code or Codex session follow those gates and write its handoff in the template's shape; the handoff appears in the Handoffs tab and in the task pane of each task it covers, and a new session started on one of those tasks reads it through `alm next`.

## Details for the worker

- **Outcome.** A person can set a project's handoff gates and template in the app or with `alm`; every session (agent or person) writes its handoff into the taskbag in the template's shape; anyone can find a task's handoffs in `alm`, the Handoffs tab and the task pane.
- **Decided.**
  - **One handoff file per session**, written from the template (owner, 2026-10-11).
  - **Gates per project, and also per agent or account**; the agent or account setting overrides the project's (owner, 2026-10-11).
  - **Default context gate 90%** (owner, 2026-10-12; the owner raised it from 70% in the SSR project on 2026-10-08). Default quota gate 3%.
  - **Settings in both the app and `alm`** (owner, 2026-10-12).
  - **We do not know who will take over when we write a handoff** (owner rule, 2026-10-12): no "from → to"; the template is written so either an agent or a person can continue.
  - **Handoffs live in the taskbag, in a `handoffs` folder beside `tasks`**, not inside a task's folder, because one session may run several tasks (owner, 2026-10-12).
  - **A Handoffs tab after the Iterations tab** (owner, 2026-10-12).
  - **Built in, not left as advice** (owner, 2026-10-12): one small file per session; short template fields, evidence goes on the card; only the latest shown by default, older ones kept as history.
  - **One card** for rules, template, storage and display (owner, 2026-10-12).
  - **The gate numbers are not written into `AGENTS.md`**: they would make the section out of date whenever a gate changes. The section says how to ask; `alm next` tells each agent its current gates (with its override) at the start of every session (worker, 2026-10-12).
  - **A session is identified by its claim name (`--as`)**: `alm handoff` updates that session's newest file. A person writing in the app makes a new file each time. The task's one-line `handoff_note` stays.
  - **Our own taskbag is upgraded to `allomos/2` right after the merge** (owner, 2026-10-12).
  - **This repository moves off `docs/AGENT_HANDOFF.md` now** (owner, 2026-10-12): Live state, Pitfalls and Current truth go to `project-notes.md`, each session writes `alm handoff`, the file is retired.
  - **The setup wizard's "existing tasks" step moves a project's handoffs too** (owner, 2026-10-12): the conversion request (v7) asks the agent to turn each current handoff into a handoff file, project-wide notes into `project-notes.md`, and stated rules into `alm handoff-rules`.
- **Decisive unknowns.**
  - Can an agent read its own quota and context use? Claude Code can (`get_usage`); Codex is unknown. Where it cannot, the gate stays a written rule the agent follows as best it can. Overlaps with [`Task-A03-02`](./Task-A03-02-account-quota-idea.md); answerable by reading provider documentation.
  - How a session is identified for "update this session's file instead of making a new one": the claim name used with `alm claim --as`, or a session name given to `alm handoff`.
  - The task's existing `handoff_note` (set by `alm release --note`, [FILE-FORMAT.md](../product/FILE-FORMAT.md)). Proposal: keep it as a one-line pointer; the handoff file holds the detail.
  - How this project moves off `docs/AGENT_HANDOFF.md` (its Live state, Pitfalls and Current truth become `project-notes.md`), as the last step, with the owner's approval.
- **Depends on** the taskbag format upgrade card [`A01-25`](./Task-A01-25-taskbag-format-upgrade.md): the new settings, template, `handoffs/` folder and `project-notes.md` change the taskbag's format, and today an older `alm` simply refuses a taskbag with fields it does not know (`field_unknown`).
- **Starting point.** This repository and the SSR project already follow such rules by hand ([AGENTS.md § Handoff](../../AGENTS.md#handoff--write-it-continuously), [AI-WORKFLOW § Handoff timing](../workflow/AI-WORKFLOW.md#handoff-timing-and-contents), [AGENT_HANDOFF.md](../AGENT_HANDOFF.md); the SSR project's `AI-WORKFLOW.md` and `AGENT_HANDOFF.md`, see [REFERENCES](../REFERENCES.md)). The defaults and template come from that practice. Lesson from SSR: its handoff prompt once pointed to a template that did not exist and used a vague threshold, so the template must be a real file and `alm brief` must state exact numbers.
- **Files owned.** The `alm` crate's brief, guide, `handoff-rules`, `handoff-template`, `handoff` and `handoffs` commands and the handoff part of `next` and `show`; taskbag `project.json` settings, the template file, `handoffs/` and `project-notes.md`; the taskbag format document ([FILE-FORMAT.md](../product/FILE-FORMAT.md)) for these; the app's Handoff settings page, Handoffs tab, task-pane link and handoff form; language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Settings: `alm` shows and changes the quota gate, context gate, overrides and template; tests cover defaults (3%, 90%), changes, an override winning over the project, and refusals of invalid values (for example a percentage over 100). `alm init` creates the default template; `alm brief` includes the gates and how to write a handoff.
  - Handoffs: `alm handoff` creates a file from the template with the right header; a second call in the same session updates it; `alm show <id>` and `alm next` find the latest handoff naming the task, including one covering several tasks; a handoff with no tasks is listed but linked to none; `alm list --json` reports no `issues` with handoffs present.
  - App: the Handoff settings page edits the same settings as `alm` (a change in one is seen in the other); the Handoffs tab after Iterations with its filters; the task pane's latest handoff; the form for a person. Language files pass the missing-text test.
  - A trial in `~/allomos-tests/A03-06-handoffs/`: in a project with non-default gates, one agent session works two tasks and writes a handoff in the template's shape; a second session started on the second task finds and follows it; the owner sees it in the Handoffs tab.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | Handoff is one file per session ("per session"); gates per project and also per agent or account ("accept this") | owner (chat) |
| 2026-10-12 | Settings page in the app and settings in `alm` ("both gui and cli"); default context gate 90% ("accept this"); "we don't know who will take over when we write a handoff" | owner (chat) |
| 2026-10-12 | Handoffs stored under the taskbag in a folder beside `tasks` ("a session run multiple tasks"); a Handoffs tab after the Iterations tab; the built-in short handoffs ("i accept these designs") | owner (chat) |
| 2026-10-12 | A03-09 merged into this card ("combine them into 1 task") | owner (chat) |
| 2026-10-12 | Proceed with the card ("proceed with a03-06"); upgrade our taskbag right after merge; move AGENT_HANDOFF.md now and "add this feature to the move current task step of the wizard process" | owner (chat) |

## Notes

- 2026-10-10 — Asked for by the owner: let a user set handoff rules for an AI agent, such as the quota gate and the context gate, and provide a default handoff template. Checked that no existing card covers it; [`Task-A03-02`](./Task-A03-02-account-quota-idea.md) (showing quota left) is related but separate.
- 2026-10-12 — Discussed with the owner after reading the SSR project's handoff rules and its 722-line handoff history. Added the settings page and `alm` commands, the 90% default, the any-reader template, handoff storage and the Handoffs tab; A03-09, opened for storage and display the same day, was merged back here and dropped. Next: the owner approves the card.
