// Tests for a held task's last heartbeat and its Lost tag in the app (Task-A01-kernel761): the
// same rule as `alm next` (crates/core/src/heartbeat.rs `lost`).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { LOST_AFTER_MINUTES, agoText, clockText, heartbeatFact, heartbeatText, lostAfterMinutes } from "../src/heartbeatFlow.ts";
import type { Project, Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };
import zh from "../src/i18n/zh-CN.json" with { type: "json" };

const words = (lang: Record<string, string>) => (key: string, params?: Record<string, string | number>) =>
  (lang[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));
const t = words(en as Record<string, string>);
const tz = words(zh as Record<string, string>);

const held: Task = {
  id: "A01-01",
  title: "one",
  area: "A01",
  state: "in_progress",
  holder: "s1",
  waiting_on: null,
  next_dependency: null,
  iteration: null,
  handoff_note: null,
  without_human_check: null,
  human_check_steps: null,
  description: "",
  heartbeat: { at: "2026-11-09T10:00:00Z", by: "s1" },
};
const at = (stamp: string) => new Date(stamp);

test("the project's limit: its own number, else 30 minutes", () => {
  assert.equal(lostAfterMinutes(null), LOST_AFTER_MINUTES);
  assert.equal(lostAfterMinutes({ heartbeat: {} } as Project), 30);
  assert.equal(lostAfterMinutes({ heartbeat: { lost_after_minutes: 120 } } as Project), 120);
  assert.equal(lostAfterMinutes({ heartbeat: { lost_after_minutes: 0 } } as Project), 30);
});

test("a fresh heartbeat: its age, not lost", () => {
  const f = heartbeatFact(held, 30, at("2026-11-09T10:12:30Z"))!;
  assert.equal(f.minutes, 12);
  assert.equal(f.lost, false);
  assert.equal(f.busyUntil, null);
});

test("past the limit: lost; exactly at it: not yet", () => {
  assert.equal(heartbeatFact(held, 30, at("2026-11-09T10:30:00Z"))!.lost, false);
  assert.equal(heartbeatFact(held, 30, at("2026-11-09T10:31:00Z"))!.lost, true);
  assert.equal(heartbeatFact(held, 120, at("2026-11-09T11:00:00Z"))!.lost, false);
});

test("a running busy window keeps it from being lost; the limit then counts from its end", () => {
  const busy = { ...held, heartbeat: { at: "2026-11-09T10:00:00Z", by: "s1", busy_until: "2026-11-09T12:00:00Z" } };
  const during = heartbeatFact(busy, 30, at("2026-11-09T11:00:00Z"))!;
  assert.equal(during.lost, false);
  assert.equal(during.busyUntil, "2026-11-09T12:00:00Z");
  const after = heartbeatFact(busy, 30, at("2026-11-09T12:20:00Z"))!;
  assert.equal(after.busyUntil, null);
  assert.equal(after.lost, false);
  assert.equal(heartbeatFact(busy, 30, at("2026-11-09T12:31:00Z"))!.lost, true);
});

test("only the holder's own heartbeat counts; with none, the task's last change", () => {
  const other = { ...held, heartbeat: { at: "2026-11-09T10:00:00Z", by: "s0" } };
  const f = heartbeatFact(other, 30, at("2026-11-09T10:10:00Z"), "2026-11-09T09:00:00Z")!;
  assert.equal(f.at, null);
  assert.equal(f.lost, true);
  assert.equal(heartbeatFact({ ...held, heartbeat: null }, 30, at("2026-11-09T10:10:00Z"), "2026-11-09T10:00:00Z")!.lost, false);
  // Nothing to date the silence from: not marked.
  assert.equal(heartbeatFact({ ...held, heartbeat: null }, 30, at("2026-11-09T12:00:00Z"))!.lost, false);
});

test("a task nobody holds, or not in progress, has no heartbeat fact", () => {
  assert.equal(heartbeatFact({ ...held, holder: null }, 30, at("2026-11-09T12:00:00Z")), null);
  assert.equal(heartbeatFact({ ...held, state: "waiting" }, 30, at("2026-11-09T12:00:00Z")), null);
});

test("ages in the largest whole unit", () => {
  assert.equal(agoText(0, t), "just now");
  assert.equal(agoText(12, t), "12 min ago");
  assert.equal(agoText(125, t), "2 h ago");
  assert.equal(agoText(3 * 24 * 60, t), "3 d ago");
  assert.equal(agoText(12, tz), "12 分钟前");
});

test("the line: age and clock time, busy window, no record", () => {
  const now = at("2026-11-09T10:12:00Z");
  const local = clockText("2026-11-09T10:00:00Z", now);
  assert.match(local, /^(\d{4}-\d\d-\d\d )?\d\d:\d\d$/);
  assert.equal(heartbeatText(heartbeatFact(held, 30, now)!, now, t), `12 min ago (${local})`);
  assert.equal(heartbeatText(heartbeatFact(held, 30, now)!, now, tz), `12 分钟前（${local}）`);
  const busy = { ...held, heartbeat: { at: "2026-11-09T10:00:00Z", by: "s1", busy_until: "2026-11-09T12:00:00Z" } };
  assert.equal(heartbeatText(heartbeatFact(busy, 30, now)!, now, tz), `12 分钟前（${local}） · 忙碌到 ${clockText("2026-11-09T12:00:00Z", now)}`);
  assert.equal(heartbeatText(heartbeatFact({ ...held, heartbeat: null }, 30, now)!, now, tz), "没有记录");
});

test("the clock time alone on the same local day, else with the date", () => {
  const now = new Date(2026, 10, 9, 15, 0);
  assert.equal(clockText(new Date(2026, 10, 9, 14, 2).toISOString(), now), "14:02");
  assert.equal(clockText(new Date(2026, 10, 8, 14, 2).toISOString(), now), "2026-11-08 14:02");
});
