// A task's history in the task pane (Task-A01-17): how each recorded change is worded.

import type { Key } from "./i18n";
import type { FieldChange } from "./types";

/** The task pane's name for each task field, so History uses the same words as the fields above it. */
const FIELD_KEYS: Record<string, Key> = {
  title: "pane.title",
  area: "pane.area",
  iteration: "pane.iteration",
  state: "pane.state",
  label: "pane.label",
  holder: "pane.holder",
  waiting_on: "pane.waiting_on",
  who: "pane.who",
  blocked_by: "pane.blocked_by",
  next_dependency: "pane.next",
  handoff_note: "pane.handoff",
  finished_without_owner: "pane.finished_without_owner",
  description: "pane.description",
};

type T = (key: Key, params?: Record<string, string | number>) => string;

/** A field's name in the interface's language; a field Allomos does not name is shown as it is stored. */
export function fieldName(field: string, t: T): string {
  const key = FIELD_KEYS[field];
  return key ? t(key) : field;
}

const SHORT = 60;

/** One side of a change: a state in words, a description only as "changed", long text shortened. */
export function valueText(field: string, value: unknown, t: T): string {
  if (value === undefined || value === null || value === "") return "—";
  if (field === "state" && typeof value === "string") return t(`state.${value}` as Key);
  if (field === "blocked_by" && Array.isArray(value)) return value.length ? value.join(", ") : "—";
  const text = typeof value === "string" ? value : JSON.stringify(value);
  const first = text.split("\n")[0];
  return first.length > SHORT || first.length < text.trimEnd().length ? `${first.slice(0, SHORT)}…` : first;
}

/** "State: Ready → In progress"; a description says only that it changed. */
export function changeLine(c: FieldChange, t: T): string {
  if (c.field === "description") return t("pane.history_description", { field: fieldName(c.field, t) });
  return `${fieldName(c.field, t)}: ${valueText(c.field, c.from, t)} → ${valueText(c.field, c.to, t)}`;
}
