import { test } from "node:test";
import assert from "node:assert/strict";
import { addRule, changeRule, cleanRule, removeRule } from "../src/riskFlow.ts";

const rules = ["Changing prices is high risk.", "Touching the importer is high risk."];

test("a rule is kept on one line, without spaces around it", () => {
  assert.equal(cleanRule("  Changing prices\n is  high risk. "), "Changing prices is high risk.");
  assert.equal(cleanRule(" \n "), "");
});

test("a rule is added at the end; an empty one or one already there is not", () => {
  assert.deepEqual(addRule(rules, " A new one. "), [...rules, "A new one."]);
  assert.equal(addRule(rules, "   "), null);
  assert.equal(addRule(rules, "Changing prices  is high risk."), null);
  assert.deepEqual(addRule([], "First."), ["First."]);
});

test("a rule is reworded in place; empty wording or another rule's wording is refused", () => {
  assert.deepEqual(changeRule(rules, 1, "Touching the exporter is high risk."), [rules[0], "Touching the exporter is high risk."]);
  assert.deepEqual(changeRule(rules, 0, rules[0]), rules);
  assert.equal(changeRule(rules, 0, ""), null);
  assert.equal(changeRule(rules, 0, rules[1]), null);
  assert.equal(changeRule(rules, 5, "x"), null);
});

test("a rule is removed by its place; the others keep their order", () => {
  assert.deepEqual(removeRule(rules, 0), [rules[1]]);
  assert.deepEqual(removeRule(rules, 9), rules);
});
