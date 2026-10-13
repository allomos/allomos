// Number labels for pointing at elements (Task-A03-12): Ctrl+Shift+S shows a short label such as
// "C3" on every element in the window, so the owner can tell an AI agent which one they mean.
// Decisions only; the drawing lives in ElementLabels.tsx.

export type Box = { top: number; left: number; width: number; height: number };

type Keys = { key: string; code?: string; ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean };

/** Ctrl+Shift+S (either case), on every platform; Cmd and Alt must not be held. */
export function isLabelShortcut(e: Keys): boolean {
  if (!e.ctrlKey || !e.shiftKey || e.altKey || e.metaKey) return false;
  return e.code === "KeyS" || e.key.toLowerCase() === "s";
}

/** A letter then a digit: A0 … A9, B0 … Z9; after 260 the letter repeats (AA0, BB0 …). */
export function labelName(index: number): string {
  const letter = String.fromCharCode(65 + (Math.floor(index / 10) % 26));
  return letter.repeat(Math.floor(index / 260) + 1) + (index % 10);
}

/** Rows this close together (in pixels) read as one line, left to right. */
const ROW = 8;

/**
 * Gives each box a label in reading order: top to bottom, then left to right. A box that covers
 * exactly the same place as one already labelled (a button and the text inside it) gets none;
 * the first in the list (the outer element, in page order) keeps the label.
 * Returns, for each input box, its label or null.
 */
export function assignLabels(boxes: Box[]): (string | null)[] {
  const order = boxes
    .map((box, at) => ({ box, at }))
    .sort((a, b) => Math.floor(a.box.top / ROW) - Math.floor(b.box.top / ROW) || Math.round(a.box.left) - Math.round(b.box.left) || a.at - b.at);
  const seen = new Set<string>();
  const labels: (string | null)[] = boxes.map(() => null);
  let next = 0;
  for (const { box, at } of order) {
    const place = [box.top, box.left, box.width, box.height].map(Math.round).join(",");
    if (seen.has(place)) continue;
    seen.add(place);
    labels[at] = labelName(next++);
  }
  return labels;
}
