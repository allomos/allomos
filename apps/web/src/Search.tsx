// Searching the task list (Task-A01-36): the search box above the list, the marked words, and the
// handoffs a search finds. What matches is decided in searchFlow.ts.

import { forwardRef } from "react";
import { useI18n } from "./i18n";
import { localTime } from "./handoffFlow";
import { pieces, snippet, type Piece } from "./searchFlow";
import type { Words } from "./stopFlow";
import type { Handoff, Task } from "./types";

/** The shortcut that focuses the search box, as this computer writes it. */
const SHORTCUT = /Mac/.test(navigator.userAgent) ? "⌘F" : "Ctrl+F";

/** Is this key press the search shortcut, Cmd+F on a Mac and Ctrl+F elsewhere? */
export function isSearchShortcut(e: KeyboardEvent): boolean {
  return (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "f";
}

/** The search box. Esc empties it; Esc again leaves it. */
export const SearchBox = forwardRef<HTMLInputElement, { value: string; onChange: (v: string) => void }>(function SearchBox(
  { value, onChange },
  ref,
) {
  const { t } = useI18n();
  return (
    <span className="search">
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="8.5" cy="8.5" r="5.5" />
        <path d="M12.5 12.5 17 17" />
      </svg>
      <input
        ref={ref}
        type="search"
        value={value}
        placeholder={t("search.placeholder", { keys: SHORTCUT })}
        aria-label={t("search.label")}
        spellCheck={false}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Escape") return;
          e.preventDefault();
          if (value) onChange("");
          else e.currentTarget.blur();
        }}
      />
      {value && (
        <button type="button" className="search-clear" aria-label={t("search.clear")} title={t("search.clear")} onClick={() => onChange("")}>
          ×
        </button>
      )}
    </span>
  );
});

/** Stretches of text, the found words marked. */
export function Marked({ parts }: { parts: Piece[] }) {
  return (
    <>
      {parts.map((p, i) => (p.hit ? <mark key={i}>{p.text}</mark> : p.text))}
    </>
  );
}

/** `text` with the search words marked. */
export function Highlight({ text, terms }: { text: string; terms: string[] }) {
  return terms.length === 0 ? <>{text}</> : <Marked parts={pieces(text, terms)} />;
}

/** The line under a task found by its description: the words around the first match. */
export function DescriptionHit({ text, terms }: { text: string; terms: string[] }) {
  const { t } = useI18n();
  const parts = terms.length > 0 ? snippet(text, terms) : null;
  if (!parts) return null;
  return (
    <span className="search-snippet">
      <span className="search-where">{t("search.in_description")}</span> <Marked parts={parts} />
    </span>
  );
}

/** Handoffs a search found, below the tasks, marked as handoffs; clicking one opens it in the Handoffs tab. */
export function HandoffHits({ handoffs, terms, onOpen }: { handoffs: Handoff[]; terms: string[]; onOpen: (file: string) => void }) {
  const { t } = useI18n();
  if (handoffs.length === 0) return null;
  return (
    <section className="area search-handoffs" aria-label={t("search.handoffs")}>
      <header className="area-head">
        <h2>{t("search.handoffs")}</h2>
        <span className="count">{t("search.handoff_count", { count: handoffs.length })}</span>
      </header>
      <ul className="tasks">
        {handoffs.map((h) => {
          const body = snippet(h.body, terms);
          return (
            <li key={h.file} className="task handoff-hit" tabIndex={0} onClick={() => onOpen(h.file)} onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onOpen(h.file);
              }
            }}>
              <span className="kind">{t("search.handoff_kind")}</span>
              <span className="title">
                <Highlight text={h.summary || t("handoffs.empty")} terms={terms} />
              </span>
              <span className="when small">{localTime(h.updated)}</span>
              <span className="meta">
                <span>
                  <Highlight text={h.writer || t("handoffs.unknown_writer")} terms={terms} />
                </span>
                {h.tasks.length > 0 && (
                  <span>
                    <Highlight text={h.tasks.join(", ")} terms={terms} />
                  </span>
                )}
                {body && (
                  <span className="search-snippet">
                    <Marked parts={body} />
                  </span>
                )}
              </span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** Tasks a search finds that the state filter hides (Task-A01-42), set apart after the groups so a
 * filter left on cannot make a task look missing. Clicking one opens it in the task pane. */
export function FilterHits({
  tasks,
  terms,
  filter,
  selectedId,
  onSelect,
  onShowAll,
}: {
  tasks: Task[];
  terms: string[];
  /** The filter's name, as the Show box words it. */
  filter: string;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onShowAll: () => void;
}) {
  const { t } = useI18n();
  const heading = t("search.hidden_by_filter", { filter });
  return (
    <section className="area search-hidden" aria-label={heading}>
      {/* "3 found" is the title; under it, which filter hides them and Show all (approver, 2026-10-14). */}
      <header className="area-head">
        <h2>{t("search.handoff_count", { count: tasks.length })}</h2>
        <span className="sub">
          {heading}{" "}
          <button type="button" className="link small" onClick={onShowAll}>
            {t("filter.show_all")}
          </button>
        </span>
      </header>
      <ul className="tasks">
        {tasks.map((task) => (
          <li
            key={task.id}
            className={task.id === selectedId ? "task selected" : "task"}
            tabIndex={0}
            aria-current={task.id === selectedId ? "true" : undefined}
            onClick={() => onSelect(task.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelect(task.id);
              }
            }}
          >
            <span className="id">
              <Highlight text={task.id} terms={terms} />
            </span>
            <span className="title">
              <Highlight text={task.title} terms={terms} />
            </span>
            <span className="state-cell">
              <span className={`state s-${task.state}`}>{t(`state.${task.state}`)}</span>
            </span>
            <span className="meta">
              {task.label && <span className="label">{task.label}</span>}
              <DescriptionHit text={task.description ?? ""} terms={terms} />
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** A banner's words, the strong ones in bold. */
function Said({ words }: { words: Words }) {
  return (
    <>
      {words.map((w, i) => (w.strong ? <strong key={i}>{w.text}</strong> : w.text))}
    </>
  );
}

/** The banners under the tabs (Task-A01-44): what the search found, in a neutral tint, on top; what
 * the Show filter shows, in the accent (blue) tint, under it, ending with Show all. */
export function ListBanners({ search, filter, onShowAll }: { search: Words; filter: Words; onShowAll: () => void }) {
  const { t } = useI18n();
  if (search.length === 0 && filter.length === 0) return null;
  return (
    <div className="list-banners">
      {search.length > 0 && (
        <p className="list-banner search-banner" role="status">
          <span>
            <Said words={search} />
          </span>
        </p>
      )}
      {filter.length > 0 && (
        <p className="list-banner filter-banner" role="status">
          <span>
            <Said words={filter} />
          </span>
          <button type="button" className="small" onClick={onShowAll}>
            {t("filter.show_all")}
          </button>
        </p>
      )}
    </div>
  );
}
