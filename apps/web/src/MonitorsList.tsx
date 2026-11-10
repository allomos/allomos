// The Monitors list (Task-A03-motor529, approver 2026-11-09): the button at the right end of the tab
// bar, shown only while a monitoring run goes on on this computer, and the list it opens, or the
// View menu's Running Monitors item opens. Each run is one row, in any project: its iteration's
// name is a link to that iteration in its project, Check now and Stop act on it as its status bar
// does, and a run whose record is missing
// can only be stopped (by its process). The list reads the run files every couple of seconds.

import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties, RefObject } from "react";
import { useI18n } from "./i18n";
import type { Messenger, MonitorRequest } from "./messenger";
import { monitorAgentName } from "./AgentPicker";
import { freeAgents, listedRuns, nextCheck, type ListedRun, type Run } from "./monitorFlow";
import { askJump } from "./iterationJumpFlow";
import { isProblem } from "./types";
import { useAgentAccounts } from "./useAgentAccounts";

/** How often the runs are read again. */
const READ_MS = 2_000;
/** How often the clock on the rows moves on. */
const TICK_MS = 15_000;

/** The runs on this computer, read every couple of seconds, with the time for the rows. */
export function useListedRuns(messenger: Messenger) {
  // The extra accounts' names for the labels (Task-A03-oven548).
  useAgentAccounts(messenger);
  const [rows, setRows] = useState<ListedRun[]>([]);
  const [now, setNow] = useState(() => Date.now());
  const last = useRef("");

  const read = useCallback(async () => {
    let list: unknown[];
    try {
      list = await messenger.monitorAll();
    } catch {
      return;
    }
    // Only a change re-renders the bar: the list is read often.
    const text = JSON.stringify(list);
    if (text === last.current) return;
    last.current = text;
    setRows(listedRuns(list));
    setNow(Date.now());
  }, [messenger]);

  useEffect(() => {
    void read();
    const timer = window.setInterval(() => void read(), READ_MS);
    return () => window.clearInterval(timer);
  }, [read]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(timer);
  }, []);

  return { rows, now, read };
}

/** The button at the right end of the tab bar: green, with the number of runs. */
export function MonitorsButton({
  count,
  open,
  buttonRef,
  onToggle,
}: {
  count: number;
  open: boolean;
  buttonRef: RefObject<HTMLButtonElement | null>;
  onToggle: () => void;
}) {
  const { t } = useI18n();
  return (
    <button
      ref={buttonRef}
      type="button"
      className={`tabbar-monitors${open ? " open" : ""}`}
      aria-haspopup="dialog"
      aria-expanded={open}
      onClick={onToggle}
    >
      <span className="dot" aria-hidden="true" />
      {t("monitors.button", { n: count })}
    </button>
  );
}

function timeOf(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** The list itself: a panel under the tab bar's button (or the top right, from the menu). */
export function MonitorsList({
  rows,
  now,
  anchor,
  buttonRef,
  messenger,
  onClose,
  onChanged,
}: {
  rows: ListedRun[];
  now: number;
  /** The button's place when the list opened; the panel hangs below it. */
  anchor: DOMRect | null;
  buttonRef: RefObject<HTMLButtonElement | null>;
  messenger: Messenger;
  onClose: () => void;
  /** Reads the runs again after an action. */
  onChanged: () => void;
}) {
  const { t, problem } = useI18n();
  const panel = useRef<HTMLDivElement>(null);
  /** What the last action said went wrong, in words already written; kept until the next action. */
  const [error, setError] = useState<string | null>(null);

  // Escape, or a press outside the panel and its button, closes the list.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const press = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (target && (panel.current?.contains(target) || buttonRef.current?.contains(target))) return;
      onClose();
    };
    document.addEventListener("keydown", key);
    document.addEventListener("pointerdown", press);
    return () => {
      document.removeEventListener("keydown", key);
      document.removeEventListener("pointerdown", press);
    };
  }, [onClose, buttonRef]);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : String(e));
  const done = () => {
    setError(null);
    onChanged();
  };
  const ask = (id: string, request: MonitorRequest) => {
    messenger.monitorAsk(id, request).then(done).catch(fail);
  };
  const place: CSSProperties = anchor
    ? { top: anchor.bottom + 6, right: Math.max(8, window.innerWidth - anchor.right) }
    : { top: 44, right: 8 };

  return (
    <div ref={panel} className="monitors-pop" role="dialog" aria-label={t("monitors.title")} style={place}>
      <div className="monitors-head">
        <h3>{t("monitors.title")}</h3>
        <button type="button" className="link" aria-label={t("monitors.close")} title={t("monitors.close")} onClick={onClose}>
          ×
        </button>
      </div>
      {rows.length === 0 ? (
        <p className="muted small monitors-none">{t("monitors.none")}</p>
      ) : (
        <ul className="monitors-rows">
          {rows.map((row) => (
            <li key={row.key} className={`monitors-row${row.run?.stopping ? " stopping" : ""}${row.lost ? " lost" : ""}`}>
              {row.run ? (
                <RunRow run={row.run} name={row.name} now={now} onIteration={row.root ? (id) => askJump({ root: row.root!, id }, () => messenger.monitorOpen(row.root ?? "")).then(onClose).catch(fail) : undefined} onCheck={() => ask(row.run!.id, { action: "check" })} onStop={() => ask(row.run!.id, { action: row.run!.stopping ? "undo_stop" : "stop" })} />
              ) : (
                <div className="monitors-lines">
                  <strong>{t("monitors.missing", { pid: row.lost?.pid ?? 0 })}</strong>
                  <div className="row-actions">
                    <button type="button" className="small stop" onClick={() => messenger.monitorEndProcess(row.lost?.pid ?? 0).then(done).catch(fail)}>
                      {t("monitor.stop")}
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {error && (
        <p className="monitors-error small" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

/** One run: its project and what it covers (an iteration's name is a link to it), a line per agent, and its actions. */
function RunRow({
  run,
  name,
  now,
  onIteration,
  onCheck,
  onStop,
}: {
  run: Run;
  name: string | null;
  now: number;
  /** Shows the run's iteration in its project; not given when the project is not known. */
  onIteration?: (iteration: string) => void;
  onCheck: () => void;
  onStop: () => void;
}) {
  const { t } = useI18n();
  const free = freeAgents(run);
  const iterationName = run.scope.kind === "iteration" ? run.scope.id : null;
  const lines: string[] = [];
  if (run.stopping && run.working.length) {
    lines.push(t("monitor.stopping", { id: run.working.map((w) => w.id).join(", ") }));
  } else {
    for (const a of run.agents) {
      const w = run.working.find((x) => x.agent === a);
      const agent = monitorAgentName(a, t);
      if (w) lines.push(`${agent} · ${w.id}`);
      else if (nextCheck(run) <= now) lines.push(`${agent}：${t("monitors.checking")}`);
      else lines.push(`${agent}：${t("monitors.waiting", { time: timeOf(nextCheck(run)) })}`);
    }
  }
  return (
    <div className="monitors-lines">
      <div>
        <strong>{name ?? ""}</strong>{" "}
        <span className="muted">
          · {run.scope.kind === "all" ? t("monitor.scope_all") : t("monitors.iteration_word")}{" "}
          {iterationName !== null &&
            (onIteration ? (
              <button type="button" className="link monitors-iteration" title={t("monitors.iteration_show", { id: iterationName })} onClick={() => onIteration(iterationName)}>
                {iterationName}
                <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true">
                  <path d="M6 4h6v6M12 4 4 12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
            ) : (
              iterationName
            ))}
        </span>
      </div>
      {lines.map((line) => (
        <div key={line} className="muted small">
          {line}
        </div>
      ))}
      <div className="row-actions">
        {!run.stopping && free.length > 0 && (
          <button type="button" className="small" onClick={onCheck}>
            {t("monitor.check_now")}
          </button>
        )}
        <button type="button" className={run.stopping ? "small" : "small stop"} onClick={onStop}>
          {run.stopping ? t("monitor.undo_stop") : t("monitor.stop")}
        </button>
      </div>
    </div>
  );
}
