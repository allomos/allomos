# Task-A03-06-agent-handoff-rules

*Card type: coding. Area A03 (Agents). Set: main set, switching ([0014](../decisions/0014-main-purpose-and-three-sets.md)). Recorded for the owner, 2026-10-10.*

## What gets built (plain words)

An AI agent's session can end without warning when its quota (the usage allowance of its account) runs out or its context (how much of the conversation it can hold) fills up. If it has not written down where it stopped, the next agent or teammate starts blind. This card lets a person decide, per project, **when** an agent must write its handoff and **what** the handoff contains. Allomos then hands both to every agent that works the project.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Handoff rules | The project's settings: the **quota gate** (for example "with 3% of the 5-hour quota left, start nothing new; write the handoff and stop"), the **context gate** (for example "at 70% context used, update the handoff, commit, and give the owner a prompt for a new session"), and **when else to write** (after every commit, finished task or finding) | the taskbag's project settings |
| Default handoff template | A ready-made outline for one handoff: what was being done, where it stopped, what is verified and what is not, what runs, what comes next, and who it waits on. A project can edit or replace it | a template file in the taskbag, created by `alm init` |
| Telling the agent | `alm brief` (the short section agents read first) states the project's gates and points to the template; `alm guide` explains them in full | `alm` command |
| Setting them | `alm` commands to show and change the rules (like `alm task-language` and `alm labels`), and a settings screen in the desktop app | `alm`, app |

At the end the owner can open a project, set "quota 5%, context 60%", and see a newly started Claude Code or Codex session follow those gates and write its handoff in the template's shape.

## Details for the worker

- **Outcome.** A person can set a project's handoff gates and template; every agent working that project is told them through `alm brief` and writes its handoff accordingly.
- **Decisive unknowns.**
  - Can an agent read its own quota and context use? (Claude Code shows usage in some tools; Codex is unknown.) Where it cannot, the gate stays a written rule the agent follows as best it can. Overlaps with [`Task-A03-02`](./Task-A03-02-account-quota-idea.md); answerable by reading provider documentation.
  - Is a handoff one file per session, or the task's handoff note (`alm release --note`)? Owner to decide; the proposal is to keep the task's handoff note, structured by the template, so it travels with the task.
  - Do gates apply per project only, or also per agent or account? Owner to decide; the proposal is per project first.
- **Starting point.** This repository already follows such rules by hand ([AGENTS.md § Handoff](../../AGENTS.md#handoff--write-it-continuously), [AI-WORKFLOW § Handoff timing](../workflow/AI-WORKFLOW.md#handoff-timing-and-contents), [AGENT_HANDOFF.md](../AGENT_HANDOFF.md)); the defaults and template should come from that practice.
- **Files owned.** The `alm` crate's brief, guide and settings commands; taskbag project settings and a new template file; the app's settings screen and its texts in en, zh-CN and ja (R94); this card. Shared: the taskbag format document, if the settings change it.
- **External operations.** None.
- **Done means mechanically.**
  - `alm` shows and changes the quota gate, context gate and template; tests cover defaults, changes and refusals of invalid values (for example a percentage over 100).
  - `alm init` creates the default template; `alm brief` includes the gates and the template's path.
  - The app shows and edits the same settings; language files pass the missing-text test.
  - A trial in `~/fliphopper-tests/A03-06-handoff-rules/`: an agent session started in a project with non-default gates writes a handoff in the template's shape.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-10 — Asked for by the owner: let a user set handoff rules for an AI agent, such as the quota gate and the context gate, and provide a default handoff template. Checked that no existing card covers it; [`Task-A03-02`](./Task-A03-02-account-quota-idea.md) (showing quota left) is related but separate.
