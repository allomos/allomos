// A newly installed agent sets itself up once (Task-A03-24, approver 2026-10-16), kept apart from
// React so it can be tested with Node alone (`npm --prefix apps/web test`). The agents panel shows
// "Not set up for this project yet." with Copy prompt on every found agent that has no entry under
// `## Agents set up` in the taskbag's project notes. The app reads only that section and checks
// nothing else: the agent does the setup and adds its own line.

/** The name an entry gives its agent: the words before its first colon, without Markdown marks
 * (`- **Claude Code**: CLAUDE.md …` → `claude code`). */
function entryName(entry: string): string {
  return entry.split(":")[0].replace(/[*_`]/g, "").trim().toLowerCase();
}

/** The agent groups (Claude, Codex) with no entry naming them. An entry names a group when the
 * group's name is a word of the entry's name, any case ("Claude Code" names Claude). */
export function notSetUp(groups: string[], entries: string[]): string[] {
  const names = entries.map(entryName);
  return groups.filter((g) => {
    const word = new RegExp(`\\b${g.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "i");
    return !names.some((n) => word.test(n));
  });
}

/** Whether `taskbag` lies inside `project` (either separator; a trailing one ignored). */
export function taskbagInside(project: string, taskbag: string): boolean {
  const norm = (p: string) => p.replace(/\\/g, "/").replace(/\/+$/, "");
  return norm(taskbag).startsWith(norm(project) + "/");
}

/** The command the prompt names: `alm guide agent-files`, with `--taskbag "<path>"` when the
 * taskbag is outside the project folder (approver, 2026-10-16), so the agent finds it. */
export function agentFilesCommand(project: string, taskbag: string): string {
  return taskbagInside(project, taskbag) ? "alm guide agent-files" : `alm --taskbag "${taskbag}" guide agent-files`;
}
