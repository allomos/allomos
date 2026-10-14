// Tests for the paths the interface works out (Task-A07-03). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { projectFolder } from "../src/paths.ts";

test("the project folder is found from a taskbag path with / or \\", () => {
  assert.equal(projectFolder("/p/app/.allomos/taskbag"), "/p/app");
  assert.equal(projectFolder("/p/app/.allomos/taskbag/"), "/p/app");
  assert.equal(projectFolder("C:\\Users\\me\\app\\.allomos\\taskbag"), "C:\\Users\\me\\app");
  assert.equal(projectFolder("C:\\Users\\me\\app\\.allomos\\taskbag\\"), "C:\\Users\\me\\app");
  // An older Allomos on Windows gave paths with both.
  assert.equal(projectFolder("C:\\Users\\me\\app\\.allomos/taskbag"), "C:\\Users\\me\\app");
});

test("a taskbag elsewhere is its own folder", () => {
  assert.equal(projectFolder("/p/tasks"), "/p/tasks");
  assert.equal(projectFolder("D:\\tasks"), "D:\\tasks");
});
