// Tests for labels in stopFlow.ts (decision 0020, Task-A01-petal376): the built-in labels, To check, which "by
// human" label Checked adds, and the label table as the app lists it.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  AI_DECISIONS,
  BUILT_IN_NAMES,
  DEPLOYED_BY_AGENT,
  DEPLOYED_BY_HUMAN,
  LOCALLY_BY_AGENT,
  LOCALLY_BY_HUMAN,
  checkedLabel,
  chipLabels,
  columnText,
  hiddenText,
  humanChecked,
  isBuiltIn,
  labelMeaning,
  labelNames,
  labelTable,
  labelText,
  presetTicks,
  stopText,
  toCheck,
  untickedNames,
  allTicked,
  tickCell,
} from "../src/stopFlow.ts";
import { changeLine, valueText } from "../src/historyFlow.ts";
import type { Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };
import zh from "../src/i18n/zh-CN.json" with { type: "json" };
import ja from "../src/i18n/ja.json" with { type: "json" };

const words = (file: Record<string, string>) => (key: string, params?: Record<string, string | number>) =>
  (file[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));
const t = words(en as Record<string, string>);

const task = (over: Partial<Task>): Task =>
  ({
    id: "A01-01",
    title: "x",
    area: "A01",
    state: "done",
    holder: null,
    waiting_on: null,
    next_dependency: null,
    iteration: null,
    handoff_note: null,
    without_human_check: null,
    human_check_steps: null,
    description: "",
    ...over,
  }) as Task;

test("Checked adds 'deployed and verified by human' after 'deployed and verified by agent', else 'locally verified by human' (as alm checked)", () => {
  assert.equal(checkedLabel(task({ labels: [DEPLOYED_BY_AGENT] })), DEPLOYED_BY_HUMAN);
  assert.equal(checkedLabel(task({ labels: [LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT] })), DEPLOYED_BY_HUMAN);
  assert.equal(checkedLabel(task({ labels: [LOCALLY_BY_AGENT] })), LOCALLY_BY_HUMAN);
  assert.equal(checkedLabel(task({ labels: [] })), LOCALLY_BY_HUMAN);
  assert.equal(checkedLabel(task({ labels: ["shipped", AI_DECISIONS] })), LOCALLY_BY_HUMAN);
  // After it, the task is no longer to check.
  for (const labels of [[DEPLOYED_BY_AGENT], [LOCALLY_BY_AGENT], []]) {
    const before = task({ labels });
    assert.equal(toCheck(before), true);
    const after = task({ labels: [...labels, checkedLabel(before)] });
    assert.equal(humanChecked(after), true);
    assert.equal(toCheck(after), false);
  }
});

test("the label table lists the built-in labels first, in their own order, then the project's own, then labels only tasks carry", () => {
  const project = { labels: [{ name: "shipped", meaning: "Out" }, { name: AI_DECISIONS, built_in: true }, { name: LOCALLY_BY_AGENT, meaning: "Ours", built_in: true }] };
  const tasks = [task({ labels: ["shipped", "found on a task"] }), task({ labels: [LOCALLY_BY_HUMAN] })];
  assert.deepEqual(labelNames(project, tasks), [...BUILT_IN_NAMES, "shipped", "found on a task"]);
  const table = labelTable(project, tasks);
  assert.equal(table[0].meaning, "Ours", "a meaning the project wrote stays");
  assert.deepEqual(
    table.map((e) => !!e.built_in),
    [true, true, true, true, true, false, false],
  );
  // A taskbag read before `allomos/7` may still send a ladder of names.
  assert.deepEqual(labelNames({ labels: ["implemented"] as never }), [...BUILT_IN_NAMES, "implemented"]);
  assert.deepEqual(labelNames(undefined), BUILT_IN_NAMES);
  assert.equal(isBuiltIn(DEPLOYED_BY_HUMAN), true);
  assert.equal(isBuiltIn("human verified"), false);
});

test("the built-in labels in en, zh-CN and ja, as decision 0020's table names them; custom labels as written", () => {
  const expected: Record<string, [string, string, string]> = {
    [LOCALLY_BY_AGENT]: ["locally verified by agent", "智能体已在本地验证", "エージェントがローカルで検証済み"],
    [DEPLOYED_BY_AGENT]: ["deployed and verified by agent", "智能体已部署并验证", "エージェントがデプロイして検証済み"],
    [LOCALLY_BY_HUMAN]: ["locally verified by human", "人已在本地验证", "人がローカルで検証済み"],
    [DEPLOYED_BY_HUMAN]: ["deployed and verified by human", "人已在部署后验证", "人がデプロイ後に検証済み"],
    [AI_DECISIONS]: ["AI decisions to review", "待审阅 AI 决定", "レビュー待ちの AI の決定"],
  };
  const langs = [t, words(zh as Record<string, string>), words(ja as Record<string, string>)];
  for (const name of BUILT_IN_NAMES) {
    langs.forEach((tr, i) => assert.equal(labelText(name, tr), expected[name][i], `${name} (${i})`));
    for (const tr of langs) assert.ok(labelMeaning({ name }, tr).length > 0, `${name} has a meaning`);
  }
  assert.equal(labelText("shipped", t), "shipped");
  assert.equal(labelMeaning({ name: "shipped", meaning: "Out" }, t), "Out");
  assert.equal(labelMeaning({ name: "shipped" }, t), "");
});

test("the chips show every label but 'AI decisions to review', which has its own badge", () => {
  assert.deepEqual(chipLabels(task({ labels: [LOCALLY_BY_AGENT, AI_DECISIONS, "shipped"] })), [LOCALLY_BY_AGENT, "shipped"]);
  assert.deepEqual(chipLabels(task({})), []);
});

test("states and built-in labels follow taskT while surrounding UI words stay in t (decision 0025 #4)", () => {
  const zhT = words(zh as Record<string, string>);
  const jaT = words(ja as Record<string, string>);
  assert.equal(columnText(LOCALLY_BY_AGENT, t, zhT), "智能体已在本地验证");
  assert.equal(columnText(LOCALLY_BY_AGENT, t, jaT), "エージェントがローカルで検証済み");
  assert.equal(presetTicks("ready", t, zhT), "就绪");
  assert.equal(hiddenText({ count: 1, states: [{ state: "ready", count: 1 }] }, t, zhT), "1 more hidden by the filter: 1 就绪");
  const filtered = tickCell(allTicked(BUILT_IN_NAMES), "done", LOCALLY_BY_AGENT, "no", false);
  assert.deepEqual(untickedNames(filtered, t, zhT), ["已完成 · 智能体已在本地验证: no"]);
  const blocked = task({ id: "A01-02", state: "blocked", blocked_by: ["A01-01"] });
  const blocker = task({ id: "A01-01", state: "in_progress" });
  assert.equal(stopText(blocked, [blocked, blocker], t, zhT), "Blocked by A01-01 (进行中)");
  assert.equal(valueText("state", "ready", t, zhT), "就绪");
  assert.equal(valueText("labels", [LOCALLY_BY_AGENT, "shipped"], t, zhT), "智能体已在本地验证, shipped");
  assert.equal(changeLine({ field: "state", from: "ready", to: "done" }, t, zhT), "State: 就绪 → 已完成");
});
