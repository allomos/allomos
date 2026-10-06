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

## Notes

- 2026-10-06 — Card written by the agent for the owner's review.
