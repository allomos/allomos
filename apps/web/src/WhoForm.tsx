// The one-time "who are you" step (Task-A01-40): before the person's first change or addition,
// Allomos asks for their own name, to record it in the history. It is a box of its own, with its
// own heading and label, so it never reads as the name of the thing being changed. Once the name
// is saved, the change the person started goes on.

import { useState } from "react";
import { useI18n } from "./i18n";
import { isProblem } from "./types";

export function WhoForm({
  name,
  onName,
  explain,
  label,
  busy,
  error,
  onSave,
  onCancel,
}: {
  name: string;
  onName: (name: string) => void;
  /** Why Allomos asks: what is recorded where. */
  explain: "edit.name_explain" | "create.name_explain";
  /** The box's label: a task's history, or the history. */
  label: "who.label_task" | "who.label";
  busy: boolean;
  error: string | null;
  onSave: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  return (
    <section className="who-form" aria-label={t("who.title")}>
      <h4>{t("who.title")}</h4>
      <p className="small">{t(explain)}</p>
      <label>
        <span>{t(label)}</span>
        <input
          type="text"
          value={name}
          placeholder={t("edit.name_placeholder")}
          onChange={(e) => onName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              e.stopPropagation();
              if (name.trim() && !busy) onSave();
            } else if (e.key === "Escape") {
              e.preventDefault();
              e.stopPropagation();
              onCancel();
            }
          }}
          disabled={busy}
          autoFocus
        />
      </label>
      <div className="editor-buttons">
        <button type="button" className="primary" onClick={onSave} disabled={busy || !name.trim()}>
          {t("edit.name_save")}
        </button>
        <button type="button" onClick={onCancel} disabled={busy}>
          {t("edit.cancel")}
        </button>
      </div>
      {error && <p className="banner error small">{error}</p>}
    </section>
  );
}

/** The saved name, in the list's toolbar (Task-A01-40): who changes are recorded as, and a way to
 * change it (or set it before the first change). */
export function WhoLine({ name, onSave }: { name: string | null; onSave: (name: string) => Promise<void> }) {
  const { t, problem } = useI18n();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    if (!draft.trim() || busy) return;
    setBusy(true);
    try {
      await onSave(draft.trim());
      setError(null);
      setEditing(false);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };
  if (!editing) {
    return (
      <span className="who-line small">
        {name ? t("who.line", { name }) : t("who.line_none")}{" "}
        <button
          type="button"
          className="link small"
          onClick={() => {
            setDraft(name ?? "");
            setError(null);
            setEditing(true);
          }}
        >
          {t(name ? "who.change" : "who.set")}
        </button>
      </span>
    );
  }
  return (
    <span className="who-line small editing">
      <label>
        <span>{t("who.label")}</span>{" "}
        <input
          type="text"
          value={draft}
          placeholder={t("edit.name_placeholder")}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void save();
            } else if (e.key === "Escape") {
              e.preventDefault();
              setEditing(false);
            }
          }}
          disabled={busy}
          autoFocus
        />
      </label>
      <button type="button" className="primary" onClick={() => void save()} disabled={busy || !draft.trim()}>
        {t("edit.name_save")}
      </button>
      <button type="button" onClick={() => setEditing(false)} disabled={busy}>
        {t("edit.cancel")}
      </button>
      {error && <span className="banner error small">{error}</span>}
    </span>
  );
}
