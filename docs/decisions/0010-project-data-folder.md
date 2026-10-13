# 0010 — A project's data lives in a folder the user chooses

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** | 2026-10-04 | 2026-10-04 (requirement) | will.w.hui |

## In plain words

In the desktop app, you decide **where each project's data is stored**. It can be any folder:
- a **folder of its own**, for example `~/Allomos/my-project`;
- a **subfolder inside an existing project folder**, for example a `Allomos/` folder inside a code repository, so the tasks live right next to the code they describe.

Allomos does not force all projects into one hidden app-data location.

## Name: "taskbag" (owner; "task box" 2026-10-04, renamed "taskbag" 2026-10-06)

We call a project's folder the **taskbag** — in the documents, the app's screens and the messages of the `allomos` command. **On disk, a new taskbag always goes in `.allomos/taskbag/`** inside the folder the user picks, at that folder's root like `.github/` or `.vscode/`. This holds also when the picked folder is empty, so agents, `alm` and the app look in one place only (owner, 2026-10-06, [0013](./0013-product-name-fliphopper.md); until then an empty folder became the taskbag itself). `alm init` creates it there. The user still picks any writable folder (R72); a folder that already holds `project.json` directly is opened as it is.

## Owner's statement (2026-10-04)

> In the desktop application, the data source folder should be able to be specified to any folder, which includes an independent folder, a subfolder of an existing project folder,

The sentence ends with a comma, so the owner may have more kinds of location in mind (question 1 below).

## Decision

- When creating or opening a local project, the user **chooses its taskbag**. Any folder they can write to is allowed.
- Supported locations include a **standalone folder** and a **subfolder of an existing project folder**, such as a code repository.
- The desktop app keeps a list of the projects it knows and their folders. Opening a folder that already holds Allomos data opens that project.
- **Commit and push run the real `git` command** (owner, 2026-10-04). When Allomos commits or pushes a project's data, it calls the `git` program installed on the user's machine. It does not reimplement Git or use a separate Git library for these actions. In plain words: it does exactly what typing `git commit` / `git push` in a terminal would do. That means:
  - the user's own Git settings apply: name and email, SSH keys, credential helpers;
  - the repository's hooks run (for example a pre-commit check), and a refusal from a hook is shown to the user, not bypassed;
  - the resulting history is ordinary Git history that any Git tool can read.

## Confirmed details (owner, 2026-10-04)

- **Self-contained folder.** Everything that defines the project lives in its folder, so moving or copying the folder moves the project. Caches and indexes that can be rebuilt are kept outside it.
- **Inside a Git repository** (for example, a subfolder of a code repo): Allomos uses that repository's Git and does not create a second repository nested inside.
- **Allomos never commits or pushes unless the user asks.** One exception, set by the owner on 2026-10-12 ([`Task-A05-03`](../tasks/Task-A05-03-taskbag-setup-wizard.md)): when the setup wizard has a taskbag (step 2), it makes sure the folder holding `.allomos/` is in a Git repository, runs `git init` there when it is not (without asking), and commits the taskbag's files (and `README-allomos.md`) to it, nothing else. It never pushes.
- **One writer at a time.** Only one running desktop app writes to a folder at a time. A second one opens it read-only, or is refused.
- **All Git actions use the `git` command**, not only commit and push: pull, status and diff too.
- **If `git` is not installed**, the desktop app asks the user to install it. Projects stay usable without Git actions until then.
- **Only standalone folders and subfolders of an existing project folder** are in scope. No special handling is planned for synced folders (Dropbox, iCloud), network drives or removable disks.
- **Folder gone or moved** (owner, 2026-10-14; built by [`Task-A01-28`](../tasks/Task-A01-28-forget-missing-last-folder.md)). The app shows the project as unavailable and lets the user locate it again, instead of silently creating an empty project.
- **One Allomos project per repository.** Several Allomos projects in different subfolders of one code repository are **out of scope**.

## How this fits earlier decisions

- [0006](./0006-local-and-cloud-projects.md): this is where a local project's data lives. It stays on the user's machine and never uploads by itself.
- [0001](./0001-database-source-of-truth.md) and [0007](./0007-git-like-project-sync.md): the folder works with either storage choice. **Decided 2026-10-04: the folder holds readable text files tracked by Git (0007), not a database file (0001).** This suits a subfolder inside a code repository especially well: task history is reviewed and versioned together with the code.
- [0009](./0009-desktop-hub-with-web-and-mobile-companions.md): the desktop hub opens projects from their folders. Companions see the projects through the desktop app, never the folders directly.

## Open questions (owner)

Answered 2026-10-04: no other kinds of location; one project per repository; ask the user to install `git`; all Git actions use the `git` command.

~~What goes in the folder~~ — **decided 2026-10-04: readable text files tracked by Git** ([0007](./0007-git-like-project-sync.md)). No questions remain open on this record.
