// Is the open project up to date with this Allomos (Task-A01-25)? Kept apart from React so it can
// be tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { VersionCheck } from "./types";

/** The instruction files whose Allomos section is older than this Allomos's, or has no version. */
export function outdatedFiles(check: VersionCheck | null): string[] {
  if (!check) return [];
  return check.sections.filter((s) => s.version === null || s.version < check.section_current).map((s) => s.file);
}

/** What the notice above the task list says, or null for no notice. The check is null before an
 * agent is connected, unless an older taskbag's tasks are to be moved in (Task-A01-45):
 * - `move_in`: the taskbag was written by an older Allomos; **Move the tasks in…** renames it
 *   aside, creates a new taskbag and opens the Connect step, where an agent is asked to move them;
 * - `moving_in`: that was done, and the new taskbag has no tasks yet: **Move the tasks in…** opens
 *   the Connect step again;
 * - `section`: Allomos's instructions are out of date.
 * A newer taskbag is not here: it cannot be read, and the problem list says "update Allomos now". */
export type UpdateNotice =
  | { kind: "move_in"; found: string }
  | { kind: "moving_in"; found: string; old: string }
  | { kind: "section"; files: string };

export function updateNotice(check: VersionCheck | null): UpdateNotice | null {
  if (!check) return null;
  const m = check.move_in;
  if (m) return m.prepared ? { kind: "moving_in", found: m.found, old: m.old } : { kind: "move_in", found: m.found };
  const files = outdatedFiles(check);
  return files.length > 0 ? { kind: "section", files: files.join(", ") } : null;
}

/** Whether a notice is about moving tasks in from an older taskbag, rather than the instructions. */
export function movingTasks(notice: UpdateNotice | null): boolean {
  return notice?.kind === "move_in" || notice?.kind === "moving_in";
}
