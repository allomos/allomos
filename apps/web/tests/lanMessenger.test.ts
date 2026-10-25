// Tests for the companion's messenger (Task-A04-02-03). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { lanMessenger, type Lost } from "../src/lanMessenger.ts";

const answer = (status: number, body: unknown) => Promise.resolve(new Response(JSON.stringify(body), { status }));

test("it reads through the server with the device's token, and refuses changes", async () => {
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
  await assert.rejects(m.editTask("/p", "A1-01", {} as never), { code: "companion_read_only" });
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
