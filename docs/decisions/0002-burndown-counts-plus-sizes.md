# 0002 — Burndown uses task counts, plus optional sizes

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-09-27 | 2026-09-27 | will.w.hui |

## Context

The owner needs a burndown chart that estimates the project's finish date. In the reference project, estimates exist only per workstream in person-weeks (`ESTIMATE.md` §4) and are not attached to cards, so a card-level estimate cannot be assumed.

## Decision

- Every task counts as **1**. The default burndown shows **remaining tasks** and **total scope** over time.
- A task may carry an **optional size** (points or days). When sizes are present, the chart can switch to a **size-weighted** view.
- The finish date is forecast from **recent throughput** (tasks finished per week, Monte Carlo simulation) and shown as a **range** (50% / 85% confidence). An estimate-based forecast (remaining size ÷ capacity) is a secondary view when sizes exist.

## Consequences

- Works from day one with zero estimation effort; accuracy improves as history accumulates.
- Scope growth (new cards) is visible as a rising total line, not hidden.
- Tasks of very different sizes skew count-based forecasts; the size field and splitting large cards are the remedies.
- Workstream-level person-week estimates may later be spread over cards as a starting size (not decided).

## Alternatives considered

- **Sizes required on every task.** Rejected: adds effort agents and people will not keep up.
- **Only workstream person-weeks.** Rejected: not tied to card completion, so progress cannot be measured against it.
