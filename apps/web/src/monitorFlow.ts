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
export const TERMINAL_AGENTS: readonly string[] = ["claude_cli", "codex_cli", "antigravity_cli"];

/** A monitor agent: an agent (`claude_cli`), or one of its extra accounts (`claude_cli:<account
 * ID>`, Task-A03-oven548). Each one is a row of its own in the dialog and gets its own sessions
 * and difficulty range. */
export type TerminalAgent = string;

/** An extra account of an agent, as the app lists it: its name and the folder it keeps its
 * settings and login in (never the login itself). */
export interface AgentAccount {
  id: string;
  agent: string;
  name: string;
  dir: string;
}

/** The app's accounts list: the extra accounts, and each agent's default folder. */
export interface AgentAccounts {
  accounts: AgentAccount[];
  defaults: Record<string, string>;
}

/** The agent of a monitor agent: `claude_cli` for `claude_cli:<account>` too. */
export function baseAgent(key: string): string {
  const at = key.indexOf(":");
  return at < 0 ? key : key.slice(0, at);
}

/** A monitor agent of an account: `<agent>:<account ID>`. */
export function accountKey(agent: string, id: string): string {
  return `${agent}:${id}`;
}

/** Whether a monitor agent is one Allomos starts in a terminal (an account of one counts too). */
export function isTerminalAgent(key: string): boolean {
  return TERMINAL_AGENTS.includes(baseAgent(key));
}

/** One row of an agent's accounts in the dialog: the monitor agent, and the account's name when
 * the agent has extra accounts (`null`: the agent alone, as before accounts existed). */
export interface AccountRow {
  key: TerminalAgent;
  name: string | null;
}

/** The rows for an agent: one when it has no extra account; otherwise the default account first
 * (named `firstName`, the person's `Account 1`), then each extra account in the order added. */
export function accountRows(agent: string, accounts: AgentAccount[], firstName: string): AccountRow[] {
  const extra = accounts.filter((a) => a.agent === agent);
  if (extra.length === 0) return [{ key: agent, name: null }];
  return [{ key: agent, name: firstName }, ...extra.map((a) => ({ key: accountKey(agent, a.id), name: a.name }))];
}

/** The name suggested for a new account: `Account 2` for the first one beside the default, then
 * one more each time (`Account 3`, …). */
export function suggestedAccountName(accounts: AgentAccount[], agent: string): string {
  return `Account ${accounts.filter((a) => a.agent === agent).length + 2}`;
}

/** The label of a monitor agent: its agent's name, and the account's name after a dot. */
export function agentLabel(agentName: string, accountName: string | null): string {
  return accountName ? `${agentName} · ${accountName}` : agentName;
}

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
  holds: (task: Task) => boolean = () => true,
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
    .filter(holds)
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
  holds: (task: Task) => boolean = () => true,
): ScopeOptions {
  const count = (id?: string) => readyTasks(tasks, areas, iterations, active, id, holds).length;
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
  /** Its session has claimed the task (A03-otter944): until then it is starting, not working. */
  claimed: boolean;
  /** The former holder of the lost task this session takes over (A03-husky861); null for a ready
   * task. Until it claims, the task is still that holder's, not this session's. */
  takeoverFrom: string | null;
  /** Since when (ms) the session shows a confirmation and waits for the person (Task-A03-candle790):
   * it does not take a slot and is not stalled while it waits; null when it shows none. */
  awaitingApproval: number | null;
}

/** Why an agent gets no session for now (A03-otter944): its quota is used up, or its last
 * session did not start for no reason the monitor could read. */
export type PauseWhy = "quota" | "no_claim";

/** An agent given no session for now. */
export interface Pause {
  agent: TerminalAgent;
  why: PauseWhy;
  /** When its quota comes back, as the agent wrote it, when it said. */
  resets: string | null;
  /** When it is looked at again (ms): its quota read again, or tried again. */
  until: number;
}

/** A session that did not start: it never claimed its task, which is given out again. */
export interface NotStarted {
  agent: TerminalAgent;
  id: string;
  /** When it was found not to have started (ms). */
  found: number;
  why: PauseWhy;
  resets: string | null;
}

/** A session the run let go of because its task went lost while it held it (A03-husky861): no
 * heartbeat past the project's limit. Its slot is free and its task goes to a new session. */
export interface LostSession {
  agent: TerminalAgent;
  id: string;
  /** Its `--as` name. */
  holder: string;
  /** When it was found lost (ms). */
  found: number;
  /** Its tab said its quota is used up, and when it comes back, when it said. */
  quota: boolean;
  resets: string | null;
}

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
  /** The difficulty levels each agent takes tasks from; every level (1–5) when not listed (Task-A03-wave335). */
  difficulties: Partial<Record<TerminalAgent, Levels>>;
  /** The agents with a free session slot that no ready task in their levels fitted at the last
   * check: they got no session (Task-A03-wave335). */
  noTask: TerminalAgent[];
  /** The tasks it let go because another session holds them, the latest few (A03-cart761):
   * their rows are gone and their slots free; the panel says why. */
  letGo: LetGo[];
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
  /** The taskbag cannot be read now (Task-A03-raven398): the run goes on trying, and says why. */
  unreadable?: Unreadable;
  /** The `alm` version the app moved this run to after an update (Task-A03-cookie729), when it did. */
  updatedTo?: string;
  /** The agents given no session for now (A03-otter944). */
  paused: Pause[];
  /** The sessions that did not start, newest last (A03-otter944). */
  notStarted: NotStarted[];
  /** The sessions found lost, newest last (A03-husky861). */
  lost: LostSession[];
  /** The tasks given up after their sessions failed several times: no more sessions this run. */
  givenUp: string[];
}

/** Why a run cannot read its taskbag, as `alm monitor run` keeps it (crates/monitor/src/unreadable.rs). */
export type Unreadable = "taskbag_format" | "taskbag_unreadable";

/** An `alm monitor run` process this computer runs with no run file: running, record missing.
 * It cannot see a request; Stop ends it. */
export interface LostRun {
  pid: number;
  id: string | null;
}

const asAgent = (a: unknown): TerminalAgent | null => (typeof a === "string" && isTerminalAgent(a) ? a : null);
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
/** A difficulty range, as the run file writes it: `1-3`, or one level `4` (Task-A03-wave335). */
export interface Levels {
  low: number;
  high: number;
}
/** Every level, 1 to 5: the range an agent has when none is chosen. */
export const ALL_LEVELS: Levels = { low: 1, high: 5 };
/** The levels of a range as text, as `alm monitor` and the run file write them: `1-5`, or `4`. */
export const levelsText = (l: Levels): string => (l.low === l.high ? String(l.low) : `${l.low}-${l.high}`);
/** Reads a range the run file lists; null when it is not `low-high` (or one level) of 1 to 5. */
export const parseLevels = (text: unknown): Levels | null => {
  const m = typeof text === "string" ? /^([1-5])(?:-([1-5]))?$/.exec(text.trim()) : null;
  if (!m) return null;
  const low = Number(m[1]);
  const high = m[2] === undefined ? low : Number(m[2]);
  return low <= high ? { low, high } : null;
};
const difficultyRanges = (map: unknown): Partial<Record<TerminalAgent, Levels>> => {
  const out: Partial<Record<TerminalAgent, Levels>> = {};
  if (map && typeof map === "object") {
    for (const [a, text] of Object.entries(map)) {
      const agent = asAgent(a);
      const levels = parseLevels(text);
      if (agent && levels) out[agent] = levels;
    }
  }
  return out;
};
const pauseWhy = (w: unknown): PauseWhy => (w === "quota" ? "quota" : "no_claim");
const text = (v: unknown): string | null => (typeof v === "string" && v ? v : null);
const pauses = (map: unknown): Pause[] => {
  const out: Pause[] = [];
  if (map && typeof map === "object") {
    for (const [a, p] of Object.entries(map as Record<string, any>)) {
      const agent = asAgent(a);
      if (agent && p && typeof p === "object") out.push({ agent, why: pauseWhy(p.why), resets: text(p.resets), until: num(p.until) });
    }
  }
  return out;
};
const notStartedList = (list: unknown): NotStarted[] =>
  (Array.isArray(list) ? list : [])
    .map((n: any): NotStarted | null => {
      const agent = asAgent(n?.agent);
      if (!agent || typeof n.id !== "string") return null;
      return { agent, id: n.id, found: num(n.found), why: pauseWhy(n.why), resets: text(n.resets) };
    })
    .filter((n): n is NotStarted => n !== null);
const lostList = (list: unknown): LostSession[] =>
  (Array.isArray(list) ? list : [])
    .map((n: any): LostSession | null => {
      const agent = asAgent(n?.agent);
      if (!agent || typeof n.id !== "string") return null;
      return { agent, id: n.id, holder: typeof n.holder === "string" ? n.holder : "", found: num(n.found), quota: n.quota === true, resets: text(n.resets) };
    })
    .filter((n): n is LostSession => n !== null);
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

/** A task a run stopped following because another session holds it (A03-cart761). */
export interface LetGo {
  id: string;
  /** The agent whose session the run had started for it. */
  agent: TerminalAgent;
  /** Who holds it now (the other session's name). */
  holder: string;
  /** When the run stopped following this session (ms). */
  at: number;
}

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
    difficulties: difficultyRanges(r.difficulties),
    noTask: agents(r.no_task),
    letGo: (Array.isArray(r.let_go) ? r.let_go : [])
      .map((l: any): LetGo | null => {
        const agent = asAgent(l?.agent);
        return agent && typeof l.id === "string" && typeof l.holder === "string" ? { id: l.id, agent, holder: l.holder, at: num(l.at) } : null;
      })
      .filter((l: LetGo | null): l is LetGo => l !== null),
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
          // A run file from before A03-otter944 does not say: taken as claimed, as it was shown.
          claimed: w.seen_held !== false,
          takeoverFrom: typeof w.prior === "string" ? w.prior : null,
          awaitingApproval: typeof w.awaiting_approval === "number" ? w.awaiting_approval : null,
        };
      })
      .filter((w: Current | null): w is Current => w !== null),
    stopping: Boolean(r.stopping),
    checked: num(r.checked),
    next: num(e.next_check, num(r.checked) + every * 60_000),
    // An unknown reason (a newer alm's) still says the taskbag cannot be read.
    ...(typeof r.unreadable === "string" ? { unreadable: r.unreadable === "taskbag_format" ? ("taskbag_format" as const) : ("taskbag_unreadable" as const) } : {}),
    ...(typeof e.program?.updated_from === "string" && typeof e.program?.version === "string" ? { updatedTo: e.program.version } : {}),
    paused: pauses(r.paused),
    notStarted: notStartedList(r.not_started),
    lost: lostList(r.lost),
    givenUp: Array.isArray(r.given_up) ? r.given_up.filter((t: unknown) => typeof t === "string") : [],
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

/** The difficulty levels an agent takes tasks from: every level unless the person chose some. */
export function levelsOf(run: Run, agent: TerminalAgent): Levels {
  return run.difficulties[agent] ?? ALL_LEVELS;
}

/** The free session slots: each agent once for every session it may still start (its count less
 * the sessions at work). Each can be given one task at the next check. */
export function freeAgents(run: Run): TerminalAgent[] {
  return run.agents.filter((a) => !run.paused.some((p) => p.agent === a)).flatMap((a) => {
    const atWork = run.working.filter((w) => w.agent === a && w.awaitingApproval === null).length;
    return Array.from({ length: Math.max(0, sessionsOf(run, a) - atWork) }, () => a);
  });
}

/** When the next check is due (ms). */
export function nextCheck(run: Run): number {
  return run.next;
}

/** The sessions that have shown no sign of life for [`STALL_MS`]. */
export function stalled(run: Run, now: number): Current[] {
  return run.working.filter((w) => w.awaitingApproval === null && now - w.changed >= STALL_MS);
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

/** A last run as this computer keeps it in its file (A03-pocket537): `ended` in ms, the rest as
 * the app's line reads it. `null` when the file does not fit. */
export function lastRunFromFile(v: unknown): LastRun | null {
  if (!v || typeof v !== "object") return null;
  const o = v as { ended?: unknown; scope?: unknown; taken?: unknown; takenBy?: unknown };
  if (typeof o.ended !== "number" || !Number.isFinite(o.ended) || !Array.isArray(o.taken) || !o.scope) return null;
  return {
    ended: new Date(o.ended).toISOString(),
    scope: o.scope as Scope,
    taken: o.taken.filter((x): x is string => typeof x === "string"),
    takenBy: (o.takenBy && typeof o.takenBy === "object" ? o.takenBy : undefined) as Record<string, TerminalAgent> | undefined,
  };
}

/** The later of two last runs (the one remembered by the window and the one kept on this
 * computer, which also counts a run that ended while the app was closed). */
export function newerLast(a: LastRun | null, b: LastRun | null): LastRun | null {
  if (!a) return b;
  if (!b) return a;
  return Date.parse(b.ended) > Date.parse(a.ended) ? b : a;
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
  for (const id of new Set(last.taken)) {
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


// A03-yolk481 (approver 2026-11-12): a running monitor is edited with one Edit monitor… dialog
// (the start dialog, filled in), and the iteration card shows an Auto-monitor section: one
// foldable panel per monitor, each with its sessions grouped by agent.

/** What a monitor panel's badge says: only one per monitor, the first that holds in this order
 * (approver 2026-11-12): a session quiet for long, a task waiting on the person, stopping, running. */
export type RunState = "stalled" | "need_you" | "stopping" | "running";
export const RUN_STATES: readonly RunState[] = ["stalled", "need_you", "stopping", "running"];

/** The tasks of `run` waiting on the person (not on someone named): they need you. */
export function needYou(run: Run, tasks: Task[]): string[] {
  const parked = [...new Set(run.taken)].filter((id) => tasks.some((t) => t.id === id && t.state === "waiting" && !t.who));
  // A session at a confirmation waits for the person too (Task-A03-candle790).
  const approving = run.working.filter((w) => w.awaitingApproval !== null && !parked.includes(w.id)).map((w) => w.id);
  return [...parked, ...approving];
}

export function runState(run: Run, tasks: Task[], now: number): RunState {
  if (stalled(run, now).length > 0) return "stalled";
  if (needYou(run, tasks).length > 0) return "need_you";
  if (run.stopping) return "stopping";
  return "running";
}

/** The section title's counts: one per state, in [`RUN_STATES`]' order, those of zero left out. */
export function stateCounts(states: RunState[]): { state: RunState; n: number }[] {
  return RUN_STATES.map((state) => ({ state, n: states.filter((s) => s === state).length })).filter((c) => c.n > 0);
}

/** One agent's group under "Sessions this monitor opened": its sessions at work, how many it may
 * have, its levels; `removed` when it was taken off the monitor and finishes its task;
 * `noTask` when it had a free slot and no ready task in its levels at the last check. */
export interface SessionGroup {
  agent: TerminalAgent;
  removed: boolean;
  working: Current[];
  sessions: number;
  levels: Levels;
  noTask: boolean;
  /** Given no session for now (A03-otter944), and why. */
  paused: Pause | null;
}

/** The groups, one per agent of the run (in its order), then the agents taken off that still
 * finish a task. */
export function sessionGroups(run: Run): SessionGroup[] {
  const removed = run.working.map((w) => w.agent).filter((a, i, all) => !run.agents.includes(a) && all.indexOf(a) === i);
  return [...run.agents, ...removed].map((agent) => ({
    agent,
    removed: !run.agents.includes(agent),
    working: run.working.filter((w) => w.agent === agent),
    sessions: sessionsOf(run, agent),
    levels: levelsOf(run, agent),
    noTask: run.noTask.includes(agent),
    paused: run.paused.find((p) => p.agent === agent) ?? null,
  }));
}

/** What became of a not-started session's task (A03-throne493): this run will give it out again,
 * gave it up after several failures, or will not give it out again (stopping, blocked, or outside
 * the run's scope or difficulty range). */
export type NotStartedState = "again" | "given_up" | "not_again";

const parseUtc = (stamp: string | null | undefined): number | null => {
  if (!stamp) return null;
  const ms = new Date(stamp.replace(/T(\d\d:\d\d)Z$/, "T$1:00Z")).getTime();
  return Number.isNaN(ms) ? null : ms;
};

/** Whether `task` is in progress with a holder whose heartbeat has passed the lost limit and has
 * no busy window running (mirrors `heartbeatFlow.ts` and `crates/core/src/heartbeat.rs`). */
function isLostTask(task: Task, now: number, lostAfterMinutes = 30): boolean {
  if (task.state !== "in_progress" || !task.holder) return false;
  const mine = task.heartbeat && task.heartbeat.by === task.holder ? task.heartbeat : null;
  const at = parseUtc(mine?.at);
  const until = parseUtc(mine?.busy_until);
  if (until !== null && until > now) return false;
  const since = at !== null || until !== null ? Math.max(at ?? -Infinity, until ?? -Infinity) : null;
  return since !== null && now - since > lostAfterMinutes * 60_000;
}

function isDispatchable(task: Task, now: number, lostAfterMinutes = 30): boolean {
  return (task.state === "ready" && !task.holder) || isLostTask(task, now, lostAfterMinutes);
}

function willDispatchAgain(run: Run, task: Task): boolean {
  if (run.stopping) return false;
  if (run.scope.kind === "iteration" && task.iteration !== run.scope.id) return false;
  if ((task.open_blockers?.length ?? 0) > 0) return false;
  const d = task.difficulty ?? 3;
  return run.agents.some((a) => {
    const l = levelsOf(run, a);
    return d >= l.low && d <= l.high;
  });
}

/** The sessions that did not start whose task is still open to be given out (ready and unheld, or
 * lost) and not worked by a session of this run now, newest first, each task once (A03-otter944,
 * A03-throne493). */
export function notStartedShown(
  run: Run,
  tasks: Task[],
  now: number,
  lostAfterMinutes = 30,
): { item: NotStarted; state: NotStartedState }[] {
  return [...run.notStarted]
    .reverse()
    .filter((n, i, all) => !run.working.some((w) => w.id === n.id) && all.findIndex((m) => m.id === n.id) === i)
    .flatMap((item) => {
      const task = tasks.find((t) => t.id === item.id);
      if (!task || !isDispatchable(task, now, lostAfterMinutes)) return [];
      const state: NotStartedState = run.givenUp.includes(item.id)
        ? "given_up"
        : willDispatchAgain(run, task)
          ? "again"
          : "not_again";
      return [{ item, state }];
    });
}

/** A local time as the panel shows it: `HH:MM` on the same local day as `now`, otherwise
 * `YYYY-MM-DD HH:MM` so records from an earlier day are never mistaken for today (A03-throne493). */
export function dayTimeText(ms: number, now: number): string {
  const d = new Date(ms);
  const cur = new Date(now);
  const p = (n: number) => String(n).padStart(2, "0");
  const clock = `${p(d.getHours())}:${p(d.getMinutes())}`;
  const sameDay = d.getFullYear() === cur.getFullYear() && d.getMonth() === cur.getMonth() && d.getDate() === cur.getDate();
  return sameDay ? clock : `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${clock}`;
}

/** What became of a lost session's task (A03-husky861): a new session of the run works it now,
 * it waits to be given out again (its agent's quota may first have to come back), or the run gave
 * it up after several failures. */
export type LostState = "taken_over" | "waiting" | "given_up";

/** The lost sessions, newest first and each task once, with what became of their tasks: the
 * panel lists them under the sessions. */
export function lostShown(run: Run): { lost: LostSession; state: LostState }[] {
  return [...run.lost]
    .reverse()
    .filter((n, i, all) => all.findIndex((m) => m.id === n.id) === i)
    .map((lost) => ({
      lost,
      state: run.givenUp.includes(lost.id) ? "given_up" : run.working.some((w) => w.id === lost.id) ? "taken_over" : "waiting",
    }));
}

/** Where a stopped task's one Requeue button goes (A03-gadget373): on its row under Lost
 * sessions, else on its row under Did not start, else (no row lists it) on a line of its own. */
export type RequeuePlace = "lost" | "not_started" | "own";

export function requeuePlace(
  run: Run,
  id: string,
  tasks: Task[],
  now: number,
  lostAfterMinutes = 30,
): RequeuePlace {
  if (run.lost.some((l) => l.id === id)) return "lost";
  if (notStartedShown(run, tasks, now, lostAfterMinutes).some(({ item }) => item.id === id)) return "not_started";
  return "own";
}

/** Whether the sessions list is grouped by agent: more than one agent, or one that may have
 * several sessions. One agent with one session lists its row alone. */
export function grouped(run: Run): boolean {
  const groups = sessionGroups(run);
  return groups.length > 1 || groups.some((g) => g.sessions > 1);
}

/** What the Edit monitor… dialog saves: the settings as the person left them. */
export interface MonitorSettings {
  scope: Scope;
  agents: TerminalAgent[];
  quiet: boolean;
  every: Frequency;
  sessions: Partial<Record<TerminalAgent, number>>;
  difficulties: Partial<Record<TerminalAgent, Levels>>;
}

const sameScope = (a: Scope, b: Scope) => a.kind === b.kind && (a.kind === "all" || (b.kind === "iteration" && a.id === b.id));

/** The requests that turn `run` into `next`, only for what changed, in the order the run takes
 * them: which tasks, the mode, the frequency, the agents, then each agent's sessions and levels.
 * No agent left leaves the agents as they are (Stop ends a run). */
export function editRequests(run: Run, next: MonitorSettings): MonitorRequestOf[] {
  const out: MonitorRequestOf[] = [];
  if (!sameScope(run.scope, next.scope)) {
    out.push({ action: "scope", scope: next.scope.kind === "all" ? { kind: "all" } : { kind: "iteration", id: next.scope.id } });
  }
  if (run.quiet !== next.quiet) out.push({ action: "mode", quiet: next.quiet });
  if (run.every !== next.every) out.push({ action: "every", minutes: next.every });
  const agents = next.agents.length ? setAgents(run, next.agents) : run.agents;
  if (agents.join("\n") !== run.agents.join("\n")) out.push({ action: "agents", agents });
  for (const a of agents) {
    const count = Math.max(1, Math.floor(next.sessions[a] ?? 1));
    if (count !== sessionsOf(run, a)) out.push({ action: "sessions", agent: a, count });
    const l = next.difficulties[a] ?? ALL_LEVELS;
    const was = levelsOf(run, a);
    if (l.low !== was.low || l.high !== was.high) out.push({ action: "difficulty", agent: a, range: levelsText(l) });
  }
  return out;
}

/** A request for a run, as `monitor_ask` takes it (messenger.ts's `MonitorRequest`; repeated here
 * so this file needs no import that Node cannot strip). */
export type MonitorRequestOf =
  | { action: "scope"; scope: { kind: "all" } | { kind: "iteration"; id: string } }
  | { action: "mode"; quiet: boolean }
  | { action: "every"; minutes: number }
  | { action: "agents"; agents: string[] }
  | { action: "sessions"; agent: string; count: number }
  | { action: "difficulty"; agent: string; range: string };

/** How long a session has been at work, in whole minutes, hours past 60: `{ unit, n }`. */
export function workedFor(at: number, now: number): { unit: "minutes" | "hours"; n: number } {
  const minutes = Math.max(0, Math.floor((now - at) / 60_000));
  return minutes >= 60 ? { unit: "hours", n: Math.floor(minutes / 60) } : { unit: "minutes", n: minutes };
}

/** What this computer remembers of the section's folding, per project: whether the section is
 * folded, and which monitor panels are open (panels start folded, the section open). */
export interface MonitorFolds {
  sectionFolded: boolean;
  open: string[];
}
export const NO_FOLDS: MonitorFolds = { sectionFolded: false, open: [] };

/** Reads what was stored; anything that does not fit is the default. */
export function readFolds(text: string | null): MonitorFolds {
  try {
    const v = text ? JSON.parse(text) : null;
    if (!v || typeof v !== "object") return NO_FOLDS;
    return {
      sectionFolded: v.sectionFolded === true,
      open: Array.isArray(v.open) ? v.open.filter((x: unknown): x is string => typeof x === "string") : [],
    };
  } catch {
    return NO_FOLDS;
  }
}

/** The folds kept: only the panels of runs still going (a run's ID is never used again). */
export function keptFolds(folds: MonitorFolds, runIds: string[]): MonitorFolds {
  return { sectionFolded: folds.sectionFolded, open: folds.open.filter((id) => runIds.includes(id)) };
}

/** Whether a task ID shown in a monitor panel can open a detail pane in this project (A03-scale435):
 * only tasks that still exist in the project's task list get a clickable link. */
export function canOpenMonitorTask(id: string, tasks: readonly Pick<Task, "id">[]): boolean {
  return tasks.some((t) => t.id === id);
}

/** Splits a translated phrase around a placeholder where one or more task IDs are rendered as
 * interactive links (A03-scale435). Falls back to `[phrase, ""]` if the placeholder is absent. */
export function splitTaskIdPhrase(phrase: string, placeholder: string): [string, string] {
  const at = phrase.indexOf(placeholder);
  if (at < 0) return [phrase, ""];
  return [phrase.slice(0, at), phrase.slice(at + placeholder.length)];
}
