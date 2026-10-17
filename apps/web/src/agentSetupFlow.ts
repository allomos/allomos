// An AI agent not yet set up for the project is one more thing Update the taskbag… does
// (Task-A03-24, approver 2026-10-17), kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). The window reads the entries under `## Agents set up` in the
// taskbag's project notes and the agents found on this computer; it checks nothing else: the
// agent does the setup (`alm guide agent-files`) and adds each agent's line.

/** The name an entry gives its agent: the words before its first colon, without Markdown marks
 * (`- **Claude Code**: CLAUDE.md …` → `claude code`). */
function entryName(entry: string): string {
  return entry.split(":")[0].replace(/[*_`]/g, "").trim().toLowerCase();
}

/** The agents (Claude, Codex: the found groups) with no entry naming them. An entry names an agent
 * when the agent's name is a word of the entry's name, any case ("Claude Code" names Claude). */
export function notSetUp(groups: string[], entries: string[]): string[] {
  const names = entries.map(entryName);
  return groups.filter((g) => {
    const word = new RegExp(`\\b${g.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "i");
    return !names.some((n) => word.test(n));
  });
}

/** Whether the window shows "Allomos needs to register more information in the taskbag": some
 * found agent is not set up, the project is connected (an agent was told about Allomos; before
 * that, Connect with the agents does the setup), and no other Update the taskbag… notice is
 * already shown (card files outside the taskbag, an older taskbag): that one's request covers the
 * agents too. */
export function setupNotice(missing: string[], connected: boolean, otherNotice: boolean): boolean {
  return missing.length > 0 && connected && !otherNotice;
}
