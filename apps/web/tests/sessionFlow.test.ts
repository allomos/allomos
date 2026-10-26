// Tests for the task pane's responsible person, agent, check and session rows (Task-A03-37).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { check, paneAgent, session, sessionPlace } from "../src/sessionFlow.ts";
import type { Task, TaskHistory } from "../src/types.ts";

const task = (over: Partial<Task> = {}): Task => ({ id: "A01-01", title: "t", area: "A01", state: "in_progress", label: null, ...over }) as Task;
const history = (over: Partial<TaskHistory> = {}): TaskHistory => ({ since: "2026-10-01T00:00:00Z", events: [], seen: [], ...over });
const now = new Date("2026-10-22T10:00:00Z");

test("the session is active while its holder changed the taskbag in the last 30 minutes", () => {
  const h = (at: string) => history({ seen: [{ by: "claude/x", at, agent: "claude" }] });
  const held = task({ holder: "claude/x" });
  assert.deepEqual(session(held, h("2026-10-22T09:31:00Z"), now), { kind: "active", at: "2026-10-22T09:31:00Z" });
  assert.deepEqual(session(held, h("2026-10-22T09:30:00Z"), now), { kind: "active", at: "2026-10-22T09:30:00Z" });
  assert.deepEqual(session(held, h("2026-10-22T09:29:00Z"), now), { kind: "last_seen", at: "2026-10-22T09:29:00Z" });
  // A holder with no recorded change (copied from an old board) is not shown as working.
  assert.deepEqual(session(held, history(), now), { kind: "unknown" });
  assert.deepEqual(session(held, null, now), { kind: "unknown" });
  // Nobody holds it: no session row.
  assert.equal(session(task(), h("2026-10-22T09:59:00Z"), now), null);
});

test("a recorded session says where it runs and is kept as the last session after release", () => {
  // Task-A03-hive049: the app session ID wins; a session ID alone resumes in a terminal.
  assert.equal(sessionPlace(task({ session: { id: "s1", kind: "desktop", app_session_id: "local_1" } })), "app");
  assert.equal(sessionPlace(task({ session: { id: "s1" } })), "terminal");
  assert.equal(sessionPlace(task({ session: { agent: "claude-code" } })), null);
  assert.equal(sessionPlace(task()), null);
  const h = history({ seen: [{ by: "claude/x", at: "2026-10-22T08:00:00Z", agent: "claude" }] });
  const released = task({ state: "ready", holder: null, session: { by: "claude/x", id: "s1" } });
  assert.deepEqual(session(released, h, now), { kind: "last_session", at: "2026-10-22T08:00:00Z" });
  assert.deepEqual(session(released, null, now), { kind: "last_session", at: null });
  // Held: the row reads as before (active, last seen).
  assert.deepEqual(session({ ...released, holder: "claude/x" }, h, now), { kind: "last_seen", at: "2026-10-22T08:00:00Z" });
  // Nothing to reopen and nobody holds it: no row.
  assert.equal(session(task({ holder: null, session: { agent: "claude-code" } }), h, now), null);
});

test("the agent is the holder, else the last session that changed the task through alm", () => {
  const h = history({
    events: [
      { at: "2026-10-21T10:00:00Z", via: "app", command: "edit", kind: "task", changes: [] },
      { at: "2026-10-20T10:00:00Z", by: "codex/y", via: "alm", command: "done", kind: "task", changes: [] },
    ],
    seen: [
      { by: "claude/x", at: "2026-10-22T09:00:00Z", agent: "claude" },
      { by: "codex/y", at: "2026-10-20T10:00:00Z", agent: "codex" },
    ],
  });
  assert.deepEqual(paneAgent(task({ holder: "claude/x" }), h), { by: "claude/x", agent: "claude", holding: true });
  assert.deepEqual(paneAgent(task({ state: "done" }), h), { by: "codex/y", agent: "codex", holding: false });
  assert.deepEqual(paneAgent(task({ holder: "old-board" }), h), { by: "old-board", agent: null, holding: true });
  assert.equal(paneAgent(task(), history()), null);
});

test("the check: checked at the top label, to check when finished without it, otherwise not yet", () => {
  const labels = ["drafted", "implemented", "human verified"];
  assert.equal(check(task({ state: "done", label: "human verified" }), labels), "checked");
  assert.equal(check(task({ state: "done", label: "implemented", without_human_check: "needs a Mac" }), labels), "to_check");
  assert.equal(check(task({ state: "done", label: "implemented" }), labels), "not_yet");
  assert.equal(check(task({ label: "human verified" }), labels), "not_yet");
});
