// The Iterations tab's sort menu (Task-A01-52): a compact button saying the sort in use, which
// opens a menu of what to sort by and which way. The order itself is decided in sortFlow.ts.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import { DEFAULT_SORT, SORT_KEYS, type Sort, type SortDir } from "./sortFlow";

export function SortMenu({ sort, onChange }: { sort: Sort; onChange: (s: Sort) => void }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement>(null);
  // Closes on a click outside the menu or on Esc.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);
  const dirText = (dir: SortDir) => t(`sort.${sort.key}_${dir}`);
  const changed = sort.key !== DEFAULT_SORT.key || sort.dir !== DEFAULT_SORT.dir;
  return (
    <span className="sort-menu" ref={box}>
      <button
        type="button"
        className={changed ? "sort-button on" : "sort-button"}
        aria-haspopup="menu"
        aria-expanded={open}
        title={t("sort.label")}
        onClick={() => setOpen((o) => !o)}
      >
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <path d="M6 4v12M3 13l3 3 3-3M14 16V4M11 7l3-3 3 3" />
        </svg>
        {t("sort.summary", { key: t(`sort.by_${sort.key}`), dir: dirText(sort.dir) })}
      </button>
      {open && (
        <div className="sort-popover" role="menu" aria-label={t("sort.label")}>
          <div className="sort-heading">{t("sort.by")}</div>
          {SORT_KEYS.map((key) => (
            <button
              key={key}
              type="button"
              role="menuitemradio"
              aria-checked={sort.key === key}
              onClick={() => onChange({ ...sort, key })}
            >
              {t(`sort.by_${key}`)}
            </button>
          ))}
          <hr />
          {(["asc", "desc"] as const).map((dir) => (
            <button
              key={dir}
              type="button"
              role="menuitemradio"
              aria-checked={sort.dir === dir}
              onClick={() => onChange({ ...sort, dir })}
            >
              {dirText(dir)}
            </button>
          ))}
        </div>
      )}
    </span>
  );
}
