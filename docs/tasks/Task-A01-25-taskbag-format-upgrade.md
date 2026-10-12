# Task-A01-25-taskbag-format-upgrade

*Card type: coding. Area A01 (Tracking). Written 2026-10-12 at the owner's request, while discussing new Allomos versions; [`A03-07`](./Task-A03-07-renew-agent-instructions.md) merged in the same day. Needed by [`A03-06`](./Task-A03-06-agent-handoff-rules.md), the first card to add new things to the taskbag since `allomos/1`.*

## What gets built (plain words)

A taskbag (the `.allomos/taskbag/` folder holding a project's tasks) is written in a **format**: which files and fields it has. `project.json` names it, today `"format": "allomos/1"` ([FILE-FORMAT.md](../product/FILE-FORMAT.md)). A project's `AGENTS.md` / `CLAUDE.md` also hold **Allomos's section**, the short instructions that tell AI agents how to work the taskbag; it too was written by some Allomos version. New Allomos versions change both: [`A03-06`](./Task-A03-06-agent-handoff-rules.md) adds handoff settings and files to the taskbag and new rules to the section. Today nothing notices when they are out of date, and an older `alm` that meets a newer taskbag fails with a technical error (`field_unknown`) that neither a person nor an agent can act on.

After this card, every time the project is opened in the app and every time an agent runs `alm`, Allomos **checks** both versions. **Allomos never rewrites the taskbag or the md files on its own** (owner, 2026-10-12): the running agent session does the update; the app only shows a button that hands the job to an agent.

| Piece | What it is in everyday words | Where it lives |
|---|---|---|
| Format number | `allomos/1`, `allomos/2`, …: goes up when the taskbag changes shape; each Allomos knows the number it writes | `crates/core`, FILE-FORMAT.md |
| Section version | The section's heading carries its version, `## Tasks: Allomos (v1)` (decided in A03-07, owner 2026-10-11); a heading without one counts as out of date | `crates/store` (section text) |
| The check | Taskbag: same, **older** or **newer** than this Allomos. Section in `AGENTS.md` / `CLAUDE.md`: current, or **out of date** (older or no version). Only made once an agent is connected: the section is there and the project folder is known (owner, 2026-10-12) | `crates/store` |
| `alm`, on every call | Taskbag **older**: the command is refused with a message telling the agent to run `alm upgrade` now, then repeat the command. Section **out of date**: the command runs, and a warning tells the agent to replace the section with `alm brief`'s output now. Taskbag **newer**: `alm` stops (own exit code) and tells the agent to let the person choose: update Allomos, or let the agent go on working with the files from its own understanding of the newer format (owner, 2026-10-12) | `alm` |
| `alm upgrade` | Run by the agent: brings an older taskbag up to this Allomos's format, adding only the new parts with their defaults; says when the section must be renewed too | `alm`, `crates/store` |
| The app | On every trigger below, an out-of-date taskbag or section shows a notice with **Update the taskbag…**, which opens the Connect panel (wizard step 4): the chosen agent opens with the **update request** typed in, and the person sends it. A newer taskbag shows "This taskbag needs a newer Allomos" | `apps/web`, `apps/desktop`, in en, zh-CN and ja (R94) |

**App triggers** (owner, 2026-10-12): File → Open Folder… (⌘O) or **Choose a folder…**; File → Open Recent; app start (last folder, or the folder given at start); setup wizard step 2 finding an existing taskbag; the taskbag's files changing while open (for example after `git pull`); and **File → Connect with the Agents…**, whose request then is the update request.

At the end the owner opens a project whose section has no version: the app shows **Update the taskbag…**; an agent run on it renews the section and the notice goes away. An agent running `alm next` in that project is told the same. A taskbag marked with a newer format makes `alm` stop with the two choices, and the app say it needs a newer Allomos.

## Details for the worker

- **Outcome.** Opening a project in the app, or running `alm`, finds an out-of-date taskbag or section and hands the update to the running agent; a newer taskbag is reported with the person's two choices.
- **Decided (owner, 2026-10-12).**
  - Card A03-07 (renew the section after an upgrade) is merged into this card and dropped.
  - Allomos does not update the taskbag or md files itself; the running agent session does. From the app, only a button.
  - The six app triggers above; checks only once an agent is connected with the taskbag and project.
  - The button reads **Update the taskbag…**.
  - A newer taskbag: the person chooses between updating Allomos and letting the agent go on from its understanding of the newer format.
  - "Not now" and "no agent found" are covered by **File → Connect with the Agents…**, available at any time.
- **Test data.** There is no `allomos/2` yet (A03-06 makes the first). The upgrade mechanism is tested with a made-up step from 1 to 2; the newer case with a taskbag marked `allomos/2`; the section with real files without a version.
- **Files owned.** Format and section version checks and the upgrade steps in `crates/core` and `crates/store`; `alm upgrade`, the check before every `alm` command, the new codes and exit code; the app's notice, button and update request; FILE-FORMAT.md and CODING_RULES (the `alm` contract); language files; this card.
- **External operations.** None.
- **Done means mechanically.**
  - Tests: the upgrade adds only the new parts and leaves every task unchanged; upgrading twice changes nothing; a newer taskbag gives `taskbag_newer` in `alm` (message and exit code 4) and writes nothing; an older taskbag refuses commands with `taskbag_older` until `alm upgrade`; a section without a version, or an older one, gives the `instructions_outdated` warning; a current one gives none; a project with no section gives none.
  - The app shows the notice and **Update the taskbag…** for an out-of-date project, and "needs a newer Allomos" for a newer one; language files pass the missing-text test.
  - A trial in `~/allomos-tests/A01-25-format-upgrade/` with test data covering each case.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | Run this card first, tested with test data; merge A03-07 into it; Allomos never updates the files itself, the agent does; the app triggers; "Update the taskbag…"; the two choices for a newer taskbag | owner (chat) |
| 2026-10-12 | The new-version question is about the taskbag's format ("i am talking about this"); an older Allomos meeting a newer taskbag "should prompt the gui or agent that user should update … right now" | owner (chat) |

## Notes

- 2026-10-12 — Asked for by the owner. Checked that no existing card covers it: [`A03-07`](./Task-A03-07-renew-agent-instructions.md) renews the agent instructions in `AGENTS.md`, not the taskbag's files; FILE-FORMAT.md today only says older `alm` builds refuse unknown fields ("reinstall `alm`"). The owner then asked to run it first, merged A03-07 into it and settled the open questions (see Decided).
- 2026-10-12 — **Done, owner verified.** Built on `claude/a01-25-format-upgrade`. Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` (core 26, store 42, desktop 15 + 1 ignored, alm 23), web typecheck, build, `check:i18n` (203 texts, 54 codes) and `npm test` (13): all PASS. Trial in `~/allomos-tests/A01-25-format-upgrade/` (old section, newer taskbag, not connected, current, app check): the owner passed all ten steps in the app and terminal: notice and **Update the taskbag…**, update-mode panel, File → Connect with the Agents…, notice gone after renewal, no notice when current or not connected, newer taskbag (wizard step 2 with Next disabled, problem list titled "Please update Allomos" with **Download the newest Allomos** → GitHub releases), Open Recent and app start, en / zh-CN / ja, and `alm` (warning; `taskbag_newer`, exit 4). Fixed while checking, at the owner's request: the wizard's Git commit raced with itself in development (`index.lock`), now one at a time; the newer-taskbag message no longer quotes agent English; "will open…" wording, shown only beside the chosen agent. Not tested for real: an *older* taskbag, since `allomos/1` is the only format (the upgrade is tested with a made-up step). New card from this check: [`A01-27`](./Task-A01-27-new-window.md) (New window…).
