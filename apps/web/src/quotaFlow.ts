// The header's agent chips (Task-A03-02): one chip per agent. The agent whose quota was reported
// last (`alm quota`) is the current agent: its chip comes first and shows the 5-hour quota of the
// account that reported last, coloured by the share left. The other chips are dimmed and say
// whether the agent is told about the project. Hovering a chip lists every reported account.

import type { QuotaReport } from "./types";

/** The agents Allomos knows by name; others are shown by the id they reported. */
export const AGENT_NAMES: Record<string, string> = { "claude-code": "Claude Code", codex: "Codex" };

export function agentName(id: string): string {
  return AGENT_NAMES[id] ?? id;
}

/** Green at 20% or more, amber from 5% to under 20%, red under 5%. */
export function tone(left: number): "green" | "amber" | "red" {
  return left >= 20 ? "green" : left >= 5 ? "amber" : "red";
}

const MINUTE = 60_000;

/**
 * Up to 10 minutes after the report: fresh (only the quota). After that, aged (the age is shown).
 * After 30 minutes, or once the reset time has passed: stale (grey, dashed, "when reported").
 */
export function freshness(report: QuotaReport, now: Date): { state: "fresh" | "aged" | "stale"; minutes: number } {
  const minutes = Math.max(0, Math.floor((now.getTime() - Date.parse(report.at)) / MINUTE));
  const resets = report.resets ? new Date(report.resets).getTime() : NaN;
  if (minutes >= 30 || (!Number.isNaN(resets) && resets <= now.getTime())) return { state: "stale", minutes };
  return { state: minutes >= 10 ? "aged" : "fresh", minutes };
}

export interface Chip {
  agent: string;
  name: string;
  /** The agent that reported last. */
  current: boolean;
  /** For the current agent: the 5-hour report of the account that reported last, if any. */
  quota: QuotaReport | null;
  /** Whether the agent is told about the project; null when Allomos does not check that agent. */
  told: boolean | null;
}

/** The chips, current first, then Claude Code and Codex, then other agents that reported. */
export function chips(reports: QuotaReport[], setup: { codex: boolean; claude: boolean }): Chip[] {
  const newest = [...reports].sort((a, b) => b.at.localeCompare(a.at));
  const current = newest[0]?.agent ?? null;
  const told = (agent: string) => (agent === "claude-code" ? setup.claude : agent === "codex" ? setup.codex : null);
  const order = [current, "claude-code", "codex", ...newest.map((r) => r.agent)].filter(
    (a, i, all): a is string => a !== null && all.indexOf(a) === i,
  );
  return order.map((agent) => {
    const isCurrent = agent === current;
    const latest = newest.find((r) => r.agent === agent);
    const quota = isCurrent
      ? newest.find((r) => r.agent === agent && r.window === "5h" && (r.account ?? null) === (latest?.account ?? null)) ??
        newest.find((r) => r.agent === agent && r.window === "5h") ??
        null
      : null;
    return { agent, name: agentName(agent), current: isCurrent, quota, told: told(agent) };
  });
}

/** The popup's rows: every report, the current agent's first, then the newest first. */
export function popupRows(reports: QuotaReport[]): QuotaReport[] {
  const newest = [...reports].sort((a, b) => b.at.localeCompare(a.at));
  const current = newest[0]?.agent;
  return [...newest.filter((r) => r.agent === current), ...newest.filter((r) => r.agent !== current)];
}
