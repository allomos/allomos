# Task board — FlipHopper

> **Last updated:** 2026-10-06

**Card status no longer lives here.** Since [`Task-A01-07`](./tasks/Task-A01-07-own-cards-in-seabag.md) (2026-10-06) FlipHopper tracks itself: every card is a task in this repository's seabag (`.fliphopper/seabag/`), changed only with the `fh` command on `main`. See it with `fh list` (who holds what: `fh list --state in_progress`; what to pick up: `fh next`) or by opening this folder in the FlipHopper app. Rules: [workflow/CARDS.md](./workflow/CARDS.md#where-status-lives). Card detail stays in [docs/tasks/](./tasks/).

This page keeps only **Now** and **Waiting on the owner**, until it is removed or generated from the seabag (owner, 2026-10-06).

## Now

**Iteration 1 — v0.1 macOS local tracker passed its exit check** on 2026-10-07 ([`A01-09`](./tasks/Task-A01-09-v0-1-exit-check.md), [PLAN § 9](./product/PLAN.md#9-roadmap--releases)). The product today:
- a macOS app that opens a seabag and shows its tasks by area, in English, Simplified Chinese and Japanese, with Show in Finder and the hopper icon ([`A01-01`](./tasks/Task-A01-01-walking-skeleton.md), [`A01-02`](./tasks/Task-A01-02-ui-languages.md), [`A01-06`](./tasks/Task-A01-06-show-seabag-in-file-manager.md), [`A07-01`](./tasks/Task-A07-01-app-icon-hopper.md));
- the **`fliphopper` / `fh` command** for agents: claim, label, park, finish and release tasks with a handoff note, one short lock per change ([`A01-03`](./tasks/Task-A01-03-gb-command.md));
- **FlipHopper's own cards live in its seabag** and agents use `fh` ([`A01-07`](./tasks/Task-A01-07-own-cards-in-seabag.md)).

**Decided:** build our own, standalone from Glidelines ([0008](./decisions/0008-build-our-own.md), [0003](./decisions/0003-relationship-to-glidelines.md)); a free local app with paid cloud ([0012](./decisions/0012-free-local-app-paid-cloud.md)); a Tauri 2 desktop hub for macOS and Windows with web and phone companions ([0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md), [0011](./decisions/0011-desktop-framework-and-shared-code.md)); readable Git-tracked files ([0007](./decisions/0007-git-like-project-sync.md)); the name FlipHopper and the seabag ([0013](./decisions/0013-product-name-fliphopper.md)); the main purpose, switching between agents and teammates ([0014](./decisions/0014-main-purpose-and-three-sets.md)). The planning documents are accepted ([`A00-01`](./tasks/Task-A00-01-planning-docs.md)).

**Done 2026-10-07 (owner verified):** File → Open Recent ([`A01-11`](./tasks/Task-A01-11-recent-folders-menu.md)); File → Open Folder… (⌘O) ([`A01-10`](./tasks/Task-A01-10-file-new-window.md)); a README in a folder FlipHopper fills ([`A01-05`](./tasks/Task-A01-05-readme-in-new-folder.md)); opening the folder given at start, even after macOS options ([`A01-04`](./tasks/Task-A01-04-startup-folder-argument.md)).

**Built, waiting on the owner's look:** `fh needs`, `fh ready`, `fh drop` and `fh init --id --labels` ([`A01-08`](./tasks/Task-A01-08-fh-fills-move-gaps.md)); the decks show the app's mark ([`A00-19`](./tasks/Task-A00-19-decks-new-mark.md), republish not done). Research for the owner: companions away from home and the pairing email ([`A04-01`](./tasks/Task-A04-01-remote-access-research.md)); starting and reopening agent sessions ([`A03-01`](./tasks/Task-A03-01-agent-launch-research.md)).

**What is next:** `fh next` names the next ready card; the owner's open questions are below.

## Waiting on the owner

The single list of open questions and approvals. Remove a row when it is answered, and record the answer in the linked document.

| Question or approval | Record | Unblocks |
|---|---|---|
| Roles: separate approver, or always the owner? Sign-in method (magic link / password / GitHub / Google)? Agent tokens per session or per provider? Membership per project or per organization? | [0004](./decisions/0004-team-accounts-and-roles.md) | W03, W06 |
| Agent sessions: which surfaces first (CLI / desktop / cloud)? Runners on whose machines, and may teammates start agents on yours? Create worktrees automatically? Set model and effort per task? | [0005](./decisions/0005-start-and-open-agent-sessions.md) | W07 |
| **Free local app** (0012 accepted): open-source the local app or keep it closed but free? Companions away from home: free via the user's own tunnel account, a paid hosted relay, or both? What else is paid besides cloud copy and teams? Any limits on free use (recommended: none)? | [0012](./decisions/0012-free-local-app-paid-cloud.md) | Iterations 10, 13–15 |
| Local projects: how a local project records who did what, and whether companions may start agents | [0005](./decisions/0005-start-and-open-agent-sessions.md), [0006](./decisions/0006-local-and-cloud-projects.md) | W05, W07 |
| **Check in the app:** choose a new empty folder; it should get `AGENTS.md` and `CLAUDE.md` telling AI agents to use `fh`, and the notice should say so | [`A01-13`](./tasks/Task-A01-13-agent-instructions-on-init.md) | A01-13 Done |
| Review the new `fh` commands (`needs`, `ready`, `drop`, `init --id --labels`; `--commit` / `--push`) | [`A01-08`](./tasks/Task-A01-08-fh-fills-move-gaps.md), [`A01-12`](./tasks/Task-A01-12-fh-commit-option.md) | A01-08, A01-12 Done |
| Republish both road show decks at their existing private addresses with the new mark (local files are done) | [`Task-A00-19`](./tasks/Task-A00-19-decks-new-mark.md) | A00-19's last step |
| Remote access and pairing email: accept or change the recommendation (own Tailscale account; later a paid relay; one FlipHopper email helper on Cloudflare Workers + Resend, which needs a domain) | [`A04-01`](./tasks/Task-A04-01-remote-access-research.md), [0009](./decisions/0009-desktop-hub-with-web-and-mobile-companions.md) | Iteration 10 |
| Agent sessions: read the findings; approve short real trials of Claude Code and Codex if wanted (Codex CLI on this Mac is broken and needs reinstalling) | [`A03-01`](./tasks/Task-A03-01-agent-launch-research.md), [0005](./decisions/0005-start-and-open-agent-sessions.md) | Iteration 8 |
| Cloud hosting target (Vercel or Cloudflare) and domain | [PLAN § 8](./product/PLAN.md#8-architecture-proposed) | Iteration 13 |
| Importer: one-time import of the SSR board, or keep syncing during the transition? | [PLAN § 9](./product/PLAN.md#9-roadmap--releases) | Iteration 7 |

## Roadmap — not yet carded

Iterations 1–15 (v0.1–v1.0) in [PLAN § 9](./product/PLAN.md#9-roadmap--releases), by area in [CARDS.md § Areas](./workflow/CARDS.md#areas). These include:
- the desktop hub (W04);
- companion viewing (W10, R69–R71);
- AI-generated project setup (W09, R50–R55);
- shared commands and the human CLI (R60–R68).

A card is written when its milestone becomes next.

## Blocked

Nothing is blocked beyond the owner questions above.
