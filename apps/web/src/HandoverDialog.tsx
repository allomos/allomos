// Prepare a handoff… (Task-A01-kind858): from the task pane of a held task. The dialog shows the
// prompt for the holding session (handoverFlow.ts), copies it, and opens that session so the person
// pastes it there and sends it. It writes nothing to the taskbag, shows no progress and can be
// closed at any time; who goes on with the task is started afterwards with another button (Work on
// it… once released, Take over… when the session is gone). `task` is the task as it is now.

import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { PASTE_KEYS } from "./AgentPicker";
import { canTakeOver } from "./proceedFlow";
import { agoText, heldFact } from "./heartbeatFlow";
import { handoverAskRequest, handoverOpen } from "./handoverFlow";
import { isProblem } from "./types";
import type { Task } from "./types";

/** How long "Copied" shows (as in Work on it…). */
const COPIED_MS = 4000;

/** Prepare a handoff…, after Take over…: only for a held task that is not finished. */
export function HandoverButton({ task, onHandOver }: { task: Task; onHandOver: () => void }) {
  const { t } = useI18n();
  if (!canTakeOver(task)) return null;
  return (
    <button
      type="button"
      className="link with-icon proceed handover"
      title={t("handover.button_title")}
      onKeyDown={(e: KeyboardEvent) => e.stopPropagation()}
      onClick={(e) => {
        e.stopPropagation();
        onHandOver();
      }}
    >
      <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
        <path d="M6.5 4.5v11l9-5.5z" />
      </svg>
      {t("handover.button")}
    </button>
  );
}

export function HandoverDialog({
  messenger,
  root,
  task,
  lostAfter,
  lastChange,
  canOpenSession,
  onTakeOver,
  onClose,
}: {
  messenger: Messenger;
  /** The open taskbag. */
  root: string;
  /** The task as it is now. */
  task: Task;
  /** The project's lost limit, in minutes. */
  lostAfter: number;
  /** When the task last changed (UTC), for a holder that never beat. */
  lastChange?: string;
  /** Whether this window can open agent sessions. */
  canOpenSession: boolean;
  /** Use Take over… instead: the holder looks gone. */
  onTakeOver: () => void;
  onClose: () => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 15_000);
    return () => window.clearInterval(id);
  }, []);
  // The holder when the dialog opened: the prompt is for that session.
  const [holder] = useState(task.holder ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  /** Open its session was pressed: the prompt is on the clipboard. */
  const [opened, setOpened] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };
  const prompt = handoverAskRequest(task, language);
  const copy = () =>
    run(async () => {
      await messenger.copyText(prompt);
      setCopied(true);
      window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
    });
  // Opening copies the prompt too, so the person only has to paste it.
  const open = () =>
    run(async () => {
      await messenger.copyText(prompt);
      await messenger.openAgentSession(root, task.id);
      setOpened(true);
    });

  const strong = <strong>{holder}</strong>;
  const named = <strong>{task.id}</strong>;
  const stateWord = <span className={`state-word tint s-${task.state}-text`}>{t(`state.${task.state}`)}</span>;
  const finished = task.state === "done" || task.state === "dropped";
  const still = !finished && task.holder === holder && !!holder;
  const fact = still ? heldFact(task, lostAfter, now, lastChange) : null;
  const silence = fact && (fact.minutes !== null ? t("takeover.silent_for", { ago: agoText(fact.minutes, t) }) : t("takeover.never_beat"));
  const gone = !!fact && (fact.at === null || fact.lost);
  const way = handoverOpen(task, canOpenSession);

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard move-cards proceed handover" role="dialog" aria-modal="true" aria-labelledby="handover-title">
        <h2 id="handover-title">{t("handover.title")}</h2>
        <div className="dialog-body wizard-body">
          {!still ? (
            <p>
              {finished
                ? rich("handover.gone_finished", { id: named, state: stateWord })
                : task.holder
                  ? rich("handover.gone_held", { id: named, holder: strong, other: <strong>{task.holder}</strong> })
                  : rich("handover.released", { holder: strong, id: named })}
            </p>
          ) : (
            <>
              <p>{rich("handover.found", { id: named, title: task.title, state: stateWord, holder: strong, ago: silence ?? "" })}</p>
              {gone && (
                <p className="banner update small">
                  <span>{rich("handover.gone_holder", { holder: strong })}</span>{" "}
                  <button type="button" className="link" onClick={onTakeOver}>
                    {t("takeover.button")}
                  </button>
                </p>
              )}
              <ol className="small handover-steps">
                <li>{t("handover.step_copy")}</li>
                <li>{t("handover.step_open")}</li>
                <li>{t("handover.step_paste", { keys: PASTE_KEYS })}</li>
              </ol>
              <textarea className="manual-request" readOnly rows={10} aria-label={t("handover.prompt")} value={prompt} onFocus={(e) => e.currentTarget.select()} />
              <p className="request-line">
                <button type="button" onClick={() => void copy()} disabled={busy}>
                  {t("handover.copy_request")}
                </button>
                <button type="button" onClick={() => void open()} disabled={busy || !way.can}>
                  {t("handover.open_session")}
                </button>
                {copied && <span className="muted small">{t("handover.copied")}</span>}
              </p>
              {!way.can && <p className="muted small">{t(`handover.cannot_open_${way.why}`)}</p>}
              {opened && <p className="muted small">{t("handover.opened", { keys: PASTE_KEYS })}</p>}
              <p className="muted small">{t("handover.after")}</p>
            </>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        <div className="row">
          <button type="button" className="primary" onClick={onClose}>
            {t("handover.close")}
          </button>
        </div>
      </section>
    </div>
  );
}
