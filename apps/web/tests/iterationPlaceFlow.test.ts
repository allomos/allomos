// Tests for each iteration's place number in the list and the New task form (Task-A00-turkey877).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { iterationPlaces, placedName } from "../src/iterationPlace.ts";
import type { Iteration } from "../src/types.ts";

const it = (id: string, order: number): Iteration => ({ id, name: id, order });

test("places are counted from 1 in plan order, whatever the IDs", () => {
  const places = iterationPlaces([it("I-tally", 6), it("I-dig", 1), it("I-knit", 4), it("I-haul", 5)]);
  assert.deepEqual([...places], [
    ["I-dig", 1],
    ["I-knit", 2],
    ["I-haul", 3],
    ["I-tally", 4],
  ]);
});

test("after a move the numbers follow the new order; equal orders go by ID", () => {
  const places = iterationPlaces([it("I-b", 1), it("I-a", 1), it("I-c", 0)]);
  assert.deepEqual([...places], [
    ["I-c", 1],
    ["I-a", 2],
    ["I-b", 3],
  ]);
});

test("the picker shows the place before the name", () => {
  assert.equal(placedName(4, "Companions at home"), "4 · Companions at home");
  assert.equal(placedName(undefined, "Gone"), "Gone");
});
