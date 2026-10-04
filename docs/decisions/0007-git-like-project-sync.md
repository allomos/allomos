# 0007 — Git-like project history and synchronization

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** — Git with readable text files is the source of truth; sync and merge details are proposals to prototype | 2026-10-01 | 2026-10-04 | will.w.hui |

## In plain words

The owner asked whether glidebox projects could work like code in Git: edit a project on your own machine, then **push** your changes to a shared cloud copy, **pull** other people's changes, and **merge** when you both changed the same thing. Glidebox does that with real Git, storing each task as small readable text files. A database is only a fast index rebuilt from those files. **The owner approved this on 2026-10-04**, and it replaces decision 0001 (the database is the source of truth). Since [0009](./0009-desktop-hub-with-web-and-mobile-companions.md), the desktop app is the hub, which would own the local Git copy.

*Update 2026-10-04:* [0010](./0010-project-data-folder.md) lets a project's task box sit inside an existing code repository. That repository's Git versions the task files, rather than glidebox creating a nested repository. Commit and push must run the user's installed `git` command (R76), not a separate Git implementation.

## Decision (owner, 2026-10-04)

- **A project's data is readable text files** in its task box ([0010](./0010-project-data-folder.md)): JSON for structured fields, Markdown for narrative text, one canonical record per task. **Git is the source of truth** and holds their history.
- **Any database is only an index**: a cache kept outside the task box, used for search, charts and speed, and rebuilt from the files at any time. It is never edited on its own.
- **All Git actions run the user's `git` command** (R76). Glidebox never commits or pushes unless the user asks (R75).
- **Accepted here:** this storage foundation. **Still proposals:** the file layout, the push/pull/merge experience, the merge rules, offline claims and approvals, and the cloud remote (sections below). They are to be prototyped with synthetic data in implementation cards, and the owner reviews them there.
- This **supersedes [0001](./0001-database-source-of-truth.md)**. 0001's goals still hold: structured fields, one home for status, events recorded for every change, and no hand-maintained prose status.

## Context

The owner asked whether project/task storage could work like Git/GitHub: use a project locally, then push, pull and merge with a cloud copy. Local-only use and optional signed-in cloud access are already accepted in [0006](./0006-local-and-cloud-projects.md). This record proposes a design; it does not approve storage technology, implementation or hosted resources.

## Proposal

Following the owner’s follow-up, the recommended candidate is an actual Git repository per project, containing schema-validated text files and versioned change history. It may have an explicitly connected cloud remote. The cloud stores the shared accepted history and enforces cloud permissions; local changes remain usable and visibly unpublished until pushed. Sign-in is required to connect to and access the remote, not to use an independent local project.

- **Create locally:** initialize project data and history without an account.
- **Connect/publish:** sign in, select or create an authorized cloud project, preview what will be shared, then publish explicitly.
- **Clone:** sign in and clone an authorized project repository and history locally.
- **Pull:** fetch remote changes and integrate them, resolving conflicts before advancing the local accepted view.
- **Push:** submit local changes with their expected remote revision. The server validates identity, permissions and project rules. If the remote advanced, pull and merge before retrying.
- **Merge:** reconcile changes from their common ancestor; record the resolution and both histories.

A project remains usable without internet after it is available locally. Offline access retains previously downloaded data; cloud membership revocation can prevent future remote access but cannot erase copies already held offline. Exactly how cloud-project offline access works remains subject to design approval.

## History and persistence

Git stores the authoritative structured files and their revision history. A local or hosted database may index them for search, charts and UI performance, but it is rebuildable from a known Git revision and must not become an independently writable source of truth. GitHub could host a remote; Glidebox could also provide its own authenticated Git service. Git is the storage/transport tool, while GitHub is one hosting choice.

Illustrative layout (not a finalized schema):

```text
project-repo/
  project.json
  areas/<stable-id>.json
  tasks/<stable-id>/task.json
  tasks/<stable-id>/description.md
  events/<unique-event-id>.json
  approvals/<unique-approval-id>.json
  .gitattributes
  .gitignore
```

Use JSON for structured fields and Markdown for narrative text. Keep stable IDs in paths; task labels such as A01-01 and area membership are fields, so renaming or regrouping need not move files. Serialize consistently for readable diffs. Give events separate files to reduce concurrent appends to a shared log. Derived boards and chart data are not separately edited tracked state. Task snapshots and their transition events must be validated and committed together; merge resolutions must record reconciled transitions. Full snapshot/event consistency rules remain to be specified.

Approvals in files are requests/records, not proof of an authenticated actor. Validate authority through the local trust model or authenticated remote acceptance. Keep runtime processes, session liveness, credentials and disposable indexes outside the tracked project content.

This decision **supersedes** the database-source-of-truth decision [0001](./0001-database-source-of-truth.md) (owner, 2026-10-04). 0001's original objection to manually maintained prose status still applies: files here would have a strict schema, one canonical task record, validation and generated views.

Do not rewrite audit history in normal use. Preserve original actor/time metadata and record remote acceptance separately. Forecasts should distinguish local/unpublished work from accepted shared history and deduplicate changes by stable ID. Exact event-time rules, snapshots, migrations and history retention remain design questions.

## Merge behavior

| Concurrent changes | Proposed treatment |
|---|---|
| Different tasks | Combine unless they violate cross-task rules |
| Different fields of one task | Combine when the resulting task is valid |
| Same field, same resulting value | Accept the common value |
| Same field, different values | Ask the user to choose or edit |
| Comments or evidence links added independently | Keep both with stable IDs |
| Delete/archive versus edit | Explicit conflict; preserve recoverable history |
| Completion versus new blocker or dependency | Explicit review; do not infer readiness from text merging |
| Claim, membership or approval changes | Enforce domain-specific authorization and concurrency rules |

A three-way merge compares the common base, local value and remote value. Timestamps alone must not decide conflicting status, ownership or approvals. Even changes to different tasks can introduce an invalid dependency cycle, so validate the merged project, not just individual records.

## Ownership and approvals

Disconnected replicas cannot guarantee one globally exclusive holder. A local claim is exclusive only within its local authority. For a connected shared project, confirmed claims require a server-side atomic operation; offline claims are tentative and visibly distinguishable. Conflicting claims need reconciliation and do not authorize simultaneous work silently.

An approval applies to an exact operation and relevant target/revision, rather than being a mergeable boolean. Ordinary edits may be local/offline; cloud approval authority must be authenticated and validated when submitted. Do not treat a locally written owner name as proof of cloud approval. External effects still require the owner's exact approval under AGENTS.md.

## Scope recommendation

Start with one shared main history per project, local change recording, explicit push/pull and field-level conflict resolution. Autosave can record local changes without asking users to make a manual commit after every task edit. Offer human-readable differences such as “you marked this verified; Alex added a failing check.” Add branches and proposal/review flows only when a demonstrated workflow requires them.

The owner asked whether Git can be the foundation with structured folders and text files. It can supply version storage and push/pull transport, reducing the custom synchronization engine we would need to build. Glidebox still needs schema validation, task-aware merge, permissions, atomic local writes and claim/approval handling. Prototype those domain rules first, in the implementation cards. A local runtime is needed to operate local Git and files; a hosted webpage alone does not establish that capability.

## Evidence and open questions

[Git merge documentation](https://git-scm.com/docs/git-merge) describes merging divergent histories with common ancestry. [Git push documentation](https://git-scm.com/docs/git-push) documents remote reference updates and non-fast-forward restrictions. Consulted 2026-10-01. These establish the analogy, not the feasibility or correctness of this unimplemented proposal.

Git as the source of truth was accepted on 2026-10-04. Still to decide: the user-visible synchronization model, local/cloud packaging, offline permissions, actor verification, forecast event times and conflict experience. Then prototype divergent edits, remote advancement during push, offline competing claims and stale approvals with synthetic data before selecting production storage.

[Git attributes documentation](https://git-scm.com/docs/gitattributes) supports custom merge drivers, but driver configuration must be installed on each relevant client/server; a tracked attributes file alone does not install the executable. Git line merging does not establish valid task semantics. Consulted 2026-10-01.
