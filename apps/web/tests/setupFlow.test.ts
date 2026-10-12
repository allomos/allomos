// Tests for the setup wizard's choices (Task-A05-03). Run: npm --prefix apps/web test
// Node runs TypeScript here by stripping the types; no test packages are needed.

import { test } from "node:test";
import assert from "node:assert/strict";
import { defaultSource, directStep, neverConnected, newFiles, offerConnect, resumeAt, stillMissing, taskbagStep } from "../src/setupFlow.ts";
import type { SetupState } from "../src/types.ts";

test("a folder with .allomos goes on with Next (no Change)", () => {
  const step = taskbagStep({ kind: "found", root: "/p/.allomos/taskbag" });
  assert.deepEqual(step, { kind: "found", root: "/p/.allomos/taskbag", buttons: ["next"] });
});

test("a folder without .allomos offers Create, empty or not", () => {
  for (const kind of ["empty", "no_taskbag"] as const) {
    const step = taskbagStep({ kind, folder: "/p", suggested_root: "/p/.allomos/taskbag" });
    assert.deepEqual(step, { kind: "missing", folder: "/p", suggestedRoot: "/p/.allomos/taskbag", buttons: ["create"] });
  }
  assert.deepEqual(taskbagStep({ kind: "not_a_folder", path: "/x" }), { kind: "not_a_folder", path: "/x" });
});

const setup = (over: Partial<SetupState> = {}): SetupState => ({
  allomos_folder: "/p",
  project_folder: null,
  agents: { codex: false, claude: false },
  has_tasks: false,
  ...over,
});

test("the project source starts as the step-1 folder, or the one already recorded", () => {
  assert.equal(defaultSource(setup()), "/p");
  assert.equal(defaultSource(setup({ project_folder: "/code/app" })), "/code/app");
});

test("Connect with the agents is offered when agents are not told or there are no tasks", () => {
  const told = { codex: true, claude: true };
  assert.equal(offerConnect(setup(), false), false, "no project folder yet");
  assert.equal(offerConnect(setup({ project_folder: "/p" }), false), true);
  assert.equal(offerConnect(setup({ project_folder: "/p", agents: told }), false), true, "no tasks");
  assert.equal(offerConnect(setup({ project_folder: "/p", agents: { codex: true, claude: false } , has_tasks: true }), false), true);
  assert.equal(offerConnect(setup({ project_folder: "/p", agents: told, has_tasks: true }), false), false);
  assert.equal(offerConnect(setup({ project_folder: "/p" }), true), false, "Not now holds");
});

test("the restart notice names only files that were missing and now exist", () => {
  const none = { agents_md: false, claude_md: false };
  const both = { agents_md: true, claude_md: true };
  assert.deepEqual(newFiles(none, both), ["AGENTS.md", "CLAUDE.md"]);
  assert.deepEqual(newFiles(none, { agents_md: true, claude_md: false }), ["AGENTS.md"]);
  assert.deepEqual(newFiles(both, both), [], "files that already existed and were only changed");
  assert.deepEqual(newFiles({ agents_md: true, claude_md: false }, both), ["CLAUDE.md"]);
  assert.equal(stillMissing(none, { agents_md: true, claude_md: false }), true);
  assert.equal(stillMissing(none, both), false);
  assert.equal(stillMissing(both, both), false);
});

test("opening a taskbag directly opens the wizard at the step that is missing", () => {
  const told = { codex: true, claude: true };
  assert.equal(directStep(setup(), false), 3, "no project folder: step 3");
  assert.equal(directStep(setup({ project_folder: "/p" }), false), 4, "agents not told: step 4");
  assert.equal(directStep(setup({ project_folder: "/p", agents: told }), false), 4, "no tasks: step 4");
  assert.equal(directStep(setup({ project_folder: "/p", agents: told, has_tasks: true }), false), null, "all set");
  assert.equal(directStep(setup(), true), null, "closed this time: not again until reopened");
});

test("File → Connect with the Agents… opens step 4, or step 3 without a project folder", () => {
  assert.equal(resumeAt(setup({ project_folder: "/p" }), 4), 4);
  assert.equal(resumeAt(setup(), 4), 3);
  assert.equal(resumeAt(setup({ project_folder: "/p" }), 3), 3);
});

test("Not now goes back to the start page only when no agent was ever told", () => {
  assert.equal(neverConnected(setup()), true);
  assert.equal(neverConnected(setup({ agents: { codex: true, claude: false } })), false);
  assert.equal(neverConnected(setup({ agents: { codex: false, claude: true } })), false);
  assert.equal(neverConnected(setup({ agents: { codex: true, claude: true } })), false);
});
