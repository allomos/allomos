# Task-A07-09-windows-setup-wizard-check

*Card type: test. Area A07 (Platform & release). Written 2026-10-12 at the owner's request: a **very late** check, planned for the last iteration. **Not started.***

## What gets built (plain words)

The setup wizard ([`A05-03`](./Task-A05-03-taskbag-setup-wizard.md)) was checked by the owner on macOS only. Two of its parts have been seen by no one, because they only show on a computer unlike the owner's Mac:

| What | When it shows | Expected |
|---|---|---|
| "Git is not installed" note | Step 3 on a computer without Git | Amber note; the questions to ask the agent, including "How do I install Git?"; the wizard goes on |
| Paste tip | Step 4 with Codex (terminal) chosen | "…paste the request with **Ctrl+V** and press Enter…", under the option and in the message after **Connect** |

The rest of the wizard on Windows (first page, the four steps, Cancel and Not now to the start page) is checked at the same time.

## Details for the worker

- **Outcome.** The owner has seen both parts on Windows and they read right.
- **Decisive unknowns.** A Windows computer without Git (a fresh one, or Git uninstalled for the check) — owner.
- **Files owned.** None unless the check finds a problem (then a new card).
- **External operations.** None.
- **Done means mechanically.** The owner's notes from the Windows check, with the installer's name and date.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-12 — Card written at the owner's request; planned for the last iteration (I15).
