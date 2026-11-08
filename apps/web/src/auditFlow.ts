import type { WitnessWhen } from "./types";
// Project settings → Audit → Identity (Task-A01-frolic587, decision 0021 § 6): what the section
// decides without the window, so it is tested on its own.

import type { IdentityLevel } from "./types";

/** Every identity level, lowest first, as `alm audit-settings --identity` takes them. */
export const LEVELS: IdentityLevel[] = ["pseudonym", "account", "sso"];

/** Lowering the level, or turning signing in again off, needs a reason (decision 0021 § 6: an
 * auditor's first question is when a control was switched off, and why). */
export function needsReason(now: { level: IdentityLevel; reauthenticate: boolean }, level: IdentityLevel, reauthenticate: boolean): boolean {
  return LEVELS.indexOf(level) < LEVELS.indexOf(now.level) || (now.reauthenticate && !reauthenticate);
}

/** A shorter retention period, and turning the legal hold on or off, need a reason
 * (Task-A01-stem986, decision 0021 § 6 item 5). `null` years: forever. */
export function retentionNeedsReason(now: { years: number | null; hold: boolean }, years: number | null, hold: boolean): boolean {
  const shorter = years !== null && (now.years === null || years < now.years);
  return shorter || hold !== now.hold;
}

/** A retention period typed in the form: a whole number of years from 1 to 100, else null. */
export function yearsFrom(text: string): number | null {
  const n = Number(text.trim());
  return Number.isInteger(n) && n >= 1 && n <= 100 ? n : null;
}

/** Turning separation of duties off needs a reason (Task-A01-grotto036, decision 0021 § 6);
 * turning it on does not. */
export function separationNeedsReason(now: boolean, wanted: boolean): boolean {
  return now && !wanted;
}

/** The fields Remove personal details… offers as checkboxes (Task-A01-tail719); others are typed. */
export const REDACT_FIELDS = ["title", "description", "handoff_note"] as const;

/** The fields to remove: the ticked ones and those typed (separated by commas or spaces), once each. */
export function redactFields(ticked: readonly string[], typed: string): string[] {
  const more = typed.split(/[\s,，、]+/).map((f) => f.trim()).filter(Boolean);
  return [...new Set([...ticked, ...more])];
}

/** Whether Remove can be pressed: a task, a field and a reason, and no legal hold. */
export function canRedact(task: string, fields: readonly string[], reason: string, legalHold: boolean): boolean {
  return !legalHold && task.trim() !== "" && fields.length > 0 && reason.trim() !== "";
}

/** One Last seen history line's state (Task-A01-tail719): the checkpoint is still in that
 * taskbag, gone from it (the history was rewritten, or the checkpoint removed), or unknown
 * (no recent folder is that project). */
export function seenState(stillThere: boolean | null): "there" | "gone" | "unknown" {
  return stillThere === null ? "unknown" : stillThere ? "there" : "gone";
}

/** Settings → This computer → Replace key… (Task-A01-ring220): how many taskbags had the old key
 * retired, and the ones that could not be written, for the line shown after replacing. */
export function replacedSummary(r: { taskbags: { root: string; outcome: string }[] }): { replaced: number; failed: string[] } {
  return {
    replaced: r.taskbags.filter((t) => t.outcome === "replaced").length,
    failed: r.taskbags.filter((t) => t.outcome === "failed").map((t) => t.root),
  };
}

/** Outside witness (Task-A01-wood982): the address a choice in the service list stands for: a
 * preset's address, or the custom address typed ("" when none). */
export function witnessUrl(choice: string, custom: string, presets: readonly { id: string; url: string }[]): string {
  if (choice === "custom") return custom.trim();
  return presets.find((p) => p.id === choice)?.url ?? "";
}

/** The list choice for a service's address: its preset's ID, or "custom". */
export function witnessChoice(url: string | undefined, presets: readonly { id: string; url: string }[]): string {
  if (!url) return "";
  return presets.find((p) => p.url === url)?.id ?? "custom";
}

/** Whether an address typed for a custom service looks like one (`alm` checks it again). */
export function witnessUrlOk(url: string): boolean {
  return /^https?:\/\/\S+$/i.test(url.trim());
}

/** How often checkpoints are made by themselves (Task-A01-bake791), in the list's order; the first
 * time the witness is turned on, daily is chosen. */
export const WITNESS_WHENS: readonly WitnessWhen[] = ["daily", "approvals", "asked"];
export const FIRST_WITNESS_WHEN: WitnessWhen = "daily";
