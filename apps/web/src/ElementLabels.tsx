import { useEffect, useState } from "react";
import { assignLabels, isLabelShortcut } from "./labelFlow";
import type { Box } from "./labelFlow";

// Elements worth pointing at: controls, links, images, headings, list rows, anything named, and
// anything that shows text of its own. Plain layout wrappers are left out so labels stay readable.
const POINTABLE =
  "a, button, input, select, textarea, summary, label, img, svg, video, canvas, h1, h2, h3, h4, h5, h6, li, th, td, dt, dd, [role], [tabindex], [title], [aria-label]";
// Inside these, the control itself is labelled, not the text it holds.
const CONTROL = "a, button, label, summary, select, [role=button], [role=tab], [role=menuitem], [role=option]";

type Placed = { label: string; top: number; left: number };

function ownText(el: Element): boolean {
  for (const node of el.childNodes) if (node.nodeType === Node.TEXT_NODE && node.textContent?.trim()) return true;
  return false;
}

/** The labelled elements now visible in the window, with where each label goes. */
function measure(): Placed[] {
  const found: { el: Element; box: Box }[] = [];
  const width = window.innerWidth;
  const height = window.innerHeight;
  for (const el of document.body.querySelectorAll("*")) {
    if (el.closest(".element-labels, script, style, template, svg *")) continue;
    if (!el.matches(POINTABLE) && !(ownText(el) && !el.parentElement?.closest(CONTROL))) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) continue;
    if (rect.bottom <= 0 || rect.right <= 0 || rect.top >= height || rect.left >= width) continue;
    const style = getComputedStyle(el);
    if (style.visibility === "hidden" || style.opacity === "0") continue;
    // Skip what is covered, for example the page behind an open dialog.
    const x = Math.min(Math.max(rect.left + rect.width / 2, 0), width - 1);
    const y = Math.min(Math.max(rect.top + rect.height / 2, 0), height - 1);
    const hit = document.elementFromPoint(x, y);
    if (!hit || !(el.contains(hit) || hit.contains(el))) continue;
    found.push({ el, box: { top: rect.top, left: rect.left, width: rect.width, height: rect.height } });
  }
  const labels = assignLabels(found.map((f) => f.box));
  const placed: Placed[] = [];
  found.forEach(({ box }, at) => {
    const label = labels[at];
    if (label) placed.push({ label, top: Math.max(box.top, 9), left: Math.max(box.left, 4) });
  });
  return placed;
}

/** Ctrl+Shift+S shows a number label on every element in the window; pressing it again hides them. */
export function ElementLabels() {
  const [placed, setPlaced] = useState<Placed[] | null>(null);
  const on = placed !== null;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isLabelShortcut(e)) {
        e.preventDefault();
        setPlaced((now) => (now ? null : measure()));
      } else if (e.key === "Escape" && on) {
        setPlaced(null);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [on]);

  // While shown, labels follow scrolling and resizing; they are given again in reading order.
  useEffect(() => {
    if (!on) return;
    let frame = 0;
    const again = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => setPlaced(measure()));
    };
    window.addEventListener("scroll", again, true);
    window.addEventListener("resize", again);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("scroll", again, true);
      window.removeEventListener("resize", again);
    };
  }, [on]);

  if (!placed) return null;
  return (
    <div className="element-labels" aria-hidden="true">
      {placed.map((p) => (
        <span key={p.label} className="element-label" style={{ top: p.top, left: p.left }}>
          {p.label}
        </span>
      ))}
    </div>
  );
}
