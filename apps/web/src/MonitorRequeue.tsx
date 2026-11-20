import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Run } from "./monitorFlow";
import type { Task } from "./types";

/** A request being written is not an acknowledgement: wait for the monitor's next record. */
export function MonitorRequeue({ run, tasks, requeue, onError }: {
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
  return <>{ids.map((id) => {
    const task = tasks.find((task) => task.id === id);
    const working = run.working.some((work) => work.id === id);
    const current = working || (task && (task.state !== "ready" || task.holder || task.open_blockers?.length));
    const unavailable = !task || (task.state !== "ready" && task.state !== "in_progress") || !!task.open_blockers?.length;
    return <div key={id} className="monitor-stalled small" role="status">
      <strong>{id}</strong>{" · "}
      {requests[id] === "sent" && run.givenUp.includes(id)
        ? t("monitor.requeue_sending")
        : !run.givenUp.includes(id)
          ? working ? t("monitor.requeue_working") : unavailable ? t("monitor.requeue_unavailable") : t("monitor.requeue_waiting")
          : t("monitor.requeue_stopped")}
      {current && task && <span>{" · "}{t("monitor.requeue_state", { state: t(`state.${task.open_blockers?.length ? "blocked" : task.state}`) })}</span>}
      {run.givenUp.includes(id) && <>{" "}<button type="button" className="small" disabled={!!requests[id] || run.stopping} onClick={() => send(id)}>
        {t("monitor.requeue")}
      </button></>}
    </div>;
  })}</>;
}
