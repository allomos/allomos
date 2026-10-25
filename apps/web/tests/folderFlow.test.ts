// Tests for project folders in the task list and pane (Task-A01-sketch327).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { folderFilterValue, keepFolder, newTaskFolders, readFolderFilter, showFolders, toggleFolder, type FolderInfo } from "../src/folderFlow.ts";
import type { Task } from "../src/types.ts";

const folder = (name: string): FolderInfo => ({ name, path: name, exists: true, section: true, tasks: 0, open: 0 });
const two = [folder("app"), folder("site")];
const task = (folders?: string[]) => ({ id: "A01-01", folders }) as unknown as Task;

test("chips and the filter only with two or more folders", () => {
  assert.equal(showFolders([]), false);
  assert.equal(showFolders([folder("app")]), false);
  assert.equal(showFolders(two), true);
});

test("a stored filter is read back, and cleared when it no longer fits", () => {
  assert.deepEqual(readFolderFilter("site", two), { kind: "folder", name: "site" });
  assert.deepEqual(readFolderFilter("none", two), { kind: "none" });
  assert.deepEqual(readFolderFilter("gone", two), { kind: "all" });
  assert.deepEqual(readFolderFilter("site", [folder("site")]), { kind: "all" });
  assert.deepEqual(readFolderFilter(null, two), { kind: "all" });
  assert.equal(folderFilterValue({ kind: "folder", name: "site" }), "site");
  assert.equal(folderFilterValue({ kind: "none" }), "none");
});

test("which tasks a filter keeps, and what a new task gets", () => {
  assert.equal(keepFolder({ kind: "all" }, task()), true);
  assert.equal(keepFolder({ kind: "none" }, task()), true);
  assert.equal(keepFolder({ kind: "none" }, task(["app"])), false);
  assert.equal(keepFolder({ kind: "folder", name: "app" }, task(["app", "site"])), true);
  assert.equal(keepFolder({ kind: "folder", name: "app" }, task([])), false);
  assert.deepEqual(newTaskFolders({ kind: "folder", name: "site" }), ["site"]);
  assert.deepEqual(newTaskFolders({ kind: "all" }), []);
});

test("ticking keeps the folders' order", () => {
  assert.deepEqual(toggleFolder(["site"], "app", two), ["app", "site"]);
  assert.deepEqual(toggleFolder(["app", "site"], "app", two), ["site"]);
});
