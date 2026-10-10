# Task-A01-21-task-language

*Card type: coding. Area A01 (Tracking). Requested by the owner (2026-10-10), after an agent in a Chinese conversation wrote a new task in English: "add a new task language setting in the seabag; it can be set to any language in the future, but now, make its value refer to the current UI language. When the agent is trying to add a new task or modify a task, it should first ask the current task language by the CLI command."*

## What gets built (plain words)

| Piece | What it is | Where |
|---|---|---|
| The setting | `task_language` in the seabag's `project.json`: `"ui"` (follow FlipHopper's interface language), or a language code such as `zh-CN`, `ja`, `en`, `fr`. New seabags get `"ui"`; a seabag without it is read as `"ui"`. | `crates/core` (model, check), `crates/store` |
| Asking | `fh language` prints the language to write tasks in, and where it comes from. With `"ui"`: the language the FlipHopper app last showed on this computer; before the app has run, the computer's own language, picked the way the app picks (Chinese → zh-CN, Japanese → ja, otherwise en). | `crates/cli`, `crates/store/src/settings.rs` |
| Changing | `fh language <code>` or `fh language ui`. | `crates/cli`, `crates/core` (rule) |
| The app tells `fh` | Whenever the interface language is chosen or changes, the app saves it in FlipHopper's per-user settings file, so `fh` can read it. | `apps/desktop`, `apps/web` |
| Agents ask first | The brief (`fh brief`), `fh guide tasks` and `fh guide workflow`, and the task-moving request: before adding a task or changing its title or description, run `fh language` and write in that language (IDs, commands, code and file names stay as they are). | `crates/store` |

**What you can see at the end:** with the app in Chinese, `fh language` says `zh-CN (follows the FlipHopper app's language)`, and an agent asked to add a task writes its title and description in Chinese.

## Details for the worker

- **Outcome.** Tasks are written in the project's language, which by default is the language the person uses FlipHopper in.
- **Decisive unknowns.** None. Choices made by the agent (owner may change): the value `"ui"` means "follow the app"; a missing field means `"ui"`; FlipHopper's own seabag is set to `en`, because this repository's cards are written in English.
- **Files owned.** `crates/core/src/{model,validate,ops,problem}.rs`, `crates/store/src/{lib,settings,guide,convert}.rs`, `crates/cli/src/lib.rs` and its tests, `apps/desktop/src-tauri/src/lib.rs`, `apps/web/src/{App.tsx,messenger.ts,i18n/*}`, `docs/product/FILE-FORMAT.md`, `docs/workflow/CODING_RULES.md`, this card.
- **External operations.** None.
- **Done means mechanically.** The usual checks pass; tests cover the setting (missing, `ui`, a code, a bad code), `fh language` resolving from the saved app language and from the system, and the brief and guide asking agents to run it; with the app in Chinese, a real agent asked to add a task writes it in Chinese.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-10 | The request above | owner (chat) |

## Notes
