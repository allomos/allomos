// Hand over to another agent… (Task-A01-minnow200): from the task pane of a held task. Three steps,
// worked out in handoverFlow.ts: choose the new agent (with its quota); ask the holding session,
// which sees the request at its next `alm` or at once when the person opens it and pastes the
// request; once it has released the task, open a new session that claims it. `task` is the task
// as it is now, so the dialog moves on by itself when the holder releases it.

import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, AgentPicker, PASTE_KEYS, useAgents } from "./AgentPicker";
import { MANUAL, requestWay } from "./agentPickerFlow";
import { canTakeOver } from "./proceedFlow";
import { agoText, heldFact } from "./heartbeatFlow";
import { handoverAskRequest, handoverClaimRequest, handoverLate, handoverStep, pendingHandover } from "./handoverFlow";
import { ageText, agentId, agentName, quotaOfChoice, tone } from "./quotaFlow";
import { titleOpen } from "./sessionFlow";
import { isProblem } from "./types";
import type { QuotaReport, Task } from "./types";

/** How long "Copied" shows (as in Work on it…). */
const COPIED_MS = 4000;

/** Hand over to another agent…, after Take over…: only for a held task that is not finished. */
export function HandoverButton({ task, onHandOver }: { task: Task; onHandOver: () => void }) {
  const { t } = useI18n();
  if (!canTakeOver(task)) return null;
  return (
    <button
      type="button"
      className="link with-icon proceed handover"
      title={t(pendingHandover(task) ? "handover.button_title_asked" : "handover.button_title")}
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
  interval,
  lastChange,
  working,
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
  /** The project's heartbeat interval, in minutes: how long the holder may take to see the request. */
  interval: number;
  /** When the task last changed (UTC), for a holder that never beat. */
  lastChange?: string;
  /** The task's working branch, from the history; null when none is recorded. */
  working: string | null;
  /** Whether this window can open agent sessions. */
  canOpenSession: boolean;
  /** Use Take over… instead: the holder did not release the task. */
  onTakeOver: () => void;
  /** The dialog closes; `message` says what happened (null: Cancel or Close). */
  onClose: (message: string | null) => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 15_000);
    return () => window.clearInterval(id);
  }, []);
  const already = pendingHandover(task);
  /** The holder asked to hand over; opened again while a request is in force, that one. */
  const [asked, setAsked] = useState<string | null>(already?.holder ?? null);
  // The holder when the dialog opened: if it lets go before the person asks, the dialog moves on too.
  const [first] = useState(task.holder ?? null);
  const step = handoverStep(task, asked ?? (task.holder !== first ? first : null));
  const { agents, slow, choice: agent, setChoice: setAgent, tool } = useAgents(messenger);
  // Opened again: the agent the request names, once the agents are found.
  const [wanted] = useState(already?.to ?? null);
  useEffect(() => {
    if (!agents || !wanted) return;
    if (wanted === MANUAL || agents.some((a) => a.id === wanted)) setAgent(wanted as typeof agent & string);
    // Only when the agents arrive.
  }, [agents]);
  const [reports, setReports] = useState<QuotaReport[]>([]);
  useEffect(() => {
    let live = true;
    messenger.quotaReports().then(
      (r) => live && setReports(r),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [messenger]);
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [manual, setManual] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  /** Open its session was pressed: the request is on the clipboard. */
  const [opened, setOpened] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };
  const copy = async (text: string) => {
    await messenger.copyText(text);
    setCopied(true);
    window.clearTimeout(copiedTimer.current);
    copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
  };

  const holder = asked ?? first ?? "";
  const strong = <strong>{holder}</strong>;
  const named = <strong>{task.id}</strong>;
  const stateWord = <span className={`state-word tint s-${task.state}-text`}>{t(`state.${task.state}`)}</span>;
  const fact = heldFact(task, lostAfter, now, lastChange);
  const silence = fact && (fact.minutes !== null ? t("takeover.silent_for", { ago: agoText(fact.minutes, t) }) : t("takeover.never_beat"));
  const askText = handoverAskRequest(task, language);
  const canOpen = !!titleOpen(task, canOpenSession);
  const late = handoverLate(pendingHandover(task), interval, now);
  const branch = <p className="takeover-branch">{working ? rich("takeover.branch", { branch: <code>{working}</code> }) : t("takeover.branch_unknown")}</p>;
  const quota = agent && agent !== MANUAL ? quotaOfChoice(reports, agent) : null;
  const age = quota ? ageText(Math.max(0, Math.floor((now.getTime() - Date.parse(quota.at)) / 60_000))) : null;
  const picker = <AgentPicker agents={agents} slow={slow} choice={agent} onChoose={setAgent} disabled={busy} name="handover-agent" />;
  const quotaLine =
    agent && agent !== MANUAL ? (
      <p className="small handover-quota">
        {quota && age ? (
          <span className={`quota-${tone(quota.left)}`}>
            {t("handover.quota", { name: agentName(agentId(agent)), left: quota.left, age: t(age.key, { [age.key === "agentline.age" ? "minutes" : age.key === "agentline.age_hours" ? "hours" : "days"]: age.n }) })}
          </span>
        ) : (
          <span className="muted">{t("handover.quota_none", { name: agentName(agentId(agent)) })}</span>
        )}
      </p>
    ) : null;

  const ask = () =>
    run(async () => {
      if (!agent || !task.holder) return;
      const named = task.holder;
      await messenger.requestHandover(root, task.id, agent);
      setAsked(named);
    });
  const cancel = () =>
    run(async () => {
      await messenger.cancelHandover(root, task.id);
      onClose(t("handover.cancelled", { id: task.id }));
    });
  const openOld = () =>
    run(async () => {
      await messenger.copyText(askText);
      await messenger.openAgentSession(root, task.id);
      setOpened(true);
    });
  const openNew = () =>
    run(async () => {
      if (!agent || taskbag === null) return;
      const request = handoverClaimRequest(task, taskbag, language, holder, working);
      if (agent === MANUAL) {
        setManual(await messenger.proceedManualRequest(request));
        return;
      }
      await messenger.proceedWithAgent(agent, root, request);
      const name = t(AGENT_KEY[agent]);
      const way = requestWay(tool);
      onClose(t(way === "typed" ? "takeover.asked" : `takeover.asked_${way}`, { agent: name, keys: PASTE_KEYS, id: task.id }));
    });

  const n = step === "choose" ? 1 : step === "wait" ? 2 : 3;
  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard move-cards proceed handover" role="dialog" aria-modal="true" aria-labelledby="handover-title">
        <h2 id="handover-title">{t("handover.title")}</h2>
        <div className="dialog-body wizard-body">
          {step !== "gone" && manual === null && <p className="muted small">{t("handover.step", { n })}</p>}
          {manual !== null ? (
            <div className="manual">
              <p className="small">{t("cards.manual_intro")}</p>
              <textarea className="manual-request" readOnly rows={12} aria-label={t("manual.request")} value={manual} onFocus={(e) => e.currentTarget.select()} />
            </div>
          ) : step === "choose" ? (
            <>
              <p>{rich("handover.found", { id: named, title: task.title, state: stateWord, holder: strong, ago: silence ?? "" })}</p>
              {branch}
              <p>{rich("handover.does", { holder: strong })}</p>
              <h3>{t("handover.choose")}</h3>
              {picker}
              {quotaLine}
            </>
          ) : step === "wait" ? (
            <>
              <p>{rich("handover.wait", { holder: strong, minutes: interval })}</p>
              <p className="handover-waiting" role="status">
                {rich("handover.waiting", { holder: strong })}
              </p>
              <p className="muted small">{t("handover.other_computer")}</p>
              <h3>{t("handover.sooner")}</h3>
              <p className="small">{t(canOpen ? "handover.sooner_open" : "handover.sooner_copy")}</p>
              <p className="request-line">
                {canOpen && (
                  <button type="button" onClick={() => void openOld()} disabled={busy}>
                    {t("handover.open_session")}
                  </button>
                )}
                <button type="button" onClick={() => void run(() => copy(askText))} disabled={busy}>
                  {t("handover.copy_request")}
                </button>
                {copied && <span className="muted small">{t("proceed.copied")}</span>}
              </p>
              {opened && <p className="muted small">{t("handover.opened", { keys: PASTE_KEYS })}</p>}
              {late && (
                <p className="banner update small">
                  <span>{rich("handover.late", { holder: strong, minutes: interval })}</span>{" "}
                  <button type="button" className="link" onClick={onTakeOver}>
                    {t("takeover.button")}
                  </button>
                </p>
              )}
            </>
          ) : step === "open" ? (
            <>
              <p>{rich("handover.released", { holder: strong, id: named })}</p>
              {task.handoff_note && <blockquote className="handover-note small">{task.handoff_note}</blockquote>}
              {branch}
              <p>{t("handover.open_does")}</p>
              {picker}
              {quotaLine}
            </>
          ) : (
            <p>
              {task.holder && task.state !== "done" && task.state !== "dropped"
                ? rich("handover.gone_held", { id: named, holder: strong, other: <strong>{task.holder}</strong> })
                : rich("handover.gone_finished", { id: named, state: stateWord })}
            </p>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        <div className="row">
          {manual !== null ? (
            <>
              <span className="copied-hint" role="status">
                {copied ? t("proceed.copied") : ""}
              </span>
              <button type="button" onClick={() => void run(() => copy(manual))}>
                {t("manual.copy")}
              </button>
              <button type="button" className="primary" onClick={() => onClose(null)}>
                {t("manual.done")}
              </button>
            </>
          ) : step === "choose" ? (
            <>
              <button type="button" onClick={() => onClose(null)} disabled={busy}>
                {t("wizard.cancel")}
              </button>
              <button type="button" className="primary" onClick={() => void ask()} disabled={busy || !agent}>
                {t("handover.ask")}
              </button>
            </>
          ) : step === "wait" ? (
            <>
              <button type="button" onClick={() => void cancel()} disabled={busy}>
                {t("handover.cancel_request")}
              </button>
              <button type="button" className="primary" onClick={() => onClose(t("handover.still_asked", { id: task.id }))} disabled={busy}>
                {t("handover.close")}
              </button>
            </>
          ) : step === "open" ? (
            <>
              <button type="button" onClick={() => onClose(null)} disabled={busy}>
                {t("wizard.cancel")}
              </button>
              <button type="button" className="primary" onClick={() => void openNew()} disabled={busy || !agent || taskbag === null}>
                {t("cards.open")}
              </button>
            </>
          ) : (
            <button type="button" className="primary" onClick={() => onClose(null)}>
              {t("manual.done")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
