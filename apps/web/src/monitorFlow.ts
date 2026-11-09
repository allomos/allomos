// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): one button starts unattended
// monitoring of the project's ready tasks, or one iteration's. Each ready task is worked in a new
// agent session; several agents can be chosen (Task-A03-fruit467). Since Task-A03-cow112
// (approver 2026-11-09) the run itself is a background `alm monitor run` process, started by the
// app's own `alm monitor start` or from a terminal, and goes on after the app is closed; the app
// only reads this computer's run files and leaves requests beside them. Kept apart from React so
// it can be tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node
// strips them.

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

/** The terminal tab Allomos opened for a session (macOS Terminal: its window and terminal device). */
export interface SessionTab {
  window: number;
  tty: string;
}

/** A session working a task for a run. */
export interface Current {
  /** The agent whose session it is (Task-A03-fruit467: a run may have several). */
  agent: TerminalAgent;
  id: string;
  /** The session ID Allomos gave the agent (Claude Code's `--session-id`); null when the agent
   * picks its own (Codex). */
  sessionId: string | null;
  tab: SessionTab | null;
  /** When the session was started (ms). */
  at: number;
  /** When its task last changed (ms): the sign of life [`stalled`] looks at. */
  changed: number;
}

/** A task the run's last sweep let go on (Task-A01-steam936): its blockers had all finished. */
export type Unblocked = { id: string; state: string };

/** One monitoring run, as its process last wrote it in its run file. */
export interface Run {
  /** The run's ID (`alm monitor status` shows it). */
  id: string;
  /** Its process. */
  pid: number;
  scope: Scope;
  agents: TerminalAgent[];
  /** How many sessions each agent may have at work at once; one when not listed (Task-A03-weasel276). */
  sessions: Partial<Record<TerminalAgent, number>>;
  /** The agents the person could choose when the run started, offered again in the status bar. */
  offered: TerminalAgent[];
  quiet: boolean;
  every: Frequency;
  started: number;
  /** The tasks it has started a session for, in order. */
  taken: string[];
  /** The agent each of those tasks was given to (task ID → agent). */
  takenBy: Record<string, TerminalAgent>;
  /** The sessions working a task now. */
  working: Current[];
  /** Stop asked while tasks are worked: the run ends when they are all finished. */
  stopping: boolean;
  /** When it last checked for a ready task (ms). */
  checked: number;
  /** When the next check is due (ms). */
  next: number;
  unblocked: Unblocked[];
}

/** An `alm monitor run` process this computer runs with no run file: running, record missing.
 * It cannot see a request; Stop ends it. */
export interface LostRun {
  pid: number;
  id: string | null;
}

const asAgent = (a: unknown): TerminalAgent | null =>
  typeof a === "string" && TERMINAL_AGENTS.includes(a) ? (a as TerminalAgent) : null;
const agents = (list: unknown): TerminalAgent[] =>
  Array.isArray(list) ? list.map(asAgent).filter((a): a is TerminalAgent => a !== null) : [];
const num = (n: unknown, or = 0): number => (typeof n === "number" ? n : or);
/** The sessions an agent may have at once, as the run file lists them: whole numbers of one or more. */
const sessionCounts = (list: unknown): Partial<Record<TerminalAgent, number>> => {
  const out: Partial<Record<TerminalAgent, number>> = {};
  if (list && typeof list === "object") {
    for (const [a, n] of Object.entries(list)) {
      const agent = asAgent(a);
      if (agent && typeof n === "number" && n >= 1) out[agent] = Math.floor(n);
    }
  }
  return out;
};
const takenByAgent = (map: unknown): Record<string, TerminalAgent> => {
  const out: Record<string, TerminalAgent> = {};
  if (map && typeof map === "object") {
    for (const [task, a] of Object.entries(map)) {
      const agent = asAgent(a);
      if (agent) out[task] = agent;
    }
  }
  return out;
};

/** One entry of `monitor_runs` (as `alm monitor status --json` lists it): a run, or a process
 * whose record is missing. `active` is the active iteration, to name the scope "current". */
export function readEntry(e: any, active: string | null | undefined): Run | LostRun | null {
  if (!e || typeof e !== "object") return null;
  if (e.record === "missing") return { pid: num(e.pid), id: typeof e.id === "string" ? e.id : null };
  const r = e.run;
  if (!r || typeof e.id !== "string") return null;
  const scope: Scope =
    r.scope?.kind === "iteration" && typeof r.scope.id === "string"
      ? { kind: "iteration", id: r.scope.id, current: r.scope.id === active }
      : { kind: "all" };
  const every = (FREQUENCIES as readonly number[]).includes(r.every) ? (r.every as Frequency) : FIRST_FREQUENCY;
  return {
    id: e.id,
    pid: num(e.pid),
    scope,
    agents: agents(r.agents),
    sessions: sessionCounts(r.sessions),
    offered: agents(r.offered),
    quiet: Boolean(r.quiet),
    every,
    started: num(r.started),
    taken: Array.isArray(r.taken) ? r.taken.filter((t: unknown) => typeof t === "string") : [],
    takenBy: takenByAgent(r.taken_by),
    working: (Array.isArray(r.working) ? r.working : [])
      .map((w: any): Current | null => {
        const agent = asAgent(w?.agent);
        if (!agent || typeof w.id !== "string") return null;
        return {
          agent,
          id: w.id,
          sessionId: typeof w.session_id === "string" ? w.session_id : null,
          tab: w.tab && typeof w.tab.window === "number" ? { window: w.tab.window, tty: String(w.tab.tty) } : null,
          at: num(w.at),
          changed: num(w.changed, num(w.at)),
        };
      })
      .filter((w: Current | null): w is Current => w !== null),
    stopping: Boolean(r.stopping),
    checked: num(r.checked),
    next: num(e.next_check, num(r.checked) + every * 60_000),
    unblocked: (Array.isArray(r.unblocked) ? r.unblocked : []).filter(
      (u: unknown): u is Unblocked => typeof (u as Unblocked)?.id === "string" && typeof (u as Unblocked)?.state === "string",
    ),
  };
}

export function isRun(e: Run | LostRun): e is Run {
  return "working" in e;
}

/** The agent list after adding or taking off one in the status bar (Task-A03-fruit467): an agent
 * added gets tasks from the next check on; one taken off finishes the task its session works. At
 * least one agent stays: an empty list leaves it as it is. Kept in the order they were offered. */
export function setAgents(run: Run, chosen: TerminalAgent[]): TerminalAgent[] {
  if (chosen.length === 0) return run.agents;
  const offered = run.offered.filter((a) => chosen.includes(a));
  return [...offered, ...chosen.filter((a) => !offered.includes(a))];
}

/** How many sessions an agent may have at work at once: one unless the person set more. */
export function sessionsOf(run: Run, agent: TerminalAgent): number {
  return Math.max(1, run.sessions[agent] ?? 1);
}

/** The free session slots: each agent once for every session it may still start (its count less
 * the sessions at work). Each can be given one task at the next check. */
export function freeAgents(run: Run): TerminalAgent[] {
  return run.agents.flatMap((a) => {
    const atWork = run.working.filter((w) => w.agent === a).length;
    return Array.from({ length: Math.max(0, sessionsOf(run, a) - atWork) }, () => a);
  });
}

/** When the next check is due (ms). */
export function nextCheck(run: Run): number {
  return run.next;
}

/** The sessions that have shown no sign of life for [`STALL_MS`]. */
export function stalled(run: Run, now: number): Current[] {
  return run.working.filter((w) => now - w.changed >= STALL_MS);
}

/** What the line beside the button keeps of a finished run. */
export interface LastRun {
  /** When it ended (ISO). */
  ended: string;
  scope: Scope;
  /** The tasks it started sessions for. */
  taken: string[];
  /** The agent each of those tasks was given to. */
  takenBy?: Record<string, TerminalAgent>;
}

/** The tasks of the last run that are done, and those that need the person now: waiting on them,
 * or done but for a person's check (`toCheck`: stopFlow's, a done task with no "by human" label,
 * decision 0020). Counted from the taskbag as it is now. */
export function lastRunCounts(
  last: LastRun,
  tasks: Task[],
  toCheck: (task: Task) => boolean,
): { done: number; needYou: number; byAgent: { agent: TerminalAgent; done: number }[] } {
  const mine = tasks.filter((t) => last.taken.includes(t.id));
  const done = mine.filter((t) => t.state === "done");
  // The done tasks per agent that gave them out (Task-A03-weasel276), in the order they were taken.
  const byAgent = new Map<TerminalAgent, number>();
  for (const id of last.taken) {
    const agent = last.takenBy?.[id];
    if (agent && done.some((t) => t.id === id)) byAgent.set(agent, (byAgent.get(agent) ?? 0) + 1);
  }
  return {
    done: done.length,
    needYou: mine.filter((t) => (t.state === "waiting" && !t.who) || toCheck(t)).length,
    byAgent: [...byAgent].map(([agent, n]) => ({ agent, done: n })),
  };
}

/** How the frequency is worded: in minutes, or in hours from 60 on (1.5 and 2 hours); singular
 * when it is a single minute or hour. */
export function frequencyText(every: Frequency): { unit: "minute" | "minutes" | "hour" | "hours"; n: number } {
  if (every >= 60) return { unit: every === 60 ? "hour" : "hours", n: every / 60 };
  return { unit: every === 1 ? "minute" : "minutes", n: every };
}

/** The line under the monitor's status bar after a sweep let tasks go on: one group per state they
 * went back to (ready, or in progress when someone holds them). */
export type UnblockNotice = { groups: { state: "ready" | "in_progress"; ids: string[] }[] };

/** What the run's last sweep let go on, as the notice says it; null when it let nothing go on. */
export function unblockNotice(unblocked: Unblocked[]): UnblockNotice | null {
  const groups = (["ready", "in_progress"] as const)
    .map((state) => ({ state, ids: unblocked.filter((r) => r.state === state).map((r) => r.id) }))
    .filter((g) => g.ids.length > 0);
  return groups.length ? { groups } : null;
}

// The Monitors list (Task-A03-motor529, approver 2026-11-09): the rows of `monitor_all`, every run
// this computer runs in every project (see MonitorsList.tsx).

/** A taskbag's project folder: the taskbag's path without its `.allomos/taskbag` (as paths.ts). */
function taskbagFolder(root: string): string {
  return root.replace(/[\\/]\.allomos[\\/]taskbag[\\/]?$/, "");
}

/** One row of the list: a run with its file, or a process whose record is missing. */
export interface ListedRun {
  /** The row's key: the run's ID, or the process number when no ID is known. */
  key: string;
  /** The run, when its file is there. */
  run: Run | null;
  /** The process whose record is missing (no file: only its process is known). */
  lost: LostRun | null;
  /** The project's folder name (its last folder), when the run names one. */
  name: string | null;
  /** The project's taskbag, which Open shows; null when the run names none. */
  root: string | null;
}

/** The last folder of `path`, with either kind of separator; null for an empty path. */
export function folderName(path: string): string | null {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : null;
}

/** The rows of `monitor_all` (oldest run first, as it lists them). An entry that is not a run or a
 * process is left out. The active iteration is not needed here: each row names its iteration. */
export function listedRuns(entries: unknown[]): ListedRun[] {
  const rows: ListedRun[] = [];
  for (const e of entries) {
    const parsed = readEntry(e, null);
    if (!parsed) continue;
    const obj = e as { project?: unknown; taskbag?: unknown };
    const taskbag = typeof obj.taskbag === "string" && obj.taskbag ? obj.taskbag : null;
    const project = typeof obj.project === "string" && obj.project ? obj.project : taskbag ? taskbagFolder(taskbag) : null;
    if (isRun(parsed)) {
      rows.push({ key: parsed.id, run: parsed, lost: null, name: project ? folderName(project) : null, root: taskbag });
    } else {
      rows.push({ key: parsed.id ?? `pid:${parsed.pid}`, run: null, lost: parsed, name: taskbag ? folderName(taskbagFolder(taskbag)) : null, root: taskbag });
    }
  }
  return rows;
}

