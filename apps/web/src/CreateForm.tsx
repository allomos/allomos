// The New task, New area and New iteration forms in the list (Task-A01-19). Each suggests the next
// free ID, which the person may change; Rust adds the item by the same rules as `alm add`, and a
// refusal is shown in its words. The person's name is asked once first, as in the task pane.

import { useEffect, useState, type KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import { canAdd, pickArea, refreshItem, refreshTask, startItem, startTask, toNewIteration, toNewTask, type BaseChoice, type Creating, type ItemDraft, type TaskDraft } from "./createFlow";
import { isProblem, type Area, type Iteration, type NewIteration, type NewTask, type NextIds } from "./types";

export type AddRequest =
  | { kind: "task"; task: NewTask }
  | { kind: "area"; id: string | null; name: string }
  | { kind: "iteration"; iteration: NewIteration };

export function CreateForm({
  creating,
  areas,
  iterations,
  next,
  base,
  ownerName,
  onSaveName,
  onAdd,
  onClose,
}: {
  creating: Creating;
  areas: Area[];
  iterations: Iteration[];
  /** The new task's base branch, from the open tasks' bases (Task-A03-22). */
  base: BaseChoice;
  /** The suggested IDs; null until read. */
  next: NextIds | null;
  /** The person's name for the history; null until asked. */
  ownerName: string | null;
  /** Keeps the person's name. Rejects with a Problem. */
  onSaveName: (name: string) => Promise<void>;
  /** Adds the item. Rejects with the Problem that refused it; then nothing was added. */
  onAdd: (request: AddRequest) => Promise<void>;
  onClose: () => void;
}) {
  const { t, problem } = useI18n();
  const areaIds = areas.map((a) => a.id);
  const [task, setTask] = useState<TaskDraft>(() =>
    creating.kind === "task" ? startTask(creating, areaIds, next, base) : startTask({ kind: "task", group: "", area: null, iteration: null }, [], null),
  );
  const itemKind = creating.kind === "area" ? "area" : "iteration";
  const [item, setItem] = useState<ItemDraft>(() => startItem(itemKind, next));
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // New suggestions (the taskbag reloaded): an ID the person has not typed follows them.
  useEffect(() => {
    setTask((d) => refreshTask(d, next));
    setItem((d) => refreshItem(d, itemKind, next));
  }, [next, itemKind]);

  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const ready = creating.kind === "task" ? canAdd(task, base) : canAdd(item);
  const save = async () => {
    if (busy || !ready) return;
    setBusy(true);
    try {
      await onAdd(
        creating.kind === "task"
          ? { kind: "task", task: toNewTask(task) }
          : creating.kind === "area"
            ? { kind: "area", id: item.id.trim() || null, name: item.name.trim() }
            : { kind: "iteration", iteration: toNewIteration(item) },
      );
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const saveName = async () => {
    if (!name.trim() || busy) return;
    setBusy(true);
    try {
      await onSaveName(name.trim());
      setError(null);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const keys = (e: KeyboardEvent, enter: () => void) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && !(e.target instanceof HTMLTextAreaElement)) {
      e.preventDefault();
      enter();
    }
  };

  const title = t(creating.kind === "task" ? "create.new_task" : creating.kind === "area" ? "create.new_area" : "create.new_iteration");
  const iterationName = (id: string) => {
    const it = iterations.find((i) => i.id === id);
    return it ? (it.release ? `${it.name} (${it.release})` : it.name) : id;
  };

  if (!ownerName) {
    return (
      <form className="create-form editor" aria-label={title} onSubmit={(e) => e.preventDefault()} onKeyDown={(e) => keys(e, () => void saveName())}>
        <h3>{title}</h3>
        <p className="small">{t("create.name_explain")}</p>
        <input type="text" value={name} placeholder={t("edit.name_placeholder")} aria-label={t("edit.name_placeholder")} onChange={(e) => setName(e.target.value)} disabled={busy} autoFocus />
        <div className="editor-buttons">
          <button type="button" className="primary" onClick={() => void saveName()} disabled={busy || !name.trim()}>
            {t("edit.name_save")}
          </button>
          <button type="button" onClick={onClose} disabled={busy}>
            {t("edit.cancel")}
          </button>
        </div>
        {error && <p className="banner error small">{error}</p>}
      </form>
    );
  }

  const idField = (value: string, suggested: string, set: (id: string) => void) => (
    <label>
      <span>{t("create.id")}</span>
      <input type="text" value={value} placeholder={suggested} onChange={(e) => set(e.target.value)} disabled={busy} spellCheck={false} />
    </label>
  );

  return (
    <form className="create-form editor" aria-label={title} onSubmit={(e) => e.preventDefault()} onKeyDown={(e) => keys(e, () => void save())}>
      <h3>{title}</h3>
      {creating.kind === "task" ? (
        <>
          {/* In an iteration group (or Unplanned) the area is chosen; in an area group it is that area. */}
          {creating.area === null && (
            <label>
              <span>{t("create.area")}</span>
              <select value={task.area} onChange={(e) => setTask((d) => pickArea(d, e.target.value, next))} disabled={busy}>
                {areas.length === 0 && <option value="">{t("create.no_areas")}</option>}
                {areas.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.id} {a.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          {idField(task.id, task.suggested, (id) => setTask((d) => ({ ...d, id })))}
          <label>
            <span>{t("create.title")}</span>
            <input type="text" value={task.title} onChange={(e) => setTask((d) => ({ ...d, title: e.target.value }))} disabled={busy} autoFocus />
          </label>
          <label>
            <span>{t("create.description")}</span>
            <textarea rows={4} value={task.description} onChange={(e) => setTask((d) => ({ ...d, description: e.target.value }))} disabled={busy} />
          </label>
          <label>
            <span>{t("create.iteration")}</span>
            <select value={task.iteration} onChange={(e) => setTask((d) => ({ ...d, iteration: e.target.value }))} disabled={busy}>
              <option value="">{t("box.unplanned")}</option>
              {iterations.map((i) => (
                <option key={i.id} value={i.id}>
                  {iterationName(i.id)}
                </option>
              ))}
            </select>
          </label>
          {/* Task-A03-22: one base among the open tasks is used; several must be picked from. */}
          {base.kind === "same" && <p className="muted small">{t("create.base_same", { base: base.base })}</p>}
          {base.kind === "pick" && (
            <label>
              <span>{t("create.base")}</span>
              <select value={task.base} onChange={(e) => setTask((d) => ({ ...d, base: e.target.value }))} disabled={busy} required>
                <option value="" disabled>
                  {t("create.base_pick")}
                </option>
                {base.bases.map((b) => (
                  <option key={b} value={b}>
                    {b}
                  </option>
                ))}
              </select>
              <span className="muted small">{t("create.base_hint")}</span>
            </label>
          )}
          <p className="muted small">{t("create.task_hint")}</p>
        </>
      ) : (
        <>
          {idField(item.id, item.suggested, (id) => setItem((d) => ({ ...d, id })))}
          <label>
            <span>{t("create.name")}</span>
            <input type="text" value={item.name} onChange={(e) => setItem((d) => ({ ...d, name: e.target.value }))} disabled={busy} autoFocus />
          </label>
          {creating.kind === "iteration" && (
            <>
              <label>
                <span>{t("create.release")}</span>
                <input type="text" value={item.release} placeholder="v0.3" onChange={(e) => setItem((d) => ({ ...d, release: e.target.value }))} disabled={busy} />
              </label>
              <label>
                <span>{t("create.start")}</span>
                <input type="date" value={item.start} onChange={(e) => setItem((d) => ({ ...d, start: e.target.value }))} disabled={busy} />
              </label>
              <label>
                <span>{t("create.end")}</span>
                <input type="date" value={item.end} onChange={(e) => setItem((d) => ({ ...d, end: e.target.value }))} disabled={busy} />
              </label>
            </>
          )}
        </>
      )}
      <div className="editor-buttons">
        <button type="button" className="primary" onClick={() => void save()} disabled={busy || !ready}>
          {busy ? t("edit.saving") : t("create.add")}
        </button>
        <button type="button" onClick={onClose} disabled={busy}>
          {t("edit.cancel")}
        </button>
      </div>
      {error && <p className="banner error small">{error}</p>}
    </form>
  );
}
