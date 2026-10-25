// Tests for the companion page's choices (Task-A04-02-03). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { shownRoot } from "../src/companionFlow.ts";

const bag = (root: string) => ({ root, name: root.toUpperCase() });

test("the only open taskbag is shown; with several, the chosen one or none", () => {
  assert.equal(shownRoot([], null), null);
  assert.equal(shownRoot([bag("/a")], null), "/a");
  assert.equal(shownRoot([bag("/a"), bag("/b")], null), null);
  assert.equal(shownRoot([bag("/a"), bag("/b")], "/b"), "/b");
  // A chosen taskbag whose window was closed is no longer shown.
  assert.equal(shownRoot([bag("/a"), bag("/c")], "/b"), null);
  assert.equal(shownRoot([bag("/a")], "/b"), "/a");
});
