// The task list's state menu (Task-A01-frost005): in a task's row, the state word is a button;
// clicking it opens the list of states, and choosing one changes the task's state at once, through
// the same change and the same rules as the task pane's state menu (editFlow.ts): a greyed-out
// state says why, Waiting asks what it waits on, and the person's name is asked once first. Esc or
// a click outside the list closes it and changes nothing. Nothing in it selects the task.

import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { useI18n, useTaskT } from "./i18n";
import { AnchoredPopover } from "./AnchoredPopover";
import { stateItems, stateMenuEdit, type StateWhy } from "./editFlow";
import { isPlainEnter } from "./enterKey";
import { WhoForm } from "./WhoForm";
import { isProblem, type Edit, type State, type Task } from "./types";

/** Keeps a key or click inside the menu from reaching the row (which would select the task). */
const own = (e: { stopPropagation: () => void }) => e.stopPropagation();

export function StateMenu({
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
  const taskT = useTaskT();
  const [open, setOpen] = useState(false);
  // Waiting was chosen: what it waits on and who, written before it is saved.
  const [waiting, setWaiting] = useState<{ on: string; who: string } | null>(null);
  // A state chosen before the person's name was known: saved once the name is.
  const [pending, setPending] = useState<Edit | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const box = useRef<HTMLSpanElement>(null);
  const popover = useRef<HTMLDivElement>(null);

  const close = () => {
    setOpen(false);
    setWaiting(null);
    setPending(null);
    setError(null);
  };
  // Closes on a click outside the menu or on Esc, changing nothing.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (!box.current?.contains(target) && !popover.current?.contains(target)) close();
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
  const send = async (edit: Edit) => {
    if (busy) return;
    if (!ownerName) {
      setPending(edit);
      return;
    }
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
  const choose = (s: State) => {
    setError(null);
    if (s === "waiting" && task.state !== "waiting") {
      setWaiting({ on: "", who: "" });
      return;
    }
    const edit = stateMenuEdit(task, s);
    if (edit) void send(edit);
    else close();
  };
  const waitingEdit = waiting ? stateMenuEdit(task, "waiting", waiting.on, waiting.who) : null;
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
  const whyText = (why: StateWhy) =>
    why.why === "claim" ? t("state_menu.in_progress_note") : why.why === "held" ? t("edit.backlog_held", { holder: why.holder }) : t("edit.backlog_finished");
  const waitingKeys = (e: ReactKeyboardEvent) => {
    if (isPlainEnter(e.nativeEvent)) {
      e.preventDefault();
      if (waitingEdit) void send(waitingEdit);
    }
  };
  const stateText = taskT(`state.${task.state}`);

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
        className={`fact-state state-word tint s-${task.state}-text state-menu-button`}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("state_menu.button_aria", { state: stateText })}
        title={t("state_menu.button_title")}
        onClick={() => (open ? close() : setOpen(true))}
      >
        {stateText}
      </button>
      {open && (
        <AnchoredPopover
          anchorRef={box}
          popoverRef={popover}
          className="sort-popover state-popover"
          label={t("state_menu.label")}
        >
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
              <div className="sort-heading">{t("state_menu.label")}</div>
              {stateItems(task).map((item) => (
                <div key={item.state} className="state-item">
                  <button
                    type="button"
                    role="menuitemradio"
                    aria-checked={item.current}
                    disabled={item.disabled || busy}
                    autoFocus={item.current}
                    onClick={() => choose(item.state)}
                  >
                    <span className={`s-${item.state}-text`}>{taskT(`state.${item.state}`)}</span>
                  </button>
                  {item.why && <p className="muted small state-why">{whyText(item.why)}</p>}
                </div>
              ))}
              {waiting && (
                <div className="state-waiting">
                  <input
                    type="text"
                    value={waiting.on}
                    placeholder={t("edit.waiting_on_placeholder")}
                    aria-label={t("pane.waiting_on")}
                    onChange={(e) => setWaiting({ ...waiting, on: e.target.value })}
                    onKeyDown={waitingKeys}
                    disabled={busy}
                    autoFocus
                  />
                  <input
                    type="text"
                    value={waiting.who}
                    placeholder={t("edit.who_placeholder")}
                    aria-label={t("pane.who")}
                    onChange={(e) => setWaiting({ ...waiting, who: e.target.value })}
                    onKeyDown={waitingKeys}
                    disabled={busy}
                  />
                  <div className="editor-buttons">
                    <button type="button" className="primary" onClick={() => waitingEdit && void send(waitingEdit)} disabled={busy || !waitingEdit}>
                      {busy ? t("edit.saving") : t("edit.save")}
                    </button>
                    <button type="button" onClick={close} disabled={busy}>
                      {t("edit.cancel")}
                    </button>
                  </div>
                </div>
              )}
              {error && <p className="banner error small state-why">{error}</p>}
            </>
          )}
        </AnchoredPopover>
      )}
    </span>
  );
}
