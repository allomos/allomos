# 0004 — Team accounts and roles

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted in part** — roles and email invitations accepted; role details still proposed | 2026-09-27 | 2026-09-27 | will.w.hui |

## Context

The owner wants teammates to collaborate online in cloud projects. [0006](./0006-local-and-cloud-projects.md) also requires local projects without sign-in; the account and email invitation flow here applies to cloud collaboration, not a prerequisite for local use. In the reference project the owner also plays a distinct role: checking results and granting exact approvals for external operations. Agent sessions (Claude, Codex) act on tasks too, but should not be able to approve their own operations.

## Decision (accepted 2026-09-27)

- Glidebox **has roles**. Every project member holds one role, and what they can do depends on it.
- **An owner or admin adds a new member by email address.** No public sign-up: a person can join a project only through such an invitation.

## Proposal — details not yet confirmed

**Roles, per project:**

| Role | Can |
|---|---|
| **owner** | everything, including deleting the project and transferring ownership; at least one per project |
| **admin** | manage members (invite by email, change roles, remove), project settings, plus everything a contributor and approver can do |
| **approver** | record owner checks and exact approvals, plus everything a contributor can do |
| **contributor** | create and update tasks, claim and release work, request approvals |
| **viewer** | read only |
| **agent** | a token-based identity for an agent session or provider, scoped to one project; can claim, update and request approvals; **cannot approve** and cannot manage members |

**Invitation flow:**

1. An owner or admin enters an email address and a role.
2. Glidebox sends an invitation email with a single-use link that expires (e.g. after 7 days).
3. The invitee signs in or creates an account with **that same email address**, and joins with the given role.
4. Pending invitations are listed; an owner or admin can resend or cancel them.
5. Adding someone who already has an account adds the project to their account directly, with a notification email.

**Rules:**

- An admin cannot grant or remove the owner role, and cannot remove an owner.
- The last owner cannot leave or be demoted.
- Every change of membership or role is recorded as an event (who, when, what).
- Agent tokens are created by owners and admins, shown once, and can be revoked.

## Open points

- Are approver and owner the same person in practice, or is a separate approver role needed?
- Sign-in method: email magic link, email + password, or also GitHub / Google?
- Should an agent token be per session (fine-grained history) or per provider (simpler setup)?
- Is organization-level membership needed (one team across many projects), or only per project?
