import { test } from "node:test";
import assert from "node:assert/strict";
import { isPlainEnter } from "../src/enterKey.ts";

test("a plain Enter saves", () => {
  assert.equal(isPlainEnter({ key: "Enter", isComposing: false, keyCode: 13 }), true);
});

test("Enter during composition (Chrome, Firefox) does not save", () => {
  assert.equal(isPlainEnter({ key: "Enter", isComposing: true, keyCode: 229 }), false);
});

test("Enter that confirmed an input method's word in WebKit (composition already ended) does not save", () => {
  assert.equal(isPlainEnter({ key: "Enter", isComposing: false, keyCode: 229 }), false);
});

test("other keys are not Enter", () => {
  assert.equal(isPlainEnter({ key: "a", isComposing: false, keyCode: 65 }), false);
});
