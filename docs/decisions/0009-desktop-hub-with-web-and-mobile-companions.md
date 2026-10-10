# 0009 — The desktop app is the hub; web and mobile apps connect to it

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; design open** | 2026-10-04 | 2026-10-04 (requirement) | will.w.hui |

## In plain words

The **desktop application is the complete Allomos**. It holds the projects, runs the full set of features, and is where agents are started. A **web app** and a **mobile app** are helpers: they connect to the desktop app while it is running, and let you look at what is going on — above all the **progress and status** of the work — from a browser or a phone.

## Owner's statement (2026-10-04)

The full product should be a desktop application. It provides a web program and a mobile program as helpers. These connect to the running desktop application, so you can view it from them. Viewing includes the progress status.

## Decision

- **The desktop app has the full feature set:**
  - local projects and their storage
  - lists, charts, burndown and forecast
  - importing projects
  - starting and opening agent sessions
  - the local CLI and MCP access for agents
- **Web and mobile are companion clients.** They connect to a **running** desktop app and show its projects. Viewing at least covers **progress and status**: task states, area progress, charts and burndown, and what is waiting on whom.
- **The mobile companion is a phone-friendly web page** (owner, confirmed 2026-10-04). It is served by the desktop app and opened in the phone's browser; there is no app-store app for now. (Earlier the same day the owner had picked an installed native app; the owner then confirmed the web page.)
- **Companions can also act, through the desktop app** (owner, 2026-10-04). For example, they can mark a check done or approve something. A companion never changes project data itself: it sends the request to the desktop app, which checks it, applies it and records who did it. The desktop app stays the only authority.
- **Companions work from outside the home or office network** (owner, 2026-10-04). The connection method must be **free or very cheap**. Any relay or tunnel service is an external operation and needs the owner's approval before it is set up.
- **Pairing by email and code** (owner, 2026-10-04). A new phone or browser gets access like this:
  1. The companion user enters a **valid email address** and an **alias** (a display name).
  2. The desktop app user sees the request and **approves the email address**.
  3. The desktop app sends a **long verification code** to that email address.
  4. The companion user enters the code and gets access.
- **Cloud projects are still needed** (owner, 2026-10-04). Companions do not replace R42.
- **When the desktop app is off** (owner, 2026-10-04), a companion shows a message telling the user to start the Allomos desktop app on their computer. Refreshing the page reconnects automatically once the desktop app is running again. Companions do not show old, possibly stale data in the meantime.
  - *Proposed addition:* the page also retries quietly in the background, so it often reconnects without a manual refresh.
- **The pairing code is sent by email through a free email-sending service** (owner, 2026-10-04). Which service is chosen by the research in [`Task-A04-01`](../tasks/Task-A04-01-remote-access-research.md). Creating the service account and its access key is an external operation needing the owner's approval. The key is a secret: never committed, and never shipped inside the app where anyone could extract and misuse it.
  - *Open design point:* where the key lives — in a small allomos-run helper that sends on the desktop app's behalf, or a key each user sets up in their own desktop app. The research card compares them.
- The desktop app works on its own; companions are optional.
- Each local project's data lives in a folder the user chooses ([0010](./0010-project-data-folder.md)).

## How this fits earlier decisions

- [0006](./0006-local-and-cloud-projects.md): local projects stay on the owner's machine. Companions read them **from the desktop app**, so nothing is uploaded to a cloud service for that. This also answers the problem raised in the interface research: a phone cannot reach a desktop-local project unless something connects them.
- [0005](./0005-start-and-open-agent-sessions.md): the desktop app is the natural place for the "runner" that starts and resumes agents on that machine.
- [INTERFACES.md](../product/INTERFACES.md) and the [interface research](../research/interface-patterns.md): this picks the packaging option "web UI + local companion + desktop shell". It also sets the release direction: desktop first, web and mobile as companions.

## Open questions (owner)

Answered 2026-10-04: companions can act (through the desktop app); access from outside, free or very cheap; pairing by email and code; cloud projects still needed.

1. ~~Mobile form~~ — **decided 2026-10-04: a phone-friendly web page.**
2. ~~When the desktop app is off~~ — **decided 2026-10-04:** tell the user to start the desktop app; refreshing reconnects automatically.
3. ~~Which desktop systems~~ — **decided 2026-10-04: macOS and Windows** ([0011](./0011-desktop-framework-and-shared-code.md)).
4. **How to reach the desktop from outside cheaply:** to be researched. Candidates have free tiers, for example a tunnel service, a private network service, or a small relay. Each is an external service needing approval. *Researched 2026-10-07* ([remote-access.md](../research/remote-access.md)); recommendation, not a decision: the user's own free Tailscale account (private network first, Funnel as fallback), and later a paid Allomos relay.
5. ~~How the desktop app sends the code email~~ — **decided 2026-10-04: through a free email-sending service.** Still open: which service, and where its key lives (see the decision above). *Researched 2026-10-07* ([remote-access.md](../research/remote-access.md)); recommendation: one Allomos-run helper on Cloudflare Workers calling Resend, its key only a secret of the helper; it needs an Allomos domain.

### Proposed pairing details (to confirm)

- The code is long and random, works **once**, and expires after a short time (for example 15 minutes).
- Each paired device is listed in the desktop app with its alias and email. The desktop user can **remove** a device at any time, which ends its access immediately.
- The desktop app records every pairing, removal and companion action as events.
- Traffic between companion and desktop is encrypted.

## Consequences

- The architecture gains a **desktop app** that contains the local store, the shared commands and a **local server** for companions, plus the CLI, MCP and runner.
- Security of the local server becomes a core design item: pairing, encryption on the network, and limiting what each companion may do.
- Milestones need a desktop-shell step and a companion-viewer step (Iterations 9–10 in [PLAN § 9](../product/PLAN.md#9-roadmap--releases)).
