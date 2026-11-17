import { useLayoutEffect, useState } from "react";
import type { CSSProperties, ReactNode, RefObject } from "react";
import { createPortal } from "react-dom";

/** Places a task-row menu above its clipping list when there is not enough room below it. */
export function AnchoredPopover({
  anchorRef,
  popoverRef,
  label,
  className,
  children,
}: {
  anchorRef: RefObject<HTMLElement | null>;
  popoverRef: RefObject<HTMLDivElement | null>;
  label: string;
  className: string;
  children: ReactNode;
}) {
  const [style, setStyle] = useState<CSSProperties>({ position: "fixed", visibility: "hidden" });

  useLayoutEffect(() => {
    const place = () => {
      const anchor = anchorRef.current;
      const popover = popoverRef.current;
      if (!anchor || !popover) return;

      const anchorBounds = anchor.getBoundingClientRect();
      const popoverBounds = popover.getBoundingClientRect();
      const gap = 6;
      const edge = 8;
      const above = Math.max(0, anchorBounds.top - gap - edge);
      const below = Math.max(0, window.innerHeight - anchorBounds.bottom - gap - edge);
      const openAbove = below < popoverBounds.height && above > below;
      const room = openAbove ? above : below;
      const visibleHeight = Math.min(popoverBounds.height, room);
      const top = openAbove
        ? Math.max(edge, anchorBounds.top - gap - visibleHeight)
        : Math.min(window.innerHeight - edge - visibleHeight, anchorBounds.bottom + gap);
      const left = Math.max(edge, Math.min(anchorBounds.left, window.innerWidth - popoverBounds.width - edge));

      setStyle({
        position: "fixed",
        left: `${left}px`,
        top: `${top}px`,
        zIndex: 20,
        maxHeight: `${room}px`,
        overflowY: popoverBounds.height > room ? "auto" : "visible",
        visibility: "visible",
      });
    };

    place();
    window.addEventListener("resize", place);
    document.addEventListener("scroll", place, true);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
    if (anchorRef.current) observer?.observe(anchorRef.current);
    if (popoverRef.current) observer?.observe(popoverRef.current);

    return () => {
      window.removeEventListener("resize", place);
      document.removeEventListener("scroll", place, true);
      observer?.disconnect();
    };
  }, [anchorRef, popoverRef]);

  return createPortal(
    <div ref={popoverRef} className={className} role="menu" aria-label={label} style={style}>
      {children}
    </div>,
    document.body,
  );
}
