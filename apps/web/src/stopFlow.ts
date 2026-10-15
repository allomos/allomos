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

/** Finished without the human check and not checked since: "Not human-checked" (Task-A03-18). */
export function toCheck(task: Task): boolean {
  return task.state === "done" && !!task.without_human_check;
}

/** Why it was finished without the human check (the note). */
export function humanCheckNote(task: Task): string {
  return task.without_human_check ?? "";
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
  /** Groups there were before the filter. */
  total: number;
};

const MARK = "\u0000";

/** `text` with the one `who` placed where its template put the mark, in bold. */
function withWho(text: string, who: string): Words {
  const at = text.indexOf(MARK);
  if (at < 0) return [{ text }];
  return [{ text: text.slice(0, at) }, { text: who, strong: true }, { text: text.slice(at + MARK.length) }].filter((w) => w.text);
}

const one = (n: number) => (n === 1 ? "one" : "other");

/** Sentences joined: a space after each, except after a full-width stop (Chinese, Japanese). */
function sentences(...parts: Words[]): Words {
  const out: Words = [];
  for (const words of parts) {
    if (words.length === 0) continue;
    if (out.length > 0 && !out[out.length - 1].text.endsWith("。")) out.push({ text: " " });
    out.push(...words);
  }
  return out;
}

/**
 * The filter banner's words (Task-A01-44): "**3 tasks** need a human (…), in 2 of 7 areas. The 5
 * areas with none are left out."; with nothing to show "No task is blocked. All 12 iterations are
 * left out." Empty for the filter "all". While a search is on, the task list uses `bothBanner`.
 */
export function filterBanner(filter: StateFilter, c: FilterBannerCount, t: T): Words {
  if (filter === "all") return [];
  const verb = t(`filter.verb.${filter}_${one(c.tasks === 0 ? 1 : c.tasks)}` as Key);
  if (c.tasks === 0) {
    return sentences(
      [{ text: t("filter.banner.none", { verb }) }],
      c.total > 0 ? [{ text: t(`filter.all_left.${c.kind}_${one(c.total)}` as Key, { count: c.total }) }] : [],
    );
  }
  const where =
    c.groups >= c.total
      ? t(`filter.where.${c.kind}_${one(c.groups)}` as Key, { count: c.groups })
      : t(`filter.where_of.${c.kind}` as Key, { shown: c.groups, total: c.total });
  const who = t(`filter.who_${one(c.tasks)}` as Key, { count: c.tasks });
  const left = c.total - c.groups;
  return sentences(
    withWho(t("filter.banner.lead", { who: MARK, verb, where }), who),
    left > 0 ? [{ text: t(`filter.left.${c.kind}_${one(left)}` as Key, { count: left }) }] : [],
  );
}

/** What the banner counts when the filter and a search narrow the task list together (Task-A01-48). */
export type BothCount = {
  kind: "area" | "iteration";
  /** The search's words, as typed. */
  query: string;
  /** Tasks listed: those that pass the filter and hold the search words. */
  tasks: number;
  /** Groups drawn. */
  groups: number;
  /** Tasks the search finds, filter aside. */
  matches: number;
};

/**
 * The filter banner while a search is on (Task-A01-48): the list is the filter and the search
 * together. "Filter and search together: only tasks that are waiting and hold “alm” are listed, in
 * 1 area. 3 more matches are hidden by the filter."; with none left "None of the tasks found for
 * “alm” is waiting. All 3 matches are hidden by the filter." Empty for the filter "all", or when
 * the search finds nothing at all ("Nothing found" says so).
 */
export function bothBanner(filter: StateFilter, c: BothCount, t: T): Words {
  if (filter === "all" || c.matches === 0) return [];
  const hidden = c.matches - c.tasks;
  const hiddenWords: Words =
    hidden > 0 ? [{ text: t(`filter.matches_hidden.${c.tasks === 0 ? "all" : "more"}_${one(hidden)}` as Key, { count: hidden }) }] : [];
  if (c.tasks === 0) {
    const verb = t(`filter.verb.${filter}_one` as Key);
    return sentences([{ text: t("filter.banner.none_both", { query: c.query, verb }) }], hiddenWords);
  }
  const verb = t(`filter.verb.${filter}_other` as Key);
  const where = t(`filter.where.${c.kind}_${one(c.groups)}` as Key, { count: c.groups });
  return sentences([{ text: t("filter.banner.both", { query: c.query, verb, where }) }], hiddenWords);
}

/** The search banner's words (Task-A01-44): "**2 tasks** found for “blocked”."; under a Show filter
 * (`filter`, its name) "**2 tasks** found for “blocked” with the “Waiting” filter on." (Task-A01-48). */
export function searchBanner(kind: "task" | "handoff", count: number, query: string, t: T, filter?: string): Words {
  const who = t(`search.who.${kind}_${one(count)}` as Key, { count });
  if (filter !== undefined) return withWho(t("search.banner_filtered", { who: MARK, query, filter }), who);
  return withWho(t("search.banner", { who: MARK, query }), who);
}

/**
 * Both banners of the task list (By area, By iteration) (Task-A01-44, A01-48). `groups` are every
 * group, `filtered` those the filter keeps, `found` those the filter and the search keep together;
 * `matches` is how many tasks the search finds, filter aside. Clearing the search gives the filter
 * banner alone; turning the filter off gives the search banner alone.
 */
export function taskListBanners(
  o: {
    kind: "area" | "iteration";
    filter: StateFilter;
    /** The search's words, as typed; empty when not searching. */
    query: string;
    groups: readonly { tasks: readonly Task[] }[];
    filtered: readonly { tasks: readonly Task[] }[];
    found: readonly { tasks: readonly Task[] }[];
    matches: number;
  },
  t: T,
): { search: Words; filter: Words } {
  const query = o.query.trim();
  if (!query) {
    return { search: [], filter: filterBanner(o.filter, { kind: o.kind, tasks: taskCount(o.filtered), groups: o.filtered.length, total: o.groups.length }, t) };
  }
  if (o.filter === "all") return { search: o.matches > 0 ? searchBanner("task", o.matches, query, t) : [], filter: [] };
  const shown = taskCount(o.found);
  return {
    search: shown > 0 ? searchBanner("task", shown, query, t, filterName(o.filter, t)) : [],
    filter: bothBanner(o.filter, { kind: o.kind, query, tasks: shown, groups: o.found.length, matches: o.matches }, t),
  };
}
