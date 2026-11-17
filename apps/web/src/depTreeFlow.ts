// The dependency tree (Task-A01-alpine458): the tasks of one group drawn as trees, a task under
// the task that blocks it (`alm block <id> --by <task-id>`, the task's `blocked_by`). It is a way
// of showing the list, not a sort: the roots, and the tasks under one parent, keep the order the
// sort gave them (sortFlow.ts). Kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). Only type imports here: Node strips them.
import type { Task } from "./types";

/** One line of the tree, in the order the lines are drawn. */
export interface TreeRow {
  task: Task;
  /** 0 for a root, 1 for a task under it, and so on. */
  depth: number;
  /** For each level between the root and this task's parent, outermost first (`depth - 1` of
   * them): whether a guide line runs through this line, because that ancestor has a later sibling. */
  lines: boolean[];
  /** The last task under its parent: its guide line ends here. */
  last: boolean;
  /** How many tasks hang directly under it. */
  kids: number;
  /** It has tasks under it and they are folded away. */
  folded: boolean;
  /** How many tasks its fold hides (all levels); 0 when it is open. */
  hidden: number;
  /** Its other blockers shown in this group: it is drawn under one parent only. */
  also: string[];
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** The tree's lines for one group.
 *
 * `sorted` holds the group's tasks as shown, already in the sort's order. A task with several
 * blockers in the group hangs under the one that comes first in that order; the others are its
 * `also`. A task with no blocker in the group is a root, and so is a task nothing depends on: a
 * tree of one, placed among the other roots by the sort. Blockers going round in a circle are cut
 * by ID: the task with the lowest ID in the circle keeps no parent from it (`idRank`, the
 * taskbag's ID order; without it, IDs compared as text with numbers counted as numbers).
 * `folded` holds the IDs of the tasks whose children are folded away. */
export function depTreeRows(sorted: Task[], folded: string[] = [], idRank?: Map<string, number>): TreeRow[] {
  const rank = new Map(sorted.map((t, i) => [t.id, i]));
  const byId = (a: Task, b: Task) => {
    const [x, y] = [idRank?.get(a.id), idRank?.get(b.id)];
    return x !== undefined && y !== undefined ? x - y : collator.compare(a.id, b.id);
  };
  // Each task's blockers in the group, first in the sort's order first.
  const inGroup = new Map<string, string[]>();
  for (const task of sorted) {
    const ids = [...new Set(task.blocked_by ?? [])].filter((id) => id !== task.id && rank.has(id));
    inGroup.set(task.id, ids.sort((a, b) => rank.get(a)! - rank.get(b)!));
  }
  // Parents are chosen from the highest ID down, each taking its first blocker that does not
  // already hang under it; so in a circle the lowest ID is the one left without a parent.
  const parent = new Map<string, string>();
  const under = (id: string, ancestor: string) => {
    for (let at: string | undefined = id; at !== undefined; at = parent.get(at)) if (at === ancestor) return true;
    return false;
  };
  for (const task of [...sorted].sort(byId).reverse()) {
    const p = inGroup.get(task.id)!.find((id) => !under(id, task.id));
    if (p !== undefined) parent.set(task.id, p);
  }
  const kids = new Map<string, Task[]>();
  const roots: Task[] = [];
  for (const task of sorted) {
    const p = parent.get(task.id);
    if (p === undefined) roots.push(task);
    else kids.set(p, [...(kids.get(p) ?? []), task]);
  }
  const size = (id: string): number => (kids.get(id) ?? []).reduce((n, k) => n + 1 + size(k.id), 0);
  const rows: TreeRow[] = [];
  const walk = (task: Task, depth: number, lines: boolean[], last: boolean) => {
    const below = kids.get(task.id) ?? [];
    const isFolded = below.length > 0 && folded.includes(task.id);
    const p = parent.get(task.id);
    rows.push({
      task,
      depth,
      lines,
      last,
      kids: below.length,
      folded: isFolded,
      hidden: isFolded ? size(task.id) : 0,
      also: inGroup.get(task.id)!.filter((id) => id !== p),
    });
    if (isFolded) return;
    const next = depth === 0 ? [] : [...lines, !last];
    below.forEach((k, i) => walk(k, depth + 1, next, i === below.length - 1));
  };
  roots.forEach((r) => walk(r, 0, [], true));
  return rows;
}

/** Whether the dependency tree is on, read back from storage; off unless it says on. */
export function parseDepTree(text: string | null): boolean {
  return text === "on";
}
