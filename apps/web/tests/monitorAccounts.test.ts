// The extra accounts of Claude Code and Codex in the monitor (Task-A03-oven548): their keys, the
// rows the dialog shows (one per account; one when there is only one), the suggested name and
// the labels. Node only: `npm --prefix apps/web test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  accountKey,
  accountRows,
  agentLabel,
  baseAgent,
  isTerminalAgent,
  suggestedAccountName,
} from "../src/monitorFlow.ts";
import type { AgentAccount } from "../src/monitorFlow.ts";
import { labelOf, rememberAccounts } from "../src/agentAccounts.ts";

const extra: AgentAccount[] = [
  { id: "a1-2", agent: "claude_cli", name: "Work", dir: "/Users/me/claude-work" },
  { id: "b3-4", agent: "codex_cli", name: "Side", dir: "/Users/me/codex-side" },
];

test("an account's monitor agent is its agent's key with the account after a colon", () => {
  assert.equal(accountKey("claude_cli", "a1-2"), "claude_cli:a1-2");
  assert.equal(baseAgent("claude_cli:a1-2"), "claude_cli");
  assert.equal(baseAgent("codex_cli"), "codex_cli");
  assert.equal(isTerminalAgent("claude_cli:a1-2"), true);
  assert.equal(isTerminalAgent("codex_cli"), true);
  assert.equal(isTerminalAgent("claude_app"), false);
  assert.equal(isTerminalAgent("claude_app:x"), false);
});

test("an agent with no extra account is one row, named as before", () => {
  assert.deepEqual(accountRows("claude_cli", [], "Account 1"), [{ key: "claude_cli", name: null }]);
  assert.deepEqual(accountRows("antigravity_cli", extra, "Account 1"), [{ key: "antigravity_cli", name: null }]);
});

test("an agent with extra accounts is one row per account, the default one first", () => {
  assert.deepEqual(accountRows("claude_cli", extra, "Account 1"), [
    { key: "claude_cli", name: "Account 1" },
    { key: "claude_cli:a1-2", name: "Work" },
  ]);
  assert.deepEqual(accountRows("codex_cli", extra, "Account 1"), [
    { key: "codex_cli", name: "Account 1" },
    { key: "codex_cli:b3-4", name: "Side" },
  ]);
});

test("the suggested name counts from Account 2 for each agent", () => {
  assert.equal(suggestedAccountName([], "claude_cli"), "Account 2");
  assert.equal(suggestedAccountName(extra, "claude_cli"), "Account 3");
  assert.equal(suggestedAccountName(extra, "codex_cli"), "Account 3");
});

test("labels name the agent, then the account after a dot", () => {
  assert.equal(agentLabel("Claude Code (terminal)", null), "Claude Code (terminal)");
  assert.equal(agentLabel("Claude Code (terminal)", "Work"), "Claude Code (terminal) · Work");
  rememberAccounts(extra);
  assert.equal(labelOf("claude_cli:a1-2", "Claude Code (terminal)"), "Claude Code (terminal) · Work");
  assert.equal(labelOf("claude_cli", "Claude Code (terminal)"), "Claude Code (terminal)");
  // An account no longer listed names only its agent.
  assert.equal(labelOf("claude_cli:gone", "Claude Code (terminal)"), "Claude Code (terminal)");
  rememberAccounts([]);
});
