//! The `alm guide workflow` text (extracted from `guide.rs` to keep `guide.rs` within the
//! file-size baseline; Task-A03-book084).

pub(crate) const WORKFLOW: &str = "# How to work the tasks

1. Start with `alm next` (or `alm list`). Choose by its rule: the task the person named, if any;
   otherwise one from the active iteration (listed first: the one marked active, or else the
   first planned one), else the earliest iteration, whose next dependency is met. Read its handoff
   note first (`alm show <id>`), and the project notes' Running, Pitfalls and Repositories, which
   `alm next` shows (the whole file is the taskbag's `project-notes.md`).
   **Taking over from a handoff** (`alm next` and `alm show` repeat these two rules with a
   handoff note): check the real state before trusting it: the working copy, branch and running processes it
   names (as your version control tool shows them; in Git: `git worktree list`, `git status -sb`;
   and listening ports). What the previous session still
   holds is not yours to change or remove; if unsure, ask the person (in a quiet run, park the
   question with `alm wait`). What the task's **Evidence** lists as already done is not done
   again: a change of agent, account or session keeps the approvals
   and what was done; it does not authorize repeating it.
2. Hold one task at a time. Pull first (get the latest changes), then `alm claim <id> --as <your
   session name> --branch <the branch you are on>` (as your version control tool shows it; in
   Git: `git branch --show-current`; alm reads no version control), and save and share the claim
   right away with your version control tool (it prints the Git commands as the example): until
   your claim is shared, other sessions do not see it and may take the task too. A task someone else holds is not yours to change.
   **Two sessions claiming one task at once** (Task-A01-lawn320). Each claims in its own copy;
   only the claim shared first counts. When your version control tool refuses to share yours (in
   Git: the push is rejected), get the latest changes before anything else (in Git:
   `git pull --rebase`). If the other session's claim came in, the task's file holds both
   claims (a conflict), and every `alm` command first says so (`notice [claim_lost]`): the task
   is the other session's, with its last heartbeat. Do not work on it: drop your own claim, the
   change that could not be shared (in Git: `git rebase --skip`; after a plain `git pull`:
   `git merge --abort`, then `git pull --rebase` and `git rebase --skip`), and take another
   ready task: alm names the next one in `alm next`'s order, and you choose. Claiming a task
   that is held already is refused (`task_held`) with the same pointer to the next ready task.
   **While you hold a task: heartbeats** (decision 0022). These rules need nothing but reading
   them and running commands, so every agent follows them, whatever its tool. Pass `--as <your
   session name>` to every `alm` command (or set ALLOMOS_HOLDER), reading ones too: that is how
   `alm` tells you when a task was taken over from you. Send a heartbeat at least every 10
   minutes (the project's interval; `alm heartbeat-settings` shows it): `alm beat --as <your
   session name>` beats every task you hold in one change; save and share it as `alm` prints,
   since a heartbeat counts only once shared. Save only the files it lists, with the commands as
   printed: never `git add -A`, `git add .` or `git commit -a` in the taskbag, which also take
   changes others have not shared yet (the person's changes in the Allomos app, other sessions'
   changes), often without the events that record them. Under half the interval after the last one it
   writes nothing, so running it often costs nothing. Any change you make to your task (`label`,
   `note`, `handoff`, …) beats along with it; reading never does. Before a long step that runs
   no `alm` command for longer than the interval (a long build, a long test run, waiting for a
   slow tool), declare it first: `alm beat <id> --busy <length>`, as long as you judge it needs
   (no upper limit; `alm next` shows it to everyone), and `alm beat <id> --busy 0m` when it ends
   early. Without heartbeats or a busy window, after the project's limit (30 minutes by default)
   your task is lost and another session may take it over.
   **While you hold a task: keep it ready for a takeover** (Task-A01-butter895). Whoever takes it
   over may be on another computer, and only what is pushed reaches it: commits not pushed,
   changes not committed and your conversation stay behind. So after each small step, update the
   handoff note (`alm handoff <id>`), not only at the end, and say what you are doing with your
   heartbeats: `alm beat --as <your session name> --doing \"<what, in a few words>\"`. While the
   project's setting to push work in progress at each heartbeat is on (`alm heartbeat-settings`;
   on by default), also commit and push to your working branch after each small step and at every
   heartbeat, half-finished work too, with WIP in the commit message, so at most one heartbeat
   interval of work is lost; `alm beat` reminds you and prints the commands. alm runs no version
   control tool: the commits and pushes, of the working branch and the taskbag, are yours. With
   the setting off, `alm beat` does not remind you and those pushes are not asked for; the handoff
   note and `--doing` stay. A branch with many WIP commits may be merged with a squash (in Git:
   `git merge --squash`, then `git branch -D` once it is in); never force-push.
   **Taking over a lost task** (decision 0022 § 6). A held task whose holder has sent no
   heartbeat for longer than the project's limit, with no busy window running, is lost: `alm next`
   lists it under \"Lost, can be taken over\" with the command. Take one over only when you mean to
   continue its work. Claiming it is the takeover: you become the holder, the claim's event
   records whom you took it from and when, and the work goes on on the task's working branch
   (the claim prints the commands to start a worktree from it). The former holder's worktree may
   still hold changes it did not save or share; the claim says where it ran when known: look,
   but do not change, commit or remove anything there: only what was pushed counts, and that folder
   is of use only on the computer it ran on (one exception: when the person chose \"Commit and push
   these changes for it first\" in the Allomos app's Take over… dialog, its request names the
   worktree and the files: commit them there as one WIP commit, push the working branch and make
   sure it is not \"ahead\", change nothing else there, and ask the person first when what you
   find differs from what the request lists); on another computer, go by the working branch and the
   handoff note, and by what the holder last said it was doing (`alm show`, `alm next`; the claim
   repeats it). A held task that is not lost is still
   refused unless you pass `--take`, and `--take` is only for a holder that has stopped or when
   a person asks for the takeover (the Allomos app's Take over… writes that in its request); on
   your own, take over only lost tasks. Two sessions taking over at once: as with two claims at once (above), the claim
   you push second is refused by your version control tool; get the latest changes
   (`git pull --rebase`), and `alm` says the task is the other session's (`notice
   [claim_lost]`): drop your claim, leave its branch alone and take another task. Once you hold it, beat like any holder (`alm beat`): when you
   go silent, another session can take it over from you in turn.
   **Taken over from you** (decision 0022 § 8). Back after a pause (your quota came back, a
   restart), any `alm` command run with `--as <your session name>` first says which of your
   tasks another session has taken over, by whom and when (`notice [taken_over]`); it refuses
   your changes to them (`task_held`). Give such a task up: stop work on it at once (no more
   commits or pushes to its working branch, and do not claim it back), leave your worktree as it
   is, tell the person in your conversation that it was taken over and by whom, and go on with
   another ready task (`alm next`) or end.
   **Asked to prepare a handoff** (Task-A01-kind858). A person can paste a prompt into your
   session, copied from the app's Prepare a handoff…, asking you to get a task you hold ready for
   the next one. Do it yourself: finish the small step you are on and start nothing new on the
   task, write your handoff (`alm handoff <id>`), commit and push your work to the working branch,
   and release the task (`alm release <id> --note \"<what the next one needs, and the working
   branch>\"`), then share that change. Stop work on it there: the person starts whoever goes on
   with it separately, on the working branch.
   **When Git cannot write the checkout** (Task-A03-pier032): `Operation not permitted` on
   `.git/…` (as in Codex's sandbox) means your agent may not change the repository from where
   it runs. Stop and say so: the agent needs Allomos's rules for Codex (`alm guide
   codex-rules`; in the app, the option in Monitor ready tasks). Do not make another copy of the
   repository to get around it: the other sessions would go on reading the stale one. Run each
   Git and `alm` command on its own (no `&&`, `;` or pipes): a command joined with others runs
   in the sandbox. Unattended Codex does not use `git -C <folder>`: it runs `git …` in that
   folder. On a task claimed with `--session-agent codex`, `alm` prints its Git commands in
   that form already, under a line naming the folder (`For Git, in <folder> …`): run each as
   it is, with that folder as its working folder. **Which `alm`:** use the one the person or the request names; when a command or
   field is unknown, check `alm --version` (or `which alm`) first.
   When you claim, pass what the Allomos app needs to reopen your session: `--session-agent
   <your agent's id>`, `--session-id <your session's ID>`, and whatever else your agent needs to
   reopen it. In the Claude desktop app you must also pass `--session-kind desktop
   --app-session-id <its local_… ID>`: without them the Allomos app resumes your session in a
   terminal instead of opening it in the Claude app (in a terminal, pass `--session-kind
   terminal`). alm stores what you pass and checks nothing; the task keeps
   it after release and done, and the next claim replaces it.
   **Branches.** Every task has a base branch, by its real name (`main`, `master`,
   `storage-v2`…): it starts from it and merges back into it; merging one base branch into
   another is a task of its own. The claim records your working branch, the one you work on
   (`--working <name>` when it is not the one you are on, for example a new branch for a
   worktree of its own),
   prints which base to start from and merge into with the Git commands as the example, and says
   so when you
   are not on the base. A task without a base gets the branch you are on.
   **Always work a task in a worktree of its own under the current project's `.worktrees/`
   directory** (in Git: `git worktree add .worktrees/<short-name> -b <working branch> <base>`,
   which `alm claim` prints; keep `/.worktrees/` in `.gitignore`), never in the main checkout:
   the main checkout stays on the base branch so several sessions can work at once. Merge from
   the main checkout, where the base is checked out. Once the task's branch is merged into its
   base, remove your own worktree and the merged branch; never another session's.
   **Before you plan: the size of the files you will change** (Task-A03-tide388,
   Task-A03-dapper008; `alm claim` repeats it). A source file over the project's line limit
   (`alm file-size-settings`; 1000 lines by default, tests not counted) must not be longer after
   any task than before it. How is your call: put the new things somewhere else, or move
   unrelated old things out of it. The project's own tests check it (`alm guide file-size`; the
   project notes' `## File size check` says where and how to run it; a project without one sets
   it up once). A refactoring splits out only the part the task changes, never the whole file, as
   a commit of its own before the feature commit, with the tests passing and the behaviour
   unchanged after it; one bigger than the feature itself, or reaching files outside the task, is
   a task of its own first (`alm block <this task> --by <new task>`). Each time the rule leads to
   a choice, record it, split or not, and when you raise a baseline number: `alm ai-decided <id>
   \"file-size: <file>: <lines before> → <lines after> (tests not counted); <what you did>; <if
   nothing was split, why>\"`, beginning with `file-size:` in every language; find them later by
   searching the taskbag for `file-size:`. alm only reminds: it measures no file and stops
   nothing.
3. As you prove each step, `alm label <id> \"<label>\"`, and keep its next dependency current
   with `alm needs`.
4. Stopped: when a person (or an outside event) must act, `alm wait <id> --on \"<what>\"
   [--who \"<who>\"]`; when another task must finish first, `alm block <id> --by <task-id>`: its
   state stays as it is, and it is blocked while a blocker has not finished (a task can be both
   waiting and blocked). Waiting and blocked tasks are never in `alm next`'s list. When a task's
   last blocker finishes (done or dropped), nothing changes in it: a ready one is then listed, a
   backlog one stays backlog; read its blockers' AI decisions and handoffs before you start it.
   To stop waiting for a blocker, `alm unblock <id>`.
5. Write your handoff when `alm handoff-rules` says (near the quota or context gate while you
   hold a task, then release it with `alm release <id> --note` naming the working branch, so the
   next session continues it; and after every meaningful step), into each task you worked: `alm handoff <id> --as <your session name>
   --text \"<where it stopped, the one next step, who it waits on>\"`, in plain words first, for
   any reader: you do not know who takes over, an agent or a person. It replaces the task's
   handoff note, so write the whole of it each time. Each operation with an outside effect that
   must not run twice goes in with `--evidence \"<what>\"` (under the description's
   **Evidence**). Record each check you ran or did not run with `alm check-result <id> --name \"<check>\"
   --result pass|fail|not-run [--detail \"<why>\"] [--screenshot <file>]`; other evidence with `alm describe`.
   **Write for a person first.** A person reads every handoff note, every AI decision and every set
   of check steps, often on the go and without your conversation. Each keeps one order:
   1. One sentence in plain words that a person can act on: where the work stands, or what was
      decided, as they would see it. \"Done and merged: the list now shows who holds each
      task\", not \"Merged 079bbb5b, cargo test passes\".
   2. What the person has to do, or that there is nothing to do: the question to answer, the
      check to make, the approval to give. Check steps say here about how many minutes they take
      and which step matters most (the one to do when there is time for only one).
   3. The technical detail last: commit IDs, commands, task IDs, file names, test counts, the
      run mode line.
   Never open with a commit ID, a command or a file name. For a handoff note, what the next
   agent needs (where it stopped, the one next step, the working branch) follows what the person
   needs. An AI decision is one line: what the person sees or what changes for them, then why, then
   names in the code. The order is how you write; the data format is the same.
6. Stopping before it is done (for example your session ends): `alm release <id> --note \"<what
   the next one needs>\"`.
7. Before you finish a task, record its risk yourself, with one sentence of why (decision
   0027): `alm classify <id> --risk low|medium|high --touches production|others-data|none --why
   \"<one sentence: which rules apply, and what in the work makes them apply>\"`. The rules are
   under **Risk rules** below; when several apply, the highest one counts, and the rules the
   project added count too. You judge: alm records what you say and checks nothing, and the
   approver may change it when reviewing.
   Finished: once the task has a \"by human\" label (a person has checked it: the human check),
   `alm done <id>`. When the work is finished and only a
   person's hands-on check is left, finish it without that label, in quiet and normal mode alike (step 8):
   `alm done <id> --without-human-check \"<why>\" --steps <file.md>`; say so in your handoff.
   Do not leave such a task waiting: `alm wait` is only for a real question, an approval or an
   outside effect (step 4). Write the steps so a person can pick it up and test
   it, in the order of step 5: first one sentence saying what they are checking, about how many
   minutes it takes and which step matters most; then the steps: the test folder, what to open,
   what to click, what they should see; commands and IDs last. The task keeps its
   label, the note and the steps, so a person finds it with `alm human`; once they have checked
   it, `alm checked <id>`. When a finished task needs more work on the same outcome (it failed
   its check, or the person asks for more), reopen it instead of adding a task:
   `alm reopen <id> --note \"<why>\"` makes it ready again, held by nobody, with the reason in
   its history; then claim it as usual.
8. Before an automatic or loop run begins (a card run, `/loop`, several tasks one after
   another), ask the person to choose quiet or normal mode: show them what
   `alm guide run-modes` prints (in the task language), as it is, and do not start until they
   choose. Write the choice in your handoff (in the task's handoff note: \"Run mode: quiet\" or
   \"Run mode: normal\", who chose it and when) and follow it until the run ends. Both modes
   finish a task the same way (step 7); they differ only in what you do when you need the
   person during the run. In normal mode you stop and ask. In quiet mode (a quiet run),
   never stop to wait for the person (approver, 2026-11-09, Task-A00-eclair542):
   - **Wording and look are reviewed before release, not before the work.** Decide them
     yourself, build and merge, and record them with `alm ai-decided <id> \"<what and why>\"`,
     with a screenshot when your agent can take one (say where it is). The approver reviews them
     under To review and reopens the task to change them. So does every other routine choice.
   - **Only these go to the approver first:** data formats, removing things, permissions and
     security, anything with a cost or an outside effect, and anything that changes the plan or
     another task. A release still needs the approver's approval.
   - **Every question carries your proposal:** park the task with
     `alm wait <id> --on \"<the question> Proposal: <what you would do>\"` (it shows in
     `alm human` under To answer) and go on with the next ready task. A question about something
     cheap to undo, still unanswered when the run ends, is settled by your proposal: do it and
     record it with `alm ai-decided`.
   - **A parked question never holds back finished work.** What is done and tested and does not
     depend on the answer is merged into the base first. When all the task's work is done, finish
     it with `alm done <id> --without-human-check` (step 7) and record the question with
     `alm ai-decided` for review, instead of leaving the task waiting.
   - Do not use `alm confirm` in such a run: it would wait until it times out.
   Questions are asked while planning, not while running: whoever makes a task ready asks its
   open questions first (`alm guide tasks`, step 7), so a quiet run meets as few as possible.
9. **A goal run** (`alm guide goal`): when the person gives you a goal, an app to build from their
   description, follow the goal prompt `alm guide goal` prints (in the task language). It is the
   one exception to asking the person before planning (approver, 2026-10-16): submitting the goal
   approved the plan in advance, recorded once on the goal task (its title starts with `Goal:`),
   so you add the areas and iterations and make your own tasks ready. Anything with an outside
   effect still waits for the approver (`alm wait`). There is no spend cap: the quota gate of
   `alm handoff-rules` is the limit. Given the goal again, continue from `alm next`.

Rules:
- Never edit the taskbag's files by hand, and never invent progress, verification or approvals.
  One exception: keep `project-notes.md` true. Update it when you start or stop a server or process, create or change
  a hosted resource or a remote repository, learn a pitfall that would change what the next
  session does, or a credential moves; change lines in place, remove lines no longer true, and
  save and share that file with your version control tool yourself.
- Never write credentials or personal details (a person's or an account's email address, account
  or user name, phone number, home address, local network address) into the project notes, a
  handoff, a task, a document or the code: write a placeholder such as
  `<CLOUDFLARE_ACCOUNT_EMAIL>` and how to look it up (`npx wrangler whoami`). Version history
  keeps whatever was written, even after it is removed.
- Every new piece of work (a new card, a new outcome) gets its own task: `alm guide tasks`, then
  `alm add task`.
- Before adding a task or changing its title or description, run `alm task-language` and write in
  that language.
- A change the person must agree to (an outside effect, or anything that is not a routine choice:
  below): ask in the conversation. Only on a job the person gave you from the Allomos app (they do
  not see your conversation) show your plan with `alm confirm` instead, with the evidence
  (`--evidence`) and the task (`--task`), and change nothing unless it exits 0. In a quiet run,
  park it with `alm wait` instead (step 8). A Go ahead covers only the plan shown: if it changes,
  ask again.
- Routine choices inside a task (a name, a file layout, how something is built inside the task, a
  small detail with an obvious default; anything cheap to undo) are yours: decide, carry on, and
  record each with `alm ai-decided <id> \"<what and why>\" --kind visible|principle|detail`, so
  the approver can review it with `alm human` (a `detail` is kept, not listed for review). Start each with what the person sees, then why (step 5). Wording and look (what users see) are reviewed before release, not before the
  work (step 8). Data formats, removing things, permissions and security, anything that changes
  the plan or another task, and anything with a cost or an outside effect go to the approver
  first (`alm wait`). In a goal run (step 9) the plan was approved in advance; outside effects
  still go to the approver.
- After every `alm` change, save and share the files it lists with your version control tool (it
  prints the Git commands as the example): only that change's files. Your other work is saved as
  usual.
";
