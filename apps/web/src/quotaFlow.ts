// Agent names and ids, as `alm quota` and the session records report them. The header's agent
// chips that showed the quota (Task-A03-02) are gone (Task-A03-root435); the task pane still
// names a session's agent with these.

/** The agents Allomos knows by name; others are shown by the id they reported. */
export const AGENT_NAMES: Record<string, string> = { "claude-code": "Claude Code", codex: "Codex", antigravity: "Antigravity" };

export function agentName(id: string): string {
  return AGENT_NAMES[id] ?? id;
}

/**
 * The agent's id from what a session reported, as alm folds it since Task-A03-39 (reports saved
 * before that are folded here): the part before any `/`, and a word starting with `claude` is
 * `claude-code`, one starting with `codex` is `codex`, one starting with `antigravity` or `agy`
 * itself is `antigravity` (Task-A03-queen969). A session that reported under its session
 * name (`claude/a09-28-kanto-picker`) no longer gets a chip of its own.
 */
export function agentId(reported: string): string {
  const word = (reported.trim().split("/")[0] ?? "").trim();
  const lower = word.toLowerCase();
  if (lower.startsWith("claude")) return "claude-code";
  if (lower.startsWith("codex")) return "codex";
  if (lower.startsWith("antigravity") || lower === "agy") return "antigravity";
  return word;
}
