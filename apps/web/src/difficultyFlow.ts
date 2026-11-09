// A task's difficulty (Task-A01-actor813, approver 2026-11-08): 1 (very easy) to 5 (very hard),
// filled in by the agent that adds the task. A task without one counts as 3, shown as the default;
// its file is not changed. Rust checks a level when it is saved, by the same rule as `alm difficulty`.

import type { Key } from "./i18n";
import type { Task } from "./types";

/** The levels, lowest first. */
export const LEVELS = [1, 2, 3, 4, 5] as const;

/** The level of a task that has none. */
export const DEFAULT_LEVEL = 3;

/** The task's level and whether it is the default (none written, or a value that is not a level). */
export function difficultyOf(task: Pick<Task, "difficulty">): { level: number; isDefault: boolean } {
  const n = task.difficulty;
  return typeof n === "number" && Number.isInteger(n) && n >= 1 && n <= 5 ? { level: n, isDefault: false } : { level: DEFAULT_LEVEL, isDefault: true };
}

type T = (key: Key, params?: Record<string, string | number>) => string;

/** A level in words: "3 · Ordinary". */
export function levelText(level: number, t: T): string {
  return t(`difficulty.level.${level}` as Key);
}

/** The task's difficulty as the list and the pane show it: "4 · Hard", or "3 · Ordinary (default)". */
export function difficultyText(task: Pick<Task, "difficulty">, t: T): string {
  const { level, isDefault } = difficultyOf(task);
  return isDefault ? t("difficulty.default", { level: levelText(level, t) }) : levelText(level, t);
}
