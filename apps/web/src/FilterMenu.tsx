// The task list's Show filter (Task-A01-lens781, Task-A01-rock075): a button saying what is hidden,
// which opens a grid of ticks: one row per state, tinted with that state's colour, and across it a
// Yes and a No for "Checked by a human" (both "by human" labels) and each other label of the
// project's label table, built-in first (decision 0020). The tick before a state sums up its row
// (all, some, none) and ticks or unticks the whole row; Select all and Clear all do the whole grid,
// grey when there is nothing for them to do, and a ↻ beside the button ticks everything again.
// The panel has two tabs (Task-A01-wigwam308): Presets, each only setting the grid's ticks, with
// what it ticks and how many tasks it lists now (✓ when the ticks are exactly its), and Custom,
// the grid. A × at the right of the tabs (Task-A01-bread766) closes the panel and leaves the ticks
// as they are, as clicking the button again, a click outside and Esc do. What the ticks keep is
// decided in stopFlow.ts.

import { Fragment, useEffect, useRef, useState } from "react";
import { useI18n, useTaskT } from "./i18n";
import {
  FILTER_STATES,
  HUMAN_COLUMN,
  SIDES,
  allTicked,
  columnText,
  emptyBy,
  filterColumns,
  filterName,
  gridTicks,
  isAll,
  isTicked,
  matchPreset,
  PRESETS,
  presetCount,
  presetFilter,
  presetName,
  presetTicks,
  rowTicks,
  tickAll,
  tickCell,
  tickRow,
  type StateFilter,
} from "./stopFlow";
import type { Task } from "./types";

type Tab = "presets" | "custom";

export function FilterMenu({
  filter,
  tasks,
  onChange,
}: {
  filter: StateFilter;
  /** The tasks the list draws from (the Folder filter applied), for each preset's count. */
  tasks: readonly Task[];
  onChange: (f: StateFilter) => void;
}) {
  const { t } = useI18n();
  const taskT = useTaskT();
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<Tab>("presets");
  const box = useRef<HTMLSpanElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  // Closes on a click outside the panel or on Esc.
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
  const columns = filterColumns(filter);
  const grid = open ? gridTicks(filter) : null;
  const preset = matchPreset(filter);
  // The panel opens on Presets while the ticks are a preset's, else on Custom.
  const toggle = () => {
    if (!open) setTab(preset ? "presets" : "custom");
    setOpen((o) => !o);
  };
  return (
    <span className="filter-menu" ref={box}>
      <span className="state-filter small">{t("filter.label")}</span>
      <button
        type="button"
        className={isAll(filter) ? "filter-button" : "filter-button on"}
        aria-haspopup="dialog"
        aria-expanded={open}
        ref={button}
        onClick={toggle}
      >
        <span>{filterName(filter, t, taskT)}</span>
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <path d="M5 8l5 5 5-5" />
        </svg>
      </button>
      <button
        type="button"
        className="filter-restore"
        disabled={isAll(filter)}
        aria-label={t("filter.restore")}
        title={t("filter.restore")}
        onClick={() => onChange(allTicked(filter.labels))}
      >
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <path d="M15.5 10a5.5 5.5 0 1 1-1.6-3.9M15.5 4v3h-3" />
        </svg>
      </button>
      {open && grid && (
        <div className="filter-popover" role="dialog" aria-label={t("filter.label")}>
          <div className="filter-tabs" role="tablist">
            {(["presets", "custom"] as const).map((k) => (
              <button
                key={k}
                type="button"
                role="tab"
                aria-selected={tab === k}
                className={tab === k ? "on" : undefined}
                onClick={() => setTab(k)}
              >
                {t(`filter.tab_${k}`)}
              </button>
            ))}
            <button
              type="button"
              className="filter-close"
              aria-label={t("filter.close")}
              title={t("filter.close")}
              onClick={() => {
                setOpen(false);
                button.current?.focus();
              }}
            >
              ×
            </button>
          </div>
          {tab === "presets" ? (
            <div className="filter-presets" role="tabpanel">
              <p className="filter-heading">
                <span>{t("filter.presets_hint")}</span>
              </p>
              {PRESETS.map((p) => {
                const n = presetCount(p, filter.labels, tasks);
                return (
                  <button
                    key={p}
                    type="button"
                    className={p === preset ? "filter-preset on" : "filter-preset"}
                    aria-pressed={p === preset}
                    onClick={() => onChange(presetFilter(p, filter.labels))}
                  >
                    <span className="filter-preset-mark" aria-hidden="true">
                      {p === preset ? "✓" : ""}
                    </span>
                    <span className="filter-preset-words">
                      <span className="filter-preset-name">{presetName(p, t)}</span>
                      <span className="filter-preset-ticks">{presetTicks(p, t, taskT)}</span>
                    </span>
                    <span className="filter-preset-count">{t(n === 1 ? "filter.who_one" : "filter.who_other", { count: n })}</span>
                  </button>
                );
              })}
            </div>
          ) : (
            <div role="tabpanel">
            <div className="filter-heading">
              <span>{t("filter.grid_hint")}</span>
              <span className="filter-group-buttons">
                <button type="button" disabled={grid.all} onClick={() => onChange(tickAll(filter, true))}>
                  {t("filter.select_all")}
                </button>
                <button type="button" disabled={grid.none} onClick={() => onChange(tickAll(filter, false))}>
                  {t("filter.clear_all")}
                </button>
              </span>
            </div>
            <div className="filter-grid-scroll">
              <table className="filter-grid">
                <thead>
                  <tr>
                    <th className="st" rowSpan={2}>
                      {t("filter.states")}
                    </th>
                    {columns.map((c) => (
                      <th key={c} colSpan={2} className="lab" title={c === HUMAN_COLUMN ? t("filter.human_hint") : undefined}>
                        {columnText(c, t, taskT)}
                      </th>
                    ))}
                  </tr>
                  <tr>
                    {columns.map((c) => (
                      <Fragment key={c}>
                        <th className="yn">{t("filter.yes")}</th>
                        <th className="yn">{t("filter.no")}</th>
                      </Fragment>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {FILTER_STATES.map((s) => {
                    const ticks = rowTicks(filter, s);
                    const state = taskT(`state.${s}`);
                    return (
                      <tr key={s} className={`r-${s}`}>
                        <td className="st">
                          <label title={t("filter.row_hint")}>
                            <input
                              type="checkbox"
                              checked={ticks === "all"}
                              ref={(el) => {
                                if (el) el.indeterminate = ticks === "some";
                              }}
                              onChange={(e) => onChange(tickRow(filter, s, e.target.checked))}
                            />
                            <span className={`state-word s-${s}-text`}>{state}</span>
                          </label>
                        </td>
                        {columns.map((c) =>
                          SIDES.map((side) => (
                            <td key={`${c}\u0000${side}`} className={side}>
                              <input
                                type="checkbox"
                                aria-label={t("filter.cell", { state, cell: t(`filter.side_${side}`, { label: columnText(c, t, taskT) }) })}
                                checked={isTicked(filter, s, c, side)}
                                onChange={(e) => onChange(tickCell(filter, s, c, side, e.target.checked))}
                              />
                            </td>
                          )),
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            {emptyBy(filter) && <p className="filter-empty small">{t("filter.empty_all")}</p>}
            </div>
          )}
        </div>
      )}
    </span>
  );
}
