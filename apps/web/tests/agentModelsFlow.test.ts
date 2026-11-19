// Tests for the model and effort choices of Monitor ready tasks (A03-alto709): the options each
// agent is offered from the online list, what Start sends, the notice when the list could not be
// read, and the model list address check.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { chosenArgs, listNotice, optionsFor, plainUrl } from "../src/agentModelsFlow.ts";
import type { ModelsRead } from "../src/agentModelsFlow.ts";

const online: ModelsRead = {
  from: "online",
  failed: null,
  url: "https://www.allomos.com/agents/models.json",
  agents: [
    { id: "claude_cli", name: "Claude Code", models: [{ value: "opus", label: "Opus" }, { value: "sonnet", label: "Sonnet" }], efforts: ["low", "high"] },
    { id: "codex_cli", name: "Codex", models: [{ value: "gpt-6.2-sol", label: "GPT-6.2 Sol" }], efforts: ["xhigh"] },
  ],
};

test("each agent, and each of its accounts, is offered its list; an unlisted agent only Default", () => {
  assert.deepEqual(optionsFor(online, "claude_cli").efforts, ["low", "high"]);
  assert.deepEqual(optionsFor(online, "claude_cli:work").models.map((m) => m.value), ["opus", "sonnet"]);
  assert.deepEqual(optionsFor(online, "antigravity_cli"), { models: [], efforts: [] });
  assert.deepEqual(optionsFor(null, "claude_cli"), { models: [], efforts: [] });
});

test("Default sends nothing; a choice sends its value; a value the list no longer has falls back", () => {
  assert.deepEqual(chosenArgs(["claude_cli", "codex_cli"], {}, online), { models: {}, efforts: {} });
  assert.deepEqual(
    chosenArgs(["claude_cli", "codex_cli"], { claude_cli: { model: "opus" }, codex_cli: { effort: "xhigh" } }, online),
    { models: { claude_cli: "opus" }, efforts: { codex_cli: "xhigh" } },
  );
  // Only the agents chosen, and only values still listed (after a Refresh, or with no list).
  assert.deepEqual(chosenArgs(["codex_cli"], { claude_cli: { model: "opus" }, codex_cli: { model: "gone" } }, online), { models: {}, efforts: {} });
  const none: ModelsRead = { agents: null, from: "none", failed: "fetch", url: online.url };
  assert.deepEqual(chosenArgs(["claude_cli"], { claude_cli: { model: "opus", effort: "high" } }, none), { models: {}, efforts: {} });
});

test("a list that could not be read says so: the saved copy, no list, or an unknown format", () => {
  assert.equal(listNotice(online), null);
  assert.equal(listNotice(null), null);
  assert.equal(listNotice({ ...online, from: "cache", failed: "fetch" }), "monitor.models_cached");
  assert.equal(listNotice({ ...online, from: "cache", failed: "format" }), "monitor.models_cached");
  assert.equal(listNotice({ agents: null, from: "none", failed: "fetch", url: "" }), "monitor.models_none");
  assert.equal(listNotice({ agents: null, from: "none", failed: "format", url: "" }), "monitor.models_bad_format");
});

test("the model list address is a web address", () => {
  assert.ok(plainUrl("https://www.allomos.com/agents/models.json"));
  assert.ok(plainUrl(" http://localhost:8080/m.json "));
  assert.ok(!plainUrl("www.allomos.com/agents/models.json"));
  assert.ok(!plainUrl("https://"));
  assert.ok(!plainUrl("file:///etc/passwd"));
  assert.ok(!plainUrl("https://a b"));
});
