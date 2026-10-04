# 0011 — Desktop platforms, shared interface code and desktop framework

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirements accepted; framework proposed** | 2026-10-04 | 2026-10-04 (requirements) | will.w.hui |

## In plain words

Glidebox's screens are written **once**, as a web interface. The desktop app shows that interface in its own window, and also serves it to browsers and phones (the companions from [0009](./0009-desktop-hub-with-web-and-mobile-companions.md)). Only the desktop app contains the parts that need the computer itself: running `git`, starting and opening AI agents, the MCP server for agents, reading and writing the data folder, and sending the pairing email. The desktop app is built for **macOS and Windows**.

## Owner's statement (2026-10-04)

> Is it possible the desktop app and the browser app share the same source code, except the desktop app has special native code for git and agent operation and MCP implementation and other necessary parts. And I want the desktop app have macOS and Windows versions.

## Decision

- **Platforms:** the desktop app ships for **macOS and Windows**. Linux is not planned for now.
- **One interface code base:** the desktop window, the browser companion and the phone companion (a phone-friendly web page) all use the **same web interface source**. Layout adapts to the screen; there is no separate desktop UI or mobile UI code.
- **Desktop-only native parts:**
  - Git actions through the `git` command ([0010](./0010-project-data-folder.md));
  - starting and opening agent sessions ([0005](./0005-start-and-open-agent-sessions.md));
  - the MCP server and the local CLI connection;
  - access to the data folder;
  - the local server for companions, with pairing and code email ([0009](./0009-desktop-hub-with-web-and-mobile-companions.md));
  - anything else that needs the operating system.

## Proposed design (to confirm)

- **The interface always talks to the desktop app's local server**, also inside the desktop window. So the same interface code runs unchanged in the desktop window, a browser and a phone. The only difference is which actions the server allows for that connection.
- The server tells the interface which **capabilities** a connection has. Desktop-only actions are shown as "send to desktop" (for example an approval, R77), or hidden, on companions.
- **Shared TypeScript packages:** `packages/core` (model, burndown, forecast), the shared commands, and the interface. The CLI, MCP server and desktop app reuse the same commands, so a change made from any of them follows the same rules.

## Framework — owner to choose

| | **Electron** (recommended) | **Tauri** |
|---|---|---|
| Native side written in | TypeScript, the **same language** as everything else | Rust (a second language), or a bundled Node helper program |
| Running `git`, agents, MCP | Built into Node.js; official TypeScript MCP SDK | Possible, but the logic is split across Rust and TypeScript, or needs the helper |
| App size | Large (~100 MB+): ships its own browser engine | Small (a few MB): uses the system's web view |
| Same look on macOS and Windows | Yes: the same engine everywhere | Mostly: macOS and Windows web views differ slightly |
| macOS + Windows | Yes | Yes |

**Recommendation: Electron.** One language from interface to native code means the CLI, MCP server and desktop app share code directly, and agents work in one toolchain. The cost is a larger download and more memory.

## Costs to know

- **Code signing:** for users to install without security warnings, apps must be signed. macOS needs an Apple Developer account (about US$99 per year) for signing and Apple's notarization check. Windows needs a code-signing certificate, which is usually paid. Unsigned development builds work but show warnings. Buying either is an external operation that needs the owner's approval.

## Open questions (owner)

1. **Framework:** Electron (recommended) or Tauri?
2. **Release order:** macOS and Windows together, or one first?
3. **Signing:** buy signing for public releases now, or use unsigned builds until the first release?
