// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): one button starts unattended
// monitoring of the project's ready tasks, or one iteration's. Each ready task is worked in a new
// agent session, one at a time; with none ready, the monitor waits and checks again at the chosen
// frequency. Every route must run unattended (approver, 2026-11-04); this first slice is the
// route Allomos runs itself: Claude Code or Codex in a terminal, started with the request. Kept
// apart from React so it can be tested with Node alone (`npm --prefix apps/web test`). Only type
// imports here: Node strips them.

import type { Language } from "./i18n";
import type { Iteration, Task } from "./types";

/** How often to check when no task is ready, in minutes: these seven only (approver, 2026-11-04). */
export const FREQUENCIES = [1, 5, 10, 30, 60, 90, 120] as const;
export type Frequency = (typeof FREQUENCIES)[number];
export const FIRST_FREQUENCY: Frequency = 30;

/** A task session with no sign of life this long (neither its task nor anything else changed) is
 * shown as stalled, for the person to look at; Allomos never ends it. */
export const STALL_MS = 30 * 60 * 1000;

/** The agents whose sessions Allomos starts itself, in a terminal. */
export type TerminalAgent = "claude_cli" | "codex_cli" | "antigravity_cli";
export const TERMINAL_AGENTS: readonly string[] = ["claude_cli", "codex_cli", "antigravity_cli"];

/** Which tasks the monitor takes: every ready one, or one iteration's (`current`: it was the
 * active iteration when the run started, shown as "current iteration"). */
export type Scope = { kind: "all" } | { kind: "iteration"; id: string; current: boolean };

/** The iterations in plan order (by `order`, then by ID, as the taskbag sorts them). */
function inPlanOrder(iterations: Iteration[]): Iteration[] {
  return [...iterations].sort((a, b) => a.order - b.order || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** The tasks `alm next` lists (or `alm next --iteration <id>`), in its order: ready and held by
 * no one; the active iteration first, then the others in plan order, unplanned last; then area
 * order, then ID (crates/core/src/ops.rs `next`). */
export function readyTasks(
  tasks: Task[],
  areas: { id: string }[],
  iterations: Iteration[],
  active: string | null | undefined,
  iteration?: string,
): Task[] {
  const planned = inPlanOrder(iterations).map((i) => i.id);
  const iterationRank = (t: Task) => {
    if (t.iteration && t.iteration === active) return 0;
    const p = t.iteration ? planned.indexOf(t.iteration) : -1;
    return p < 0 ? Number.MAX_SAFE_INTEGER : p + 1;
  };
  const areaRank = (t: Task) => {
    const p = areas.findIndex((a) => a.id === t.area);
    return p < 0 ? Number.MAX_SAFE_INTEGER : p;
  };
  return tasks
    .filter((t) => t.state === "ready" && !t.holder)
    .filter((t) => iteration === undefined || t.iteration === iteration)
    .sort((a, b) => iterationRank(a) - iterationRank(b) || areaRank(a) - areaRank(b) || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** The Which tasks field: its options with their counts of ready tasks nobody holds. */
export interface ScopeOptions {
  /** The active iteration, when there is one. */
  current: { id: string; ready: number } | null;
  /** Every ready task. */
  all: number;
  /** One iteration…: "other" (Other iteration…) beside an active one, "one" (One iteration…)
   * without; `items` in plan order, the active one left out; empty when the project has none. */
  other: { name: "other" | "one"; items: { place: number; id: string; ready: number }[]; none: boolean };
}

export function scopeOptions(
  tasks: Task[],
  areas: { id: string }[],
  iterations: Iteration[],
  active: string | null | undefined,
): ScopeOptions {
  const count = (id?: string) => readyTasks(tasks, areas, iterations, active, id).length;
  const current = active && iterations.some((i) => i.id === active) ? { id: active, ready: count(active) } : null;
  const items = inPlanOrder(iterations)
    .map((it, i) => ({ place: i + 1, id: it.id, ready: count(it.id) }))
    .filter((it) => it.id !== current?.id);
  return {
    current,
    all: count(),
    other: { name: current ? "other" : "one", items, none: iterations.length === 0 },
  };
}

/** Where the dialog was opened from: the top bar, or the active iteration's card. */
export type OpenedFrom = "top" | "iteration";

/** The choice the dialog starts on, worked out again every time it opens (never remembered):
 * from the active iteration's card, that iteration; from the top bar, the active iteration when
 * it has ready tasks, else every ready task. */
export function firstScope(options: ScopeOptions, from: OpenedFrom): Scope {
  const c = options.current;
  if (c && (from === "iteration" || c.ready > 0)) return { kind: "iteration", id: c.id, current: true };
  return { kind: "all" };
}

/** How many ready tasks nobody holds the chosen scope has now. */
export function scopeReady(options: ScopeOptions, scope: Scope): number {
  if (scope.kind === "all") return options.all;
  if (scope.current) return options.current?.ready ?? 0;
  return options.other.items.find((i) => i.id === scope.id)?.ready ?? 0;
}

/** The terminal tab Allomos opened for a session (macOS Terminal: its window and terminal device),
 * so that tab alone is closed once its task is done (Task-A03-sierra014). */
export interface SessionTab {
  window: number;
  tty: string;
}

/** The task a session is working for the monitor. */
export interface Current {
  id: string;
  /** The session ID Allomos gave the agent (Claude Code's `--session-id`); null when the agent
   * picks its own (Codex). */
  sessionId: string | null;
  /** The tab it runs in, when Allomos can close it (null: it stays open whatever happens). */
  tab: SessionTab | null;
  /** When the session was started (ms). */
  at: number;
  /** Seen held by someone since it was started: it was claimed. */
  seenHeld: boolean;
  /** The task as last seen (what changes when the session works it), and since when. */
  seen: string;
  changed: number;
}

/** One monitoring run, kept by the app on this computer only, never in the taskbag. */
export interface Run {
  scope: Scope;
  agent: TerminalAgent;
  quiet: boolean;
  every: Frequency;
  /** When it started (ms). */
  started: number;
  /** The tasks it has started a session for, in order (the current one last while it works). */
  taken: string[];
  current: Current | null;
  /** Stop asked while a task is worked: the run ends when that task is finished. */
  stopping: boolean;
  /** When it last checked for a ready task (ms). */
  checked: number;
  /** The person was told no task is ready (Task-A00-eclair542): told once per stretch without one,
   * again only after a task has been taken. */
  idleTold?: boolean;
}

/** A new run; it checks at once. */
export function startRun(scope: Scope, agent: TerminalAgent, quiet: boolean, every: Frequency, now: number): Run {
  return { scope, agent, quiet, every, started: now, taken: [], current: null, stopping: false, checked: now - every * 60_000 };
}

/** What of a task its session changes as it works it. */
function seenOf(task: Task | undefined): string {
  return task ? JSON.stringify([task.state, task.holder, task.labels ?? [], task.handoff_note, task.waiting_on]) : "";
}

/** The run with its current task followed in the taskbag as it is now. The task is finished when
 * it is done, dropped, waiting or blocked (alm keeps the last holder on those), when no one holds
 * it any more after being claimed or without ever leaving `ready` unheld, or when it is gone. */
export function follow(run: Run, tasks: Task[], now: number): { run: Run; finished: string | null } {
  const cur = run.current;
  if (!cur) return { run, finished: null };
  const task = tasks.find((t) => t.id === cur.id);
  const seen = seenOf(task);
  const seenHeld = cur.seenHeld || Boolean(task?.holder);
  const stopped = task !== undefined && ["done", "dropped", "waiting", "blocked"].includes(task.state);
  const finished = !task || stopped || (!task.holder && (seenHeld || task.state !== "ready"));
  if (finished) return { run: { ...run, current: null }, finished: cur.id };
  const next = seen === cur.seen && seenHeld === cur.seenHeld ? cur : { ...cur, seen, seenHeld, changed: seen === cur.seen ? cur.changed : now };
  return { run: next === cur ? run : { ...run, current: next }, finished: null };
}

/** The tab to close now that the run's current task is finished: only when the task is **done**
 * (approver, 2026-11-08); waiting, blocked, released, dropped or gone leaves the tab open, so the
 * person can see why it stopped. `run` is the run before [`follow`] let the task go. */
export function tabToClose(run: Run, tasks: Task[]): SessionTab | null {
  const cur = run.current;
  if (!cur?.tab) return null;
  return tasks.find((t) => t.id === cur.id)?.state === "done" ? cur.tab : null;
}

/** When the next check is due (ms). */
export function nextCheck(run: Run): number {
  return run.checked + run.every * 60_000;
}

/** The next task to start a session for: the first ready one in the scope this run has not
 * taken yet (a task its session released is not taken again in the same run). */
export function pick(run: Run, tasks: Task[], areas: { id: string }[], iterations: Iteration[], active: string | null | undefined): Task | null {
  const id = run.scope.kind === "iteration" ? run.scope.id : undefined;
  return readyTasks(tasks, areas, iterations, active, id).find((t) => !run.taken.includes(t.id)) ?? null;
}

/** The run with a session started for `task`. */
export function take(run: Run, task: Task, sessionId: string | null, now: number, tab: SessionTab | null = null): Run {
  return {
    ...run,
    taken: [...run.taken, task.id],
    current: { id: task.id, sessionId, tab, at: now, seenHeld: Boolean(task.holder), seen: seenOf(task), changed: now },
    checked: now,
    idleTold: false,
  };
}

/** What the monitor tells the person when it is stuck (Task-A00-eclair542, approver 2026-11-09):
 * a task its session parked waiting on a person, or no task ready. */
export type StuckAlert = { kind: "waiting"; id: string; on: string } | { kind: "idle" };

/** The task that just finished (`finished`, from [`follow`]) stopped waiting on a person: tell
 * them. A task waiting on someone named (`who`) is theirs, not the person's, so it is not told. */
export function waitingAlert(finished: string | null, tasks: Task[]): StuckAlert | null {
  if (!finished) return null;
  const task = tasks.find((t) => t.id === finished);
  if (task?.state !== "waiting" || task.who) return null;
  return { kind: "waiting", id: task.id, on: task.waiting_on ?? "" };
}

/** A check found no task to take: tell the person once for this stretch without one. Returns the
 * run marked as told, with the alert; or the run as it is and none when already told. */
export function idleAlert(run: Run): { run: Run; alert: StuckAlert | null } {
  if (run.idleTold) return { run, alert: null };
  return { run: { ...run, idleTold: true }, alert: { kind: "idle" } };
}

/** The current task's session has shown no sign of life for [`STALL_MS`]. */
export function stalled(run: Run, now: number): boolean {
  return run.current !== null && now - run.current.changed >= STALL_MS;
}

/** What the line beside the button keeps of a finished run. */
export interface LastRun {
  /** When it ended (ISO). */
  ended: string;
  scope: Scope;
  /** The tasks it started sessions for. */
  taken: string[];
}

/** The tasks of the last run that are done, and those that need the person now: waiting on them,
 * or done but for a person's check (`toCheck`: stopFlow's, a done task with no "by human" label,
 * decision 0020). Counted from the taskbag as it is now. */
export function lastRunCounts(last: LastRun, tasks: Task[], toCheck: (task: Task) => boolean): { done: number; needYou: number } {
  const mine = tasks.filter((t) => last.taken.includes(t.id));
  return {
    done: mine.filter((t) => t.state === "done").length,
    needYou: mine.filter((t) => (t.state === "waiting" && !t.who) || toCheck(t)).length,
  };
}

/** How the frequency is worded: in minutes, or in hours from 60 on (1.5 and 2 hours); singular
 * when it is a single minute or hour. */
export function frequencyText(every: Frequency): { unit: "minute" | "minutes" | "hour" | "hours"; n: number } {
  if (every >= 60) return { unit: every === 60 ? "hour" : "hours", n: every / 60 };
  return { unit: every === 1 ? "minute" : "minutes", n: every };
}

/** The name an agent passes to `alm claim --session-agent`. */
function sessionAgent(agent: TerminalAgent): string {
  return agent === "claude_cli" ? "claude-code" : agent === "antigravity_cli" ? "antigravity" : "codex";
}

/** How a terminal Claude Code session reads its quota (Task-A03-folk351): it has no `get_usage`
 * (only the Claude desktop app gives its sessions that), but `claude -p /usage` prints the
 * "Current session" and "Current week" lines, answered by Claude Code itself without the model.
 * Empty for the other agents. The paragraph ends with a blank line. */
function usageParagraph(agent: TerminalAgent, language: Language): string {
  if (agent !== "claude_cli") return "";
  if (language === "zh-CN") {
    return `配额：这个终端会话没有 get_usage。AGENTS.md 或 alm 要你报告配额时，运行 claude -p /usage（Claude Code 自己回答，不问模型），按它的两行报告：“Current session” 用 alm quota --agent claude-code --left <100 减已用的百分数> --resets <它的重置时间，本地时间，例如 2026-11-09T11:20> --window 5h；“Current week” 一样，用 --window week。\n\n`;
  }
  if (language === "ja") {
    return `クォータ：このターミナルのセッションには get_usage がありません。AGENTS.md や alm がクォータの報告を求めたら、claude -p /usage を実行し（Claude Code が自分で答え、モデルには聞きません）、その 2 行を報告してください：「Current session」は alm quota --agent claude-code --left <100 から使用済みの割合を引いた数> --resets <そのリセット時刻、ローカル時刻、例 2026-11-09T11:20> --window 5h、「Current week」は同じように --window week で。\n\n`;
  }
  return `Quota: this terminal session has no get_usage. When AGENTS.md or alm asks for your quota, run claude -p /usage (Claude Code answers it itself, without the model) and report its two lines: "Current session" with alm quota --agent claude-code --left <100 minus the percent used> --resets <its reset time, local, e.g. 2026-11-09T11:20> --window 5h, and "Current week" the same way with --window week.\n\n`;
}

/** The request a new session gets for one task, in the interface language. `taskbag` is the
 * taskbag's path as the agent, working in the project folder, passes it to `--taskbag`. */
export function monitorRequest(
  task: Task,
  taskbag: string,
  language: Language,
  agent: TerminalAgent,
  sessionId: string | null,
  quiet: boolean,
): string {
  const id = task.id;
  const tb = `--taskbag "${taskbag}"`;
  const sa = sessionAgent(agent);
  const usage = usageParagraph(agent, language);
  if (language === "zh-CN") {
    const sid = sessionId ?? "<你的会话 ID>";
    const mode = quiet
      ? `运行模式：安静模式（开始监测的人选的）。不要停下来等人：常规选择和用户看到的用词、样子都自己决定，做好、合入，并用 alm ai-decided 记下（能截图就附上），批准者在发布前审。只有数据格式、删掉东西、权限和安全、费用、对外影响、改变计划或其他任务的事才是真正的问题：带上你的提案，用 alm wait ${id} --on "<问题> 提案：<你的提案>" 搁置，然后结束这个会话；不依赖回答、已做完测完的工作先合入。工作都做完了、只剩人的亲手检查时，用 alm done ${id} --without-human-check 收尾（还有问题就用 alm ai-decided 记下，不再搁置）。`
      : "运行模式：普通模式（开始监测的人选的）。需要人时照常问。";
    return `请做这个项目任务袋里的任务 ${id}（“${task.title}”）。「换肩」正在自动监测就绪任务，为这一个任务开了这个会话。每条 alm 命令都加上 ${tb}。

${mode}

${usage}1. 阅读：alm show ${id}（它的说明和最新的交接）。
2. 按 AGENTS.md 的说法认领它，带上这个会话的信息：alm claim ${id} --as <你的会话名> --branch <你的分支> --session-agent ${sa} --session-id ${sid} --session-kind terminal，并把认领分享出去。
3. 在它自己的工作树里做，工作树从认领时列出的基础分支切出。
4. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。
5. 用 alm handoff ${id} 写交接，然后结束这个会话。只做这一个任务，不要去取下一个：「换肩」会为下一个任务另开会话。`;
  }
  if (language === "ja") {
    const sid = sessionId ?? "<あなたのセッション ID>";
    const mode = quiet
      ? `実行モード：静かなモード（監視を始めた人が選びました）。人を待って止まらないでください：いつもの選択も、利用者が目にする言葉や見た目も自分で決め、作ってマージし、alm ai-decided で記録してください（撮れるならスクリーンショット付き）。承認者はリリース前に見直します。本当の質問は、データの形式、何かの削除、権限とセキュリティ、費用、外部への影響、計画や他のタスクの変更だけです：あなたの案を添えて alm wait ${id} --on "<質問> 案：<あなたの案>" で保留にし、このセッションを終えてください。答えによらない、作業済みでテスト済みの部分は先にマージします。作業がすべて終わり、人の手による確認だけが残ったら alm done ${id} --without-human-check で終えてください（質問が残っていれば保留にせず alm ai-decided で記録します）。`
      : "実行モード：通常のモード（監視を始めた人が選びました）。人が必要なときはいつもどおり聞いてください。";
    return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）を進めてください。「肩代」が準備完了のタスクを自動で監視していて、このタスクひとつのためにこのセッションを開きました。alm コマンドはすべて ${tb} を付けて実行してください。

${mode}

${usage}1. 読む：alm show ${id}（説明と最新の引き継ぎ）。
2. AGENTS.md のとおりに、このセッションの情報を付けて引き受け（alm claim ${id} --as <あなたのセッション名> --branch <あなたのブランチ> --session-agent ${sa} --session-id ${sid} --session-kind terminal）、その引き受けを共有してください。
3. 専用のワークツリーで作業してください。ワークツリーは、引き受けたときに表示されるベースブランチから作ります。
4. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。
5. alm handoff ${id} で引き継ぎを書き、このセッションを終えてください。このタスクだけを進め、次のタスクは取らないでください：次のタスクには「肩代」が新しいセッションを開きます。`;
  }
  const sid = sessionId ?? "<your session's ID>";
  const mode = quiet
    ? `Run mode: quiet (chosen by the person who started the monitoring). Never wait for the person: decide routine choices yourself, and the wording and look users see too: build them, merge them and record them with alm ai-decided (with a screenshot when you can take one); the approver reviews them before release. Only data formats, removing things, permissions and security, cost, outside effects and changes to the plan or another task are real questions: park one with your proposal, alm wait ${id} --on "<the question> Proposal: <yours>", then end this session; merge first the work that is done and tested and does not depend on the answer. When all the work is done and only a person's hands-on check is left, finish with alm done ${id} --without-human-check (record any question left with alm ai-decided instead of parking it).`
    : "Run mode: normal (chosen by the person who started the monitoring). Ask the person as usual when you need them.";
  return `Work on task ${id} ("${task.title}") in this project's taskbag. Allomos is monitoring ready tasks and opened this session for this one task. Run every alm command with ${tb}.

${mode}

${usage}1. Read it: alm show ${id} (its description and latest handoff note).
2. Claim it as AGENTS.md says, with this session's details: alm claim ${id} --as <your session name> --branch <your branch> --session-agent ${sa} --session-id ${sid} --session-kind terminal, and share the claim.
3. Work it in a worktree of its own, cut from the base branch the claim prints.
4. Report progress with alm label; finish with alm done, or stop with alm release --note.
5. Write a handoff with alm handoff ${id}, then end this session. Work only this task; do not take another one: Allomos opens a new session for the next.`;
}

/** A task the monitor's sweep let go on (Task-A01-steam936): every blocker had finished. As
 * `release_finished` sends it. */
export type Released = { task: Task; blockers: string[] };

/** The line under the monitor's status bar after a sweep let tasks go on: one group per state they
 * went back to (ready, or in progress when someone holds them). `seen` is set once the check it
 * belongs to has run; the next check clears it. */
export type UnblockNotice = { groups: { state: "ready" | "in_progress"; ids: string[] }[]; seen: boolean };

/** What the sweep let go on, as the notice says it; null when it let nothing go on. */
export function unblockNotice(released: Released[]): UnblockNotice | null {
  const groups = (["ready", "in_progress"] as const)
    .map((state) => ({ state, ids: released.filter((r) => r.task.state === state).map((r) => r.task.id) }))
    .filter((g) => g.ids.length > 0);
  return groups.length ? { groups, seen: false } : null;
}

/** The notice after a check: the check it belongs to marks it seen; the next one clears it. */
export function noticeAfterCheck(notice: UnblockNotice | null): UnblockNotice | null {
  return notice && !notice.seen ? { ...notice, seen: true } : null;
}

/** The tasks as the monitor picks from them: a task the sweep let go on, while the taskbag as read
 * still shows it blocked (it is read again only when its files change), as the sweep left it. */
export function withReleased(tasks: Task[], released: Task[]): Task[] {
  if (!released.length) return tasks;
  return tasks.map((t) => (t.state === "blocked" ? (released.find((r) => r.id === t.id) ?? t) : t));
}
