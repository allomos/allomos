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
    // Format 2 (A03-pony586): models of one agent offer different levels, and one takes none.
    {
      id: "antigravity_cli",
      name: "Antigravity",
      models: [
        { value: "gemini-4-neon", label: "Gemini 4 Neon", efforts: ["low", "medium", "high", "xhigh"] },
        { value: "gemini-4-carbon", label: "Gemini 4 Carbon", efforts: ["low", "medium", "high"] },
        { value: "gpt-oss-120b", label: "GPT-OSS 120B", efforts: ["medium"] },
        { value: "gemini-pro-orchestrator", label: "Orchestrator", efforts: [] },
      ],
      efforts: ["low", "medium", "high", "xhigh"],
    },
  ],
};

test("each agent, and each of its accounts, is offered its list; an unlisted agent only Default", () => {
  assert.deepEqual(optionsFor(online, "claude_cli").efforts, ["low", "high"]);
  assert.deepEqual(optionsFor(online, "claude_cli:work").models.map((m) => m.value), ["opus", "sonnet"]);
  assert.deepEqual(optionsFor(online, "grok_cli"), { models: [], efforts: [] });
  assert.deepEqual(optionsFor(null, "claude_cli"), { models: [], efforts: [] });
});

test("the efforts offered follow the chosen model: its own list, the agent's, or none", () => {
  assert.deepEqual(optionsFor(online, "antigravity_cli", "gemini-4-carbon").efforts, ["low", "medium", "high"]);
  assert.deepEqual(optionsFor(online, "antigravity_cli", "gpt-oss-120b").efforts, ["medium"]);
  assert.deepEqual(optionsFor(online, "antigravity_cli", "gemini-pro-orchestrator").efforts, []);
  // A model of an agent still listed in format 1, and an unknown model, use the agent's efforts.
  assert.deepEqual(optionsFor(online, "claude_cli", "opus").efforts, ["low", "high"]);
  assert.deepEqual(optionsFor(online, "antigravity_cli", "gone").efforts, ["low", "medium", "high", "xhigh"]);
});

test("an effort the chosen model does not offer is not sent", () => {
  assert.deepEqual(
    chosenArgs(["antigravity_cli"], { antigravity_cli: { model: "gemini-4-carbon", effort: "xhigh" } }, online),
    { models: { antigravity_cli: "gemini-4-carbon" }, efforts: {} },
  );
  assert.deepEqual(
    chosenArgs(["antigravity_cli"], { antigravity_cli: { model: "gemini-4-neon", effort: "xhigh" } }, online),
    { models: { antigravity_cli: "gemini-4-neon" }, efforts: { antigravity_cli: "xhigh" } },
  );
  // A model that takes no reasoning effort sends no effort, even one the agent offers.
  assert.deepEqual(
    chosenArgs(["antigravity_cli"], { antigravity_cli: { model: "gemini-pro-orchestrator", effort: "high" } }, online),
    { models: { antigravity_cli: "gemini-pro-orchestrator" }, efforts: {} },
  );
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
