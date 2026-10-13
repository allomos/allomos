import { test } from "node:test";
import assert from "node:assert/strict";
import { almOfferLines } from "../src/almOfferFlow.ts";

const none = { offer: false, found: null, replaces_file: null, shadowed_by: null };

test("no offer, no lines", () => {
  assert.deepEqual(almOfferLines(null), []);
  assert.deepEqual(almOfferLines(none), []);
});

test("a missing alm is offered plainly", () => {
  assert.deepEqual(almOfferLines({ ...none, offer: true }), [{ key: "alm.offer_missing" }]);
});

test("another alm is named, a real file it replaces too, and one found first is warned about", () => {
  const lines = almOfferLines({
    offer: true,
    found: "/Users/me/.cargo/bin/alm",
    replaces_file: "/usr/local/bin/alm",
    shadowed_by: "/Users/me/.cargo/bin/alm",
  });
  assert.deepEqual(
    lines.map((l) => l.key),
    ["alm.offer_other", "alm.replaces_file", "alm.shadowed"],
  );
  assert.equal(lines[0].params?.found, "/Users/me/.cargo/bin/alm");
  assert.equal(lines[1].params?.file, "/usr/local/bin/alm");
});
