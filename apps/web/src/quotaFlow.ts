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

/**
 * The agent's id from what a session reported, as alm folds it since Task-A03-39 (reports saved
 * before that are folded here): the part before any `/`, and a word starting with `claude` is
 * `claude-code`, one starting with `codex` is `codex`. A session that reported under its session
 * name (`claude/a09-28-kanto-picker`) no longer gets a chip of its own.
 */
export function agentId(reported: string): string {
  const word = (reported.trim().split("/")[0] ?? "").trim();
  const lower = word.toLowerCase();
  return lower.startsWith("claude") ? "claude-code" : lower.startsWith("codex") ? "codex" : word;
}

/** Every report under its folded agent id, newest first. */
function folded(reports: QuotaReport[]): QuotaReport[] {
  return reports.map((r) => ({ ...r, agent: agentId(r.agent) })).sort((a, b) => b.at.localeCompare(a.at));
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

/**
 * How old a report reads on the chip (Task-A03-flour054): minutes up to an hour, then whole hours
 * up to a day, then whole days, so a report from yesterday no longer reads "630 min ago".
 */
export function ageText(minutes: number): { key: "agentline.age" | "agentline.age_hours" | "agentline.age_days"; n: number } {
  if (minutes < 60) return { key: "agentline.age", n: minutes };
  if (minutes < 24 * 60) return { key: "agentline.age_hours", n: Math.floor(minutes / 60) };
  return { key: "agentline.age_days", n: Math.floor(minutes / (24 * 60)) };
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
  const newest = folded(reports);
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
  const newest = folded(reports);
  const current = newest[0]?.agent;
  return [...newest.filter((r) => r.agent === current), ...newest.filter((r) => r.agent !== current)];
}
