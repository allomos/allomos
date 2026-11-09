// Tests for the Codex rules option in Monitor ready tasks… (Task-A03-pier032): what it offers for
// each state of ~/.codex/rules/allomos.rules, whether it starts checked, and what Start does.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { codexActionAtStart, codexFirstChecked, codexOffer, rulesList } from "../src/codexRulesFlow.ts";
import type { CodexRules } from "../src/codexRulesFlow.ts";

const base = { path: "/h/.codex/rules/allomos.rules", shipped: 2, rules: [] };
const none = { added: [], removed: [] };

test("each state of the file gets its offer and first check", () => {
  const cases: [CodexRules, string, boolean][] = [
    [{ ...base, state: "missing" }, "add", true],
    [{ ...base, state: "older", version: 1, changes: none }, "update", true],
    [{ ...base, state: "changed", changes: none }, "changed", false],
    [{ ...base, state: "current" }, "keep", true],
    [{ ...base, state: "newer", version: 3 }, "newer", false],
  ];
  for (const [rules, offer, checked] of cases) {
    assert.equal(codexOffer(rules), offer);
    assert.equal(codexFirstChecked(codexOffer(rules)), checked);
  }
});

test("Start writes only when the option says so", () => {
  assert.equal(codexActionAtStart("add", true), "install");
  assert.equal(codexActionAtStart("add", false), null);
  assert.equal(codexActionAtStart("update", true), "install");
  // Unchecked, an update keeps the old file this time.
  assert.equal(codexActionAtStart("update", false), null);
  assert.equal(codexActionAtStart("changed", true), "install");
  assert.equal(codexActionAtStart("changed", false), null);
  assert.equal(codexActionAtStart("keep", true), null);
  // Unchecked later, Allomos removes its file.
  assert.equal(codexActionAtStart("keep", false), "remove");
  assert.equal(codexActionAtStart("newer", true), null);
});

test("a change lists the added and the removed rules, forbidden ones marked", () => {
  assert.equal(rulesList([], "never:", "none"), "none");
  assert.equal(
    rulesList(
      [
        { command: "git pull", decision: "allow" },
        { command: "git push --force", decision: "forbidden" },
      ],
      "never:",
      "none",
    ),
    "git pull, never: git push --force",
  );
});
