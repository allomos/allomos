# 0004 — Team accounts and roles

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Proposed** | 2026-09-27 | — | will.w.hui |

## Context

The owner wants teammates to collaborate online. In the reference project the owner also plays a distinct role: checking results and granting exact approvals for external operations. Agent sessions (Claude, Codex) act on tasks too, but should not be able to approve their own operations.

## Proposal

- **Invite-only** accounts; no public sign-up.
- Per-project roles:
  - **owner** — everything, including project settings and membership
  - **approver** — records checks and approvals
  - **contributor** — creates and updates tasks, claims work
  - **viewer** — read-only
  - **agent** — token-based identity for one agent session or provider, scoped to one project; can claim, update, request approvals; cannot approve
- Every change records the actor, so history shows who (or which session) did what.

## Open points

- Are approver and owner the same person in practice?
- Should an agent token be per session (fine-grained history) or per provider (simpler setup)?
- Is organization-level membership needed, or only per project?
