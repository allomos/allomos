// Tests for the link from the Monitors list to an iteration (Task-A03-meteor649): which project a
// request is for, reading and taking a request kept in storage, and asking for one.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { JUMP_KEY, askJump, jumpIsFor, projectKey, readJump, takeJump } from "../src/iterationJumpFlow.ts";

/** A stand-in for the browser's local storage and window, for the calls the flow makes. */
function browser() {
  const store = new Map<string, string>();
  const heard: string[] = [];
  (globalThis as { localStorage?: unknown }).localStorage = {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  };
  (globalThis as { window?: unknown }).window = {
    dispatchEvent: (e: Event) => heard.push(e.type),
  };
  return { store, heard };
}

test("a project is the same whether it is given as its folder or as its taskbag", () => {
  assert.equal(projectKey("/Users/me/jobs/.allomos/taskbag"), "/Users/me/jobs");
  assert.equal(projectKey("/Users/me/jobs/.allomos/taskbag/"), "/Users/me/jobs");
  assert.equal(projectKey("C:\\Users\\me\\jobs\\.allomos\\taskbag"), "C:\\Users\\me\\jobs");
  assert.equal(projectKey("/Users/me/jobs/"), "/Users/me/jobs");
  assert.equal(jumpIsFor({ root: "/a/b/.allomos/taskbag", id: "I-sand" }, "/a/b"), true);
  assert.equal(jumpIsFor({ root: "/a/b/.allomos/taskbag", id: "I-sand" }, "/a/c"), false);
  assert.equal(jumpIsFor(null, "/a/b"), false);
});

test("a request kept in storage is read only when it is well formed", () => {
  assert.deepEqual(readJump('{"root":"/a/b/.allomos/taskbag","id":"I-sand"}'), { root: "/a/b/.allomos/taskbag", id: "I-sand" });
  assert.equal(readJump(null), null);
  assert.equal(readJump("not json"), null);
  assert.equal(readJump('{"root":3}'), null);
});

test("taking a request is for its project only, and it is taken once", () => {
  const { store } = browser();
  store.set(JUMP_KEY, '{"root":"/a/b/.allomos/taskbag","id":"I-sand"}');
  assert.equal(takeJump("/x/y"), null);
  assert.ok(store.has(JUMP_KEY), "another project's request stays where it is");
  assert.deepEqual(takeJump("/a/b"), { root: "/a/b/.allomos/taskbag", id: "I-sand" });
  assert.equal(takeJump("/a/b"), null);
  assert.equal(store.has(JUMP_KEY), false);
});

test("asking keeps the request, tells this window, and then opens the project", async () => {
  const { store, heard } = browser();
  const calls: string[] = [];
  await askJump({ root: "/a/b/.allomos/taskbag", id: "I-sand" }, async () => void calls.push("open"));
  assert.deepEqual(JSON.parse(store.get(JUMP_KEY)!), { root: "/a/b/.allomos/taskbag", id: "I-sand" });
  assert.deepEqual(heard, ["allomos:jump-iteration"]);
  assert.deepEqual(calls, ["open"]);
});
