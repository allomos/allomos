// The task list's difficulty menu (Task-A01-pewter675): in a task's row, the difficulty ("4 · Hard")
// is a button; clicking it opens the levels 1 to 5, each with its name, the task's own ticked, and
// choosing one changes the difficulty at once, through the same change as the task pane's
// difficulty field and `alm difficulty` (the history records it). Built like the state menu
// (StateMenu.tsx): the person's name is asked once first, Esc or a click outside the list closes it
// and changes nothing, and nothing in it selects the task.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import { difficultyOf, difficultyText, levelItems, levelMenuEdit, levelText } from "./difficultyFlow";
import { WhoForm } from "./WhoForm";
import { isProblem, type Edit, type Task } from "./types";

/** Keeps a key or click inside the menu from reaching the row (which would select the task). */
const own = (e: { stopPropagation: () => void }) => e.stopPropagation();

export function DifficultyMenu({
  task,
  ownerName,
  onSaveName,
  onEdit,
}: {
  task: Task;
  /** The person's name for the history; null until it is asked (Task-A01-18). */
  ownerName: string | null;
  onSaveName: (name: string) => Promise<void>;
  /** Makes the change. Rejects with the Problem that refused it; then nothing changed. */
  onEdit: (edit: Edit) => Promise<void>;
}) {
  const { t, problem } = useI18n();
  const [open, setOpen] = useState(false);
  // A level chosen before the person's name was known: saved once the name is.
  const [pending, setPending] = useState<Edit | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const box = useRef<HTMLSpanElement>(null);

  const close = () => {
    setOpen(false);
    setPending(null);
    setError(null);
  };
  // Closes on a click outside the menu or on Esc, changing nothing.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const choose = async (level: number) => {
    if (busy) return;
    setError(null);
    const edit = levelMenuEdit(task, level);
    if (!edit) return close();
    if (!ownerName) return setPending(edit);
    setBusy(true);
    try {
      await onEdit(edit);
      close();
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const saveName = async () => {
    if (!name.trim() || busy || !pending) return;
    setBusy(true);
    try {
      await onSaveName(name.trim());
      await onEdit(pending);
      close();
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const { level: shown, isDefault } = difficultyOf(task);
  const text = difficultyText(task, t);

  return (
    <span
      className="state-menu"
      ref={box}
      onClick={own}
      onKeyDown={(e) => {
        // Stopped here, the key never reaches the document's listener: Esc closes from inside too.
        own(e);
        if (e.key === "Escape" && open) {
          e.preventDefault();
          close();
        }
      }}
    >
      <button
        type="button"
        className={`state-menu-button difficulty-menu-button${isDefault ? " muted" : ""}`}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("difficulty_menu.button_aria", { level: text })}
        title={t("difficulty_menu.button_title")}
        onClick={() => (open ? close() : setOpen(true))}
      >
        {text}
      </button>
      {open && (
        <div className="sort-popover state-popover difficulty-popover" role="menu" aria-label={t("difficulty_menu.label")}>
          {pending ? (
            <WhoForm
              name={name}
              onName={setName}
              explain="edit.name_explain"
              label="who.label_task"
              busy={busy}
              error={error}
              onSave={() => void saveName()}
              onCancel={close}
            />
          ) : (
            <>
              <div className="sort-heading">{t("difficulty_menu.label")}</div>
              {levelItems(task).map((item) => (
                <button
                  key={item.level}
                  type="button"
                  role="menuitemradio"
                  aria-checked={item.current}
                  disabled={busy}
                  // The task's own level takes the focus; without one, the level it counts as.
                  autoFocus={item.level === shown}
                  onClick={() => void choose(item.level)}
                >
                  {levelText(item.level, t)}
                </button>
              ))}
              {error && <p className="banner error small state-why">{error}</p>}
            </>
          )}
        </div>
      )}
    </span>
  );
}
