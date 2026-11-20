// Clickable task IDs inside a monitor panel (A03-jazz112, A03-scale435): reuses the panel's
// `.monitor-task-id` button when the task still exists in the project, and renders plain text
// otherwise so deleted or missing tasks never show a dead link.

import { canOpenMonitorTask, splitTaskIdPhrase } from "./monitorFlow";
import type { Task } from "./types";

/** Sentinel passed into translated strings (`monitor.let_go`, `monitor.task_lost`) so task IDs can
 * be rendered as interactive buttons regardless of word order in any language. */
export const TASK_IDS_PLACEHOLDER = "\0ALLOMOS_TASK_IDS\0";

export function MonitorTaskId({
  id,
  tasks,
  onOpenTask,
}: {
  id: string;
  tasks: readonly Pick<Task, "id">[];
  onOpenTask: (id: string) => void;
}) {
  if (!canOpenMonitorTask(id, tasks)) {
    return <strong>{id}</strong>;
  }
  return (
    <button type="button" className="monitor-task-id" onClick={() => onOpenTask(id)}>
      {id}
    </button>
  );
}

export function MonitorTaskPhrase({
  phrase,
  ids,
  tasks,
  onOpenTask,
}: {
  phrase: string;
  ids: readonly string[];
  tasks: readonly Pick<Task, "id">[];
  onOpenTask: (id: string) => void;
}) {
  const [before, after] = splitTaskIdPhrase(phrase, TASK_IDS_PLACEHOLDER);
  return (
    <>
      {before}
      {ids.map((id, index) => (
        <span key={`${id}-${index}`}>
          {index > 0 && ", "}
          <MonitorTaskId id={id} tasks={tasks} onOpenTask={onOpenTask} />
        </span>
      ))}
      {after}
    </>
  );
}
