# Task-A07-15-one-alm-link-last-install-wins

*Card type: coding. Area A07 (Platform & release). Written 2026-10-14 at the owner's request: "what if i only keep one of these links. the target should be determined by the later creating link procedure".*

## What gets built (plain words)

On a Mac there is one `alm` that Terminal uses: the link `/usr/local/bin/alm`. Two procedures set where it leads, and whichever ran last wins:

| Procedure | Link leads to | Where |
|---|---|---|
| Installing the app (`.pkg`, or **Install the alm command…** in the app) | the `alm` inside `Allomos.app` | `apps/desktop/pkg/postinstall`, `apps/desktop/src-tauri/src/alm_command.rs` |
| Installing a development build | `~/.cargo/bin/alm` (built from source by `cargo install`) | `scripts/install-dev-alm.sh` |

No second link (such as `~/.local/bin/alm`) comes before it, so `which -a alm` shows which copy is in use. When Terminal's `alm` is not the app's own, the banner says which kind it is (a development build, another program, or none) and shows both paths: the `alm` in use, and the app's own.

## Details for the worker

- **Outcome.** The latest install, app or development, decides which `alm` Terminal runs; the app's banner says which is in use and offers to switch back to the app's own.
- **Decisive unknowns.** None left: the owner decided the link, retired the rule and approved the card in chat (below).
- **Files owned.** `apps/desktop/pkg/postinstall`; `apps/desktop/src-tauri/src/alm_command.rs` (status and its tests); `apps/desktop/src-tauri/src/lib.rs` (`alm_command_status` only); `apps/web/src/almOfferFlow.ts`, `apps/web/src/types.ts` (`AlmStatus`), the banner in `apps/web/src/App.tsx`, `apps/web/src/i18n/{en,zh-CN,ja}.json` (`alm.*`), `apps/web/tests/almOfferFlow.test.ts`; new `scripts/install-dev-alm.sh`; `docs/workflow/CODING_RULES.md` (Install on a Mac); this card. Shared: the taskbag's `project-notes.md` line "`alm` on this Mac".
- **External operations.** None. Changing this Mac's own `/usr/local/bin/alm` and removing `~/.local/bin/alm` are the owner's own steps in the human check.
- **Done means mechanically.**
  - `postinstall` makes the link whatever is at `/usr/local/bin/alm` (no `alm`, a link to anything, a real file): checked in a scratch folder.
  - `install-dev-alm.sh` installs from source and points the link at `~/.cargo/bin/alm`; run again it asks nothing when the link is already there.
  - Unit tests for the status: missing, development build (directly and through the link), another program, the app's own; both paths reported.
  - Web tests for the banner lines; typecheck, translation check, clippy, Rust and web tests pass.
  - Human check on this Mac: steps on the task.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | One link, `/usr/local/bin/alm`, set by whichever procedure ran last; the rule "never replace someone else's alm" (2026-10-13) retired; the banner names the kind and shows the `alm` in use and the app's own; start the card | owner (chat) |

## Notes

- 2026-10-14: `~/.local/bin/alm` → `~/.cargo/bin/alm` on this Mac was made by hand (no procedure in the repo makes it); it put the development build ahead of the link.
- Relation to task A07-14: its third point ("a development `alm` that wins") is covered here; "Don't ask again" per mismatch and Windows stay there.
- 2026-10-14, implemented on `claude/a07-15-one-alm-link`:
  - `postinstall` now always links to the app's `alm`. Scratch check (link folder redirected): nothing there, a link to the development build, a link to another program, a real file → all four end as a link to `Allomos.app/Contents/MacOS/alm`.
  - `scripts/install-dev-alm.sh`: `cargo install`, then `sudo ln -sfn ~/.cargo/bin/alm /usr/local/bin/alm` only when the link leads elsewhere; dry run (cargo and sudo left out, link folder redirected) linked it, and a second run asked nothing. It says when this shell finds another `alm` first.
  - Status gains `kind` (missing / development / other), `own`, `leads_to`; the banner shows the sentence, then "In use: … → …" and "This Allomos's own: …", and for a development build the button **Use this Allomos's alm…**. en, zh-CN, ja.
  - Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` (7 `alm_command` cases, 1 new), web `typecheck`, `check:i18n` (362 texts), `test` (79), `build`: all PASS. NOT RUN: the banner in a built Allomos.app, and the real `/usr/local/bin` link (both the owner's human check).
