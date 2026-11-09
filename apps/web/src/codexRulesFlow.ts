// Monitor ready tasks… with Codex (Task-A03-pier032, approver 2026-11-09): the option that adds
// Allomos's rules for Codex (~/.codex/rules/allomos.rules). The dialog only reads the file's
// state; Start applies what the option says, through the app (`codex_rules_apply`).

/** One rule as a person reads it: `git pull`, or `git push --force | -f` with `forbidden`. */
export type CodexRule = { command: string; decision: string };

export type CodexRuleChanges = { added: CodexRule[]; removed: CodexRule[] };

/** `codex_rules_status`: where the file goes, the rules this Allomos ships, and the file's state. */
export type CodexRules = {
  path: string;
  shipped: number;
  rules: CodexRule[];
} & (
  | { state: "missing" }
  | { state: "current" }
  | { state: "older"; version: number; changes: CodexRuleChanges }
  | { state: "newer"; version: number }
  | { state: "changed"; changes: CodexRuleChanges }
);

/**
 * What the option shows: `add` (no file), `update` (an older version), `changed` (somebody
 * edited it: asked, not checked), `keep` (this version is there; unchecking removes it), or
 * `newer` (a newer Allomos's file, left alone, no checkbox).
 */
export type CodexOffer = "add" | "update" | "changed" | "keep" | "newer";

export function codexOffer(rules: CodexRules): CodexOffer {
  switch (rules.state) {
    case "missing":
      return "add";
    case "older":
      return "update";
    case "changed":
      return "changed";
    case "current":
      return "keep";
    case "newer":
      return "newer";
  }
}

/** Checked when the dialog opens: on, except for a file somebody changed (never replaced unasked). */
export function codexFirstChecked(offer: CodexOffer): boolean {
  return offer !== "changed" && offer !== "newer";
}

/**
 * What Start does to the file. An unchecked update keeps the old file this time; an unchecked
 * `keep` removes Allomos's file.
 */
export function codexActionAtStart(offer: CodexOffer, checked: boolean): "install" | "remove" | null {
  if (offer === "newer") return null;
  if (offer === "keep") return checked ? null : "remove";
  return checked ? "install" : null;
}

/** A rule as shown in the command list. */
export function ruleText(rule: CodexRule, never: string): string {
  return rule.decision === "forbidden" ? `${never} ${rule.command}` : rule.command;
}

/** The rules of a change, joined for a sentence, or `none` when there are none. */
export function rulesList(rules: CodexRule[], never: string, none: string): string {
  return rules.length ? rules.map((r) => ruleText(r, never)).join(", ") : none;
}
