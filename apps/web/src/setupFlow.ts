// The setup wizard's choices (Task-A05-03), kept apart from React so they can be tested with
// Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { InstructionFiles, Inspection, SetupState } from "./types";

/** Step 2 of the wizard: what the person can do with the folder chosen in step 1. */
export type TaskbagStep =
  /** The folder has `.allomos` (or is a taskbag): go on with Next (Previous to choose another). */
  | { kind: "found"; root: string; buttons: ["next"] }
  /** No `.allomos` yet: Create makes one there. */
  | { kind: "missing"; folder: string; suggestedRoot: string; buttons: ["create"] }
  | { kind: "not_a_folder"; path: string };

export function taskbagStep(i: Inspection): TaskbagStep {
  switch (i.kind) {
    case "found":
      return { kind: "found", root: i.root, buttons: ["next"] };
    case "empty":
    case "no_taskbag":
      return { kind: "missing", folder: i.folder, suggestedRoot: i.suggested_root, buttons: ["create"] };
    case "not_a_folder":
      return { kind: "not_a_folder", path: i.path };
  }
}

/** Step 3: the project source folder offered first — the one already recorded, else the folder
 * chosen in step 1 (the folder holding `.allomos/`). */
export function defaultSource(setup: SetupState): string {
  return setup.project_folder ?? setup.allomos_folder;
}

/** A folder path compared as the same folder: without a trailing `/` or `\`. */
function samePath(a: string, b: string): boolean {
  const trim = (p: string) => p.replace(/[\\/]+$/, "");
  return trim(a) === trim(b);
}

/** Step 3's Add Another Folder… (Task-A05-king717): `path` joins the other folders unless it is
 * the main one or listed already. */
export function addFolder(main: string, more: readonly string[], path: string): string[] {
  if (samePath(path, main) || more.some((m) => samePath(m, path))) return [...more];
  return [...more, path];
}

/** Changing the main folder to `path` takes it off the other folders, if it was there. */
export function withMain(path: string, more: readonly string[]): string[] {
  return more.filter((m) => !samePath(m, path));
}

/** Step 3 marks the main folder "holds the taskbag" only while it is that folder. */
export function holdsTaskbag(folder: string, allomosFolder: string): boolean {
  return samePath(folder, allomosFolder);
}

/** Whether opening a taskbag goes on to the wizard's step 4 by itself: a project folder is known,
 * and either an agent is not told about Allomos (Connect with the agents) or the taskbag has no
 * tasks yet (Update the taskbag: the project may keep tasks elsewhere, approver 2026-10-19,
 * Task-A03-27). "Not now" holds until the folder is opened again. */
export function offerConnect(setup: SetupState, skipped: boolean): boolean {
  if (skipped || !setup.project_folder) return false;
  return !(setup.agents.codex && setup.agents.claude) || !setup.has_tasks;
}

/** The instruction files that were missing before agents were connected and exist now. */
export function newFiles(before: InstructionFiles, now: InstructionFiles): string[] {
  const names: string[] = [];
  if (!before.agents_md && now.agents_md) names.push("AGENTS.md");
  if (!before.claude_md && now.claude_md) names.push("CLAUDE.md");
  return names;
}

/** Whether there is still a file to watch for: one that was missing before and is not there yet. */
export function stillMissing(before: InstructionFiles, now: InstructionFiles): boolean {
  return (!before.agents_md && !now.agents_md) || (!before.claude_md && !now.claude_md);
}

/** Opening a taskbag directly (not through the wizard): the wizard step to open by itself, or
 * none. No project folder known: step 3. Known, but an agent is not told or there are no tasks:
 * step 4 (owner, 2026-10-12: the wizard replaces the page's own setup cards). */
export function directStep(setup: SetupState, skipped: boolean): 3 | 4 | null {
  if (skipped) return null;
  if (!setup.project_folder) return 3;
  return offerConnect(setup, false) ? 4 : null;
}

/** Where a wizard asked to open at `wanted` really opens: step 3 while no project folder is known. */
export function resumeAt(setup: SetupState, wanted: 3 | 4): 3 | 4 {
  return setup.project_folder ? wanted : 3;
}

/** Not now on step 4 goes back to the start page when no agent has ever been told about Allomos
 * (neither AGENTS.md nor CLAUDE.md mentions it); otherwise the taskbag stays open (owner, 2026-10-12). */
export function neverConnected(setup: SetupState): boolean {
  return !setup.agents.codex && !setup.agents.claude;
}

/** What the wizard's step 4 shows (Task-A05-09, approver 2026-10-17). */
export type StepFourPage = "agents" | "cards" | "done";

/** Step 4's page: Update the taskbag while an older taskbag's tasks wait to be moved in
 * (`movingIn`, Task-A01-45: the same panel as for card files); else Connect with the agents when
 * agents are not told or the person chose Connect an agent anyway; else Update the taskbag when it
 * needs updating (approver 2026-10-18, Task-A03-27): card files outside it, or Allomos's
 * instructions out of date (`needsUpdate`), or when it has no tasks yet (approver 2026-10-19: the
 * agent looks for tasks the project keeps elsewhere; the window shows no notice for this); else
 * Setup done. An agent not registered under `## Agents set up` is no reason: each agent registers
 * itself when it starts work. */
export function stepFourPage(
  setup: SetupState,
  needsUpdate: boolean,
  cardsOutside: number,
  anyway: boolean,
  movingIn = false,
): StepFourPage {
  if (anyway) return "agents";
  if (movingIn && setup.project_folder) return "cards";
  if (!setup.project_folder || !(setup.agents.codex && setup.agents.claude)) return "agents";
  return needsUpdate || cardsOutside > 0 || !setup.has_tasks ? "cards" : "done";
}
