import { test } from "node:test";
import assert from "node:assert/strict";
import { atDefaults, changeOf, checkMinutes, limitTooShort, type HeartbeatSettings } from "../src/heartbeatSettingsFlow.ts";

const defaults: HeartbeatSettings = {
  interval_minutes: null,
  lost_after_minutes: null,
  applied_interval_minutes: 10,
  applied_lost_after_minutes: 30,
  default_interval_minutes: 10,
  default_lost_after_minutes: 30,
};

test("whole minutes of at least 1 pass; anything else is refused with heartbeat_minutes_invalid", () => {
  assert.deepEqual(checkMinutes("Interval", "15"), { ok: true, minutes: 15 });
  assert.deepEqual(checkMinutes("Interval", " 1 "), { ok: true, minutes: 1 });
  for (const bad of ["0", "", "1.5", "-3", "abc", "10m", "0.5"]) {
    assert.deepEqual(checkMinutes("Interval", bad), {
      ok: false,
      problem: { code: "heartbeat_minutes_invalid", params: { field: "Interval", value: bad } },
    });
  }
});

test("only a changed number is sent; the first bad one is named", () => {
  const f = (text: string, field = "A") => ({ field, text });
  assert.deepEqual(changeOf(defaults, f("10"), f("30")), { ok: true, change: {} });
  assert.deepEqual(changeOf(defaults, f("5"), f("30")), { ok: true, change: { interval: "5" } });
  assert.deepEqual(changeOf(defaults, f("10"), f("45")), { ok: true, change: { lostAfter: "45" } });
  const bad = changeOf(defaults, f("10", "Interval"), f("0", "Lost after"));
  assert.equal(bad.ok, false);
  assert.equal(!bad.ok && bad.problem.params.field, "Lost after");
});

test("Restore Defaults has nothing to do only when nothing is written", () => {
  assert.equal(atDefaults(defaults), true);
  assert.equal(atDefaults({ ...defaults, interval_minutes: 10 }), false);
});

test("a limit not longer than the interval is said", () => {
  assert.equal(limitTooShort(10, 30), false);
  assert.equal(limitTooShort(10, 10), true);
  assert.equal(limitTooShort(20, 15), true);
});
