// Tests for collapsing and expanding sections (Task-A01-26). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { foldStorageKey, parseKeys, setKeys, toggleKey, visibleItems, withKey, withoutKey } from "../src/foldFlow.ts";

test("each project and each list has its own storage key", () => {
  assert.notEqual(foldStorageKey("/p/one", "groups"), foldStorageKey("/p/two", "groups"));
  assert.notEqual(foldStorageKey("/p/one", "groups"), foldStorageKey("/p/one", "handoffs"));
});

test("remembered keys survive a round trip; unreadable storage counts as nothing", () => {
  assert.deepEqual(parseKeys(JSON.stringify(["a-A01", "i-I02"])), ["a-A01", "i-I02"]);
  assert.deepEqual(parseKeys(null), []);
  assert.deepEqual(parseKeys("not json"), []);
  assert.deepEqual(parseKeys('{"a": 1}'), []);
  assert.deepEqual(parseKeys('["a-A01", 3, "a-A01"]'), ["a-A01"]);
});

test("toggling collapses a section, toggling again expands it", () => {
  const once = toggleKey([], "a-A01");
  assert.deepEqual(once, ["a-A01"]);
  assert.deepEqual(toggleKey(once, "a-A01"), []);
});

test("collapse all and expand all act only on the sections shown now", () => {
  // Grouped by iteration: an area collapsed earlier stays collapsed.
  const keys = ["a-A01"];
  const iterations = ["i-I01", "i-I02", "unplanned"];
  const all = setKeys(keys, iterations, true);
  assert.deepEqual(all, ["a-A01", "i-I01", "i-I02", "unplanned"]);
  assert.deepEqual(setKeys(all, iterations, true), all, "twice changes nothing");
  assert.deepEqual(setKeys(all, iterations, false), ["a-A01"]);
});

test("a task opened from elsewhere expands its group", () => {
  assert.deepEqual(withoutKey(["a-A01", "a-A02"], "a-A01"), ["a-A02"]);
  assert.deepEqual(withoutKey(["a-A02"], "a-A01"), ["a-A02"]);
  assert.deepEqual(withKey(["a-A02"], "a-A02"), ["a-A02"]);
});

test("the arrow keys skip tasks in collapsed groups", () => {
  const groups = [
    { key: "a-A01", tasks: ["A01-01", "A01-02"] },
    { key: "a-A02", tasks: ["A02-01"] },
    { key: "a-A03", tasks: ["A03-01"] },
  ];
  assert.deepEqual(visibleItems(groups, []), ["A01-01", "A01-02", "A02-01", "A03-01"]);
  assert.deepEqual(visibleItems(groups, ["a-A02"]), ["A01-01", "A01-02", "A03-01"]);
});
