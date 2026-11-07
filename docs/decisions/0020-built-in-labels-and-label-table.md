# 0020 — Three built-in labels, and a label table per project

| Status | Raised | Decided | Approver |
|---|---|---|---|
| **Proposed** | 2026-11-07 | — | will.w.hui |

Card: task `A01-meadow698`. Raised by the approver in chat on 2026-11-07, while reviewing the "Needs you" preset of the Show filter (task `A01-wigwam308`).

## In plain words

Today every project defines its own **label ladder**: an ordered list of progress steps in `project.json`, and each task carries one of them as its label. This project's ladder is drafted → implemented → locally verified → deployed and verified → human verified.

Looking at this project's 316 tasks, only three of those steps carry real information:

- **locally verified**: built and tested on this computer;
- **deployed and verified**: live, and checked after deploying (mostly releases);
- **human verified**: a person checked it by hand (the human check).

"Drafted" was never used, and "implemented" was used where "locally verified" was meant (most of its tasks say their tests passed).

**Proposal:**

1. Allomos itself defines these **three built-in labels**, always in this order. A project no longer chooses its own ladder.
2. Each project keeps a **label table**: every label the project knows, with a short meaning. The three built-in labels are always in it. When a label not yet in the table is found (an agent sets one, or a taskbag is moved in), it is **added** to the table, and `alm` says so. An entry is **never removed automatically** just because no task uses it any more. This keeps the way open for projects to define their own labels later.
3. **AI decisions waiting for review become a label too** ("AI decided"), next to the progress label. So a task can carry **several labels**.
4. The taskbag format moves to `allomos/7`. Existing ladders become label tables: nothing a task carries is lost.

## Context

- The ladder lives in `project.json` as `labels` (an ordered list; default `implemented`, `locally verified`, `deployed and verified`, `human verified`), see [FILE-FORMAT](../product/FILE-FORMAT.md). A task's `label` must be a step on it; `alm label` refuses any other (`crates/core/src/ops.rs`).
- The top step is the **human check**: `alm done` without `--without-human-check` needs it, and setting it on a done task clears `without_human_check` (as `alm checked` does).
- The Show filter offers each step with **Yes** ("reached this step or higher") and **No**; it translates the default steps and shows other names as written (`apps/web/src/stopFlow.ts`).
- `alm labels "<a,b,…>"` sets the ladder (refused if a task's label would be left off) and `alm rename label` renames a step (`alm guide customize`).
- AGENTS.md rule 4 names this project's ladder.

### What this project's tasks actually carry (2026-11-07, 316 tasks)

| State | no label | implemented | locally verified | deployed and verified | human verified |
|---|---|---|---|---|---|
| backlog | 22 | | | | |
| in progress | 1 | | | | |
| waiting | 1 | | | | |
| done | 7 | 17 | 105 | 26 | 116 |
| dropped | 17 | 2 | 2 | | |

No task is `ready` or `blocked` right now; no task carries `drafted`. Of the done tasks, 144 are **to check** (finished without the human check): 105 at locally verified, 20 at deployed and verified, 17 at implemented, 2 with no label.

## Proposal in detail

1. **Built-in labels.** `locally verified` < `deployed and verified` < `human verified`, fixed by Allomos, translated in every interface language. `human verified` is the human check everywhere (`alm done`, `alm checked`, "to check", the filter), with no lookup of a project's top step.
2. **The label table** replaces the ladder in `project.json`: one entry per label, each with its name, an optional one-line meaning, and whether it is built in. Order matters only among the built-in three; other labels have no place in that order (the filter shows them with a plain Yes / No).
3. **Finding new labels.** When `alm label` (or the app) sets a label that is not in the table, or a move-in brings one, it is added to the table and `alm` says so in its output ("Added the label 'x' to the label table; if that was a typo, …"). This replaces today's refusal. Following "`alm` tells; the agent checks", `alm` does not judge whether the new label is wanted: the agent does, and can rename it.
4. **Never removed automatically.** An unused entry stays. A person or agent can remove one on purpose (for example `alm labels remove "<name>"`), refused while a task still carries it; the three built-in labels cannot be removed.
5. **AI decisions become a label** (approver, 2026-11-07: "i want the ai decide mark be move to the label field too"). See [AI decisions as a label](#ai-decisions-as-a-label) below. Because a task keeps its progress label at the same time, this needs **several labels per task**.
6. **Format `allomos/6` → `allomos/7`.** Each ladder step becomes a table entry; every task keeps its label. Older Allomos builds see the newer format and ask to be updated.
7. **Where the table lives: `project.json`, not `project-notes.md`.** `alm` and the app must read and update it reliably; `project-notes.md` is free text that agents edit by hand. People see the table in `alm show`-style output and in the app's settings.

## AI decisions as a label

**What an AI decision is today.** Inside a task, an agent makes routine choices on its own instead of asking: a name, a file layout, how something is built within the task, a small detail with an obvious default (anything cheap to undo). AGENTS.md asks it to record each one with `alm ai-decided <id> "<what and why>"`, so the approver can look at them later and overturn any. The task then shows a badge in the app and is listed by `alm human` under **To review: AI decisions**, until the approver runs `alm reviewed <id>` (or says so in the app).

**Where it is stored today.** Not in `task.json`: each decision is an event in the taskbag's history (`events/`, a change of the field `ai_decision`), and `alm reviewed` adds an event that marks all earlier ones reviewed. Whether a task has decisions to review is worked out from the history. That is why no state or label tick in the Show filter can find these tasks (77 here today, 13 of them already `human verified`).

**Proposal.** A built-in label **AI decided** (shown translated): `alm ai-decided` adds it to the task, `alm reviewed` removes it. The decisions themselves stay in the history, as now; the label only says "there are decisions to review". It is not a progress step, so it sits outside the order of the three progress labels.

**What it needs.**

- **Several labels per task:** `label` (one name) becomes `labels` (a list) in `task.json`, still with at most one of the three progress labels. The filter's "this step or higher" applies to the progress label only; other labels are plain Yes / No.
- **Existing tasks:** the format change gives the label to every task that has unreviewed decisions today (worked out once from the history).
- **The same could later hold for "to check"** (`without_human_check`): it too is a mark beside the progress label. Not proposed here, since it also carries the reason and the check steps; the approver may ask for it.

## What happens to the old steps

| Old step | Option A (recommended): keep | Option B: drop | Option C: raise |
|---|---|---|---|
| `drafted` (unused here) | stays in the table, unused | removed | — |
| `implemented` (19 tasks here) | stays in the table as a custom label; the 19 tasks keep it | the 19 tasks lose their label | the 19 tasks become `locally verified` |
| other projects' own step names | stay as custom labels | — | — |
| `owner verified` (older taskbags) | becomes `human verified`, as today | | |

**Recommended: A.** The format change is then purely mechanical: no task's progress claim is lowered or raised, and nothing needs an agent's judgement. Cleaning up afterwards (for example `alm rename label "implemented" "locally verified"` for tasks whose notes prove their tests passed, or removing `drafted`) is an ordinary change the approver can ask for at any time. B loses information; C raises 19 progress claims without evidence that each one holds.

## Alternatives considered

| Option | What it is | Why not (proposed) |
|---|---|---|
| **Built-in three + table** (proposed) | as above | — |
| Keep per-project ladders | today's model; only remove `drafted` and fold `implemented` here | Keeps the cost of a configurable ladder (lookups, translations only for default names, `alm labels`) for a choice the data shows nobody uses. |
| Fixed three, no table | built-in labels only; anything else refused | Simplest, but closes the door on project labels, which the approver wants to keep open. |
| Merge labels into states | `done` split by how it was finished | Labels appear almost only on done tasks here, but a task can wait or be blocked part-way ("waiting, locally verified"); merging changes every state and the run loop. Left out of this decision. |
| Table in `project-notes.md` | a Markdown table agents keep | Free text: `alm` and the app cannot update or read it reliably. |

## Consequences

- **Format:** `allomos/7`; [FILE-FORMAT](../product/FILE-FORMAT.md) describes the table and drops "must be a step on the ladder" in favour of "is added to the table".
- **`alm`:** `alm label` adds unknown labels instead of refusing them; `alm labels` becomes the table's command (list, add with a meaning, remove an unused one); `alm rename label` stays; the human check is always `human verified`. `alm guide terms` and `alm guide customize` are rewritten for the table.
- **App:** the Show filter lists the built-in three first (with "or higher"), then the table's other labels; the settings show the table.
- **AGENTS.md:** rule 4 names the three built-in labels instead of this project's ladder; running sessions are told (shared rule change).
- **Filter presets (`A01-wigwam308`):** "Needs you" can rely on `human verified` being fixed and tick the **AI decided** label; whether "to check" gets a tick of its own follows open point 4.
- **Several labels per task:** `task.json`'s `label` becomes `labels`; `alm label` adds or replaces the progress label and keeps the others; the task list and pane show every label; the history records the list.
- **AI decisions:** the badge and `alm human`'s To review read the **AI decided** label instead of working it out from the history; `alm ai-decided` and `alm reviewed` set and clear it.
- **Not changed:** states, `without_human_check`, where the decisions themselves are kept (the history).

## Open for the approver

1. Accept the three built-in labels and the label table in `project.json`?
2. Old steps: option A (keep as custom labels), B (drop) or C (raise)?
3. Unknown labels: add them and say so (proposed), or keep refusing until someone adds them to the table?
4. AI decisions as the built-in label **AI decided**, with several labels per task: accept? And should "to check" become a label the same way?
