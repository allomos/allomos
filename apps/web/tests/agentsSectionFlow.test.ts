// Tests for the Agents section of the taskbag's settings (Task-A05-bag963).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { agentRows, rowText, type SectionFolder } from "../src/agentsSectionFlow.ts";

const folder = (name: string, claude: boolean, codex: boolean, exists = true): SectionFolder => ({
  name,
  full: `/p/${name}`,
  exists,
  setup: exists ? { claude, codex } : null,
});

test("two folders, one set up for Claude Code: 1 of 2, naming the other", () => {
  const [claude, codex] = agentRows([folder("app", true, true), folder("site", false, true)]);
  assert.equal(claude.name, "Claude Code");
  assert.deepEqual([claude.done, claude.total], [1, 2]);
  assert.deepEqual(claude.missing, [{ name: "site", full: "/p/site", exists: true }]);
  assert.deepEqual(rowText(claude), { key: "agents_section.count", done: 1, total: 2 });
  assert.equal(codex.name, "Codex");
  assert.deepEqual([codex.done, codex.total, codex.missing.length], [2, 2, 0]);
});

test("a folder not found counts as missing and says so", () => {
  const [claude] = agentRows([folder("app", true, true), folder("gone", false, false, false)]);
  assert.deepEqual(claude.missing, [{ name: "gone", full: "/p/gone", exists: false }]);
});

test("one folder says set up or not set up, as the agent line does", () => {
  const [claude, codex] = agentRows([folder("app", true, false)]);
  assert.equal(rowText(claude).key, "agents_section.set_up");
  assert.equal(rowText(codex).key, "agents_section.not_set_up");
  assert.equal(rowText(agentRows([])[0]).key, "agents_section.not_set_up");
});
