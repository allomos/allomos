# 0007 — Git-like project history and synchronization

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Proposed** | 2026-10-01 | — | will.w.hui |

## Context

The owner asked whether project/task storage could work like Git/GitHub: use a project locally, then push, pull and merge with a cloud copy. Local-only use and optional signed-in cloud access are already accepted in [0006](./0006-local-and-cloud-projects.md). This record proposes a design; it does not approve storage technology, implementation or hosted resources.

## Proposal

Each project has a local database and versioned change history. It may have an explicitly connected cloud remote. The cloud stores the shared accepted history and enforces cloud permissions; local changes remain usable and visibly unpublished until pushed. Sign-in is required to connect to and access the remote, not to use an independent local project.

- **Create locally:** initialize project data and history without an account.
- **Connect/publish:** sign in, select or create an authorized cloud project, preview what will be shared, then publish explicitly.
- **Clone:** sign in and download an authorized project and history to a local database.
- **Pull:** fetch remote changes and integrate them, resolving conflicts before advancing the local accepted view.
- **Push:** submit local changes with their expected remote revision. The server validates identity, permissions and project rules. If the remote advanced, pull and merge before retrying.
- **Merge:** reconcile changes from their common ancestor; record the resolution and both histories.

A project remains usable without internet after it is available locally. Offline access retains previously downloaded data; cloud membership revocation can prevent future remote access but cannot erase copies already held offline. Exactly how cloud-project offline access works remains subject to design approval.

## History and persistence

Proposed storage: a local database (SQLite is a candidate) and a hosted database (Postgres is a candidate). Versioned change records carry stable project/task IDs, revision parents, operations, actor identity and schema version. The database provides queries and local transactions; the history supplies synchronization and audit information. Avoid merging binary database files directly.

Current database-source-of-truth decision [0001](./0001-database-source-of-truth.md) needs clarification if this is accepted: each replica is an authoritative view of its own revision, while the remote accepted revision is the shared publication point. No replica is assumed globally current while disconnected. Markdown remains an export.

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

Actual Git with structured task files is another candidate. It offers established version transport and history, but task-aware merge, permissions, claims and approvals still need application logic. Compare this candidate with database-backed versioning before selecting an engine. No engine is selected here.

## Evidence and open questions

[Git merge documentation](https://git-scm.com/docs/git-merge) describes merging divergent histories with common ancestry. [Git push documentation](https://git-scm.com/docs/git-push) documents remote reference updates and non-fast-forward restrictions. Consulted 2026-10-01. These establish the analogy, not the feasibility or correctness of this unimplemented proposal.

Decide the user-visible synchronization model, actual Git versus database-backed history, local/cloud packaging, offline permissions, actor verification, forecast event times and conflict experience. Then prototype divergent edits, remote advancement during push, offline competing claims and stale approvals with synthetic data before selecting production storage.
