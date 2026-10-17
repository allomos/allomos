// Tests for the agents panel's setup prompt (Task-A03-24). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { agentFilesCommand, notSetUp, taskbagInside } from "../src/agentSetupFlow.ts";

test("an agent is set up when an entry under Agents set up names it", () => {
  const entries = [
    "Claude Code: `CLAUDE.md` imports AGENTS.md and keeps only its own lines. 2026-10-16.",
    "Codex: reads `AGENTS.md` directly; no file of its own. 2026-10-16.",
  ];
  assert.deepEqual(notSetUp(["Claude", "Codex"], entries), []);
  assert.deepEqual(notSetUp(["Claude", "Codex"], [entries[1]]), ["Claude"]);
  // Nobody listed (a new taskbag says "None yet."): every found agent gets the note.
  assert.deepEqual(notSetUp(["Claude", "Codex"], []), ["Claude", "Codex"]);
  // Any case, Markdown marks ignored; only the name before the colon counts, as a whole word.
  assert.deepEqual(notSetUp(["Claude", "Codex"], ["**codex**: AGENTS.md", "Gemini CLI: mentions Claude"]), ["Claude"]);
  assert.deepEqual(notSetUp(["Codex"], ["Codexify: x"]), ["Codex"]);
});

test("the prompt names the taskbag only when it is outside the project folder", () => {
  assert.equal(agentFilesCommand("/p/app", "/p/app/.allomos/taskbag"), "alm guide agent-files");
  assert.equal(agentFilesCommand("C:\\p\\app\\", "C:\\p\\app\\.allomos\\taskbag"), "alm guide agent-files");
  assert.equal(
    agentFilesCommand("/p/app", "/p/app-taskbag/.allomos/taskbag"),
    'alm --taskbag "/p/app-taskbag/.allomos/taskbag" guide agent-files',
  );
  assert.equal(taskbagInside("/p/app", "/p/app"), false);
});
