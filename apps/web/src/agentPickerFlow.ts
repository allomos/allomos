// The shared agent picker (Task-A05-07), kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// Every feature that opens an AI agent (Connect with the agents, Update the taskbag…, Build an
// App) lets the person choose it the same way: agents grouped (Claude, Codex), each with its app
// and terminal as sub-choices, and Manual last (approver, 2026-10-17).

import type { Agent } from "./types";

/** The Manual choice: no agent is opened; the person copies the request. */
export const MANUAL = "manual";

/** A choice: one agent tool, or Manual. */
export type Choice = Agent["id"] | typeof MANUAL;

/** The tools of each agent, in the order the first is chosen: the app before the terminal
 * (owner, 2026-10-10), Claude before Codex. */
export const GROUPS: { name: string; tools: Agent["id"][] }[] = [
  { name: "Claude", tools: ["claude_app", "claude_cli"] },
  { name: "Codex", tools: ["codex_app", "codex_cli"] },
];

/** The groups with at least one tool found, each with its found tools in order. */
export function foundGroups(found: Agent[]): { name: string; tools: Agent[] }[] {
  return GROUPS.map((g) => ({
    name: g.name,
    tools: g.tools.map((id) => found.find((a) => a.id === id)).filter((a): a is Agent => a !== undefined),
  })).filter((g) => g.tools.length > 0);
}

/** The choice made first: the first found tool, else Manual (offered even when none is found). */
export function firstChoice(found: Agent[]): Choice {
  return foundGroups(found)[0]?.tools[0]?.id ?? MANUAL;
}

/** The group a choice belongs to (`MANUAL` for Manual). */
export function groupOf(choice: Choice | null): string | null {
  if (choice === null || choice === MANUAL) return choice;
  return GROUPS.find((g) => g.tools.includes(choice))?.name ?? null;
}

/** Choosing a group (its radio button): the tool last chosen in it, else its first found one. */
export function chooseGroup(found: Agent[], group: string, last: Partial<Record<string, Agent["id"]>>): Choice {
  if (group === MANUAL) return MANUAL;
  const tools = foundGroups(found).find((g) => g.name === group)?.tools ?? [];
  const kept = tools.find((a) => a.id === last[group]);
  return (kept ?? tools[0])?.id ?? MANUAL;
}
