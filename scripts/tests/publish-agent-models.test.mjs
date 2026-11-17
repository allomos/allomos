// Tests for scripts/publish-agent-models.mjs (task A03-coral174). Run:
//   node --test scripts/tests/publish-agent-models.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, statSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { check, publish, render } from "../publish-agent-models.mjs";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const good = () => ({
  version: 1,
  updated: "2026-11-17",
  agents: [
    { id: "claude_cli", name: "Claude Code", models: [{ value: "opus", label: "Opus（最新）" }], efforts: ["low", "high"] },
    { id: "codex_cli", name: "Codex", models: [{ value: "gpt-6", label: "GPT-6" }], efforts: ["medium"] },
  ],
});
const has = (problems, words) => assert.ok(problems.some((p) => p.includes(words)), `expected "${words}" in ${JSON.stringify(problems)}`);

test("the repository's own list passes the check", () => {
  const data = JSON.parse(readFileSync(join(repo, "data", "agent-models.json"), "utf8"));
  assert.deepEqual(check(data), []);
});

test("a good list has no problems", () => assert.deepEqual(check(good()), []));

test("wrong version, bad date and unknown fields are refused", () => {
  const d = good();
  d.version = 2; d.updated = "2026-02-30"; d.extra = true;
  const p = check(d);
  has(p, '"version" must be 1');
  has(p, '"updated" must be a date');
  has(p, 'unknown field "extra"');
});

test("no agents, or an agent with no models or efforts, is refused", () => {
  has(check({ ...good(), agents: [] }), "at least one agent");
  const d = good();
  d.agents[0].models = [];
  d.agents[1].efforts = [];
  const p = check(d);
  has(p, 'agent "claude_cli": "models" must list at least one model');
  has(p, 'agent "codex_cli": "efforts" must list at least one reasoning effort');
});

test("duplicates are refused: agent id, model value, effort", () => {
  const d = good();
  d.agents[1].id = "claude_cli";
  d.agents[0].models.push({ value: "opus", label: "again" });
  d.agents[0].efforts.push("low");
  const p = check(d);
  has(p, 'agent "claude_cli" is listed twice');
  has(p, 'model "opus" is listed twice');
  has(p, 'effort "low" is listed twice');
});

test("bad ids, empty names and malformed models are refused", () => {
  const d = good();
  d.agents[0].id = "Claude CLI";
  d.agents[1].name = " ";
  d.agents[1].models = [{ value: "gpt-6" }];
  const p = check(d);
  has(p, 'agent 1: "id" must be lower case');
  has(p, '"name" must not be empty');
  has(p, 'must have exactly a "value" and a "label"');
});

test("publish writes once, then changes nothing; a bad source writes nothing", () => {
  const dir = mkdtempSync(join(tmpdir(), "agent-models-"));
  try {
    const source = join(dir, "src.json");
    const target = join(dir, "site", "agents", "models.json");
    writeFileSync(source, JSON.stringify(good()));
    assert.deepEqual(publish(source, target), { changed: true, problems: [] });
    assert.equal(readFileSync(target, "utf8"), render(good()));
    const before = statSync(target).mtimeMs;
    assert.deepEqual(publish(source, target), { changed: false, problems: [] });
    assert.equal(statSync(target).mtimeMs, before);

    writeFileSync(source, "{ not json");
    const r = publish(source, target);
    assert.equal(r.changed, false);
    has(r.problems, "cannot read");
    assert.equal(readFileSync(target, "utf8"), render(good()));

    const other = join(dir, "other", "models.json");
    writeFileSync(source, JSON.stringify({ ...good(), agents: [] }));
    assert.equal(publish(source, other).changed, false);
    assert.equal(existsSync(other), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
