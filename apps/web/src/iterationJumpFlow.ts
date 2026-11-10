// Jumping to an iteration from the Monitors list (Task-A03-meteor649): the iteration's name in a row
// is a link; it brings the project's window forward and shows that iteration in the plan view.
// The request is kept in local storage, so a window that is only now opening (or another window
// of the same app) can take it up; the same window hears a custom event at once.

export const JUMP_KEY = "allomos.jumpIteration";
export const JUMP_EVENT = "allomos:jump-iteration";

export interface IterationJump {
  root: string;
  id: string;
}

/** The project's folder, whether it is given as the folder or as its taskbag. */
export function projectKey(root: string): string {
  return root.replace(/[\\/]\.allomos[\\/]taskbag[\\/]?$/, "").replace(/[\\/]+$/, "");
}

/** Whether a request is for this project. */
export function jumpIsFor(jump: IterationJump | null, root: string): jump is IterationJump {
  return jump !== null && projectKey(jump.root) === projectKey(root);
}

/** Reads a request kept in storage; a broken or missing one is no request. */
export function readJump(raw: string | null): IterationJump | null {
  if (!raw) return null;
  try {
    const v = JSON.parse(raw) as Partial<IterationJump>;
    return typeof v?.root === "string" && typeof v?.id === "string" ? { root: v.root, id: v.id } : null;
  } catch {
    return null;
  }
}

/** Takes the request kept for this project and clears it, so it is taken once. */
export function takeJump(root: string): IterationJump | null {
  try {
    const jump = readJump(localStorage.getItem(JUMP_KEY));
    if (!jumpIsFor(jump, root)) return null;
    localStorage.removeItem(JUMP_KEY);
    return jump;
  } catch {
    return null; // storage unavailable: nothing to take
  }
}

/** Asks the project's window to show an iteration: keeps the request, tells this window, and brings
 * the project's window forward (`open`, the app's own way to open or focus a project). */
export function askJump(jump: IterationJump, open: () => Promise<void>): Promise<void> {
  try {
    localStorage.setItem(JUMP_KEY, JSON.stringify(jump));
  } catch {
    /* storage unavailable: this window still hears the event below */
  }
  window.dispatchEvent(new CustomEvent<IterationJump>(JUMP_EVENT, { detail: jump }));
  return open();
}
