// Task navigation from the All monitors list (Task-A03-jazz112).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readTaskJump, taskJumpIsFor } from "../src/taskJumpFlow.ts";

test("a task jump matches the same project whether its folder or taskbag path is used", () => {
  const jump = { root: "/work/project/.allomos/taskbag", id: "A03-jazz112" };
  assert.equal(taskJumpIsFor(jump, "/work/project"), true);
  assert.equal(taskJumpIsFor(jump, "/work/project/.allomos/taskbag/"), true);
  assert.equal(taskJumpIsFor(jump, "/work/other"), false);
});

test("a stored task jump is read only when it contains string project and task IDs", () => {
  assert.deepEqual(readTaskJump('{"root":"/work/project","id":"A03-jazz112"}'), { root: "/work/project", id: "A03-jazz112" });
  assert.equal(readTaskJump(null), null);
  assert.equal(readTaskJump("not json"), null);
  assert.equal(readTaskJump('{"root":"/work/project","id":3}'), null);
});
