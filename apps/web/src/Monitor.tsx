// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): the run itself, and what shows
// in place of its button while it runs. Allomos starts a new terminal session of the chosen agent
// for each ready task, one at a time, and follows the task in the taskbag (read again whenever its
// files change) to see it claimed and then let go. With no task ready it waits and checks again
// at the chosen frequency. The run is kept by this window only, on this computer, never in the
// taskbag; the last one's summary is remembered per project for the line beside the button.

import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY } from "./AgentPicker";
import { localTime } from "./timeFlow";
import { sessionPlace } from "./sessionFlow";
import { toCheck } from "./stopFlow";
import {
  FREQUENCIES,
  follow,
  frequencyText,
  lastRunCounts,
  monitorRequest,
  nextCheck,
  pick,
  stalled,
  startRun,
  take,
} from "./monitorFlow";
import type { Frequency, LastRun, Run, Scope, TerminalAgent } from "./monitorFlow";
import { isProblem } from "./types";
import type { Iteration, Task } from "./types";

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

type Box = { areas: { id: string }[]; iterations: Iteration[]; tasks: Task[] };

/** One project's monitor: the run (null when not monitoring), the last run, and what the status
 * bar does. `box` is the taskbag as read now. */
export function useMonitor({
  messenger,
  root,
  box,
  active,
  onError,
}: {
  messenger: Messenger;
  root: string;
  box: Box | null;
  active: string | null | undefined;
  onError: (message: string) => void;
}) {
  const { t, problem, language } = useI18n();
  const [run, setRun] = useState<Run | null>(null);
  const [last, setLast] = useState<LastRun | null>(() => readLast(root));
  const [now, setNow] = useState(() => Date.now());
  const runRef = useRef(run);
  runRef.current = run;
  /** A session is being started: no second one until it is. */
  const starting = useRef(false);

  const end = useCallback(
    (r: Run) => {
      const summary: LastRun = { ended: new Date().toISOString(), scope: r.scope, taken: r.taken };
      saveLast(root, summary);
      setLast(summary);
      setRun(null);
    },
    [root],
  );

  // Another project opened in this window, or the window closes: the run ends.
  useEffect(() => {
    setLast(readLast(root));
    return () => {
      const r = runRef.current;
      if (r) saveLast(root, { ended: new Date().toISOString(), scope: r.scope, taken: r.taken });
      runRef.current = null;
      setRun(null);
    };
  }, [root]);

  // The clock runs only while monitoring.
  const running = run !== null;
  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(timer);
  }, [running]);

  const startSession = useCallback(
    async (r: Run, task: Task) => {
      starting.current = true;
      const sessionId = r.agent === "claude_cli" ? crypto.randomUUID() : null;
      try {
        const taskbag = await messenger.proceedTaskbag(root).catch(() => root);
        const request = monitorRequest(task, taskbag, language, r.agent, sessionId, r.quiet);
        await messenger.monitorStartSession(r.agent, root, request, sessionId);
        setRun((now) => (now && now.started === r.started ? take(now, task, sessionId, Date.now()) : now));
      } catch (e) {
        const detail = isProblem(e) ? problem(e) : String(e);
        onError(t("monitor.start_failed", { agent: t(AGENT_KEY[r.agent]), id: task.id, detail }));
        const current = runRef.current;
        if (current && current.started === r.started) end(current);
      } finally {
        starting.current = false;
      }
    },
    [messenger, root, language, t, problem, onError, end],
  );

  // Each time the taskbag is read, the clock moves on or the run changes: follow the current
  // task; when it is finished, end the run if a stop was asked, else take the next ready task at
  // once; with none, check again when the frequency says.
  useEffect(() => {
    if (!run || !box) return;
    const at = Date.now();
    const followed = follow(run, box.tasks, at);
    let r = followed.run;
    if (followed.finished && r.stopping) return end(r);
    if (!r.current && !r.stopping && !starting.current && (followed.finished || at >= nextCheck(r))) {
      const task = pick(r, box.tasks, box.areas, box.iterations, active);
      if (task) {
        if (r !== run) setRun(r);
        void startSession(r, task);
        return;
      }
      r = { ...r, checked: at };
    }
    if (r !== run) setRun(r);
  }, [run, box, now, active, end, startSession]);

  return {
    run,
    last,
    now,
    start: (scope: Scope, agent: TerminalAgent, quiet: boolean, every: Frequency) => setRun(startRun(scope, agent, quiet, every, Date.now())),
    stop: () => {
      if (!run) return;
      if (run.current) setRun({ ...run, stopping: true });
      else end(run);
    },
    undoStop: () => run && setRun({ ...run, stopping: false }),
    checkNow: () => run && setRun({ ...run, checked: 0 }),
    setEvery: (every: Frequency) => run && setRun({ ...run, every }),
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
  onError: (e: unknown) => void;
  /** Picks the Show filter's "Needs you" preset (Task-A01-wigwam308): "K need you" is then a link. */
  onNeedYou?: () => void;
}) {
  const { t } = useI18n();
  const { run, last, now } = monitor;
  const [menu, setMenu] = useState(false);

  if (!run) {
    const counts = last ? lastRunCounts(last, tasks, toCheck) : null;
    return (
      <span className="monitor">
        {last && counts && (
          <span className="monitor-last muted small">
            {t("monitor.last_run", { time: localTime(last.ended), scope: scopeName(last.scope, t), done: counts.done })}
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
        <button type="button" className="monitor-button" onClick={onOpen} title={ready === 0 ? t("monitor.button_none_tip") : undefined}>
          {t("monitor.button")}
        </button>
      </span>
    );
  }

  const cur = run.current;
  const task = cur ? tasks.find((x) => x.id === cur.id) : undefined;
  const place = task ? sessionPlace(task) : null;
  const { unit, n } = frequencyText(run.every);
  const nextAt = new Date(nextCheck(run)).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const status = run.stopping && cur
    ? t("monitor.stopping", { id: cur.id })
    : cur
      ? t("monitor.on_working", { id: cur.id, n: run.taken.length })
      : nextCheck(run) <= now
        ? t("monitor.on_starting")
        : t("monitor.on_waiting", { time: nextAt });
  const agentName = t(AGENT_KEY[run.agent]);
  return (
    <span className={run.stopping ? "monitor-bar stopping" : "monitor-bar"} role="status">
      <span className="dot" aria-hidden="true" />
      {showScope && <span className="monitor-scope">{scopeName(run.scope, t)} ·</span>}
      <span>{status}</span>
      {!cur && (
        <button type="button" className="link small" onClick={monitor.checkNow}>
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
                    monitor.setEvery(f);
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
      {cur &&
        (place ? (
          <button
            type="button"
            className="link with-icon holder-open"
            title={t(place === "app" ? "session.open_in_claude" : "session.resume_in_terminal")}
            onClick={() => onOpenSession(cur.id).catch(onError)}
          >
            {agentName} · {cur.id}
            <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M9 4.5H5a1 1 0 0 0-1 1v9.5a1 1 0 0 0 1 1h9.5a1 1 0 0 0 1-1V11" />
              <path d="M12 3.5h4.5V8M16.5 3.5 9.5 10.5" />
            </svg>
          </button>
        ) : (
          <span className="muted">
            {agentName} · {cur.id}
          </span>
        ))}
      {stalled(run, now) && cur && <span className="monitor-stalled">{t("monitor.stalled", { id: cur.id })}</span>}
      {run.stopping ? (
        <button type="button" className="small" onClick={monitor.undoStop}>
          {t("monitor.undo_stop")}
        </button>
      ) : (
        <button type="button" className="small stop" onClick={monitor.stop}>
          {t("monitor.stop")}
        </button>
      )}
    </span>
  );
}
