// One task in the task list (Task-A01-cactus586): its ID and title on the first line, then its facts
// the way the task pane shows them, a small grey name before each value, two to a line (one on a
// narrow list). A stopped task's Waiting on or Blocked by gets a full-width row of its own, cut at
// three lines with Show all. Clicking the row (or Enter / Space on it) opens the task pane; clicking
// it again closes it. The holder's link and Show all do not select the task. Held by always shows
// (Nobody when no session holds the task), and a task nobody holds that is not finished gets
// Work on it… after it (Task-A03-galley051).

import { useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { useI18n } from "./i18n";
import { HeartbeatLine, LostTag } from "./Heartbeat";
import { LabelChips } from "./LabelChips";
import { DescriptionHit, Highlight } from "./Search";
import { canProceed } from "./proceedFlow";
import { sessionPlace } from "./sessionFlow";
import { chipLabels, humanCheckNote, toCheck, toReview, type StopFact } from "./stopFlow";
import { localTime } from "./timeFlow";
import type { Task } from "./types";

/** Keeps a key on a control inside the row from selecting the task (its click stops itself). */
const ownKeys = (e: KeyboardEvent) => e.stopPropagation();

export function TaskRow({
  task,
  selected,
  onSelect,
  terms,
  stop,
  modified,
  lostAfter,
  onOpenSession,
  onProceed,
  onError,
}: {
  task: Task;
  selected: boolean;
  onSelect: (id: string) => void;
  /** The search's words, marked in the row (Task-A01-36); empty when not searching. */
  terms: string[];
  /** Why the task is stopped: its Waiting on or Blocked by fact; null when it is not stopped. */
  stop: StopFact | null;
  /** When the task last changed (UTC) (Task-A01-52). */
  modified?: string;
  /** The project's lost limit, in minutes (Task-A01-kernel761). */
  lostAfter: number;
  /** Reopens the session the task records, as the task pane's button does (the desktop app only). */
  onOpenSession?: (id: string) => Promise<void>;
  /** Opens Work on it… for the task (the desktop app only, where agents run). */
  onProceed?: (task: Task) => void;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const decisions = task.ai_decisions?.length ?? 0;
  const facts: [string, ReactNode, string?][] = [
    [
      t("pane.state"),
      <span className={`fact-state state-word tint s-${task.state}-text`}>{t(`state.${task.state}`)}</span>,
    ],
    [t("row.modified"), modified ? <time dateTime={modified}>{localTime(modified)}</time> : <span className="muted">—</span>],
  ];
  // Labels only when the task has some (Task-A01-cell542): every label it carries (decision 0020);
  // "AI decisions to review" as its own fact below. The details pane always shows them.
  if (chipLabels(task).length > 0) facts.push([t("pane.label"), <LabelChips task={task} />]);
  // The decisions from the history; before it is read (or once none is left there), the label alone.
  if (toReview(task))
    facts.push([
      t("row.ai_decided"),
      decisions > 0 ? t("row.ai_decided_value", { count: decisions }) : t("task.ai_decided"),
      decisions > 0 ? t("task.ai_decided_title", { count: decisions }) : t("label.meaning.ai_decisions"),
    ]);
  if (toCheck(task)) facts.push([t("pane.check"), t("task.not_human_checked"), checkTitle(task, t)]);
  facts.push([
    t("row.held_by"),
    <span className="held-by">
      {task.holder ? <Holder task={task} onOpenSession={onOpenSession} onError={onError} /> : <span className="muted">{t("row.nobody")}</span>}
      {task.holder && <LostTag task={task} lostAfter={lostAfter} lastChange={modified} />}
      {onProceed && canProceed(task) && (
        <button
          type="button"
          className="link with-icon proceed"
          title={t("proceed.button_title")}
          onKeyDown={ownKeys}
          onClick={(e) => {
            e.stopPropagation();
            onProceed(task);
          }}
        >
          <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
            <path d="M6.5 4.5v11l9-5.5z" />
          </svg>
          {t("proceed.button")}
        </button>
      )}
    </span>,
  ]);
  // Task-A01-kernel761: a held, in-progress task's last heartbeat and busy window.
  if (task.state === "in_progress" && task.holder)
    facts.push([t("pane.heartbeat"), <HeartbeatLine task={task} lostAfter={lostAfter} lastChange={modified} />]);
  return (
    <li
      id={`task-${task.id}`}
      className={selected ? "task selected" : "task"}
      tabIndex={0}
      aria-current={selected ? "true" : undefined}
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
      <div className="task-body">
        <span className="title">
          <Highlight text={task.title} terms={terms} />
        </span>
        <dl className="row-facts">
          {facts.map(([name, value, title]) => (
            <div key={name} title={title}>
              <dt>{name}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        {stop && <StopRow fact={stop} state={task.state} />}
        <DescriptionHit text={task.description ?? ""} terms={terms} />
        <DescriptionHit text={task.handoff_note ?? ""} terms={terms} where="search.in_handoff" />
      </div>
    </li>
  );
}

/** Who holds the task. Where the pane would offer to open the holder's session, the name is that
 * button (Open in Claude, or Resume in terminal: its tooltip says which); otherwise plain text. */
function Holder({ task, onOpenSession, onError }: { task: Task; onOpenSession?: (id: string) => Promise<void>; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [asked, setAsked] = useState(false);
  const place = onOpenSession ? sessionPlace(task) : null;
  if (!place || !onOpenSession) return <>{task.holder}</>;
  const open = () => {
    setAsked(false);
    onOpenSession(task.id).then(
      () => setAsked(place === "app"),
      onError,
    );
  };
  return (
    <>
      <button
        type="button"
        className="link with-icon holder-open"
        title={t(place === "app" ? "session.open_in_claude" : "session.resume_in_terminal")}
        onKeyDown={ownKeys}
        onClick={(e) => {
          e.stopPropagation();
          open();
        }}
      >
        {task.holder}
        <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
          <path d="M9 4.5H5a1 1 0 0 0-1 1v9.5a1 1 0 0 0 1 1h9.5a1 1 0 0 0 1-1V11" />
          <path d="M12 3.5h4.5V8M16.5 3.5 9.5 10.5" />
        </svg>
      </button>
      {asked && <span className="muted"> {t("session.asked_claude")}</span>}
    </>
  );
}

/** Waiting on or Blocked by: the name on a line of its own, the text under it in the state's
 * colour, at most three lines; Show all opens it in place and Show less folds it back. The link
 * shows only when the text is cut, and the choice is not remembered. */
function StopRow({ fact, state }: { fact: StopFact; state: Task["state"] }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [cut, setCut] = useState(false);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || open) return;
    const look = () => setCut(el.scrollHeight > el.clientHeight + 1);
    look();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(look);
    watch.observe(el);
    return () => watch.disconnect();
  }, [fact.text, open]);
  return (
    <div className="row-stop">
      <div className="row-stop-name">{t(fact.name)}</div>
      <div ref={ref} className={`row-stop-text s-${state}-text${open ? "" : " clamped"}`}>
        {fact.text}
      </div>
      {cut && (
        <button type="button" className="link small" onKeyDown={ownKeys} onClick={(e) => {
            e.stopPropagation();
            setOpen(!open);
          }}>
          {t(open ? "row.show_less" : "row.show_all")}
        </button>
      )}
    </div>
  );
}

/** Why a task is to check, on hover: the reason it was finished without the human check, or,
 * with none written, that no person has checked it (decision 0020). */
export function checkTitle(task: Task, t: ReturnType<typeof useI18n>["t"]): string {
  const why = humanCheckNote(task).trim();
  return why ? t("task.not_human_checked_title", { why }) : t("task.not_human_checked_plain");
}
