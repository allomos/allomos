// The Allomos mark (Task-A07-apex359): the top bar's and the About window's mark. When a window
// opens, the mark first plays the waiting animation (IconLoader.tsx) once, in the mark's own
// shapes, size and place, faster than the loader (3 s, approver 2026-10-22), and ends on the still
// mark, so nothing jumps when it stops. Once per
// window: a later render, a reload of the taskbag or another project in the same window shows the
// still mark. With reduced motion the still mark shows at once.

import { useState } from "react";

/** Whether this window has played the opening round already (one JavaScript module per window). */
let played = false;

function reducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

export function BrandMark({ className }: { className?: string }) {
  const [intro, setIntro] = useState(() => !played && !reducedMotion());
  const done = () => {
    played = true;
    setIntro(false);
  };
  const classes = [className, intro ? "mark-intro" : null].filter(Boolean).join(" ");
  return (
    <svg
      className={classes || undefined}
      viewBox="100 100 824 824"
      aria-hidden="true"
      onAnimationEnd={intro ? (e) => e.animationName === "mi-spark" && done() : undefined}
    >
      {/* A ring where a ball was, a solid ball where it arrived, a spark for the jump (owner's
          choice: layout Y2, colours D4). Same drawing as the app icon's small form. */}
      <circle className="ring jade mi-ring-jade" cx="265" cy="512" r="105" />
      <circle className="jade mi-jade" cx="759" cy="512" r="138" />
      <circle className="ring terracotta mi-ring-terra" cx="512" cy="265" r="105" />
      <circle className="terracotta mi-terra" cx="512" cy="759" r="138" />
      <path className="spark mi-spark" d="M512 392 L552 472 L632 512 L552 552 L512 632 L472 552 L392 512 L472 472 Z" />
    </svg>
  );
}
