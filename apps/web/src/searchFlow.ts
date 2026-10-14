// Searching the task list (Task-A01-36): which tasks and handoffs hold the words typed in the search
// box, and where in their text, so the window can show only those and mark the words. Kept apart
// from React so it can be tested with Node alone (`npm --prefix apps/web test`). Only type imports
// here: Node strips them.

import type { Handoff, Task } from "./types";

/** One stretch of shown text: `hit` when it matches a search word. */
export interface Piece {
  text: string;
  hit: boolean;
}

/**
 * One character as written (a letter and any marks that follow it, such as a Japanese voicing
 * mark, also the half-width ﾞ and ﾟ), so `ｶﾞ` and `ガ`, or `e` + accent and `é`, compare alike.
 */
const CHARACTER = /[\s\S](?:[\p{M}ﾞﾟ])*/gu;

/**
 * `text` made comparable: full-width and half-width forms joined (NFKC), then lower case. `from`
 * and `to` give, for each unit of the result, where its character starts and ends in `text`, so a
 * match found in the result can be marked in the text as written.
 */
export function fold(text: string): { folded: string; from: number[]; to: number[] } {
  let folded = "";
  const from: number[] = [];
  const to: number[] = [];
  for (const m of text.matchAll(CHARACTER)) {
    const start = m.index ?? 0;
    const f = m[0].normalize("NFKC").toLowerCase();
    folded += f;
    for (let i = 0; i < f.length; i++) {
      from.push(start);
      to.push(start + m[0].length);
    }
  }
  return { folded, from, to };
}

/** The words of a search, made comparable: split at spaces (also the full-width space), each once. */
export function searchTerms(query: string): string[] {
  return [...new Set(fold(query).folded.split(/\s+/).filter((w) => w.length > 0))];
}

/** Where the words occur in `text`, as [start, end) in the text as written, in order, joined where they touch. */
export function hitRanges(text: string, terms: string[]): [number, number][] {
  if (terms.length === 0 || !text) return [];
  const { folded, from, to } = fold(text);
  const found: [number, number][] = [];
  for (const term of terms) {
    for (let at = folded.indexOf(term); at >= 0; at = folded.indexOf(term, at + term.length)) {
      found.push([from[at], to[at + term.length - 1]]);
    }
  }
  found.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const joined: [number, number][] = [];
  for (const r of found) {
    const last = joined[joined.length - 1];
    if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
    else joined.push([r[0], r[1]]);
  }
  return joined;
}

/** Does `text` hold `term` (an already comparable word)? */
function holds(text: string, term: string): boolean {
  return fold(text).folded.includes(term);
}

/** `text` cut into stretches that match and stretches that do not, for marking the words. */
export function pieces(text: string, terms: string[]): Piece[] {
  const out: Piece[] = [];
  let at = 0;
  for (const [s, e] of hitRanges(text, terms)) {
    if (s > at) out.push({ text: text.slice(at, s), hit: false });
    out.push({ text: text.slice(s, e), hit: true });
    at = e;
  }
  if (at < text.length || out.length === 0) out.push({ text: text.slice(at), hit: false });
  return out;
}

/** Moves `i` off the second half of a character written with two units (such as an emoji). */
function onCharacter(text: string, i: number): number {
  const c = text.charCodeAt(i);
  return c >= 0xdc00 && c <= 0xdfff ? i - 1 : i;
}

/**
 * A short piece of a long text around its first match, on one line, with "…" where it was cut,
 * so a task found by its description shows why; null when the text does not match.
 */
export function snippet(text: string, terms: string[], around = 40): Piece[] | null {
  const ranges = hitRanges(text, terms);
  if (ranges.length === 0) return null;
  // Line breaks become spaces, one for one, so the found places stay where they were.
  const line = text.replace(/\s/g, " ");
  const [s, e] = ranges[0];
  const start = s - around > 0 ? onCharacter(line, s - around) : 0;
  const end = e + around < line.length ? onCharacter(line, e + around) : line.length;
  const cut = (start > 0 ? "…" : "") + line.slice(start, end).trim() + (end < line.length ? "…" : "");
  return pieces(cut, terms);
}

/** Does every word occur in at least one of the texts? */
function allIn(texts: string[], terms: string[]): boolean {
  return terms.every((term) => texts.some((text) => holds(text, term)));
}

/** A task matches when each word is in its ID, title or description (Task-A01-36). */
export function taskMatches(task: Pick<Task, "id" | "title" | "description">, terms: string[]): boolean {
  return terms.length === 0 || allIn([task.id, task.title, task.description ?? ""], terms);
}

/** A handoff matches when each word is in its summary, text, writer or the task IDs it names. */
export function handoffMatches(h: Pick<Handoff, "summary" | "body" | "writer" | "tasks">, terms: string[]): boolean {
  return terms.length === 0 || allIn([h.summary, h.body, h.writer, ...h.tasks], terms);
}

/**
 * The groups as a search shows them: each with only its matching tasks, and groups with none left
 * out. `counted` keeps the group's whole list, so its count and progress do not change.
 */
export function searchGroups<G extends { tasks: Task[]; counted?: Task[] }>(groups: G[], terms: string[]): G[] {
  if (terms.length === 0) return groups;
  return groups
    .map((g) => ({ ...g, counted: g.counted ?? g.tasks, tasks: g.tasks.filter((task) => taskMatches(task, terms)) }))
    .filter((g) => g.tasks.length > 0);
}

/**
 * The collapsed groups while searching: none of the groups shown, since each holds a match and
 * opens by itself (owner, 2026-10-14). What the person collapsed is kept, and comes back when the
 * search is cleared.
 */
export function collapsedWhileSearching(collapsed: string[], shown: { key: string }[], terms: string[]): string[] {
  if (terms.length === 0) return collapsed;
  return collapsed.filter((k) => !shown.some((g) => g.key === k));
}
