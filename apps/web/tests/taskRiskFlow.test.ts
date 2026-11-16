// Tests for a task's risk in the list and pane (Task-A01-garden385, decision 0027).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { riskClass, riskOf, riskText } from "../src/taskRiskFlow.ts";
import { withDecisions } from "../src/stopFlow.ts";
import en from "../src/i18n/en.json" with { type: "json" };
import zh from "../src/i18n/zh-CN.json" with { type: "json" };
import ja from "../src/i18n/ja.json" with { type: "json" };
import type { Opened, Task } from "../src/types.ts";

const translator = (lang: "en" | "zh-CN" | "ja") => (key: string) => ({ en, "zh-CN": zh, ja })[lang][key as keyof typeof en] ?? key;

const task = (risk?: string | null) => ({ id: "A01-01", title: "t", state: "done", risk }) as unknown as Task;

test("a task shows the risk recorded for it; one with none reads Not classified", () => {
  const t = translator("en");
  assert.equal(riskOf(task("high")), "high");
  assert.equal(riskOf(task()), null);
  assert.equal(riskOf(task(null)), null);
  assert.equal(riskOf(task("huge")), null, "not a level: never guessed");
  assert.deepEqual(["high", "medium", "low"].map((r) => riskText(task(r), t)), ["High", "Medium", "Low"]);
  assert.equal(riskText(task(), t), "Not classified");
  assert.equal(riskText(task(), translator("zh-CN")), "未分级");
  assert.equal(riskText(task("high"), translator("zh-CN")), "高");
  assert.equal(riskText(task(), translator("ja")), "未分類");
  assert.equal(riskClass(task("high")), "risk risk-high");
  assert.equal(riskClass(task()), "risk risk-none");
});

test("the risk from the history goes on its task when the taskbag is opened", () => {
  const opened = {
    root: "/r",
    readme_added: false,
    loaded: {
      taskbag: {
        project: { format: "allomos/7", id: "P", name: "P" },
        areas: [],
        iterations: [],
        tasks: [task(), { ...task(), id: "A01-02" }, { ...task(), id: "A01-03" }],
      },
      issues: [],
    },
    // A01-03 is classified, but with no risk: still not classified here.
    classifications: { "A01-01": { risk: "medium", change_type: "normal" }, "A01-03": { change_type: "standard" } },
  } as unknown as Opened;
  const tasks = withDecisions(opened).loaded.taskbag!.tasks;
  assert.deepEqual(tasks.map(riskOf), ["medium", null, null]);
  // An older desktop app sends no classifications: every task reads Not classified.
  const old = withDecisions({ ...opened, classifications: undefined }).loaded.taskbag!.tasks;
  assert.deepEqual(old.map(riskOf), [null, null, null]);
});
