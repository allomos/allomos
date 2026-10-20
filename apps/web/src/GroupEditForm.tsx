// The Edit form of an area or iteration group in the list (Task-A01-29): rename an area; change an
// iteration's name, release and dates; delete either. Rust makes each change by the same rules as
// `alm rename area`, `alm change iteration` and `alm delete`, and a refusal (for example a group
// that still has tasks) is shown in its words. The person's name is asked once first.

import { useState, type KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import { WhoForm } from "./WhoForm";
import { canSaveGroup, startGroupEdit, type Editing, type GroupDraft } from "./groupEditFlow";
import { isProblem, type Area, type Iteration } from "./types";

export function GroupEditForm({
  editing,
  item,
  ownerName,
  onSaveName,
  onSave,
  onDelete,
  onClose,
}: {
  editing: Editing;
  /** The area or iteration as it is now. */
  item: Area | Iteration;
  /** The person's name for the history; null until asked. */
  ownerName: string | null;
  /** Keeps the person's name. Rejects with a Problem. */
  onSaveName: (name: string) => Promise<void>;
  /** Saves the change. Rejects with the Problem that refused it; then nothing changed. */
  onSave: (draft: GroupDraft) => Promise<void>;
  /** Deletes the group. Rejects with the Problem that refused it; then nothing changed. */
  onDelete: () => Promise<void>;
  onClose: () => void;
}) {
  const { t, problem } = useI18n();
  const [draft, setDraft] = useState<GroupDraft>(() => startGroupEdit(item));
  const [name, setName] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState<"save" | "delete" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const area = editing.kind === "area";

  const run = async (what: "save" | "delete", action: () => Promise<void>) => {
    if (busy) return;
    setBusy(what);
    try {
      await action();
      setError(null);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      setConfirming(false);
    } finally {
      setBusy(null);
    }
  };
  const save = () => {
    if (canSaveGroup(editing.kind, draft, item)) void run("save", () => onSave(draft));
  };
  const keys = (e: KeyboardEvent, enter: () => void) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      enter();
    }
  };
  const title = t(area ? "group_edit.edit_area" : "group_edit.edit_iteration", { id: item.id });

  if (!ownerName) {
    return (
      <div className="create-form editor" aria-label={title}>
        <h3>{title}</h3>
        <WhoForm
          name={name}
          onName={setName}
          explain="edit.name_explain"
          label="who.label"
          busy={!!busy}
          error={error}
          onSave={() => void run("save", () => onSaveName(name.trim()))}
          onCancel={onClose}
        />
      </div>
    );
  }

  const field = (key: keyof GroupDraft, label: string, type: "text" | "date", extra?: { placeholder?: string; autoFocus?: boolean }) => (
    <label>
      <span>{label}</span>
      <input type={type} value={draft[key]} onChange={(e) => setDraft((d) => ({ ...d, [key]: e.target.value }))} disabled={!!busy} {...extra} />
    </label>
  );

  return (
    <form className="create-form editor" aria-label={title} onSubmit={(e) => e.preventDefault()} onKeyDown={(e) => keys(e, save)}>
      <h3>{title}</h3>
      <p className="muted small">{t("group_edit.id_stays", { id: item.id })}</p>
      {field("name", t("create.name"), "text", { autoFocus: true })}
      {!area && (
        <>
          {field("release", t("create.release"), "text", { placeholder: "v0.3" })}
          {field("start", t("create.start"), "date")}
          {field("end", t("create.end"), "date")}
        </>
      )}
      <div className="editor-buttons">
        <button type="button" className="primary" onClick={save} disabled={!!busy || !canSaveGroup(editing.kind, draft, item)}>
          {busy === "save" ? t("edit.saving") : t("edit.save")}
        </button>
        <button type="button" onClick={onClose} disabled={!!busy}>
          {t("edit.cancel")}
        </button>
        {!confirming && (
          <button type="button" className="delete-group" onClick={() => setConfirming(true)} disabled={!!busy}>
            {t("group_edit.delete")}
          </button>
        )}
      </div>
      {confirming && (
        <div className="delete-confirm">
          <p className="small">{t(area ? "group_edit.delete_area_confirm" : "group_edit.delete_iteration_confirm", { id: item.id })}</p>
          <div className="editor-buttons">
            <button type="button" className="delete-group" onClick={() => void run("delete", onDelete)} disabled={!!busy}>
              {busy === "delete" ? t("group_edit.deleting") : t("group_edit.delete_yes")}
            </button>
            <button type="button" onClick={() => setConfirming(false)} disabled={!!busy}>
              {t("group_edit.keep")}
            </button>
          </div>
        </div>
      )}
      {error && <p className="banner error small">{error}</p>}
    </form>
  );
}
