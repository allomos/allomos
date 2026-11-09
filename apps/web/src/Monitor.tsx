// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): what shows in place of its button
// while runs go on. Since Task-A03-cow112 (approver 2026-11-09) a run is a background `alm monitor
// run` process of this computer, started here with the app's own `alm monitor start` or from a
// terminal, and it goes on after the app is closed: the app runs no loop of its own. It reads the
// project's run files every few seconds and shows a status bar for each run (a project may have
// several); Stop, Check now and the menus leave a request for the run. The last run's summary is
// remembered per project for the line beside the button.

import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger, MonitorRequest } from "./messenger";
import { monitorAgentName } from "./AgentPicker";
import { localTime } from "./timeFlow";
import { sessionPlace } from "./sessionFlow";
import { toCheck } from "./stopFlow";
import {
  FREQUENCIES,
  freeAgents,
  frequencyText,
  isRun,
  lastRunCounts,
  levelsOf,
  levelsText,
  nextCheck,
  readEntry,
  setAgents,
  sessionsOf,
  stalled,
  unblockNotice,
} from "./monitorFlow";
import type { Frequency, LastRun, LostRun, Levels, Run, Scope, TerminalAgent, UnblockNotice } from "./monitorFlow";
import { LevelsPicker } from "./MonitorDialog";
import { useAgentAccounts } from "./useAgentAccounts";
import { isProblem } from "./types";
import type { Task } from "./types";

/** How often the status bar's clock moves on, so the next check and a stalled session show in time. */
const TICK_MS = 15_000;
/** How often the runs' files are read again. */
const READ_MS = 2_000;

const LAST_KEY = "allomos.monitor.last";

function readLast(root: string): LastRun | null {
  try {
    const text = localStorage.getItem(`${LAST_KEY}:${root}`);
    return text ? (JSON.parse(text) as LastRun) : null;
  } catch {
    return null;
  }
}

function saveLast(root: string, last: LastRun) {
  try {
    localStorage.setItem(`${LAST_KEY}:${root}`, JSON.stringify(last));
  } catch {
    /* storage unavailable: the line lasts until the window closes */
  }
}

/** One project's monitors: the runs going on (none when not monitoring), the last run, and what
 * the status bars do. */
export function useMonitor({
  messenger,
  root,
  active,
  onError,
}: {
  messenger: Messenger;
  root: string;
  active: string | null | undefined;
  onError: (message: string) => void;
}) {
  const { problem } = useI18n();
  // The extra accounts' names for the labels (Task-A03-oven548).
  useAgentAccounts(messenger);
  const [runs, setRuns] = useState<Run[]>([]);
  const [lost, setLost] = useState<LostRun[]>([]);
  const [last, setLast] = useState<LastRun | null>(() => readLast(root));
  const [now, setNow] = useState(() => Date.now());
  /** The runs seen at the last read: one gone since then has ended. */
  const seen = useRef<Map<string, Run>>(new Map());
  /** The sweep notices the person closed, by run and check. */
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());

  const read = useCallback(async () => {
    let list: unknown[];
    try {
      list = await messenger.monitorRuns(root);
    } catch {
      return;
    }
    const entries = list.map((e) => readEntry(e, active)).filter((e): e is Run | LostRun => e !== null);
    const found = entries.filter(isRun);
    for (const [id, r] of seen.current) {
      if (!found.some((f) => f.id === id)) {
        const summary: LastRun = { ended: new Date().toISOString(), scope: r.scope, taken: r.taken, takenBy: r.takenBy };
        saveLast(root, summary);
        setLast(summary);
      }
    }
    seen.current = new Map(found.map((r) => [r.id, r]));
    setRuns(found);
    setLost(entries.filter((e): e is LostRun => !isRun(e)));
  }, [messenger, root, active]);

  // Another project opened in this window: its own runs and last run.
  useEffect(() => {
    seen.current = new Map();
    setRuns([]);
    setLost([]);
    setLast(readLast(root));
    void read();
    const timer = window.setInterval(() => void read(), READ_MS);
    return () => window.clearInterval(timer);
  }, [root, read]);

  // The clock runs only while monitoring.
  const running = runs.length > 0;
  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(timer);
  }, [running]);

  const ask = (run: Run, request: MonitorRequest) => {
    messenger
      .monitorAsk(run.id, request)
      .then(() => window.setTimeout(() => void read(), 1_200))
      .catch((e) => onError(isProblem(e) ? problem(e) : String(e)));
  };

  return {
    runs,
    lost,
    last,
    now,
    /** The line under a run's status bar after its sweep let blocked tasks go on, until the next
     * check or until closed. */
    notice: (run: Run): UnblockNotice | null => (dismissed.has(`${run.id}:${run.checked}`) ? null : unblockNotice(run.unblocked)),
    dismissNotice: (run: Run) => setDismissed((d) => new Set(d).add(`${run.id}:${run.checked}`)),
    start: (
      scope: Scope,
      agents: TerminalAgent[],
      quiet: boolean,
      every: Frequency,
      sessions: Partial<Record<TerminalAgent, number>>,
      difficulties: Partial<Record<TerminalAgent, Levels>>,
    ) => {
      messenger
        .monitorStart(
          root,
          agents,
          every,
          quiet,
          scope.kind === "iteration" ? scope.id : null,
          sessions,
          Object.fromEntries(Object.entries(difficulties).map(([a, l]) => [a, levelsText(l as Levels)])),
        )
        .then(() => read())
        .catch((e) => onError(isProblem(e) ? problem(e) : String(e)));
    },
    stop: (run: Run) => ask(run, { action: "stop" }),
    undoStop: (run: Run) => ask(run, { action: "undo_stop" }),
    checkNow: (run: Run) => ask(run, { action: "check" }),
    setEvery: (run: Run, every: Frequency) => ask(run, { action: "every", minutes: every }),
    /** Adds or takes off agents while the run goes on (Task-A03-fruit467). */
    setAgents: (run: Run, agents: TerminalAgent[]) => ask(run, { action: "agents", agents: setAgents(run, agents) }),
    /** How many sessions an agent may have at work at once, from the next check on (Task-A03-weasel276). */
    setSessions: (run: Run, agent: TerminalAgent, count: number) => ask(run, { action: "sessions", agent, count }),
    /** The difficulty levels an agent takes tasks from, from the next check on (Task-A03-wave335). */
    setLevels: (run: Run, agent: TerminalAgent, levels: Levels) => ask(run, { action: "difficulty", agent, range: levelsText(levels) }),
    /** Ends a run whose record is missing. */
    endLost: (l: LostRun) => {
      messenger
        .monitorEndProcess(l.pid)
        .then(() => read())
        .catch((e) => onError(isProblem(e) ? problem(e) : String(e)));
    },
  };
}

export type Monitor = ReturnType<typeof useMonitor>;

/** The scope as the status bar and the last run line name it. */
function scopeName(scope: Scope, t: ReturnType<typeof useI18n>["t"]): string {
  return scope.kind === "all" ? t("monitor.scope_all") : t("monitor.scope_label", { id: scope.id });
}

/** The button (with the last run's line beside it), or the status bar while monitoring. */
export function MonitorControl({
  monitor,
  tasks,
  ready,
  showScope,
  onOpen,
  onOpenSession,
  onOpenTask,
  onError,
  onNeedYou,
}: {
  monitor: Monitor;
  tasks: Task[];
  /** Ready tasks nobody holds in the whole project, for the button's hint when there are none. */
  ready: number;
  /** Name the scope in the status bar (on the Iterations tab). */
  showScope?: boolean;
  onOpen: () => void;
  onOpenSession: (id: string) => Promise<void>;
  /** Opens a task the monitor's line names (Task-A01-steam936). */
  onOpenTask?: (id: string) => void;
  onError: (e: unknown) => void;
  /** Picks the Show filter's "Needs you" preset (Task-A01-wigwam308): "K need you" is then a link. */
  onNeedYou?: () => void;
}) {
  const { t } = useI18n();
  const { runs, lost, last, now } = monitor;

  const button = (
    <button type="button" className="monitor-button" onClick={onOpen} title={ready === 0 ? t("monitor.button_none_tip") : undefined}>
      {t("monitor.button")}
    </button>
  );
  if (!runs.length && !lost.length) {
    const counts = last ? lastRunCounts(last, tasks, toCheck) : null;
    // Per agent when more than one agent gave out tasks (Task-A03-weasel276).
    const split = counts && counts.byAgent.length > 1 ? counts.byAgent.map((b) => `${monitorAgentName(b.agent, t)} ${b.done}`).join(" · ") : null;
    return (
      <span className="monitor">
        {last && counts && (
          <span className="monitor-last muted small">
            {split
              ? t("monitor.last_run_split", { time: localTime(last.ended), scope: scopeName(last.scope, t), done: counts.done, split })
              : t("monitor.last_run", { time: localTime(last.ended), scope: scopeName(last.scope, t), done: counts.done })}
            {counts.needYou > 0 && (
              <>
                {" · "}
                {onNeedYou ? (
                  <button type="button" className="link" title={t("monitor.last_run_need_tip")} onClick={onNeedYou}>
                    {t("monitor.last_run_need", { n: counts.needYou })}
                  </button>
                ) : (
                  t("monitor.last_run_need", { n: counts.needYou })
                )}
              </>
            )}
          </span>
        )}
        {button}
      </span>
    );
  }

  return (
    <span className="monitor-stack">
      {runs.map((run) => (
        <RunBar
          key={run.id}
          run={run}
          monitor={monitor}
          tasks={tasks}
          now={now}
          showScope={showScope || runs.length > 1}
          onOpenSession={onOpenSession}
          onOpenTask={onOpenTask}
          onError={onError}
        />
      ))}
      {lost.map((l) => (
        <span key={l.pid} className="monitor-bar stopping" role="status">
          <span className="dot" aria-hidden="true" />
          <span>{t("monitor.lost", { pid: l.pid, id: l.id ?? "" })}</span>
          <button type="button" className="small stop" onClick={() => monitor.endLost(l)}>
            {t("monitor.stop")}
          </button>
        </span>
      ))}
      <span className="monitor">{button}</span>
    </span>
  );
}

/** One run's status bar: what it is doing, Check now, its frequency and agents, the sessions at
 * work (each a link to its session), and Stop. */
function RunBar({
  run,
  monitor,
  tasks,
  now,
  showScope,
  onOpenSession,
  onOpenTask,
  onError,
}: {
  run: Run;
  monitor: Monitor;
  tasks: Task[];
  now: number;
  showScope?: boolean;
  onOpenSession: (id: string) => Promise<void>;
  onOpenTask?: (id: string) => void;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const [menu, setMenu] = useState(false);
  const working = run.working;
  const ids = working.map((w) => w.id).join(", ");
  const free = freeAgents(run);
  const { unit, n } = frequencyText(run.every);
  const nextAt = new Date(nextCheck(run)).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const due = nextCheck(run) <= now;
  const status = run.stopping && working.length
    ? t("monitor.stopping", { id: ids })
    : working.length
      ? free.length
        ? t(due ? "monitor.on_working_starting" : "monitor.on_working_waiting", { id: ids, n: run.taken.length, time: nextAt })
        : t("monitor.on_working", { id: ids, n: run.taken.length })
      : due
        ? t("monitor.on_starting")
        : t("monitor.on_waiting", { time: nextAt });
  const late = stalled(run, now);
  const notice = monitor.notice(run);
  return (
    <>
    <span className={run.stopping ? "monitor-bar stopping" : "monitor-bar"} role="status" title={t("monitor.run_tip", { id: run.id })}>
      <span className="dot" aria-hidden="true" />
      {showScope && <span className="monitor-scope">{scopeName(run.scope, t)} ·</span>}
      <span>{status}</span>
      {free.length > 0 && !run.stopping && (
        <button type="button" className="link small" onClick={() => monitor.checkNow(run)}>
          {t("monitor.check_now")}
        </button>
      )}
      <span className="monitor-every-wrap">
        <button type="button" className="every" aria-haspopup="menu" aria-expanded={menu} aria-label={t("monitor.freq_aria")} onClick={() => setMenu((m) => !m)}>
          {t(`monitor.every_${unit}`, { n })} ✎
        </button>
        {menu && (
          <div className="monitor-menu" role="menu">
            <p className="small">
              <strong>{t("monitor.freq_menu_title")}</strong>
            </p>
            {FREQUENCIES.map((f) => {
              const w = frequencyText(f);
              return (
                <button
                  key={f}
                  type="button"
                  role="menuitemradio"
                  aria-checked={f === run.every}
                  className={f === run.every ? "on" : undefined}
                  onClick={() => {
                    monitor.setEvery(run, f);
                    setMenu(false);
                  }}
                >
                  {f === run.every ? "✓ " : ""}
                  {t(`monitor.freq_${w.unit}`, { n: w.n })}
                </button>
              );
            })}
          </div>
        )}
      </span>
      <AgentsMenu
        run={run}
        onChange={(agents) => monitor.setAgents(run, agents)}
        onSessions={(a, n) => monitor.setSessions(run, a, n)}
        onLevels={(a, l) => monitor.setLevels(run, a, l)}
      />
      {run.noTask.length > 0 && (
        <span className="muted small">
          {t("monitor.no_task_in_levels", { agents: run.noTask.map((a) => monitorAgentName(a, t)).join(", ") })}
        </span>
      )}
      {working.map((cur) => {
        const task = tasks.find((x) => x.id === cur.id);
        const place = task ? sessionPlace(task) : null;
        const label = `${monitorAgentName(cur.agent, t)} · ${cur.id}`;
        return place ? (
          <button
            key={cur.id}
            type="button"
            className="link with-icon holder-open"
            title={t(place === "app" ? "session.open_in_claude" : "session.resume_in_terminal")}
            onClick={() => onOpenSession(cur.id).catch(onError)}
          >
            {label}
            <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M9 4.5H5a1 1 0 0 0-1 1v9.5a1 1 0 0 0 1 1h9.5a1 1 0 0 0 1-1V11" />
              <path d="M12 3.5h4.5V8M16.5 3.5 9.5 10.5" />
            </svg>
          </button>
        ) : (
          <span key={cur.id} className="muted">
            {label}
          </span>
        );
      })}
      {late.map((cur) => (
        <span key={cur.id} className="monitor-stalled">
          {t("monitor.stalled", { id: cur.id })}
        </span>
      ))}
      {run.stopping ? (
        <button type="button" className="small" onClick={() => monitor.undoStop(run)}>
          {t("monitor.undo_stop")}
        </button>
      ) : (
        <button type="button" className="small stop" onClick={() => monitor.stop(run)}>
          {t("monitor.stop")}
        </button>
      )}
    </span>
    {notice && <UnblockLine notice={notice} onOpenTask={onOpenTask} onClose={() => monitor.dismissNotice(run)} />}
    </>
  );
}

/** The status bar's agents (Task-A03-fruit467, approver 2026-11-08): the agents the run gives
 * tasks to, with a menu to add one offered at the start or take one off, and for each chosen one
 * how many sessions it may have at work at once (Task-A03-weasel276). One taken off finishes the
 * task its session works; the last one cannot be taken off (Stop ends the run). */
function AgentsMenu({
  run,
  onChange,
  onSessions,
  onLevels,
}: {
  run: Run;
  onChange: (agents: TerminalAgent[]) => void;
  onSessions: (agent: TerminalAgent, count: number) => void;
  onLevels: (agent: TerminalAgent, levels: Levels) => void;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  /** What the person is typing into a count, until it is left (the run's count shows otherwise). */
  const [draft, setDraft] = useState<Partial<Record<TerminalAgent, string>>>({});
  const names = run.agents
    .map((a) => {
      const count = sessionsOf(run, a) > 1 ? ` ×${sessionsOf(run, a)}` : "";
      const levels = levelsOf(run, a);
      const range = levels.low === 1 && levels.high === 5 ? "" : ` (${levelsText(levels)})`;
      return `${monitorAgentName(a, t)}${count}${range}`;
    })
    .join(", ");
  return (
    <span className="monitor-every-wrap">
      <button
        type="button"
        className="every"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("monitor.agents_aria")}
        onClick={() => setOpen((o) => !o)}
      >
        {names} ✎
      </button>
      {open && (
        <div className="monitor-menu" role="menu">
          <p className="small">
            <strong>{t("monitor.agents_menu_title")}</strong>
          </p>
          {run.offered.map((a) => {
            const on = run.agents.includes(a);
            const last = on && run.agents.length === 1;
            const busy = run.working.find((w) => w.agent === a);
            return (
              <div key={a} className="monitor-menu-agent">
                <button
                  type="button"
                  role="menuitemcheckbox"
                  aria-checked={on}
                  className={on ? "on" : undefined}
                  disabled={last}
                  title={last ? t("monitor.agents_last") : undefined}
                  onClick={() => onChange(on ? run.agents.filter((x) => x !== a) : [...run.agents, a])}
                >
                  {on ? "✓ " : ""}
                  {monitorAgentName(a, t)}
                  {!on && busy && <span className="muted small"> · {t("monitor.agents_finishing", { id: busy.id })}</span>}
                </button>
                {on && (
                  <label className="small monitor-sessions" title={t("monitor.sessions_label")}>
                    <input
                      type="number"
                      min={1}
                      step={1}
                      inputMode="numeric"
                      aria-label={t("monitor.sessions_aria", { agent: monitorAgentName(a, t) })}
                      value={draft[a] ?? String(sessionsOf(run, a))}
                      onChange={(e) => {
                        setDraft((d) => ({ ...d, [a]: e.target.value }));
                        const n = Math.floor(Number(e.target.value));
                        if (n >= 1 && n !== sessionsOf(run, a)) onSessions(a, n);
                      }}
                      onBlur={() => setDraft((d) => ({ ...d, [a]: undefined }))}
                    />
                  </label>
                )}
                {on && <LevelsPicker agent={a} name={monitorAgentName(a, t)} levels={levelsOf(run, a)} onChange={(l) => onLevels(a, l)} />}
              </div>
            );
          })}
          <p className="muted small">{t("monitor.agents_menu_note")}</p>
          <p className="muted small">{t("monitor.sessions_menu_note")}</p>
          <p className="muted small">{t("monitor.levels_menu_note")}</p>
        </div>
      )}
    </span>
  );
}

/** The line under the status bar after a sweep let blocked tasks go on (Task-A01-steam936,
 * approver 2026-11-08): which tasks, each one a link to it; kept until the next check or ×. */
function UnblockLine({
  notice,
  onOpenTask,
  onClose,
}: {
  notice: UnblockNotice;
  onOpenTask?: (id: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const MARK = "\u0000";
  return (
    <span className="monitor-notice muted small" role="status">
      {notice.groups.map((g) => {
        const [before, after = ""] = t(g.state === "ready" ? "monitor.unblocked_ready" : "monitor.unblocked_in_progress", { ids: MARK }).split(MARK);
        return (
          <span key={g.state}>
            {before}
            {g.ids.map((id, i) => (
              <span key={id}>
                {i > 0 && ", "}
                {onOpenTask ? (
                  <button type="button" className="link" onClick={() => onOpenTask(id)}>
                    {id}
                  </button>
                ) : (
                  id
                )}
              </span>
            ))}
            {after}{" "}
          </span>
        );
      })}
      <button type="button" className="link" aria-label={t("monitor.unblocked_close")} title={t("monitor.unblocked_close")} onClick={onClose}>
        ×
      </button>
    </span>
  );
}
