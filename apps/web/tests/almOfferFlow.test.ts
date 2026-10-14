import { test } from "node:test";
import assert from "node:assert/strict";
import { almOffer } from "../src/almOfferFlow.ts";

const none = { offer: false, kind: null, own: null, found: null, leads_to: null, replaces_file: null, shadowed_by: null };
const own = "/Applications/Allomos.app/Contents/MacOS/alm";

test("no offer, nothing shown", () => {
  assert.equal(almOffer(null), null);
  assert.equal(almOffer(none), null);
});

test("a missing alm is offered plainly, with the app's own path", () => {
  const o = almOffer({ ...none, offer: true, kind: "missing", own });
  assert.deepEqual(o?.lines, [{ key: "alm.offer_missing" }]);
  assert.deepEqual(o?.paths, [{ key: "alm.own", params: { own } }]);
  assert.equal(o?.button, "alm.install");
});

test("a development build through the link: both paths, and the button switches back", () => {
  const o = almOffer({
    ...none,
    offer: true,
    kind: "development",
    own,
    found: "/usr/local/bin/alm",
    leads_to: "/Users/me/.cargo/bin/alm",
  });
  assert.deepEqual(o?.lines, [{ key: "alm.offer_development" }]);
  assert.deepEqual(o?.paths, [
    { key: "alm.in_use_link", params: { found: "/usr/local/bin/alm", target: "/Users/me/.cargo/bin/alm" } },
    { key: "alm.own", params: { own } },
  ]);
  assert.equal(o?.button, "alm.switch");
});

test("another alm: a real file it replaces and one found first are said too", () => {
  const o = almOffer({
    ...none,
    offer: true,
    kind: "other",
    own,
    found: "/opt/homebrew/bin/alm",
    replaces_file: "/usr/local/bin/alm",
    shadowed_by: "/opt/homebrew/bin/alm",
  });
  assert.deepEqual(
    o?.lines.map((l) => l.key),
    ["alm.offer_other", "alm.replaces_file", "alm.shadowed"],
  );
  assert.deepEqual(o?.paths[0], { key: "alm.in_use", params: { found: "/opt/homebrew/bin/alm" } });
  assert.equal(o?.button, "alm.install");
});
