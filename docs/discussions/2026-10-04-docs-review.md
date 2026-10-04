# 2026-10-04 — Documentation review and new decisions

Participants: will.w.hui (owner), Claude (Claude Code session).

## Request

The owner asked Claude to read and review everything another agent had written between 2026-10-01 and 10-03, and report problems. The owner then asked for the problems to be fixed, and made two decisions.

## Review findings (summary)

1. The planning card `Task-W00-01` had become a catch-all: claimed 10 times for 6 different outcomes, owning "everything under `docs/`", with a 6.5 KB work log.
2. The competitor research recommended evaluating an open-source base (Backlog.md), but nothing put that choice before the owner. The possible plan to sell glidebox was recorded only in research and handoff notes.
3. Contradictions:
   - "atomic claims, no merge conflicts" (R12, plan) against tentative offline claims (0007, interfaces);
   - agent-start permissions by role (0005) against local projects with no accounts (0006);
   - the plan's architecture had no local storage;
   - R50–R51 and R60–R62 had no milestone or workstream.
4. A second restart document (the 2026-10-03 "handoff" discussion) duplicated the handoff file and board. "Do not assume" in the handoff had become a notes log.
5. The board's Now section was unchanged since 09-30. The worktree `glidebox-local-cloud` was left unused.
6. The owner requests of 10-01 to 10-03 had no discussion record.
7. Smaller points:
   - one wrong source link (Obsidian Git pointed to an unrelated copy);
   - glossary missing the new terms;
   - dense jargon in the new contracts;
   - commits that don't say which agent made them;
   - requirement sections out of order.

All 50 external links were checked; the other 49 work. Internal links, anchors, tables and whitespace were clean.

## Owner decisions

| Decision | Record |
|---|---|
| **Build our own**: do not start from an open-source tracker; read others only as references | [0008](../decisions/0008-build-our-own.md) |
| **Desktop app as the hub**: the complete product is a desktop application; web and mobile programs are companions that connect to the running desktop app and can view it, including progress and status | [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md), R69–R71 |

## Changes made

- **Cards:** split the planning card into its own cards (`W00-04` to `W00-09`) and opened `W00-10` for this review.
- **Contradictions resolved:**
  - R12 revised;
  - 0005 gained a local-projects section;
  - the plan gained the desktop hub, local store, companions and milestones M2/M2b/M4;
  - workstreams W03/W04 renamed, W09/W10 added.
- **Plain summaries** added at the top of INTERFACES, 0007 and AI-GENERATED-PROJECTS.
- **Records:** the glossary extended; the source link fixed; the 10-03 handoff record reduced; the 10-01 to 10-03 requests reconstructed.
- **Rules:** AGENTS.md gained "one card per outcome" and "say who made each commit".
- **Board and handoff** rewritten to current state.

**Not done:** removing the unused worktree `glidebox-local-cloud` was blocked by the permission system. It is listed for the owner in the handoff.

## Later the same day: project data folder

The owner added: in the desktop application, the data source folder should be able to be set to **any folder**, including **a standalone folder** and **a subfolder of an existing project folder**. Recorded as R72–R73 (accepted) and R74–R75 (proposed) in [0010](../decisions/0010-project-data-folder.md), card [`W00-11`](../tasks/Task-W00-11-project-data-folder.md). The owner's sentence ended with a comma, so the record asks whether more kinds of location were meant.

The owner then added that **commit and push are performed by calling `git` commands**. This is recorded as R76 (accepted): glidebox runs the user's installed `git`, so the user's own settings, credentials and hooks apply.

## Mobile companion form

Asked whether the mobile companion should be an installed app or a phone-friendly web page, the owner chose an **installed native app**. Recorded in [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) and R70. **Later reversed by the owner:** the mobile companion is a phone-friendly web page (see the table below).

## Owner answers to the open questions

| Question | Answer | Recorded in |
|---|---|---|
| May companions act, not only view? | Yes, for example mark a check done or approve — **by notifying/controlling the desktop app** | R77, 0009 |
| Same network only, or from outside? | **From outside**; needs a **free or very cheap** solution | R78, 0009, card [`W10-01`](../tasks/Task-W10-01-remote-access-research.md) |
| How is a phone or browser allowed in? | The companion gives a valid email and an alias; the desktop user approves the email; the desktop app sends a long verification code to it; the code gives access | R79, 0009 |
| Mobile: installed app or phone-friendly web page? | First "installed native app", then "use this" for the web page; asked again, the owner **confirmed the phone-friendly web page** | 0009, R70 |
| Are cloud projects still needed? | Yes | 0009 |
| Folder self-contained; no nested repository; no commit/push unless asked; one writer at a time | Yes to all four | R74–R75, 0010 |
| Other kinds of location? | No | 0010 |
| Several projects in one repository? | Not in scope | 0010, PLAN non-goals |
| Ask the user to install `git` if it is missing? | Yes | R76, 0010 |
| Pull, status and diff also via the `git` command? | Yes | R76, 0010 |
| Database file or text files in the folder? | After a plain-words explanation: **approved text files tracked by Git** (option B) | [0007](../decisions/0007-git-like-project-sync.md) accepted, 0001 superseded, R10 |

## Repository settings

The owner created `mypals-ml/glidebox` on GitHub and pushed. The owner briefly asked for the local Git user name `willwhui`, then chose to keep `will.w.hui`. The owner decided that **agents push after every commit** (AGENTS.md).

## Desktop platforms and shared code

The owner asked whether the desktop app and the browser app can share the same source code, with only the desktop adding native code for Git, agent operations, MCP and other necessary parts, and asked for macOS and Windows versions. Answer: yes. The interface is written once, and only the desktop adds native parts. Recorded as R81–R82 in [0011](../decisions/0011-desktop-framework-and-shared-code.md). Electron is recommended over Tauri for one language throughout; the owner is to choose. Code-signing costs were noted.

## Companions when the desktop is off

The owner decided: tell the user to run the desktop app on a computer, and refresh to reconnect automatically. Recorded as R83 in [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md).

## Pairing code email

The owner decided the code is sent by email through a free email-sending service. Recorded as R84 in [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md); the service choice was added to research card [`W10-01`](../tasks/Task-W10-01-remote-access-research.md).

## Framework follow-up

The owner asked whether there are more framework options, and whether Electron is still maintained or replaced. The owner also said a second language is not a problem for agent-written code, chose to release macOS and Windows together, and chose unsigned builds until the first public release (R85). Four options were compared from current sources in [0011](../decisions/0011-desktop-framework-and-shared-code.md); the recommendation is now **Tauri 2**.

## Framework chosen

After the detailed comparison ([research](../research/desktop-frameworks.md)) and an explanation of code signing versus update signing, the owner chose **Tauri 2**. Recorded as R86 in [0011](../decisions/0011-desktop-framework-and-shared-code.md). The first code card `Task-W01-01` is now a Rust card.

## Iterative release roadmap

The owner asked whether the milestones followed rapid iteration, and said every milestone must produce a usable product, growing from a small prototype. They did not: the old milestones were layered (M1 a library, M2 view-only, editing only from M3).

Re-planned in [PLAN § 9](../product/PLAN.md#9-roadmap--releases), using the roadmap-update skill, as seven releases, each an installable, usable desktop app. The first code card is now the v0.1 walking skeleton [`Task-W04-01`](../tasks/Task-W04-01-walking-skeleton.md). This replaces the "proposed plan change" below.

## Proposed plan change to confirm (superseded by the release roadmap)

Cloud projects move from M2 to M4, so that M2 is the desktop hub and M2b is companion viewing. This follows from 0009, but the owner has not confirmed the new order.
