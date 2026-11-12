// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): what shows in place of its button
// while runs go on. Since Task-A03-cow112 (approver 2026-11-09) a run is a background `alm monitor
// run` process of this computer, started here with the app's own `alm monitor start` or from a
// terminal, and it goes on after the app is closed: the app runs no loop of its own. It reads the
// project's run files every few seconds and shows them in the Auto-monitor section, one foldable
// panel per run (A03-yolk481); Stop monitor, Check now and Edit monitor… leave requests for the
// run. The last run's summary is remembered per project for the line beside the button.

import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger, MonitorRequest } from "./messenger";
import { monitorAgentName } from "./AgentPicker";
import { localTime } from "./timeFlow";
import { sessionPlace } from "./sessionFlow";
import { toCheck } from "./stopFlow";
import {
  NO_FOLDS,
  STALL_MS,
  editRequests,
  freeAgents,
  frequencyText,
  grouped,
  isRun,
  keptFolds,
  lastRunCounts,
  lastRunFromFile,
  levelsOf,
  levelsText,
  newerLast,
  nextCheck,
  readEntry,
  readFolds,
  runState,
  sessionGroups,
  sessionsOf,
  stalled,
  stateCounts,
  unblockNotice,
  workedFor,
} from "./monitorFlow";
import type { Current, Frequency, LastRun, LostRun, Levels, MonitorFolds, MonitorSettings, Run, RunState, Scope, TerminalAgent, UnblockNotice } from "./monitorFlow";
import { useAgentAccounts } from "./useAgentAccounts";
import { isProblem } from "./types";
import type { Task } from "./types";

/** How often the status bar's clock moves on, so the next check and a stalled session show in time. */
const TICK_MS = 15_000;

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
  }, [root]);
  // Read when the project opens, then again whenever the app's monitor loop says the runs changed
  // (Task-A03-spider528): the window no longer asks on its own.
  useEffect(() => {
    void read();
    let live = true;
    let stop: (() => void) | undefined;
    void messenger
      .onMonitorsChanged(() => void read())
      .then((un) => {
        if (live) stop = un;
        else un();
      })
      .catch(() => undefined);
    return () => {
      live = false;
      stop?.();
    };
  }, [messenger, read]);

  // The read a resume effect uses: it runs once per opening of the project, not per change of
  // `active`.
  const readRef = useRef(read);
  useEffect(() => {
    readRef.current = read;
  }, [read]);

  // Opening the project starts again the runs that stopped without ending (a restart, an update,
  // a log-out) from the settings they saved, with the sessions they had at work (A03-pocket537).
  // Nothing is shown when there is nothing to start: the status bar is what the person sees.
  useEffect(() => {
    let current = true;
    messenger
      .monitorResume(root)
      .then((ids) => {
        if (current && ids.length > 0) void readRef.current();
      })
      .catch(() => {
        /* the monitor is optional here: a project opens without it */
      });
    return () => {
      current = false;
    };
  }, [messenger, root]);

  // The last run kept on this computer, which counts a run that ended while the app was closed
  // (A03-pocket537). The later of it and the window's own is shown.
  useEffect(() => {
    let current = true;
    messenger
      .monitorLast(root)
      .then((v) => {
        const kept = lastRunFromFile(v);
        if (current && kept) setLast((cur) => newerLast(cur, kept));
      })
      .catch(() => {
        /* no file: the window's own last run stays */
      });
    return () => {
      current = false;
    };
  }, [messenger, root]);

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
    /** Edit monitor…'s Save (A03-yolk481): one request for each setting that changed, in order. */
    edit: (run: Run, next: MonitorSettings) => {
      editRequests(run, next)
        .reduce((p, r) => p.then(() => messenger.monitorAsk(run.id, r)), Promise.resolve())
        .then(() => window.setTimeout(() => void read(), 1_200))
        .catch((e) => onError(isProblem(e) ? problem(e) : String(e)));
    },
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

type T = ReturnType<typeof useI18n>["t"];

const timeOf = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

/** A difficulty range as the panel shows it: `1–5`, or one level. */
const rangeText = (l: Levels) => (l.low === l.high ? String(l.low) : `${l.low}–${l.high}`);

/** How long a session has worked, in words. */
function forText(at: number, now: number, t: T): string {
  const w = workedFor(at, now);
  return t(w.unit === "hours" ? "monitor.for_hours" : "monitor.for_minutes", { n: w.n });
}

const FOLDS_KEY = "allomos.monitor.folds";

function loadFolds(root: string): MonitorFolds {
  try {
    return readFolds(localStorage.getItem(`${FOLDS_KEY}:${root}`));
  } catch {
    return NO_FOLDS;
  }
}

/** The section's and the panels' folds, kept on this computer per project (A03-yolk481): never
 * in the taskbag. A panel opens by itself once when one of its sessions goes quiet for long. */
function useFolds(root: string, runs: Run[], now: number) {
  const [folds, setFolds] = useState<MonitorFolds>(() => loadFolds(root));
  useEffect(() => setFolds(loadFolds(root)), [root]);
  const save = (next: MonitorFolds) => {
    const kept = runs.length ? keptFolds(next, runs.map((r) => r.id)) : next;
    setFolds(kept);
    try {
      localStorage.setItem(`${FOLDS_KEY}:${root}`, JSON.stringify(kept));
    } catch {
      /* storage unavailable: the folds last until the window closes */
    }
  };
  // Each stalled session opens its panel once (keyed by run and task).
  const opened = useRef<Set<string>>(new Set());
  useEffect(() => {
    const add: string[] = [];
    for (const r of runs) {
      for (const w of stalled(r, now)) {
        const key = `${r.id}:${w.id}`;
        if (opened.current.has(key)) continue;
        opened.current.add(key);
        if (!folds.open.includes(r.id) && !add.includes(r.id)) add.push(r.id);
      }
    }
    if (add.length) save({ ...folds, open: [...folds.open, ...add] });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [runs, now]);
  return {
    sectionOpen: !folds.sectionFolded,
    toggleSection: () => save({ ...folds, sectionFolded: !folds.sectionFolded }),
    isOpen: (id: string) => folds.open.includes(id),
    toggle: (id: string) => save({ ...folds, open: folds.open.includes(id) ? folds.open.filter((x) => x !== id) : [...folds.open, id] }),
  };
}

/** A panel's one-line summary (folded, and in the section title when there is one monitor). */
function runSummary(run: Run, now: number, t: T, withSettings: boolean): string {
  const ids = run.working.map((w) => w.id);
  if (run.stopping && ids.length) return t("monitor.stopping", { id: ids.join(", ") });
  let head: string;
  if (ids.length === 1) head = t("monitor.sum_working", { id: ids[0] });
  else if (ids.length > 1) {
    const shown = ids.length > 2 ? t("monitor.sum_and_more", { ids: ids.slice(0, 2).join(", ") }) : ids.join(", ");
    head = t("monitor.sum_many", { n: ids.length, ids: shown });
  } else head = nextCheck(run) <= now ? t("monitor.sum_checking") : t("monitor.sum_idle", { time: timeOf(nextCheck(run)) });
  if (!withSettings || ids.length !== 1) return head;
  const { unit, n } = frequencyText(run.every);
  return `${head} · ${t(`monitor.every_${unit}`, { n })} · ${t(run.quiet ? "monitor.sum_quiet" : "monitor.sum_normal")}`;
}

/** The button (with the last run's line beside it), or the Auto-monitor section while monitors
 * run (A03-yolk481, approver 2026-11-12): its title with one badge per state, then one foldable
 * panel per monitor, then Start another monitor…. */
export function MonitorControl({
  monitor,
  tasks,
  ready,
  root,
  onOpen,
  onEdit,
  onOpenSession,
  onOpenTask,
  onError,
  onNeedYou,
}: {
  monitor: Monitor;
  tasks: Task[];
  /** Ready tasks nobody holds in the whole project, for the button's hint when there are none. */
  ready: number;
  /** The project, whose folds this computer remembers. */
  root: string;
  onOpen: () => void;
  /** Opens Edit monitor… for a run (A03-yolk481). */
  onEdit: (run: Run) => void;
  onOpenSession: (id: string) => Promise<void>;
  /** Opens a task the monitor's line names (Task-A01-steam936). */
  onOpenTask?: (id: string) => void;
  onError: (e: unknown) => void;
  /** Picks the Show filter's "Needs you" preset (Task-A01-wigwam308): "K need you" is then a link. */
  onNeedYou?: () => void;
}) {
  const { t } = useI18n();
  const { runs, lost, last, now } = monitor;
  const folds = useFolds(root, runs, now);

  const button = (
    <button type="button" className="monitor-button" onClick={onOpen} title={ready === 0 ? t("monitor.button_none_tip") : undefined}>
      {/* A run is going (Task-A03-mug608): the button opens another one beside it. */}
      {t(runs.length ? "monitor.button_another" : "monitor.button")}
    </button>
  );
  if (!runs.length && !lost.length) {
    const counts = last ? lastRunCounts(last, tasks, toCheck) : null;
    // Per agent when more than one agent gave out tasks (Task-A03-weasel276).
    const split = counts && counts.byAgent.length > 1 ? counts.byAgent.map((b) => `${monitorAgentName(b.agent, t)} ${b.done}`).join(" · ") : null;
    return (
      <span className="monitor">
        {button}
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
      </span>
    );
  }

  const states = runs.map((r) => runState(r, tasks, now));
  const counts = stateCounts(states);
  const atWork = runs.reduce((n, r) => n + r.working.length, 0);
  // Folded with one monitor, the summary says what it does; otherwise how many sessions work.
  const summary =
    runs.length === 1 && !folds.sectionOpen
      ? runSummary(runs[0], now, t, false)
      : atWork
        ? t("monitor.sum_sessions", { n: atWork })
        : t("monitor.sum_none_at_work");
  return (
    <section className={`monitor-section${folds.sectionOpen ? "" : " folded"}`} aria-label={t("monitor.section")}>
      <div className="monitor-section-head">
        <button type="button" className="monitor-fold" aria-expanded={folds.sectionOpen} title={t("monitor.section_aria")} onClick={folds.toggleSection}>
          <strong>{t("monitor.section")}</strong>
          {counts.map((c) => (
            <span key={c.state} className={`monitor-badge ${c.state}`}>
              {t(`monitor.count_${c.state}`, { n: c.n })}
            </span>
          ))}
          {lost.length > 0 && <span className="monitor-badge stopping">{t("monitor.count_stopping", { n: lost.length })}</span>}
          <span className="muted small monitor-summary">{summary}</span>
        </button>
      </div>
      {folds.sectionOpen && (
        <div className="monitor-section-body">
          {runs.map((run, i) => (
            <RunPanel
              key={run.id}
              run={run}
              state={states[i]}
              monitor={monitor}
              tasks={tasks}
              now={now}
              open={folds.isOpen(run.id)}
              onToggle={() => folds.toggle(run.id)}
              onEdit={() => onEdit(run)}
              onOpenSession={onOpenSession}
              onOpenTask={onOpenTask}
              onError={onError}
            />
          ))}
          {lost.map((l) => (
            <div key={l.pid} className="monitor-panel lost" role="status">
              <div className="monitor-panel-head">
                <span className="monitor-panel-text">{t("monitor.lost", { pid: l.pid, id: l.id ?? "" })}</span>
                <span className="monitor-actions">
                  <button type="button" className="small stop" onClick={() => monitor.endLost(l)}>
                    {t("monitor.stop_monitor")}
                  </button>
                </span>
              </div>
            </div>
          ))}
          <div className="monitor-another">{button}</div>
        </div>
      )}
    </section>
  );
}

/** One monitor's panel (A03-yolk481): folded, one line (its badge when it is not plainly
 * running, a summary, Check now and Stop monitor, which never fold or unfold it); open, also its
 * settings, Edit monitor… and the sessions it opened, grouped by agent when it has several. */
function RunPanel({
  run,
  state,
  monitor,
  tasks,
  now,
  open,
  onToggle,
  onEdit,
  onOpenSession,
  onOpenTask,
  onError,
}: {
  run: Run;
  state: RunState;
  monitor: Monitor;
  tasks: Task[];
  now: number;
  open: boolean;
  onToggle: () => void;
  onEdit: () => void;
  onOpenSession: (id: string) => Promise<void>;
  onOpenTask?: (id: string) => void;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const notice = monitor.notice(run);
  const free = freeAgents(run);
  const late = stalled(run, now);
  // With several sessions, the badge counts the quiet ones.
  const badge =
    state === "running"
      ? null
      : state === "stalled" && run.working.length > 1
        ? t("monitor.count_stalled", { n: late.length })
        : t(`monitor.badge_${state}`);
  const { unit, n } = frequencyText(run.every);
  const settings = [
    t("monitor.set_every", { every: t(`monitor.every_${unit}`, { n }) }),
    ...run.agents.map((a) => t("monitor.set_agent", { agent: monitorAgentName(a, t), n: sessionsOf(run, a), levels: rangeText(levelsOf(run, a)) })),
    t(run.quiet ? "build.mode_quiet" : "build.mode_normal"),
    t("monitor.set_opened", { n: run.taken.length }),
  ].join(" · ");
  return (
    <div className={`monitor-panel ${state}${open ? " open" : ""}`} title={t("monitor.run_tip", { id: run.id })}>
      <div className="monitor-panel-head">
        <button type="button" className="monitor-fold" aria-expanded={open} title={t("monitor.panel_aria")} onClick={onToggle}>
          <span className="monitor-panel-title">{t("monitor.panel_title", { scope: scopeName(run.scope, t) })}</span>
          {badge && <span className={`monitor-badge ${state}`}>{badge}</span>}
          {!open && <span className="muted small monitor-summary">{runSummary(run, now, t, true)}</span>}
        </button>
        <span className="monitor-actions">
          {open && !run.stopping && (
            <button type="button" className="small" onClick={onEdit}>
              {t("monitor.edit_button")}
            </button>
          )}
          {!run.stopping && (
            <button type="button" className="small" disabled={free.length === 0} onClick={() => monitor.checkNow(run)}>
              {t("monitor.check_now")}
            </button>
          )}
          {run.stopping ? (
            <button type="button" className="small" onClick={() => monitor.undoStop(run)}>
              {t("monitor.undo_stop")}
            </button>
          ) : (
            <button type="button" className="small stop" onClick={() => monitor.stop(run)}>
              {t("monitor.stop_monitor")}
            </button>
          )}
        </span>
      </div>
      {open && (
        <div className="monitor-panel-body">
          <p className="muted small monitor-settings">{settings}</p>
          <h4 className="monitor-sessions-title small">{t("monitor.sessions_title")}</h4>
          {grouped(run) ? (
            sessionGroups(run).map((g) => (
              <div key={g.agent} className="monitor-group">
                <p className="small monitor-group-head">
                  <strong>{monitorAgentName(g.agent, t)}</strong>
                  {" · "}
                  {g.removed
                    ? t("monitor.group_removed")
                    : `${t("monitor.group_count", { n: g.working.length, max: g.sessions })} · ${t("monitor.group_levels", { levels: rangeText(g.levels) })}`}
                </p>
                {g.working.length ? (
                  <ul className="monitor-rows">
                    {g.working.map((w) => (
                      <SessionRow key={w.id} cur={w} tasks={tasks} now={now} finishing={g.removed} inGroup onOpenSession={onOpenSession} onError={onError} />
                    ))}
                  </ul>
                ) : (
                  <p className="muted small monitor-group-note">{t(g.noTask ? "monitor.group_no_task" : "monitor.group_idle")}</p>
                )}
              </div>
            ))
          ) : run.working.length ? (
            <ul className="monitor-rows">
              {run.working.map((w) => (
                <SessionRow key={w.id} cur={w} tasks={tasks} now={now} finishing={false} onOpenSession={onOpenSession} onError={onError} />
              ))}
            </ul>
          ) : (
            <p className="muted small monitor-group-note">{t(run.noTask.length ? "monitor.group_no_task" : "monitor.sessions_none")}</p>
          )}
          {free.length > 0 && !run.stopping && (
            <p className="muted small monitor-next">
              {nextCheck(run) <= now ? t("monitor.sum_checking") : t("monitor.next_check", { time: timeOf(nextCheck(run)) })}
            </p>
          )}
        </div>
      )}
      {notice && <UnblockLine notice={notice} onOpenTask={onOpenTask} onClose={() => monitor.dismissNotice(run)} />}
    </div>
  );
}

/** One session the monitor opened: its task, its agent (when not grouped), how long it has
 * worked, and Open session ↗ when it can be reopened. A quiet one is marked. */
function SessionRow({
  cur,
  tasks,
  now,
  finishing,
  inGroup,
  onOpenSession,
  onError,
}: {
  cur: Current;
  tasks: Task[];
  now: number;
  finishing: boolean;
  inGroup?: boolean;
  onOpenSession: (id: string) => Promise<void>;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const task = tasks.find((x) => x.id === cur.id);
  const place = task ? sessionPlace(task) : null;
  const quiet = now - cur.changed >= STALL_MS;
  const time = forText(cur.at, now, t);
  return (
    <li className={`monitor-row${quiet ? " stalled" : ""}`}>
      <span className="monitor-row-text">
        <strong>{cur.id}</strong>
        {!inGroup && <span className="muted"> · {monitorAgentName(cur.agent, t)}</span>}
        <span className="muted small"> · {t(finishing ? "monitor.session_finishing" : "monitor.session_working", { time })}</span>
        {quiet && <span className="monitor-stalled small"> · {t("monitor.session_stalled")}</span>}
      </span>
      {place && (
        <button
          type="button"
          className="link with-icon holder-open small"
          title={t(place === "app" ? "session.open_in_claude" : "session.resume_in_terminal")}
          onClick={() => onOpenSession(cur.id).catch(onError)}
        >
          {t("monitor.open_session")}
          <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
            <path d="M9 4.5H5a1 1 0 0 0-1 1v9.5a1 1 0 0 0 1 1h9.5a1 1 0 0 0 1-1V11" />
            <path d="M12 3.5h4.5V8M16.5 3.5 9.5 10.5" />
          </svg>
        </button>
      )}
    </li>
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
