// The task pane (Task-A01-16): everything about one task, beside the list. Read only; editing
// arrives later in Iteration 2 and will happen here.

import { useEffect, useRef } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import { LatestHandoff } from "./Handoffs";
import type { Area, Handoff, Iteration, Task } from "./types";

export function TaskPane({
  task,
  area,
  iteration,
  onClose,
  onLink,
  latest,
  onAllHandoffs,
}: {
  task: Task;
  area: Area | undefined;
  iteration: Iteration | undefined;
  onClose: () => void;
  onLink: (href: string) => void;
  /** The task's latest handoff (Task-A03-06). */
  latest: Handoff | null;
  /** Opens the Handoffs tab on this task. */
  onAllHandoffs: () => void;
}) {
  const { t } = useI18n();
  const ref = useRef<HTMLElement>(null);
  // A new task starts at the top of the pane.
  useEffect(() => {
    ref.current?.scrollTo?.({ top: 0 });
  }, [task.id]);

  const iterationText = iteration
    ? iteration.release
      ? `${iteration.name} (${iteration.release})`
      : iteration.name
    : task.iteration
      ? task.iteration
      : t("box.unplanned");
  const rows: [string, string | null][] = [
    [t("pane.area"), area ? `${area.id} ${area.name}` : task.area],
    [t("pane.iteration"), iterationText],
    [t("pane.state"), t(`state.${task.state}`)],
    [t("pane.label"), task.label],
    [t("pane.holder"), task.holder],
    [t("pane.waiting_on"), task.waiting_on],
    [t("pane.next"), task.next_dependency],
    [t("pane.handoff"), task.handoff_note],
  ];

  return (
    <aside className="task-pane" ref={ref} aria-label={t("pane.label_aria", { id: task.id })}>
      <header className="pane-head">
        <h2>
          <span className="id">{task.id}</span> {task.title}
        </h2>
        <button type="button" className="close" onClick={onClose} title={t("pane.close")} aria-label={t("pane.close")}>
          ×
        </button>
      </header>
      <dl className="fields">
        {rows.map(([name, value]) => (
          <div key={name}>
            <dt>{name}</dt>
            <dd className={value ? undefined : "muted"}>{value || "—"}</dd>
          </div>
        ))}
      </dl>
      <section>
        <h3>{t("pane.description")}</h3>
        {task.description.trim() ? (
          <Markdown source={task.description} onLink={onLink} />
        ) : (
          <p className="muted small">{t("pane.no_description")}</p>
        )}
      </section>
      <LatestHandoff handoff={latest} onAll={onAllHandoffs} />
      <section>
        <h3>{t("pane.history")}</h3>
        <p className="muted small">{t("pane.history_none")}</p>
      </section>
    </aside>
  );
}
