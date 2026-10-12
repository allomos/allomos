// Is the open project up to date with this Allomos (Task-A01-25)? Kept apart from React so it can
// be tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { VersionCheck } from "./types";

/** The instruction files whose Allomos section is older than this Allomos's, or has no version. */
export function outdatedFiles(check: VersionCheck | null): string[] {
  if (!check) return [];
  return check.sections.filter((s) => s.version === null || s.version < check.section_current).map((s) => s.file);
}

/** What the notice above the task list says, or null for no notice. Only once an agent is
 * connected (the check is null before). A newer taskbag is not here: it cannot be read, and the
 * problem list says "update Allomos now". */
export type UpdateNotice = { kind: "taskbag" } | { kind: "section"; files: string };

export function updateNotice(check: VersionCheck | null): UpdateNotice | null {
  if (!check) return null;
  if (check.format.state === "older") return { kind: "taskbag" };
  const files = outdatedFiles(check);
  return files.length > 0 ? { kind: "section", files: files.join(", ") } : null;
}
