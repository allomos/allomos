// A task's risk in the list and the pane (Task-A01-garden385, decision 0027): high, medium or low,
// recorded by the agent that worked on the task (`alm classify`). It lives in the history, not in
// task.json, and is put on the task when the taskbag is opened. Allomos only shows it: a task
// with none reads "Not classified", it is never guessed.

import type { Key } from "./i18n";
import type { Task } from "./types";

/** The levels, lowest first (crates/core/src/classify.rs). */
export const RISKS = ["low", "medium", "high"] as const;
export type Risk = (typeof RISKS)[number];

/** The task's risk; null when none is recorded (or the value is not a level). */
export function riskOf(task: Pick<Task, "risk">): Risk | null {
  return RISKS.find((r) => r === task.risk) ?? null;
}

type T = (key: Key, params?: Record<string, string | number>) => string;

/** The risk as the list and the pane show it: "High", or "Not classified". */
export function riskText(task: Pick<Task, "risk">, t: T): string {
  return t(`task.risk.${riskOf(task) ?? "none"}` as Key);
}

/** The class of the risk's word: a high one stands out, an unclassified one is greyed. */
export function riskClass(task: Pick<Task, "risk">): string {
  return `risk risk-${riskOf(task) ?? "none"}`;
}
