// One task in the task list (Task-A01-cactus586): its ID and title on the first line, then its facts
// the way the task pane shows them, a small grey name before each value, two to a line (one on a
// narrow list). A stopped task's Waiting on or Blocked by gets a full-width row of its own, cut at
// three lines with Show all. Clicking the row (or Enter / Space on it) opens the task pane; clicking
// it again closes it. The holder's link and Show all do not select the task. Held by always shows
// (Nobody when no session holds the task), and a task nobody holds that is not finished gets
// Work on it… after it (Task-A03-galley051); a lost task gets Take over… after its Lost tag
// (Task-A01-voyage603). In the dependency tree (Task-A01-alpine458) the row is indented under the
// task that blocks it, with guide lines and a fold button when tasks hang under it. Every blocker
// is named in Blocked by (Task-A01-border342), so the title carries no marks for them.

import { useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { useI18n, useTaskT } from "./i18n";
import { HeartbeatLine, LostTag, TakeOverButton } from "./Heartbeat";
import { LabelChips } from "./LabelChips";
import { DescriptionHit, Highlight } from "./Search";
import { canProceed } from "./proceedFlow";
import { sessionPlace } from "./sessionFlow";
import { chipLabels, decisionsToReview, humanCheckNote, isBlocked, toCheck, toReview, type BlockerPart, type StopFact } from "./stopFlow";
import { localTime } from "./timeFlow";
import { difficultyOf, difficultyText } from "./difficultyFlow";
import { riskClass, riskText } from "./taskRiskFlow";
import type { TreeRow } from "./depTreeFlow";
import type { Edit, Task } from "./types";
import { StateMenu } from "./StateMenu";
import { DifficultyMenu } from "./DifficultyMenu";

/** A row's place in the dependency tree (Task-A01-alpine458). */
export interface TreeMark {
  row: TreeRow;
  /** Folds or opens the tasks under it. */
  onFold: () => void;
}

/** Keeps a key on a control inside the row from selecting the task (its click stops itself). */
const ownKeys = (e: KeyboardEvent) => e.stopPropagation();

export function TaskRow({
  task,
  selected,
  onSelect,
  terms,
  stops,
  blockerTitle,
  modified,
  lostAfter,
  tree,
  onOpenSession,
  onProceed,
  onError,
  stateMenu,
}: {
  /** The state menu and the difficulty menu in the row (Task-A01-frost005, Task-A01-pewter675): the
   * person's name and the change to make; left out, the state and the difficulty are plain text. */
  stateMenu?: { ownerName: string | null; onSaveName: (name: string) => Promise<void>; onEdit: (id: string, edit: Edit) => Promise<void> };
  /** Its place in the dependency tree, when the tree is on. */
  tree?: TreeMark;
  task: Task;
  selected: boolean;
  onSelect: (id: string) => void;
  /** The search's words, marked in the row (Task-A01-36); empty when not searching. */
  terms: string[];
  /** Its Waiting on and Blocked by facts; none when it waits on nobody and has no blocker. */
  stops: StopFact[];
  /** A blocker's hover text: its title and the group it is in. */
  blockerTitle?: (id: string) => string;
  /** When the task last changed (UTC) (Task-A01-52). */
  modified?: string;
  /** The project's lost limit, in minutes (Task-A01-kernel761). */
  lostAfter: number;
  /** Reopens the session the task records, as the task pane's button does (the desktop app only). */
  onOpenSession?: (id: string) => Promise<void>;
  /** Opens Work on it… for the task, or Take over… for a lost one (the desktop app only, where agents run). */
  onProceed?: (task: Task) => void;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const taskT = useTaskT();
  const decisions = decisionsToReview(task).length;
  const facts: [string, ReactNode, string?][] = [
    [
      t("pane.state"),
      <>
        {/* Task-A01-frost005: the state word opens the state list; "Blocked" is only a note beside it. */}
        {stateMenu ? (
          <StateMenu task={task} ownerName={stateMenu.ownerName} onSaveName={stateMenu.onSaveName} onEdit={(edit) => stateMenu.onEdit(task.id, edit)} />
        ) : (
          <span className={`fact-state state-word tint s-${task.state}-text`}>{taskT(`state.${task.state}`)}</span>
        )}
        {/* Decision 0026: blocked is worked out, shown beside the task's own state. */}
        {isBlocked(task) && <span className="fact-blocked s-blocked-text"> · {t("task.blocked_mark")}</span>}
      </>,
    ],
    [t("row.modified"), modified ? <time dateTime={modified}>{localTime(modified)}</time> : <span className="muted">—</span>],
  ];
  // Task-A01-actor813: how hard it is; a task with none counts as 3, shown greyed as the default.
  facts.push([
    t("pane.difficulty"),
    // Task-A01-pewter675: the difficulty opens the list of levels, as the state word opens the states.
    stateMenu ? (
      <DifficultyMenu task={task} ownerName={stateMenu.ownerName} onSaveName={stateMenu.onSaveName} onEdit={(edit) => stateMenu.onEdit(task.id, edit)} />
    ) : (
      <span className={difficultyOf(task).isDefault ? "muted" : undefined}>{difficultyText(task, t)}</span>
    ),
    t("pane.difficulty_title"),
  ]);
  // Task-A01-garden385 (decision 0027): the risk an agent recorded; none reads "Not classified".
  facts.push([t("task.risk"), <span className={riskClass(task)}>{riskText(task, t)}</span>, t("task.risk_title")]);
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
      {onProceed && task.holder && (
        <TakeOverButton task={task} lostAfter={lostAfter} lastChange={modified} onKeyDown={ownKeys} onTakeOver={() => onProceed(task)} />
      )}
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
  const main = (
    <>
      <span className="id">
        <Highlight text={task.id} terms={terms} />
      </span>
      <div className="task-body">
        <span className="title">
          <Highlight text={task.title} terms={terms} />
          {tree && <TreeNotes tree={tree} onOpenTask={onSelect} />}
        </span>
        <dl className="row-facts">
          {facts.map(([name, value, title]) => (
            <div key={name} title={title}>
              <dt>{name}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        {stops.map((fact) => (
          <StopRow key={fact.kind} fact={fact} state={task.state} blockerTitle={blockerTitle} onOpenTask={onSelect} />
        ))}
        <DescriptionHit text={task.description ?? ""} terms={terms} />
        <DescriptionHit text={task.handoff_note ?? ""} terms={terms} where="search.in_handoff" />
      </div>
    </>
  );
  const finished = task.state === "done" || task.state === "dropped";
  const classes = ["task", selected ? "selected" : "", tree ? "in-tree" : "", tree && tree.row.kids > 0 && finished ? "tree-parent-finished" : ""];
  return (
    <li
      id={`task-${task.id}`}
      className={classes.filter(Boolean).join(" ")}
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
      {tree ? (
        <>
          <TreeLead tree={tree} />
          <div className="tree-rest">
            {main}
          </div>
        </>
      ) : (
        main
      )}
    </li>
  );
}

/** How many colours the tree's levels cycle through (styles.css, `.tree-cell.lv-N`). */
const TREE_COLOURS = 6;
const lv = (depth: number) => `lv-${depth % TREE_COLOURS}`;

/** The row's indent in the dependency tree, drawn like a group tree: one column per level, each
 * level in its own colour. A rail runs down a column while that level has more tasks below; the
 * line to this row's parent turns into the row with a rounded corner; then this row's node, a
 * dot, which is the fold button (− open, + folded) when tasks hang under it, with its own rail
 * running down to them while open. */
function TreeLead({ tree }: { tree: TreeMark }) {
  const { t } = useI18n();
  const { row } = tree;
  const node = `tree-cell node ${lv(row.depth)}${row.kids > 0 && !row.folded ? " open" : ""}`;
  return (
    <span className="tree-lead">
      {row.lines.map((on, i) => (
        <span key={i} className={`tree-cell ${lv(i)}${on ? " line" : ""}`} />
      ))}
      {row.depth > 0 && <span className={`tree-cell elbow ${lv(row.depth - 1)}${row.last ? " last" : ""}`} />}
      {row.kids > 0 ? (
        <span className={node}>
          <button
            type="button"
            className="tree-dot tree-fold"
            aria-expanded={!row.folded}
            aria-label={t(row.folded ? "tree.expand" : "tree.collapse", { id: row.task.id })}
            title={t(row.folded ? "tree.expand" : "tree.collapse", { id: row.task.id })}
            onKeyDown={ownKeys}
            onClick={(e) => {
              e.stopPropagation();
              tree.onFold();
            }}
          >
            <svg viewBox="0 0 10 10" aria-hidden="true">
              <path d={row.folded ? "M2 5h6M5 2v6" : "M2 5h6"} />
            </svg>
          </button>
        </span>
      ) : (
        <span className={node}>
          <span className="tree-dot" />
        </span>
      )}
    </span>
  );
}

/** After the title in the dependency tree: how many tasks its fold hides. Its blockers are all in
 * Blocked by (Task-A01-border342), not here. */
function TreeNotes({ tree, onOpenTask }: { tree: TreeMark; onOpenTask: (id: string) => void }) {
  const { t } = useI18n();
  const { row } = tree;
  if (row.hidden === 0 && row.also.length === 0 && row.outside.length === 0) return null;
  return (
    <span className="tree-notes">
      {row.also.map((id) => <button type="button" key={`also-${id}`} className="tree-note tree-dependency" onKeyDown={ownKeys} onClick={(e) => { e.stopPropagation(); onOpenTask(id); }}>{t("tree.also", { id })}</button>)}
      {row.outside.map((id) => <button type="button" key={`outside-${id}`} className="tree-note tree-dependency" onKeyDown={ownKeys} onClick={(e) => { e.stopPropagation(); onOpenTask(id); }}>{t("tree.outside", { id })}</button>)}
      {row.hidden > 0 && <span className="tree-note tree-more" title={t("tree.more_title", { count: row.hidden })}>+{row.hidden}</span>}
    </span>
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

/** Waiting on or Blocked by: the name on a line of its own, the text under it, at most three lines;
 * Show all opens it in place and Show less folds it back. The link shows only when the text is
 * cut, and the choice is not remembered. Waiting on is in the state's colour. Blocked by names
 * every blocker (Task-A01-border342): those not finished first, in the blocked colour, then the
 * finished in the done colour; hovering one shows its title and the group it is in. */
function StopRow({ fact, state, blockerTitle, onOpenTask }: { fact: StopFact; state: Task["state"]; blockerTitle?: (id: string) => string; onOpenTask: (id: string) => void }) {
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
      <div ref={ref} className={`row-stop-text${fact.parts ? "" : ` s-${state}-text`}${open ? "" : " clamped"}`}>
        {fact.parts ? <BlockerList parts={fact.parts} title={blockerTitle} onOpenTask={onOpenTask} /> : fact.text}
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

/** A task's blockers as the list and the task pane show them (Task-A01-border342), in the order
 * `blockerParts` gives: one not finished in the blocked colour, a finished one in the done colour.
 * Hovering one shows `title(id)`, or its title alone without it. */
export function BlockerList({ parts, title, onOpenTask }: { parts: BlockerPart[]; title?: (id: string) => string; onOpenTask?: (id: string) => void }) {
  return (
    <>
      {parts.map((b, i) => (
        <span key={b.id}>
          {i > 0 && <span className="muted">, </span>}
          {onOpenTask && b.title !== null ? (
            <button type="button" className={`row-blocker row-blocker-link ${b.finished ? "finished" : "open"}`} title={title?.(b.id) ?? b.title ?? undefined} onKeyDown={ownKeys} onClick={(e) => { e.stopPropagation(); onOpenTask(b.id); }}>{b.text}</button>
          ) : (
            <span className={`row-blocker ${b.finished ? "finished" : "open"}`} title={title?.(b.id) ?? b.title ?? undefined}>{b.text}</span>
          )}
        </span>
      ))}
    </>
  );
}

/** Why a task is to check, on hover: the reason it was finished without the human check, or,
 * with none written, that no person has checked it (decision 0020). */
export function checkTitle(task: Task, t: ReturnType<typeof useI18n>["t"]): string {
  const why = humanCheckNote(task).trim();
  return why ? t("task.not_human_checked_title", { why }) : t("task.not_human_checked_plain");
}
