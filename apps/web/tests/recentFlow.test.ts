// Tests for the "Open recent" dialog's list (Task-A01-43). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { folderName, keepListed, noteOpened, parseRecentInfo, recentRows } from "../src/recentFlow.ts";

test("stored notes are read, and anything unreadable counts as none", () => {
  assert.deepEqual(parseRecentInfo(null), {});
  assert.deepEqual(parseRecentInfo("not json"), {});
  assert.deepEqual(parseRecentInfo("[1,2]"), {});
  assert.deepEqual(
    parseRecentInfo(JSON.stringify({ "/p/a": { name: "Bakery", opened: "2026-10-15T08:00:00.000Z" }, "/p/b": 3, "/p/c": { name: 4, opened: "soon" } })),
    { "/p/a": { name: "Bakery", opened: "2026-10-15T08:00:00.000Z" }, "/p/c": {} },
  );
});

test("opening a folder notes its name and time; a missing name keeps the one noted before", () => {
  const now = new Date("2026-10-15T09:30:00.000Z");
  const once = noteOpened({}, "/p/a", "Bakery", now);
  assert.deepEqual(once, { "/p/a": { name: "Bakery", opened: "2026-10-15T09:30:00.000Z" } });
  const later = new Date("2026-10-16T09:30:00.000Z");
  assert.deepEqual(noteOpened(once, "/p/a", null, later), { "/p/a": { name: "Bakery", opened: "2026-10-16T09:30:00.000Z" } });
});

test("notes of folders that left the recent list are dropped", () => {
  const info = { "/p/a": { name: "A" }, "/p/b": { name: "B" } };
  assert.deepEqual(keepListed(info, ["/p/b", "/p/c"]), { "/p/b": { name: "B" } });
  assert.deepEqual(keepListed(info, []), {});
});

test("without a noted name, a line is named after its folder, on macOS and Windows", () => {
  assert.equal(folderName("/Users/me/bakery"), "bakery");
  assert.equal(folderName("/Users/me/bakery/"), "bakery");
  assert.equal(folderName("C:\\Users\\me\\bakery"), "bakery");
  assert.equal(folderName("/"), "/");
});

test("lines keep the recent list's order and mark folders that are gone", () => {
  const rows = recentRows(["/p/new", "/p/gone", "/p/old"], { "/p/new": { name: "New", opened: "2026-10-15T09:30:00.000Z" } }, new Set(["/p/gone"]));
  assert.deepEqual(rows, [
    { folder: "/p/new", name: "New", opened: "2026-10-15T09:30:00.000Z", missing: false },
    { folder: "/p/gone", name: "gone", opened: null, missing: true },
    { folder: "/p/old", name: "old", opened: null, missing: false },
  ]);
  assert.deepEqual(recentRows([], {}, new Set()), []);
});
