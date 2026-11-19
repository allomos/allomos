// One task changed in this window (Task-A01-parade485): the window shows the change by replacing
// that one task in the list it already has, instead of reading the whole taskbag again (seconds on
// a taskbag with thousands of events). Everything that follows the task (other tasks' "blocked"
// notes, group counts and bars, filters, sorting, "last changed") is worked out from the list, so
// it follows by itself.
//
// Which changes go which way:
// - In place: every edit of one task (state, labels, waiting on, blocked by, next dependency,
//   handoff note, iteration, title, description, folders, difficulty, reopen).
// - The whole taskbag is read again, as before: a change that reached beyond the task (a label
//   new to the label table; Checked, which may write the outside witness's checkpoint); a
//   description change of a task with approvals (whether each still applies is worked out from
//   the history); a taskbag whose files have problems (the change may have mended one); a
//   fingerprint that is not the one this window last saw (another session changed the taskbag
//   too); and every other kind of change: a new task, area or iteration, "New…" iteration and
//   plan in it, renaming or deleting a group, Approve, Withdraw, AI decisions reviewed.

// Only type imports here, so Node can test it.
import type { CardOutside, Evidence, Opened, Task } from "./types";

/** What `edit_task` answers (apps/desktop/src-tauri/src/task_change.rs). */
export interface TaskChange {
  task: Task;
  /** The task's new version. */
  version: string;
  /** When the change was recorded (UTC); null when nothing changed. */
  changed_at: string | null;
  /** The card file its `Card:` link points to outside the taskbag, if any. */
  card_outside: CardOutside | null;
  /** Its Evidence screen, as the core works it out (Task-A01-inlet632). */
  evidence?: Evidence;
  /** The change reached beyond this task: read the whole taskbag again. */
  whole: boolean;
  /** The taskbag's fingerprint just before the change was written, and just after. */
  stamp_before: string;
  stamp_after: string;
}

/**
 * `opened` with the changed task in place, as reading the whole taskbag again would give it; or
 * null when only reading it again can tell (see the list above). `stamp`: the fingerprint this
 * window last saw; `decorated`: stopFlow's, which puts on a task what `opened` holds for it.
 */
export function withTaskChange(
  opened: Opened,
  change: TaskChange,
  stamp: string | null,
  decorated: (task: Task, opened: Opened) => Task,
): Opened | null {
  const box = opened.loaded.taskbag;
  if (!box || change.whole || stamp === null || stamp !== change.stamp_before) return null;
  if (opened.loaded.issues.length > 0) return null;
  const id = change.task.id;
  const at = box.tasks.findIndex((t) => t.id === id);
  if (at < 0) return null;
  const before = box.tasks[at];
  if (before.description !== change.task.description && (opened.approvals?.[id]?.length ?? 0) > 0) return null;

  // An agent's report of a card file wins over the link and is not changed by an edit.
  const cards = opened.cards_outside ?? [];
  const reported = cards.some((c) => c.task === id && c.source !== "link");
  const linked = change.card_outside;
  let cards_outside = cards;
  if (!reported && (linked || cards.some((c) => c.task === id))) {
    // Kept in the tasks' order, as reading again lists them.
    const order = new Map(box.tasks.map((t, i) => [t.id, i]));
    cards_outside = [...cards.filter((c) => c.task !== id), ...(linked ? [linked] : [])].sort(
      (a, b) => (order.get(a.task) ?? 0) - (order.get(b.task) ?? 0),
    );
  }

  const fresh = change.evidence ? { ...opened, evidence: { ...opened.evidence, [id]: change.evidence } } : opened;
  const tasks = box.tasks.slice();
  tasks[at] = decorated(change.task, fresh);
  return {
    ...fresh,
    loaded: { ...opened.loaded, taskbag: { ...box, tasks } },
    versions: { ...opened.versions, [id]: change.version },
    last_changed: change.changed_at ? { ...opened.last_changed, [id]: change.changed_at } : opened.last_changed,
    cards_outside,
  };
}
