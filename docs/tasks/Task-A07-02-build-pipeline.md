# Task-A07-02-build-pipeline

*Card type: coding. Area A07 (Platform & release), Iteration 2. Written 2026-10-09 at the owner's request for the rest of v0.2 ([PLAN § 9](../product/PLAN.md#9-roadmap--releases): "one build pipeline producing both systems' builds"; [R85](../product/REQUIREMENTS.md): macOS and Windows released together, unsigned until v1.0). **Rewritten 2026-10-11** (owner): build Windows on the Mac with one script instead of a GitHub pipeline, because the `mypals-ml` organization is on GitHub's Free plan and the repository is private, so every pipeline run would use up the free minutes (macOS counts 10×, Windows 2×). A GitHub job that is started by hand, for running the tests on a real Windows computer, is left to [`A07-03`](./Task-A07-03-windows-works.md).*

## What gets built (plain words)

Allomos's Windows version can be built on the owner's Mac, without a Windows computer and without GitHub. One script makes the Windows installer and `alm.exe` and puts them in one folder. The Mac can only *build* the Windows version; it cannot run it, so no tests run on Windows here.

| Piece | What it is | Where |
|---|---|---|
| The script | Builds the unsigned Windows installer (NSIS `.exe`, which needs no administrator rights) and `alm.exe`, and copies both to one folder. Checks the tools are installed first and says how to install any that are missing. | `apps/desktop/scripts/build-windows.sh` |
| How to build for Windows | One-time tool set-up, the command, where the files land, the expected "unknown publisher" warning. | `docs/workflow/CODING_RULES.md` |

**What you can see at the end:** run `bash apps/desktop/scripts/build-windows.sh` in a worktree; `target/windows/` holds `Allomos_<version>_x64-setup.exe` and `alm.exe`.

## Details for the worker

- **Outcome.** A Windows build exists without a Windows computer and without GitHub minutes.
- **Tools** (installed on the owner's Mac 2026-10-11, with the owner's approval in chat): Homebrew `rustup` (keg-only: it does not replace Homebrew's `rust`; used only through a PATH prefix) with Rust 1.85.1 and the `x86_64-pc-windows-msvc` target; `cargo-xwin` =0.19.2 (the last release that supports Rust 1.85; MIT); Homebrew `makensis` 3.13 and `llvm`.
- **Files owned.** `apps/desktop/scripts/build-windows.sh` (new), `docs/workflow/CODING_RULES.md` (one section), this card.
- **External operations.** None. (`cargo-xwin` downloads Microsoft's Windows SDK libraries into a local cache on first use, and Tauri downloads its NSIS helper plug-in; both are downloads only.)
- **Done means mechanically.**
  - The script, run in a fresh worktree, produces both files; `file` reports a Nullsoft installer and a PE32+ x86-64 console program.
  - The script fails with a clear message when a tool is missing.
  - `git diff --check`; the docs' links resolve.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-09 | Write the card (not start it) | owner (chat) |
| 2026-10-11 | Install the Windows build tools on the Mac (rustup alongside Homebrew's Rust, cargo-xwin, makensis) and build the installer | owner (chat) |
| 2026-10-11 | Rewrite the card as a local Windows build script, no GitHub pipeline; start it | owner (chat) |
| 2026-10-11 | Approve the card; an agent runs the script in a worktree for the owner and closes it | owner (chat) |

## Notes

- 2026-10-09 — Card written by the agent. Next: the owner approves the card and its external operations.
- 2026-10-11 — First cross-build on the Mac made `Allomos_0.1.0_x64-setup.exe` (2.2 MB) and `alm.exe` (1.8 MB). **Pitfall found:** Homebrew `makensis` 3.13 crashes (`std::bad_alloc`) on any script when no locale is set (`LANG` empty, as in agent shells); rebuilding it from source did not help, `LC_ALL=en_US.UTF-8` does. The script sets it.
- 2026-10-11 — Script and CODING_RULES section written (branch `claude/a07-02-windows-build`). Evidence, in a fresh worktree on the Mac:
  - `PATH=/usr/bin:/bin bash apps/desktop/scripts/build-windows.sh` → `build-windows: makensis not found. Install: brew install makensis`, exit 1 — PASS.
  - `bash apps/desktop/scripts/build-windows.sh` (no locale set) → exit 0; `target/windows/Allomos_0.1.0_x64-setup.exe` (2,192,849 bytes, "Nullsoft Installer self-extracting archive") and `alm.exe` (1,776,640 bytes, "PE32+ executable (console) x86-64") — PASS.
  - `git diff --check`, links — PASS. Running the installer on Windows — NOT RUN (no Windows computer; A07-03/A07-04).
  - Next: the owner runs the script once (owner verified).
- 2026-10-11 — Taken over from `claude/a07-02-windows-build` (idle, branch merged) by `claude/a07-02-script-run` at the owner's request. The script, run from `main` (now including A07-06) in a fresh worktree, no locale set: exit 0; `target/windows/Allomos_0.1.0_x64-setup.exe` (2,590,165 bytes, "Nullsoft Installer self-extracting archive"; its `installer.nsi` carries `alm.exe`) and `alm.exe` (1,776,640 bytes, "PE32+ executable (console) x86-64") — PASS. The same script's installers were installed and checked by the owner on Windows for A07-05 and A07-06. **Owner verified** (owner approved, 2026-10-11).
