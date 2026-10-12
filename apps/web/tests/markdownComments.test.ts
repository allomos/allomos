// Tests for hiding Markdown comments (Task-A03-11). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { stripComments } from "../src/markdownComments.ts";

test("text without comments is unchanged", () => {
  const text = "# Title\n\n- one\n- two\n";
  assert.equal(stripComments(text), text);
});

test("a comment at the end of a line goes, the line stays", () => {
  assert.equal(
    stripComments("- **Checkout:** <!-- worktree, branch, last commit -->\n- **Running:** none"),
    "- **Checkout:**\n- **Running:** none",
  );
});

test("a line holding only a comment goes, so the list stays one list", () => {
  assert.equal(stripComments("- one\n<!-- a hint -->\n- two"), "- one\n- two");
});

test("a comment over several lines goes", () => {
  assert.equal(stripComments("before\n<!-- line one\nline two -->\nafter"), "before\nafter");
  assert.equal(stripComments("keep <!-- one\ntwo --> this"), "keep  this");
});

test("two comments on one line go, the text between stays", () => {
  assert.equal(stripComments("a <!-- x --> b <!-- y --> c"), "a  b  c");
});

test("a comment in a code block or inline code stays", () => {
  const block = "```\n<!-- shown -->\n```";
  assert.equal(stripComments(block), block);
  assert.equal(stripComments("Write `<!-- hint -->` to hide a hint."), "Write `<!-- hint -->` to hide a hint.");
  assert.equal(stripComments("```\n<!-- shown -->\n```\nafter <!-- hidden -->"), "```\n<!-- shown -->\n```\nafter");
});

test("an unclosed comment is shown as written", () => {
  assert.equal(stripComments("text <!-- never closed\nmore"), "text <!-- never closed\nmore");
});

test("the handoff template shows its headings without the hints", () => {
  const template = [
    "# Handoff",
    "",
    "## Restart details",
    "",
    "- **Checkout:** <!-- worktree, branch, last commit, pushed or not -->",
    "- **Uncommitted changes:** <!-- file, and why; or none -->",
    "",
    "<!-- Keep each line short. Evidence goes on the task's card, not here. -->",
    "",
  ].join("\n");
  const shown = stripComments(template);
  assert.ok(!shown.includes("<!--") && !shown.includes("-->"));
  assert.ok(shown.includes("- **Checkout:**\n- **Uncommitted changes:**"));
});
