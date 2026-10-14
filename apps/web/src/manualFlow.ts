// The Manual choice of "Connect with the agents" (Task-A05-06), kept apart from React so it can be
// tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// Manual is for an AI that Allomos does not know: the panel shows the same request automatic
// sending would type in, with a Copy button; the person pastes it into their AI and sends it.
// How the wizard knows the AI finished (owner rule A00-29, worker's choice): it watches for what
// the automatic path already watches for (an instruction file appearing, or AGENTS.md / CLAUDE.md
// starting to mention Allomos) and says so; Done closes the step either way.

import type { InstructionFiles } from "./types";

/** The Manual choice's name among the agent groups (Claude, Codex). */
export const MANUAL = "manual";

/** How long "Copied" shows after Copy. */
export const COPIED_MS = 2000;

/** How often the Manual choice looks whether the AI has finished. */
export const MANUAL_LOOK_MS = 2000;

/** The choice ticked first: the first agent group with a found tool, else Manual (which is
 * offered even when no known agent is found). */
export function firstChoice(foundGroups: string[]): string {
  return foundGroups[0] ?? MANUAL;
}

/** The step's buttons besides Not now. `primary`: Connect opens the chosen agent; Done closes
 * the Manual choice. `write`: Allomos may add the instructions itself (no agent found, and
 * nothing is out of date: an update is the agent's work). */
export function connectActions(
  choice: string | null,
  agentsFound: number,
  updating: boolean,
): { primary: "connect" | "done" | null; write: boolean } {
  const primary = choice === MANUAL ? "done" : choice !== null && agentsFound > 0 ? "connect" : null;
  return { primary, write: agentsFound === 0 && !updating };
}

/** What the project looked like: its instruction files, and which of them mention Allomos. */
export interface Seen {
  files: InstructionFiles;
  agents: { codex: boolean; claude: boolean };
}

/** The instruction files the AI has written since `before`: newly made, or now mentioning
 * Allomos (AGENTS.md is Codex's, CLAUDE.md is Claude's). Empty: nothing seen yet. */
export function manualWritten(before: Seen, now: Seen): string[] {
  const names: string[] = [];
  if ((!before.files.agents_md && now.files.agents_md) || (!before.agents.codex && now.agents.codex)) names.push("AGENTS.md");
  if ((!before.files.claude_md && now.files.claude_md) || (!before.agents.claude && now.agents.claude)) names.push("CLAUDE.md");
  return names;
}
