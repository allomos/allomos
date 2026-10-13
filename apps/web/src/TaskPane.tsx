// The task pane (Task-A01-16): everything about one task, beside the list. History lists the
// task's recorded changes (Task-A01-17). Eight fields can be changed here (Task-A01-18); Rust
// checks each change by the same rule as its `alm` command, and a refusal is shown in its words.

import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import { LatestHandoff } from "./Handoffs";
import { localTime } from "./handoffFlow";
import { changeLine } from "./historyFlow";
import { draftChanged, startDraft, stateChoices, toEdit, type Draft, type EditField } from "./editFlow";
import { isProblem, type Area, type Edit, type Handoff, type Iteration, type Task, type TaskHistory } from "./types";

export function TaskPane({
  task,
  area,
  iteration,
  onClose,
  onLink,
  latest,
  onAllHandoffs,
  history,
  labels,
  iterations,
  ownerName,
  onSaveName,
  onEdit,
}: {
  task: Task;
  area: Area | undefined;
  iteration: Iteration | undefined;
  onClose: () => void;
  onLink: (href: string) => void;
  /** The task's latest handoff (Task-A03-06). */
  latest: Handoff | null;
  /** Opens the Handoffs tab on this task. */
  onAllHandoffs: () => void;
  /** The task's recorded changes; null until read, or when they cannot be. */
  history: TaskHistory | null;
  /** The project's label ladder, lowest step first. */
  labels: string[];
  /** The project's iterations, in order. */
  iterations: Iteration[];
  /** The person's name for the history; null until asked (asked once, kept in the app's settings). */
  ownerName: string | null;
  /** Keeps the person's name. Rejects with a Problem. */
  onSaveName: (name: string) => Promise<void>;
  /** Makes one change. Rejects with the Problem that refused it; then nothing changed. */
  onEdit: (edit: Edit) => Promise<void>;
}) {
  const { t, problem } = useI18n();
  const ref = useRef<HTMLElement>(null);
  const [editing, setEditing] = useState<EditField | null>(null);
  const [draft, setDraft] = useState<Draft>({ value: "", waitingOn: "" });
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A new task starts at the top of the pane, with nothing being changed.
  useEffect(() => {
    ref.current?.scrollTo?.({ top: 0 });
    setEditing(null);
    setError(null);
  }, [task.id]);

  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const begin = (field: EditField) => {
    setEditing(field);
    setDraft(startDraft(task, field));
    setError(null);
  };
  const cancel = () => {
    setEditing(null);
    setError(null);
  };
  const save = async () => {
    if (!editing || busy || !draftChanged(task, editing, draft)) return;
    setBusy(true);
    try {
      await onEdit(toEdit(editing, draft));
      setEditing(null);
      setError(null);
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
  const keys = (e: KeyboardEvent, enterSaves: boolean) => {
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    } else if (e.key === "Enter" && enterSaves && !e.nativeEvent.isComposing) {
      e.preventDefault();
      void save();
    }
  };

  const fieldName = (field: EditField) =>
    t(
      field === "next_dependency"
        ? "pane.next"
        : field === "handoff_note"
          ? "pane.handoff"
          : (`pane.${field}` as "pane.state"),
    );

  /** The control for the field being changed. */
  const control = (field: EditField): ReactNode => {
    const set = (value: string) => setDraft((d) => ({ ...d, value }));
    const label = t("edit.field_aria", { field: fieldName(field) });
    switch (field) {
      case "state":
        return (
          <>
            <select value={draft.value} onChange={(e) => set(e.target.value)} disabled={busy} aria-label={label} autoFocus>
              {stateChoices(task).map((s) => (
                <option key={s} value={s} disabled={s !== "ready" && s !== "waiting" && s !== "done" && s !== "dropped"}>
                  {t(`state.${s}`)}
                </option>
              ))}
            </select>
            {draft.value === "waiting" && (
              <input
                type="text"
                value={draft.waitingOn}
                placeholder={t("edit.waiting_on_placeholder")}
                aria-label={t("pane.waiting_on")}
                onChange={(e) => setDraft((d) => ({ ...d, waitingOn: e.target.value }))}
                onKeyDown={(e) => keys(e, true)}
                disabled={busy}
              />
            )}
          </>
        );
      case "label":
        return (
          <select value={draft.value} onChange={(e) => set(e.target.value)} disabled={busy} aria-label={label} autoFocus>
            {!task.label && (
              <option value="" disabled>
                {t("edit.no_label")}
              </option>
            )}
            {labels.map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>
        );
      case "iteration":
        return (
          <select value={draft.value} onChange={(e) => set(e.target.value)} disabled={busy} aria-label={label} autoFocus>
            <option value="">{t("box.unplanned")}</option>
            {task.iteration && !iterations.some((i) => i.id === task.iteration) && <option value={task.iteration}>{task.iteration}</option>}
            {iterations.map((i) => (
              <option key={i.id} value={i.id}>
                {i.release ? `${i.name} (${i.release})` : i.name}
              </option>
            ))}
          </select>
        );
      case "description":
      case "handoff_note":
        return (
          <textarea
            value={draft.value}
            rows={field === "description" ? 12 : 3}
            onChange={(e) => set(e.target.value)}
            onKeyDown={(e) => keys(e, false)}
            disabled={busy}
            aria-label={label}
            autoFocus
          />
        );
      default:
        return (
          <input
            type="text"
            value={draft.value}
            onChange={(e) => set(e.target.value)}
            onKeyDown={(e) => keys(e, true)}
            disabled={busy}
            aria-label={label}
            autoFocus
          />
        );
    }
  };

  /** The editor in place of a field's value: the name asked once first, then the control, Save and Cancel. */
  const editor = (field: EditField): ReactNode =>
    !ownerName ? (
      <div className="editor">
        <p className="small">{t("edit.name_explain")}</p>
        <input
          type="text"
          value={name}
          placeholder={t("edit.name_placeholder")}
          aria-label={t("edit.name_placeholder")}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void saveName();
            } else if (e.key === "Escape") {
              e.preventDefault();
              cancel();
            }
          }}
          disabled={busy}
          autoFocus
        />
        <div className="editor-buttons">
          <button type="button" className="primary" onClick={() => void saveName()} disabled={busy || !name.trim()}>
            {t("edit.name_save")}
          </button>
          <button type="button" onClick={cancel} disabled={busy}>
            {t("edit.cancel")}
          </button>
        </div>
        {error && <p className="banner error small">{error}</p>}
      </div>
    ) : (
      <div className="editor">
        {control(field)}
        {field === "waiting_on" && task.state !== "waiting" && <p className="muted small">{t("edit.waiting_on_hint")}</p>}
        <div className="editor-buttons">
          <button type="button" className="primary" onClick={() => void save()} disabled={busy || !draftChanged(task, field, draft)}>
            {busy ? t("edit.saving") : t("edit.save")}
          </button>
          <button type="button" onClick={cancel} disabled={busy}>
            {t("edit.cancel")}
          </button>
        </div>
        {error && <p className="banner error small">{error}</p>}
      </div>
    );

  const changeButton = (field: EditField) =>
    editing === field ? null : (
      <button type="button" className="link small change" onClick={() => begin(field)} aria-label={t("edit.change_aria", { field: fieldName(field) })}>
        {t("edit.change")}
      </button>
    );

  const iterationText = iteration
    ? iteration.release
      ? `${iteration.name} (${iteration.release})`
      : iteration.name
    : task.iteration
      ? task.iteration
      : t("box.unplanned");
  // The area, holder and "finished without the owner's check" are not changed here: holders are
  // agent sessions (owner, 2026-10-14), and the others have no change of their own in `alm`.
  const rows: [string, string | null, EditField | null][] = [
    [t("pane.area"), area ? `${area.id} ${area.name}` : task.area, null],
    [t("pane.iteration"), iterationText, "iteration"],
    [t("pane.state"), t(`state.${task.state}`), "state"],
    [t("pane.label"), task.label, "label"],
    [t("pane.holder"), task.holder, null],
    [t("pane.waiting_on"), task.waiting_on, "waiting_on"],
    [t("pane.next"), task.next_dependency, "next_dependency"],
    [t("pane.handoff"), task.handoff_note, "handoff_note"],
    [t("pane.finished_without_owner"), task.finished_without_owner, null],
  ];

  return (
    <aside className="task-pane" ref={ref} aria-label={t("pane.label_aria", { id: task.id })}>
      <header className="pane-head">
        {editing === "title" ? (
          editor("title")
        ) : (
          <h2>
            <span className="id">{task.id}</span> {task.title} {changeButton("title")}
          </h2>
        )}
        <button type="button" className="close" onClick={onClose} title={t("pane.close")} aria-label={t("pane.close")}>
          ×
        </button>
      </header>
      <dl className="fields">
        {rows.map(([label, value, field]) => (
          <div key={label}>
            <dt>{label}</dt>
            {field && editing === field ? (
              <dd>{editor(field)}</dd>
            ) : (
              <dd>
                <span className={value ? undefined : "muted"}>{value || "—"}</span> {field && changeButton(field)}
              </dd>
            )}
          </div>
        ))}
      </dl>
      <section>
        <h3>
          {t("pane.description")} {changeButton("description")}
        </h3>
        {editing === "description" ? (
          editor("description")
        ) : task.description.trim() ? (
          <Markdown source={task.description} onLink={onLink} />
        ) : (
          <p className="muted small">{t("pane.no_description")}</p>
        )}
      </section>
      <LatestHandoff handoff={latest} onAll={onAllHandoffs} />
      <section>
        <h3>{t("pane.history")}</h3>
        {!history?.since ? (
          <p className="muted small">{t("pane.history_none")}</p>
        ) : history.events.length === 0 ? (
          <p className="muted small">{t("pane.history_empty", { date: localTime(history.since) })}</p>
        ) : (
          <>
            <ol className="history">
              {history.events.map((e, i) => (
                <li key={`${e.at}-${i}`} className="small">
                  <span className="muted">
                    {localTime(e.at)} · {e.by || t("pane.history_someone")}
                  </span>
                  {e.changes.map((c) => (
                    <div key={c.field}>{changeLine(c, t)}</div>
                  ))}
                </li>
              ))}
            </ol>
            <p className="muted small">{t("pane.history_since", { date: localTime(history.since) })}</p>
          </>
        )}
      </section>
    </aside>
  );
}
