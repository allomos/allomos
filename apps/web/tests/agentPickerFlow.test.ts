// Tests for the shared agent picker (Task-A05-07). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { MANUAL, chooseGroup, firstChoice, foundGroups, groupOf } from "../src/agentPickerFlow.ts";
import type { Agent } from "../src/types.ts";

const agent = (id: Agent["id"]): Agent => ({ id, found: id, how: id.endsWith("app") ? "link" : "paste" });

test("found agents are grouped, the app before the terminal, Claude before Codex", () => {
  const found = [agent("codex_cli"), agent("claude_cli"), agent("claude_app")];
  assert.deepEqual(
    foundGroups(found).map((g) => [g.name, g.tools.map((a) => a.id)]),
    [
      ["Claude", ["claude_app", "claude_cli"]],
      ["Codex", ["codex_cli"]],
    ],
  );
  // A group with no tool found is not offered.
  assert.deepEqual(foundGroups([agent("codex_app")]).map((g) => g.name), ["Codex"]);
  assert.deepEqual(foundGroups([]), []);
});

test("the first found tool is chosen first; with none found, Manual", () => {
  assert.equal(firstChoice([agent("codex_cli"), agent("claude_cli"), agent("claude_app")]), "claude_app");
  assert.equal(firstChoice([agent("codex_app")]), "codex_app");
  assert.equal(firstChoice([]), MANUAL);
});

test("choosing a group keeps the tool last chosen in it", () => {
  const found = [agent("claude_app"), agent("claude_cli"), agent("codex_cli")];
  assert.equal(chooseGroup(found, "Claude", {}), "claude_app");
  assert.equal(chooseGroup(found, "Claude", { Claude: "claude_cli" }), "claude_cli");
  // A remembered tool that is gone falls back to the first found.
  assert.equal(chooseGroup([agent("claude_app")], "Claude", { Claude: "claude_cli" }), "claude_app");
  assert.equal(chooseGroup(found, "Codex", {}), "codex_cli");
  assert.equal(chooseGroup(found, MANUAL, {}), MANUAL);
  assert.equal(groupOf("codex_app"), "Codex");
  assert.equal(groupOf(MANUAL), MANUAL);
  assert.equal(groupOf(null), null);
});
