// The searchable language menu (Task-A05-board831, approver 2026-10-26, on an interactive mockup):
// one component wherever a language is picked (Settings → Tasks, Translate the taskbag…, and its
// sister tasks). It opens in place under the field: a search box at its top filters by the
// language's own or English name (an exact code matches too); with the box empty the place's own
// first entry comes first, then the interface languages, then every other language A to Z. ↑ and
// ↓ move, Enter picks, Esc closes.

import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import { allLanguages, languageLabel, searchLanguages, type LanguageOption } from "./languageFlow";

/** The place's own first entry, such as Follow the interface or Don't translate. */
export interface FirstEntry {
  value: string;
  label: string;
}

interface Row {
  value: string;
  own: string;
  english?: string;
}

export function LanguageMenu({
  value,
  first,
  onChange,
  label,
  disabled,
}: {
  /** The chosen code, or the first entry's value; null: nothing chosen yet. */
  value: string | null;
  first?: FirstEntry;
  /** `name` is the language's own name, as the field shows it. */
  onChange: (value: string, name: string) => void;
  /** The field's accessible name. */
  label: string;
  disabled?: boolean;
}) {
  const { t } = useI18n();
  const id = useId();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLUListElement>(null);

  const rows = useMemo<Row[]>(() => {
    const found: Row[] = searchLanguages(allLanguages(), query).map((o: LanguageOption) => ({
      value: o.code,
      own: o.own,
      english: o.english !== o.own ? o.english : undefined,
    }));
    return first && !query.trim() ? [{ value: first.value, own: first.label }, ...found] : found;
  }, [query, first]);

  // A click outside closes it.
  useEffect(() => {
    if (!open) return;
    const away = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [open]);
  // The active row stays in view.
  useEffect(() => {
    list.current?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [active, open]);

  /** Opening starts with an empty search, on the chosen row (else the first). */
  const start = () => {
    setQuery("");
    const chosen = [...(first ? [first.value] : []), ...allLanguages().map((o) => o.code)].indexOf(value ?? "");
    setActive(Math.max(0, chosen));
    setOpen(true);
  };
  const pick = (row: Row) => {
    setOpen(false);
    onChange(row.value, row.own);
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(rows.length - 1, a + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(0, a - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (rows[active]) pick(rows[active]);
    } else if (e.key === "Escape") {
      // Only the menu closes, not the dialog it sits in.
      e.preventDefault();
      e.stopPropagation();
      e.nativeEvent.stopImmediatePropagation();
      setOpen(false);
    }
  };

  const shown = value === null ? t("language_menu.choose") : first && value === first.value ? first.label : languageLabel(value);

  return (
    <div className="language-menu" ref={root}>
      <button
        type="button"
        className="language-field"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : start())}
      >
        <span className={value === null ? "muted" : undefined}>{shown}</span>
        <span aria-hidden="true">▾</span>
      </button>
      {open && (
        <div className="language-drop" data-own-escape="">
          <input
            type="search"
            role="combobox"
            aria-label={t("language_menu.search")}
            aria-expanded="true"
            aria-controls={`${id}-list`}
            aria-activedescendant={rows[active] ? `${id}-${active}` : undefined}
            placeholder={t("language_menu.search")}
            value={query}
            autoFocus
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKey}
          />
          <ul id={`${id}-list`} role="listbox" aria-label={label} ref={list}>
            {rows.length === 0 && <li className="muted small none">{t("language_menu.none")}</li>}
            {rows.map((row, i) => (
              <li
                key={row.value}
                id={`${id}-${i}`}
                data-index={i}
                role="option"
                aria-selected={row.value === value}
                className={i === active ? "active" : undefined}
                onMouseEnter={() => setActive(i)}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pick(row)}
              >
                <span>{row.own}</span>
                {row.english && <span className="muted small">{row.english}</span>}
              </li>
            ))}
          </ul>
          <p className="muted small hint">{t("language_menu.hint")}</p>
        </div>
      )}
    </div>
  );
}
