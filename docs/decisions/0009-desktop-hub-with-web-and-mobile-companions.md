# 0009 — The desktop app is the hub; web and mobile apps connect to it

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; design open** | 2026-10-04 | 2026-10-04 (requirement) | will.w.hui |

## In plain words

The **desktop application is the complete glidebox**. It holds the projects, runs the full set of features, and is where agents are started. A **web app** and a **mobile app** are helpers: they connect to the desktop app while it is running, and let you look at what is going on — above all the **progress and status** of the work — from a browser or a phone.

## Owner's statement (2026-10-04)

The full product should be a desktop application. It provides a web program and a mobile program as helpers. These connect to the running desktop application, so you can view it from them. Viewing includes the progress status.

## Decision

- **The desktop app has the full feature set:**
  - local projects and their storage
  - lists, charts, burndown and forecast
  - importing projects
  - starting and opening agent sessions
  - the local CLI and MCP access for agents
- **Web and mobile are companion clients.** They connect to a **running** desktop app and show its projects. Viewing at least covers **progress and status**: task states, workstream progress, charts and burndown, and what is waiting on whom.
- The desktop app works on its own; companions are optional.
- Each local project's data lives in a folder the user chooses ([0010](./0010-project-data-folder.md)).

## How this fits earlier decisions

- [0006](./0006-local-and-cloud-projects.md): local projects stay on the owner's machine. Companions read them **from the desktop app**, so nothing is uploaded to a cloud service for that. This also answers the problem raised in the interface research: a phone cannot reach a desktop-local project unless something connects them.
- [0005](./0005-start-and-open-agent-sessions.md): the desktop app is the natural place for the "runner" that starts and resumes agents on that machine.
- [INTERFACES.md](../product/INTERFACES.md) and the [interface research](../research/interface-patterns.md): this picks the packaging option "web UI + local companion + desktop shell". It also sets the release direction: desktop first, web and mobile as companions.

## Open questions (owner)

1. **View only, or also act?** May companions also change things (for example mark a check done, approve, add a task), or only view? The owner named viewing, including progress, as the core.
2. **Where companions connect from:** the same home or office network only, or also away from home? Away-from-home access needs a relay or tunnel, which is a hosted service and an external operation.
3. **Pairing and security:** how a phone or browser is allowed in (for example by scanning a QR code shown on the desktop), and how access is removed again.
4. **When the desktop app is off:** should companions show nothing, or the last data they saw (read-only)?
5. **Cloud projects (R42):** are they still needed, or does desktop-to-companion connection replace them for now? Today they remain an accepted requirement, so this record does not drop them.
6. **Form of the mobile app:** an installed native app, or a phone-friendly web page served by the desktop app?
7. **Which desktop systems first:** macOS only, or also Windows and Linux?

## Consequences

- The architecture gains a **desktop app** that contains the local store, the shared commands and a **local server** for companions, plus the CLI, MCP and runner.
- Security of the local server becomes a core design item: pairing, encryption on the network, and limiting what each companion may do.
- Milestones need a desktop-shell step and a companion-viewer step (see [PLAN § 9](../product/PLAN.md#9-milestones)).
