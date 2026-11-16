// Opening a task from the All monitors list (Task-A03-jazz112): the request reaches this window
// at once, another window through storage, or waits until the matching project opens.

import { projectKey } from "./iterationJumpFlow";

export const TASK_JUMP_KEY = "allomos.jumpTask";
export const TASK_JUMP_EVENT = "allomos:jump-task";

export interface TaskJump {
  root: string;
  id: string;
}

export function taskJumpIsFor(jump: TaskJump | null, root: string): jump is TaskJump {
  return jump !== null && projectKey(jump.root) === projectKey(root);
}

export function readTaskJump(raw: string | null): TaskJump | null {
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<TaskJump>;
    return typeof value?.root === "string" && typeof value?.id === "string" ? { root: value.root, id: value.id } : null;
  } catch {
    return null;
  }
}

export function takeTaskJump(root: string): TaskJump | null {
  try {
    const jump = readTaskJump(localStorage.getItem(TASK_JUMP_KEY));
    if (!taskJumpIsFor(jump, root)) return null;
    localStorage.removeItem(TASK_JUMP_KEY);
    return jump;
  } catch {
    return null;
  }
}

export function askTaskJump(jump: TaskJump, open: () => Promise<void>): Promise<void> {
  try {
    localStorage.setItem(TASK_JUMP_KEY, JSON.stringify(jump));
  } catch {
    /* storage unavailable: this window still hears the event below */
  }
  window.dispatchEvent(new CustomEvent<TaskJump>(TASK_JUMP_EVENT, { detail: jump }));
  return open();
}
