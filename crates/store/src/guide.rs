//! What `alm guide` tells an AI agent (Task-A05-02). The short brief in a project's `AGENTS.md`
//! ([`crate::agent_section`]) names these topics; an agent reads them from `alm` when it needs
//! them, so the details always match the installed `alm`, not a copy that went stale. English:
//! agents read it, the app does not show it.

/// The topics, in the order `alm guide` prints them.
pub const GUIDE_TOPICS: [&str; 5] = ["terms", "customize", "tasks", "commands", "workflow"];

const TERMS: &str = "# Allomos terms

Allomos relies on these terms. Use them as defined here, in commands and when you talk
about the project's tasks.

- **Taskbag**: the folder holding the project's tasks (`.allomos/taskbag/`): readable JSON and
  Markdown files tracked by Git. Change it only with `alm`.
- **Project folder**: the folder with the project's code and its AGENTS.md / CLAUDE.md. Usually
  the folder holding `.allomos/`; `project_folder` in the taskbag's project.json records it.
- **Task**: one observable outcome: what a person or agent can do when it is done. It has an
  ID, a title, an area, a state, and optionally a label, an iteration, a holder, what it waits on
  (and who), the tasks that block it, its next dependency, a handoff note and a description
  (description.md).
- **Area**: *what* part of the product a task belongs to. Every task has exactly one.
- **Iteration**: *when* a task is planned: a time-box that ends in a release. Optional; a task
  without one is unplanned.
- **State**: where a task is in its life: backlog, ready (approved to start), in progress (held
  by someone), waiting (a person or an outside event must act: its waiting-on says what, its
  optional who says who; no who means the approver), blocked (another task in this taskbag must
  finish first: its blocked-by lists them), done, or dropped (will not be done). A task blocked by
  a task and also waiting on a person is blocked; the person part goes in its next dependency.
- **Label**: the highest step of the project's progress **ladder** the task has proven, for
  example implemented → locally verified → human verified. It is the only progress claim; test
  counts support a label but are never one.
- **Human check**: a person tries the finished task by hand. Its step is the ladder's top label.
  A task finished without it (in an automatic or loop run) carries why, and the steps for the
  person who checks it; it is **to check** until someone marks it checked.
- **Approver**: the person who decides and approves (cards, plans, external effects). A waiting
  task without a who waits for the approver.
- **Holder**: the person or agent session working a task (`--as <name>`). One holder at a time.
- **Next dependency**: the one thing needed before the task's next label.
- **Handoff note**: what the next holder needs to know, left when a task is released.
- **Task language**: the language new and changed tasks are written in (titles, descriptions,
  notes). `alm task-language` tells you which; by default it follows the language the person
  uses Allomos in.
";

const CUSTOMIZE: &str = "# What a project may change, and what is fixed

| Term | This project may change | How |
|---|---|---|
| Areas | which areas exist, their IDs and names | `alm add area <id> \"<name>\"`, `alm rename area`, `alm delete area` (only an empty one) |
| Iterations | which exist, their names, releases and dates; which task is in which | `alm add iteration`, `alm change iteration`, `alm delete iteration` (only an empty one), `alm plan` |
| Label ladder | its steps and their order | `alm labels \"<a,b,…>\"` (refused if a task's label would be left off); `alm rename label \"<old>\" \"<new>\"` renames a step and the tasks that carry it |
| Task IDs and titles | any ID of letters, digits, `-` or `_` (for example A01-02) | `alm add task <id> --area <id> --title \"…\"` |
| Description | free Markdown in description.md | written with `alm add task --description` |
| Task language | `ui` (follow the Allomos app's language, the default) or a language code | `alm task-language <code or ui>`; only when the person asks |

Fixed for every project:

- **The seven states**: backlog, ready, in progress, waiting, blocked, done, dropped.
- **The words \"area\" and \"iteration\"**: a project cannot rename them yet (planned: requirement
  R90). Use these words with `alm`, even if the team says \"workstream\" or \"sprint\".
- **The file format**: never edit the taskbag's files by hand; `alm` checks every change.

Read the project's current areas, iterations and tasks with `alm list` (`--by iteration`), and its
ladder in the taskbag's project.json.
";

const TASKS: &str = "# How to add a task

Read this before you add a task, so it fits this project.

0. **Language.** Run `alm task-language` and write the title and description in that language.
   IDs, commands, code and file names stay as they are.
1. **Look first.** `alm list` shows the areas and the task IDs already used; `alm show <id>` shows
   how an existing task is written. Follow what you see.
2. **Area.** Use an area that exists. Add a new one (`alm add area <id> \"<name>\"`) only when the
   person asks for it or agrees to it.
3. **ID.** Leave it out: `alm add task --area <area> --title \"…\"` takes the next free number in
   the area (A01-04 after A01-03). Give an ID only to follow a pattern the project already uses,
   for example a child task `A01-03-01` under `A01-03`.
4. **Title.** One observable outcome: what a person can do or see when it is done (\"A customer
   orders a cake for a pick-up day\"), not a technical step (\"add an orders table\").
5. **Description** (`--description`, Markdown), in three short parts:
   - **Outcome.** One or two plain sentences.
   - **Done means.** The checks or demonstrations that prove it.
   - **Notes.** Anything the next person needs: links, open questions, who must decide.
6. **State.** A new task starts in backlog. Do not make it ready yourself: the person approves it
   (`alm ready <id>`). Plan it in an iteration (`alm plan <id> <iteration>`) only when asked.
7. **One outcome per task.** If it is really two outcomes, add two tasks.

When the project shares its tasks through Git, run the Git commands `alm add task` prints, so
others see the new task.
";

const COMMANDS: &str = "# alm commands

Every command takes `--taskbag <path>` (or ALLOMOS_TASKBAG; default: the nearest
`.allomos/taskbag/` above the current folder), `--as <name>` (or ALLOMOS_HOLDER), `--json`
(one JSON object on standard output). Exit codes: 0 done, 1 refused by a rule, 2 wrong usage,
3 taskbag busy or not writable.

`alm` never runs or reads Git. After a change it prints \"Commit and push this change:\" and the
exact `git add`, `git commit` and `git push` commands naming only the files that change wrote
(`--json`: `changed_files` and `git_commands`); run them yourself and leave other changes as they
are. `alm claim` asks you to push at once, before you start the work. Changes the person made in
the Allomos app and not shared yet come with their own commands, to commit as the person's.
`alm init` prints none: commit the new taskbag yourself.

See:
- `alm next [--iteration <id>]`: every ready task no one holds (in that iteration), each with its
  latest handoff, and the rule for choosing one; then the blocked tasks whose blockers are all
  done. `alm` chooses and moves nothing; you do.
- `alm list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area|iteration]`
  (`--state blocked`, `--state waiting`, …);
  `--without-human-check`: only tasks finished without the human check, not checked since.
- `alm human [--iteration <id>] [--area <id>] [--steps]`: what needs a person: **To check**
  (finished without the human check, each with why; `--steps` adds how to check it) and **To
  answer** (waiting on a person, each with what). Backlog tasks are not listed.
- `alm show <id>`: one task in full, with its description.
- `alm history <id>`: every recorded change to the task, newest first: when, who, what changed.

Work a task:
- `alm claim <id>`: you hold it; state in progress. `--take` only when its holder has stopped.
- `alm label <id> \"<label>\"`: a step of the ladder.
- `alm needs <id> \"<what>\"`: its next dependency (no text clears it).
- `alm note <id> \"<what the next one needs>\"`: its handoff note (no text clears it).
- `alm wait <id> --on \"<what>\" [--who \"<who>\"]`: state waiting: a person (or an outside
  event) must act; without `--who`, the approver.
- `alm block <id> --by <task-id> [<task-id>…]`: state blocked: those tasks must finish first.
  `alm list`, `show` and `next` show each blocker's state; a dropped blocker is flagged and never
  counts as done. When all are done, `alm next` says so, and moves nothing: check, then
  `alm unblock <id>` (ready, or in progress when someone holds it).
- `alm release <id> --note \"<what the next one needs>\"`: stop short of done.
- `alm done <id> [--without-human-check \"<why, which run>\" --steps <file.md>]`: finished (see
  `alm guide workflow`, step 7); it ends with the rule for checking the handoffs that name the
  task; follow it. The steps are Markdown for the person who checks it: the test folder, what to
  open, what to click, what they should see. On a task already done, it only records its human
  check again (the old `--without-owner` still works).
- `alm checked <id>`: a person has checked the finished task: it gets the top label and leaves
  To check; the note and steps stay in its history. Run it only when the person says so.
  `alm drop <id>`; `alm ready <id>` (an unheld backlog, waiting or blocked task).

Plan:
- `alm add area [<id>] \"<name>\"`, `alm add iteration [<id>] \"<name>\" [--release …] [--start …]
  [--end …]`: without an ID, the next free one, like A08 or I16.
- `alm rename area <id> \"<name>\"`; `alm change iteration <id> [--name …] [--release …]
  [--start …] [--end …]` (an empty value clears a release or date). IDs stay.
- `alm delete area <id>`, `alm delete iteration <id>`: refused while any task is in it, done and
  dropped ones too (plan them elsewhere first).
- `alm add task [<id>] --area <id> --title \"<title>\" [--description \"<markdown>\"]`: without
  an ID, the next free one in the area. Read `alm guide tasks` first.
- `alm title <id> \"<title>\"`; `alm describe <id> --text \"<markdown>\"` or
  `alm describe <id> --from <file.md>`: a new title or description (`--text \"\"` removes the
  description). Same holder rule as `alm label`; each is recorded in `alm history`.
- `alm plan <id> [<iteration>]`; `alm labels \"<a,b,…>\"`; `alm rename label \"<old>\" \"<new>\"`
  (a step of the ladder, and every task that carries it)
- `alm task-language`: the language to write new and changed tasks in (run it before
  `alm add task`, `alm title` and `alm describe`);
  `alm task-language <code|ui>` sets it.

Handoffs (the note a session leaves so whoever continues does not start blind):
- `alm handoff-rules`: when to write one (quota and context gates, with any override for you);
  `--quota <n> --context <n> [--write-when \"<text>\"] [--agent <name>] [--clear]` change them.
- `alm handoff --tasks <id,id> --as <your session name>`: this session's handoff file, made from
  the template the first time (later calls update the same file); fill it in, or pass
  `--from <file.md>`. It prints the file's path.
- `alm handoffs [--task <id>] [--latest]`: handoffs, newest first. `alm next` and `alm show <id>`
  show a task's latest one.
- `alm handoffs --check [--task <id>]`: the rule for checking the handoffs yourself, one by one,
  and removing those whose tasks are all done or dropped.
- `alm handoff --remove <file>`: remove a handoff whose tasks are all finished (Git keeps it).
- `alm handoff-template [--set <file.md>]`: show or replace the project's template.

Ask the person:
- `alm confirm --title \"<heading>\" --plan <file.md>`: shows your plan in the Allomos app and
  waits; exit 0 means go ahead, anything else means change nothing.

About Allomos:
- `alm brief`: the section for AGENTS.md. `alm guide [terms|customize|tasks|commands|workflow]`:
  this guide.
- `alm init`: create a taskbag (only when there is none), in `.allomos/taskbag/` of the current
  folder, or where `--taskbag` says.
- `alm migrate`: when `alm` says the taskbag is in an older format, the steps for updating it;
  follow them. `alm migrate --finish` ends the update once `alm list --json` reports no issues.
";

const WORKFLOW: &str = "# How to work the tasks

1. Start with `alm next` (or `alm list`). Choose by its rule: the task the person named, if any;
   otherwise one from the earliest iteration whose next dependency is met. Read its handoff
   first (`alm show <id>`).
2. Hold one task at a time. Pull first, then `alm claim <id> --as <your session name>`, and run
   the Git commands it prints right away: until your claim is pushed, other sessions do not see
   it and may take the task too. A task someone else holds is not yours to change.
3. As you prove each step, `alm label <id> \"<label>\"`, and keep its next dependency current
   with `alm needs`.
4. Stopped: when a person (or an outside event) must act, `alm wait <id> --on \"<what>\"
   [--who \"<who>\"]`; when another task must finish first, `alm block <id> --by <task-id>` (if
   a person must act too, say so with `alm needs`). Blocked and waiting tasks are never in
   `alm next`'s list: they are skipped by their state. When `alm next` says all of a task's
   blockers are done, check it, then `alm unblock <id>`.
5. Write your handoff when `alm handoff-rules` says (near the quota or context gate, and after
   every meaningful step): `alm handoff --tasks <id> --as <your session name>`, then fill in the
   file it names, in plain words first, for any reader: you do not know who takes over, an agent
   or a person. Evidence goes on the task's card, not in the handoff.
6. Stopping before it is done (for example your session ends): `alm release <id> --note \"<what
   the next one needs>\"`.
7. Finished: only once the task has the top label of the ladder (a person has checked it: the
   human check), then `alm done <id>`. The one exception: in an automatic or loop run the
   person set up to go on without them (for example a card run or `/loop`), you may finish a task
   below the top label with
   `alm done <id> --without-human-check \"<why, which run>\" --steps <file.md>`; say so in
   your handoff. Write the steps so a person can pick it up and test
   it: the test folder, what to open, what to click, what they should see. The task keeps its
   label, the note and the steps, so a person finds it with `alm human`; once they have checked
   it, `alm checked <id>`.

Rules:
- Never edit the taskbag's files by hand, and never invent progress, verification or approvals.
  The one exception: fill in your own handoff file, the one `alm handoff` names.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- Before a change the person must agree to, show your plan with `alm confirm` and change nothing
  unless it exits 0.
- After every `alm` change, run the Git commands it prints: they name only that change's files.
  Your other work is committed as usual.
";

/// One topic of the guide, or all of them when `topic` is `None`. `None` for an unknown topic.
pub fn guide(topic: Option<&str>) -> Option<String> {
    let text = |t: &str| match t {
        "terms" => Some(TERMS),
        "customize" => Some(CUSTOMIZE),
        "tasks" => Some(TASKS),
        "commands" => Some(COMMANDS),
        "workflow" => Some(WORKFLOW),
        _ => None,
    };
    match topic {
        Some(t) => text(t).map(String::from),
        None => Some(
            GUIDE_TOPICS
                .iter()
                .filter_map(|t| text(t))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_is_there_and_the_brief_names_them() {
        for topic in GUIDE_TOPICS {
            assert!(
                guide(Some(topic)).is_some_and(|t| t.starts_with("# ")),
                "{topic}"
            );
            assert!(
                crate::agent_section().contains(&format!("alm guide {topic}")),
                "{topic}"
            );
        }
        assert_eq!(guide(Some("nope")), None);
        let all = guide(None).unwrap();
        assert!(all.contains("# Allomos terms") && all.contains("# How to work the tasks"));
        // The fixed parts are said plainly.
        assert!(
            guide(Some("customize"))
                .unwrap()
                .contains("The seven states")
        );
        for text in [guide(Some("commands")).unwrap(), crate::agent_section()] {
            assert!(text.contains("alm block <id> --by"), "{text}");
            assert!(text.contains("alm unblock <id>"), "{text}");
            assert!(text.contains("--who"), "{text}");
        }
        assert!(
            guide(Some("tasks"))
                .unwrap()
                .contains("Do not make it ready yourself")
        );
        assert!(guide(Some("tasks")).unwrap().contains("alm task-language"));
        assert!(crate::agent_section().contains("run `alm task-language`"));
        // The rule for finishing, with its automatic/loop exception (Task-A03-15).
        for text in [guide(Some("workflow")).unwrap(), crate::agent_section()] {
            assert!(text.contains("top label"), "{text}");
            assert!(text.contains("automatic or loop run"), "{text}");
            assert!(text.contains("--without-human-check"), "{text}");
            assert!(text.contains("--steps <file.md>"), "{text}");
            assert!(text.contains("alm checked <id>"), "{text}");
            assert!(text.contains("alm human"), "{text}");
            assert!(!text.contains("owner"), "{text}");
            assert!(text.contains("handoff"), "{text}");
        }
        assert!(crate::agent_section().contains("every ready task nobody holds"));
        // Decision 0017: no `--commit` / `--push`; pull before claiming, push the claim at once,
        // and run the Git commands each change prints.
        for text in [guide(None).unwrap(), crate::agent_section()] {
            assert!(!text.contains("--push"), "{text}");
            assert!(!text.contains("--commit"), "{text}");
            assert!(
                text.contains("Pull first") || text.contains("pull first"),
                "{text}"
            );
            assert!(text.contains("until your claim is pushed"), "{text}");
            assert!(text.contains("Git commands"), "{text}");
        }
    }
}
