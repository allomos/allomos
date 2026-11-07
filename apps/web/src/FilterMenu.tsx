// The task list's Show filter (Task-A01-lens781): a button saying what is hidden, which opens a
// panel of ticks in two groups, states and labels, each label with a Yes and a No; each group has
// Select all and Clear all, grey when there is nothing for them to do, and a ↻ beside the button
// ticks everything again. The Labels column starts with Labeled (Task-A01-teacup726), then every
// label of the project's label table, built-in first (decision 0020), indented under it and greyed
// out while its Yes is unticked. What the ticks keep is decided in stopFlow.ts.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import {
  FILTER_STATES,
  allTicked,
  emptyBy,
  filterName,
  groupTicks,
  isAll,
  labelsOff,
  labelText,
  tickGroup,
  tickLabel,
  tickLabeled,
  tickState,
  type Side,
  type StateFilter,
} from "./stopFlow";

export function FilterMenu({ filter, onChange }: { filter: StateFilter; onChange: (f: StateFilter) => void }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement>(null);
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
  const empty = emptyBy(filter);
  const groupButtons = (group: "states" | "labels") => {
    const ticks = groupTicks(filter, group);
    return (
      <span className="filter-group-buttons">
        <button type="button" disabled={ticks.all} onClick={() => onChange(tickGroup(filter, group, true))}>
          {t("filter.select_all")}
        </button>
        <button type="button" disabled={ticks.none} onClick={() => onChange(tickGroup(filter, group, false))}>
          {t("filter.clear_all")}
        </button>
      </span>
    );
  };
  const greyed = labelsOff(filter);
  const side = (label: string, s: Side) => (
    <label className="filter-side">
      <input
        type="checkbox"
        checked={!filter[s].includes(label)}
        disabled={greyed}
        onChange={(e) => onChange(tickLabel(filter, label, s, e.target.checked))}
      />
      {t(s === "yes" ? "filter.yes" : "filter.no")}
    </label>
  );
  const labeledSide = (s: Side) => (
    <label className="filter-side">
      <input type="checkbox" checked={!filter.labeled.includes(s)} onChange={(e) => onChange(tickLabeled(filter, s, e.target.checked))} />
      {t(s === "yes" ? "filter.yes" : "filter.no")}
    </label>
  );
  return (
    <span className="filter-menu" ref={box}>
      <span className="state-filter small">{t("filter.label")}</span>
      <button
        type="button"
        className={isAll(filter) ? "filter-button" : "filter-button on"}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span>{filterName(filter, t)}</span>
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
      {open && (
        <div className="filter-popover" role="dialog" aria-label={t("filter.label")}>
          <div className="filter-columns">
            <div className="filter-column">
              <div className="filter-heading">
                <span>{t("filter.states")}</span>
                {groupButtons("states")}
              </div>
              {FILTER_STATES.map((s) => (
                <label key={s} className="filter-row">
                  <input
                    type="checkbox"
                    checked={!filter.states.includes(s)}
                    onChange={(e) => onChange(tickState(filter, s, e.target.checked))}
                  />
                  <span className={`state-word s-${s}-text`}>{t(`state.${s}`)}</span>
                </label>
              ))}
            </div>
            {filter.labels.length > 0 && (
              <div className="filter-column">
                <div className="filter-heading">
                  <span title={t("filter.labels_hint")}>{t("filter.labels")}</span>
                  {groupButtons("labels")}
                </div>
                <div className="filter-row filter-label-row">
                  <span className="filter-label-name" title={t("filter.labeled_hint")}>
                    {t("filter.labeled")}
                  </span>
                  {labeledSide("yes")}
                  {labeledSide("no")}
                </div>
                {filter.labels.map((label) => (
                  <div key={label} className={greyed ? "filter-row filter-label-row filter-child off" : "filter-row filter-label-row filter-child"}>
                    <span className="filter-label-name">{labelText(label, t)}</span>
                    {side(label, "yes")}
                    {side(label, "no")}
                  </div>
                ))}
              </div>
            )}
          </div>
          {empty && (
            <p className="filter-empty small">
              {empty.kind === "states"
                ? t("filter.empty_states")
                : empty.kind === "labeled"
                  ? t("filter.empty_labeled")
                  : t("filter.empty_label", { label: labelText(empty.label, t) })}
            </p>
          )}
        </div>
      )}
    </span>
  );
}
