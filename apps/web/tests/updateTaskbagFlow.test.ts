// Tests for when Update the taskbag… shows and which agents it offers (Task-A03-27).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { lastUsed, needsUpdate, offer, offered, registered } from "../src/updateTaskbagFlow.ts";
import type { Agent, VersionCheck } from "../src/types.ts";

const check = (over: Partial<VersionCheck> = {}): VersionCheck => ({
  format_current: "allomos/4",
  section_current: 15,
  format: { state: "current" },
  move_in: null,
  sections: [{ file: "AGENTS.md", version: 15 }],
  ...over,
});

test("the taskbag needs updating only for card files, an older taskbag or an out-of-date section", () => {
  assert.equal(needsUpdate(0, null), false);
  assert.equal(needsUpdate(0, check()), false);
  assert.equal(needsUpdate(2, check()), true);
  assert.equal(needsUpdate(0, check({ move_in: { old: "x", found: "allomos/3", prepared: false } })), true);
  assert.equal(needsUpdate(0, check({ sections: [{ file: "AGENTS.md", version: 14 }] })), true);
  assert.equal(needsUpdate(0, check({ sections: [{ file: "CLAUDE.md", version: null }] })), true);
  // No tasks yet is not a reason: nothing here even asks about tasks.
});

test("an agent is registered when an entry under Agents set up names it", () => {
  const entries = [
    "Claude Code: `CLAUDE.md` imports AGENTS.md and keeps only its own lines. 2026-10-16.",
    "Codex: reads `AGENTS.md` directly; no file of its own. 2026-10-16.",
  ];
  assert.equal(registered("Claude", entries), true);
  assert.equal(registered("Codex", [entries[0]]), false);
  assert.equal(registered("Codex", []), false);
  // Any case, Markdown marks ignored; only the name before the colon counts, as a whole word.
  assert.equal(registered("Codex", ["**codex**: AGENTS.md"]), true);
  assert.equal(registered("Claude", ["Gemini CLI: mentions Claude"]), false);
  assert.equal(registered("Codex", ["Codexify: x"]), false);
});

test("the agent used last comes from the events, else from what the app opened", () => {
  assert.deepEqual(lastUsed({ agent: "claude", tool: "claude_cli" }, "codex_app"), { group: "Claude", tool: "claude_cli" });
  assert.deepEqual(lastUsed({ agent: "codex", tool: null }, null), { group: "Codex", tool: null });
  assert.deepEqual(lastUsed({ agent: "claude", tool: "something" }, null), { group: "Claude", tool: null });
  assert.deepEqual(lastUsed(null, "codex_cli"), { group: "Codex", tool: "codex_cli" });
  assert.deepEqual(lastUsed({ agent: "gemini", tool: null }, "claude_app"), { group: "Claude", tool: "claude_app" });
  assert.equal(lastUsed(null, null), null);
});

const claudeCli: Agent = { id: "claude_cli", found: "/bin/claude", how: "link" };
const claudeCliPaste: Agent = { id: "claude_cli", found: "/bin/claude", how: "paste" };
const claudeApp: Agent = { id: "claude_app", found: "Claude", how: "link" };
const codexCli: Agent = { id: "codex_cli", found: "/bin/codex", how: "paste" };
const codexApp: Agent = { id: "codex_app", found: "Codex", how: "link" };
const everything = [claudeCli, claudeApp, codexCli, codexApp];
const claudeOnly = ["Claude Code: CLAUDE.md. 2026-10-16."];

test("not found, registered, unreadable notes or no longer here: every agent", () => {
  assert.deepEqual(offer(everything, null, []), { kind: "all" });
  assert.deepEqual(offer(everything, { group: "Claude", tool: "claude_cli" }, claudeOnly), { kind: "all" });
  assert.deepEqual(offer(everything, { group: "Codex", tool: null }, null), { kind: "all" });
  assert.deepEqual(offer([claudeCli, claudeApp], { group: "Codex", tool: "codex_app" }, claudeOnly), { kind: "all" });
  assert.deepEqual(offered(everything, { kind: "all" }), everything);
});

test("used last and not registered: only that agent, its last tool first", () => {
  const o = offer(everything, { group: "Codex", tool: "codex_app" }, claudeOnly);
  assert.deepEqual(o, { kind: "only", group: "Codex", tools: ["codex_app"] });
  assert.deepEqual(offered(everything, o), [codexApp]);
  assert.deepEqual(offer(everything, { group: "Claude", tool: "claude_cli" }, []), {
    kind: "only",
    group: "Claude",
    tools: ["claude_cli"],
  });
});

test("its last tool cannot be opened: the agent's other tools that open with the request typed in", () => {
  // The last tool is not on this computer any more.
  assert.deepEqual(offer([claudeApp, codexApp], { group: "Claude", tool: "claude_cli" }, []), {
    kind: "only",
    group: "Claude",
    tools: ["claude_app"],
  });
  // Tool unknown (the environment did not tell): every one that opens with the request typed in.
  assert.deepEqual(offer(everything, { group: "Claude", tool: null }, []), {
    kind: "only",
    group: "Claude",
    tools: ["claude_cli", "claude_app"],
  });
  // The terminal opens without the request typed in (it is pasted): the app instead.
  assert.deepEqual(offer([claudeCliPaste, claudeApp], { group: "Claude", tool: "claude_cli" }, []), {
    kind: "only",
    group: "Claude",
    tools: ["claude_app"],
  });
});

test("no tool of that agent opens with the request typed in: Manual alone, to paste into it", () => {
  const o = offer([claudeApp, codexCli], { group: "Codex", tool: "codex_cli" }, claudeOnly);
  assert.deepEqual(o, { kind: "manual", group: "Codex" });
  assert.deepEqual(offered([claudeApp, codexCli], o), []);
});
