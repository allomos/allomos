// Adding tasks, areas and iterations in the list (Task-A01-19): what each form starts from, how
// the suggested ID follows the area picked, and what is sent to Rust. Rust decides whether the new
// item is allowed, by the same rules as `alm add`; nothing here refuses one.

import type { NewIteration, NewTask, NextIds } from "./types";

/** Which form is open: a new task in a group, or a new area or iteration at the end of the list. */
export type Creating =
  | { kind: "task"; group: string; area: string | null; iteration: string | null }
  | { kind: "area" }
  | { kind: "iteration" };

/** What the person typed in the New task form. `suggested` is the ID offered for the area now picked. */
export type TaskDraft = { id: string; suggested: string; area: string; title: string; description: string; iteration: string };

/** What the person typed in the New area or New iteration form. `suggested` is the ID offered. */
export type ItemDraft = { id: string; suggested: string; name: string; release: string; start: string; end: string };

/** The task ID suggested in `area`: the next free one (empty until known, or with no area). */
export function suggestedTaskId(next: NextIds | null, area: string): string {
  return (area && next?.tasks[area]) || "";
}

/**
 * The New task form's start. In an area group the area is that one; in an iteration group the
 * person picks it (the first area to begin with), and the task is planned in that iteration.
 */
export function startTask(creating: Extract<Creating, { kind: "task" }>, areas: string[], next: NextIds | null): TaskDraft {
  const area = creating.area ?? areas[0] ?? "";
  const suggested = suggestedTaskId(next, area);
  return { id: suggested, suggested, area, title: "", description: "", iteration: creating.iteration ?? "" };
}

/** Another area picked: the suggested ID follows it, unless the person typed their own. */
export function pickArea(draft: TaskDraft, area: string, next: NextIds | null): TaskDraft {
  const suggested = suggestedTaskId(next, area);
  const typed = draft.id.trim() !== "" && draft.id !== draft.suggested;
  return { ...draft, area, suggested, id: typed ? draft.id : suggested };
}

/** New suggestions arrived (the taskbag reloaded): an ID the person did not type follows them. */
export function refreshTask(draft: TaskDraft, next: NextIds | null): TaskDraft {
  return pickArea(draft, draft.area, next);
}

/** The New area or New iteration form's start, with the suggested ID. */
export function startItem(kind: "area" | "iteration", next: NextIds | null): ItemDraft {
  const suggested = (kind === "area" ? next?.area : next?.iteration) ?? "";
  return { id: suggested, suggested, name: "", release: "", start: "", end: "" };
}

/** New suggestions arrived: an ID the person did not type follows them. */
export function refreshItem(draft: ItemDraft, kind: "area" | "iteration", next: NextIds | null): ItemDraft {
  const suggested = startItem(kind, next).suggested;
  const typed = draft.id.trim() !== "" && draft.id !== draft.suggested;
  return { ...draft, suggested, id: typed ? draft.id : suggested };
}

/** Whether the form has what Save needs to be offered: a title or a name. */
export function canAdd(draft: TaskDraft | ItemDraft): boolean {
  return ("title" in draft ? draft.title : draft.name).trim() !== "";
}

const orNull = (text: string) => text.trim() || null;

/** The new task Rust is asked for. An empty ID box means the next free one, as in `alm add task`. */
export function toNewTask(draft: TaskDraft): NewTask {
  return {
    id: orNull(draft.id),
    area: draft.area,
    title: draft.title.trim(),
    description: draft.description.trim() ? draft.description : null,
    iteration: orNull(draft.iteration),
  };
}

/** The new iteration Rust is asked for; empty dates and release are left out. */
export function toNewIteration(draft: ItemDraft): NewIteration {
  return { id: orNull(draft.id), name: draft.name.trim(), release: orNull(draft.release), start: orNull(draft.start), end: orNull(draft.end) };
}
