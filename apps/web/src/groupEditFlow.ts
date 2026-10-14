// Renaming an area, changing an iteration, and deleting either, from its group in the list
// (Task-A01-29). Rust decides whether the change is allowed, by the same rules as `alm rename area`,
// `alm change iteration` and `alm delete`; nothing here refuses one (a delete of a group with tasks
// is sent, and Rust's refusal is shown).

import type { Area, Iteration, IterationChange } from "./types";

/** Which group's Edit form is open. */
export type Editing = { kind: "area"; id: string } | { kind: "iteration"; id: string };

/** What the person typed in the Edit form. An area uses only `name`. */
export type GroupDraft = { name: string; release: string; start: string; end: string };

/** The Edit form's start: the group as it is now. */
export function startGroupEdit(item: Area | Iteration): GroupDraft {
  const it = item as Partial<Iteration>;
  return { name: item.name, release: it.release ?? "", start: it.start ?? "", end: it.end ?? "" };
}

/** Whether Save is offered: there is a name, and something differs from the group as it is. */
export function canSaveGroup(kind: Editing["kind"], draft: GroupDraft, item: Area | Iteration): boolean {
  if (draft.name.trim() === "") return false;
  const now = startGroupEdit(item);
  if (kind === "area") return draft.name.trim() !== now.name;
  return (["name", "release", "start", "end"] as const).some((k) => draft[k].trim() !== now[k]);
}

/** The iteration change Rust is asked for: every field, an empty release or date clearing it. */
export function toIterationChange(draft: GroupDraft): IterationChange {
  return { name: draft.name.trim(), release: draft.release.trim(), start: draft.start.trim(), end: draft.end.trim() };
}
