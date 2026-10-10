//! What `fh guide` tells an AI agent (Task-A05-02). The short brief in a project's `AGENTS.md`
//! ([`crate::agent_section`]) names these topics; an agent reads them from `fh` when it needs
//! them, so the details always match the installed `fh`, not a copy that went stale. English:
//! agents read it, the app does not show it.

/// The topics, in the order `fh guide` prints them.
pub const GUIDE_TOPICS: [&str; 5] = ["terms", "customize", "tasks", "commands", "workflow"];

const TERMS: &str = "# FlipHopper terms

FlipHopper relies on these terms. Use them as defined here, in commands and when you talk
about the project's tasks.

- **Seabag**: the folder holding the project's tasks (`.fliphopper/seabag/`): readable JSON and
  Markdown files tracked by Git. Change it only with `fh`.
- **Project folder**: the folder with the project's code and its AGENTS.md / CLAUDE.md. Usually
  the folder holding `.fliphopper/`; `project_folder` in the seabag's project.json records it.
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
  notes). `fh task-language` tells you which; by default it follows the language the person
  uses FlipHopper in.
";

const CUSTOMIZE: &str = "# What a project may change, and what is fixed

| Term | This project may change | How |
|---|---|---|
| Areas | which areas exist, their IDs and names | `fh add area <id> \"<name>\"` |
| Iterations | which exist, their names, releases and dates; which task is in which | `fh add iteration`, `fh plan` |
| Label ladder | its steps and their order | `fh labels \"<a,b,…>\"` (refused if a task's label would be left off) |
| Task IDs and titles | any ID of letters, digits, `-` or `_` (for example A01-02) | `fh add task <id> --area <id> --title \"…\"` |
| Description | free Markdown in description.md | written with `fh add task --description` |
| Task language | `ui` (follow the FlipHopper app's language, the default) or a language code | `fh task-language <code or ui>`; only when the person asks |

Fixed for every project:

- **The six states**: backlog, ready, in progress, waiting, done, dropped.
- **The words \"area\" and \"iteration\"**: a project cannot rename them yet (planned: requirement
  R90). Use these words with `fh`, even if the team says \"workstream\" or \"sprint\".
- **The file format**: never edit the seabag's files by hand; `fh` checks every change.

Read the project's current areas, iterations and tasks with `fh list` (`--by iteration`), and its
ladder in the seabag's project.json.
";

const TASKS: &str = "# How to add a task

Read this before you add a task, so it fits this project.

0. **Language.** Run `fh task-language` and write the title and description in that language.
   IDs, commands, code and file names stay as they are.
1. **Look first.** `fh list` shows the areas and the task IDs already used; `fh show <id>` shows
   how an existing task is written. Follow what you see.
2. **Area.** Use an area that exists. Add a new one (`fh add area <id> \"<name>\"`) only when the
   person asks for it or agrees to it.
3. **ID.** Leave it out: `fh add task --area <area> --title \"…\"` takes the next free number in
   the area (A01-04 after A01-03). Give an ID only to follow a pattern the project already uses,
   for example a child task `A01-03-01` under `A01-03`.
4. **Title.** One observable outcome: what a person can do or see when it is done (\"A customer
   orders a cake for a pick-up day\"), not a technical step (\"add an orders table\").
5. **Description** (`--description`, Markdown), in three short parts:
   - **Outcome.** One or two plain sentences.
   - **Done means.** The checks or demonstrations that prove it.
   - **Notes.** Anything the next person needs: links, open questions, who must decide.
6. **State.** A new task starts in backlog. Do not make it ready yourself: the person approves it
   (`fh ready <id>`). Plan it in an iteration (`fh plan <id> <iteration>`) only when asked.
7. **One outcome per task.** If it is really two outcomes, add two tasks.

Add it with `--push` when the project shares its tasks through Git, so others see it.
";

const COMMANDS: &str = "# fh commands

Every command takes `--seabag <path>` (or FLIPHOPPER_SEABAG; default: the nearest
`.fliphopper/seabag/` above the current folder), `--as <name>` (or FLIPHOPPER_HOLDER), `--json`
(one JSON object on standard output), and for changes `--commit` or `--push` (commit only the
seabag's files; push the current branch). Exit codes: 0 done, 1 refused by a rule, 2 wrong
usage, 3 seabag busy or not writable.

See:
- `fh next`: the first ready task no one holds, with its handoff note.
- `fh list [--area <id>] [--iteration <id>] [--state <state>] [--mine] [--by area|iteration]`
- `fh show <id>`: one task in full, with its description.

Work a task:
- `fh claim <id>`: you hold it; state in progress. `--take` only when its holder has stopped.
- `fh label <id> \"<label>\"`: a step of the ladder.
- `fh needs <id> \"<what>\"`: its next dependency (no text clears it).
- `fh wait <id> --on \"<what>\"`: state waiting.
- `fh release <id> --note \"<what the next one needs>\"`: stop short of done.
- `fh done <id>`; `fh drop <id>`; `fh ready <id>` (an unheld backlog or waiting task).

Plan:
- `fh add area <id> \"<name>\"`, `fh add iteration <id> \"<name>\" [--release …] [--start …] [--end …]`
- `fh add task [<id>] --area <id> --title \"<title>\" [--description \"<markdown>\"]`: without
  an ID, the next free one in the area. Read `fh guide tasks` first.
- `fh plan <id> [<iteration>]`; `fh labels \"<a,b,…>\"`
- `fh task-language`: the language to write new and changed tasks in (run it first);
  `fh task-language <code|ui>` sets it.

Ask the person:
- `fh confirm --title \"<heading>\" --plan <file.md>`: shows your plan in the FlipHopper app and
  waits; exit 0 means go ahead, anything else means change nothing.

About FlipHopper:
- `fh brief`: the section for AGENTS.md. `fh guide [terms|customize|tasks|commands|workflow]`:
  this guide.
- `fh init`: create a seabag (only when there is none).
";

const WORKFLOW: &str = "# How to work the tasks

1. Start with `fh next` (or `fh list`), and read the task's handoff note and description.
2. Hold one task at a time: `fh claim <id> --as <your session name> --push`. A task someone else
   holds is not yours to change.
3. As you prove each step, `fh label <id> \"<label>\" --push`, and keep its next dependency
   current with `fh needs`.
4. Blocked on a person or another task: `fh wait <id> --on \"<what>\" --push`.
5. Stopping before it is done (for example your session ends): `fh release <id> --note \"<what
   the next one needs>\" --push`.
6. Finished: the top label, then `fh done <id> --push`.

Rules:
- Never edit the seabag's files by hand, and never invent progress, verification or approvals.
- Before adding a task or changing its title or description, run `fh task-language` and write in
  that language.
- Before a change the person must agree to, show your plan with `fh confirm` and change nothing
  unless it exits 0.
- `--push` commits only the seabag's files; your other work is committed as usual.
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
                crate::agent_section().contains(&format!("fh guide {topic}")),
                "{topic}"
            );
        }
        assert_eq!(guide(Some("nope")), None);
        let all = guide(None).unwrap();
        assert!(all.contains("# FlipHopper terms") && all.contains("# How to work the tasks"));
        // The fixed parts are said plainly.
        assert!(guide(Some("customize")).unwrap().contains("The six states"));
        assert!(
            guide(Some("tasks"))
                .unwrap()
                .contains("Do not make it ready yourself")
        );
        assert!(guide(Some("tasks")).unwrap().contains("fh task-language"));
        assert!(crate::agent_section().contains("run `fh task-language`"));
    }
}
