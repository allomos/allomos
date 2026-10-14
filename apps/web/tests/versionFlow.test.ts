// Tests for the update notice (Task-A01-25). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { outdatedFiles, updateNotice } from "../src/versionFlow.ts";
import type { VersionCheck } from "../src/types.ts";

const check = (format: VersionCheck["format"], sections: VersionCheck["sections"]): VersionCheck => ({
  format_current: "allomos/1",
  section_current: 1,
  format,
  sections,
});

test("nothing is shown before an agent is connected", () => {
  assert.equal(updateNotice(null), null);
  assert.deepEqual(outdatedFiles(null), []);
});

test("an up-to-date project shows nothing", () => {
  assert.equal(updateNotice(check({ state: "current" }, [{ file: "AGENTS.md", version: 1 }])), null);
});

test("a section without a version, or an older one, is out of date", () => {
  const c = check({ state: "current" }, [
    { file: "AGENTS.md", version: null },
    { file: "CLAUDE.md", version: 0 },
  ]);
  assert.deepEqual(updateNotice(c), { kind: "section", files: "AGENTS.md, CLAUDE.md" });
});

test("an older taskbag comes first", () => {
  const c = check({ state: "older", found: "allomos/1" }, [{ file: "AGENTS.md", version: null }]);
  assert.deepEqual(updateNotice(c), { kind: "taskbag" });
});

test("a newer taskbag is left to the problem list", () => {
  assert.equal(updateNotice(check({ state: "newer", found: "allomos/2" }, [{ file: "AGENTS.md", version: 1 }])), null);
});

test("a taskbag too old to migrate is left to the problem list (Task-A01-32)", () => {
  const c = check({ state: "too_old", found: "allomos/2", oldest: "allomos/3" }, [{ file: "AGENTS.md", version: 1 }]);
  assert.equal(updateNotice(c), null);
});
