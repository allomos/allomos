// Editing the project's own labels in the taskbag's settings (Task-A05-yeti951; decision 0020,
// Task-A01-petal376: the label table, whose five built-in labels cannot be changed), kept apart
// from React so Node can test it. Each row remembers the label's name before editing (`from`, none
// for a new one), so Save sends renames (which reach every task with that label) and the list
// together. (The names still say "ladder" and "step", from before the label table.)

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

/** Why Save is off: an empty name, or two labels with one name (a name in `taken`, the built-in
 * labels, counts as used); null when it can be saved. No label of its own at all is fine. */
export function ladderProblem(steps: LadderStep[], taken: readonly string[] = []): "empty" | "duplicate" | null {
  const names = steps.map((s) => s.to.trim());
  if (names.some((n) => n === "")) return "empty";
  if (new Set(names).size !== names.length || names.some((n) => taken.includes(n))) return "duplicate";
  return null;
}

/** Whether anything differs from the labels as they were. */
export function ladderChanged(labels: string[], steps: LadderStep[]): boolean {
  return steps.length !== labels.length || steps.some((s, i) => s.from !== labels[i] || s.to.trim() !== labels[i]);
}
