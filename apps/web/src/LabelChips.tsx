// A task's labels as small chips (decision 0020, Task-A01-petal376): in the task list and the task
// pane, every label it carries, built-in ones in the interface language and the project's own as
// written. "AI decisions to review" is left to its own badge. A "by human" label (the human
// check) is drawn solid.

import { useI18n } from "./i18n";
import { BY_HUMAN, chipLabels, labelText } from "./stopFlow";
import type { Task } from "./types";

export function LabelChips({ task }: { task: Task }) {
  const { t } = useI18n();
  const labels = chipLabels(task);
  if (labels.length === 0) return null;
  return (
    <span className="label-chips">
      {labels.map((l) => (
        <span key={l} className={BY_HUMAN.includes(l) ? "label-chip by-human" : "label-chip"}>
          {labelText(l, t)}
        </span>
      ))}
    </span>
  );
}
