# Allomos — agent working guide

Allomos is developed by AI agents (Claude Code, Codex) working for the owner, **will.w.hui**. This file is the shared authority for every agent and is always loaded (Codex reads it directly; Claude Code imports it from `CLAUDE.md`). Keep it short: detail lives in the documents it links.

**What Allomos is:** a desktop app (the hub, with every feature) plus web and mobile companions that connect to it, built as our own code ([0008](docs/decisions/0008-build-our-own.md), [0009](docs/decisions/0009-desktop-hub-with-web-and-mobile-companions.md)). Its main job is **switching**: a project hops between AI agents (and their accounts) and between teammates without losing its tasks, progress or handoff notes. Two more sets build on that: **AI skills** and **progress management** (areas, lists and charts, a burndown with a forecast finish date) ([0014](docs/decisions/0014-main-purpose-and-three-sets.md)). Local-only projects need no sign-in; optional cloud projects add roles and email invitations. See [docs/product/PLAN.md](docs/product/PLAN.md) and [docs/product/REQUIREMENTS.md](docs/product/REQUIREMENTS.md).

## Always-on boundaries

- **No external effect without the owner's exact approval**, recorded on the card: creating or changing hosted resources (Supabase projects, databases, hosting, DNS), applying SQL to a real target, deployments, secrets, sending email, creating GitHub repositories, adding or changing a remote, and anything touching other people's data. Approving a card authorizes only its local work.
- **Never print or commit credentials**, tokens, cookies or local environment files (`.env*` stay ignored). Record *where* a credential lives, never its value — in [docs/AGENT_HANDOFF.md § Current truth](docs/AGENT_HANDOFF.md#current-truth).
- **Reference projects are read-only.** The SSR migration board and Glidelines (`glidepace-vibe-cc`), listed in [docs/REFERENCES.md](docs/REFERENCES.md), may be read and quoted; never edit, commit to, or run write operations in them.
- **Preserve unrelated changes** in the working tree and in other sessions' worktrees.
- **Do not upgrade dependencies incidentally**; pin versions deliberately in the card that introduces them.
- **Decisions are made by the owner.** A decision record with status *Open* or *Proposed* is not an approval. Do not build on an open decision beyond what the record says is safe (e.g. [0003](docs/decisions/0003-relationship-to-glidelines.md): core work is safe under every option).

## AI working mode

| Work | Codex | Claude |
|---|---|---|
| Everyday implementation, debugging, tests, docs, authorized commits | selected model, Medium reasoning | selected model, Medium effort where supported |
| Difficult architecture, security, data-model or forecasting decisions; focused review of a risky change | same model, highest reasoning | same model, higher effort (Extra high; Max only for the hardest bounded question) |

- One worker owns everyday progress within its card. Raise effort for a specific hard question, then return to routine effort. Model and effort are separate; never claim a model or effort switch that did not happen.
- Delegate only bounded, independent work with exact file ownership and acceptance criteria. Do not duplicate reading, implementation or review across agents.
- A provider or session change preserves decisions, approvals and evidence; it does not authorize repeating operations.

Details and handoff contents: [docs/workflow/AI-WORKFLOW.md](docs/workflow/AI-WORKFLOW.md).

## Working procedure

1. **The unit of work is one observable outcome**, written as one card: a file in [docs/tasks/](docs/tasks/) and a task in this repository's taskbag (`.allomos/taskbag/`, worked with `alm`). Format, labels and claiming: [docs/workflow/CARDS.md](docs/workflow/CARDS.md).
2. **Several sessions may work at once**, each in its own git worktree and branch (`claude/…`, `codex/…`), each holding **one** card. Claim before you work, following the [claiming protocol](docs/workflow/CARDS.md#claiming-protocol).
3. **Status has one home: the task in the taskbag on `main`**, changed only with `alm` (`alm list`, `alm next`, `alm claim`, `alm label`, `alm wait`, `alm done`, `alm release`). Card files hold detail, never a second copy of state. [docs/TASK_BOARD.md](docs/TASK_BOARD.md) keeps only Now and the owner's open questions.
4. **Labels are the only progress claims:** drafted → implemented → locally verified → deployed and verified → owner verified (Done). Each open card names its single next dependency.
5. **Integration first:** connect the real flow end to end, then refine pieces behind it.
6. **One card per outcome, including research and decision records.** Do not keep adding unrelated work to an existing card; open a new card, so each piece can be finished and reviewed on its own.
7. **Push after every commit** (owner, 2026-10-04): push your branch to `origin` (`mypals-ml/allomos`, renamed from `glidebox` 2026-10-06 and from `fliphopper` 2026-10-11) right after each commit, and confirm it landed (`git status -sb` shows no "ahead"). Never force-push, and never push to a remote other than `origin`.
8. **Say who made each commit.** End every commit message with a line naming the agent, for example `Co-Authored-By: Claude …` or `Agent: codex/<session-name>`, so history shows which agent and session did the work.
9. **Documents:** create a new file only for a **contract** (something code or agents must conform to), **evidence** (what actually ran), a **decision record**, or a **discussion record**. Status goes in the taskbag; supersede stale lines by editing them in place.
10. **Explain in plain words.** Every report and card says what happened and why in everyday language first, defining technical terms on first use; evidence (numbers, IDs, logs) comes after.

## Handoff — write it continuously

Quota can end a session without warning. After every meaningful step (a task finished, a finding, a commit, a process started or stopped), update your block in [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md#active-resume) and commit it with the work. Check usage at step boundaries where the tool exposes it (Claude Code: `get_usage`). Near **3% left** in the 5-hour window, start nothing new: finish the handoff and stop. At about **70% context used** (or past half at a natural break), bring the handoff up to date, commit, and give the owner the exact prompt for a new session. Rules: [docs/workflow/AI-WORKFLOW.md](docs/workflow/AI-WORKFLOW.md#handoff-timing-and-contents).

## Read by activity

| Activity | Read |
|---|---|
| Orient / resume | [docs/TASK_BOARD.md](docs/TASK_BOARD.md) and `alm list` / `alm next` → your card in [docs/tasks/](docs/tasks/) → [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md) (your block, Live state, Pitfalls). New to the project: [docs/README.md](docs/README.md) |
| Plan a card | [REQUIREMENTS.md](docs/product/REQUIREMENTS.md), [PLAN.md](docs/product/PLAN.md), the relevant [decisions](docs/decisions/README.md) |
| Implement / test | the card, affected source and tests; coding rules once they exist (`docs/workflow/CODING_RULES.md`, created by the first code card) |
| Look at prior art | [docs/REFERENCES.md](docs/REFERENCES.md) — read only the named files |
| Record a decision or discussion | [docs/decisions/README.md](docs/decisions/README.md), [docs/discussions/](docs/discussions/) |
| Handoff | actual diff and results; your handoff block; the board row |

Reuse context you already have; do not re-read unchanged documents. **The measure:** how many documents must be read before the next change. If it falls, this is working.

## Verification and reporting

- Status changes: `alm list --json` reports no `issues`.
- Documentation-only changes: check links and paths, instruction consistency, and `git diff --check`.
- Test folders for manual checks and agent trials go under `~/fliphopper-tests/<card>-<name>/` ([CODING_RULES](docs/workflow/CODING_RULES.md#writing-code)).
- Code changes: run the card's named checks; report PASS / FAIL / NOT RUN separately, and untested gates separately.
- Report briefly: what changed, what was verified, what remains and who it waits on.
