# Desktop framework comparison

**Evidence collected 2026-10-04** for [decision 0011](../decisions/0011-desktop-framework-and-shared-code.md), at the owner's request for a more detailed comparison. Public sources only: official documentation, release pages, and GitHub figures read through the GitHub API on 2026-10-04. Nothing was installed, built or measured. App sizes are the projects' own claims, not tests.

## In plain words

We need a framework that turns our web interface into a **desktop app for macOS and Windows**, and gives us a "native side" that can run `git`, start AI agents, act as an MCP server, read the data folder, and run a small server for the phone and browser companions.

Six frameworks can do this. They differ mainly in:
- **app size** — whether they ship their own copy of Chrome;
- **the native-side language**;
- **how mature they are.**

**Tauri 2** remains the recommendation: small, stable, frequently released, and it covers every glidebox need. **Electron** is the safe, heavyweight alternative. The others are either too young (Electrobun), still in beta for their next version (Wails), or weaker fits (Neutralinojs, NW.js).

## What glidebox needs from the framework

| Need | Source |
|---|---|
| macOS **and** Windows, released together | R81, R85 |
| Desktop window shows the **same web interface** as browser/phone companions | R82 |
| Run the **`git` command** and **agent programs** (Claude Code, Codex) as child processes | R76, R30–R31 |
| Host an **MCP server** for agents | R11 |
| Run a **local server** that companions connect to, with pairing | R70–R79 |
| Read/write a **user-chosen data folder** | R72–R75 |
| **Free/cheap** to build and update; **unsigned builds** until the first public release | R78, R85 |
| Long-term maintenance by agents | AGENTS.md |

## Overview table

| | **Tauri 2** | **Electron** | **Electrobun 2** | **Wails 2** | **Neutralinojs** | **NW.js** |
|---|---|---|---|---|---|---|
| **Native-side language** | Rust (sidecars in any language) | TypeScript / JavaScript (Node.js) | TypeScript (own runtime "Cottontail", Bun-compatible); also Zig, Rust, Go | Go | JavaScript API over WebSocket; extensions in any language | JavaScript (Node.js) |
| **Web engine in the window** | System: WKWebView (Safari engine) on macOS, WebView2 (Chromium) on Windows | Its own bundled Chromium, identical everywhere | System web view, or optional bundled Chromium (CEF) | System: WKWebView / WebView2 | System web view | Its own bundled Chromium |
| **App size (project claim)** | Minimal app under 600 KB; real apps a few MB | Typically ~100 MB+ | ~12–14 MB base | Small (a few MB) | Very small | Large, like Electron |
| **macOS** | Yes (Apple Silicon and Intel) | Yes (Apple Silicon and Intel) | **Apple Silicon (arm64) only** — no Intel Mac build | Yes | Yes (x64 and arm64) | Yes |
| **Windows** | Yes | Yes | x64 (stable); ARM via emulation; Windows 11+ officially | Yes | Yes | Yes |
| **Latest stable (2026-10-04)** | 2.12.1 (2026-09-30) | 44.5.1 (2026-09-29) | 2.0.2 (2026-09-29) | 2.14.0 (2026-08-10) | 6.9.0 (2026-07-24) | (not checked in detail) |
| **Next version** | 3.0 in alpha (alpha.4, 2026-10-01) | 45 in alpha; new major every 8 weeks | 2.0.3 in beta | 3.0 in beta (beta.27, 2026-10-01) | nightly builds | — |
| **Release rhythm** | Frequent minor/patch releases | Major every 8 weeks; latest 3 majors supported | Very frequent; v1 (Feb 2026) → v2 (Aug 2026) with breaking changes | Regular minors; v3 in beta for a long time | Every 1–2 months | — |
| **GitHub stars (2026-10-04)** | ~112k | ~123k | ~13k | ~36k | ~9k | ~41k |
| **Licence** | Apache-2.0 / MIT | MIT | MIT | MIT | not auto-detected by GitHub (check before use) | MIT |
| **Backing** | Tauri Programme (open-source foundation-style governance) | OpenJS Foundation | Blackboard (small company) | Community / maintainer-led | Community | Community (originally Intel) |
| **Run `git` / agent programs** | Yes: from Rust, or the shell plugin with permission scopes | Yes: Node `child_process` | Yes | Yes: Go `os/exec` | Yes: `os.execCommand`, limited API | Yes: Node |
| **MCP server SDK** | Official Rust SDK (`rmcp`, MCP spec 2026-07-28) | Official TypeScript SDK | TypeScript SDK (via Node/Bun compatibility — untested) | Official Go SDK exists (not checked in detail) | Via an extension in another language | TypeScript SDK |
| **Local HTTP/WebSocket server for companions** | Yes (Rust server libraries) | Yes (Node) | Yes | Yes (Go standard library) | Already runs one internally; customising is limited | Yes (Node) |
| **Auto-update** | Official updater plugin; update files signed with **our own key** (always required, free); hosts on GitHub Releases or any static host | Built-in updater; free service `update.electronjs.org` needs a public GitHub repo **and a code-signed macOS app** | Built-in updater (details not checked) | No official updater in v2 (community options) | Built-in updater | Manual |
| **Security model** | Capabilities/permissions: the interface can only call what is explicitly allowed; security audits for major/minor releases | Powerful but easy to misconfigure; must follow Electron's security checklist (context isolation, no Node in the renderer) | Not established by this research | Standard | Native API allow-list | Node available in pages by default (riskier) |
| **Mobile apps** | iOS/Android supported (not needed: companions are web pages) | No | No | Experimental in v3 | No | No |

## Each option in more detail

### Tauri 2 — recommended

- **For:**
  - Small app and low memory, because it uses the system's web engine.
  - Stable 2.x line with releases every few weeks: 2.11.6, 2.12.0 and 2.12.1 all came in September 2026.
  - A permission system that blocks risky actions unless allowed. This fits glidebox, whose companions can send actions.
  - An official Rust MCP SDK.
  - An official updater that works with free hosting (GitHub Releases). It signs updates with our own key, so it is usable before we buy OS code signing.
- **Against:**
  - The native side is Rust. The owner has said that is fine.
  - Rendering differs slightly between Safari's engine (macOS) and Chromium (Windows). Our interface must work in Safari anyway, for iPhone companions.
  - **Tauri 3 is in alpha.** We would build on 2.x and plan a migration once 3.0 is stable; the size of that migration is unknown.
- **Fit for glidebox:** covers every need in the table above.

### Electron — safe alternative

- **For:**
  - The most mature and widely used option, with an 8-week release rhythm and three supported major versions, governed by the OpenJS Foundation.
  - Identical rendering on both systems.
  - Huge ecosystem; the TypeScript MCP SDK.
- **Against:**
  - Large app (~100 MB+) and higher memory use.
  - Security is easy to get wrong; it needs careful configuration.
  - The free update service needs a **signed** macOS app, which we will not have until the first public release.
- **Fit:** covers every need. It costs size and memory, not capability.

### Electrobun 2 — promising but too young

- **For:**
  - Small app with a TypeScript native side.
  - Optional bundled Chromium (CEF) for identical rendering.
  - Fast-moving.
- **Against:**
  - **No Intel Mac support**: macOS builds are Apple Silicon only.
  - v1 shipped in February 2026 and v2 in August 2026 with **breaking changes**, so the rules change often.
  - Small company behind it; young ecosystem with reported rough edges.
- **Fit:** the missing Intel Mac support, and the rapid breaking changes, make it a risk for the core of the product.

### Wails 2 — workable, but its next version is in long beta

- **For:**
  - Small app; Go is simple and good at running processes and servers.
  - Stable v2.
- **Against:**
  - No official updater in v2.
  - v3 (multi-window and more) has been in beta for a long time (beta.27 in October 2026).
  - Smaller community than Tauri.
- **Fit:** possible, but Tauri offers the same small size with more maturity and an official updater.

### Neutralinojs — too limited

- **For:** extremely small; simple.
- **Against:**
  - The native API is limited; real native work goes into separate extension processes.
  - Smaller community; licence to be checked.
- **Fit:** weak for glidebox's heavy native side (Git, agents, MCP, servers).

### NW.js — older approach, no advantage

- Bundles Chromium and Node like Electron, but has a smaller ecosystem and riskier defaults.
- No reason to choose it over Electron.

## Recommendation

1. **Tauri 2** (stable 2.x line), with a planned check of the Tauri 3 migration when 3.0 becomes stable.
2. **Electron** if the owner prefers maximum maturity and identical rendering over app size.

Not recommended for glidebox now:
- **Electrobun**: no Intel Mac support, and too young.
- **Wails**: v3 in beta, no official updater in v2.
- **Neutralinojs**: limited native side.
- **NW.js**: no advantage over Electron.

**If Tauri is chosen:** the project rules are written in Rust inside the desktop app, and the CLI and MCP server reuse them. The web interface stays TypeScript. `Task-W01-01` would be written in Rust.

## Sources (read 2026-10-04)

- GitHub API: stars, licences and releases for `electron/electron`, `tauri-apps/tauri`, `blackboardsh/electrobun`, `wailsapp/wails`, `neutralinojs/neutralinojs`, `nwjs/nw.js`.
- Electron: [releases](https://releases.electronjs.org/), [timelines](https://www.electronjs.org/docs/latest/tutorial/electron-timelines), [updates](https://www.electronjs.org/docs/latest/tutorial/updates).
- Tauri: [what is Tauri](https://v2.tauri.app/start/), [updater plugin](https://v2.tauri.app/plugin/updater/), [shell plugin](https://v2.tauri.app/plugin/shell/), [release notes](https://v2.tauri.app/release/tauri/), [3.0.0-alpha.0](https://newreleases.io/project/github/tauri-apps/tauri/release/tauri-v3.0.0-alpha.0).
- Electrobun: [compatibility](https://framework.blackboard.sh/electrobun/guides/compatability), [v2 changelog](https://framework.blackboard.sh/electrobun/guides/changelog/v2-x/), [Better Stack guide](https://betterstack.com/community/guides/scaling-nodejs/electrobun-desktop-apps-typescript/).
- Wails: [v3 beta release](https://newreleases.io/project/github/wailsapp/wails/release/v3.0.0-beta.0).
- Neutralinojs: [documentation](https://neutralino.js.org/docs/).
- MCP: [official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk).
