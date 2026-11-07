// Tests for Approve and Withdraw in the task pane (Task-A01-baker800). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canMakeReady, canWithdraw, expiresValue, hasPlan, withdrawnNotice } from "../src/approvalFlow.ts";
import type { ApprovalRecord } from "../src/types.ts";

const approval = (id: string, status: ApprovalRecord["status"]): ApprovalRecord => ({
  id,
  scope: { tasks: ["A01-01"], plan_sha256: "0".repeat(64), plan: "description" },
  at: "2026-11-08T00:00:00Z",
  channel: "app",
  status,
});

test("only an approval that applies offers Withdraw…", () => {
  assert.equal(canWithdraw({ status: "applies" }), true);
  for (const s of ["withdrawn", "expired", "plan_changed"] as const) assert.equal(canWithdraw({ status: s }), false);
});

test("Approve and make ready only on a backlog task", () => {
  assert.equal(canMakeReady({ state: "backlog" }), true);
  assert.equal(canMakeReady({ state: "ready" }), false);
  assert.equal(canMakeReady({ state: "in_progress" }), false);
});

test("an empty description has no plan to approve", () => {
  assert.equal(hasPlan({ description: " \n" }), false);
  assert.equal(hasPlan({ description: "## Outcome" }), true);
});

test("the expiry is a date or nothing", () => {
  assert.equal(expiresValue(""), null);
  assert.equal(expiresValue("2026-12-31"), "2026-12-31");
  assert.equal(expiresValue("soon"), null);
});

test("a withdrawn latest approval is pointed out only while in progress", () => {
  const withdrawn = [approval("ap-1", "applies"), approval("ap-2", "withdrawn")];
  assert.equal(withdrawnNotice({ state: "in_progress", approvals: withdrawn })?.id, "ap-2");
  assert.equal(withdrawnNotice({ state: "ready", approvals: withdrawn }), null);
  assert.equal(withdrawnNotice({ state: "in_progress", approvals: [...withdrawn, approval("ap-3", "applies")] }), null);
  assert.equal(withdrawnNotice({ state: "in_progress", approvals: [] }), null);
  assert.equal(withdrawnNotice({ state: "in_progress" }), null);
});
