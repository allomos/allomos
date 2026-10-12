# Task-A03-06-agent-handoff-rules

*Card type: coding. Area A03 (Agents). Set: main set, switching ([0014](../decisions/0014-main-purpose-and-three-sets.md)). Recorded for the owner, 2026-10-10; scope revised with the owner 2026-10-12. Storing and showing handoffs is the separate card [`A03-09`](./Task-A03-09-session-handoffs.md).*

## What gets built (plain words)

An AI agent's session can end without warning when its quota (the usage allowance of its account) runs out or its context (how much of the conversation it can hold) fills up. A person can also stop mid-task. If nobody wrote down where the work stopped, whoever continues starts blind. This card lets a person decide, per project, **when** a handoff must be written and **what** it contains, through a settings screen in the app and the same settings in `alm`. Allomos then tells every agent that works the project.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Handoff rules | The project's settings: the **quota gate** (default: with 3% of the 5-hour quota left, start nothing new; write the handoff and stop), the **context gate** (default: at 90% context used, finish the current step, write the handoff, commit, and give the owner the exact prompt for a new session), and **when else to write** (default: after every meaningful step: a task finished, a finding, a commit, a process started or stopped) | `project.json` in the taskbag |
| Overrides | An agent or account may have its own gates, which win over the project's (for example "Codex: context 80%") | `project.json` |
| Default handoff template | The outline every handoff follows, written for **any** reader, because the writer does not know who will take over (owner rule, 2026-10-12). Part 1, *for anyone, in plain words*: what was being done, where it stopped, what is verified and what is not, what waits on whom, the single next step. Part 2, *restart details*: checkout and branch, uncommitted changes, what is running and how to restart it, approvals still in force. Short fields, no room for evidence (that goes on the task's card). A project can edit or replace it | `handoff-template.md` in the taskbag, created by `alm init` |
| Telling the agent | `alm brief` (the short section agents read first) states the project's gates and points to the template; `alm guide` explains them in full | `alm` |
| Settings in `alm` | `alm handoff-rules` shows the rules; `alm handoff-rules --quota 5 --context 80` changes them; `--agent <agent or account>` sets an override; `alm handoff-template` shows or replaces the template. Invalid values are refused | `alm` |
| Settings screen in the app | A **Handoff** page in project settings: the two gates, when else to write, a button that opens the template, and a list of agent or account overrides | `apps/web`, in en, zh-CN and ja (R94) |

At the end the owner opens a project, sets "quota 5%, context 80%" in the app (or with `alm`), and sees a newly started Claude Code or Codex session follow those gates and write its handoff in the template's shape.

## Details for the worker

- **Outcome.** A person can set a project's handoff gates and template in the app or with `alm`; every agent working that project is told them through `alm brief` and writes its handoffs accordingly.
- **Decided.**
  - **One handoff file per session**, written from the template (owner, 2026-10-11). Where those files live and how they are shown: [`A03-09`](./Task-A03-09-session-handoffs.md).
  - **Gates per project, and also per agent or account**; the agent or account setting overrides the project's (owner, 2026-10-11).
  - **Default context gate 90%** (owner, 2026-10-12; the owner raised it from 70% in the SSR project on 2026-10-08). Default quota gate 3%.
  - **Settings in both the app and `alm`** (owner, 2026-10-12).
  - **No "from → to" in a handoff: we do not know who will take over** (owner rule, 2026-10-12). The template is written so either an agent or a person can continue.
- **Decisive unknown.** Can an agent read its own quota and context use? Claude Code can (`get_usage`); Codex is unknown. Where it cannot, the gate stays a written rule the agent follows as best it can. Overlaps with [`Task-A03-02`](./Task-A03-02-account-quota-idea.md); answerable by reading provider documentation.
- **Starting point.** This repository and the SSR project already follow such rules by hand ([AGENTS.md § Handoff](../../AGENTS.md#handoff--write-it-continuously), [AI-WORKFLOW § Handoff timing](../workflow/AI-WORKFLOW.md#handoff-timing-and-contents), [AGENT_HANDOFF.md](../AGENT_HANDOFF.md); the SSR project's `AI-WORKFLOW.md` and `AGENT_HANDOFF.md`, see [REFERENCES](../REFERENCES.md)). The defaults and template come from that practice. Lesson from SSR: its handoff prompt once pointed to a template that did not exist and used a vague threshold, so the template must be a real file and `alm brief` must state exact numbers.
- **Files owned.** The `alm` crate's brief, guide, `handoff-rules` and `handoff-template` commands; taskbag `project.json` settings and the template file; the app's Handoff settings page and its texts in en, zh-CN and ja; this card. Shared: the taskbag format document ([FILE-FORMAT.md](../product/FILE-FORMAT.md)), for the new settings and template file.
- **External operations.** None.
- **Done means mechanically.**
  - `alm` shows and changes the quota gate, context gate, overrides and template; tests cover defaults (3%, 90%), changes, overrides winning over the project, and refusals of invalid values (for example a percentage over 100).
  - `alm init` creates the default template; `alm brief` includes the gates (with any override for the asking agent) and the template's path.
  - The app's Handoff page shows and edits the same settings; changes made in one are seen in the other; language files pass the missing-text test.
  - A trial in `~/allomos-tests/A03-06-handoff-rules/`: an agent session started in a project with non-default gates writes a handoff in the template's shape.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | Handoff is one file per session ("per session"); gates per project and also per agent or account ("accept this") | owner (chat) |
| 2026-10-12 | Settings screen in the app and settings in `alm` ("both gui and cli"); default context gate 90% ("accept this"); no "from → to": "we don't know who will take over when we write a handoff"; storage and display split into A03-09 | owner (chat) |

## Notes

- 2026-10-10 — Asked for by the owner: let a user set handoff rules for an AI agent, such as the quota gate and the context gate, and provide a default handoff template. Checked that no existing card covers it; [`Task-A03-02`](./Task-A03-02-account-quota-idea.md) (showing quota left) is related but separate.
- 2026-10-12 — Discussed with the owner after reading the SSR project's handoff rules. Added the settings screen and `alm` commands, the 90% default, the any-reader template, and moved storage and the Handoffs tab to [`A03-09`](./Task-A03-09-session-handoffs.md). Next: the owner approves the card.
