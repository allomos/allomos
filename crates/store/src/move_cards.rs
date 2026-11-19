//! `alm guide move-cards` (Task-A00-30), moved out of `guide.rs` to keep that file from growing
//! (Task-A01-easel128). `guide.rs` prints it with the other topics.

/// `alm guide move-cards` (Task-A00-30, approver 2026-10-16): Allomos reads only the tasks'
/// `Card:` links (Task-A01-51); the agent finds the other card files, reports them with
/// `alm cards found`, and moves them in after the person's Go ahead (`alm confirm`).
pub(crate) const MOVE_CARDS: &str = "# Moving card files in beside their tasks

A card file holds one task's detail in a file of its own (its outcome, approvals, notes and
evidence). In Allomos it lives in the taskbag, beside the task's own files, as
`tasks/<id>/card.md`, and the task's description links to it with a line
`Card: [card.md](card.md)`, so a task is kept in one place and the Allomos app opens its card.
`alm` and the app read only each task's `Card:` link: `alm cards` lists the tasks whose link
points outside the task's folder, and the app shows them. You find the rest, and nothing moves
until the person says Go ahead.

1. Look first, change nothing. Start from `alm cards`: the tasks whose `Card:` link points
   outside the taskbag, and what agents reported. Then find the descriptions that link to a card
   file in another way it does not read (`alm list --json` gives every description). Also look in the project folder for card files named after task IDs (for
   example `docs/tasks/Task-A01-02-*.md`) that no description links to. A card file belongs to the
   task its link or its name names. Card files without a task are never moved: list them for the
   person. A description that holds a whole card merged into it may be split back the same way:
   the card's part becomes the card file, the description keeps the rest and the `Card:` link.
2. Report what you found that `alm cards` does not list yet, or where a link names the wrong
   file, so the person sees it even before you ask: `alm cards found <id>=<path> <id>=<path> …`
   (each path relative to the project folder; several in one command; a report wins over the
   task's link). The Allomos app shows \"N tasks keep their card files in the project folder\"
   with **Update the taskbag…**, which opens an agent with this guide's request typed in (`alm
   move-in` prints it). `alm cards` lists them; `alm cards found <id>=` withdraws a report.
3. The one question. Write the plan in Markdown to a file outside the project: every card file
   and the task it goes to (`tasks/<id>/card.md`); the links that change (each description's
   `Card:` link; links inside a card that are relative to its old place; links in other documents
   to a card's old place); the originals you will remove from the project folder; card files
   without a task; tasks you cannot change (an open task held by another session: its holder
   moves it later); and how you will commit. Then `alm confirm --title \"Update the taskbag\"
   --plan <file>`. Exit code 0 means go ahead with the whole plan; any other means change
   nothing: tell the person and stop. (In a quiet run, do not use `alm confirm`: park the plan as
   a question with `alm wait`.)
4. Move each card. When it links to other files by relative paths, first write a copy outside the
   project whose links work from `tasks/<id>/`: a link to another card becomes `../<ID>/card.md`;
   a link to a project file becomes its path from the task's folder (the form the `Card:` links
   used, for example `../../../../docs/x.md` when the taskbag is the project's
   `.allomos/taskbag/`); a link to a file that no longer exists becomes plain text. Then
   `alm card <id> --from <copy>` (without `--from`: the reported file, else the linked one, as it
   is). `alm` writes
   `tasks/<id>/card.md`, points the description's `Card:` link to it (adding one at the top when
   there is none) and clears the report; it never replaces a different `card.md` (`card_exists`).
   A finished or dropped task can be changed whoever is named as its holder.
5. Finish what the plan names: remove the originals from the project folder (in Git: `git rm`), change the
   documents that linked to a card's old place, and remove the cards folder once it is empty.
   Save and share the taskbag's changes (the files `alm` listed; it printed the Git commands as the
   example) and the project's changes, each in its own repository. Check: `alm cards` lists none, `alm list --json` reports no issues, and
   every moved task's `Card:` link opens its `card.md`.
6. Tell the person what moved, what did not and why, and the card files without a task.
";
