// When Update the taskbag… shows, and which agents it offers (Task-A03-27, approver 2026-10-18),
// kept apart from React so it can be tested with Node alone (`npm --prefix apps/web test`). Only
// type imports here: Node strips them.
//
// The taskbag needs updating only when card files are kept outside it, an older taskbag's tasks
// are to be moved in, or Allomos's section in the project's instruction files is out of date; a
// taskbag with no tasks yet does not count. Then:
// - the agent used last is found and not registered under `## Agents set up`: only that agent
//   (it registers itself and updates the taskbag in one go);
// - it is registered, not found, or no longer on this computer: every agent.
// The app cannot tell which agent the person uses next, so it says nothing about agents that are
// not registered: each registers itself when it starts work (AGENTS.md, `alm guide agent-files`).

import type { Agent, VersionCheck } from "./types";

/** The agents as the picker groups them, and the name `alm` records in events for each. */
const AGENTS: { group: string; recorded: string; tools: Agent["id"][] }[] = [
  { group: "Claude", recorded: "claude", tools: ["claude_app", "claude_cli"] },
  { group: "Codex", recorded: "codex", tools: ["codex_app", "codex_cli"] },
];

/** Whether the taskbag needs updating: card files outside it (`cardsOutside`), an older taskbag's
 * tasks to move in, or an out-of-date section (`check`). */
export function needsUpdate(cardsOutside: number, check: VersionCheck | null): boolean {
  // As versionFlow's `outdatedFiles` (a section older than this Allomos's, or with no version).
  const outdated = check?.sections.some((s) => s.version === null || s.version < check.section_current) ?? false;
  return cardsOutside > 0 || !!check?.move_in || outdated;
}

/** The name an entry gives its agent: the words before its first colon, without Markdown marks
 * (`- **Claude Code**: CLAUDE.md …` → `claude code`). */
function entryName(entry: string): string {
  return entry.split(":")[0].replace(/[*_`]/g, "").trim().toLowerCase();
}

/** Whether an entry under `## Agents set up` names the agent `group` (Claude, Codex): its name is a
 * word of the entry's name, any case ("Claude Code" names Claude). */
export function registered(group: string, entries: string[]): boolean {
  const word = new RegExp(`\\b${group.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "i");
  return entries.map(entryName).some((n) => word.test(n));
}

/** The agent used last for this project: its group, and its tool when known. */
export interface LastUsed {
  group: string;
  tool: Agent["id"] | null;
}

/** The agent used last: the one the newest task change through `alm` records (`fromEvents`),
 * else the tool the app last opened for this project (`opened`: Build a new app…, Connect with
 * the agents, Update the taskbag…), else none. */
export function lastUsed(
  fromEvents: { agent: string; tool: string | null } | null,
  opened: Agent["id"] | null,
): LastUsed | null {
  if (fromEvents) {
    const a = AGENTS.find((x) => x.recorded === fromEvents.agent);
    if (a) {
      const tool = a.tools.find((t) => t === fromEvents.tool) ?? null;
      return { group: a.group, tool };
    }
  }
  if (opened) {
    const a = AGENTS.find((x) => x.tools.includes(opened));
    if (a) return { group: a.group, tool: opened };
  }
  return null;
}

/** Which agents Update the taskbag… offers:
 * - `all`: every agent found, and Manual;
 * - `only`: just `tools` of the agent `group` (the tool used last, else its other tools that open
 *   with the request typed in), and Manual to paste into that agent;
 * - `manual`: none of that agent's tools opens with the request typed in: Manual alone, to paste
 *   into `group`. */
export type Offer =
  | { kind: "all" }
  | { kind: "only"; group: string; tools: Agent["id"][] }
  | { kind: "manual"; group: string };

/** The rule (approver, 2026-10-18). `entries`: the lines under `## Agents set up`; null when the
 * notes cannot be read (unsure: every agent). */
export function offer(found: Agent[], last: LastUsed | null, entries: string[] | null): Offer {
  if (!last || entries === null || registered(last.group, entries)) return { kind: "all" };
  const tools = AGENTS.find((a) => a.group === last.group)?.tools ?? [];
  const here = found.filter((a) => tools.includes(a.id));
  // No longer on this computer: as if not found.
  if (here.length === 0) return { kind: "all" };
  const typedIn = here.filter((a) => a.how === "link");
  const same = typedIn.find((a) => a.id === last.tool);
  if (same) return { kind: "only", group: last.group, tools: [same.id] };
  if (typedIn.length > 0) return { kind: "only", group: last.group, tools: typedIn.map((a) => a.id) };
  return { kind: "manual", group: last.group };
}

/** The agents the picker shows for an offer. */
export function offered(found: Agent[], o: Offer): Agent[] {
  if (o.kind === "all") return found;
  if (o.kind === "manual") return [];
  return found.filter((a) => o.tools.includes(a.id));
}

/** Where the window remembers the tool it last opened for each project (taskbag root → tool). */
const OPENED_KEY = "allomos.lastOpenedAgent";

/** The tool the app last opened for the taskbag at `root`; none when unknown or unreadable. */
export function readOpened(root: string): Agent["id"] | null {
  try {
    const all = JSON.parse(localStorage.getItem(OPENED_KEY) ?? "{}") as Record<string, Agent["id"]>;
    return typeof all[root] === "string" ? all[root] : null;
  } catch {
    return null;
  }
}

/** Remember that the app opened `tool` for the taskbag at `root`. */
export function saveOpened(root: string, tool: Agent["id"]) {
  try {
    const all = JSON.parse(localStorage.getItem(OPENED_KEY) ?? "{}") as Record<string, Agent["id"]>;
    all[root] = tool;
    localStorage.setItem(OPENED_KEY, JSON.stringify(all));
  } catch {
    /* storage unavailable: not remembered */
  }
}
