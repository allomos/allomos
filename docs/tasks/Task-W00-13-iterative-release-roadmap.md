# Task-W00-13-iterative-release-roadmap

- **Outcome.** The owner can see a roadmap in which glidebox grows from a small working prototype, through a series of releases, each of which is a usable product. Each release says who can use it for what, and how to check that it works.
- **Decisive unknowns.** The owner's confirmation of the release order and of what goes into the first release.
- **Files owned.** [PLAN § 9](../product/PLAN.md#9-roadmap--releases), [CARDS § Workstreams](../workflow/CARDS.md#workstreams), the first release's first card, `Task-W01-01` (re-scoped), the board.
- **External operations.** None.
- **Done means mechanically.** Every release has: a "usable for" sentence, the requirements it covers, an exit check, and its dependencies. All accepted requirements have a release. Links and tables check clean.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-04 | Re-plan milestones in a rapid-iteration pattern: start from a small prototype; every milestone produces a usable product | owner (chat) |
| 2026-10-04 | v0.1 is used for the current project's (glidebox's) own tasks | owner (chat) |

## Notes

- 2026-10-04 — Owner: v0.1 tracks **glidebox's own tasks** (dogfooding accepted). Recorded in PLAN § 9, AI-WORKFLOW and `Task-W04-01`. Added the "what ships" column at the owner's request.

- 2026-10-04 — Re-planned with the `roadmap-update` skill as Now / Next / Later releases: v0.1 local tracker → v0.2 progress and forecast → v0.3 agents first-class → v0.4 on the go → v0.5 new projects in minutes → v0.6 team in the cloud → v1.0 public release.
  - Each release has a "usable for" sentence, its requirements and an exit check.
  - A coverage check found every requirement in a release; R60 applies to all.
  - Opened the first v0.1 card [`Task-W04-01`](./Task-W04-01-walking-skeleton.md) (walking skeleton) and moved `Task-W01-01` to v0.2, so that no release is "only a library".
  - The old milestones were layered: M1 was a library and an importer, and nothing could be edited until M3.
