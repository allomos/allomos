// The start page's Build a new app… (Task-A03-25-02), kept apart from React so it can be tested
// with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// The person picks an empty folder, describes the app, chooses an agent (or Manual) and quiet or
// normal mode. Start creates the taskbag there as the setup wizard does and types one request
// into the agent (connect, then the goal prompt); the person presses Enter. Manual shows the same
// request with Copy.

import type { Agent, Inspection } from "./types";

/** The Manual choice: no agent is opened; the person copies the request. */
export const MANUAL = "manual";

/** The agent tools in the order the first found one is chosen: as in Connect with the agents
 * (the app before the terminal; Claude before Codex). */
export const TOOL_ORDER: Agent["id"][] = ["claude_app", "claude_cli", "codex_app", "codex_cli"];

export type RunMode = "quiet" | "normal";

/** Quiet is ticked first (A03-25, approver 2026-10-16). */
export const FIRST_MODE: RunMode = "quiet";

/** The found tools, in [`TOOL_ORDER`], followed by Manual. */
export function agentChoices(found: Agent[]): string[] {
  const ids = TOOL_ORDER.filter((id) => found.some((a) => a.id === id));
  return [...ids, MANUAL];
}

/** The choice ticked first: the first found tool, else Manual. */
export function firstAgent(found: Agent[]): string {
  return agentChoices(found)[0];
}

/** What the chosen folder allows: only an empty one (nothing but hidden entries) is used, so the
 * agent builds the app in a folder of its own. */
export type FolderCheck =
  | { kind: "ok"; folder: string; root: string }
  | { kind: "not_empty"; folder: string }
  | { kind: "not_a_folder"; path: string };

export function checkFolder(i: Inspection): FolderCheck {
  switch (i.kind) {
    case "empty":
      return { kind: "ok", folder: i.folder, root: i.suggested_root };
    case "no_taskbag":
      return { kind: "not_empty", folder: i.folder };
    case "found":
      return { kind: "not_empty", folder: i.root };
    case "not_a_folder":
      return { kind: "not_a_folder", path: i.path };
  }
}

/** Start is offered once there is an empty folder, a description and an agent choice. */
export function canStart(folder: FolderCheck | null, description: string, agent: string | null): boolean {
  return folder?.kind === "ok" && description.trim() !== "" && agent !== null;
}
