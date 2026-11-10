// The names of the extra accounts of the terminal agents (Task-A03-oven548), for the labels of
// the monitor's rows, list and status bar. A run remembers only an account's ID (`claude_cli:<id>`);
// the name is looked up here, from the list the app last read. Kept apart from React and from
// the messenger, so it can be tested with Node alone.

import type { AgentAccount } from "./monitorFlow";

let known: AgentAccount[] = [];

/** Keeps the list the app last read: the names the labels use. */
export function rememberAccounts(list: AgentAccount[]): void {
  known = list;
}

/** The name of a monitor agent's account, or null for an agent alone (or an account no longer
 * listed: its label then names only the agent). */
export function accountNameOf(key: string): string | null {
  const at = key.indexOf(":");
  if (at < 0) return null;
  const id = key.slice(at + 1);
  const agent = key.slice(0, at);
  return known.find((a) => a.id === id && a.agent === agent)?.name ?? null;
}

/** The label of a monitor agent: the agent's name (given by the caller, translated) with the
 * account's name after a dot when it has one. */
export function labelOf(key: string, agentName: string): string {
  const name = accountNameOf(key);
  return name ? `${agentName} · ${name}` : agentName;
}
