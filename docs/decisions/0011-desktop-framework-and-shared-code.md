# 0011 — Desktop platforms, shared interface code and desktop framework

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** — framework: Tauri 2 | 2026-10-04 | 2026-10-04 | will.w.hui |

## In plain words

Allomos's screens are written **once**, as a web interface. The desktop app shows that interface in its own window, and also serves it to browsers and phones (the companions from [0009](./0009-desktop-hub-with-web-and-mobile-companions.md)). Only the desktop app contains the parts that need the computer itself: running `git`, starting and opening AI agents, the MCP server for agents, reading and writing the taskbag, and sending the pairing email. The desktop app is built for **macOS and Windows**.

## Owner's statement (2026-10-04)

> Is it possible the desktop app and the browser app share the same source code, except the desktop app has special native code for git and agent operation and MCP implementation and other necessary parts. And I want the desktop app have macOS and Windows versions.

## Decision

- **Platforms:** the desktop app ships for **macOS and Windows**. Linux is not planned for now.
- **One interface code base:** the desktop window, the browser companion and the phone companion (a phone-friendly web page) all use the **same web interface source**. Layout adapts to the screen; there is no separate desktop UI or mobile UI code.
- **Desktop-only native parts:**
  - Git actions through the `git` command ([0010](./0010-project-data-folder.md));
  - starting and opening agent sessions ([0005](./0005-start-and-open-agent-sessions.md));
  - the MCP server and the local CLI connection;
  - access to the taskbag;
  - the local server for companions, with pairing and code email ([0009](./0009-desktop-hub-with-web-and-mobile-companions.md));
  - anything else that needs the operating system.

## Proposed design (to confirm)

- **The interface always talks to the desktop app's local server** (inside the Tauri window too, over the same local connection), also inside the desktop window. So the same interface code runs unchanged in the desktop window, a browser and a phone. The only difference is which actions the server allows for that connection.
- The server tells the interface which **capabilities** a connection has. Desktop-only actions are shown as "send to desktop" (for example an approval, R77), or hidden, on companions.
- **Shared code:** the Rust crates (`crates/core`, `crates/store`, …) hold the project rules and commands; the CLI, MCP server and desktop app reuse them, so a change made from any of them follows the same rules. The TypeScript interface (`apps/web`) is shared by the desktop window, browser and phone.

## Framework — decided: Tauri 2

**The owner chose Tauri 2 on 2026-10-04.** We build on the stable 2.x line and review a move to Tauri 3 once 3.0 is stable. The project rules are written in Rust inside the desktop app; the CLI and MCP server reuse them; the web interface stays TypeScript. The analysis that led here follows.

**Owner input, 2026-10-04:**
- Using a second programming language is **not a problem** (the code is written by agents).
- macOS and Windows ship **together**.
- **Unsigned builds** are used until the first public release.

These change the comparison: Electron's main advantage (one language) no longer weighs much.

Options that let the desktop window reuse the web interface, checked 2026-10-04. **Detailed comparison:** [research/desktop-frameworks.md](../research/desktop-frameworks.md). It covers six frameworks, adding Neutralinojs and NW.js, and finds two things not in this summary: **Electrobun has no Intel Mac build**, and **Tauri 3 is in alpha** (we would build on stable 2.x).

| | Native side | App size | Web engine in the window | Maturity (checked 2026-10-04) |
|---|---|---|---|---|
| **Tauri 2** (recommended) | Rust | Small (a few MB) | System web view: Safari's engine on macOS, Edge WebView2 (Chromium) on Windows | Stable; v2.11.5 released 2026-07-01; active |
| **Electron** | TypeScript (Node.js) | Large (~100 MB+) | Its own bundled Chromium, identical everywhere | Very mature; new major version every 8 weeks, latest three supported; latest v44.5.1 (2026-09-29); governed by the OpenJS Foundation |
| **Electrobun** | TypeScript (Bun) + Zig | Small (~12–14 MB) | System web view | Newest: v1.0 released February 2026; young ecosystem, rough edges reported |
| **Wails** | Go | Small | System web view | v2 stable; v3 still in beta |

**Is Electron still maintained? Is there a replacement?** Electron is very actively maintained. It is not being replaced; many large apps still ship on it. The newer "lighter Electron" options are Tauri (mature, Rust) and Electrobun (TypeScript, but only since Feb 2026).

**Recommendation now: Tauri 2.**
- **Small and light.** The app is a few MB and uses less memory than Electron.
- **Different web engines aren't extra work here.** The companions already run in phone and desktop browsers, including Safari on iPhone. The interface must work in several engines anyway, so the desktop window using the system engine adds no new testing.
- **Its native-side libraries cover what we need:** running `git` and agent programs as child processes, and an **official Rust MCP SDK** (`rmcp`, implementing the MCP 2026-07-28 specification).
- **Stable and actively released.** Electrobun is promising but too young to be the core of the product.

**Effect on the code layout if Tauri is chosen:**
- The **project rules** live in the desktop app's Rust side, as the one authority. That covers the model, validation, events, burndown and forecast, and Git storage.
- The CLI and MCP server use that same Rust code.
- The TypeScript web interface only displays results and sends requests.
- `Task-A02-01` (core model) would then be written in Rust instead of TypeScript.

## Costs to know

- **Code signing:** decided 2026-10-04 — **unsigned builds until the first public release**. Unsigned builds work, but macOS and Windows show security warnings when installing. Before the first public release, signing must be bought: an Apple Developer account (about US$99 per year) and a Windows code-signing certificate, usually paid. That purchase is an external operation needing the owner's approval.

## Open questions (owner)

1. ~~Framework~~ — **decided 2026-10-04: Tauri 2.**
2. ~~Release order~~ — **decided 2026-10-04: macOS and Windows together.**
3. ~~Signing~~ — **decided 2026-10-04: unsigned builds until the first public release.**

## Sources (read 2026-10-04)

- [Electron releases](https://releases.electronjs.org/) and [Electron timelines](https://www.electronjs.org/docs/latest/tutorial/electron-timelines)
- [Tauri 2 release notes](https://v2.tauri.app/release/tauri/)
- [Electrobun guide (Better Stack)](https://betterstack.com/community/guides/scaling-nodejs/electrobun-desktop-apps-typescript/)
- [Wails v3 beta release](https://newreleases.io/project/github/wailsapp/wails/release/v3.0.0-beta.0)
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
