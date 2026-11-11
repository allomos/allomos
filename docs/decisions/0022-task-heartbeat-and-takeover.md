# 0022 — Task heartbeat and takeover

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (with the give-up rule of § 8) | 2026-11-08 | 2026-11-09 | will.w.hui |

Card: task `A00-letter120`. Written from the approver's discussion of 2026-11-08; every point below was agreed there. On 2026-11-09 the approver added that a taken-over session gives the task up (§ 8) and accepted the record (see [Decided](#decided)).

## In plain words

Several sessions may now take ready tasks at the same time. A session can vanish in the middle of a task: its quota runs out, the agent crashes, the computer sleeps. Today such a task stays **in progress**, held by a session that will never come back, and nobody else may touch it.

**Proposal:** each session that holds a task sends a small sign of life, a **heartbeat**, at least every 10 minutes. A held task with no heartbeat for 30 minutes (and no busy window still running) counts as **lost**. Any other session may then claim it: that is a **takeover**. It continues on the same working branch, and the session it took over from is told so if it ever comes back.

| Point | Proposed |
|---|---|
| What counts as a heartbeat | Only one that has been **pushed** (shared). Every session is treated as if it ran on another teammate's computer; same computer or not makes no difference. |
| Where it is written | In the task's own file (`tasks/<id>/task.json`), on the taskbag's main branch: one field, the last heartbeat. **No event**, so burndown and timeline are unchanged (R7). |
| How often / when lost | Every 10 minutes; lost after 30 minutes without one (and no busy window). Both numbers are project settings. |
| Long quiet work | `alm beat <id> --busy <duration>`: the agent says how long it will be quiet; no upper limit; `alm next` shows it to everyone. |
| Fewer commits | Read-only commands write no heartbeat; commands that change a task write one with the change; `alm beat` writes nothing if the last one is under 5 minutes old; `alm beat --as <session>` with no ID beats for every task that session holds. |
| Takeover | Claiming a lost task takes it over: the former holder is recorded, work goes on on the original working branch, and `alm` reminds about uncommitted changes in the old worktree. Two sessions taking over at once: the Git push decides who wins. |
| Chains | The session that took over beats like any other and can itself be taken over when lost. |
| Coming back | Every `alm` command accepts `--as <session>`. With it, `alm` first checks whether a task this session held has been taken over; if so it tells the session and refuses its further changes to that task. The old session **gives the task up**: it stops work on it and goes on with other work. |
| Who checks "lost" | `alm` itself. This is a check made by `alm`, which AGENTS.md says needs the approver's approval: the approver agreed to this one in the discussion of 2026-11-08. |
| Not done | No reminder after a task is lost several times (not chosen in the discussion). |

Out of scope: questions that wait on the approver are handled as before in quiet mode (`alm wait`); a waiting task is not held by a working session and gets no heartbeat.

## Context

- A task has one `holder` ([FILE-FORMAT](../product/FILE-FORMAT.md)). Holder rules ([CODING_RULES § Holder rules](../workflow/CODING_RULES.md)): `label`, `wait`, `done` and `release` are refused (`task_held`) on a task someone else holds. Nothing today tells a holder that is working from one that is gone.
- Sessions already share their claims by pushing the taskbag: "until your claim is shared, other sessions do not see it" (AGENTS.md § Tasks: Allomos). A heartbeat follows the same path.
- R7: burndown, timeline, "recently finished" and the session list are computed from recorded events. A heartbeat every 10 minutes per task would flood that history, so it must not be an event.
- [0021](./0021-audit-trail.md) signs events and fingerprints the whole taskbag in checkpoints. The heartbeat field is ordinary content: a checkpoint fixes whatever value it had at that time; it is not signed on its own.

## Proposal in detail

### 1. A heartbeat counts only once pushed

Every session is treated as if it ran on another computer of the team. A heartbeat written but not pushed is invisible to the others, so it does not count; `alm` cannot see pushes (it never runs Git, [0017](./0017-agents-run-git-not-alm.md)), so, as with a claim, `alm beat` prints the Git commands and the agent pushes. Other sessions judge "lost" from what they pulled.

*Not chosen:* treating sessions on the same computer differently (for example by checking whether their process still runs). It would need a second way of judging, would not help a teammate on another computer, and a session can be alive as a process yet stuck.

### 2. One field in the task's own file, no event

`task.json` gains one field for the last heartbeat (its time, and the busy window when one was declared; exact names and the format version are left to the implementation card). Writing it records **no event**: burndown, timeline and history do not change (R7). The field lives on the taskbag's main branch, the same branch claims are shared on.

*Not chosen:*
- a heartbeat event in `events/`: floods the history and the burndown's inputs;
- a separate branch or file for heartbeats: a second place to pull, easy to forget, and another place where "who holds what" could disagree.

### 3. 10 minutes, 30 minutes, both project settings

A holding session beats at least every 10 minutes. A held task (`in_progress`) whose last heartbeat is more than 30 minutes old, with no busy window still running, is **lost**. Both numbers are project settings (`alm` and the app's project settings), with these defaults.

*Not chosen:* fixed numbers (projects and agents differ in how long a step takes); a shorter limit (more takeovers of tasks that are only slow).

### 4. Busy windows: `alm beat <id> --busy <duration>`

Before a long quiet step (a long build, a test run), the agent declares how long it will be quiet. The agent judges the length; there is **no upper limit**. While the window runs the task is not lost; after it ends, the usual 30 minutes count from its end. `alm next` shows the window to everyone ("busy until 14:30").

*Not chosen:* an upper limit (the agent knows its work best; a capped window would make long steps look lost); no busy windows (forces beats from inside long steps, which an agent often cannot do).

### 5. Fewer commits

- Read-only commands (`alm next`, `list`, `show`) write no heartbeat: reading must not create commits.
- Commands that change a task (`label`, `note`, `handoff`, …) write the heartbeat along with the change: no extra commit.
- `alm beat` writes nothing when the last heartbeat is less than 5 minutes old, and says so.
- `alm beat --as <session>` with no task ID beats for every task that session holds, in one change.

*Not chosen:* beating on read-only commands (a commit for every look); one commit per task when a session holds several.

### 6. Takeover

- Claiming a lost task **is** the takeover; no separate command. `alm claim` on a lost task says that it is lost and since when, records the former holder (in the claim's event), and makes the new session the holder.
- The new session continues on the task's **original working branch**: the claim prints it and the commands to start a new worktree from it.
- `alm` reminds that the old worktree may still hold uncommitted changes, and where it is when known; it does not look inside it (`alm` tells, the agent checks).
- Two sessions taking over at once: whichever push lands first wins; the other gets a Git conflict on pull, as with any two claims today.
- Claiming a task that is held and **not** lost is still refused, as today, unless `--take` is passed.
- **Amended 2026-11-11** (approver, task `A01-rocket260`): a person may take over any held task that is not finished from the app (**Take over…** in the task pane), lost or not, for example to go on with another agent or account. The new session claims it with `--take`, and its request says the person asked. `--take` is used only when the holder has stopped or a person asks for the takeover; an agent on its own still takes over only lost tasks. A holder that is not lost learns of the takeover at its next `alm` command (§ 8), up to about one heartbeat interval later, and until then both sessions may commit and push on the working branch, so the app warns before it opens anything.

*Not chosen:* a new branch for the new session (loses the old work or needs a merge of two branches of one task); asking the approver before each takeover (a lost task would then wait for a person, against quiet mode); `alm` itself resolving a race (it never runs Git).

### 7. Takeover chains

The session that took over is an ordinary holder: it beats, and when it is lost it can be taken over in turn. The former holders stay in the task's history in order.

### 8. A session coming back: `--as <session>` on every command

Every `alm` command accepts `--as <session>`. With it, before anything else, `alm` checks whether a task this session held has since been taken over. If so, it says so (by whom, when) and refuses the session's further changes to that task (`task_held`, with the takeover named). The session may still read the task, but it **gives the task up** (approver, 2026-11-09):

- it stops work on that task at once: no more commits or pushes to the task's working branch, and no attempt to claim it back;
- it leaves its worktree as it is (any uncommitted changes stay there for the new holder to look at, as § 6 says), and tells the person in its own conversation that the task was taken over and by whom;
- it goes on with other work: another ready task (`alm next`), or it ends.

`alm`'s message says this in so many words ("this task was taken over by … at …; stop work on it").

*Not chosen:* letting the old session take the task back automatically, or finish the step it was in (two sessions would then work one branch).

### 9. "Lost" is a check `alm` makes, approved

AGENTS.md says `alm` tells and the agent checks, and that a check of `alm`'s own needs the approver's approval. Judging "lost" (time since the last heartbeat against the setting) and refusing a taken-over session are such checks. The approver agreed to them in the discussion of 2026-11-08; this record writes that down.

### 10. Not done

No reminder or escalation when a task is lost several times in a row (discussed and not chosen). If it becomes a problem, it is a new decision.

## Consequences

- **Format:** `task.json` gains the heartbeat field; the project file gains the two settings. This changes the taskbag's shape, so the format version goes up through the usual guided upgrade (the exact number is set by the implementation card).
- **Commands:** new `alm beat`; `--as` on every command; `alm claim` takes over lost tasks; task-changing commands write the heartbeat; `alm next` / `list` / `show` show the last heartbeat, busy windows and lost tasks (and `--json` gains them).
- **Contract text:** AGENTS.md's Tasks section (`alm brief`) tells agents to beat and push every 10 minutes, to declare busy windows, how a takeover works, and that a session whose task was taken over gives it up; CODING_RULES § Holder rules gains "a lost task may be claimed".
- **App:** shows lost tasks and busy windows; the two numbers in project settings; texts in en, zh-CN and ja.
- **History:** no new events for heartbeats; takeovers are claim events with the former holder; burndown and timeline unchanged.
- **Commits:** about one taskbag commit per holding session every 10 minutes when it changes nothing else.

## Decided

2026-11-09, will.w.hui: **Accepted**, with one addition made that day: a session whose task was taken over gives the task up (§ 8). The agent's three routine choices (English and number 0022; field names left to the implementation card; the two implied points in § 6 and § 8) were reviewed and kept.

2026-11-11, will.w.hui: amended § 6: a person may take over any held task from the app (task `A01-rocket260`).
