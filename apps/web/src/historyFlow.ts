// A task's history in the task pane (Task-A01-17): how each recorded change is worded.

import type { Key } from "./i18n";
import type { EffectRecord, FieldChange } from "./types";

/** The task pane's name for each task field, so History uses the same words as the fields above it. */
const FIELD_KEYS: Record<string, Key> = {
  title: "pane.title",
  area: "pane.area",
  iteration: "pane.iteration",
  state: "pane.state",
  label: "pane.label",
  labels: "pane.label",
  holder: "pane.holder",
  waiting_on: "pane.waiting_on",
  who: "pane.who",
  blocked_by: "pane.blocked_by",
  next_dependency: "pane.next",
  handoff_note: "pane.handoff",
  without_human_check: "pane.without_human_check",
  human_check_steps: "pane.human_check_steps",
  finished_without_owner: "pane.without_human_check",
  ai_decision: "pane.ai_decision",
  approval: "pane.approval",
  human_check: "pane.human_check",
  reopen_note: "pane.reopen_note",
  reason: "pane.reason",
  traces: "pane.traces",
  effect: "pane.effect",
  card: "pane.card",
  card_outside: "pane.card_outside",
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
  // An approval record (Task-A01-coach782) by its ID; older approvals were a text.
  if (field === "approval" && typeof value === "object" && "id" in value) return String((value as { id: unknown }).id);
  // An outside effect (Task-A01-job650): where, what, and what happened once it has a result.
  if (field === "effect" && typeof value === "object") return effectText(value as Pick<EffectRecord, "target" | "action" | "result">, t);
  // A human check (Task-A01-table778): its result, where, who and the person's words.
  if (field === "human_check" && typeof value === "object") return humanCheckText(value as HumanCheck, t);
  if ((field === "blocked_by" || field === "labels" || field === "traces") && Array.isArray(value)) return value.length ? value.join(", ") : "—";
  const text = typeof value === "string" ? value : JSON.stringify(value);
  const first = text.split("\n")[0];
  return first.length > SHORT || first.length < text.trimEnd().length ? `${first.slice(0, SHORT)}…` : first;
}

/** "State: Ready → In progress"; a description says only that it changed. */
export function changeLine(c: FieldChange, t: T): string {
  if (c.field === "description") return t("pane.history_description", { field: fieldName(c.field, t) });
  return `${fieldName(c.field, t)}: ${valueText(c.field, c.from, t)} → ${valueText(c.field, c.to, t)}`;
}

/** A `human_check` record (decision 0021 § 1.1), as stored in a `checked` event. */
export type HumanCheck = {
  checker?: string | null;
  steps_sha256?: string | null;
  result?: string;
  words?: string | null;
  where?: string;
};

/** "Passed (deployed), by p-abc123, steps 0123abcd: "Works"". */
export function humanCheckText(c: HumanCheck, t: T): string {
  const parts = [
    `${t(c.result === "partly" ? "pane.human_check_partly" : "pane.human_check_pass")} (${t(
      c.where === "deployed" ? "pane.human_check_deployed" : "pane.human_check_local",
    )})`,
  ];
  if (c.checker) parts.push(t("pane.human_check_by", { who: c.checker }));
  parts.push(c.steps_sha256 ? t("pane.human_check_steps_sha", { sha: c.steps_sha256.slice(0, 8) }) : t("pane.human_check_no_steps"));
  return c.words ? `${parts.join(", ")}: "${c.words}"` : parts.join(", ");
}

/** An outside effect (Task-A01-job650) in one line: "production: deploy 0.1.43 → deployed", or
 * "… (under way)" before its result. */
export function effectText(e: Pick<EffectRecord, "target" | "action" | "result">, t: T): string {
  const head = `${e.target}: ${e.action}`;
  return e.result ? `${head} → ${e.result}` : `${head} (${t("pane.effect_under_way")})`;
}
