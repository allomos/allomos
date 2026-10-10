# 0012 — The local app is free; payment is only for the cloud

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (direction); which features are paid and the source licence are open | 2026-10-04 | 2026-10-04 | will.w.hui |

## In plain words

Developers and team leaders get a real choice. They can **use the Allomos desktop app on their own computer for free**, with no account and no payment. People who want what only a hosted service can give, such as a cloud copy or a team working together online, can **pay for that**. Free use is not a trial or a crippled version: it is the owner's own wish to offer it.

## Owner's statement (2026-10-04)

> I want to offer the developers and the team leaders a new choice: they can use a local app free, if they don't want to pay. Which is also my own wish.

## Decision

- **The local desktop app is free of charge**, for individuals and for team leaders, on macOS and Windows. It covers local projects, agents and the command line.
- **Paying is optional** and is for **cloud features**: cloud copies, team sign-in, invitations, roles and live collaboration (iterations 13–14).
- **No account and no payment details** are needed to download and use the local app (this matches R41).

## Why this fits

- The architecture is already local-first ([0006](./0006-local-and-cloud-projects.md), [0009](./0009-desktop-hub-with-web-and-mobile-companions.md), [0010](./0010-project-data-folder.md)). Iterations 1–12 need no Allomos server.
- It separates Allomos from cloud-first trackers (Linear, Jira, Vibe Kanban cloud): your task data stays on your machine and in your own Git repository.
- Free local tools for agents already exist, for example Backlog.md (open source, free). "Free and local" is therefore **not unique on its own**. Allomos's difference has to come from:
  - the desktop experience;
  - verified progress and owner decisions;
  - the burndown and forecast;
  - starting and opening agent sessions.

## Consequences

- **Free users must cost (almost) nothing to serve.** Any feature in the free app that uses a hosted service needs one of three solutions:
  - use the user's own free account (for example their own tunnel or email account);
  - stay within a provider's free tier with a safe limit;
  - or become a paid feature.

  This affects:
  - **Iteration 9** (pairing email): the sending service's free tier has a monthly limit.
  - **Iteration 10** (companions away from home): a relay or tunnel carries traffic. If Allomos runs it, every free user costs money.
  - **Auto-update downloads**: free when hosted on GitHub Releases.
- **Fixed costs remain even with no revenue:**
  - code signing (Apple about US$99 per year, plus a Windows certificate) from Iteration 15;
  - the domain and download page;
  - support time.
- **Free does not decide open source.** The local app can be free but closed-source, or free and open-source. The choice affects trust, contributions and whether competitors can copy it. Still open.
- **Paid features must be worth paying for**, and cannot be features free users expect. Today's candidates are cloud copy and team work; others (for example hosted remote access for companions) are open.

## Open questions (owner)

1. **Source licence:** keep the local app closed-source but free, or open-source it (for example MIT or Apache-2.0)?
2. **Companions away from home** (Iteration 10): free using the user's own tunnel account, or a paid hosted relay, or both?
3. **What else is paid,** besides cloud copy and teams (iterations 13–14)?
4. **Any limits on free use** (for example the number of projects)? The recommendation is **none**, in line with the owner's wish.
