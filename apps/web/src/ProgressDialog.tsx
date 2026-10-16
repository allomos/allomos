// The progress popup (Task-A05-08, approver 2026-10-17). While an AI agent works on a job the app
// gave it (Connect with the agents, Update the taskbag…, Build a new app…), it reports with
// `alm progress`. The window looks about once a second and shows the report as a popup over
// everything: the job, the step, a bar when the agent counts its steps. **Go to background** puts
// it away; a chip in the corner brings it back. Finished, it says what happened and Close removes
// it; an agent silent for 10 minutes is named as such, so the window is never stuck. An
// `alm confirm` question shows on top (ConfirmDialog comes later in the window).

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import type { Progress } from "./types";
import { arrange, fraction, progressState } from "./progressFlow";

const LOOK_MS = 1000;

export function ProgressDialog({ messenger, project }: { messenger: Messenger; project: string }) {
  const { t } = useI18n();
  const [reports, setReports] = useState<Progress[]>([]);
  const [background, setBackground] = useState<Set<string>>(new Set());
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
  const close = (id: string) => {
    setReports((list) => list.filter((p) => p.id !== id));
    toFront(id);
    void messenger.dismissProgress(project, id).catch(() => undefined);
  };

  const { popup, chips } = arrange(reports, background);
  const count = (p: Progress) => (p.done !== undefined && p.of ? ` (${t("progress.count", { done: p.done, of: p.of })})` : "");

  return (
    <>
      {chips.length > 0 && (
        <div className="progress-chips">
          {chips.map((p) => (
            <button key={p.id} type="button" className="progress-chip" onClick={() => toFront(p.id)}>
              {progressState(p, now) === "working" && <span className="spinner" aria-hidden="true" />}{" "}
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
            <p className="muted">{popup.from ? t("progress.from", { from: popup.from }) : t("progress.from_unknown")}</p>
            {progressState(popup, now) === "finished" ? (
              <p className="banner update" role="status">
                {popup.finished}
              </p>
            ) : (
              <>
                <p role="status">
                  {progressState(popup, now) === "working" && <span className="spinner" aria-hidden="true" />}{" "}
                  {popup.step ?? t("progress.starting")}
                  {count(popup)}
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
              ) : (
                <>
                  {progressState(popup, now) === "quiet" && (
                    <button type="button" onClick={() => close(popup.id)}>
                      {t("progress.close")}
                    </button>
                  )}
                  <button type="button" className="primary" onClick={() => toBackground(popup.id)}>
                    {t("progress.background")}
                  </button>
                </>
              )}
            </div>
          </section>
        </div>
      )}
    </>
  );
}
