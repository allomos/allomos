// The task pane's inner edge (Task-A01-56): drag it, or use the arrow keys when it has focus, to
// make the pane wider or narrower; double-click it to go back to the default width. The width is
// kept in the app's own storage, so it is the same the next time the pane opens and after a restart.

import { useRef, type RefObject } from "react";
import { useI18n } from "./i18n";
import { KEY_STEP, MIN_PANE, PANE_WIDTH_KEY, clampPane, dragWidth, maxPane } from "./paneWidthFlow";

/** The column the edge sits in, between the list and the pane, in pixels. */
export const EDGE = 14;

export function PaneResizer({
  layout,
  width,
  onWidth,
}: {
  /** The list and pane's shared box, to measure against. */
  layout: RefObject<HTMLDivElement | null>;
  /** The chosen width, or null for the default. */
  width: number | null;
  onWidth: (width: number | null) => void;
}) {
  const { t } = useI18n();
  const dragging = useRef(false);
  const box = () => layout.current?.getBoundingClientRect() ?? null;
  // The pane's width now, measured when no width has been chosen yet.
  const current = () => {
    if (width !== null) return width;
    const pane = layout.current?.querySelector<HTMLElement>(".task-pane");
    return pane ? pane.getBoundingClientRect().width : MIN_PANE;
  };
  const keep = (w: number | null) => {
    try {
      if (w === null) localStorage.removeItem(PANE_WIDTH_KEY);
      else localStorage.setItem(PANE_WIDTH_KEY, String(w));
    } catch {
      /* storage unavailable: the width lasts until the window closes */
    }
  };
  const rect = box();
  return (
    <div
      className="pane-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={t("pane.resize")}
      title={t("pane.resize")}
      aria-valuemin={MIN_PANE}
      aria-valuemax={rect ? maxPane(rect.width, EDGE) : undefined}
      aria-valuenow={Math.round(current())}
      tabIndex={0}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        dragging.current = true;
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        const r = box();
        if (!dragging.current || !r) return;
        // The pointer sits in the middle of the edge's column.
        onWidth(dragWidth(e.clientX + EDGE / 2, r.right, r.width, EDGE));
      }}
      onPointerUp={(e) => {
        if (!dragging.current) return;
        dragging.current = false;
        e.currentTarget.releasePointerCapture(e.pointerId);
        const r = box();
        if (r) {
          const w = dragWidth(e.clientX + EDGE / 2, r.right, r.width, EDGE);
          onWidth(w);
          keep(w);
        }
      }}
      onDoubleClick={() => {
        onWidth(null);
        keep(null);
      }}
      onKeyDown={(e) => {
        const r = box();
        if (!r) return;
        const by = e.key === "ArrowLeft" ? KEY_STEP : e.key === "ArrowRight" ? -KEY_STEP : 0;
        const to =
          by !== 0
            ? clampPane(current() + by, r.width, EDGE)
            : e.key === "Home"
              ? MIN_PANE
              : e.key === "End"
                ? maxPane(r.width, EDGE)
                : null;
        if (to === null) return;
        e.preventDefault();
        onWidth(to);
        keep(to);
      }}
    />
  );
}
