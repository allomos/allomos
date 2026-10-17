// The start page's Build a new app… (Task-A03-25-02), kept apart from React so it can be tested
// with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// The person picks an empty folder, describes the app, chooses an agent (or Manual) and quiet or
// normal mode. Start creates the taskbag there as the setup wizard does and types one request
// into the agent (connect, then the goal prompt); the person presses Enter. Manual shows the same
// request with Copy.

import type { Inspection } from "./types";

/** The Manual choice: no agent is opened; the person copies the request. */
export const MANUAL = "manual";

export type RunMode = "quiet" | "normal";

/** Quiet is ticked first (A03-25, approver 2026-10-16). */
export const FIRST_MODE: RunMode = "quiet";

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
