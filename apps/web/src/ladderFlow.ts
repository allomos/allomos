// Editing the progress ladder in the taskbag's settings (Task-A05-yeti951), kept apart from React
// so Node can test it. Each row remembers the step's name before editing (`from`, none for a new
// step), so Save sends renames (which reach every task with that label) and the new order together.

export interface LadderStep {
  from: string | null;
  to: string;
}

export function startLadder(labels: string[]): LadderStep[] {
  return labels.map((l) => ({ from: l, to: l }));
}

export function renameStep(steps: LadderStep[], at: number, to: string): LadderStep[] {
  return steps.map((s, i) => (i === at ? { ...s, to } : s));
}

export function addStep(steps: LadderStep[]): LadderStep[] {
  return [...steps, { from: null, to: "" }];
}

export function removeStep(steps: LadderStep[], at: number): LadderStep[] {
  return steps.filter((_, i) => i !== at);
}

/** Move a step one place up (`-1`) or down (`1`); unchanged at either end. */
export function moveStep(steps: LadderStep[], at: number, by: -1 | 1): LadderStep[] {
  const to = at + by;
  if (to < 0 || to >= steps.length) return steps;
  const next = [...steps];
  [next[at], next[to]] = [next[to], next[at]];
  return next;
}

/** Why Save is off: an empty name, two steps with one name, or no step; null when it can be saved. */
export function ladderProblem(steps: LadderStep[]): "empty" | "duplicate" | "none" | null {
  if (steps.length === 0) return "none";
  const names = steps.map((s) => s.to.trim());
  if (names.some((n) => n === "")) return "empty";
  if (new Set(names).size !== names.length) return "duplicate";
  return null;
}

/** Whether anything differs from the ladder as it was. */
export function ladderChanged(labels: string[], steps: LadderStep[]): boolean {
  return steps.length !== labels.length || steps.some((s, i) => s.from !== labels[i] || s.to.trim() !== labels[i]);
}
