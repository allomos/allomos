// Tests for the shared agent picker (Task-A05-07). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { MANUAL, chooseGroup, firstChoice, foundGroups, groupOf, requestWay } from "../src/agentPickerFlow.ts";
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
  // Task-A03-queen969: Antigravity's terminal agent is a group of its own, after Codex.
  assert.deepEqual(
    foundGroups([agent("antigravity_cli"), agent("codex_cli")]).map((g) => [g.name, g.tools.map((a) => a.id)]),
    [
      ["Codex", ["codex_cli"]],
      ["Antigravity", ["antigravity_cli"]],
    ],
  );
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

test("the request is sent at once for Antigravity, pasted for other terminals, typed in for apps", () => {
  assert.equal(requestWay({ id: "antigravity_cli", how: "paste" }), "sent");
  assert.equal(requestWay(agent("codex_cli")), "paste");
  assert.equal(requestWay(agent("claude_cli")), "paste");
  assert.equal(requestWay(agent("claude_app")), "typed");
  assert.equal(requestWay(undefined), "typed");
});
