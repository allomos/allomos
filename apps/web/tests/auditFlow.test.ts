import assert from "node:assert/strict";
import { test } from "node:test";
import { FIRST_WITNESS_WHEN, LEVELS, needsReason, replacedSummary, WITNESS_WHENS, witnessChoice, witnessUrl, witnessUrlOk } from "../src/auditFlow.ts";

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

test("Remove personal details takes the ticked and typed fields once each", async () => {
  const { redactFields, canRedact } = await import("../src/auditFlow.ts");
  assert.deepEqual(redactFields(["description"], "handoff_note, description  title"), ["description", "handoff_note", "title"]);
  assert.deepEqual(redactFields([], " "), []);
  assert.equal(canRedact("A01-01", ["description"], "a phone number", false), true);
  assert.equal(canRedact("A01-01", ["description"], "a phone number", true), false);
  assert.equal(canRedact("A01-01", ["description"], " ", false), false);
  assert.equal(canRedact(" ", ["description"], "why", false), false);
  assert.equal(canRedact("A01-01", [], "why", false), false);
});

test("a Last seen history line is there, gone or unknown", async () => {
  const { seenState } = await import("../src/auditFlow.ts");
  assert.equal(seenState(true), "there");
  assert.equal(seenState(false), "gone");
  assert.equal(seenState(null), "unknown");
});

test("after Replace key…, the taskbags where the old key was retired are counted and failures named", () => {
  const r = {
    taskbags: [
      { root: "/a/.allomos/taskbag", outcome: "replaced" },
      { root: "/b/.allomos/taskbag", outcome: "not_here" },
      { root: "/c/.allomos/taskbag", outcome: "failed" },
      { root: "/d/.allomos/taskbag", outcome: "replaced" },
    ],
  };
  assert.deepEqual(replacedSummary(r), { replaced: 2, failed: ["/c/.allomos/taskbag"] });
  assert.deepEqual(replacedSummary({ taskbags: [] }), { replaced: 0, failed: [] });
});

test("outside witness: a list choice stands for an address, and back (Task-A01-wood982)", () => {
  const presets = [
    { id: "freetsa", url: "https://freetsa.org/tsr" },
    { id: "digicert", url: "http://timestamp.digicert.com" },
  ];
  assert.equal(witnessUrl("freetsa", "", presets), "https://freetsa.org/tsr");
  assert.equal(witnessUrl("custom", " https://tsa.example.com/tsr ", presets), "https://tsa.example.com/tsr");
  assert.equal(witnessUrl("", "", presets), "");
  assert.equal(witnessChoice("http://timestamp.digicert.com", presets), "digicert");
  assert.equal(witnessChoice("https://tsa.example.com/tsr", presets), "custom");
  assert.equal(witnessChoice(undefined, presets), "");
  assert.ok(witnessUrlOk("https://tsa.example.com/tsr"));
  assert.ok(!witnessUrlOk("tsa.example.com"));
  assert.ok(!witnessUrlOk("https://a b"));
});

test("outside witness: how often is daily the first time, in the list's order (Task-A01-bake791)", () => {
  assert.equal(FIRST_WITNESS_WHEN, "daily");
  assert.deepEqual([...WITNESS_WHENS], ["daily", "approvals", "asked"]);
});
