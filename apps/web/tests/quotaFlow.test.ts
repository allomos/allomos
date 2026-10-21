// Tests for the header's agent chips and quota marks (Task-A03-02). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { ageText, agentId, chips, freshness, popupRows, tone } from "../src/quotaFlow.ts";
import type { QuotaReport } from "../src/types.ts";

const r = (agent: string, left: number, at: string, more: Partial<QuotaReport> = {}): QuotaReport => ({
  agent,
  left,
  at,
  window: "5h",
  ...more,
});

test("colours: green at 20% or more, amber from 5%, red under 5%", () => {
  assert.deepEqual([tone(100), tone(20), tone(19), tone(5), tone(4), tone(0)], ["green", "green", "amber", "amber", "red", "red"]);
});

test("fresh for 10 minutes, then the age, stale after 30 minutes or once reset", () => {
  const now = new Date("2026-10-20T10:40:00Z");
  assert.deepEqual(freshness(r("codex", 50, "2026-10-20T10:31:00Z"), now), { state: "fresh", minutes: 9 });
  assert.deepEqual(freshness(r("codex", 50, "2026-10-20T10:15:00Z"), now), { state: "aged", minutes: 25 });
  assert.equal(freshness(r("codex", 50, "2026-10-20T10:10:00Z"), now).state, "stale");
  const reset = new Date(now.getTime() - 60_000);
  const local = `${reset.getFullYear()}-${String(reset.getMonth() + 1).padStart(2, "0")}-${String(reset.getDate()).padStart(2, "0")}T${String(reset.getHours()).padStart(2, "0")}:${String(reset.getMinutes()).padStart(2, "0")}`;
  assert.equal(freshness(r("codex", 50, "2026-10-20T10:39:00Z", { resets: local }), now).state, "stale");
});

test("the agent that reported last comes first with its last account's 5-hour quota; others are dimmed", () => {
  const reports = [
    r("claude-code", 52, "2026-10-20T10:00:00Z"),
    r("codex", 80, "2026-10-20T10:05:00Z", { account: "home" }),
    r("codex", 30, "2026-10-20T10:20:00Z", { account: "work" }),
    r("codex", 70, "2026-10-20T10:30:00Z", { account: "work", window: "week" }),
  ];
  const list = chips(reports, { codex: true, claude: false });
  assert.deepEqual(
    list.map((c) => [c.name, c.current, c.quota?.left ?? null, c.told]),
    [
      ["Codex", true, 30, true],
      ["Claude Code", false, null, false],
    ],
  );
  assert.deepEqual(
    popupRows(reports).map((x) => `${x.agent}/${x.account ?? ""}/${x.window}`),
    ["codex/work/week", "codex/work/5h", "codex/home/5h", "claude-code//5h"],
  );
});

test("no reports: the two known agents, none current; another agent that reported gets a chip", () => {
  assert.deepEqual(
    chips([], { codex: true, claude: true }).map((c) => [c.name, c.current]),
    [
      ["Claude Code", false],
      ["Codex", false],
    ],
  );
  assert.deepEqual(
    chips([r("gemini", 9, "2026-10-20T10:00:00Z")], { codex: false, claude: true }).map((c) => [c.name, c.current, c.told]),
    [
      ["gemini", true, null],
      ["Claude Code", false, true],
      ["Codex", false, false],
    ],
  );
});

test("a report under a session name or short form counts for its agent, not as another agent", () => {
  assert.deepEqual(
    ["claude-code", "claude/a09-28-kanto-picker", " Claude ", "codex-cli/loop-2", "gemini/run-3"].map(agentId),
    ["claude-code", "claude-code", "claude-code", "codex", "gemini"],
  );
  const reports = [
    r("codex", 40, "2026-10-20T10:00:00Z"),
    r("claude/a09-28-kanto-picker", 86, "2026-10-20T10:30:00Z"),
  ];
  assert.deepEqual(
    chips(reports, { codex: true, claude: true }).map((c) => [c.name, c.current, c.quota?.left ?? null]),
    [
      ["Claude Code", true, 86],
      ["Codex", false, null],
    ],
  );
  assert.deepEqual(popupRows(reports).map((x) => x.agent), ["claude-code", "codex"]);
});

test("an old report reads in hours or days, not hundreds of minutes (A03-flour054)", () => {
  assert.deepEqual(ageText(25), { key: "agentline.age", n: 25 });
  assert.deepEqual(ageText(59), { key: "agentline.age", n: 59 });
  assert.deepEqual(ageText(60), { key: "agentline.age_hours", n: 1 });
  assert.deepEqual(ageText(633), { key: "agentline.age_hours", n: 10 });
  assert.deepEqual(ageText(24 * 60 - 1), { key: "agentline.age_hours", n: 23 });
  assert.deepEqual(ageText(24 * 60), { key: "agentline.age_days", n: 1 });
  assert.deepEqual(ageText(3 * 24 * 60 + 5), { key: "agentline.age_days", n: 3 });
});
