# Task-A05-05-commit-recognized-project-folder

*Card type: coding. Area A05 (Project setup & import). Written 2026-10-12 from the owner's check of [`A05-03`](./Task-A05-03-taskbag-setup-wizard.md). **Not started: the owner asked for the card only.***

## What gets built (plain words)

When a taskbag is opened and Allomos finds `AGENTS.md` or `CLAUDE.md` beside `.allomos/`, it records that folder as the project folder in the taskbag's `project.json` by itself (`setup_state`, [OPEN-FOLDER-FLOW](../product/OPEN-FOLDER-FLOW.md)). That write is left uncommitted: `git status` shows `project.json` changed until the wizard (or an agent's `--commit`/`--push`) commits it. Seen on 2026-10-12 with the test project `all-set`.

After this card, Allomos commits that change right away, the same way the wizard keeps the taskbag in Git ("Update Allomos taskbag", only the taskbag's files, never a push; decision [0010](../decisions/0010-project-data-folder.md)'s wizard exception would need to cover it).

## Details for the worker

- **Outcome.** Opening a taskbag whose project folder Allomos recognizes leaves no uncommitted change.
- **Decisive unknowns.** Whether committing outside the wizard is within the owner's Git rule (owner); whether to commit when the folder is in no repository (the wizard runs `git init` then; here probably not — owner).
- **Files owned.** `apps/desktop/src-tauri/src/lib.rs` (`project_setup` / `keep_in_git`), `apps/web/src/App.tsx`, tests; `docs/product/OPEN-FOLDER-FLOW.md`.
- **External operations.** None.
- **Done means mechanically.** A test: open a taskbag with `AGENTS.md` beside it in a repository → `project.json` recorded and committed, nothing else staged or committed.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-12 — Card written at the owner's request; not to be started yet.
