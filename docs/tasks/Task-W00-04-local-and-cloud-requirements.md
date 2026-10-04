# Task-W00-04-local-and-cloud-requirements

*Split out of `Task-W00-01` on 2026-10-04; the work was done on 2026-10-01.*

- **Outcome.** The owner's requirement is recorded: glidebox works on purely local projects with no sign-in, and offers optional cloud projects that do require sign-in. Earlier documents no longer assume that every project lives in a hosted database.
- **Decisive unknowns.** None for the record itself. The storage technology, packaging and moving projects between modes are left open in [0006](../decisions/0006-local-and-cloud-projects.md).
- **Files owned.** [REQUIREMENTS R40–R43](../product/REQUIREMENTS.md#local-and-cloud-projects), [0006](../decisions/0006-local-and-cloud-projects.md), related edits to PLAN, 0001 and 0004.
- **External operations.** None.
- **Done means mechanically.** R40–R43 recorded as accepted; 0006 written; links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-01 | Record the requirement for purely local projects and optional signed-in cloud projects; update related planning documents locally | owner (chat) |

## Notes

- 2026-10-01 — Added R40–R43 and 0006. Revised the plan and the earlier database and account decisions, so local tracking needs no cloud account or service. Storage technology, packaging and transfer/sync remain undecided. Checks: relative links PASS, `git diff --check` clean; runtime NOT RUN (documentation only).
