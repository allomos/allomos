# 0010 — A project's data lives in a folder the user chooses

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-04 | 2026-10-04 (requirement) | will.w.hui |

## In plain words

In the desktop app, you decide **where each project's data is stored**. It can be any folder:
- a **folder of its own**, for example `~/glidebox/my-project`;
- a **subfolder inside an existing project folder**, for example a `glidebox/` folder inside a code repository, so the tasks live right next to the code they describe.

Glidebox does not force all projects into one hidden app-data location.

## Name: "task box" (owner, 2026-10-04)

We call a project's folder the **task box** — in the documents, the app's screens and the messages of the `glidebox` command. On disk its proposed default name inside a repository is `.glidebox/`.

## Owner's statement (2026-10-04)

> In the desktop application, the data source folder should be able to be specified to any folder, which includes an independent folder, a subfolder of an existing project folder,

The sentence ends with a comma, so the owner may have more kinds of location in mind (question 1 below).

## Decision

- When creating or opening a local project, the user **chooses its task box**. Any folder they can write to is allowed.
- Supported locations include a **standalone folder** and a **subfolder of an existing project folder**, such as a code repository.
- The desktop app keeps a list of the projects it knows and their folders. Opening a folder that already holds glidebox data opens that project.
- **Commit and push run the real `git` command** (owner, 2026-10-04). When glidebox commits or pushes a project's data, it calls the `git` program installed on the user's machine. It does not reimplement Git or use a separate Git library for these actions. In plain words: it does exactly what typing `git commit` / `git push` in a terminal would do. That means:
  - the user's own Git settings apply: name and email, SSH keys, credential helpers;
  - the repository's hooks run (for example a pre-commit check), and a refusal from a hook is shown to the user, not bypassed;
  - the resulting history is ordinary Git history that any Git tool can read.

## Confirmed details (owner, 2026-10-04)

- **Self-contained folder.** Everything that defines the project lives in its folder, so moving or copying the folder moves the project. Caches and indexes that can be rebuilt are kept outside it.
- **Inside a Git repository** (for example, a subfolder of a code repo): glidebox uses that repository's Git and does not create a second repository nested inside.
- **Glidebox never commits or pushes unless the user asks.**
- **One writer at a time.** Only one running desktop app writes to a folder at a time. A second one opens it read-only, or is refused.
- **All Git actions use the `git` command**, not only commit and push: pull, status and diff too.
- **If `git` is not installed**, the desktop app asks the user to install it. Projects stay usable without Git actions until then.
- **Only standalone folders and subfolders of an existing project folder** are in scope. No special handling is planned for synced folders (Dropbox, iCloud), network drives or removable disks.
- **One glidebox project per repository.** Several glidebox projects in different subfolders of one code repository are **out of scope**.

## Proposed detail (not yet confirmed)

- **Default name inside a repository: `.glidebox/`** at the repository root, like `.github/` or `.vscode/`. `gb init` creates it there unless told otherwise; any other folder can still be chosen (R72).

- **Folder gone or moved.** The app shows the project as unavailable and lets the user locate it again, instead of silently creating an empty project.

## How this fits earlier decisions

- [0006](./0006-local-and-cloud-projects.md): this is where a local project's data lives. It stays on the user's machine and never uploads by itself.
- [0001](./0001-database-source-of-truth.md) and [0007](./0007-git-like-project-sync.md): the folder works with either storage choice. **Decided 2026-10-04: the folder holds readable text files tracked by Git (0007), not a database file (0001).** This suits a subfolder inside a code repository especially well: task history is reviewed and versioned together with the code.
- [0009](./0009-desktop-hub-with-web-and-mobile-companions.md): the desktop hub opens projects from their folders. Companions see the projects through the desktop app, never the folders directly.

## Open questions (owner)

Answered 2026-10-04: no other kinds of location; one project per repository; ask the user to install `git`; all Git actions use the `git` command.

~~What goes in the folder~~ — **decided 2026-10-04: readable text files tracked by Git** ([0007](./0007-git-like-project-sync.md)). No questions remain open on this record.
