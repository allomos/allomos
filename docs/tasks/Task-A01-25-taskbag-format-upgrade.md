# Task-A01-25-taskbag-format-upgrade

*Card type: coding. Area A01 (Tracking). Written 2026-10-12 at the owner's request, while discussing new Allomos versions. Needed by [`A03-06`](./Task-A03-06-agent-handoff-rules.md), the first card to add new things to the taskbag since `allomos/1`.*

## What gets built (plain words)

A taskbag (the `.allomos/taskbag/` folder holding a project's tasks) is written in a **format**: which files and fields it has. `project.json` names it, today `"format": "allomos/1"` ([FILE-FORMAT.md](../product/FILE-FORMAT.md)). New Allomos versions will add to it: [`A03-06`](./Task-A03-06-agent-handoff-rules.md) adds handoff settings, a template, a `handoffs/` folder and `project-notes.md`. Two things can then go wrong:

1. **A newer Allomos opens an older taskbag.** Its files lack the new parts. Allomos must bring the taskbag up to the new format (an *upgrade*).
2. **An older Allomos or `alm` meets a newer taskbag**, for example a teammate who has not updated Allomos pulls a taskbag a newer version already upgraded. Today it refuses with a technical error (`field_unknown`) that neither a person nor an agent can act on.

After this card both are handled clearly.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Format number | The format gets a number that goes up when the taskbag changes shape (`allomos/1`, `allomos/2`, …); each Allomos knows the highest number it understands | `crates/store`, FILE-FORMAT.md |
| Upgrade (newer meets older) | Allomos brings an older taskbag up to its format: adds the new files and settings with their defaults, changes nothing else, and raises the number. Each step from one number to the next is a small, tested change | `crates/store`; the app and `alm` |
| "Update Allomos now" (older meets newer) | **Owner rule (2026-10-12): it tells the person or agent right away that Allomos must be updated now.** The app shows it at once when the project opens; `alm` stops with a plain message and its own exit code, so the agent tells the person instead of guessing. The message names both versions and how to update | `apps/web`, `apps/desktop`, `alm`, in en, zh-CN and ja (R94) |
| Upgrade in `alm` | `alm upgrade` shows what would change and does it; other `alm` commands on an older taskbag say an upgrade is needed | `alm` |

At the end the owner opens an `allomos/1` project in a newer Allomos and it is upgraded; then, with an older `alm`, the same project says "this taskbag needs a newer Allomos — update Allomos now", and an agent passes that on to the owner.

## Details for the worker

- **Outcome.** A newer Allomos upgrades an older taskbag; an older Allomos or `alm` that meets a newer taskbag tells the person or agent at once to update Allomos.
- **Decided.**
  - Older meets newer: prompt the app's user or the agent right away that Allomos must be updated (owner, 2026-10-12).
- **Decisive unknowns.**
  - **Upgrade by itself, or ask first?** When a newer Allomos opens an older taskbag, does it change the files on its own, or first show "this project will be upgraded; teammates must then update Allomos too — Upgrade / Not now"? It matters because a taskbag is often shared through Git: once upgraded and pushed, every teammate on an older Allomos is stopped until they update. Owner to decide.
  - While an older Allomos waits to be updated, may it still *show* the newer taskbag's tasks (read only), or show nothing but the message?
  - Whether an upgrade also renews Allomos's section in `AGENTS.md` ([`A03-07`](./Task-A03-07-renew-agent-instructions.md)), since a new format usually comes with new instructions for agents.
- **Files owned.** Format reading, checks and upgrade steps in `crates/store`; `alm upgrade` and the new exit code; the app's upgrade and "update Allomos now" messages; FILE-FORMAT.md's format and versioning section; language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: an `allomos/1` taskbag upgrades to the current format, gaining only the new parts with their defaults, every task unchanged; upgrading twice changes nothing; a taskbag with a higher number than this Allomos understands gives the "update Allomos now" result in `alm` (message and exit code) and in the app, and nothing is written to it.
  - Language files pass the missing-text test.
  - A trial in `~/allomos-tests/A01-25-format-upgrade/`: an old project is upgraded by the newer app, then opened with an older `alm` by an agent, which reports that Allomos must be updated.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | The new-version question is about the taskbag's format ("i am talking about this"); an older Allomos meeting a newer taskbag "should prompt the gui or agent that user should update … right now" | owner (chat) |

## Notes

- 2026-10-12 — Asked for by the owner. Checked that no existing card covers it: [`A03-07`](./Task-A03-07-renew-agent-instructions.md) renews the agent instructions in `AGENTS.md`, not the taskbag's files; FILE-FORMAT.md today only says older `alm` builds refuse unknown fields ("reinstall `alm`"). Next: the owner answers "upgrade by itself or ask first" and approves the card.
