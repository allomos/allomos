// Tests for pairing from the companion page (Task-A04-02-02). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { TOKEN_KEY, ask, authHeaders, cleanCode, nameOk, readToken, saveToken, status, typeCode } from "../src/pairFlow.ts";

test("a code is 9 digits, spaces may be typed", () => {
  assert.equal(cleanCode("482 913 076"), "482913076");
  assert.equal(cleanCode(" 482913076 "), "482913076");
  assert.equal(cleanCode("48291307"), null);
  assert.equal(cleanCode("48291307a"), null);
});

test("a device name is required and short", () => {
  assert.equal(nameOk("  "), false);
  assert.equal(nameOk("Kitchen iPad"), true);
  assert.equal(nameOk("x".repeat(41)), false);
});

test("the token is kept, and a refusing store does not break the page", () => {
  const items = new Map<string, string>();
  const store = { getItem: (k: string) => items.get(k) ?? null, setItem: (k: string, v: string) => void items.set(k, v), removeItem: (k: string) => void items.delete(k) };
  saveToken(store, "abc");
  assert.equal(items.get(TOKEN_KEY), "abc");
  assert.equal(readToken(store), "abc");
  saveToken(store, null);
  assert.equal(readToken(store), null);
  const broken = { getItem: () => { throw new Error("no"); }, setItem: () => { throw new Error("no"); }, removeItem: () => { throw new Error("no"); } };
  assert.equal(readToken(broken), null);
  saveToken(broken, "abc");
  assert.deepEqual(authHeaders(null), {});
  assert.deepEqual(authHeaders("t"), { Authorization: "Bearer t" });
});

test("the pairing calls", async () => {
  const sent: { url: string; init?: RequestInit }[] = [];
  const reply = (status: number, body: unknown) => Promise.resolve(new Response(JSON.stringify(body), { status }));
  let next: () => Promise<Response> = () => reply(200, { request: "r1" });
  const fetchFn = (url: string, init?: RequestInit) => {
    sent.push({ url, init });
    return next();
  };
  assert.equal(await ask(fetchFn, " Phone "), "r1");
  assert.equal(sent[0].url, "/api/pair");
  assert.deepEqual(JSON.parse(String(sent[0].init?.body)), { name: "Phone" });
  next = () => reply(200, { status: "approved", tries_left: 5 });
  assert.deepEqual(await status(fetchFn, "r1"), { status: "approved", tries_left: 5 });
  assert.equal(sent[1].url, "/api/pair?request=r1");
  next = () => reply(400, { error: "wrong_code", tries_left: 4 });
  assert.deepEqual(await typeCode(fetchFn, "r1", "000000000"), { ok: false, error: "wrong_code", tries_left: 4 });
  next = () => reply(400, { error: "new_code" });
  assert.deepEqual(await typeCode(fetchFn, "r1", "000000000"), { ok: false, error: "new_code" });
  next = () => reply(200, { token: "t", device: "Phone" });
  assert.deepEqual(await typeCode(fetchFn, "r1", "123456789"), { ok: true, token: "t", device: "Phone" });
  next = () => reply(500, "x");
  assert.deepEqual(await typeCode(fetchFn, "r1", "123456789"), { ok: false, error: "failed" });
});
