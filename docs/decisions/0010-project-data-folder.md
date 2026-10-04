# 0010 — A project's data lives in a folder the user chooses

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Requirement accepted; design open** | 2026-10-04 | 2026-10-04 (requirement) | will.w.hui |

## In plain words

In the desktop app, you decide **where each project's data is stored**. It can be any folder:
- a **folder of its own**, for example `~/glidebox/my-project`;
- a **subfolder inside an existing project folder**, for example a `glidebox/` folder inside a code repository, so the tasks live right next to the code they describe.

Glidebox does not force all projects into one hidden app-data location.

## Owner's statement (2026-10-04)

> In the desktop application, the data source folder should be able to be specified to any folder, which includes an independent folder, a subfolder of an existing project folder,

The sentence ends with a comma, so the owner may have more kinds of location in mind (question 1 below).

## Decision

- When creating or opening a local project, the user **chooses its data folder**. Any folder they can write to is allowed.
- Supported locations include a **standalone folder** and a **subfolder of an existing project folder**, such as a code repository.
- The desktop app keeps a list of the projects it knows and their folders. Opening a folder that already holds glidebox data opens that project.
- **Commit and push run the real `git` command** (owner, 2026-10-04). When glidebox commits or pushes a project's data, it calls the `git` program installed on the user's machine. It does not reimplement Git or use a separate Git library for these actions. In plain words: it does exactly what typing `git commit` / `git push` in a terminal would do. That means:
  - the user's own Git settings apply: name and email, SSH keys, credential helpers;
  - the repository's hooks run (for example a pre-commit check), and a refusal from a hook is shown to the user, not bypassed;
  - the resulting history is ordinary Git history that any Git tool can read.

## Proposed details (not yet confirmed)

- **Self-contained folder.** Everything that defines the project lives in its folder, so moving or copying the folder moves the project. Caches and indexes that can be rebuilt sit outside it, or are clearly marked so version control ignores them.
- **Inside a Git repository** (for example, a subfolder of a code repo): glidebox does not create a second repository nested inside. The project's files are versioned by the existing repository. Glidebox never commits or pushes by itself unless the user asks. This fits [0007](./0007-git-like-project-sync.md) (Git storage), if that is accepted.
- **One writer at a time.** Two running desktop apps on the same folder must not corrupt it. The second one opens it read-only, or is refused, until the design says otherwise.
- **Folder gone or moved.** The app shows the project as unavailable and lets the user locate it again, instead of silently creating an empty project.

## How this fits earlier decisions

- [0006](./0006-local-and-cloud-projects.md): this is where a local project's data lives. It stays on the user's machine and never uploads by itself.
- [0001](./0001-database-source-of-truth.md) and [0007](./0007-git-like-project-sync.md): the folder works with either storage choice. Under 0001 it holds a database file (for example SQLite). Under 0007 it holds structured text files tracked by Git. A subfolder inside a code repository suits 0007 especially well: task history is reviewed and versioned together with the code. **This leans towards 0007 but does not decide it.**
- [0009](./0009-desktop-hub-with-web-and-mobile-companions.md): the desktop hub opens projects from their folders. Companions see the projects through the desktop app, never the folders directly.

## Open questions (owner)

1. **Other locations?** The owner's sentence ended with a comma. Are there more cases, for example a folder synced by Dropbox or iCloud, a network drive, or a removable disk?
2. **What goes in the folder:** a single database file (0001) or readable text files (0007)?
3. **Several projects in one repository:** may one code repository hold more than one glidebox project, in different subfolders?
4. **Sync tools:** if the folder is inside Dropbox or iCloud, should glidebox warn about edits from two machines at once?
5. **Git not installed:** if the `git` program is missing, should the desktop app ask the user to install it, or carry its own copy? Until decided: show a clear message, and keep the project fully usable without commit/push.
6. **Other Git actions:** do pull, status and diff also go through the `git` command? (Recommended, for the same reasons.)
