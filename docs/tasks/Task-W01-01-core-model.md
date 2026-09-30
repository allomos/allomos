# Task-W01-01-core-model

- **Outcome.** Given a project's workstreams, tasks and their event history, a developer (and later the web app and CLI) can compute per-workstream progress, a burndown series (remaining and total scope over time), and a forecast finish-date range at 50% and 85% confidence — as a pure, tested TypeScript library.
- **Decisive unknowns.**
  - Toolchain: proposed TypeScript, pnpm workspaces, Vitest; Node version to pin (owner reviews with the card).
  - How much of Glidelines' `burndown.ts` and forecast code to port vs. rewrite (worker reads the files named in [REFERENCES.md](../REFERENCES.md); safe under every option of [0003](../decisions/0003-relationship-to-glidelines.md)).
  - Default throughput window (4–6 weeks) and simulation count (worker proposes; owner confirms with the result).
- **Files owned.** `package.json`, `pnpm-workspace.yaml`, `tsconfig*.json`, `packages/core/**`, `docs/workflow/CODING_RULES.md` (created here: the first coding rules), this card.
- **External operations.** None (installing npm packages locally is local work).
- **Done means mechanically.** `pnpm test` passes for `packages/core` with cases for: counts and size-weighted burndown; scope growth; tasks without completion dates (reconstructed band); throughput Monte Carlo producing stable 50%/85% dates with a fixed seed; empty and single-task projects. `pnpm typecheck` passes. A fixture built from the Glidelines SSR snapshot (read-only copy of its structure, not its private text) renders a burndown series in a test.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- Model fields: [PLAN § 5](../product/PLAN.md#5-data-model-first-cut); forecast method: [PLAN § 7](../product/PLAN.md#7-burndown-and-forecast), [decision 0002](../decisions/0002-burndown-counts-plus-sizes.md).
