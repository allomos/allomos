// Grouping and presentation helpers for the unshared app changes notice (Task-A01-cliff402).

import type { AppChanges, Loaded } from "./types";

/** One target (a task, area, iteration, or the project itself) with its unshared direct app edits. */
export interface AppSharingItem {
  /** Stable key for lists (`id`, or `"__project__"` for project-level edits). */
  key: string;
  /** Task, area or iteration ID; `null` when the change was on the project itself. */
  id: string | null;
  /** Title or name looked up from the open taskbag, when available. */
  title: string | null;
  /** Distinct field names changed across the combined events, in first-seen order. */
  fields: string[];
  /** Number of direct app edit events combined into this item. */
  count: number;
  /** ISO timestamp of the newest edit for this item. */
  latestAt: string;
}

/** Direct edits made in the desktop app, grouped together for the notice. */
export interface AppSharingGroup {
  kind: "app_direct";
  /** Total unshared direct app edit events. */
  count: number;
  /** Changed targets, newest edited first. */
  items: AppSharingItem[];
}

type TaskbagMeta = Pick<NonNullable<Loaded["taskbag"]>, "project" | "tasks" | "areas" | "iterations"> | null | undefined;

/**
 * Groups unshared changes made directly in the app into one section, merging repeated edits to
 * the same task, area, iteration or project so the user can scan what they edited at a glance.
 */
export function groupAppChanges(app: AppChanges | null | undefined, taskbag?: TaskbagMeta): AppSharingGroup | null {
  if (!app || app.count <= 0) return null;
  const names = new Map<string, string>();
  if (taskbag) {
    for (const task of taskbag.tasks) names.set(task.id, task.title);
    for (const area of taskbag.areas) names.set(area.id, area.name);
    for (const iter of taskbag.iterations) names.set(iter.id, iter.name);
  }
  const projectId = taskbag?.project.id ?? null;
  const byTarget = new Map<string, AppSharingItem>();
  for (const entry of app.entries) {
    const isProject = !entry.id || (projectId !== null && entry.id === projectId);
    const id = isProject ? null : entry.id;
    const key = id ?? "__project__";
    let item = byTarget.get(key);
    if (!item) {
      item = {
        key,
        id,
        title: id ? (names.get(id) ?? null) : null,
        fields: [],
        count: 0,
        latestAt: entry.at,
      };
      byTarget.set(key, item);
    }
    item.count += 1;
    for (const field of entry.fields) {
      if (!item.fields.includes(field)) item.fields.push(field);
    }
    if (entry.at > item.latestAt) item.latestAt = entry.at;
  }
  const items = [...byTarget.values()].sort((a, b) => (a.latestAt < b.latestAt ? 1 : a.latestAt > b.latestAt ? -1 : 0));
  return { kind: "app_direct", count: app.count, items };
}
