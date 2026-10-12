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
  ID, a title, an area, a state, and optionally a label, an iteration, a holder, what it waits on,
  its next dependency, a handoff note and a description (description.md).
- **Area**: *what* part of the product a task belongs to. Every task has exactly one.
- **Iteration**: *when* a task is planned: a time-box that ends in a release. Optional; a task
  without one is unplanned.
- **State**: where a task is in its life: backlog, ready (approved to start), in progress (held
  by someone), waiting (on something, said in its waiting-on), done, or dropped (will not be done).
- **Label**: the highest step of the project's progress **ladder** the task has proven, for
  example implemented → locally verified → owner verified. It is the only progress claim; test
  counts support a label but are never one.
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
| Areas | which areas exist, their IDs and names | `alm add area <id> \"<name>\"` |
| Iterations | which exist, their names, releases and dates; which task is in which | `alm add iteration`, `alm plan` |
| Label ladder | its steps and their order | `alm labels \"<a,b,…>\"` (refused if a task's label would be left off) |
| Task IDs and titles | any ID of letters, digits, `-` or `_` (for example A01-02) | `alm add task <id> --area <id> --title \"…\"` |
| Description | free Markdown in description.md | written with `alm add task --description` |
| Task language | `ui` (follow the Allomos app's language, the default) or a language code | `alm task-language <code or ui>`; only when the person asks |

Fixed for every project:

- **The six states**: backlog, ready, in progress, waiting, done, dropped.
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

Add it with `--push` when the project shares its tasks through Git, so others see it.
";

const COMMANDS: &str = "# alm commands

Every command takes `--taskbag <path>` (or ALLOMOS_TASKBAG; default: the nearest
`.allomos/taskbag/` above the current folder), `--as <name>` (or ALLOMOS_HOLDER), `--json`
(one JSON object on standard output), and for changes `--commit` or `--push` (commit only the
taskbag's files; push the current branch). Exit codes: 0 done, 1 refused by a rule, 2 wrong
usage, 3 taskbag busy or not writable.

See:
- `alm next`: the first ready task no one holds, with its handoff note.
- `alm list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area|iteration]`
- `alm show <id>`: one task in full, with its description.

Work a task:
- `alm claim <id>`: you hold it; state in progress. `--take` only when its holder has stopped.
- `alm label <id> \"<label>\"`: a step of the ladder.
- `alm needs <id> \"<what>\"`: its next dependency (no text clears it).
- `alm wait <id> --on \"<what>\"`: state waiting.
- `alm release <id> --note \"<what the next one needs>\"`: stop short of done.
- `alm done <id>`; `alm drop <id>`; `alm ready <id>` (an unheld backlog or waiting task).

Plan:
- `alm add area <id> \"<name>\"`, `alm add iteration <id> \"<name>\" [--release …] [--start …] [--end …]`
- `alm add task [<id>] --area <id> --title \"<title>\" [--description \"<markdown>\"]`: without
  an ID, the next free one in the area. Read `alm guide tasks` first.
- `alm plan <id> [<iteration>]`; `alm labels \"<a,b,…>\"`
- `alm task-language`: the language to write new and changed tasks in (run it first);
  `alm task-language <code|ui>` sets it.

Ask the person:
- `alm confirm --title \"<heading>\" --plan <file.md>`: shows your plan in the Allomos app and
  waits; exit 0 means go ahead, anything else means change nothing.

About Allomos:
- `alm brief`: the section for AGENTS.md. `alm guide [terms|customize|tasks|commands|workflow]`:
  this guide.
- `alm init`: create a taskbag (only when there is none).
";

const WORKFLOW: &str = "# How to work the tasks

1. Start with `alm next` (or `alm list`), and read the task's handoff note and description.
2. Hold one task at a time: `alm claim <id> --as <your session name> --push`. A task someone else
   holds is not yours to change.
3. As you prove each step, `alm label <id> \"<label>\" --push`, and keep its next dependency
   current with `alm needs`.
4. Blocked on a person or another task: `alm wait <id> --on \"<what>\" --push`.
5. Stopping before it is done (for example your session ends): `alm release <id> --note \"<what
   the next one needs>\" --push`.
6. Finished: the top label, then `alm done <id> --push`.

Rules:
- Never edit the taskbag's files by hand, and never invent progress, verification or approvals.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- Before a change the person must agree to, show your plan with `alm confirm` and change nothing
  unless it exits 0.
- `--push` commits only the taskbag's files; your other work is committed as usual.
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
        assert!(guide(Some("customize")).unwrap().contains("The six states"));
        assert!(
            guide(Some("tasks"))
                .unwrap()
                .contains("Do not make it ready yourself")
        );
        assert!(guide(Some("tasks")).unwrap().contains("alm task-language"));
        assert!(crate::agent_section().contains("run `alm task-language`"));
    }
}
