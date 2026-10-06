# Task-A01-02-ui-languages

*Second card of **Iteration 1 — v0.1 macOS local tracker** ([PLAN § 9](../product/PLAN.md#9-roadmap--releases)). A **coding** card. It implements R94 while the interface is still small (owner, 2026-10-06; [discussion](../discussions/2026-10-04-docs-review.md#ui-languages-2026-10-06)).*

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| Language files | One file per language holding every text the interface shows: English (`en`), Simplified Chinese (`zh-CN`), Japanese (`ja`). English is the reference. | `apps/web/src/i18n/` |
| Language choice | The app starts in the computer's language and falls back to English when that language is not one of the three. A small setting in the window lets you change it, and the app remembers your choice. | `apps/web/src/App.tsx`, and `apps/desktop` if the computer's language must come from Rust |
| Problem codes | Rust no longer sends English sentences to the interface. It sends a **code** (for example `field_missing`) plus the details (file, field, value). The interface turns each code into a sentence in the chosen language. Rust keeps one English wording per code, for logs and later for `gb`. | `crates/core`, `crates/store`, `apps/desktop/src-tauri` |
| Translation check | An automatic check that **fails** when a language file misses a text, has an extra or empty one, or uses different `{placeholders}` from English. It also fails when Rust can send a code that a language file does not word. | `apps/web` check script; a Rust test |
| Rule for agents | A coding rule: no interface text written directly in the code; write English first and add Chinese and Japanese in the same edit; keep text short and plain. | `docs/workflow/CODING_RULES.md` |

**What you can see at the end:** the same app as today, in English, Chinese or Japanese. A broken file shows its problem in the chosen language, still naming the file and field. Switching the language changes every screen at once.

## Details for the worker

- **Outcome.** The owner opens glidebox on a Mac set to Chinese or Japanese and sees every screen in that language, including problems found in files. They can switch to any of the three languages, and the choice is kept after restarting. An agent that forgets a translation, or adds a Rust problem without words for it, gets a failing check before committing.
- **What is translated, and what is not.**
  - Translated: everything the interface itself says, such as buttons, headings, notices, state names (Backlog, Ready …) and problem explanations.
  - Not translated: the user's own data (project, area and task names, descriptions), file paths and field names. Technical detail passed through from a library (for example a JSON parser's "line 3 column 5") is shown as it comes, after the translated sentence.
  - The words a project chooses for "area" and "iteration" (R90) come later. Text keys stay simple so that renaming can be added without rewriting them.
- **Decisive unknowns** (the worker finds out):
  - **How the app learns the computer's language.** Inside a Tauri window on macOS, `navigator.language` may report English unless the app declares the languages it supports in its bundle. The worker checks this on a Mac set to Chinese and to Japanese. If the window cannot be trusted, the language is read on the Rust side and passed to the interface, using a small pinned crate (for example `sys-locale`) with its licence noted.
  - **How the translation check runs without a new test tool.** Node 22 is installed. A plain Node script run by `npm --prefix apps/web run check:i18n` is enough. If the language files are TypeScript typed from the English file, `typecheck` also catches missing texts. Any new package is pinned in this card.
  - **How the Rust codes reach the check.** Proposal: the Rust problem codes form one list (an enum). A Rust test writes that list to a committed file, `apps/web/src/i18n/codes.json`, and fails when the committed file is out of date. The web check then requires words for every code in every language.
  - **Errors that are not problems.** Today, some desktop commands return a plain English error string, for example when creating a task box in a folder that is not empty. These become codes too.
- **Files owned.**
  - `apps/web/src/i18n/**` (new), `apps/web/src/App.tsx`, `apps/web/src/types.ts`, `apps/web/src/messenger.ts` (the folder picker's title), `apps/web/package.json` (the check script only);
  - `crates/core/src/validate.rs`, `crates/store/src/lib.rs` and their tests (problems carry a code and details);
  - `apps/desktop/src-tauri/**` (error codes; the computer's language, if needed; the bundle's language list, if needed);
  - `docs/workflow/CODING_RULES.md` (text rules and the new check in "Checks before a commit");
  - this card. Shared: `Cargo.toml`, `Cargo.lock` only if a crate is added.
- **External operations.** None.
- **Done means mechanically.**
  - These pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n`.
  - **The check catches mistakes:** removing one text from `ja`, or adding a Rust code without words, makes the check fail. This is shown once and recorded on the card, then undone.
  - No interface text remains written directly in `App.tsx` or `messenger.ts`. The worker shows this with a search, recorded on the card.
  - With the Mac set to Chinese, the app starts in Chinese; set to Japanese, in Japanese; set to anything else, in English. A chosen language survives a restart.
  - A broken file's problem shows in each language and still names the file and field.
  - The owner sees the app in Chinese and Japanese and checks that the wording reads naturally.

## Owner answers (2026-10-06)

1. **The starter project's sample areas and tasks stay in English.** They become the user's own data, not interface text.
2. **The owner checks the Chinese and Japanese wording** at the visual check; the agent writes both translations, and the card records anything the owner changes.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | Card approved: all local work (source, tests, docs, commits, pushes; installing pinned packages locally). No external operations. | owner (chat) |

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
- 2026-10-06 — Owner verified: wording and restart check passed. Done.
- 2026-10-06 — Built and locally verified by the agent. The first session stopped at its usage limit before committing; a second session resumed the same worktree and finished it.

## Evidence (2026-10-06, agent)

**How it works, in plain words.** Every text the interface shows now lives in three language files (`apps/web/src/i18n/en.json`, `zh-CN.json`, `ja.json`), 53 texts each. Rust no longer sends English sentences: a problem is a **code** plus details (19 codes, listed once in `crates/core/src/problem.rs`), and the interface words it. The computer's language is read in Rust with `sys-locale` =0.3.2 (MIT or Apache-2.0); the user's choice is stored in the window's storage. The text rules are in [CODING_RULES.md § Interface languages](../workflow/CODING_RULES.md#interface-languages).

**Decisive unknowns, answered.**
- Computer's language: `sys-locale` returns `["ja-JP", "en-JP", "zh-Hant-JP", "zh-Hans-JP"]` on the owner's Mac (system language Japanese), matching `defaults read -g AppleLanguages`. The window's own `navigator.language` was not needed and was not tested separately.
- Check without a new tool: a plain Node script, `npm --prefix apps/web run check:i18n`. No new npm package.
- Rust codes reach the check through `apps/web/src/i18n/codes.json`, kept current by the test `codes_file_is_up_to_date`.
- Non-problem errors: creating a task box now returns the codes `folder_not_empty` or `create_failed`.

**Checks (all PASS):** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (18 tests), `npm --prefix apps/web run typecheck`, `npm --prefix apps/web run build`, `npm --prefix apps/web run check:i18n` ("53 texts in en, zh-CN and ja; 19 Rust codes").

**The check catches mistakes** (shown, then undone):
- Removing `pending.cancel` from `ja` and dropping `{total}` from `area.count`: `check:i18n` exits 1 with `ja: missing "pending.cancel"` and `ja: "area.count" uses {done} but English uses {done,total}`; `typecheck` exits 2.
- Adding a Rust code `demo_only` without words: `cargo test` fails with "codes.json is out of date"; after rewriting the file, `check:i18n` exits 1 with `en: Rust code "demo_only" has no words`.

**No text written in the code:** a search of `App.tsx` and `messenger.ts` for text between tags, sentence-like strings and literal `title` / `aria-label` / `placeholder` values finds only `aria-label="Glidebox"`, the product name.

**In the built app** (`target/debug/bundle/macos/Glidebox.app`, a synthetic task box with one task given the state `doing`; language set for one launch with `-AppleLanguages`, the Mac's own setting unchanged):
- Japanese: the window, state names and the problem (「"finished" は有効な値ではありません…」) are in Japanese, still naming `tasks/A01-02/task.json` and field `state`.
- Simplified Chinese: 「此任务盒中有 1 个问题」, 「"doing" 不是有效的值。请使用以下之一…」, file and field named.
- French (not one of the three): English.
- Choosing a language and restarting: checked by the owner (below). The app's stored choice read `zh-CN`, then `en`, then none (follow the computer), matching each pick.

**Owner check (2026-10-06): passed.** The owner read the Chinese and Japanese wording and found nothing to change, and confirmed that a chosen language survives a restart.

**Found on the way (not part of this card):** the app reads only its *first* start-up argument as the folder, so an argument placed before the folder makes it ignore the folder and reopen the last one.
