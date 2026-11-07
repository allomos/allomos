import assert from "node:assert/strict";
import { test } from "node:test";
import { LEVELS, needsReason } from "../src/auditFlow.ts";

test("the levels go from the pseudonym up", () => {
  assert.deepEqual(LEVELS, ["pseudonym", "account", "sso"]);
});

test("lowering the identity level or turning signing in again off asks for a reason", () => {
  const pseudonym = { level: "pseudonym" as const, reauthenticate: false };
  assert.equal(needsReason(pseudonym, "pseudonym", false), false);
  assert.equal(needsReason(pseudonym, "sso", true), false);
  const sso = { level: "sso" as const, reauthenticate: true };
  assert.equal(needsReason(sso, "sso", true), false);
  assert.equal(needsReason(sso, "sso", false), true);
  assert.equal(needsReason(sso, "account", true), true);
  assert.equal(needsReason(sso, "pseudonym", false), true);
});
