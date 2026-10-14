// Tests for searching the task list (Task-A01-36). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  collapsedWhileSearching,
  handoffMatches,
  hitRanges,
  pieces,
  searchGroups,
  searchTerms,
  snippet,
  taskMatches,
} from "../src/searchFlow.ts";
import type { Task } from "../src/types.ts";

function task(id: string, title: string, description = "", area = "A01"): Task {
  return {
    id,
    title,
    area,
    state: "ready",
    label: null,
    holder: null,
    waiting_on: null,
    next_dependency: null,
    iteration: null,
    handoff_note: null,
    without_human_check: null,
    human_check_steps: null,
    description,
  };
}

test("words are split at spaces, also the full-width space, and compared in lower case", () => {
  assert.deepEqual(searchTerms("  Login   PAGE "), ["login", "page"]);
  assert.deepEqual(searchTerms("登录　页面"), ["登录", "页面"]);
  assert.deepEqual(searchTerms("a A"), ["a"]);
  assert.deepEqual(searchTerms("   "), []);
});

test("a task is found by its ID, title or description, in any case", () => {
  const t = task("A01-36", "Search box", "Filters the **task list** as you type.");
  assert.ok(taskMatches(t, searchTerms("a01-36")));
  assert.ok(taskMatches(t, searchTerms("SEARCH")));
  assert.ok(taskMatches(t, searchTerms("task list")));
  assert.ok(!taskMatches(t, searchTerms("burndown")));
  // Every word must be found, each in any of the three.
  assert.ok(taskMatches(t, searchTerms("a01 type")));
  assert.ok(!taskMatches(t, searchTerms("search burndown")));
});

test("Chinese and Japanese text is found, also across full-width and half-width forms", () => {
  assert.ok(taskMatches(task("A02-01", "应用主窗口有搜索框"), searchTerms("搜索")));
  assert.ok(taskMatches(task("A02-02", "検索ボックスを追加する"), searchTerms("ボックス")));
  // Half-width katakana with a voicing mark, written as two characters, matches the full-width one.
  assert.ok(taskMatches(task("A02-03", "ｶﾞｲﾄﾞを書く"), searchTerms("ガイド")));
  // Full-width Latin letters and digits match plain ones.
  assert.ok(taskMatches(task("A02-04", "ＡＰＩ　キー"), searchTerms("api")));
  assert.ok(!taskMatches(task("A02-05", "进度图"), searchTerms("搜索")));
});

test("the found places are given in the text as written, joined where they touch", () => {
  assert.deepEqual(hitRanges("Search the search box", ["search"]), [
    [0, 6],
    [11, 17],
  ]);
  assert.deepEqual(hitRanges("abcd", ["ab", "bc"]), [[0, 3]]);
  // ｶﾞ is two units in the text but one character when compared: the mark covers both.
  assert.deepEqual(hitRanges("ｶﾞｲﾄﾞ", ["ガイド"]), [[0, 5]]);
  assert.deepEqual(hitRanges("text", []), []);
});

test("text is cut into marked and unmarked stretches", () => {
  assert.deepEqual(pieces("应用主窗口有搜索框", ["搜索"]), [
    { text: "应用主窗口有", hit: false },
    { text: "搜索", hit: true },
    { text: "框", hit: false },
  ]);
  assert.deepEqual(pieces("ID", ["id"]), [{ text: "ID", hit: true }]);
  assert.deepEqual(pieces("nothing", ["x"]), [{ text: "nothing", hit: false }]);
  assert.deepEqual(pieces("", ["x"]), [{ text: "", hit: false }]);
});

test("a long description shows a short line around its first match", () => {
  const long = `${"a".repeat(100)}\nthe NEEDLE here\n${"b".repeat(100)}`;
  const s = snippet(long, ["needle"], 10);
  assert.ok(s);
  const text = s.map((p) => p.text).join("");
  assert.ok(text.startsWith("…") && text.endsWith("…"));
  assert.ok(!text.includes("\n"));
  assert.deepEqual(
    s.filter((p) => p.hit).map((p) => p.text),
    ["NEEDLE"],
  );
  assert.equal(snippet("short text", ["short"])?.map((p) => p.text).join(""), "short text");
  assert.equal(snippet("short text", ["none"]), null);
});

test("a cut never splits a character written with two units", () => {
  const s = snippet(`${"😀".repeat(30)}找到`, ["找到"], 5);
  assert.ok(s);
  assert.ok(!/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/.test(s.map((p) => p.text).join("")));
});

test("handoffs are found by summary, text, writer or the tasks they name", () => {
  const h = { summary: "交接：搜索框完成一半", body: "Next: highlight the words.", writer: "claude/a01-36", tasks: ["A01-36"] };
  assert.ok(handoffMatches(h, searchTerms("搜索框")));
  assert.ok(handoffMatches(h, searchTerms("HIGHLIGHT")));
  assert.ok(handoffMatches(h, searchTerms("claude")));
  assert.ok(handoffMatches(h, searchTerms("a01-36 highlight")));
  assert.ok(!handoffMatches(h, searchTerms("burndown")));
});

test("groups keep only matching tasks, keep their whole count, and empty groups are left out", () => {
  const groups = [
    { key: "a-A01", tasks: [task("A01-01", "搜索框"), task("A01-02", "进度图")] },
    { key: "a-A02", tasks: [task("A02-01", "导出")] },
  ];
  const found = searchGroups(groups, searchTerms("搜索"));
  assert.equal(found.length, 1);
  assert.equal(found[0].key, "a-A01");
  assert.deepEqual(
    found[0].tasks.map((x) => x.id),
    ["A01-01"],
  );
  assert.equal(found[0].counted?.length, 2);
  // No words: the groups as they were.
  assert.equal(searchGroups(groups, []), groups);
});

test("a collapsed group with a match opens while searching, and is collapsed again after", () => {
  const collapsed = ["a-A01", "a-A03"];
  const shown = [{ key: "a-A01" }, { key: "a-A02" }];
  assert.deepEqual(collapsedWhileSearching(collapsed, shown, ["x"]), ["a-A03"]);
  assert.deepEqual(collapsedWhileSearching(collapsed, shown, []), collapsed);
});
