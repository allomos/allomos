// The Risk management section of the taskbag's settings (decision 0027 § 6, Task-A01-beetle749):
// what an edit of the project's own rules sends. A rule is one sentence on one line, written for
// the agent that classifies a task; an empty one and one the project already has are not sent
// (crates/core/src/risk.rs leaves them out too). Kept apart from React so it can be tested with
// Node alone.

/** The settings as the desktop app sends them (apps/desktop/src-tauri/src/risk_settings.rs). */
export interface RiskSettings {
  /** The fixed rules for high risk, by short name (H1…H7); the app words them. */
  fixed: string[];
  /** The project's own rules, one sentence each. */
  rules: string[];
  /** Limit 1: a high-risk task cannot be finished without the human check. */
  high_needs_human_check: boolean;
  /** Limit 2: in a quiet run, an agent does not merge a high-risk task itself. */
  high_needs_approver_merge: boolean;
}

/** A rule as it is kept: on one line, without spaces around it. */
export function cleanRule(text: string): string {
  return text.split(/\s+/).filter(Boolean).join(" ");
}

/** The rules with one added at the end, or null when it is empty or already there. */
export function addRule(rules: string[], text: string): string[] | null {
  const rule = cleanRule(text);
  if (!rule || rules.includes(rule)) return null;
  return [...rules, rule];
}

/** The rules with the one at `index` reworded, or null when the new wording is empty or another
 * rule already says it. The same wording as before is a change of nothing: the rules as they are. */
export function changeRule(rules: string[], index: number, text: string): string[] | null {
  const rule = cleanRule(text);
  if (!rule || index < 0 || index >= rules.length) return null;
  if (rules.some((r, i) => i !== index && r === rule)) return null;
  return rules.map((r, i) => (i === index ? rule : r));
}

/** The rules without the one at `index`. */
export function removeRule(rules: string[], index: number): string[] {
  return rules.filter((_, i) => i !== index);
}
