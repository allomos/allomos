// Tests for the agent ids and names (quotaFlow.ts). The header's quota chips are gone
// (Task-A03-root435). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { agentId, agentName } from "../src/quotaFlow.ts";

test("a report under a session name or short form counts for its agent, not as another agent", () => {
  assert.deepEqual(
    ["claude-code", "claude/a09-28-kanto-picker", " Claude ", "codex-cli/loop-2", "gemini/run-3"].map(agentId),
    ["claude-code", "claude-code", "claude-code", "codex", "gemini"],
  );
  // Task-A03-queen969: Antigravity, by its id, its name or its command.
  assert.deepEqual(["antigravity", "Antigravity CLI", "agy/loop-1", "agyx"].map(agentId), ["antigravity", "antigravity", "antigravity", "agyx"]);
});

test("known agents by product name, others by the id they reported", () => {
  assert.deepEqual(["claude-code", "codex", "antigravity", "gemini"].map(agentName), ["Claude Code", "Codex", "Antigravity", "gemini"]);
});
