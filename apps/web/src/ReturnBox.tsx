// Send back… (Task-A01-music456, decision 0030 § 4; was Reopen…, Task-A01-50): at the bottom of the
// Evidence screen and in the task pane of a finished task. The person writes why (required); on the
// desktop app they may attach a screenshot of this window, kept in the task's evidence folder (one
// of its 3, decision 0029 § 6) and named at the end of the reason. Disagreeing with one AI decision
// opens it with that decision's words filled in. One task at a time: each reason is its own.
// The task goes back to ready, held by nobody; the next session reads the reason first (§ 5, 6).

import { useState, type ReactNode } from "react";
import { useI18n } from "./i18n";
import { onlyFilledIn, reopenEdit } from "./reopenFlow";
import type { Edit, Task } from "./types";

export function ReturnBox({
  task,
  reason,
  filled,
  onOpen,
  onClose,
  onReason,
  onEdit,
  capture,
  nameForm,
  describe,
}: {
  task: Task;
  /** The reason being written; null while the form is closed. */
  reason: string | null;
  /** What the box was opened with (a disagreed AI decision's words); null: opened empty. */
  filled: string | null;
  onOpen: () => void;
  onClose: () => void;
  onReason: (text: string) => void;
  onEdit: (edit: Edit) => Promise<void>;
  /** Captures this window into the task's evidence folder, answering the kept name; left out
   * where it cannot be done (the companions), and then the choice is not offered. */
  capture?: (task: string) => Promise<string>;
  /** The form asking the person's name, when the app has none yet. */
  nameForm: (() => ReactNode) | null;
  /** Turns a refusal into the person's words. */
  describe: (e: unknown) => string;
}) {
  const { t } = useI18n();
  const [attach, setAttach] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [kept, setKept] = useState<{ task: string; file: string } | null>(null);
  const close = () => {
    setAttach(false);
    setError(null);
    setKept(null);
    onClose();
  };
  const ready = reason !== null && reopenEdit(reason) !== null && !onlyFilledIn(reason, filled);

  const send = async () => {
    if (!ready || busy || reason === null) return;
    setBusy(true);
    setError(null);
    try {
      // The picture first, while the screen still shows what was wrong; kept once if the
      // sending is tried again.
      let shot = kept;
      if (attach && capture && !shot) {
        shot = { task: task.id, file: await capture(task.id) };
        setKept(shot);
      }
      const edit = reopenEdit(reason, attach ? shot ?? undefined : undefined);
      if (edit) await onEdit(edit);
      setAttach(false);
      setKept(null);
      onClose();
    } catch (e) {
      setError(describe(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="pane-check pane-reopen">
      {reason === null ? (
        <button type="button" onClick={onOpen} disabled={busy}>
          {t("pane.reopen_button")}
        </button>
      ) : nameForm ? (
        nameForm()
      ) : (
        <div className="editor">
          <label className="small" htmlFor={`return-${task.id}`}>
            {t("pane.reopen_note")}
          </label>
          <textarea
            id={`return-${task.id}`}
            value={reason}
            rows={filled ? 6 : 3}
            placeholder={t("pane.reopen_placeholder")}
            onChange={(e) => onReason(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                close();
              }
            }}
            disabled={busy}
            autoFocus
          />
          {capture && (
            <label className="small return-shot">
              <input type="checkbox" checked={attach} onChange={(e) => setAttach(e.target.checked)} disabled={busy} />{" "}
              {kept ? t("pane.return_shot_kept", { file: kept.file }) : t("pane.return_shot")}
            </label>
          )}
          <div className="editor-buttons">
            <button type="button" className="primary" onClick={() => void send()} disabled={busy || !ready}>
              {busy ? t("edit.saving") : t("pane.reopen_save")}
            </button>
            <button type="button" onClick={close} disabled={busy}>
              {t("edit.cancel")}
            </button>
          </div>
          {error && <p className="banner error small">{error}</p>}
        </div>
      )}
      <p className="muted small">{t("pane.reopen_hint")}</p>
    </div>
  );
}
