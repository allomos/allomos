import { useEffect, useRef, useState, type ReactNode } from "react";
import { useI18n } from "./i18n";
import { requeuePlace, type Run } from "./monitorFlow";
import type { Task } from "./types";

/** Requeue (A03-gadget373). A request being written is not an acknowledgement: wait for the
 * monitor's next record. `status(id, found)` is what a row says about a stopped or requeued
 * task, with its one Requeue button, or null when the row keeps its own text. */
export function useRequeue({ run, tasks, requeue, onError }: {
  run: Run;
  tasks: Task[];
  requeue: (id: string) => Promise<void>;
  onError: (error: unknown) => void;
}) {
  const { t } = useI18n();
  const [requests, setRequests] = useState<Record<string, "sent" | "queued">>({});
  const locked = useRef(new Set<string>());
  useEffect(() => {
    setRequests((before) => {
      const next = { ...before };
      let changed = false;
      for (const [id, phase] of Object.entries(before)) {
        if (phase === "sent" && !run.givenUp.includes(id)) {
          next[id] = "queued";
          changed = true;
        } else if (phase === "queued" && run.givenUp.includes(id)) {
          delete next[id];
          locked.current.delete(id);
          changed = true;
        }
      }
      return changed ? next : before;
    });
  }, [run.givenUp]);
  const send = (id: string) => {
    if (locked.current.has(id)) return;
    locked.current.add(id);
    setRequests((before) => ({ ...before, [id]: "sent" }));
    void requeue(id).catch((error) => {
      locked.current.delete(id);
      setRequests((before) => {
        const next = { ...before };
        delete next[id];
        return next;
      });
      onError(error);
    });
  };
  const ids = [...new Set([...run.givenUp, ...Object.keys(requests)])];
  /** `found` is the row's formatted time it was found; a line of its own has none. */
  const status = (id: string, found?: string): ReactNode => {
    const stopped = run.givenUp.includes(id);
    if (!stopped && !requests[id]) return null;
    const task = tasks.find((task) => task.id === id);
    const working = run.working.some((work) => work.id === id);
    const current = working || (task && (task.state !== "ready" || task.holder || task.open_blockers?.length));
    const unavailable = !task || (task.state !== "ready" && task.state !== "in_progress") || !!task.open_blockers?.length;
    const text = stopped && requests[id] !== "sent"
      ? found ? t("monitor.lost_given_up", { time: found }) : t("monitor.requeue_stopped")
      : `${found ? `${t("monitor.requeue_found", { time: found })} · ` : ""}${
        stopped ? t("monitor.requeue_sending") : working ? t("monitor.requeue_working") : unavailable ? t("monitor.requeue_unavailable") : t("monitor.requeue_waiting")}`;
    return <>
      <span className="monitor-stalled small" role="status">{text}</span>
      {current && task && <span className="muted small">{" · "}{t("monitor.requeue_state", { state: t(`state.${task.open_blockers?.length ? "blocked" : task.state}`) })}</span>}
      {stopped && <>{" "}<button type="button" className="small" disabled={!!requests[id] || run.stopping} onClick={() => send(id)}>
        {t("monitor.requeue")}
      </button></>}
    </>;
  };
  /** Stopped or requeued tasks no lower row lists (a session now works a requeued one: its row
   * says so): each gets a line of its own, so its button is never lost. */
  const own = ids.filter((id) => requeuePlace(run, id) === "own" && (run.givenUp.includes(id) || !run.working.some((w) => w.id === id)));
  return { status, own };
}

export function RequeueLines({ ids, status }: { ids: string[]; status: (id: string) => ReactNode }) {
  return <>{ids.map((id) => <div key={id} className="small"><strong>{id}</strong>{" · "}{status(id)}</div>)}</>;
}
