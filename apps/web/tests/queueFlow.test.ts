// Tests for the For You window's list (Task-A01-oak609, decision 0028).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { askText, askedNow, forYouTitle, nextChoice, riskWord, splitQueue, todoText, waitedText, type QueueRow } from "../src/queueFlow.ts";
import en from "../src/i18n/en.json" with { type: "json" };
import zh from "../src/i18n/zh-CN.json" with { type: "json" };
import type { ConfirmRequest } from "../src/types.ts";

const tr = (words: Record<string, string>) => (key: string, params: Record<string, string | number> = {}) =>
  (words[key] ?? key).replace(/\{(\w+)\}/g, (_, p: string) => String(params[p]));
const t = tr(en as Record<string, string>);
const tz = tr(zh as Record<string, string>);

const row = (task: string, extra: Partial<QueueRow> = {}): QueueRow => ({
  task,
  todo: ["check"],
  decisions: 0,
  agent_waiting: false,
  risk: null,
  waiting_since: null,
  blocks: 0,
  ...extra,
});

test("a row says every thing to do, in one line", () => {
  assert.equal(todoText(row("A", { todo: ["check", "review"], decisions: 2 }), t), "check + review 2 AI decisions");
  assert.equal(todoText(row("A", { todo: ["check", "review"], decisions: 2 }), tz), "检查 + 审 2 条 AI 决定");
  assert.equal(todoText(row("A", { todo: ["review"], decisions: 0 }), t), "review AI decisions");
  assert.equal(askText(row("A", { todo: ["answer"] }), tz), "要你做：回答。");
});

test("tasks not classified fold into one line; an agent waiting never folds", () => {
  const rows = [
    row("W", { agent_waiting: true, todo: ["answer"] }),
    row("H", { risk: "high" }),
    row("M", { risk: "medium" }),
    row("U1"),
    row("U2", { risk: "something else" }),
  ];
  const { shown, unclassified } = splitQueue(rows);
  assert.deepEqual(shown.map((r) => r.task), ["W", "H", "M"]);
  assert.deepEqual(unclassified.map((r) => r.task), ["U1", "U2"]);
});

test("the risk reads as a word; one not in the list reads Not classified", () => {
  assert.equal(riskWord("high", tz), "风险高");
  assert.equal(riskWord(null, t), "Not classified");
  assert.equal(riskWord("odd", t), "Not classified");
});

test("how long it waited, in minutes, hours or days", () => {
  const now = new Date("2026-11-17T12:00:00Z");
  assert.equal(waitedText("2026-11-17T11:50:00Z", now, t), "waiting 10 min");
  assert.equal(waitedText("2026-11-16T12:00:00Z", now, t), "waiting 24 h");
  assert.equal(waitedText("2026-11-10T12:00:00Z", now, tz), "等了 7 天");
  assert.equal(waitedText(null, now, t), null);
});

test("a handled row leaves the list and the one below it is chosen", () => {
  const before = [row("A"), row("B"), row("C")];
  assert.equal(nextChoice(before, [row("A"), row("C")], "B"), "C");
  assert.equal(nextChoice(before, [row("A"), row("B")], "C"), "B");
  assert.equal(nextChoice(before, [row("B"), row("A"), row("C")], "A"), "A");
  assert.equal(nextChoice([], [row("A")], null), "A");
  assert.equal(nextChoice(before, [], "A"), null);
});

test("open alm confirm questions that name a task count as plans to approve", () => {
  const asked = askedNow([
    { id: "1", title: "x", plan: "", from: null, asked: 1763380800, until: 0, task: "A01-01" },
    { id: "2", title: "y", plan: "", from: null, asked: 1763380800, until: 0, task: null },
  ] as ConfirmRequest[]);
  assert.deepEqual(asked, [{ task: "A01-01", at: "2025-11-17T12:00:00Z" }]);
});

test("the window is named after its project", () => {
  assert.equal(forYouTitle("Allomos", tz), "待你处理 · Allomos");
  assert.equal(forYouTitle(null, t), "For You");
});
