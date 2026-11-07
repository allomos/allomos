# 0020 — Built-in labels, several labels per task, and a label table per project

| Status | Raised | Decided | Approver |
|---|---|---|---|
| **Accepted** | 2026-11-07 | 2026-11-07 | will.w.hui |

Card: task `A01-meadow698`. Raised by the approver in chat on 2026-11-07, while reviewing the "Needs you" preset of the Show filter (task `A01-wigwam308`); accepted in the same chat (see [Decided](#decided)).

## In plain words

Today every project defines its own **label ladder**: an ordered list of progress steps in `project.json`, and each task carries **one** of them. This project's ladder is drafted → implemented → locally verified → deployed and verified → human verified. Two other facts a person needs (a task finished without the human check, and AI decisions waiting for review) are kept elsewhere, so the Show filter cannot find them.

**Decided:**

1. Allomos has **five built-in labels**: locally verified, deployed and verified, human verified, **to check** and **AI decided**.
2. A task can carry **several labels**. **All labels are plain** (approver: "treat them all as stateless"): no kinds, no order, no "one of these at a time". Each label simply says one thing about the task, and a task has it or not.
3. Each project has a **label table** in `project.json`: every label it knows, with a short meaning. The built-in labels are always in it. A label not yet in the table is **added when it is first found**, and `alm` says so. An entry is **never removed automatically** because no task uses it.
4. The taskbag format moves to `allomos/7`. Existing ladder steps that are not built in stay as **custom labels**; no task loses a label.

## The label values

| Label | Built in | Says | en | zh-CN | ja | Added by | Removed by |
|---|---|---|---|---|---|---|---|
| `locally verified` | yes | built and tested on this computer | locally verified | 本地已验证 | ローカルで検証済み | `alm label <id> "locally verified"` | removing it on purpose; `alm reopen` |
| `deployed and verified` | yes | live, and checked after deploying | deployed and verified | 已部署并验证 | デプロイして検証済み | `alm label` | removing it on purpose; `alm reopen` |
| `human verified` | yes | a person checked it by hand (the human check) | human verified | 人工已验证 | 人が確認済み | `alm label`, `alm checked`, the app's **Checked** | removing it on purpose; `alm reopen` |
| `to check` | yes | finished without the human check | to check | 待检查 | 要確認 | `alm done <id> --without-human-check "<why>" --steps <file>` | `alm checked` (which adds `human verified`); `alm reopen` |
| `AI decided` | yes | an agent decided things here that wait for review | AI decided | 待审阅 AI 决定 | レビュー待ちの AI の決定 | `alm ai-decided <id> "…"` | `alm reviewed <id>` |
| `drafted` | no (this project) | — | — | — | — | `alm label` | removing it on purpose |
| `implemented` | no (this project) | — | — | — | — | `alm label` | removing it on purpose |

- The interface names come from the app's existing texts (`filter.ladder.*`, `check.to_check`, `task.ai_decided`); the work may shorten them for the label chips.
- Custom labels are shown as written. In this project only `drafted` (no task) and `implemented` (19 tasks) are custom; other projects bring their own.
- **To check** keeps its reason and check steps in the task's fields (`without_human_check`, `human_check_steps`); the label is what lists and filters read.
- **AI decided** says "there are decisions to review"; the decisions themselves stay in the task's history, as now.

## Context (before this decision)

- The ladder lives in `project.json` as `labels` (default `implemented`, `locally verified`, `deployed and verified`, `human verified`), see [FILE-FORMAT](../product/FILE-FORMAT.md). A task's `label` must be a step on it; `alm label` refuses any other.
- The top step is the human check: `alm done` without `--without-human-check` needs it, and setting it on a done task clears `without_human_check` (as `alm checked` does).
- **To check** is a done task with `without_human_check` set (written by `alm done --without-human-check`); `alm human` lists it and the app shows "To check".
- **AI decisions**: inside a task an agent makes routine choices on its own (a name, a file layout, a small detail with an obvious default) and records each with `alm ai-decided`. They are events in the taskbag's history (field `ai_decision`); `alm reviewed` adds an event that marks them reviewed, and "has decisions to review" is worked out from the history. The app shows a badge and `alm human` lists the task under To review.
- The Show filter offers each ladder step with Yes ("this step or higher") and No; it cannot see "to check" or "AI decided".

### What this project's tasks carried (2026-11-07, 316 tasks)

| State | no label | implemented | locally verified | deployed and verified | human verified |
|---|---|---|---|---|---|
| backlog | 22 | | | | |
| in progress | 1 | | | | |
| waiting | 1 | | | | |
| done | 7 | 17 | 105 | 26 | 116 |
| dropped | 17 | 2 | 2 | | |

No task carried `drafted`. 144 done tasks were to check (105 locally verified, 20 deployed and verified, 17 implemented, 2 no label); 77 tasks had AI decisions to review, 13 of them already human verified.

## Decided

The approver's answers on 2026-11-07, in this session's chat:

1. Built-in labels and the table: "yes, but more than 3 build in labels right?" Yes: five, with "to check" and "AI decided".
2. Old steps: "ok" to keeping them as custom labels (option A below).
3. Unknown labels: "this." — `alm` adds them to the table and says so.
4. AI decisions as a label, several labels per task: "yes". To check as a label the same way: "yes".
5. Kinds (progress, mark, custom) and an order among labels: "no, i would like to treat them all as stateless". Every label is plain; nothing about a label is saved beyond its name, meaning and whether it is built in.

Earlier in the same chat: "only these ladders are useful: locally verified, deployed and verified, human verified … we only need to update the taskbag version and map these 3 ladder values"; "i also want to allow the labels to be customized in the future. so we need a label table … updated if new labels are found. it should not be auto deleted if a defined label in the table is not used"; "i want the ai decide mark be move to the label field too".

## In detail

1. **All labels are plain.** No label has a kind or a place in an order, and none excludes another: a task can carry `locally verified` and `deployed and verified` at once. The filter shows every label with a plain Yes ("has it") and No ("does not have it"); "this step or higher" goes away.
2. **Commands keep their meaning.** Some commands add or remove built-in labels as part of what they do (the table above): `alm done --without-human-check` adds `to check`; `alm checked` removes `to check` and adds `human verified`; `alm ai-decided` and `alm reviewed` add and remove `AI decided`; `alm reopen` clears the labels it clears today. `human verified` is the human check everywhere: `alm done` without `--without-human-check` needs it on the task.
3. **Several labels per task:** `task.json`'s `label` (one name) becomes `labels` (a list). `alm label <id> "<label>"` adds a label and keeps the others; removing one is on purpose (the exact commands are the implementing task's choice).
4. **The label table** replaces the ladder in `project.json`: one entry per label, with its name, an optional one-line meaning, and whether it is built in.
5. **New labels are added, not refused:** when `alm label` (or the app) sets a label that is not in the table, or a move-in brings one, it is added and `alm` says so ("Added the label 'x' to the label table; if that was a typo, …"). Following "`alm` tells; the agent checks", `alm` does not judge whether it is wanted.
6. **Never removed automatically:** an unused entry stays. Someone can remove one on purpose, refused while a task still carries it; built-in labels cannot be removed.
7. **Where the table lives: `project.json`**, not `project-notes.md`: `alm` and the app must read and update it reliably, and the project notes are free text agents edit by hand.
8. **Format `allomos/6` → `allomos/7`:** each ladder step becomes a table entry; each task's label moves into its list; `to check` is added to every done task with `without_human_check`, and `AI decided` to every task whose history has unreviewed decisions (worked out once). Older Allomos builds see the newer format and ask to be updated.

### What happens to the old steps

| Old step | **A: keep (decided)** | B: drop | C: raise |
|---|---|---|---|
| `drafted` (unused here) | stays in the table, unused | removed | — |
| `implemented` (19 tasks here) | stays as a custom label; the 19 tasks keep it | the tasks lose it | the tasks become `locally verified` |
| other projects' own steps | stay as custom labels | — | — |
| `owner verified` (older taskbags) | becomes `human verified`, as today | | |

A keeps the format change mechanical: no task gains or loses a claim. Cleaning up later (for example renaming `implemented` on tasks whose notes prove their tests passed) is an ordinary change.

## Alternatives considered

| Option | What it is | Why not |
|---|---|---|
| **Built-in labels + table, several per task** (decided) | as above | — |
| Keep per-project ladders | today's model, minus `drafted` | Keeps a configurable ladder nobody uses, and the filter still cannot find "to check" or "AI decided". |
| Kinds and an order (progress, mark, custom) | progress labels ordered and one at a time; marks set only by commands | Proposed first; the approver chose plain labels instead (Decided, 5). |
| Built-in labels only, no table | anything else refused | Closes the door on project labels, which the approver wants to keep open. |
| Merge labels into states | `done` split by how it was finished | A task can wait or be blocked part-way ("waiting, locally verified"); merging changes every state and the run loop. |
| Table in `project-notes.md` | a Markdown table agents keep | Free text: `alm` and the app cannot update or read it reliably. |

## Consequences

- **Format:** `allomos/7`; [FILE-FORMAT](../product/FILE-FORMAT.md) describes `labels` on tasks, the label table (name, meaning, built in), and which commands add or remove built-in labels.
- **`alm`:** `alm label` keeps other labels and adds unknown ones to the table; `alm labels` manages the table (list, add with a meaning, remove an unused one); `alm rename label` stays; `alm done --without-human-check`, `alm checked`, `alm ai-decided`, `alm reviewed` and `alm reopen` add and remove built-in labels; `alm human` reads `to check` and `AI decided`; `alm guide terms` and `alm guide customize` describe the table.
- **App:** the task list and pane show every label; the Show filter lists every label in the table with a plain Yes / No (built-in first); the AI-decision badge and "To check" read their labels; the settings show the table; texts in en, zh-CN and ja.
- **Filter presets (`A01-wigwam308`):** "Needs you" can tick the labels `to check` and `AI decided`. `alm human` also lists waiting tasks (To answer); today's filter joins its state and label ticks with "and", so taking in waiting tasks too is that task's question.
- **AGENTS.md:** rule 4 names the built-in labels instead of this project's ladder; running sessions are told (shared rule change).
- **Not changed:** the states; where the human-check reason and steps, and the AI decisions themselves, are kept.
