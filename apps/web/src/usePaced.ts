// Run a check now and again and again, at the pace paceFlow.ts sets (Task-A03-reef286): quickly
// while the window is in use, slowly while it is in the background, and at once when it comes
// back into use.

import { useEffect, useRef } from "react";
import { inUse, paceMs } from "./paceFlow";

export function usePaced(check: () => void, awakeMs: number, idleMs: number, deps: readonly unknown[]): void {
  const checkRef = useRef(check);
  checkRef.current = check;
  useEffect(() => {
    let timer: number | undefined;
    let live = true;
    const using = () => inUse(document);
    const schedule = () => {
      if (!live) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(run, paceMs(using(), awakeMs, idleMs));
    };
    const run = () => {
      if (!live) return;
      checkRef.current();
      schedule();
    };
    // Back in use: check at once rather than waiting out the slow pause.
    const wake = () => {
      if (using()) run();
      else schedule();
    };
    run();
    window.addEventListener("focus", wake);
    window.addEventListener("blur", schedule);
    document.addEventListener("visibilitychange", wake);
    return () => {
      live = false;
      window.clearTimeout(timer);
      window.removeEventListener("focus", wake);
      window.removeEventListener("blur", schedule);
      document.removeEventListener("visibilitychange", wake);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [awakeMs, idleMs, ...deps]);
}
