// Monitor ready tasks (Task-A03-moth285, approver 2026-11-03/04): the run itself, and what shows
// in place of its button while it runs. Allomos starts a new terminal session of a chosen agent
// for each ready task, one at a time per agent (several agents: Task-A03-fruit467), and follows the task in the taskbag (read again whenever its
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
  freeAgents,
  frequencyText,
  idleAlert,
  lastRunCounts,
  monitorRequest,
  nextCheck,
  noticeAfterCheck,
  setAgents,
  share,
  stalled,
  startRun,
  tabToClose,
  take,
  unblockNotice,
  waitingAlert,
  withReleased,
} from "./monitorFlow";
import type { Frequency, LastRun, Run, Scope, StuckAlert, TerminalAgent, UnblockNotice } from "./monitorFlow";
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
  /** What the last sweep let go on (Task-A01-steam936): the line under the status bar, and the
   * tasks as it left them until the taskbag is read again. */
  const [notice, setNotice] = useState<UnblockNotice | null>(null);
  const [released, setReleased] = useState<Task[]>([]);

  /** When the monitor starts and on Check now: every blocked task whose blockers have all finished
   * goes on first, so this check can take it. A failed sweep is said, and the check goes on. */
  const sweep = useCallback(async () => {
    try {
      const list = await messenger.releaseFinished(root);
      setReleased(list.map((r) => r.task));
      setNotice(unblockNotice(list));
    } catch (e) {
      setNotice(null);
      onError(isProblem(e) ? problem(e) : String(e));
    }
  }, [messenger, root, onError, problem]);

  const end = useCallback(
    (r: Run) => {
      const summary: LastRun = { ended: new Date().toISOString(), scope: r.scope, taken: r.taken };
      saveLast(root, summary);
      setLast(summary);
      setRun(null);
      setNotice(null);
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

  /** Tells the person the monitor is stuck (Task-A00-eclair542): a system notification. */
  const tell = useCallback(
    (alert: StuckAlert, r: Run, at: number) => {
      const body =
        alert.kind === "waiting"
          ? t("monitor.notify_waiting", { id: alert.id, on: alert.on })
          : t("monitor.notify_idle", {
              time: new Date(at + r.every * 60_000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
            });
      void messenger.monitorNotify(t("monitor.notify_title"), body).catch(() => {});
    },
    [messenger, t],
  );

  /** Starts one session per agent and task, one after another; a failure is said, and that agent
   * is taken off the run (the run ends when it was the last one and nothing else is worked). */
  const startSessions = useCallback(
    async (r: Run, shares: { agent: TerminalAgent; task: Task }[]) => {
      starting.current = true;
      try {
        const taskbag = await messenger.proceedTaskbag(root).catch(() => root);
        for (const { agent, task } of shares) {
          const sessionId = agent === "claude_cli" ? crypto.randomUUID() : null;
          try {
            const request = monitorRequest(task, taskbag, language, agent, sessionId, r.quiet);
            const tab = await messenger.monitorStartSession(agent, root, request, sessionId);
            setRun((now) => (now && now.started === r.started ? take(now, agent, task, sessionId, Date.now(), tab) : now));
          } catch (e) {
            const detail = isProblem(e) ? problem(e) : String(e);
            onError(t("monitor.start_failed", { agent: t(AGENT_KEY[agent]), id: task.id, detail }));
            const current = runRef.current;
            if (!current || current.started !== r.started) return;
            const left = current.agents.filter((x) => x !== agent);
            if (left.length === 0 && current.working.length === 0) {
              end(current);
              return;
            }
            const next = left.length ? setAgents(current, left) : { ...current, stopping: true };
            runRef.current = next;
            setRun(next);
          }
        }
      } finally {
        starting.current = false;
      }
    },
    [messenger, root, language, t, problem, onError, end],
  );

  // Each time the taskbag is read, the clock moves on or the run changes: follow every session's
  // task; when one is finished, end the run if a stop was asked and nothing else is worked, else
  // give the free agents the next ready tasks at once; with none, check again when the frequency
  // says.
  useEffect(() => {
    if (!run || !box) return;
    // The taskbag read again shows what the sweep let go on: no need to keep its copies.
    if (released.length && released.every((r) => box.tasks.find((t) => t.id === r.id)?.state !== "blocked")) {
      setReleased([]);
    }
    const tasks = withReleased(box.tasks, released);
    const at = Date.now();
    const followed = follow(run, box.tasks, at);
    for (const cur of followed.finished) {
      // Its task done, the session's tab closes after 30 seconds unless a key is pressed in it.
      const tab = tabToClose(cur, box.tasks);
      if (tab) {
        const texts = { prompt: t("monitor.close_prompt"), kept: t("monitor.close_kept") };
        void messenger.monitorCloseSession(cur.agent, tab, texts).catch(() => {});
      }
      // Its session parked a question for the person: tell them.
      const waiting = waitingAlert(cur.id, box.tasks);
      if (waiting) tell(waiting, run, at);
    }
    let r = followed.run;
    if (r.stopping) {
      if (!r.working.length) return end(r);
      if (r !== run) setRun(r);
      return;
    }
    const free = freeAgents(r);
    if (free.length && !starting.current && (followed.finished.length > 0 || at >= nextCheck(r))) {
      setNotice(noticeAfterCheck);
      const shares = share(r, tasks, box.areas, box.iterations, active);
      if (shares.length) {
        if (r !== run) setRun(r);
        void startSessions(r, shares);
        return;
      }
      r = { ...r, checked: at };
      // Nothing to take and no session at work: tell the person once, until a task is taken again.
      const idle = idleAlert(r);
      if (idle.alert) tell(idle.alert, idle.run, at);
      r = idle.run;
    }
    if (r !== run) setRun(r);
  }, [run, box, released, now, active, end, startSessions, tell, messenger, t]);

  return {
    run,
    last,
    now,
    notice,
    dismissNotice: () => setNotice(null),
    start: (scope: Scope, agents: TerminalAgent[], quiet: boolean, every: Frequency, offered?: TerminalAgent[]) => {
      void sweep().then(() => setRun(startRun(scope, agents, quiet, every, Date.now(), offered)));
    },
    stop: () => {
      if (!run) return;
      if (run.working.length) setRun({ ...run, stopping: true });
      else end(run);
    },
    undoStop: () => run && setRun({ ...run, stopping: false }),
    checkNow: () => {
      if (!run) return;
      void sweep().then(() => setRun((now) => (now ? { ...now, checked: 0 } : now)));
    },
    setEvery: (every: Frequency) => run && setRun({ ...run, every }),
    /** Adds or takes off agents while the run goes on (Task-A03-fruit467). */
    setAgents: (agents: TerminalAgent[]) => setRun((now) => (now ? setAgents(now, agents) : now)),
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
  return (
    <span className="monitor-stack">
    <span className={run.stopping ? "monitor-bar stopping" : "monitor-bar"} role="status">
      <span className="dot" aria-hidden="true" />
      {showScope && <span className="monitor-scope">{scopeName(run.scope, t)} ·</span>}
      <span>{status}</span>
      {free.length > 0 && !run.stopping && (
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
      <AgentsMenu run={run} onChange={monitor.setAgents} />
      {working.map((cur) => {
        const task = tasks.find((x) => x.id === cur.id);
        const place = task ? sessionPlace(task) : null;
        const label = `${t(AGENT_KEY[cur.agent])} · ${cur.id}`;
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
        <button type="button" className="small" onClick={monitor.undoStop}>
          {t("monitor.undo_stop")}
        </button>
      ) : (
        <button type="button" className="small stop" onClick={monitor.stop}>
          {t("monitor.stop")}
        </button>
      )}
    </span>
    {monitor.notice && <UnblockLine notice={monitor.notice} onOpenTask={onOpenTask} onClose={monitor.dismissNotice} />}
    </span>
  );
}

/** The status bar's agents (Task-A03-fruit467, approver 2026-11-08): the agents the run gives
 * tasks to, with a menu to add one offered at the start or take one off. One taken off finishes
 * the task its session works; the last one cannot be taken off (Stop ends the run). */
function AgentsMenu({ run, onChange }: { run: Run; onChange: (agents: TerminalAgent[]) => void }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const names = run.agents.map((a) => t(AGENT_KEY[a])).join(", ");
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
              <button
                key={a}
                type="button"
                role="menuitemcheckbox"
                aria-checked={on}
                className={on ? "on" : undefined}
                disabled={last}
                title={last ? t("monitor.agents_last") : undefined}
                onClick={() => onChange(on ? run.agents.filter((x) => x !== a) : [...run.agents, a])}
              >
                {on ? "✓ " : ""}
                {t(AGENT_KEY[a])}
                {!on && busy && <span className="muted small"> · {t("monitor.agents_finishing", { id: busy.id })}</span>}
              </button>
            );
          })}
          <p className="muted small">{t("monitor.agents_menu_note")}</p>
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
