// The messenger a phone or another computer's browser uses (Task-A04-02-03): the same interface
// as the desktop window's (`messenger.ts`), talking to the desktop app's local server instead of
// Tauri, so the companion page is the desktop app's own `App` with its views, components and
// look. It reads what the window reads, and makes the window's changes (Task-A04-02-04, R77)
// through the server's `/api/change`, under the person's name on this device and the device's
// name ("will (iPhone)"). What only the computer itself can do is refused with the
// `companion_read_only` problem (R68), and the menu and window listeners do nothing. When the desktop app cannot be reached, or the
// device is no longer paired, `onLost` is told, so the page stops showing the data it had.

// Only type imports here, so Node can test it.

import type { Messenger } from "./messenger";
import type { Area, Iteration, Opened, Task, TaskHistory } from "./types";
import type { FolderInfo as ProjectFolderInfo } from "./folderFlow";

export type Lost = "no_app" | "pairing";

const READ_ONLY = { code: "companion_read_only" };

/** Where this browser keeps the person's name for changes (asked once, like the window's). */
export const NAME_KEY = "allomos.lan.name";

/** The person's name on this device, kept in the browser; storage that refuses is no name. */
export function storedName(store: Pick<Storage, "getItem"> | undefined): string | null {
  try {
    return store?.getItem(NAME_KEY)?.trim() || null;
  } catch {
    return null;
  }
}

/** `shape`: what the window does to an opened taskbag after reading it (`withDecisions`). */
export function lanMessenger(
  token: string,
  onLost: (why: Lost) => void,
  fetchFn: typeof fetch = (...args) => fetch(...args),
  shape: (opened: Opened) => Opened = (opened) => opened,
  store: Pick<Storage, "getItem" | "setItem"> | undefined = typeof localStorage === "undefined" ? undefined : localStorage,
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

  // One change, made by the desktop app under its own rules (Task-A04-02-04). A refusal comes back
  // as the Problem the window would get, so the page words it the same way.
  const change = async <T>(root: string, op: string, args: Record<string, unknown>): Promise<T> => {
    let r: Response;
    try {
      r = await fetchFn("/api/change", {
        method: "POST",
        headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
        body: JSON.stringify({ root, name: storedName(store) ?? "", op, args }),
      });
    } catch {
      onLost("no_app");
      throw { code: "companion_no_app" };
    }
    if (r.status >= 500) {
      onLost("no_app");
      throw { code: "companion_no_app" };
    }
    if (r.status === 403) {
      onLost("pairing");
      throw { code: "companion_no_app" };
    }
    const body = (await r.json().catch(() => ({}))) as { ok?: T; problem?: unknown };
    if (!r.ok) throw body.problem ?? { code: "companion_not_open" };
    return body.ok as T;
  };

  const reading: Partial<Messenger> = {
    ownerName: () => Promise.resolve(storedName(store)),
    saveOwnerName: (name) => {
      try {
        store?.setItem(NAME_KEY, name.trim());
      } catch {
        // Kept for this page only; asked again next time.
      }
      return Promise.resolve();
    },
    editTask: (root, id, edit) => change<Task>(root, "edit_task", { id, edit }),
    reviewAiDecisions: (root, id) => change<void>(root, "review_ai_decisions", { id }),
    addArea: (root, id, name) => change<Area>(root, "add_area", { id, name }),
    addIteration: (root, iteration) => change<Iteration>(root, "add_iteration", { iteration }),
    addTask: (root, task) => change<Task>(root, "add_task", { task }),
    addIterationAndPlan: (root, task, iteration) => change<Task>(root, "add_iteration_and_plan", { task, iteration }),
    renameArea: (root, id, name) => change<Area>(root, "rename_area", { id, name }),
    changeIteration: (root, id, c) => change<Iteration>(root, "change_iteration", { id, change: c }),
    deleteArea: (root, id) => change<void>(root, "delete_area", { id }),
    deleteIteration: (root, id) => change<void>(root, "delete_iteration", { id }),
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
