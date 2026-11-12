// Tests for tagline.ts (Task-A01-aunt631): the About window's slogan in phrases, in every language.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { taglinePhrases } from "../src/tagline.ts";

const tagline = (lang: string) =>
  JSON.parse(readFileSync(new URL(`../src/i18n/${lang}.json`, import.meta.url), "utf8"))["about.tagline"] as string;

test("the slogan splits after its first comma or colon, in every language", () => {
  assert.deepEqual(taglinePhrases(tagline("zh-CN")), ["AI 时代的开发，", "换谁来做都接得住。"]);
  assert.deepEqual(taglinePhrases(tagline("ja")), ["AI 時代の開発、", "誰が引き継いでも取りこぼさない。"]);
  assert.deepEqual(taglinePhrases(tagline("en")), ["Development in the AI era:", "hand it to anyone. Nothing drops."]);
});

test("a slogan with no comma or colon stays one phrase", () => {
  assert.deepEqual(taglinePhrases("Allomos"), ["Allomos"]);
});
