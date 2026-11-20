// Clickable task IDs in the monitor panel (Task-A03-scale435).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { canOpenMonitorTask, splitTaskIdPhrase } from "../src/monitorFlow.ts";

test("canOpenMonitorTask is true only when the task still exists in the project", () => {
  const tasks = [{ id: "A03-scale435" }, { id: "A03-jazz112" }];
  assert.equal(canOpenMonitorTask("A03-scale435", tasks), true);
  assert.equal(canOpenMonitorTask("A03-jazz112", tasks), true);
  assert.equal(canOpenMonitorTask("A03-missing", tasks), false);
  assert.equal(canOpenMonitorTask("A03-scale435", []), false);
});

test("splitTaskIdPhrase splits translated monitor.let_go and monitor.task_lost around the placeholder in every locale", () => {
  const placeholder = "\0ALLOMOS_TASK_IDS\0";
  for (const locale of ["en", "zh-CN", "ja"]) {
    const dict = JSON.parse(readFileSync(new URL(`../src/i18n/${locale}.json`, import.meta.url), "utf8")) as Record<string, string>;
    const letGo = dict["monitor.let_go"].replace("{id}", placeholder).replace("{holder}", "other-session");
    const [lgBefore, lgAfter] = splitTaskIdPhrase(letGo, placeholder);
    assert.equal(`${lgBefore}A03-scale435${lgAfter}`, dict["monitor.let_go"].replace("{id}", "A03-scale435").replace("{holder}", "other-session"));

    const taskLost = dict["monitor.task_lost"].replace("{ids}", placeholder);
    const [tlBefore, tlAfter] = splitTaskIdPhrase(taskLost, placeholder);
    assert.equal(`${tlBefore}A03-scale435, A03-jazz112${tlAfter}`, dict["monitor.task_lost"].replace("{ids}", "A03-scale435, A03-jazz112"));
  }
});

test("splitTaskIdPhrase falls back cleanly when the placeholder is not present", () => {
  assert.deepEqual(splitTaskIdPhrase("no placeholder here", "__IDS__"), ["no placeholder here", ""]);
});
