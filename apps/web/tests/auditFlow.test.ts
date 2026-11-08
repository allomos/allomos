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

test("turning separation of duties off asks for a reason; turning it on does not", async () => {
  const { separationNeedsReason } = await import("../src/auditFlow.ts");
  assert.equal(separationNeedsReason(false, true), false);
  assert.equal(separationNeedsReason(true, true), false);
  assert.equal(separationNeedsReason(true, false), true);
  assert.equal(separationNeedsReason(false, false), false);
});

test("a shorter retention period and the legal hold ask for a reason (Task-A01-stem986)", async () => {
  const { retentionNeedsReason, yearsFrom } = await import("../src/auditFlow.ts");
  const forever = { years: null, hold: false };
  assert.equal(retentionNeedsReason(forever, null, false), false);
  assert.equal(retentionNeedsReason(forever, 7, false), true);
  assert.equal(retentionNeedsReason({ years: 7, hold: false }, 10, false), false);
  assert.equal(retentionNeedsReason({ years: 7, hold: false }, null, false), false);
  assert.equal(retentionNeedsReason({ years: 7, hold: false }, 5, false), true);
  assert.equal(retentionNeedsReason(forever, null, true), true);
  assert.equal(retentionNeedsReason({ years: null, hold: true }, null, false), true);
  assert.equal(yearsFrom("7"), 7);
  assert.equal(yearsFrom("0"), null);
  assert.equal(yearsFrom("2.5"), null);
  assert.equal(yearsFrom(""), null);
});
