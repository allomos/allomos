// The popup for `alm confirm` (Task-A05-02): an AI agent asks the person before it changes the
// project, for example before moving the project's tasks into Allomos. The window looks for
// questions about the folder it shows about once a second; the person reads the agent's plan and
// chooses Go ahead or Cancel, and `alm confirm` hands that answer back to the agent.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { ConfirmRequest } from "./types";

/** How often the window looks for questions. `alm confirm` opens the app if none is seen in 3 s. */
const LOOK_MS = 1000;

export function ConfirmDialog({
  messenger,
  project,
  onCancelled,
}: {
  messenger: Messenger;
  project: string;
  /** The person chose Cancel: the window says nothing changed and how to ask again. */
  onCancelled?: () => void;
}) {
  const { t, problem } = useI18n();
  const [request, setRequest] = useState<ConfirmRequest | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!project) return;
    let live = true;
    const look = () =>
      void messenger
        .confirmRequests(project)
        .then((list) => {
          if (!live) return;
          // Keep showing the same question while it waits; drop it when it is settled or expired.
          setRequest((shown) => list.find((r) => r.id === shown?.id) ?? list[0] ?? null);
        })
        .catch(() => undefined);
    look();
    const timer = window.setInterval(look, LOOK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, project]);

  if (!request) return null;

  const answer = async (yes: boolean) => {
    setBusy(true);
    setError(null);
    try {
      await messenger.answerConfirm(project, request.id, yes, request.fingerprint);
      setRequest(null);
      if (!yes) onCancelled?.();
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" role="presentation">
      <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
        <h2 id="confirm-title">{request.title}</h2>
        <p className="muted">
          {request.from ? t("confirm.from", { from: request.from }) : t("confirm.from_unknown")}
        </p>
        {request.task && <p className="muted small">{t("confirm.for_task", { id: request.task })}</p>}
        <div className="dialog-body">
          <Markdown source={request.plan} onLink={() => undefined} />
          {request.evidence && (
            <>
              <h3>{t("confirm.evidence")}</h3>
              <Markdown source={request.evidence} onLink={() => undefined} />
            </>
          )}
        </div>
        <p className="muted small">{t("confirm.scope")}</p>
        {error && <p className="banner error">{error}</p>}
        <div className="row">
          <button type="button" onClick={() => void answer(false)} disabled={busy}>
            {t("confirm.cancel")}
          </button>
          <button type="button" className="primary" onClick={() => void answer(true)} disabled={busy}>
            {t("confirm.go")}
          </button>
        </div>
      </section>
    </div>
  );
}
