# Task-A07-16-update-reminder

*Card type: coding. Area A07 (Platform & release). Written 2026-10-15 at the approver's request: "check the github release address, remind the user to update".*

## What gets built (plain words)

Allomos asks GitHub which version is the newest one published. When it is newer than the Allomos that is running, the window says so and offers the download, and the app menu offers it too, the way the Claude app offers its update.

| Piece | What it is in everyday words | Where |
|---|---|---|
| The check | Asks GitHub's "latest release" address with `curl` (already on macOS and Windows 10+, so no new library), at most once a day, and remembers the answer and when it got it | `apps/desktop/src-tauri/src/update.rs`, settings in `crates/store/src/settings.rs` |
| The notice | Above the task list: "Allomos 0.1.6 is available. You have 0.1.5." with **Download the newest Allomos**, **Skip this version** and × | `UpdateOffer` in `apps/web/src/App.tsx`, `apps/web/src/updateFlow.ts` |
| The menu item | **Check for Updates…** under About in the Allomos menu (macOS) or in Help (Windows). Once a newer version is known it reads **Download Allomos 0.1.6…** and opens the download page | `build_menu`, `set_update_item` in `apps/desktop/src-tauri/src/lib.rs` |

Rules:
- At start, and every hour while open, the window asks the app; the app asks GitHub only when its last answer is 24 hours old (approver, 2026-10-15). Check for Updates… asks GitHub at once.
- A check on its own stays quiet when there is nothing newer, or when GitHub cannot be reached. Check for Updates… always answers: newer, up to date, or "couldn't reach GitHub".
- **Skip this version** is remembered: that version is not shown again on its own, but a later one is, and the menu still offers it. × hides the notice in this window until the app is started again.
- Versions are compared as numbers (`0.1.10` is newer than `0.1.9`); a tag that is not `vX.Y.Z` is ignored.

A real **Restart to Update** (Allomos downloads and installs the new version itself) is a separate card, [`A07-17`](./Task-A07-17-restart-to-update.md).

## Details for the worker

- **Outcome.** A person running an older Allomos learns that a newer one exists and can reach its download in one click.
- **Files owned.** The files in the table; the three `apps/web/src/i18n/*.json`; `apps/web/src/types.ts`, `messenger.ts`, `styles.css` (the notice's icon); `apps/web/tests/updateFlow.test.ts`.
- **External operations.** None. The app reads GitHub's public release information; nothing is published or changed.
- **Done means.**
  - `cargo test --workspace`, `npm --prefix apps/web test`, `typecheck` and `check:i18n` pass.
  - In the app, with a newer release known, the notice and the menu item show; with none, nothing shows; Check for Updates… answers in each case; offline, the start is quiet.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | The design as shown in chat (notice with Download, Skip this version, ×); check again when the last check is 24 hours old; a menu item for updates as the Claude app has | approver |

## Notes

- 2026-10-15 — Built in branch `claude/a07-16-update-reminder`. Plain words: the app now asks GitHub once a day which Allomos is newest, shows a notice when a newer one exists, and has a Check for Updates… menu item that turns into "Download Allomos <version>…" once one is known.
  Evidence: `cargo test --workspace` all pass (5 new tests in `update::tests`, the settings test extended); `cargo clippy` clean, `cargo fmt --check` pass; `npm --prefix apps/web test` 100 pass (5 new in `updateFlow.test.ts`); `typecheck` pass; `check:i18n` 444 texts. GitHub's real answer read by hand: `releases/latest` → `v0.1.5`, not draft, not pre-release.
  Not yet looked at in the window: the development build ran with a test settings folder (`~/allomos-tests/A07-16-update/`, which pretends GitHub said 0.1.6), but no screenshot of it could be taken in this session.
