// Why a stopped task is stopped, in words (decision 0016, Task-A01-31): `waiting` means a person
// (or an outside event) must act — "Waiting for you: approve the card", "Waiting for Ann: …";
// `blocked` means another task must finish first — "Blocked by A01-17 (in progress)", each
// blocker with its state now. Also the list's state filter. Nothing here moves a task: when every
// blocker is done the app only says so, as `alm` does.

import type { Key } from "./i18n";
import type { LabelEntry, Opened, Project, State, Task } from "./types";

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

/** A stopped task's fact in the task list (Task-A01-cactus586): its name ("Waiting on", "Blocked by")
 * on a line of its own, its text under it. A person named to act leads the text ("Waiting for Ann: …");
 * `null` when the task is not stopped. */
export type StopFact = { name: Key; text: string };

export function stopFact(task: Task, tasks: readonly Task[], t: T): StopFact | null {
  if (task.state === "waiting") {
    const what = task.waiting_on ?? "";
    return { name: "pane.waiting_on", text: task.who ? t("task.waiting_for", { who: task.who, what }) : what };
  }
  if (task.state === "blocked") {
    const list = blockers(task, tasks)
      .map((b) => blockerText(b, t))
      .join(", ");
    return { name: "pane.blocked_by", text: allBlockersDone(task, tasks) ? `${list} — ${t("task.blockers_done")}` : list };
  }
  return null;
}

// Labels (decision 0020, Task-A01-petal376): Allomos's five built-in labels, a task's labels, the
// project's label table as the app lists it, To check, and which "by human" label Checked adds.
// Every label is plain: no kind, no order, none excludes another; a task has a label or not.
// The same rules as crates/core/src/labels.rs; Rust decides every change.

export const LOCALLY_BY_AGENT = "locally verified by agent";
export const DEPLOYED_BY_AGENT = "deployed and verified by agent";
export const LOCALLY_BY_HUMAN = "locally verified by human";
export const DEPLOYED_BY_HUMAN = "deployed and verified by human";
export const AI_DECISIONS = "AI decisions to review";

/** The built-in labels in the table's order, each with its words in the interface language. */
const BUILT_IN: { name: string; key: Key; meaning: Key }[] = [
  { name: LOCALLY_BY_AGENT, key: "label.locally_by_agent", meaning: "label.meaning.locally_by_agent" },
  { name: DEPLOYED_BY_AGENT, key: "label.deployed_by_agent", meaning: "label.meaning.deployed_by_agent" },
  { name: LOCALLY_BY_HUMAN, key: "label.locally_by_human", meaning: "label.meaning.locally_by_human" },
  { name: DEPLOYED_BY_HUMAN, key: "label.deployed_by_human", meaning: "label.meaning.deployed_by_human" },
  { name: AI_DECISIONS, key: "label.ai_decisions", meaning: "label.meaning.ai_decisions" },
];

export const BUILT_IN_NAMES: readonly string[] = BUILT_IN.map((b) => b.name);

/** Either one is the human check. */
export const BY_HUMAN: readonly string[] = [LOCALLY_BY_HUMAN, DEPLOYED_BY_HUMAN];

export function isBuiltIn(label: string): boolean {
  return BUILT_IN_NAMES.includes(label);
}

/** The labels a task carries, in the order they were added. */
export function taskLabels(task: Task): string[] {
  return task.labels ?? [];
}

export function hasLabel(task: Task, label: string): boolean {
  return taskLabels(task).includes(label);
}

/** A person has checked it: it carries a "by human" label. */
export function humanChecked(task: Task): boolean {
  return BY_HUMAN.some((l) => hasLabel(task, l));
}

/** To check (decision 0020): a done task with neither "by human" label. Not a label: worked out. */
export function toCheck(task: Task): boolean {
  return task.state === "done" && !humanChecked(task);
}

/** The task pane's Check row: a person checked it (a "by human" label on a done task), it waits
 * for one (To check), or not yet (not done). */
export type Check = "checked" | "to_check" | "not_yet";

export function check(task: Task): Check {
  if (task.state !== "done") return "not_yet";
  return humanChecked(task) ? "checked" : "to_check";
}

/** An agent decided things here that wait for review: the task carries "AI decisions to review". */
export function toReview(task: Task): boolean {
  return hasLabel(task, AI_DECISIONS);
}

/** The "by human" label Checked adds, as `alm checked` decides it: "deployed and verified by
 * human" when the task carries "deployed and verified by agent", else "locally verified by human". */
export function checkedLabel(task: Task): string {
  return hasLabel(task, DEPLOYED_BY_AGENT) ? DEPLOYED_BY_HUMAN : LOCALLY_BY_HUMAN;
}

/** The project's label table as the app lists it: the built-in labels first, in their own order,
 * then the others as the table lists them, then any label a task carries that the table does not
 * list yet (Rust adds it on the next change). Each name once. */
export function labelTable(project: Pick<Project, "labels"> | undefined, tasks: readonly Task[] = []): LabelEntry[] {
  // A taskbag read before `allomos/7` may still send plain names.
  const table = (project?.labels ?? []).map((e): LabelEntry => (typeof e === "string" ? { name: e } : e));
  const out: LabelEntry[] = BUILT_IN.map((b) => ({ ...(table.find((e) => e.name === b.name) ?? { name: b.name }), built_in: true }));
  const add = (e: LabelEntry) => {
    if (!out.some((o) => o.name === e.name)) out.push({ ...e, built_in: false });
  };
  table.forEach(add);
  tasks.forEach((task) => taskLabels(task).forEach((name) => add({ name })));
  return out;
}

/** The names of the table, built-in first. */
export function labelNames(project: Pick<Project, "labels"> | undefined, tasks: readonly Task[] = []): string[] {
  return labelTable(project, tasks).map((e) => e.name);
}

/** A label as the app shows it: a built-in one in the interface language, others as written. */
export function labelText(label: string, t: T): string {
  const b = BUILT_IN.find((x) => x.name === label);
  return b ? t(b.key) : label;
}

/** A label's meaning as the settings show it: a built-in one in the interface language, a custom
 * one as the project wrote it (empty when it wrote none). */
export function labelMeaning(entry: LabelEntry, t: T): string {
  const b = BUILT_IN.find((x) => x.name === entry.name);
  return b ? t(b.meaning) : (entry.meaning ?? "");
}

/** The labels a task shows as chips in the list and the pane: every label it carries, except
 * "AI decisions to review", which its own badge shows. */
export function chipLabels(task: Task): string[] {
  return taskLabels(task).filter((l) => l !== AI_DECISIONS);
}

/** Why it was finished without the human check (the note). */
export function humanCheckNote(task: Task): string {
  return task.without_human_check ?? "";
}

/** The opened taskbag with each task's AI decisions to review put on the task (Task-A03-19). */
export function withDecisions(opened: Opened): Opened {
  const box = opened.loaded.taskbag;
  const all = opened.ai_decisions ?? {};
  if (!box) return opened;
  const tasks = box.tasks.map((task) => (all[task.id] ? { ...task, ai_decisions: all[task.id] } : task));
  return { ...opened, loaded: { ...opened.loaded, taskbag: { ...box, tasks } } };
}

/**
 * The list's filter (Task-A01-lens781; decision 0020, Task-A01-petal376): ticks in two groups,
 * states and labels; a task is listed when its state is ticked and, for each label of the
 * project's label table, its side is ticked: Yes when the task has that label, No when it does
 * not. Every label is plain: no order, no "this one or higher". Everything ticked is no filter at
 * all. Kept as what is unticked, so a state or label new to the list starts ticked; `labels` are
 * the table's names, built-in first, filled in when the list draws. Labeled (Task-A01-teacup726)
 * comes first: its Yes keeps tasks that carry a label, its No those that carry none; a task with
 * no label is decided by Labeled's No alone, and each label's sides apply only to labelled tasks.
 */
export type StateFilter = { states: State[]; labeled: Side[]; yes: string[]; no: string[]; labels: string[] };

export const FILTER_STATES: State[] = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"];

/** Everything ticked: every task listed. */
export function allTicked(labels: readonly string[] = []): StateFilter {
  return { states: [], labeled: [], yes: [], no: [], labels: [...labels] };
}

/** One side of a label's tick. */
export type Side = "yes" | "no";

const SIDES: Side[] = ["yes", "no"];

/** The filter for this label table: unticked labels no longer in it are let go (so are an older
 * build's ladder steps, which `allomos/7` renamed). */
export function onTable(filter: StateFilter, labels: readonly string[]): StateFilter {
  const keep = (list: string[]) => list.filter((l) => labels.includes(l));
  return {
    states: filter.states.filter((s) => FILTER_STATES.includes(s)),
    labeled: SIDES.filter((s) => filter.labeled.includes(s)),
    yes: keep(filter.yes),
    no: keep(filter.no),
    labels: [...labels],
  };
}

/** Whether nothing is unticked. */
export function isAll(filter: StateFilter): boolean {
  return filter.states.length === 0 && filter.labeled.length === 0 && filter.yes.length === 0 && filter.no.length === 0;
}

/** The tasks the filter keeps: its state ticked; with no label, Labeled's No ticked; with labels,
 * Labeled's Yes ticked and, for every label in the table, Yes ticked if the task has it, No if not. */
export function keepState(filter: StateFilter, task: Task): boolean {
  if (filter.states.includes(task.state)) return false;
  const has = taskLabels(task);
  if (has.length === 0) return !filter.labeled.includes("no");
  if (filter.labeled.includes("yes")) return false;
  return filter.labels.every((label) => !(has.includes(label) ? filter.yes : filter.no).includes(label));
}

/** A state ticked or unticked. */
export function tickState(filter: StateFilter, state: State, on: boolean): StateFilter {
  const states = filter.states.filter((s) => s !== state);
  return { ...filter, states: on ? states : FILTER_STATES.filter((s) => s === state || states.includes(s)) };
}

/** One side of Labeled ticked or unticked; each label's ticks stay as they were. */
export function tickLabeled(filter: StateFilter, side: Side, on: boolean): StateFilter {
  const list = filter.labeled.filter((s) => s !== side);
  return { ...filter, labeled: on ? list : SIDES.filter((s) => s === side || list.includes(s)) };
}

/** Whether the labels' rows are greyed out: Labeled's Yes is unticked, so no labelled task is shown. */
export function labelsOff(filter: StateFilter): boolean {
  return filter.labeled.includes("yes");
}

/** One side of a label ticked or unticked. */
export function tickLabel(filter: StateFilter, label: string, side: Side, on: boolean): StateFilter {
  const list = filter[side].filter((l) => l !== label);
  return { ...filter, [side]: on ? list : filter.labels.filter((l) => l === label || list.includes(l)) };
}

/** A whole group ticked (Select all) or unticked (Clear all). */
export function tickGroup(filter: StateFilter, group: "states" | "labels", on: boolean): StateFilter {
  if (group === "states") return { ...filter, states: on ? [] : [...FILTER_STATES] };
  return { ...filter, labeled: on ? [] : [...SIDES], yes: on ? [] : [...filter.labels], no: on ? [] : [...filter.labels] };
}

/** Whether a group is all ticked, or all unticked: the grey Select all, the grey Clear all. */
export function groupTicks(filter: StateFilter, group: "states" | "labels"): { all: boolean; none: boolean } {
  if (group === "states") return { all: filter.states.length === 0, none: filter.states.length === FILTER_STATES.length };
  const off = filter.labeled.length + filter.yes.length + filter.no.length;
  return { all: off === 0, none: off === (filter.labels.length + 1) * 2 };
}

/** Why the filter lists no task at all, whatever the tasks: no state ticked, Labeled with neither
 * side ticked, or, with Labeled's No unticked, a label with neither side; `null` when it can list some. */
export function emptyBy(filter: StateFilter): { kind: "states" } | { kind: "labeled" } | { kind: "label"; label: string } | null {
  if (filter.states.length === FILTER_STATES.length) return { kind: "states" };
  if (filter.labeled.length === SIDES.length) return { kind: "labeled" };
  if (!filter.labeled.includes("no")) return null;
  const label = filter.labels.find((l) => filter.yes.includes(l) && filter.no.includes(l));
  return label === undefined ? null : { kind: "label", label };
}

/** The filter as stored: what is unticked. */
export function storeFilter(filter: StateFilter): string {
  return JSON.stringify({ states: filter.states, labeled: filter.labeled, yes: filter.yes, no: filter.no });
}

/**
 * A stored filter read back; anything else is everything ticked. Before Task-A01-lens781 the filter
 * was one choice (`old`): a state comes back as that state alone, the rest (all, open, human) as
 * everything ticked.
 */
export function readFilter(value: string | null, old: string | null = null): StateFilter {
  const words = (x: unknown) => (Array.isArray(x) ? x.filter((w): w is string => typeof w === "string") : []);
  if (value) {
    try {
      const v = JSON.parse(value) as Record<string, unknown>;
      const states = words(v.states).filter((s): s is State => (FILTER_STATES as string[]).includes(s));
      const labeled = SIDES.filter((s) => words(v.labeled).includes(s));
      return { states, labeled, yes: words(v.yes), no: words(v.no), labels: [] };
    } catch {
      return allTicked();
    }
  }
  if (old && (FILTER_STATES as string[]).includes(old)) return { ...allTicked(), states: FILTER_STATES.filter((s) => s !== old) };
  return allTicked();
}

/** What the filter leaves out of a list of tasks: how many, and how many in each state, in the
 * filter's order of states (Task-A01-42). Nothing is left out when everything is ticked. */
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

/** The unticked boxes by name, in the panel's order: states, Labeled's Yes and No, then each
 * label's Yes and No, unless the labels are greyed out (their ticks then hide nothing). */
export function untickedNames(filter: StateFilter, t: T): string[] {
  const side = (label: string, s: Side) => t(`filter.side_${s}`, { label: labelText(label, t) });
  return [
    ...FILTER_STATES.filter((s) => filter.states.includes(s)).map((s) => t(`state.${s}` as Key)),
    ...SIDES.filter((s) => filter.labeled.includes(s)).map((s) => t(`filter.side_${s}`, { label: t("filter.labeled") })),
    ...(labelsOff(filter) ? [] : filter.labels).flatMap((l) => [
      ...(filter.yes.includes(l) ? [side(l, "yes")] : []),
      ...(filter.no.includes(l) ? [side(l, "no")] : []),
    ]),
  ];
}

/** The filter's name as the Show button words it: "All tasks", "Hide Backlog, Dropped", "Filtered (3 hidden)". */
export function filterName(filter: StateFilter, t: T): string {
  const off = untickedNames(filter, t);
  if (off.length === 0) return t("filter.all_tasks");
  if (off.length <= 2) return t("filter.hide", { names: off.join(t("filter.list_sep")) });
  return t("filter.filtered", { count: off.length });
}

/** What a filtered list is grouped by: areas or iterations (Task-A01-44). */
export type GroupKind = "area" | "iteration";

/**
 * The groups as the filter shows them (Task-A01-44): each with only the tasks the filter keeps,
 * `counted` keeping its whole list for the count and progress, and groups with none left out. With
 * the filter off every group stays as it was.
 */
export function filterGroups<G extends { tasks: Task[]; counted?: Task[] }>(groups: G[], filter: StateFilter): G[] {
  if (isAll(filter)) return groups;
  return groups
    .map((g) => ({ ...g, counted: g.counted ?? g.tasks, tasks: g.tasks.filter((task) => keepState(filter, task)) }))
    .filter((g) => g.tasks.length > 0);
}

/** How many different tasks the groups show: a task in two groups counts once. */
export function taskCount(groups: readonly { tasks: readonly Task[] }[]): number {
  return new Set(groups.flatMap((g) => g.tasks.map((task) => task.id))).size;
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
 * The filter banner's words (Task-A01-44): "**3 tasks** match the filter, in 2 of 7 areas. The 5
 * areas with none are left out."; with nothing to show "No task matches the filter. All 12 iterations are
 * left out." Empty with everything ticked. While a search is on, the task list uses `bothBanner`.
 */
export function filterBanner(filter: StateFilter, c: FilterBannerCount, t: T): Words {
  if (isAll(filter)) return [];
  const verb = t(`filter.verb.match_${one(c.tasks === 0 ? 1 : c.tasks)}` as Key);
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
 * together. "Filter and search together: only tasks that match the filter and hold “alm” are listed, in
 * 1 area. 3 more matches are hidden by the filter."; with none left "None of the tasks found for
 * “alm” matches the filter. All 3 matches are hidden by the filter." Empty with everything ticked, or when
 * the search finds nothing at all ("Nothing found" says so).
 */
export function bothBanner(filter: StateFilter, c: BothCount, t: T): Words {
  if (isAll(filter) || c.matches === 0) return [];
  const hidden = c.matches - c.tasks;
  const hiddenWords: Words =
    hidden > 0 ? [{ text: t(`filter.matches_hidden.${c.tasks === 0 ? "all" : "more"}_${one(hidden)}` as Key, { count: hidden }) }] : [];
  if (c.tasks === 0) {
    const verb = t("filter.verb.match_one");
    return sentences([{ text: t("filter.banner.none_both", { query: c.query, verb }) }], hiddenWords);
  }
  const verb = t("filter.verb.match_other");
  const where = t(`filter.where.${c.kind}_${one(c.groups)}` as Key, { count: c.groups });
  return sentences([{ text: t("filter.banner.both", { query: c.query, verb, where }) }], hiddenWords);
}

/** The search banner's words (Task-A01-44): "**2 tasks** found for “blocked”."; under a Show filter
 * (`filter`, its name) "**2 tasks** found for “blocked” with the “Waiting” filter on." (Task-A01-48). */
export function searchBanner(kind: "task", count: number, query: string, t: T, filter?: string): Words {
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
  if (isAll(o.filter)) return { search: o.matches > 0 ? searchBanner("task", o.matches, query, t) : [], filter: [] };
  const shown = taskCount(o.found);
  return {
    search: shown > 0 ? searchBanner("task", shown, query, t, filterName(o.filter, t)) : [],
    filter: bothBanner(o.filter, { kind: o.kind, query, tasks: shown, groups: o.found.length, matches: o.matches }, t),
  };
}
