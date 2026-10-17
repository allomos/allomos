// Tests for an agent not yet set up as part of Update the taskbag… (Task-A03-24).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { notSetUp, setupNotice } from "../src/agentSetupFlow.ts";

test("an agent is set up when an entry under Agents set up names it", () => {
  const entries = [
    "Claude Code: `CLAUDE.md` imports AGENTS.md and keeps only its own lines. 2026-10-16.",
    "Codex: reads `AGENTS.md` directly; no file of its own. 2026-10-16.",
  ];
  assert.deepEqual(notSetUp(["Claude", "Codex"], entries), []);
  assert.deepEqual(notSetUp(["Claude", "Codex"], [entries[1]]), ["Claude"]);
  // Nobody listed (a new taskbag says "None yet."): every found agent.
  assert.deepEqual(notSetUp(["Claude", "Codex"], []), ["Claude", "Codex"]);
  // Any case, Markdown marks ignored; only the name before the colon counts, as a whole word.
  assert.deepEqual(notSetUp(["Claude", "Codex"], ["**codex**: AGENTS.md", "Gemini CLI: mentions Claude"]), ["Claude"]);
  assert.deepEqual(notSetUp(["Codex"], ["Codexify: x"]), ["Codex"]);
});

test("the notice shows for a connected project with an agent not set up and no other update notice", () => {
  assert.equal(setupNotice(["Codex"], true, false), true);
  assert.equal(setupNotice([], true, false), false);
  // Not connected yet: Connect with the agents does the setup.
  assert.equal(setupNotice(["Codex"], false, false), false);
  // Card files outside, or an older taskbag: that notice's Update the taskbag… covers it.
  assert.equal(setupNotice(["Codex"], true, true), false);
});
