// Why a stopped task is stopped, in words (decision 0016, Task-A01-31): `waiting` means a person
// (or an outside event) must act — "Waiting for you: approve the card", "Waiting for Ann: …";
// `blocked` means another task must finish first — "Blocked by A01-17 (in progress)", each
// blocker with its state now. Also the list's state filter. Nothing here moves a task: when every
// blocker is done the app only says so, as `alm` does.

import type { Key } from "./i18n";
import type { Opened, State, Task } from "./types";

type T = (key: Key, params?: Record<string, string | number>) => string;

/** One task that blocks another, with its state now (`null`: no such task in the taskbag). */
export type Blocker = { id: string; state: State | null };

/** The tasks blocking `task`, in the order it lists them, each with its current state. */
export function blockers(task: Task, tasks: readonly Task[]): Blocker[] {
  return (task.blocked_by ?? []).map((id) => ({ id, state: tasks.find((x) => x.id === id)?.state ?? null }));
}

/** Whether a blocked task's blockers are all done. A dropped blocker never counts as done. */
export function allBlockersDone(task: Task, tasks: readonly Task[]): boolean {
  const list = blockers(task, tasks);
  return task.state === "blocked" && list.length > 0 && list.every((b) => b.state === "done");
}

/** "A01-17 (in progress)"; a dropped blocker is flagged; an unknown one said so. */
export function blockerText(b: Blocker, t: T): string {
  if (b.state === null) return t("task.blocker_missing", { id: b.id });
  if (b.state === "dropped") return t("task.blocker_dropped", { id: b.id });
  return t("task.blocker", { id: b.id, state: t(`state.${b.state}` as Key) });
}

/** The line a stopped task shows in the list and the task pane; `null` when it is not stopped. */
export function stopText(task: Task, tasks: readonly Task[], t: T): string | null {
  if (task.state === "waiting") {
    const what = task.waiting_on ?? "";
    return task.who ? t("task.waiting_for", { who: task.who, what }) : t("task.waiting_for_you", { what });
  }
  if (task.state === "blocked") {
    const list = blockers(task, tasks)
      .map((b) => blockerText(b, t))
      .join(", ");
    const line = t("task.blocked_by", { list });
    return allBlockersDone(task, tasks) ? `${line} — ${t("task.blockers_done")}` : line;
  }
  return null;
}

/** Finished without the human check and not checked since: "Not human-checked" (Task-A03-18).
 * A task still carrying the note's name from before allomos/4 counts too. */
export function toCheck(task: Task): boolean {
  return task.state === "done" && !!(task.without_human_check || task.finished_without_owner);
}

/** Why it was finished without the human check (the note, under either name). */
export function humanCheckNote(task: Task): string {
  return task.without_human_check ?? task.finished_without_owner ?? "";
}

/** An AI agent decided something here on its own, and nobody has reviewed it yet: "AI decided"
 * (Task-A03-19). */
export function toReview(task: Task): boolean {
  return (task.ai_decisions?.length ?? 0) > 0;
}

/** The opened taskbag with each task's AI decisions to review put on the task (Task-A03-19). */
export function withDecisions(opened: Opened): Opened {
  const box = opened.loaded.taskbag;
  const all = opened.ai_decisions ?? {};
  if (!box) return opened;
  const tasks = box.tasks.map((task) => (all[task.id] ? { ...task, ai_decisions: all[task.id] } : task));
  return { ...opened, loaded: { ...opened.loaded, taskbag: { ...box, tasks } } };
}

/** The list's state filter: every state, one, or every task that needs a person: to check, to
 * answer (waiting) or to review (AI decisions), as `alm human` lists them (Task-A03-18, A03-19). */
export type StateFilter = "all" | "human" | State;

export const FILTER_STATES: State[] = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"];

/** The tasks the filter keeps. */
export function keepState(filter: StateFilter, task: Task): boolean {
  if (filter === "human") return toCheck(task) || task.state === "waiting" || toReview(task);
  return filter === "all" || task.state === filter;
}

/** A stored filter read back; anything else is "all". */
export function readFilter(value: string | null): StateFilter {
  if (value === "human") return "human";
  return value && (FILTER_STATES as string[]).includes(value) ? (value as State) : "all";
}

/** What the filter leaves out of a list of tasks: how many, and how many in each state, in the
 * filter's order of states (Task-A01-42). Nothing is left out when the filter is "all". */
export type Hidden = { count: number; states: { state: State; count: number }[] };

export function hiddenByFilter(filter: StateFilter, tasks: readonly Task[]): Hidden {
  const hidden = tasks.filter((task) => !keepState(filter, task));
  const states = FILTER_STATES.map((state) => ({ state, count: hidden.filter((x) => x.state === state).length })).filter(
    (s) => s.count > 0,
  );
  return { count: hidden.length, states };
}

/** "38 more hidden by the filter: 1 Waiting, 4 Ready, 33 Done". */
export function hiddenText(hidden: Hidden, t: T): string {
  const states = hidden.states.map((s) => t("filter.state_count", { count: s.count, state: t(`state.${s.state}` as Key) })).join(t("filter.list_sep"));
  return t("filter.hidden", { count: hidden.count, states });
}

/** The filter's name as the Show box words it. */
export function filterName(filter: StateFilter, t: T): string {
  if (filter === "all") return t("filter.all");
  if (filter === "human") return t("filter.human");
  return t(`state.${filter}` as Key);
}

/** What a filtered list is grouped by: areas, iterations, or handoffs (each the group of the tasks
 * it names) (Task-A01-44). */
export type GroupKind = "area" | "iteration" | "handoff";

/**
 * The groups as the filter shows them (Task-A01-44): each with only the tasks the filter keeps,
 * `counted` keeping its whole list for the count and progress, and groups with none left out. With
 * the filter off every group stays as it was.
 */
export function filterGroups<G extends { tasks: Task[]; counted?: Task[] }>(groups: G[], filter: StateFilter): G[] {
  if (filter === "all") return groups;
  return groups
    .map((g) => ({ ...g, counted: g.counted ?? g.tasks, tasks: g.tasks.filter((task) => keepState(filter, task)) }))
    .filter((g) => g.tasks.length > 0);
}

/** How many different tasks the groups show: a task named by two handoffs counts once. */
export function taskCount(groups: readonly { tasks: readonly Task[] }[]): number {
  return new Set(groups.flatMap((g) => g.tasks.map((task) => task.id))).size;
}

/** Each handoff, by its file, as a group of the tasks it names that are in the taskbag. */
export function handoffGroups(handoffs: readonly { file: string; tasks: string[] }[], tasks: readonly Task[]): { file: string; tasks: Task[] }[] {
  return handoffs.map((h) => ({ file: h.file, tasks: h.tasks.flatMap((id) => tasks.filter((task) => task.id === id)) }));
}

/** A stretch of a banner's words; a `strong` one is shown in bold. */
export type Words = { text: string; strong?: boolean }[];

/** What the filter banner counts (Task-A01-44). */
export type FilterBannerCount = {
  kind: GroupKind;
  /** Different tasks the filter shows. */
  tasks: number;
  /** Groups drawn. */
  groups: number;
  /** Groups there were before the filter (or, while searching, before the filter left any out). */
  total: number;
  /** While a search is on in a task list: how many tasks it found that the filter hides. */
  searchHidden?: number;
};

const MARK = "\u0000";

/** `text` with the one `who` placed where its template put the mark, in bold. */
function withWho(text: string, who: string): Words {
  const at = text.indexOf(MARK);
  if (at < 0) return [{ text }];
  return [{ text: text.slice(0, at) }, { text: who, strong: true }, { text: text.slice(at + MARK.length) }].filter((w) => w.text);
}

const one = (n: number) => (n === 1 ? "one" : "other");

/**
 * The filter banner's words (Task-A01-44): "**3 tasks** need a human (…), in 2 of 7 areas. The 5
 * areas with none are left out."; with nothing to show "No task is blocked. All 12 iterations are
 * left out."; while searching "**1 of them** needs a human, in 1 area. 1 more is listed below,
 * hidden by the filter." Empty for the filter "all".
 */
export function filterBanner(filter: StateFilter, c: FilterBannerCount, t: T): Words {
  if (filter === "all") return [];
  const verb = t(`filter.verb.${filter}_${one(c.tasks === 0 ? 1 : c.tasks)}` as Key);
  const searching = c.searchHidden !== undefined;
  const out: Words = [];
  const add = (words: Words) => {
    // Sentences ending in a full-width stop (Chinese, Japanese) need no space after it.
    if (out.length > 0 && !out[out.length - 1].text.endsWith("。")) out.push({ text: " " });
    out.push(...words);
  };
  if (c.tasks === 0) {
    if (searching) {
      add([{ text: t("filter.banner.none_search", { verb }) }]);
      return out;
    }
    add([{ text: t("filter.banner.none", { verb }) }]);
    if (c.total > 0) add([{ text: t(`filter.all_left.${c.kind}_${one(c.total)}` as Key, { count: c.total }) }]);
    return out;
  }
  const where =
    searching || c.groups >= c.total
      ? t(`filter.where.${c.kind}_${one(c.groups)}` as Key, { count: c.groups })
      : t(`filter.where_of.${c.kind}` as Key, { shown: c.groups, total: c.total });
  if (searching) {
    const who = t("filter.who_of_them", { count: c.tasks });
    add(withWho(t("filter.banner.lead_search", { who: MARK, verb, where }), who));
    if (c.searchHidden! > 0) add([{ text: t(`filter.below_${one(c.searchHidden!)}` as Key, { count: c.searchHidden! }) }]);
    return out;
  }
  const who = t(`filter.who_${one(c.tasks)}` as Key, { count: c.tasks });
  add(withWho(t("filter.banner.lead", { who: MARK, verb, where }), who));
  const left = c.total - c.groups;
  if (left > 0) add([{ text: t(`filter.left.${c.kind}_${one(left)}` as Key, { count: left }) }]);
  return out;
}

/** The search banner's words (Task-A01-44): "**2 tasks** found for “blocked”." */
export function searchBanner(kind: "task" | "handoff", count: number, query: string, t: T): Words {
  const who = t(`search.who.${kind}_${one(count)}` as Key, { count });
  return withWho(t("search.banner", { who: MARK, query }), who);
}
