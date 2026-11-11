// When the agent line under the taskbag's name checks again (Task-A03-spade155): when the
// project opens, every 30 minutes, and whenever the person presses its refresh button. It used to
// check every 4 seconds, reading the whole change history each time, so Allomos kept using CPU
// in the background. Nothing here reads files: the caller passes the check and the timer.

/** How often the line checks again on its own: the instruction files are outside the taskbag,
 * and quota reports change only when an agent reports. */
export const CHECK_MS = 30 * 60 * 1000;

export interface Timer {
  setInterval(run: () => void, ms: number): number;
  clearInterval(id: number): void;
}

export interface Checks {
  /** Check now and start the 30 minutes again (the refresh button). */
  refresh(): void;
  /** Stop checking (the line goes away or shows another project). */
  stop(): void;
}

/** Runs `check` now and every [`CHECK_MS`] after; `refresh` runs it at once and restarts the wait. */
export function startChecks(check: () => void, timer: Timer): Checks {
  let id: number | null = null;
  const start = () => {
    check();
    id = timer.setInterval(check, CHECK_MS);
  };
  const stop = () => {
    if (id !== null) timer.clearInterval(id);
    id = null;
  };
  start();
  return {
    refresh() {
      stop();
      start();
    },
    stop,
  };
}
