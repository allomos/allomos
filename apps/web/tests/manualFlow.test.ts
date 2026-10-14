// Tests for the Manual choice of Connect with the agents (Task-A05-06). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { MANUAL, connectActions, firstChoice, manualWritten } from "../src/manualFlow.ts";
import type { Seen } from "../src/manualFlow.ts";

test("the first found agent group is chosen first; with none found, Manual", () => {
  assert.equal(firstChoice(["Claude", "Codex"]), "Claude");
  assert.equal(firstChoice(["Codex"]), "Codex");
  assert.equal(firstChoice([]), MANUAL);
});

test("Manual closes with Done, even when no known agent is found", () => {
  assert.deepEqual(connectActions(MANUAL, 2, false), { primary: "done", write: false });
  assert.deepEqual(connectActions(MANUAL, 0, false), { primary: "done", write: true });
  // Out of date: the agent updates the files, so Allomos offers no writing.
  assert.deepEqual(connectActions(MANUAL, 0, true), { primary: "done", write: false });
});

test("a found agent is connected; nothing is offered before agents are known", () => {
  assert.deepEqual(connectActions("Claude", 1, false), { primary: "connect", write: false });
  assert.deepEqual(connectActions(null, 0, false), { primary: null, write: true });
});

const seen = (agents_md: boolean, claude_md: boolean, codex: boolean, claude: boolean): Seen => ({
  files: { agents_md, claude_md },
  agents: { codex, claude },
});

test("Manual notices a new instruction file, or one that now mentions Allomos", () => {
  // Nothing changed: nothing seen yet.
  assert.deepEqual(manualWritten(seen(false, false, false, false), seen(false, false, false, false)), []);
  // New files.
  assert.deepEqual(manualWritten(seen(false, false, false, false), seen(true, true, true, true)), ["AGENTS.md", "CLAUDE.md"]);
  // The files were there; the agent added Allomos's section to them.
  assert.deepEqual(manualWritten(seen(true, true, false, false), seen(true, true, true, false)), ["AGENTS.md"]);
  assert.deepEqual(manualWritten(seen(true, true, true, false), seen(true, true, true, true)), ["CLAUDE.md"]);
  // Already connected before: nothing new to notice (Done closes the step).
  assert.deepEqual(manualWritten(seen(true, true, true, true), seen(true, true, true, true)), []);
});
