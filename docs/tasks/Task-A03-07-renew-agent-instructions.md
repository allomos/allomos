# Task-A03-07-renew-agent-instructions

*Card type: coding. Area A03 (Agents). Recorded for the owner, 2026-10-10.*

## What gets built (plain words)

A project's `AGENTS.md` (and `CLAUDE.md`) holds FlipHopper's section, the short instructions that tell AI agents how to work the seabag with `fh`. The section was written by whichever FlipHopper version set the project up. When FlipHopper is upgraded, its section may change (new commands, new rules, for example the handoff gates of [`A03-06`](./Task-A03-06-agent-handoff-rules.md)), but the project's copy stays old and agents keep following old instructions. Today nothing notices this: the check from [`A03-04`](./Task-A03-04-agent-awareness-line.md) only asks whether the section is *there*, and "Write the files for me" leaves an existing section alone.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Section version | A number for each version of FlipHopper's section, raised whenever its wording changes, that FlipHopper can read back from a project's files | `crates/store` (section text), the project's `AGENTS.md` |
| The mismatch check | Compares the version found in `AGENTS.md` / `CLAUDE.md` with the version this FlipHopper writes: current, older (needs renewing), or unknown (a section without a version, written before this card or by hand) | `crates/store` |
| The prompt in the app | When the section is older or unknown, the agent line (A03-04) says "FlipHopper's instructions in AGENTS.md are out of date" with **Update…** | `apps/web`, in en, zh-CN and ja (R94) |
| Renewing | **Update…** works like the setup panel ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)): it opens the chosen agent with a request to replace the old section with the new one, keeping the rest of the file; or, with no agent, **Write the files for me** replaces only the section. `fh` gets the same: a command that reports the mismatch and prints the request (and a write option) | `apps/desktop`, `fh` |

At the end the owner upgrades FlipHopper, opens an older project, sees "instructions out of date — Update…", presses it, and after the agent's edit the line turns back to ✓.

## Details for the worker

- **Outcome.** When a project's FlipHopper section is older than the app's, the app (and `fh`) says so and lets the person renew it in one step, without losing anything else in the file.
- **Decisive unknowns.**
  - **Where the version is written.** The owner asked for no marker lines in the file (A01-14, 2026-10-08). Options: a short visible line inside the section (for example "FlipHopper instructions, version 3"); the version in the heading; or the version recorded in the seabag's project settings when the section is written, plus a comparison of the section's text. Owner to choose.
  - **How to find the old section to replace** without markers: from its heading `## Tasks: FlipHopper` to the next heading of the same level is the proposal; an agent-led renewal can also handle sections the agent merged with other text.
  - **Snoozing:** whether "Not now" hides the prompt until the next app version. Owner to choose; proposal: yes.
- **Files owned.** The section text and the check in `crates/store`; the agent line and renewal panel in `apps/web` and `apps/desktop`; the `fh` command for reporting and renewing; language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: section at the current version → current; older version → older; no version → unknown; renewing replaces only the section and keeps the file's other text byte for byte; renewing twice changes nothing.
  - The app shows the out-of-date line and **Update…** for an older project, and ✓ after renewal; language files pass the missing-text test.
  - A trial in `~/fliphopper-tests/A03-07-renew-instructions/`: a project set up with an older section is renewed by an agent through **Update…**.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-10 — Asked for by the owner: renew the FlipHopper section in AGENTS.md when the app is upgraded, and have the interface prompt the person when the section's version does not match. Checked that no existing card covers it: A01-13, A01-14 and A03-03 write or request the section once, and A03-04 checks only that it is present.
