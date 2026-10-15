import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { recentRows, type RecentInfoMap } from "./recentFlow";

/** The start page's **Open recent** dialog (Task-A01-43): the folders of File → Open Recent, newest
 * first, each with its project name, folder and when it was last opened. Clicking one opens it in
 * this window; × (shown on hover or keyboard focus) removes it from the recent list only. A folder
 * that no longer exists is greyed and can only be removed. Closes with ×, Esc or a click outside. */
export function RecentDialog({
  messenger,
  folders,
  info,
  onOpen,
  onForget,
  onClose,
}: {
  messenger: Messenger;
  folders: string[];
  info: RecentInfoMap;
  onOpen: (folder: string) => void;
  onForget: (folder: string) => void;
  onClose: () => void;
}) {
  const { t, language } = useI18n();
  const [missing, setMissing] = useState<ReadonlySet<string>>(new Set());
  const closeRef = useRef<HTMLButtonElement>(null);

  // Which folders are gone, asked when the dialog opens.
  useEffect(() => {
    let live = true;
    void Promise.all(
      folders.map(async (f) => ((await messenger.inspectFolder(f).catch(() => null))?.inspection.kind === "not_a_folder" ? f : null)),
    ).then((gone) => live && setMissing(new Set(gone.filter((f): f is string => f !== null))));
    return () => {
      live = false;
    };
  }, [messenger, folders]);

  // Esc closes it; the dialog's own × takes the focus when it opens.
  const closeNow = useRef(onClose);
  closeNow.current = onClose;
  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeNow.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const when = useMemo(() => {
    const format = new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" });
    return (iso: string) => format.format(new Date(iso));
  }, [language]);

  const rows = recentRows(folders, info, missing);
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog recent-dialog" role="dialog" aria-modal="true" aria-labelledby="recent-title">
        <div className="recent-head">
          <h2 id="recent-title">{t("recent.title")}</h2>
          <button type="button" ref={closeRef} className="close" aria-label={t("recent.close")} title={t("recent.close")} onClick={onClose}>
            ×
          </button>
        </div>
        {rows.length === 0 ? (
          <p className="muted">{t("recent.empty")}</p>
        ) : (
          <ul className="recent-list">
            {rows.map((r) => (
              <li key={r.folder} className={r.missing ? "recent-item missing" : "recent-item"}>
                <button type="button" className="recent-open" disabled={r.missing} onClick={() => onOpen(r.folder)}>
                  <strong>{r.name}</strong>
                  <code title={r.folder}>{r.folder}</code>
                  <span className="muted">
                    {r.missing ? t("recent.missing") : r.opened ? t("recent.last_opened", { when: when(r.opened) }) : ""}
                  </span>
                </button>
                <button
                  type="button"
                  className="recent-forget"
                  aria-label={t("recent.forget", { name: r.name })}
                  title={t("recent.forget", { name: r.name })}
                  onClick={() => onForget(r.folder)}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
