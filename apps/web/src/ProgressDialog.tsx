// The progress popup (Task-A05-08, approver 2026-10-17). While an AI agent works on a job the app
// gave it (Connect with the agents, Update the taskbag…, Build a new app…), it reports with
// `alm progress`. The window looks about once a second and shows the report as a popup over
// everything: the job, the step, a bar when the agent counts its steps. **Go to background** puts
// it away; a chip in the corner brings it back. Finished, it says what happened and Close removes
// it; an agent silent for 10 minutes is named as such, so the window is never stuck. An
// `alm confirm` question shows on top (ConfirmDialog comes later in the window). From the moment
// the app opens an agent for a job (`agentOpened`), the popup says it is waiting for the agent,
// until the agent's first report takes its place (approver, 2026-10-17).

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import type { Progress } from "./types";
import { WAITING_ID, arrange, fraction, progressState, stillWaiting } from "./progressFlow";
import type { Job, Waiting } from "./progressFlow";
import type { Key } from "./i18n";
import { IconLoader } from "./IconLoader";

const JOB_EVENT = "allomos:agent-opened";

const JOB_TITLE: Record<Job, Key> = {
  connect: "progress.job_connect",
  update: "progress.job_update",
  move_in: "progress.job_move_in",
  cards: "progress.job_cards",
  build_app: "progress.job_build_app",
};

/** The app has opened an agent for `job` on the taskbag at `root`: the window's progress popup
 * shows "Waiting for the AI agent to respond…" at once. */
export function agentOpened(root: string, job: Job) {
  const detail: Waiting = { root, job, since: Date.now() / 1000 };
  window.dispatchEvent(new CustomEvent<Waiting>(JOB_EVENT, { detail }));
}

const LOOK_MS = 1000;

export function ProgressDialog({ messenger, project, root }: { messenger: Messenger; project: string; root: string }) {
  const { t } = useI18n();
  const [reports, setReports] = useState<Progress[]>([]);
  const [waiting, setWaiting] = useState<Waiting | null>(null);
  useEffect(() => {
    const opened = (e: Event) => {
      const w = (e as CustomEvent<Waiting>).detail;
      if (w.root !== root) return;
      setWaiting(w);
      setNow(w.since);
      setBackground((b) => {
        const next = new Set(b);
        next.delete(WAITING_ID);
        return next;
      });
    };
    window.addEventListener(JOB_EVENT, opened);
    return () => window.removeEventListener(JOB_EVENT, opened);
  }, [root]);
  useEffect(() => {
    if (waiting && !stillWaiting(waiting, reports)) setWaiting(null);
  }, [waiting, reports]);
  const [background, setBackground] = useState<Set<string>>(new Set());
  /** Quiet reports the person closed, with the time of the report they closed (see `arrange`). */
  const [closed, setClosed] = useState<Map<string, number>>(new Map());
  const [now, setNow] = useState(() => Date.now() / 1000);

  useEffect(() => {
    if (!project) return;
    let live = true;
    const look = () =>
      void messenger
        .progressReports(project)
        .then((list) => {
          if (!live) return;
          setReports(list);
          setNow(Date.now() / 1000);
        })
        .catch(() => undefined);
    look();
    const timer = window.setInterval(look, LOOK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, project]);

  const toBackground = (id: string) => setBackground((b) => new Set(b).add(id));
  const toFront = (id: string) =>
    setBackground((b) => {
      const next = new Set(b);
      next.delete(id);
      return next;
    });
  /** A quiet agent's popup: hidden, the report kept, so a later report brings it back. */
  const hideQuiet = (p: Progress) => {
    toFront(p.id);
    if (p.id === WAITING_ID) {
      setWaiting(null);
      return;
    }
    setClosed((c) => new Map(c).set(p.id, p.updated));
  };
  /** A finished job's popup: removed. */
  const close = (id: string) => {
    toFront(id);
    if (id === WAITING_ID) {
      setWaiting(null);
      return;
    }
    setReports((list) => list.filter((p) => p.id !== id));
    void messenger.dismissProgress(project, id).catch(() => undefined);
  };

  // Until the agent's first report, an entry of the window's own: waiting for it to respond.
  const shown: Progress[] = waiting
    ? [{ id: WAITING_ID, title: t(JOB_TITLE[waiting.job]), step: t("progress.waiting"), started: waiting.since, updated: waiting.since }, ...reports]
    : reports;
  const { popup, chips } = arrange(shown, background, closed);
  const count = (p: Progress) => (p.done !== undefined && p.of ? ` (${t("progress.count", { done: p.done, of: p.of })})` : "");

  return (
    <>
      {chips.length > 0 && (
        <div className="progress-chips">
          {chips.map((p) => (
            <button key={p.id} type="button" className="progress-chip" onClick={() => toFront(p.id)}>
              {progressState(p, now) === "working" && <IconLoader />}
              {progressState(p, now) === "finished"
                ? t("progress.chip_finished", { title: p.title })
                : `${p.title}: ${p.step ?? t("progress.starting")}${count(p)}`}
            </button>
          ))}
        </div>
      )}
      {popup && (
        <div className="overlay" role="presentation">
          <section className="dialog progress-dialog" role="dialog" aria-modal="true" aria-labelledby="progress-title">
            <h2 id="progress-title">{popup.title}</h2>
            {popup.id !== WAITING_ID && (
              <p className="muted">
                {progressState(popup, now) === "finished"
                  ? popup.from
                    ? t("progress.finished_from", { from: popup.from })
                    : t("progress.finished_unknown")
                  : popup.from
                    ? t("progress.from", { from: popup.from })
                    : t("progress.from_unknown")}
              </p>
            )}
            {progressState(popup, now) === "finished" ? (
              // The agent's own words (`alm progress --finished`), quoted as such.
              <blockquote className="progress-finished" role="status">
                <span aria-hidden="true">✓</span> <span>{popup.finished}</span>
              </blockquote>
            ) : (
              <>
                <p className="searching" role="status">
                  {(popup.id === WAITING_ID || progressState(popup, now) === "working") && <IconLoader />}
                  <span>
                    {popup.step ?? t("progress.starting")}
                    {count(popup)}
                  </span>
                </p>
                {fraction(popup) !== null && <progress className="progress-bar" value={fraction(popup) ?? 0} max={1} />}
                {progressState(popup, now) === "quiet" && <p className="banner error">{t("progress.quiet")}</p>}
              </>
            )}
            <div className="row">
              {progressState(popup, now) === "finished" ? (
                <button type="button" className="primary" onClick={() => close(popup.id)}>
                  {t("progress.close")}
                </button>
              ) : progressState(popup, now) === "quiet" ? (
                <button type="button" onClick={() => hideQuiet(popup)}>
                  {t("progress.close")}
                </button>
              ) : (
                <button type="button" onClick={() => toBackground(popup.id)}>
                  {t("progress.background")}
                </button>
              )}
            </div>
          </section>
        </div>
      )}
    </>
  );
}
