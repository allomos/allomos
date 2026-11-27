// The model and reasoning effort each agent's monitored sessions start with (A03-alto709). The
// choices come from the online model list (apps/desktop/src-tauri/src/agent_models.rs), read when
// the Monitor ready tasks dialog opens and at its Refresh; the first choice of each is "Default",
// chosen until the person picks another, and then the agent's command is as it was before. Only
// type imports here: Node strips them (`npm --prefix apps/web test`).

import type { TerminalAgent } from "./monitorFlow";

/** The agent of a monitor agent key (`claude_cli:<account>` is Claude Code's). */
const baseAgent = (key: string): string => key.split(":")[0];

export interface ListedModel {
  value: string;
  label: string;
  /** Format 2 (A03-pony586): the reasoning efforts this model offers. Absent: its agent's.
   * Empty: the model takes no reasoning effort. */
  efforts?: string[];
}

export interface ListedAgent {
  id: string;
  name: string;
  models: ListedModel[];
  efforts: string[];
}

/** What the app read: the list (null when neither the web nor this computer's copy had one),
 * where it came from, and why the web could not be read. */
export interface ModelsRead {
  agents: ListedAgent[] | null;
  from: "online" | "cache" | "none";
  /** `fetch`: no answer (offline, or the address is wrong); `format`: not a list this app reads. */
  failed: "fetch" | "format" | null;
  url: string;
}

/** One agent's choice: a value missing is Default (the agent's own setting). */
export interface Picks {
  model?: string;
  effort?: string;
}

/** The models and efforts the list offers an agent (an extra account's are its agent's). With
 * `model`, the efforts that model offers: its own when the list gives it one, else the agent's;
 * an empty list means the model takes no reasoning effort. */
export function optionsFor(
  read: ModelsRead | null,
  agent: TerminalAgent,
  model?: string,
): { models: ListedModel[]; efforts: string[] } {
  const listed = read?.agents?.find((a) => a.id === baseAgent(agent));
  const models = listed?.models ?? [];
  if (model === undefined) return { models, efforts: listed?.efforts ?? [] };
  const chosen = models.find((m) => m.value === model);
  return { models, efforts: chosen?.efforts ?? listed?.efforts ?? [] };
}

/** What Start sends for the chosen agents: only values chosen and still offered by the list, so a
 * value gone after a Refresh falls back to Default. */
export function chosenArgs(
  picked: TerminalAgent[],
  picks: Partial<Record<TerminalAgent, Picks>>,
  read: ModelsRead | null,
): { models: Record<string, string>; efforts: Record<string, string> } {
  const models: Record<string, string> = {};
  const efforts: Record<string, string> = {};
  for (const a of picked) {
    const p = picks[a];
    if (!p) continue;
    const o = optionsFor(read, a, p.model);
    if (p.model && o.models.some((m) => m.value === p.model)) models[a] = p.model;
    if (p.effort && o.efforts.includes(p.effort)) efforts[a] = p.effort;
  }
  return { models, efforts };
}

/** The line shown under the list's Refresh when the web could not be read: the saved copy is in
 * use, or there is no list and only Default can be chosen. Null: read from the web. */
export function listNotice(read: ModelsRead | null): "monitor.models_cached" | "monitor.models_none" | "monitor.models_bad_format" | null {
  if (!read || read.failed === null) return null;
  if (read.from === "cache") return "monitor.models_cached";
  return read.failed === "format" ? "monitor.models_bad_format" : "monitor.models_none";
}

/** A model list address the app takes (as the app checks it): `https://` or `http://`, a host,
 * and no spaces. An empty one is Allomos's own. */
export function plainUrl(text: string): boolean {
  const url = text.trim();
  const rest = url.startsWith("https://") ? url.slice(8) : url.startsWith("http://") ? url.slice(7) : null;
  return rest !== null && rest.length > 0 && !rest.startsWith("/") && url.length <= 2048 && !/\s/.test(url);
}
