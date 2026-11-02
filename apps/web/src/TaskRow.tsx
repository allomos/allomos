// One task in the task list: its ID and title, and what the list says about it. Clicking the row
// (or Enter / Space on it) opens the task pane; clicking it again closes it.

import { useI18n } from "./i18n";
import { DescriptionHit, Highlight } from "./Search";
import { AiDecidedBadge, HumanCheckBadge } from "./TaskPane";
import { toCheck, toReview } from "./stopFlow";
import { localTime } from "./timeFlow";
import type { Task } from "./types";

export function TaskRow({
  task,
  selected,
  onSelect,
  terms,
  stop,
  modified,
  iteration,
  showFolders: chips,
}: {
  task: Task;
  selected: boolean;
  onSelect: (id: string) => void;
  /** The search's words, marked in the row (Task-A01-36); empty when not searching. */
  terms: string[];
  /** Why the task is stopped: "Waiting for you: …", "Blocked by A01-17 (in progress)". */
  stop: string | null;
  /** When the task last changed (UTC), shown at the row's right (Task-A01-52). */
  modified?: string;
  /** The iteration it is planned in, in the by-area view. */
  iteration?: string | null;
  /** The task's folder chips, shown while two or more folders are bound (Task-A01-sketch327). */
  showFolders?: boolean;
}) {
  const { t } = useI18n();
  return (
    <li
      id={`task-${task.id}`}
      className={selected ? "task selected" : "task"}
      tabIndex={0}
      aria-current={selected ? "true" : undefined}
      onClick={() => onSelect(task.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(task.id);
        }
      }}
    >
      <span className="id">
        <Highlight text={task.id} terms={terms} />
      </span>
      <span className="title">
        <Highlight text={task.title} terms={terms} />
      </span>
      <span className="state-cell">
        {toCheck(task) && <HumanCheckBadge task={task} />}
        {toReview(task) && <AiDecidedBadge task={task} />}
        <span className={`state s-${task.state}`}>{t(`state.${task.state}`)}</span>
      </span>
      <span className="meta">
        {chips &&
          ((task.folders ?? []).length > 0 ? (
            (task.folders ?? []).map((f) => (
              <span key={f} className="folder-chip">
                {f}
              </span>
            ))
          ) : (
            <span className="muted">{t("task.no_folder")}</span>
          ))}
        {task.label && <span className="label">{task.label}</span>}
        {iteration && <span>{t("task.iteration", { iteration })}</span>}
        {task.holder && <span>{t("task.held_by", { holder: task.holder })}</span>}
        {stop && <span className={`stop s-${task.state}-text`}>{stop}</span>}
        {task.handoff_note && <span>{t("task.handoff_note", { note: task.handoff_note })}</span>}
        <DescriptionHit text={task.description ?? ""} terms={terms} />
      </span>
      {modified && (
        <time className="modified" dateTime={modified} title={t("task.modified", { time: localTime(modified) })}>
          {localTime(modified)}
        </time>
      )}
    </li>
  );
}
