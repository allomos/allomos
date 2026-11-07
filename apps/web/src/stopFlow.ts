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

/** The opened taskbag with each task's AI decisions to review (Task-A03-19) and its approvals
 * (Task-A01-coach782) put on the task. */
export function withDecisions(opened: Opened): Opened {
  const box = opened.loaded.taskbag;
  const all = opened.ai_decisions ?? {};
  const approvals = opened.approvals ?? {};
  if (!box) return opened;
  const tasks = box.tasks.map((task) => {
    let out = all[task.id] ? { ...task, ai_decisions: all[task.id] } : task;
    if (approvals[task.id]) out = { ...out, approvals: approvals[task.id] };
    return out;
  });
  return { ...opened, loaded: { ...opened.loaded, taskbag: { ...box, tasks } } };
}

/**
 * The list's filter (Task-A01-lens781; decision 0020; Task-A01-rock075): a grid with one row per
 * state and, across each row, a Yes and a No for each column. The columns are "Checked by a human"
 * (the two "by human" labels together: Yes when the task carries either, No when it carries
 * neither), then every other label of the project's label table, built-in first. A task is listed
 * when it matches any ticked cell of its own state's row: Yes when it has that column's label, No
 * when it does not. So everything ticked lists every task, and a row with nothing ticked lists
 * none of that state's tasks. The tick before a state only sums up its row. Kept as the unticked
 * cells, so a label new to the table starts ticked in every row; `labels` are the table's names,
 * built-in first, filled in when the list draws.
 */
export type StateFilter = { off: Cell[]; labels: string[] };

/** One tick of the grid: a state's row, a column, its Yes or No. */
export type Cell = { state: State; column: string; side: Side };

/** One side of a column's tick. */
export type Side = "yes" | "no";

export const SIDES: Side[] = ["yes", "no"];

export const FILTER_STATES: State[] = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"];

/** The "Checked by a human" column's name in the grid; no label is named with the empty word. */
export const HUMAN_COLUMN = "";

/** Everything ticked: every task listed. */
export function allTicked(labels: readonly string[] = []): StateFilter {
  return { off: [], labels: [...labels] };
}

/** The grid's columns: "Checked by a human" first, then every other label of the table. */
export function filterColumns(filter: Pick<StateFilter, "labels">): string[] {
  return [HUMAN_COLUMN, ...filter.labels.filter((l) => !BY_HUMAN.includes(l))];
}

/** A column's heading in the interface language. */
export function columnText(column: string, t: T): string {
  return column === HUMAN_COLUMN ? t("filter.human") : labelText(column, t);
}

/** Which side of a column a task is on: Yes when it has that column's label ("by human": either one). */
export function sideOf(task: Task, column: string): Side {
  const has = column === HUMAN_COLUMN ? humanChecked(task) : hasLabel(task, column);
  return has ? "yes" : "no";
}

const cellKey = (state: State, column: string, side: Side) => `${state}\u0000${column}\u0000${side}`;

/** The filter's unticked cells as keys and its columns, worked out once per filter (the list asks
 * for every task, so a big taskbag stays quick). */
const looked = new WeakMap<StateFilter, { off: Set<string>; columns: string[] }>();
function lookup(filter: StateFilter): { off: Set<string>; columns: string[] } {
  let found = looked.get(filter);
  if (!found) {
    found = { off: new Set(filter.off.map((c) => cellKey(c.state, c.column, c.side))), columns: filterColumns(filter) };
    looked.set(filter, found);
  }
  return found;
}

/** Whether a cell is ticked. */
export function isTicked(filter: StateFilter, state: State, column: string, side: Side): boolean {
  return !lookup(filter).off.has(cellKey(state, column, side));
}

/** The filter with these cells unticked, in the grid's order (rows, then columns, then Yes, No). */
function withOff(filter: StateFilter, off: Set<string>): StateFilter {
  const columns = filterColumns(filter);
  const cells: Cell[] = [];
  for (const state of FILTER_STATES)
    for (const column of columns) for (const side of SIDES) if (off.has(cellKey(state, column, side))) cells.push({ state, column, side });
  return { off: cells, labels: filter.labels };
}

/** The filter for this label table: unticked cells of columns no longer in it are let go. */
export function onTable(filter: StateFilter, labels: readonly string[]): StateFilter {
  const table = { off: [], labels: [...labels] };
  const columns = filterColumns(table);
  const keep = filter.off.filter((c) => FILTER_STATES.includes(c.state) && columns.includes(c.column) && SIDES.includes(c.side));
  return withOff(table, new Set(keep.map((c) => cellKey(c.state, c.column, c.side))));
}

/** The list's filter for an opened project: the stored ticks on the project's label table, by
 * name (Task-A01-key755). The desktop app sends the table as entries (`allomos/7`), never names:
 * 0.1.41 put those entries in the filter as they came, and opening the Show filter then drew an
 * entry as text, which React cannot do, so the window went blank and stopped answering. */
export function tableFilter(stored: StateFilter, project: Pick<Project, "labels"> | undefined, tasks: readonly Task[] = []): StateFilter {
  return onTable(stored, labelNames(project, tasks));
}

/** Whether nothing is unticked. */
export function isAll(filter: StateFilter): boolean {
  return filter.off.length === 0;
}

/** The tasks the filter keeps: those matching any ticked cell of their own state's row. */
export function keepState(filter: StateFilter, task: Task): boolean {
  if (filter.off.length === 0) return true;
  const { off, columns } = lookup(filter);
  return columns.some((column) => !off.has(cellKey(task.state, column, sideOf(task, column))));
}

/** One cell ticked or unticked. */
export function tickCell(filter: StateFilter, state: State, column: string, side: Side, on: boolean): StateFilter {
  const off = new Set(lookup(filter).off);
  if (on) off.delete(cellKey(state, column, side));
  else off.add(cellKey(state, column, side));
  return withOff(filter, off);
}

/** How a state's row is ticked, which the tick before the state shows: all, some or none. */
export type RowTicks = "all" | "some" | "none";

export function rowTicks(filter: StateFilter, state: State): RowTicks {
  const { off, columns } = lookup(filter);
  const unticked = columns.reduce((n, column) => n + SIDES.filter((side) => off.has(cellKey(state, column, side))).length, 0);
  return unticked === 0 ? "all" : unticked === columns.length * SIDES.length ? "none" : "some";
}

/** A whole row ticked or unticked (the tick before the state). */
export function tickRow(filter: StateFilter, state: State, on: boolean): StateFilter {
  const off = new Set(lookup(filter).off);
  for (const column of filterColumns(filter))
    for (const side of SIDES) {
      if (on) off.delete(cellKey(state, column, side));
      else off.add(cellKey(state, column, side));
    }
  return withOff(filter, off);
}

/** The whole grid ticked (Select all) or unticked (Clear all). */
export function tickAll(filter: StateFilter, on: boolean): StateFilter {
  if (on) return { off: [], labels: filter.labels };
  return withOff(filter, new Set(FILTER_STATES.flatMap((s) => filterColumns(filter).flatMap((c) => SIDES.map((side) => cellKey(s, c, side))))));
}

/** Whether the grid is all ticked, or all unticked: the grey Select all, the grey Clear all. */
export function gridTicks(filter: StateFilter): { all: boolean; none: boolean } {
  return { all: filter.off.length === 0, none: FILTER_STATES.every((s) => rowTicks(filter, s) === "none") };
}

/** Whether the filter lists no task at all, whatever the tasks: nothing ticked in any row. */
export function emptyBy(filter: StateFilter): boolean {
  return gridTicks(filter).none;
}

/** The filter as stored: the unticked cells, each as [state, column, side]. */
export function storeFilter(filter: StateFilter): string {
  return JSON.stringify({ grid: filter.off.map((c) => [c.state, c.column, c.side]) });
}

/**
 * A stored filter read back; anything else is everything ticked. A filter stored before the grid
 * (Task-A01-rock075: states and labels joined by "and"; before Task-A01-lens781 a single choice)
 * comes back as everything ticked too.
 */
export function readFilter(value: string | null): StateFilter {
  if (!value) return allTicked();
  try {
    const v = JSON.parse(value) as Record<string, unknown>;
    if (!Array.isArray(v.grid)) return allTicked();
    const off: Cell[] = [];
    for (const c of v.grid) {
      if (!Array.isArray(c) || c.length !== 3) continue;
      const [state, column, side] = c as unknown[];
      if (!(FILTER_STATES as unknown[]).includes(state) || typeof column !== "string" || !(SIDES as unknown[]).includes(side)) continue;
      off.push({ state: state as State, column, side: side as Side });
    }
    return { off, labels: [] };
  } catch {
    return allTicked();
  }
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

/** The unticked boxes by name, in the grid's order: a row with nothing ticked by its state's name,
 * otherwise each unticked cell ("Done · Checked by a human: no"). */
export function untickedNames(filter: StateFilter, t: T): string[] {
  return FILTER_STATES.flatMap((state) => {
    const ticks = rowTicks(filter, state);
    const name = t(`state.${state}` as Key);
    if (ticks === "all") return [];
    if (ticks === "none") return [name];
    return filter.off
      .filter((c) => c.state === state)
      .map((c) => t("filter.cell", { state: name, cell: t(`filter.side_${c.side}`, { label: columnText(c.column, t) }) }));
  });
}

/**
 * The Show filter's presets (Task-A01-wigwam308): each only sets the grid's ticks, adding no rule
 * of its own. Five tick whole rows; "Needs you" ticks what `alm human` lists: To check (Done,
 * "Checked by a human: No"), To answer (the whole Waiting row) and To review ("AI decisions to
 * review: Yes" in every row, a checked Done task among them).
 */
export type Preset = "all" | "not_finished" | "ready" | "in_hand" | "needs_you" | "finished";

export const PRESETS: Preset[] = ["all", "not_finished", "ready", "in_hand", "needs_you", "finished"];

/** The rows a row preset ticks whole. */
const PRESET_ROWS: Record<Exclude<Preset, "needs_you">, State[]> = {
  all: FILTER_STATES,
  not_finished: ["backlog", "ready", "in_progress", "waiting", "blocked"],
  ready: ["ready"],
  in_hand: ["in_progress", "waiting", "blocked"],
  finished: ["done", "dropped"],
};

/** The grid's ticks a preset sets, on this label table. */
export function presetFilter(preset: Preset, labels: readonly string[]): StateFilter {
  const none = tickAll(allTicked(labels), false);
  if (preset !== "needs_you") return PRESET_ROWS[preset].reduce((f, s) => tickRow(f, s, true), none);
  let f = tickRow(none, "waiting", true);
  for (const s of FILTER_STATES) f = tickCell(f, s, AI_DECISIONS, "yes", true);
  return tickCell(f, "done", HUMAN_COLUMN, "no", true);
}

/** The preset whose ticks the filter has exactly, worked out once per filter; `null`: none. */
const matched = new WeakMap<StateFilter, Preset | null>();
export function matchPreset(filter: StateFilter): Preset | null {
  if (matched.has(filter)) return matched.get(filter) ?? null;
  const off = lookup(filter).off;
  const found =
    PRESETS.find((p) => {
      const cells = presetFilter(p, filter.labels).off;
      return cells.length === off.size && cells.every((c) => off.has(cellKey(c.state, c.column, c.side)));
    }) ?? null;
  matched.set(filter, found);
  return found;
}

/** A preset's name: "All tasks", "Needs you". */
export function presetName(preset: Preset, t: T): string {
  return preset === "all" ? t("filter.all_tasks") : t(`filter.preset.${preset}` as Key);
}

/** What a preset ticks, in words: its rows by name, or a sentence for "All tasks" and "Needs you". */
export function presetTicks(preset: Preset, t: T): string {
  if (preset === "all" || preset === "needs_you") return t(`filter.preset_ticks.${preset}` as Key);
  return PRESET_ROWS[preset].map((s) => t(`state.${s}` as Key)).join(t("filter.list_sep"));
}

/** How many of these tasks a preset lists now. */
export function presetCount(preset: Preset, labels: readonly string[], tasks: readonly Task[]): number {
  const f = presetFilter(preset, labels);
  return tasks.filter((task) => keepState(f, task)).length;
}

/** The filter's name as the Show button words it: a preset's name when the ticks are exactly a
 * preset's ("All tasks", "Needs you"), else "Hide Backlog, Dropped", "Filtered (3 hidden)". */
export function filterName(filter: StateFilter, t: T): string {
  const preset = matchPreset(filter);
  if (preset) return presetName(preset, t);
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
