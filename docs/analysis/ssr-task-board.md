# Analysis — the SSR migration task board

The reference case for glidebox: how the owner currently tracks multi-agent work, and what goes wrong. Read on 2026-09-27.

**Source:** `/Users/wanghui/seamaple-work/smp-coding/products/jobs/jobs-dev-mig-to-ssr/next-ssr/docs/migration-plan-ssr/`

## How the board works today

| File | Role | Size (2026-09-27) |
|---|---|---|
| `TASK_BOARD.md` | Session registry, **Now**, waiting-on-owner, mermaid gantt timeline, card index (active / done), blocked | 28 KB, 243 lines |
| `workstreams/Task-W{ws}-{nn}[-{nn}]-{desc}.md` | One file per card: a status table (Status, Label, Approval, Session, Next dependency) and a narrative (Outcome, Decisive unknowns, Files owned, External operations, Done means mechanically), followed by dated progress notes | 46 card files |
| `workstreams/W00…W15-*.md`, `workstreams/readme.md` | Workstream meaning and task lists; index of W0–W15 | — |
| `ESTIMATE.md` §4 | Person-week estimates per workstream (low / likely / high); total ≈ 85–97 person-weeks remaining | 18 KB |
| `AI-WORKFLOW.md` | Rules for Claude / Codex roles, effort levels, handoff timing, when to start a new session | 10 KB |
| `AGENT_HANDOFF.md` | Resume context per active piece of work (checkout, uncommitted changes, running processes, next action) | 185 KB |

**Conventions.**

- **IDs:** `Task-W{2-digit workstream}-{2-digit task}-{description}`; children add another two-digit number (`Task-W07-01-09-seeker-resume-list`).
- **Label ladder**, the only progress claim: *implemented → locally verified → connected → deployed and verified*, followed by owner verification.
- **Approvals:** approving a card authorizes local work only; each external operation (hosted SQL, secrets, deploys, email, production data) needs its own exact approval, recorded on the card.
- **Sessions:** each Claude or Codex session works in its own git worktree/branch. A session claims a card by writing its branch name into the card file and the registry and committing that change on its own. Merge conflicts on that line decide races.

**Snapshot, 2026-09-27:** 18 open cards, 30 done, 3 claimed (two Codex, one Claude). Of the open cards, most have a next dependency that is the owner: a visual check, an exact hosted-operation approval, or a card review.

## Problems

1. **Status is duplicated.** One card's state appears in its own file, the card index, the Now narrative, the Timeline gantt, "Recently finished" and the workstream's task table. The board states that when documents disagree the board wins and stale lines must be corrected in place — drift is expected, ongoing work.
2. **Status is prose.** Labels such as "locally verified; owner verified, Done (test Worker not redeployed)" cannot be counted, filtered or charted. The underlying ladder is structured, but it is embedded in sentences.
3. **The owner is the bottleneck, without a queue.** The things waiting on the owner are scattered across "Next dependency" cells and the Now text, instead of one inbox.
4. **Claiming is manual.** The claiming protocol has six rules and relies on separate commits and git conflicts.
5. **No forecast.** Estimates live per workstream in person-weeks and are not tied to cards. Not-started roadmap tasks live only in workstream files, so the board does not know the total scope. The gantt is selective and drawn by hand. Nothing answers "when will we finish?"
6. **Resuming is expensive.** Each session reads the board, the handoff (185 KB) and card files before it starts.
7. **No collaboration.** Everything is Markdown inside one repo; teammates cannot see or act on it without the repo and its conventions.

## What glidebox should keep

- The **card discipline**: one observable outcome per card, "done means mechanically", files owned, external operations listed.
- The **label ladder** and **exact approvals** — as configurable, structured fields.
- **Workstreams** as the grouping, with hierarchical task IDs.
- **Narrative stays in the repo**; glidebox links to card files and evidence rather than absorbing them.

## Prior attempt

Glidelines (`glidepace-vibe-cc`) converted this board once: `docs/task-files/convert-ssr-plan.py` produced `examples/ssr-migration.tasklist.json` (44 tasks, 16 workstreams, as of 2026-09-08). Its notes record the difficulties: stale lines had to be reconciled by hand, statuses needed normalization, completion dates were missing for many tasks, estimates were not converted, and the converter aborts when the board's wording changes. That experience motivated [decision 0001](../decisions/0001-database-source-of-truth.md).
