// The task pane (Task-A01-16): everything about one task, beside the list. History lists the
// task's recorded changes (Task-A01-17). Eight fields can be changed here (Task-A01-18); Rust
// checks each change by the same rule as its `alm` command, and a refusal is shown in its words.
// A task finished without the human check shows "Not human-checked", its steps and a Checked
// button, which works like `alm checked` (Task-A03-18). The iteration is chosen from a drop-down
// whose first item, "New…", creates an iteration and plans the task in it (Task-A01-39). A task
// with AI decisions nobody has reviewed shows "AI decisions to review", the decisions and a Reviewed button,
// like `alm reviewed` (Task-A03-19). A done or dropped task has Reopen…, which asks why and works
// like `alm reopen` (Task-A01-50).

import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useI18n } from "./i18n";
import { WhoForm } from "./WhoForm";
import { Markdown } from "./Markdown";
import { LatestHandoff } from "./Handoffs";
import { localTime } from "./handoffFlow";
import { changeLine } from "./historyFlow";
import {
  EMPTY_NEW_ITERATION,
  NEW_ITERATION,
  blockerChoices,
  draftChanged,
  iterationChoices,
  iterationSavable,
  startDraft,
  stateChoices,
  toEdit,
  toPaneIteration,
  toggleBlocker,
  type Draft,
  type EditField,
  changedVersion,
  type NewIterationDraft,
} from "./editFlow";
import { canReopen, reopenEdit } from "./reopenFlow";
import { blockerText, blockers, humanCheckNote, stopText, toCheck, toReview } from "./stopFlow";
import { isProblem, type Area, type Edit, type Handoff, type Iteration, type NewIteration, type Task, type TaskHistory } from "./types";

export function TaskPane({
  task,
  tasks,
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
  version,
  onReview,
  nextIterationId,
  onNewIteration,
}: {
  task: Task;
  /** Every task of the taskbag: a blocked task's blockers, and the "Blocked by" picker (decision 0016). */
  tasks: Task[];
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
  /** The task's version now (Task-A01-57); a title or description is saved against the version
   * shown when the change began. */
  version: string | null;
  /** Marks the task's AI decisions reviewed. Rejects with the Problem that refused it. */
  onReview: () => Promise<void>;
  /** The ID "New…" will give, when known (the next free one). */
  nextIterationId: string | null;
  /** "New…": adds the iteration and plans this task in it. Rejects with the Problem that refused it; then nothing changed. */
  onNewIteration: (iteration: NewIteration) => Promise<void>;
}) {
  const { t, problem } = useI18n();
  const ref = useRef<HTMLElement>(null);
  const [editing, setEditing] = useState<EditField | null>(null);
  const [draft, setDraft] = useState<Draft>({ value: "", waitingOn: "", who: "", blockedBy: [] });
  const [created, setCreated] = useState<NewIterationDraft>(EMPTY_NEW_ITERATION);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  // The Checked or Reviewed button asked for the person's name first (asked once, like a change).
  const [asking, setAsking] = useState<"checked" | "reviewed" | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Task-A01-57: the task's version when the change began, and, when a save was refused because
  // the task changed since, the version it has now.
  const [startVersion, setStartVersion] = useState<string | null>(null);
  const [conflict, setConflict] = useState<string | null>(null);
  // Reopen… (Task-A01-50): the reason being written; null while the form is closed.
  const [reason, setReason] = useState<string | null>(null);
  // A new task starts at the top of the pane, with nothing being changed.
  useEffect(() => {
    ref.current?.scrollTo?.({ top: 0 });
    setEditing(null);
    setAsking(null);
    setReason(null);
    setError(null);
    setConflict(null);
  }, [task.id]);

  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const begin = (field: EditField) => {
    setReason(null);
    setEditing(field);
    setDraft(startDraft(task, field));
    setStartVersion(version);
    setConflict(null);
    setCreated(EMPTY_NEW_ITERATION);
    setError(null);
  };
  const cancel = () => {
    setConflict(null);
    setEditing(null);
    setAsking(null);
    setReason(null);
    setError(null);
  };
  /** Reopen (Task-A01-50): through the same rule as `alm reopen`; refused in Rust's words. */
  const reopen = async () => {
    const edit = reopenEdit(reason ?? "");
    if (!edit || busy) return;
    setBusy(true);
    try {
      await onEdit(edit);
      setReason(null);
      setError(null);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  /** Checked (Task-A03-18): top label, off To check. Reviewed (Task-A03-19): the AI decisions
   * leave To review. Refused in Rust's words like any change. */
  const mark = (which: "checked" | "reviewed") => (which === "checked" ? onEdit({ field: "checked" }) : onReview());
  const markDone = async (which: "checked" | "reviewed") => {
    if (busy) return;
    if (!ownerName) {
      setEditing(null);
      setAsking(which);
      return;
    }
    setBusy(true);
    try {
      await mark(which);
      setAsking(null);
      setError(null);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const savable = (field: EditField) => (field === "iteration" ? iterationSavable(task, draft, created) : draftChanged(task, field, draft));
  // `against`: the version this save is made from; "Keep mine" saves against the newer one.
  const save = async (against: string | null = startVersion) => {
    if (!editing || busy || (!savable(editing) && !conflict)) return;
    setBusy(true);
    try {
      if (editing === "iteration" && draft.value === NEW_ITERATION) await onNewIteration(toPaneIteration(created));
      else await onEdit(toEdit(editing, draft, against));
      setEditing(null);
      setError(null);
      setConflict(null);
    } catch (e) {
      // Changed while the person was editing (Task-A01-57): nothing was saved; show both.
      const now = changedVersion(e);
      if (now) {
        setConflict(now);
        setError(null);
      } else say(e);
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
      // Asked for by the Checked or Reviewed button: go on with it.
      if (asking) {
        await mark(asking);
        setAsking(null);
      }
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

  /** The "Blocked by" task picker: every other task, ticked when it blocks this one. */
  const picker = (): ReactNode => (
    <fieldset className="blocker-picker" disabled={busy}>
      <legend className="small">{t("edit.blocked_by_legend")}</legend>
      {blockerChoices(task, tasks).map((x) => (
        <label key={x.id} className="small">
          <input type="checkbox" checked={draft.blockedBy.includes(x.id)} onChange={() => setDraft((d) => toggleBlocker(d, x.id))} />{" "}
          <span className="id">{x.id}</span> {x.title} <span className="muted">({t(`state.${x.state}`)})</span>
        </label>
      ))}
    </fieldset>
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
                <option key={s} value={s} disabled={s === "backlog" || s === "in_progress"}>
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
            {draft.value === "waiting" && (
              <input
                type="text"
                value={draft.who}
                placeholder={t("edit.who_placeholder")}
                aria-label={t("pane.who")}
                onChange={(e) => setDraft((d) => ({ ...d, who: e.target.value }))}
                onKeyDown={(e) => keys(e, true)}
                disabled={busy}
              />
            )}
            {draft.value === "blocked" && picker()}
          </>
        );
      case "blocked_by":
        return picker();
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
          <>
            <select value={draft.value} onChange={(e) => set(e.target.value)} disabled={busy} aria-label={label} autoFocus>
              {iterationChoices(task, iterations).map((c) => (
                <option key={c.kind + c.value} value={c.value}>
                  {c.kind === "new"
                    ? t("edit.new_iteration")
                    : c.kind === "none"
                      ? t("edit.no_iteration")
                      : [c.id, c.name, c.release && `(${c.release})`].filter(Boolean).join(" ")}
                </option>
              ))}
            </select>
            {draft.value === NEW_ITERATION && (
              <div className="new-iteration">
                <label className="small">
                  <span>{t("create.name")}</span>
                  <input
                    type="text"
                    value={created.name}
                    onChange={(e) => setCreated((c) => ({ ...c, name: e.target.value }))}
                    onKeyDown={(e) => keys(e, true)}
                    disabled={busy}
                    autoFocus
                  />
                </label>
                <label className="small">
                  <span>{t("create.start")}</span>
                  <input type="date" value={created.start} onChange={(e) => setCreated((c) => ({ ...c, start: e.target.value }))} disabled={busy} />
                </label>
                <label className="small">
                  <span>{t("create.end")}</span>
                  <input type="date" value={created.end} onChange={(e) => setCreated((c) => ({ ...c, end: e.target.value }))} disabled={busy} />
                </label>
                <p className="muted small">
                  {nextIterationId ? t("edit.new_iteration_hint_id", { id: nextIterationId }) : t("edit.new_iteration_hint")}
                </p>
              </div>
            )}
          </>
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
  const nameForm = (): ReactNode => (
    <WhoForm
      name={name}
      onName={setName}
      explain="edit.name_explain"
      label="who.label_task"
      busy={busy}
      error={error}
      onSave={() => void saveName()}
      onCancel={cancel}
    />
  );
  const editor = (field: EditField): ReactNode =>
    !ownerName ? (
      nameForm()
    ) : (
      <div className="editor">
        {control(field)}
        {field === "waiting_on" && task.state !== "waiting" && <p className="muted small">{t("edit.waiting_on_hint")}</p>}
        {field === "blocked_by" && task.state !== "blocked" && <p className="muted small">{t("edit.blocked_by_hint")}</p>}
        <div className="editor-buttons">
          <button type="button" className="primary" onClick={() => void save()} disabled={busy || !savable(field)}>
            {busy ? t("edit.saving") : t("edit.save")}
          </button>
          <button type="button" onClick={cancel} disabled={busy}>
            {t("edit.cancel")}
          </button>
        </div>
        {error && <p className="banner error small">{error}</p>}
        {conflict && (field === "title" || field === "description") && (
          <div className="conflict banner error" role="alert">
            <strong>{t("edit.conflict_title")}</strong>
            <p className="small">{t("edit.conflict_body")}</p>
            <span className="small">{t("edit.conflict_newer", { field: fieldName(field) })}</span>
            {version === conflict ? (
              <pre className="newer">{field === "title" ? task.title : task.description.trimEnd() || t("pane.no_description")}</pre>
            ) : (
              <p className="muted small">{t("edit.conflict_loading")}</p>
            )}
            <div className="editor-buttons">
              <button type="button" className="primary" onClick={() => void save(conflict)} disabled={busy}>
                {t("edit.keep_mine")}
              </button>
              <button type="button" onClick={cancel} disabled={busy}>
                {t("edit.keep_newer")}
              </button>
            </div>
          </div>
        )}
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
  // The area, holder and "finished without the human check" are not changed here: holders are
  // agent sessions (owner, 2026-10-14), and the others have no change of their own in `alm` (the
  // Checked button below clears the note).
  const rows: [string, string | null, EditField | null][] = [
    [t("pane.area"), area ? `${area.id} ${area.name}` : task.area, null],
    [t("pane.iteration"), iterationText, "iteration"],
    [t("pane.state"), stopText(task, tasks, t) ?? t(`state.${task.state}`), "state"],
    [t("pane.label"), task.label, "label"],
    [t("pane.holder"), task.holder, null],
    [t("pane.waiting_on"), task.waiting_on, "waiting_on"],
    [t("pane.who"), task.state === "waiting" ? (task.who ?? t("task.you")) : null, null],
    [
      t("pane.blocked_by"),
      (task.blocked_by ?? []).length ? blockers(task, tasks).map((b) => blockerText(b, t)).join(", ") : null,
      "blocked_by",
    ],
    [t("pane.next"), task.next_dependency, "next_dependency"],
    [t("pane.handoff"), task.handoff_note, "handoff_note"],
    [t("pane.without_human_check"), humanCheckNote(task) || null, null],
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
      {toCheck(task) && (
        <div className="pane-check">
          <HumanCheckBadge task={task} />
          {asking === "checked" && !ownerName ? (
            nameForm()
          ) : (
            <>
              <button type="button" className="primary" onClick={() => void markDone("checked")} disabled={busy}>
                {t("pane.checked_button")}
              </button>
              <p className="muted small">{t("pane.checked_hint", { label: labels[labels.length - 1] ?? "" })}</p>
              {error && !editing && asking !== "reviewed" && reason === null && <p className="banner error small">{error}</p>}
            </>
          )}
        </div>
      )}
      {toReview(task) && (
        <div className="pane-check pane-review">
          <AiDecidedBadge task={task} />
          <h3>{t("pane.ai_decisions")}</h3>
          <ul className="ai-decisions">
            {(task.ai_decisions ?? []).map((d, i) => (
              <li key={`${d.at}-${i}`}>
                {d.text}{" "}
                <span className="muted small">{t("pane.ai_decision_by", { by: d.by || t("pane.history_someone"), at: localTime(d.at) })}</span>
              </li>
            ))}
          </ul>
          {asking === "reviewed" && !ownerName ? (
            nameForm()
          ) : (
            <>
              <button type="button" className="primary" onClick={() => void markDone("reviewed")} disabled={busy}>
                {t("pane.reviewed_button")}
              </button>
              <p className="muted small">{t("pane.reviewed_hint")}</p>
              {error && !editing && asking === "reviewed" && <p className="banner error small">{error}</p>}
            </>
          )}
        </div>
      )}
      {canReopen(task) && (
        <div className="pane-check pane-reopen">
          {reason === null ? (
            <button
              type="button"
              onClick={() => {
                setEditing(null);
                setAsking(null);
                setError(null);
                setReason("");
              }}
              disabled={busy}
            >
              {t("pane.reopen_button")}
            </button>
          ) : !ownerName ? (
            nameForm()
          ) : (
            <div className="editor">
              <textarea
                value={reason}
                rows={3}
                placeholder={t("pane.reopen_placeholder")}
                aria-label={t("pane.reopen_note")}
                onChange={(e) => setReason(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") {
                    e.preventDefault();
                    cancel();
                  }
                }}
                disabled={busy}
                autoFocus
              />
              <div className="editor-buttons">
                <button type="button" className="primary" onClick={() => void reopen()} disabled={busy || !reopenEdit(reason)}>
                  {busy ? t("edit.saving") : t("pane.reopen_save")}
                </button>
                <button type="button" onClick={cancel} disabled={busy}>
                  {t("edit.cancel")}
                </button>
              </div>
              {error && <p className="banner error small">{error}</p>}
            </div>
          )}
          <p className="muted small">{t("pane.reopen_hint")}</p>
        </div>
      )}
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
      {toCheck(task) && (
        <section>
          <h3>{t("pane.human_check_steps")}</h3>
          {task.human_check_steps?.trim() ? (
            <Markdown source={task.human_check_steps} onLink={onLink} />
          ) : (
            <p className="muted small">{t("pane.no_steps")}</p>
          )}
        </section>
      )}
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
                  {e.changes.map((c, j) => (
                    <div key={`${c.field}-${j}`}>{changeLine(c, t)}</div>
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

/** "Not human-checked" on a task finished without the human check (Task-A03-18): in the lists
 * and the task pane's header; hovering shows why. */
export function HumanCheckBadge({ task }: { task: Task }) {
  const { t } = useI18n();
  return (
    <span className="badge to-check" title={t("task.not_human_checked_title", { why: humanCheckNote(task) })}>
      {t("task.not_human_checked")}
    </span>
  );
}

/** "AI decisions to review" on a task with AI decisions nobody has reviewed yet (Task-A03-19): in the lists
 * and the task pane; hovering says how many. */
export function AiDecidedBadge({ task }: { task: Task }) {
  const { t } = useI18n();
  return (
    <span className="badge ai-decided" title={t("task.ai_decided_title", { count: task.ai_decisions?.length ?? 0 })}>
      {t("task.ai_decided")}
    </span>
  );
}
