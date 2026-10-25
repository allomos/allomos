// The messenger a phone or another computer's browser uses (Task-A04-02-03): the same interface
// as the desktop window's (`messenger.ts`), talking to the desktop app's local server instead of
// Tauri, so the companion page is the desktop app's own `App` with its views, components and
// look. It reads what the window reads; everything else (changes, which come with A04-02-04, and
// what only the computer itself can do) is refused with the `companion_read_only` problem, and
// the menu and window listeners do nothing. When the desktop app cannot be reached, or the
// device is no longer paired, `onLost` is told, so the page stops showing the data it had.

// Only type imports here, so Node can test it.

import type { Messenger } from "./messenger";
import type { Opened, TaskHistory } from "./types";
import type { FolderInfo as ProjectFolderInfo } from "./folderFlow";

export type Lost = "no_app" | "pairing";

const READ_ONLY = { code: "companion_read_only" };

/** `shape`: what the window does to an opened taskbag after reading it (`withDecisions`). */
export function lanMessenger(
  token: string,
  onLost: (why: Lost) => void,
  fetchFn: typeof fetch = (...args) => fetch(...args),
  shape: (opened: Opened) => Opened = (opened) => opened,
): Messenger {
  const get = async <T>(path: string, params: Record<string, string>): Promise<T> => {
    const query = new URLSearchParams(params).toString();
    let r: Response;
    try {
      r = await fetchFn(`/api/${path}?${query}`, { headers: { Authorization: `Bearer ${token}` } });
    } catch {
      onLost("no_app");
      throw { code: "companion_no_app" };
    }
    // Nothing answering behind a proxy or router counts as unreachable too.
    if (r.status >= 500) {
      onLost("no_app");
      throw { code: "companion_no_app" };
    }
    if (r.status === 403) {
      onLost("pairing");
      throw { code: "companion_no_app" };
    }
    if (!r.ok) throw { code: "companion_not_open" };
    return (await r.json()) as T;
  };

  const reading: Partial<Messenger> = {
    startupFolder: () => Promise.resolve(null),
    systemLanguages: () => Promise.resolve([...(navigator.languages ?? [])]),
    openTaskbag: (root) => get<Opened>("opened", { root }).then(shape),
    taskbagStamp: (root) => get<string>("stamp", { root }),
    taskHistory: (root, id) => get<TaskHistory>("history", { root, id }),
    projectFolders: (root) => get<ProjectFolderInfo[]>("folders", { root }),
    // Nothing the browser can act on: no notice, no quota chips.
    versionCheck: () => Promise.resolve(null),
    quotaReports: () => Promise.resolve([]),
  };

  return new Proxy(reading as Messenger, {
    get(target, name) {
      if (name in target) return target[name as keyof Messenger];
      // Not a method: so the messenger is never taken for a promise or a JSON value.
      if (typeof name !== "string" || name === "then" || name === "toJSON") return undefined;
      // Menu and window events never come here.
      if (typeof name === "string" && /^on[A-Z]/.test(name)) return () => Promise.resolve(() => undefined);
      return () => Promise.reject(READ_ONLY);
    },
  });
}
