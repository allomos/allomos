// The task list's sort menu (Task-A01-52; in the By area tab too, Task-A01-plum588): a compact
// button saying the sort in use, which opens a menu of what to sort by and which way. The order
// itself is decided in sortFlow.ts. Under the sort, a Show section turns the dependency tree on and
// off (Task-A01-alpine458): a way of showing the list, not a sort, so the sort stays as chosen.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import { DEFAULT_SORT, SORT_KEYS, type Sort, type SortDir } from "./sortFlow";

export function SortMenu({
  sort,
  onChange,
  tree,
  onTree,
  stale,
  onReorder,
}: {
  sort: Sort;
  onChange: (s: Sort) => void;
  /** Whether the dependency tree is on. */
  tree: boolean;
  onTree: (on: boolean) => void;
  /** A local edit changed the modified-time order; the person can apply it when ready. */
  stale: boolean;
  onReorder: () => void;
}) {
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
      {stale && (
        <button type="button" className="link small sort-reorder" onClick={onReorder}>
          {t("sort.reorder")}
        </button>
      )}
      {tree && (
        <span className="sort-tag" title={t("sort.dep_tree_title")}>
          {t("sort.dep_tree")}
        </span>
      )}
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
          <hr />
          <div className="sort-heading">{t("sort.show")}</div>
          <button type="button" role="menuitemcheckbox" aria-checked={tree} title={t("sort.dep_tree_title")} onClick={() => onTree(!tree)}>
            {t("sort.dep_tree")}
          </button>
        </div>
      )}
    </span>
  );
}
