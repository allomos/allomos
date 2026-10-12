# Task-A03-07-renew-agent-instructions

*Card type: coding. Area A03 (Agents). Recorded for the owner, 2026-10-10. **Dropped 2026-10-12: merged into [`A01-25`](./Task-A01-25-taskbag-format-upgrade.md)** at the owner's request; its decisions (version in the heading) carry over there, and the app now only hands the update to an agent.*

## What gets built (plain words)

A project's `AGENTS.md` (and `CLAUDE.md`) holds Allomos's section, the short instructions that tell AI agents how to work the taskbag with `alm`. The section was written by whichever Allomos version set the project up. When Allomos is upgraded, its section may change (new commands, new rules, for example the handoff gates of [`A03-06`](./Task-A03-06-agent-handoff-rules.md)), but the project's copy stays old and agents keep following old instructions. Today nothing notices this: the check from [`A03-04`](./Task-A03-04-agent-awareness-line.md) only asks whether the section is *there*, and "Write the files for me" leaves an existing section alone.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Section version | A number for each version of Allomos's section, raised whenever its wording changes, that Allomos can read back from a project's files | `crates/store` (section text), the project's `AGENTS.md` |
| The mismatch check | Compares the version found in `AGENTS.md` / `CLAUDE.md` with the version this Allomos writes: current, older (needs renewing), or unknown (a section without a version, written before this card or by hand) | `crates/store` |
| The prompt in the app | When the section is older or unknown, the agent line (A03-04) says "Allomos's instructions in AGENTS.md are out of date" with **Update…** | `apps/web`, in en, zh-CN and ja (R94) |
| Renewing | **Update…** works like the setup panel ([`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md)): it opens the chosen agent with a request to replace the old section with the new one, keeping the rest of the file; or, with no agent, **Write the files for me** replaces only the section. `alm` gets the same: a command that reports the mismatch and prints the request (and a write option) | `apps/desktop`, `alm` |

At the end the owner upgrades Allomos, opens an older project, sees "instructions out of date — Update…", presses it, and after the agent's edit the line turns back to ✓.

## Details for the worker

- **Outcome.** When a project's Allomos section is older than the app's, the app (and `alm`) says so and lets the person renew it in one step, without losing anything else in the file.
- **Decisive unknowns.**
  - **Where the version is written: in the section's heading** (owner, 2026-10-11), for example `## Tasks: Allomos (v3)`. No marker lines. A heading without a version counts as unknown.
  - **How to find the old section to replace** without markers: from its heading `## Tasks: Allomos` to the next heading of the same level is the proposal; an agent-led renewal can also handle sections the agent merged with other text.
  - **Dismissing the prompt — decided (owner, 2026-10-11): it comes back.** "Not now" hides the out-of-date line only while the project stays open; the next time the project is opened, the line shows again, until the section is renewed. *Why:* the instructions are important, so the reminder must not be lost.
- **Files owned.** The section text and the check in `crates/store`; the agent line and renewal panel in `apps/web` and `apps/desktop`; the `alm` command for reporting and renewing; language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: section at the current version → current; older version → older; no version → unknown; renewing replaces only the section and keeps the file's other text byte for byte; renewing twice changes nothing.
  - The app shows the out-of-date line and **Update…** for an older project, and ✓ after renewal; language files pass the missing-text test.
  - A trial in `~/allomos-tests/A03-07-renew-instructions/`: a project set up with an older section is renewed by an agent through **Update…**.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-11 | Card approved ("approve"): ready | owner (chat) |
| 2026-10-11 | The version goes in the section's heading, e.g. `## Tasks: Allomos (v3)` | owner (chat) |
| 2026-10-12 | Merged into A01-25 ("Merge A03-07 into A01-25") | owner (chat) |
| 2026-10-11 | "Not now" comes back each time the project is opened (option A: "A, because it is important") | owner (chat) |

## Notes

- 2026-10-10 — Asked for by the owner: renew the Allomos section in AGENTS.md when the app is upgraded, and have the interface prompt the person when the section's version does not match. Checked that no existing card covers it: A01-13, A01-14 and A03-03 write or request the section once, and A03-04 checks only that it is present.
- 2026-10-11 — Owner chose: the version goes in the section's heading. The "Not now" question was unclear and was asked again in plain words.
- 2026-10-11 — Owner chose option A for "Not now": the reminder comes back each time the project is opened. Recorded after the rename to Allomos finished ([0015](../decisions/0015-product-name-allomos.md)); this card now says Allomos, `alm` and `.allomos/taskbag/`. Its board title still says "FlipHopper" because `alm` cannot change a title. Next: the owner moves it to ready.
