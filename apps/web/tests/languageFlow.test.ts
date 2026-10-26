// Tests for the searchable language menu's list (Task-A05-board831). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { allLanguages, fold, languageOption, searchLanguages } from "../src/languageFlow.ts";

test("the interface languages come first, then the rest A to Z by English name", () => {
  const all = allLanguages();
  assert.deepEqual(
    all.slice(0, 3).map((o) => o.code),
    ["en", "zh-CN", "ja"],
  );
  const rest = all.slice(3).map((o) => o.english);
  assert.deepEqual(rest, [...rest].sort((a, b) => a.localeCompare(b, "en")));
  assert.ok(all.length > 150, `${all.length} languages`);
  // Each code once.
  assert.equal(new Set(all.map((o) => o.code)).size, all.length);
});

test("each language has its own name and its English name", () => {
  assert.deepEqual(languageOption("vi"), { code: "vi", own: "Tiếng Việt", english: "Vietnamese" });
  assert.equal(languageOption("de").own, "Deutsch");
  assert.equal(languageOption("es").own, "Español");
});

test("searching finds a language by its own or English name, without accents, or by its code", () => {
  const all = allLanguages();
  for (const q of ["viet", "Tiếng", "tieng", "Vietnamese", "VI"]) {
    assert.ok(
      searchLanguages(all, q).some((o) => o.code === "vi"),
      q,
    );
  }
  assert.deepEqual(
    searchLanguages(all, "pt-BR").map((o) => o.code),
    ["pt-BR"],
  );
  assert.deepEqual(searchLanguages(all, "zzzz"), []);
  assert.equal(searchLanguages(all, "  ").length, all.length);
  assert.equal(fold("Tiếng Việt"), "tieng viet");
});
