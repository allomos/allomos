# Task-A07-14-alm-mismatch

*Card type: coding. Area A07 (Platform & release). Written 2026-10-16 from the task the owner asked for on 2026-10-14: the app should ask to install its own `alm` whenever the `alm` Terminal uses does not match the running app.*

## What gets built (plain words)

When Allomos opens, it checks which `alm` a new Terminal window would run. If that is not the app's own `alm`, a banner offers to fix it (**Install the alm command…**, or **Use this Allomos's alm…** for a development build). Two gaps are closed here:

| Gap | Before | After | Where |
|---|---|---|---|
| "Don't ask again" | Silenced the banner forever, even after a new Allomos was installed or the link moved. | Silences only the mismatch it was clicked for: the app remembers the path Terminal's `alm` led to, and asks again when that path changes. | `crates/store/src/settings.rs`, `apps/desktop/src-tauri/src/alm_command.rs` (`decide`), `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/App.tsx`, `apps/web/src/almOfferFlow.ts` |
| Windows | No check. The installer adds the app's folder at the **end** of the person's own PATH, so an `alm.exe` earlier on PATH wins silently. | The app reads the PATH a new terminal gets, finds the first `alm.exe`, and offers the same banner. The click moves the app's folder to the **front** of the person's own PATH (no administrator rights). An `alm.exe` in the system-wide PATH (searched first by Windows) cannot be beaten this way; the banner says it will still be used until removed. | `apps/desktop/src-tauri/src/alm_command.rs` (`windows_terminal`, `install_windows`) |

Terms: **PATH** is the list of folders a terminal searches, in order, for a command; the first match wins. The **link** on a Mac is `/usr/local/bin/alm`, which leads to one real `alm` program.

## Details for the worker

- **Outcome.** The banner shows whenever Terminal's `alm` is not the app's own, on Mac and Windows; "Don't ask again" holds only for the `alm` it was clicked for.
- **Decisive unknowns.** None left for the Mac. Windows: whether the PowerShell fix and the change announcement work on a real Windows computer (untested; see Notes).
- **Files owned.** `apps/desktop/src-tauri/src/alm_command.rs`; `apps/desktop/src-tauri/src/lib.rs` (`alm_command_status`, `install_alm_command`, `dismiss_alm_offer` only); `crates/store/src/settings.rs`, `crates/store/src/lib.rs` (the export line); `apps/web/src/{App.tsx (the alm banner), almOfferFlow.ts, messenger.ts, types.ts}`; `apps/web/src/i18n/{en,zh-CN,ja}.json` (`alm.*`); `apps/web/tests/almOfferFlow.test.ts`; this card.
- **External operations.** None. Changing this Mac's `/usr/local/bin/alm` is the owner's own step in the human check; nothing was installed during testing.
- **Done means mechanically.**
  - Unit tests for `decide`: no offer (no own `alm`, own in use), missing, development build, another program, a real file, one found first; "Don't ask again" for the same mismatch, a changed one, none, a broken link; Windows: own first (any letter case), first in the person's PATH, in the system PATH, none; PATH reading.
  - Settings test: the path is stored; the old plain "yes" setting still reads, silences nothing, and is dropped at the next save.
  - Web test for what "Don't ask again" sends.
  - The seven named checks pass; Windows cross-check (`cargo xwin clippy`) passes; the debug app builds.
  - Human check on this Mac (and on Windows when one is available): steps below.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | The task (both gaps, en/zh-CN/ja, tests, human check steps); the "development `alm` that wins" point moved to A07-15 | owner (chat, recorded on the task) |

## Human check steps

**Mac** (debug app built in this worktree; the `/Applications` Allomos stays as it is):

1. Open the worktree's app: `open -n /Users/wanghui/coding/glidepace/allomos/.claude/worktrees/a07-14-alm-mismatch/target/debug/bundle/macos/Allomos.app`. Terminal's `alm` leads to `/Applications/Allomos.app/…/alm`, not this app's, so the banner says "Terminal uses an alm that is not this Allomos's own…" with both paths. Hover **Don't ask again**: the hint says it is only for this `alm`.
2. Click **Don't ask again**. `~/Library/Application Support/io.allomos/settings.json` now holds `"alm_offer_dismissed_for": "/Applications/Allomos.app/Contents/MacOS/alm"`. Quit and open the app again (step 1's command): no banner.
3. Change Terminal's `alm` (your own step, it asks for the password): `scripts/install-dev-alm.sh` (the link now leads to `~/.cargo/bin/alm`). Open the app again: the banner is back, now naming a development build, with **Use this Allomos's alm…**.
4. Put things back: open `/Applications/Allomos.app`; its banner offers **Use this Allomos's alm…**; click it (password). `ls -l /usr/local/bin/alm` leads to `/Applications/Allomos.app/Contents/MacOS/alm` again.
5. Optional, old setting: quit Allomos, put `"alm_offer_dismissed": true` into `settings.json` in place of the new line, open the worktree app: the banner shows (the old setting silences nothing); **Don't ask again** writes the new line and removes the old one.

**Windows** (untested; needs an installer from this branch, `apps/desktop/scripts/build-windows.sh`):

1. Install it. In PowerShell: `New-Item -ItemType Directory C:\alm-test`, copy any `.exe` there as `alm.exe`, and put `C:\alm-test` **first** in your own PATH (Settings → Environment variables → your user's Path → Move up).
2. Open Allomos: the banner names `C:\alm-test\alm.exe` and the app's own. Click **Don't ask again**, reopen: no banner.
3. Rename `C:\alm-test` to `C:\alm-test2` and fix the PATH entry to match; reopen Allomos: the banner is back.
4. Click **Install the alm command…**: no password; the banner says it is installed. A **new** PowerShell: `(Get-Command alm).Source` is the app's `alm.exe`; your Path starts with the app's folder and every other entry is unchanged (including ones like `%USERPROFILE%\…`).
5. Remove `C:\alm-test2` and its PATH entry.

## Notes

- 2026-10-16, implemented on `claude/a07-14-alm-mismatch` (claude/a07-14-alm-mismatch, quiet loop run):
  - **Don't ask again per mismatch.** The setting `alm_offer_dismissed` (yes/no) became `alm_offer_dismissed_for` (a path). The status the window gets carries `mismatch`, the path Terminal's `alm` leads to (links resolved; where it is when the link leads nowhere; empty when Terminal finds none), and the click sends it back. `decide` is silent only when the remembered path equals the current one.
  - **Windows.** `windows_terminal` asks PowerShell (no window) for the system-wide and the person's PATH as a new terminal gets them, rather than the app's own PATH, which is fixed when the app starts and would miss the fix just made. The first folder holding `alm.exe` wins. `install_windows` edits the person's raw PATH value (so `%…%` entries stay as written), moves only the app's folder to the front, then announces the change. A development build of the app on Windows (`target\debug`) gets no offer.
  - Texts: `alm.offer_missing` and `alm.shadowed` no longer name the Mac or `/usr/local/bin` (they now fit Windows too); new `alm.dismiss_hint`. en, zh-CN, ja.
  - Decided: the remembered mismatch is the path Terminal's `alm` led to only, as the task says, not also the app's own path; a new Allomos installed at the same place does not re-ask unless Terminal's `alm` changes too.
  - Decided: the old yes/no setting is read without error but silences nothing (it names no path), so the person is asked once more; the next save drops it.
  - Decided: on Windows the fix moves the app's folder to the front of the person's own PATH, and an `alm.exe` in the system-wide PATH is reported as "still used until you remove it" (changing the system PATH needs administrator rights; not done).
  - Decided: the change announcement uses .NET's `SetEnvironmentVariable` on an unused variable (it broadcasts the change), instead of compiling a native call in PowerShell.
- Checks, 2026-10-16: `cargo fmt --check` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace` PASS (`alm_command` 12 tests, 5 new; settings test extended); `npm --prefix apps/web run typecheck` PASS; `build` PASS; `check:i18n` PASS (452 texts); `test` PASS (106, 1 new). Extra: `cargo xwin clippy --target x86_64-pc-windows-msvc -p allomos-desktop --all-targets -- -D warnings` PASS (with an empty placeholder `alm.exe`, removed after). Debug app built: `tauri build --debug --bundles app` → `target/debug/bundle/macos/Allomos.app` with `Contents/MacOS/alm`.
- NOT RUN: the banner in the built app (human check, Mac steps); everything on Windows (no Windows computer here): detection, the PATH fix, the announcement. `/usr/local/bin` was not changed and nothing was installed.
