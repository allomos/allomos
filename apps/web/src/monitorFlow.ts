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
export type TerminalAgent = "claude_cli" | "codex_cli";
export const TERMINAL_AGENTS: readonly string[] = ["claude_cli", "codex_cli"];

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

/** The task a session is working for the monitor. */
export interface Current {
  id: string;
  /** The session ID Allomos gave the agent (Claude Code's `--session-id`); null when the agent
   * picks its own (Codex). */
  sessionId: string | null;
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
export function take(run: Run, task: Task, sessionId: string | null, now: number): Run {
  return {
    ...run,
    taken: [...run.taken, task.id],
    current: { id: task.id, sessionId, at: now, seenHeld: Boolean(task.holder), seen: seenOf(task), changed: now },
    checked: now,
  };
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
  return agent === "claude_cli" ? "claude-code" : "codex";
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
  if (language === "zh-CN") {
    const sid = sessionId ?? "<你的会话 ID>";
    const mode = quiet
      ? `运行模式：安静模式（开始监测的人选的）。不要停下来等人：常规选择自己决定并用 alm ai-decided 记下；真正的问题用 alm wait ${id} --on "<问题>" 搁置，然后结束这个会话。只剩人的亲手检查时，用 alm done ${id} --without-human-check。`
      : "运行模式：普通模式（开始监测的人选的）。需要人时照常问。";
    return `请做这个项目任务袋里的任务 ${id}（“${task.title}”）。「换肩」正在自动监测就绪任务，为这一个任务开了这个会话。每条 alm 命令都加上 ${tb}。

${mode}

1. 阅读：alm show ${id}（它的说明和最新的交接）。
2. 按 AGENTS.md 的说法认领它，带上这个会话的信息：alm claim ${id} --as <你的会话名> --branch <你的分支> --session-agent ${sa} --session-id ${sid} --session-kind terminal，并把认领分享出去。
3. 在它自己的工作树里做，工作树从认领时列出的基础分支切出。
4. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。
5. 用 alm handoff ${id} 写交接，然后结束这个会话。只做这一个任务，不要去取下一个：「换肩」会为下一个任务另开会话。`;
  }
  if (language === "ja") {
    const sid = sessionId ?? "<あなたのセッション ID>";
    const mode = quiet
      ? `実行モード：静かなモード（監視を始めた人が選びました）。人を待って止まらないでください：いつもの選択は自分で決めて alm ai-decided で記録し、本当の質問は alm wait ${id} --on "<質問>" で保留にして、このセッションを終えてください。人の手による確認だけが残ったら alm done ${id} --without-human-check を使ってください。`
      : "実行モード：通常のモード（監視を始めた人が選びました）。人が必要なときはいつもどおり聞いてください。";
    return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）を進めてください。「肩代」が準備完了のタスクを自動で監視していて、このタスクひとつのためにこのセッションを開きました。alm コマンドはすべて ${tb} を付けて実行してください。

${mode}

1. 読む：alm show ${id}（説明と最新の引き継ぎ）。
2. AGENTS.md のとおりに、このセッションの情報を付けて引き受け（alm claim ${id} --as <あなたのセッション名> --branch <あなたのブランチ> --session-agent ${sa} --session-id ${sid} --session-kind terminal）、その引き受けを共有してください。
3. 専用のワークツリーで作業してください。ワークツリーは、引き受けたときに表示されるベースブランチから作ります。
4. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。
5. alm handoff ${id} で引き継ぎを書き、このセッションを終えてください。このタスクだけを進め、次のタスクは取らないでください：次のタスクには「肩代」が新しいセッションを開きます。`;
  }
  const sid = sessionId ?? "<your session's ID>";
  const mode = quiet
    ? `Run mode: quiet (chosen by the person who started the monitoring). Never wait for the person: decide routine choices yourself and record them with alm ai-decided; park a real question with alm wait ${id} --on "<the question>", then end this session. When only a person's hands-on check is left, finish with alm done ${id} --without-human-check.`
    : "Run mode: normal (chosen by the person who started the monitoring). Ask the person as usual when you need them.";
  return `Work on task ${id} ("${task.title}") in this project's taskbag. Allomos is monitoring ready tasks and opened this session for this one task. Run every alm command with ${tb}.

${mode}

1. Read it: alm show ${id} (its description and latest handoff note).
2. Claim it as AGENTS.md says, with this session's details: alm claim ${id} --as <your session name> --branch <your branch> --session-agent ${sa} --session-id ${sid} --session-kind terminal, and share the claim.
3. Work it in a worktree of its own, cut from the base branch the claim prints.
4. Report progress with alm label; finish with alm done, or stop with alm release --note.
5. Write a handoff with alm handoff ${id}, then end this session. Work only this task; do not take another one: Allomos opens a new session for the next.`;
}
