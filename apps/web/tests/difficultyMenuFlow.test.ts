// Tests for the task list's difficulty menu (Task-A01-pewter675): the levels 1 to 5, the task's
// own ticked, and the same change as the task pane's difficulty field.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { levelItems, levelMenuEdit, levelText, difficultyText } from "../src/difficultyFlow.ts";
import { toEdit, startDraft } from "../src/editFlow.ts";
import type { Task } from "../src/types.ts";

const task = (difficulty?: number | null) => ({ id: "A01-01", title: "t", state: "ready", difficulty }) as unknown as Task;

test("the menu lists the levels 1 to 5, the task's own ticked", () => {
  const items = levelItems(task(4));
  assert.deepEqual(items.map((i) => i.level), [1, 2, 3, 4, 5]);
  assert.deepEqual(items.filter((i) => i.current).map((i) => i.level), [4]);
});

test("a task without a difficulty ticks none: its 3 is only the default", () => {
  assert.equal(levelItems(task()).some((i) => i.current), false);
  assert.equal(levelItems(task(null)).some((i) => i.current), false);
  assert.equal(levelItems(task(9)).some((i) => i.current), false);
});

test("choosing a level sends the pane's change; nothing for the task's own level", () => {
  assert.deepEqual(levelMenuEdit(task(4), 2), { field: "difficulty", difficulty: 2 });
  assert.deepEqual(levelMenuEdit(task(4), 2), toEdit("difficulty", { ...startDraft(task(4), "difficulty"), value: "2" }));
  assert.equal(levelMenuEdit(task(4), 4), null);
  // Choosing 3 for a task without a difficulty writes it.
  assert.deepEqual(levelMenuEdit(task(), 3), { field: "difficulty", difficulty: 3 });
  assert.equal(levelMenuEdit(task(2), 0), null);
  assert.equal(levelMenuEdit(task(2), 6), null);
  assert.equal(levelMenuEdit(task(2), 2.5), null);
});

test("each level reads with its name in every language, as the row shows it", () => {
  for (const lang of ["zh-CN", "ja", "en"]) {
    const words = JSON.parse(readFileSync(new URL(`../src/i18n/${lang}.json`, import.meta.url), "utf8")) as Record<string, string>;
    const t = (key: string, params?: Record<string, string | number>) =>
      (words[key] ?? "").replace(/\{(\w+)\}/g, (_, k: string) => String(params?.[k] ?? ""));
    const names = levelItems(task(4)).map((i) => levelText(i.level, t));
    assert.equal(new Set(names).size, 5, lang);
    names.forEach((n, i) => assert.ok(n.startsWith(`${i + 1} · `), `${lang}: ${n}`));
    // The ticked item is the text the row shows.
    assert.equal(names[3], difficultyText(task(4), t));
    for (const key of ["difficulty_menu.label", "difficulty_menu.button_title", "difficulty_menu.button_aria"]) assert.ok(words[key], `${lang}: ${key}`);
    assert.ok(t("difficulty_menu.button_aria", { level: names[3] }).includes(names[3]), lang);
  }
});
