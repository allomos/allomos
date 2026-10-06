# Task-A00-11-project-data-folder

- **Outcome.** The owner's requirement is recorded: in the desktop app, a project's seabag can be any folder the user picks, including a standalone folder or a subfolder of an existing project folder. Related documents agree with it.
- **Decisive unknowns.** None for the record. The design questions it raises go to the owner (see [0010](../decisions/0010-project-data-folder.md)).
- **Files owned.** [decisions/0010](../decisions/0010-project-data-folder.md), [REQUIREMENTS R72–R76](../product/REQUIREMENTS.md#local-and-cloud-projects), related edits to PLAN, 0006, 0007, 0009, GLOSSARY, the board and the handoff.
- **External operations.** None.
- **Done means mechanically.** The requirements and decision are recorded; links and tables check clean; the owner reviews them.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-04 | "In the desktop application, the data source folder should be able to be specified to any folder, which includes an independent folder, a subfolder of an existing project folder" | owner (chat) |
| 2026-10-04 | "The commit and push action should be performed by calling git commands" | owner (chat) |

## Notes

- 2026-10-04 — The owner confirmed R74–R75. All Git actions use the `git` command, and the app asks the user to install `git` if it is missing. No other kinds of location; several projects in one repository are out of scope. Still open: a database file or text files in the folder.
