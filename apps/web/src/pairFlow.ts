// Pairing from the companion page (Task-A04-02-02): what the page asks the desktop app's server,
// and what it keeps. The page asks for a device name, waits while the person approves it on the
// computer, then takes the 9-digit code the desktop app shows; the token it gets is kept in this
// browser, so the device pairs once.

/** Where this browser keeps its access (a random token; the desktop app keeps only its hash). */
export const TOKEN_KEY = "allomos.lan.token";

export function readToken(store: Pick<Storage, "getItem"> | undefined): string | null {
  try {
    return store?.getItem(TOKEN_KEY) ?? null;
  } catch {
    return null;
  }
}

export function saveToken(store: Pick<Storage, "setItem" | "removeItem"> | undefined, token: string | null): void {
  try {
    if (token) store?.setItem(TOKEN_KEY, token);
    else store?.removeItem(TOKEN_KEY);
  } catch {
    // Private windows may refuse: the device then pairs again next time.
  }
}

/** The code as typed, without spaces; `null` unless it is 9 digits. */
export function cleanCode(text: string): string | null {
  const code = text.replace(/\s+/g, "");
  return /^\d{9}$/.test(code) ? code : null;
}

/** The longest device name (`NAME_MAX` in pairing.rs). */
export const NAME_MAX = 40;

export function nameOk(name: string): boolean {
  const n = name.trim();
  return n.length > 0 && [...n].length <= NAME_MAX;
}

export type PairStatus = { status: "waiting" } | { status: "approved"; tries_left: number } | { status: "refused" } | { status: "expired" };

/** What typing the code gave. */
export type CodeResult = { ok: true; token: string; device: string } | { ok: false; error: "wrong_code"; tries_left: number } | { ok: false; error: "new_code" | "not_approved" | "failed" };

type Fetch = (url: string, init?: RequestInit) => Promise<Response>;

const json = (body: unknown): RequestInit => ({ method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });

/** Ask to pair under `name`; resolves to the request's ID. */
export async function ask(fetchFn: Fetch, name: string): Promise<string> {
  const r = await fetchFn("/api/pair", json({ name: name.trim() }));
  if (!r.ok) throw new Error(String(r.status));
  return ((await r.json()) as { request: string }).request;
}

export async function status(fetchFn: Fetch, request: string): Promise<PairStatus> {
  const r = await fetchFn(`/api/pair?request=${encodeURIComponent(request)}`);
  if (!r.ok) throw new Error(String(r.status));
  return (await r.json()) as PairStatus;
}

export async function typeCode(fetchFn: Fetch, request: string, code: string): Promise<CodeResult> {
  const r = await fetchFn("/api/pair/code", json({ request, code }));
  const v = (await r.json().catch(() => ({}))) as { token?: string; device?: string; error?: string; tries_left?: number };
  if (r.ok && v.token) return { ok: true, token: v.token, device: v.device ?? "" };
  if (v.error === "wrong_code") return { ok: false, error: "wrong_code", tries_left: v.tries_left ?? 0 };
  if (v.error === "new_code" || v.error === "not_approved") return { ok: false, error: v.error };
  return { ok: false, error: "failed" };
}

/** The headers that send this device's access, when it has one. */
export function authHeaders(token: string | null): Record<string, string> {
  return token ? { Authorization: `Bearer ${token}` } : {};
}
