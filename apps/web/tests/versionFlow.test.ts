// Tests for the update notice (Task-A01-25, Task-A01-45). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { movingTasks, outdatedFiles, updateNotice, updateTaskbagLines, upgradeFrom } from "../src/versionFlow.ts";
import type { VersionCheck } from "../src/types.ts";

const check = (
  format: VersionCheck["format"],
  sections: VersionCheck["sections"],
  move_in: VersionCheck["move_in"] = null,
): VersionCheck => ({
  format_current: "allomos/4",
  section_current: 1,
  format,
  move_in,
  sections,
});

test("nothing is shown before an agent is connected", () => {
  assert.equal(updateNotice(null), null);
  assert.deepEqual(outdatedFiles(null), []);
  assert.equal(movingTasks(null), false);
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
  assert.equal(movingTasks(updateNotice(c)), false);
});

test("an older Allomos's taskbag offers to move its tasks in, before the section (Task-A01-45)", () => {
  for (const found of ["allomos/1", "allomos/2", "allomos/3"]) {
    const c = check({ state: "older", found }, [{ file: "AGENTS.md", version: null }], {
      old: "/p/.allomos/taskbag-allomos-1",
      found,
      prepared: false,
    });
    assert.deepEqual(updateNotice(c), { kind: "move_in", found });
    assert.equal(movingTasks(updateNotice(c)), true);
  }
});

test("renamed aside, with no task moved in yet: the move is offered again", () => {
  const c = check({ state: "current" }, [{ file: "AGENTS.md", version: 1 }], {
    old: "/p/.allomos/taskbag-allomos-3",
    found: "allomos/3",
    prepared: true,
  });
  assert.deepEqual(updateNotice(c), { kind: "moving_in", found: "allomos/3", old: "/p/.allomos/taskbag-allomos-3" });
  assert.equal(movingTasks(updateNotice(c)), true);
});

test("a newer taskbag is left to the problem list", () => {
  assert.equal(updateNotice(check({ state: "newer", found: "allomos/9" }, [{ file: "AGENTS.md", version: 1 }])), null);
});

test("one Update the taskbag banner lists every reason, in order (Task-A03-train969)", () => {
  assert.deepEqual(updateTaskbagLines(null, 0, 0), []);
  assert.deepEqual(updateTaskbagLines({ kind: "section", files: "AGENTS.md" }, 0, 1), [
    { key: "update.notice_section", params: { files: "AGENTS.md" } },
    { key: "old_handoffs.notice", params: { count: 1 } },
  ]);
  assert.deepEqual(updateTaskbagLines({ kind: "move_in", found: "allomos/3" }, 2, 3), [
    { key: "move.notice_old", params: { found: "allomos/3" } },
    { key: "cards.notice", params: { count: 2 } },
    { key: "old_handoffs.notice", params: { count: 3 } },
  ]);
  assert.deepEqual(updateTaskbagLines(null, 2, 0), [{ key: "cards.notice", params: { count: 2 } }]);
});

test("an allomos/5 taskbag is offered the upgrade, its line first (Task-A05-fish286)", () => {
  const five = { ...check({ state: "current" }, [{ file: "AGENTS.md", version: 0 }]), format_current: "allomos/6", upgrade: "allomos/5" };
  assert.equal(upgradeFrom(five), "allomos/5");
  assert.equal(upgradeFrom(null), null);
  assert.equal(upgradeFrom({ ...five, upgrade: null }), null);
  // Moving an older taskbag's tasks in comes first.
  assert.equal(upgradeFrom({ ...five, move_in: { old: "/x", found: "allomos/3", prepared: false } }), null);
  const lines = updateTaskbagLines(updateNotice(five), 2, 0, upgradeFrom(five), five.format_current);
  assert.deepEqual(
    lines.map((l) => l.key),
    ["upgrade.notice", "update.notice_section", "cards.notice"],
  );
  assert.deepEqual(lines[0].params, { found: "allomos/5", current: "allomos/6" });
});
