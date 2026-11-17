use std::path::PathBuf;

use clap::Subcommand;

use crate::{difficulty_level, difficulty_range};

#[derive(Subcommand)]
pub(crate) enum CardsWhat {
    /// Report card files you found in the project folder: <id>=<path> (relative to the project
    /// folder), several at once; <id>= withdraws a report.
    Found {
        #[arg(value_name = "ID=PATH", required = true)]
        items: Vec<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum Add {
    /// Add an area: `alm add area [<id>] <name>`. With only a name, the ID is the next free one,
    /// like A08 after A07.
    Area {
        /// The area's ID, then its name; or only its name.
        #[arg(value_name = "ID")]
        first: String,
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
    /// Add an iteration (when tasks are planned), ordered after the existing ones:
    /// `alm add iteration [<id>] <name>`. With only a name, the ID is the next free one, like I16
    /// after I15.
    Iteration {
        /// The iteration's ID, then its name; or only its name.
        #[arg(value_name = "ID")]
        first: String,
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// The release it produces, for example v0.2.
        #[arg(long)]
        release: Option<String>,
        /// First day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        start: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        end: Option<String>,
    },
    /// Add a task in backlog.
    Task {
        /// The task's ID. Default: the next free one in the area, like A01-04.
        id: Option<String>,
        #[arg(long)]
        area: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        /// The branch you are on, as your version control tool shows it (in Git:
        /// `git branch --show-current`; alm reads no version control). When it is a
        /// task's working branch, the new task gets that task's base; otherwise this branch.
        #[arg(long, value_name = "NAME")]
        branch: Option<String>,
        /// The base branch to use instead of the one --branch chooses.
        #[arg(long, value_name = "NAME")]
        base: Option<String>,
        /// The iteration to plan it in. Default: the active iteration, if one is
        /// (Task-A01-root343); otherwise none.
        #[arg(long, value_name = "ID")]
        iteration: Option<String>,
        /// A project folder it belongs to, by its short name (repeatable; decision 0019). While
        /// the taskbag has folders, say --folder or --no-folder: you decide, alm does not guess.
        #[arg(long = "folder", value_name = "NAME", conflicts_with = "no_folder")]
        folders: Vec<String>,
        /// It belongs to no particular folder (planning, naming, decisions).
        #[arg(long)]
        no_folder: bool,
        /// How hard it is, from 1 to 5: 1 one small change in one place; 2 a small change across
        /// a few files; 3 an ordinary feature or fix with its tests; 4 several parts change
        /// together, or risky; 5 needs design decisions or crosses many parts. Give it when you add
        /// a task (without it the task counts as 3).
        #[arg(long, value_name = "1-5", value_parser = difficulty_level)]
        difficulty: Option<u8>,
    },
}

#[derive(Subcommand)]
pub(crate) enum FolderWhat {
    /// Each folder: its name, path, whether it is there, whether its AGENTS.md has Allomos's
    /// section, and how many tasks name it.
    List,
    /// Bind a folder to the taskbag and write Allomos's section (pointing back here) into its
    /// AGENTS.md. Default name: from the folder's own name.
    Add {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    /// Give a folder a new short name, here and in every task that names it; the folder on disk
    /// keeps its name.
    Rename { name: String, new_name: String },
    /// Record where a folder is now, when it is not where the taskbag says. Moves no files.
    Locate { name: String, path: PathBuf },
    /// Unbind a folder. Without --confirm it says what it will do. Its tasks stay (the folder's
    /// name is taken off them), its files stay, and only Allomos's section leaves its AGENTS.md.
    Remove {
        name: String,
        #[arg(long)]
        confirm: bool,
    },
}

/// `alm labels …`: the project's label table (decision 0020).
#[derive(Subcommand, Debug)]
pub(crate) enum LabelsWhat {
    /// Add a label to the table, or set the meaning of one it has.
    Add {
        name: String,
        /// A one-line meaning.
        #[arg(long)]
        meaning: Option<String>,
    },
    /// Remove a label from the table: refused for a built-in label and while a task carries it.
    Remove { name: String },
}

#[derive(Subcommand)]
pub(crate) enum Rename {
    /// Give an area a new name; its ID stays.
    Area { id: String, name: String },
    /// Give an iteration a new ID, for example `I-knit` (Task-A00-rope803). Every task planned in
    /// it, done and dropped ones too, moves along; its name, release, dates, state and order stay.
    Iteration { old: String, new: String },
    /// Rename a step of the label ladder, and the label of every task that carries it (done and
    /// dropped ones too). Its place on the ladder stays.
    Label { old: String, new: String },
}

#[derive(Subcommand)]
pub(crate) enum ChangeWhat {
    /// Change an iteration's name, release, dates or state; its ID stays. What is not given stays
    /// as it is; an empty release or date ("") clears it. `--state`: planned, active or closed; at
    /// most one iteration is active.
    Iteration {
        id: String,
        /// planned, active or closed (Task-A01-root343).
        #[arg(long, value_name = "STATE")]
        state: Option<String>,
        #[arg(long)]
        name: Option<String>,
        /// The release it produces, for example v0.2.
        #[arg(long)]
        release: Option<String>,
        /// First day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        start: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long, value_name = "DATE")]
        end: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum MoveWhat {
    /// Put an iteration just before or just after another one (Task-A00-skate868). Its ID, name,
    /// release, dates and state stay; the iterations are numbered again in the new order.
    #[command(group(clap::ArgGroup::new("place").required(true).args(["before", "after"])))]
    Iteration {
        id: String,
        /// The iteration it goes just before.
        #[arg(long, value_name = "ID")]
        before: Option<String>,
        /// The iteration it goes just after.
        #[arg(long, value_name = "ID")]
        after: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum Delete {
    /// Delete an area. Refused while any task is in it, done and dropped ones included.
    Area { id: String },
    /// Delete an iteration. Refused while any task is planned in it, done and dropped ones
    /// included: plan them elsewhere first.
    Iteration { id: String },
}

