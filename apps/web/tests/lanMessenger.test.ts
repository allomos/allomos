// Tests for the companion's messenger (Task-A04-02-03). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { lanMessenger, type Lost } from "../src/lanMessenger.ts";

const answer = (status: number, body: unknown) => Promise.resolve(new Response(JSON.stringify(body), { status }));

test("it reads through the server with the device's token, and refuses what only the computer does", async () => {
  const asked: { url: string; auth: string | null }[] = [];
  const lost: Lost[] = [];
  const m = lanMessenger(
    "tok",
    (why) => lost.push(why),
    ((url: string, init?: RequestInit) => {
      asked.push({ url, auth: new Headers(init?.headers).get("Authorization") });
      return answer(200, "42");
    }) as typeof fetch,
  );
  assert.equal(await m.taskbagStamp("/p/.allomos/taskbag"), "42");
  assert.deepEqual(asked, [{ url: "/api/stamp?root=%2Fp%2F.allomos%2Ftaskbag", auth: "Bearer tok" }]);
  await assert.rejects(m.showInFileManager("/p"), { code: "companion_read_only" });
  // Menu and window events never come: listening does nothing.
  const stop = await m.onWindowFocus(() => undefined);
  stop();
  assert.equal(await m.versionCheck("/p"), null);
  assert.deepEqual(lost, []);
});

test("an unreachable desktop app, or a removed device, is told", async () => {
  const lost: Lost[] = [];
  const down = lanMessenger("tok", (why) => lost.push(why), (() => Promise.reject(new TypeError("offline"))) as typeof fetch);
  await assert.rejects(down.taskbagStamp("/p"), { code: "companion_no_app" });
  const removed = lanMessenger("tok", (why) => lost.push(why), (() => answer(403, { error: "pairing_required" })) as typeof fetch);
  await assert.rejects(removed.openTaskbag("/p"), { code: "companion_no_app" });
  const closed = lanMessenger("tok", (why) => lost.push(why), (() => answer(404, { error: "not_open" })) as typeof fetch);
  await assert.rejects(closed.taskHistory("/p", "A1-01"), { code: "companion_not_open" });
  assert.deepEqual(lost, ["no_app", "pairing"]);
});

test("it is not taken for a promise", async () => {
  const m = lanMessenger("tok", () => undefined, (() => answer(200, null)) as typeof fetch);
  assert.equal((m as unknown as { then?: unknown }).then, undefined);
  assert.equal(await Promise.resolve(m), m);
});

test("changes go to the desktop app under the name kept on this device (Task-A04-02-04)", async () => {
  const items = new Map<string, string>();
  const store = { getItem: (k: string) => items.get(k) ?? null, setItem: (k: string, v: string) => void items.set(k, v) };
  const sent: { url: string; body: unknown }[] = [];
  let reply = () => answer(200, { ok: { id: "A1-01", title: "x" } });
  const m = lanMessenger(
    "tok",
    () => undefined,
    ((url: string, init?: RequestInit) => {
      sent.push({ url, body: JSON.parse(String(init?.body)) });
      return reply();
    }) as typeof fetch,
    (o) => o,
    store,
  );
  assert.equal(await m.ownerName(), null);
  await m.saveOwnerName(" Will ");
  assert.equal(await m.ownerName(), "Will");
  const t = await m.editTask("/p", "A1-01", { field: "state", state: "ready" });
  assert.equal(t.id, "A1-01");
  assert.deepEqual(sent[0], { url: "/api/change", body: { root: "/p", name: "Will", op: "edit_task", args: { id: "A1-01", edit: { field: "state", state: "ready" } } } });
  // A refusal is the window's Problem, so the page words it the same way.
  reply = () => answer(400, { problem: { code: "task_changed", params: { id: "A1-01" } } });
  await assert.rejects(m.editTask("/p", "A1-01", { field: "title", title: "y" }), { code: "task_changed" });
  reply = () => answer(200, { ok: { id: "A2", name: "Core" } });
  await m.addArea("/p", null, "Core");
  assert.equal((sent[2].body as { op: string }).op, "add_area");
});
