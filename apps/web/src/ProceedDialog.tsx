// Work on it… (Task-A03-galley051, approver 2026-11-03): from a row no session holds, hand the task to
// an AI agent. The dialog has the frame of Update the taskbag… (`MoveCardsDialog`): what Allomos
// found (the task and its state), Edit the request…, what proceeding does, the shared agent
// picker, then Cancel and Open. Open opens the agent in the project folder with the request typed
// in and sends nothing; Manual shows the request to copy instead. A blocked or waiting task cannot
// start now, so its request asks the agent to tell the person why and ask what to do. The edited
// request is kept for this dialog only.
// Take over… (Task-A01-voyage603) opens the same dialog from a lost task's row: it says who holds
// the task, how long it has been silent and the working branch to go on on, and its request asks
// the new session to claim it (the takeover) and leave the former session's worktree alone.
// From the task pane it opens for any held task (Task-A01-rocket260, decision 0022 § 6): when the
// holder is not lost, the dialog warns that its session may still be at work, its confirm button
// says so, and the request claims with --take.
// Saving what the former worktree has not pushed (Task-A01-flag101, approver 2026-11-12): when that
// worktree is on this computer the dialog lists its files not committed and commits not pushed, and
// a box (checked by default) adds a step to the request: commit them as WIP and push the working
// branch first. For a holder that is not lost the dialog first asks how to take over: preparing a
// handoff with the holding session (recommended, the default; it opens Prepare a handoff…) or
// forcing the takeover, with a red warning and a red confirm button.

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, AgentPicker, PASTE_KEYS, useAgents } from "./AgentPicker";
import { MANUAL, requestWay } from "./agentPickerFlow";
import { canTakeOver, edited, hasUnpushed, proceedRequest, stopped, takeOverRequest } from "./proceedFlow";
import type { FormerWorktree } from "./proceedFlow";
import { agoText, clockText, doingText, heldFact } from "./heartbeatFlow";
import { isProblem } from "./types";
import type { Task } from "./types";

/** How long "Copied" shows beside Copy (approver, 2026-11-03: about four seconds). */
const COPIED_MS = 4000;

export function ProceedDialog({
  messenger,
  root,
  task,
  lostAfter,
  interval,
  lastChange,
  working,
  onHandOver,
  onClose,
}: {
  messenger: Messenger;
  /** The open taskbag. */
  root: string;
  task: Task;
  /** The project's lost limit, in minutes (Task-A01-kernel761). */
  lostAfter: number;
  /** The project's heartbeat interval, in minutes: how long a holder may take to learn of a takeover. */
  interval: number;
  /** When the task last changed (UTC), for a holder that never beat. */
  lastChange?: string;
  /** The task's working branch, from the history; null when none is recorded. */
  working: string | null;
  /** Prepare a handoff… instead of taking over (Task-A01-flag101): this dialog closes and that one opens. */
  onHandOver?: () => void;
  /** The dialog closes; `message` says what happened (null: Cancel or Done). */
  onClose: (message: string | null) => void;
}) {
  const { t, rich, problem, language } = useI18n();
  // Take over… (Task-A01-voyage603): a held task, lost or not, worked out when the dialog opens.
  const [now] = useState(() => new Date());
  const [fact] = useState(() => (canTakeOver(task) ? heldFact(task, lostAfter, now, lastChange) : null));
  const takeOver = fact !== null;
  /** Taking over a holder that is not lost (Task-A01-rocket260): its session may still be at work. */
  const held = takeOver && !fact.lost;
  const ask = !takeOver && stopped(task);
  /** How to take over a holder that is not lost (Task-A01-flag101): ask it for a handoff first (the default) or force it. */
  const [way, setWay] = useState<"handover" | "force">("handover");
  const force = held && way === "force";
  /** The simple form: a held task whose session is asked for a handoff first. */
  const simple = held && !force && !!onHandOver;
  /** The former worktree on this computer (undefined while it is read, null when it is not here). */
  const [former, setFormer] = useState<FormerWorktree | null | undefined>(takeOver && working ? undefined : null);
  useEffect(() => {
    if (!takeOver || !working) return;
    let live = true;
    messenger.formerWorktree(root, task.session?.folder ?? null, working).then(
      (w) => live && setFormer(w),
      () => live && setFormer(null),
    );
    return () => {
      live = false;
    };
  }, [messenger, root, takeOver, working, task.session?.folder]);
  /** Commit and push the former worktree's changes first (checked by default, approver 2026-11-12). */
  const [save, setSave] = useState(true);
  const saving = save && !!former && hasUnpushed(former) ? former : null;
  const { agents, slow, choice: agent, setChoice: setAgent, tool } = useAgents(messenger);
  /** The taskbag as the agent passes it to `--taskbag`; null while it is read. */
  const [taskbag, setTaskbag] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    messenger.proceedTaskbag(root).then(
      (arg) => live && setTaskbag(arg),
      () => live && setTaskbag(root),
    );
    return () => {
      live = false;
    };
  }, [messenger, root]);
  const original =
    taskbag === null
      ? null
      : takeOver
        ? takeOverRequest(task, taskbag, language, { lostAfter: held ? null : fact.lostAfter, working, save: saving })
        : proceedRequest(task, taskbag, language);
  /** The person's own request, for this dialog only (null: Allomos's). */
  const [mine, setMine] = useState<string | null>(null);
  const request = mine ?? original;
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** Manual: the request to copy. */
  const [manual, setManual] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  const open = async () => {
    if (!agent || request === null) return;
    setBusy(true);
    setError(null);
    try {
      if (agent === MANUAL) {
        setManual(await messenger.proceedManualRequest(request));
        return;
      }
      await messenger.proceedWithAgent(agent, root, request);
      const name = t(AGENT_KEY[agent]);
      const way = requestWay(tool);
      if (way === "typed") {
        onClose(t(takeOver ? "takeover.asked" : ask ? "proceed.asked_stopped" : "proceed.asked", { agent: name, id: task.id }));
      } else {
        onClose(t(takeOver ? `takeover.asked_${way}` : ask ? `proceed.asked_stopped_${way}` : `proceed.asked_${way}`, { agent: name, keys: PASTE_KEYS, id: task.id }));
      }
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (manual === null) return;
    setError(null);
    try {
      await messenger.copyText(manual);
      setCopied(true);
      window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
    } catch (e) {
      fail(e);
    }
  };

  const stateWord = <span className={`state-word tint s-${task.state}-text`}>{t(`state.${task.state}`)}</span>;
  const named = <strong>{task.id}</strong>;
  const silence = fact && (fact.minutes !== null ? t("takeover.silent_for", { ago: agoText(fact.minutes, t) }) : t("takeover.never_beat"));
  const found = held
    ? rich("takeover.found_held", {
        id: named,
        title: task.title,
        state: stateWord,
        holder: <strong>{task.holder}</strong>,
        ago: fact.busyUntil ? t("takeover.ago_busy", { ago: silence!, busy: t("heartbeat.busy", { time: clockText(fact.busyUntil, now) }) }) : silence!,
      })
    : takeOver
    ? rich("takeover.found", {
        id: named,
        title: task.title,
        holder: <strong>{task.holder}</strong>,
        ago: silence!,
      })
    : task.state === "blocked"
      ? rich("proceed.found_blocked", { id: named, title: task.title, state: stateWord, by: (task.blocked_by ?? []).join(", ") })
      : task.state === "waiting"
        ? rich("proceed.found_waiting", { id: named, title: task.title, state: stateWord, on: task.waiting_on ?? "" })
        : rich("proceed.found", { id: named, title: task.title, state: stateWord });

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard move-cards proceed" role="dialog" aria-modal="true" aria-labelledby="proceed-title">
        <h2 id="proceed-title">{t(held ? "takeover.title_held" : takeOver ? "takeover.title" : ask ? "proceed.title_ask" : "proceed.title")}</h2>
        <div className="dialog-body wizard-body">
          {manual === null ? (
            <>
              <p>{found}</p>
              {held && fact.doing && <p className="takeover-doing">{doingText(fact, now, t)}</p>}
              {held && onHandOver && (
                <fieldset className="takeover-way">
                  <legend>{t("takeover.how")}</legend>
                  <label>
                    <input type="radio" name="takeover-way" checked={way === "handover"} onChange={() => setWay("handover")} disabled={busy} />
                    <span>
                      {t("takeover.way_handover")}
                      <span className="muted small block">{t("takeover.way_handover_note")}</span>
                    </span>
                  </label>
                  <label>
                    <input type="radio" name="takeover-way" checked={way === "force"} onChange={() => setWay("force")} disabled={busy} />
                    <span>{t("takeover.way_force")}</span>
                  </label>
                </fieldset>
              )}
              {held && (force || !onHandOver) && <p className="banner error small">{t("takeover.force_warning", { minutes: interval })}</p>}
              {!simple && (
                <>
                  <p className="request-line">
                    <button type="button" onClick={() => setEditing(true)} disabled={request === null || busy}>
                      {t("proceed.edit_request")}
                    </button>
                    {mine !== null && <span className="muted small">{t("proceed.edited")}</span>}
                  </p>
                  {takeOver && (
                    <p className="takeover-branch">
                      {working ? rich("takeover.branch", { branch: <code>{working}</code> }) : t("takeover.branch_unknown")}
                    </p>
                  )}
                  {takeOver && !held && fact.doing && <p className="takeover-doing">{doingText(fact, now, t)}</p>}
                  {takeOver && former === undefined && <p className="muted small">{t("takeover.changes_looking")}</p>}
                  {takeOver && former && <FormerChanges former={former} />}
                  {takeOver && former && hasUnpushed(former) && (
                    <div className="takeover-save">
                      <label>
                        <input type="checkbox" checked={save} onChange={(e) => setSave(e.currentTarget.checked)} disabled={busy} />
                        <span>{t("takeover.save")}</span>
                      </label>
                      <p className="muted small">{t("takeover.save_note")}</p>
                      {held && <p className="error-text small">{t("takeover.save_held_warning")}</p>}
                      {save && <p className="small">{t("takeover.save_added")}</p>}
                    </div>
                  )}
                  {takeOver && former === null && (
                    <p className="banner update small">
                      {t("takeover.worktree")} {working ? t("takeover.elsewhere") : ""}
                    </p>
                  )}
                  {!takeOver && task.state === "backlog" && <p className="banner update small">{t("proceed.backlog")}</p>}
                  <p>
                    {t(held ? "takeover.does_held" : takeOver ? "takeover.does" : task.state === "blocked" ? "proceed.does_blocked" : task.state === "waiting" ? "proceed.does_waiting" : "proceed.does")}
                  </p>
                  <p className="muted small">{t("proceed.how")}</p>
                  <AgentPicker agents={agents} slow={slow} choice={agent} onChoose={setAgent} disabled={busy} name="proceed-agent" />
                </>
              )}
            </>
          ) : (
            <div className="manual">
              <p className="small">{t("cards.manual_intro")}</p>
              <textarea
                className="manual-request"
                readOnly
                rows={12}
                aria-label={t("manual.request")}
                value={manual}
                onFocus={(e) => e.currentTarget.select()}
              />
            </div>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        <div className="row">
          {manual === null ? (
            <>
              <button type="button" onClick={() => onClose(null)} disabled={busy}>
                {t("wizard.cancel")}
              </button>
              {simple ? (
                <button type="button" className="primary" onClick={() => onHandOver?.()}>
                  {t("handover.button")}
                </button>
              ) : (
                <button
                  type="button"
                  className={held ? "primary danger" : "primary"}
                  onClick={() => void open()}
                  disabled={busy || !agent || request === null}
                >
                  {t(held ? "takeover.confirm_force" : "cards.open")}
                </button>
              )}
            </>
          ) : (
            <>
              <span className="copied-hint" role="status">
                {copied ? t("proceed.copied") : ""}
              </span>
              <button type="button" onClick={() => void copy()}>
                {t("manual.copy")}
              </button>
              <button type="button" className="primary" onClick={() => onClose(null)}>
                {t("manual.done")}
              </button>
            </>
          )}
        </div>

        {editing &&
          original !== null &&
          createPortal(
            <EditRequest
              id={task.id}
              text={request ?? original}
              original={original}
              onCancel={() => setEditing(false)}
              onSave={(text) => {
                setMine(edited(text, original));
                setEditing(false);
              }}
            />,
            document.body,
          )}
      </section>
    </div>
  );
}

/** The former worktree's changes not pushed (Task-A01-flag101): in a box of its own, titled, with
 * where the worktree is; it scrolls inside when long, so the dialog keeps its height. */
function FormerChanges({ former }: { former: FormerWorktree }) {
  const { t } = useI18n();
  return (
    <section className="former-changes" aria-labelledby="former-changes-title">
      <h3 id="former-changes-title">{t("takeover.changes_title")}</h3>
      <p className="muted small">{t("takeover.changes_where", { path: former.path })}</p>
      {!hasUnpushed(former) ? (
        <p className="small">{t("takeover.changes_none")}</p>
      ) : (
        <div className="former-list">
          {former.files.length > 0 && (
            <>
              <p className="small">{t("takeover.changes_files", { count: former.files.length })}</p>
              <ul>
                {former.files.map((f) => (
                  <li key={f.path}>
                    <span className={`file-kind k-${f.kind}`}>{t(`takeover.kind_${f.kind}`)}</span> <code>{f.path}</code>
                  </li>
                ))}
              </ul>
            </>
          )}
          {former.commits.length > 0 && (
            <>
              <p className="small">{t("takeover.changes_commits", { count: former.commits.length })}</p>
              <ul>
                {former.commits.map((c) => (
                  <li key={c.hash}>
                    <code>{c.hash}</code> {c.subject}
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </section>
  );
}

/** Edit the request…: the request in a popup of its own, editable. Use Allomos's request puts the
 * original back (greyed while the text is the original); Save keeps it for this dialog only. */
function EditRequest({
  id,
  text,
  original,
  onCancel,
  onSave,
}: {
  id: string;
  text: string;
  original: string;
  onCancel: () => void;
  onSave: (text: string) => void;
}) {
  const { t } = useI18n();
  const [value, setValue] = useState(text);
  return (
    <div className="overlay above" role="presentation">
      <section className="dialog card-list-dialog edit-request" role="dialog" aria-modal="true" aria-labelledby="edit-request-title">
        <h2 id="edit-request-title">{t("proceed.edit_title", { id })}</h2>
        <div className="dialog-body wizard-body">
          <p className="muted small">{t("proceed.edit_note")}</p>
          <textarea
            className="manual-request"
            rows={14}
            aria-label={t("manual.request")}
            value={value}
            autoFocus
            onChange={(e) => setValue(e.target.value)}
          />
        </div>
        <div className="row">
          <button type="button" className="link reset" disabled={value === original} onClick={() => setValue(original)}>
            {t("proceed.use_original")}
          </button>
          <button type="button" onClick={onCancel}>
            {t("wizard.cancel")}
          </button>
          <button type="button" className="primary" onClick={() => onSave(value)}>
            {t("proceed.save")}
          </button>
        </div>
      </section>
    </div>
  );
}
