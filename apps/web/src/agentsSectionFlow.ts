// The Agents section of the taskbag's settings (Task-A05-bag963), kept apart from React so Node
// can test it: for each AI agent, in which of the taskbag's project folders its instruction files
// tell it about Allomos. Claude Code and Codex are the agents with instruction files of their own
// (CLAUDE.md, AGENTS.md); the picker's app and command-line versions read the same files.

/** The agents the section has a row for, in the picker's order. */
export const SECTION_AGENTS = ["claude", "codex"] as const;
export type SectionAgent = (typeof SECTION_AGENTS)[number];

/** A folder the section counts: a bound folder, or the taskbag's one project folder when none is bound. */
export interface SectionFolder {
  name: string;
  full: string;
  exists: boolean;
  /** Which agents its instruction files tell about Allomos; null while not read (or not found). */
  setup: Record<SectionAgent, boolean> | null;
}

export interface AgentRow {
  agent: SectionAgent;
  name: string;
  /** Folders where this agent is set up, and how many folders there are. */
  done: number;
  total: number;
  /** Folders where it is not, in the bound order; `exists: false` ones cannot be set up from here. */
  missing: { name: string; full: string; exists: boolean }[];
}

/** Product names, as the agent line shows them (`quotaFlow.ts`'s AGENT_NAMES); not translated. */
const NAMES: Record<SectionAgent, string> = { claude: "Claude Code", codex: "Codex" };

export function agentName(agent: SectionAgent): string {
  return NAMES[agent];
}

export function agentRows(folders: readonly SectionFolder[]): AgentRow[] {
  return SECTION_AGENTS.map((agent) => {
    const missing = folders.filter((f) => !f.setup?.[agent]).map(({ name, full, exists }) => ({ name, full, exists }));
    return { agent, name: agentName(agent), done: folders.length - missing.length, total: folders.length, missing };
  });
}

/** With one folder the row says "set up" or "not set up", as the agent line does; with more, "N of M". */
export function rowText(row: AgentRow): { key: "agents_section.set_up" | "agents_section.not_set_up" | "agents_section.count"; done: number; total: number } {
  if (row.total <= 1) return { key: row.done === row.total && row.total === 1 ? "agents_section.set_up" : "agents_section.not_set_up", done: row.done, total: row.total };
  return { key: "agents_section.count", done: row.done, total: row.total };
}
