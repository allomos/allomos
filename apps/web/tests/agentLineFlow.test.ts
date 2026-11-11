// Tests for when the agent line checks again (Task-A03-spade155): at once, every 30 minutes,
// and when the refresh button is pressed. Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { CHECK_MS, startChecks, type Timer } from "../src/agentLineFlow.ts";

// A clock that moves only when the test says.
function fakeTimer() {
  let now = 0;
  let next = 1;
  const timers = new Map<number, { run: () => void; ms: number; due: number }>();
  const timer: Timer = {
    setInterval(run, ms) {
      const id = next++;
      timers.set(id, { run, ms, due: now + ms });
      return id;
    },
    clearInterval(id) {
      timers.delete(id);
    },
  };
  const advance = (ms: number) => {
    const end = now + ms;
    for (;;) {
      const due = [...timers.values()].filter((t) => t.due <= end).sort((a, b) => a.due - b.due)[0];
      if (!due) break;
      now = due.due;
      due.due += due.ms;
      due.run();
    }
    now = end;
  };
  return { timer, advance, running: () => timers.size };
}

test("the line checks every 30 minutes, not every few seconds", () => {
  assert.equal(CHECK_MS, 30 * 60 * 1000);
  const { timer, advance } = fakeTimer();
  let checks = 0;
  startChecks(() => checks++, timer);
  assert.equal(checks, 1, "checks when the project opens");
  advance(10 * 60 * 1000);
  assert.equal(checks, 1, "nothing in ten idle minutes");
  advance(20 * 60 * 1000);
  assert.equal(checks, 2);
  advance(60 * 60 * 1000);
  assert.equal(checks, 4);
});

test("the refresh button checks at once and starts the 30 minutes again", () => {
  const { timer, advance, running } = fakeTimer();
  let checks = 0;
  const line = startChecks(() => checks++, timer);
  advance(25 * 60 * 1000);
  line.refresh();
  assert.equal(checks, 2);
  assert.equal(running(), 1, "one timer, not two");
  advance(25 * 60 * 1000);
  assert.equal(checks, 2, "the wait started again at the refresh");
  advance(5 * 60 * 1000);
  assert.equal(checks, 3);
});

test("stopping ends the checks", () => {
  const { timer, advance, running } = fakeTimer();
  let checks = 0;
  const line = startChecks(() => checks++, timer);
  line.stop();
  assert.equal(running(), 0);
  advance(2 * CHECK_MS);
  assert.equal(checks, 1);
});
