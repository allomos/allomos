# Task-A03-02-account-quota-idea

*Card type: research (idea, parked). Area A03 (Agents). Set: main set, switching ([0014](../decisions/0014-main-purpose-and-three-sets.md)). Recorded for the owner, 2026-10-06.*

## The idea (plain words)

FlipHopper could show **how much of each AI agent account's quota is left** (Claude Code, Codex, several accounts) and **suggest when to switch** to another agent or account, before a session is cut off in the middle of a task. This serves the main set directly: switching between agents before the quota ends, instead of after.

## What a research card would find out

- What each provider lets a program read: remaining usage, reset time, plan limits (official interfaces only; no scraping of web pages and no use of someone's sign-in cookies).
- Whether the agent itself can report it (for example a usage command inside the session) so `fh` can record it with the task.
- What "suggest a switch" would look like: a warning in the app, a note in the handoff, or a line in `fh next`.

## Details for the worker

- **State:** parked. The owner was not sure whether to research it now or later (2026-10-06). It is not started until the owner moves it to `ready`.
- **Files owned:** this card, and a research report in `docs/research/` if it runs.
- **External operations:** none expected; reading public provider documentation only.
- **Done means:** a short report of what each provider exposes, with sources, and a recommendation for or against building it.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-06 — Idea raised while reorganizing the deck ([`Task-A00-18`](./Task-A00-18-positioning-and-deck.md)); the owner asked to record it.
