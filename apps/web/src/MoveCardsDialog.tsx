// Update the taskbag… (Task-A00-30; named so by the approver, 2026-10-17): from the notice "N tasks
// keep their card files in the project folder", or File → Update the Taskbag…. The dialog first
// says what Allomos found (each task, its title and where its card file is now) and what updating
// does. The person chooses an agent (the ones found, or Manual);
// Allomos opens it in the project folder with the move-cards request typed in, and sends nothing.
// The agent finds the card files, shows its plan in Allomos with `alm confirm`, and changes
// nothing until the person chooses Go ahead. Manual shows the same request with Copy.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, PASTE_KEYS } from "./AgentsPanel";
import { COPIED_MS } from "./manualFlow";
import { MANUAL, agentChoices, firstAgent } from "./buildAppFlow";
import { isProblem } from "./types";
import type { CardRow } from "./moveCardsFlow";
import type { Agent } from "./types";

export function MoveCardsDialog({
  messenger,
  root,
  found,
  onClose,
}: {
  messenger: Messenger;
  /** The open taskbag. */
  root: string;
  /** What Allomos found: the tasks whose card file is still outside the taskbag. */
  found: CardRow[];
  /** The dialog closes; `message` says what happened (null: Cancel). */
  onClose: (message: string | null) => void;
}) {
  const { t, problem, language } = useI18n();
  const [agents, setAgents] = useState<Agent[] | null>(null);
  const [agent, setAgent] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** Manual: the request to copy. */
  const [request, setRequest] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  useEffect(() => {
    let live = true;
    messenger
      .findAgents()
      .then((found) => {
        if (!live) return;
        setAgents(found);
        setAgent(firstAgent(found));
      })
      .catch(() => {
        if (!live) return;
        setAgents([]);
        setAgent(MANUAL);
      });
    return () => {
      live = false;
    };
  }, [messenger]);

  const tool = agents?.find((a) => a.id === agent) ?? null;

  const open = async () => {
    if (!agent) return;
    setBusy(true);
    setError(null);
    try {
      if (agent === MANUAL) {
        setRequest(await messenger.moveCardsRequest(root, language));
        return;
      }
      await messenger.moveCards(agent, root, language);
      const name = t(AGENT_KEY[agent as Agent["id"]]);
      onClose(tool?.how === "paste" ? t("cards.asked_paste", { agent: name, keys: PASTE_KEYS }) : t("cards.asked", { agent: name }));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (request === null) return;
    setError(null);
    try {
      await messenger.copyText(request);
      setCopied(true);
      window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
    } catch (e) {
      fail(e);
    }
  };

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard" role="dialog" aria-modal="true" aria-labelledby="cards-title">
        <h2 id="cards-title">{t("cards.title")}</h2>
        {request === null && (
          <>
            <p>{found.length > 0 ? t("cards.found", { count: found.length }) : t("cards.found_none")}</p>
            {found.length > 0 && (
              <ul className="card-found small">
                {found.map((c) => (
                  <li key={c.task}>
                    <strong>{c.task}</strong> {c.title} — <code>{c.path}</code>
                    {c.reported && <span className="muted"> ({t("cards.reported")})</span>}
                  </li>
                ))}
              </ul>
            )}
            <p>{t("cards.update")}</p>
            <p className="muted small">{t("cards.body")}</p>
          </>
        )}

        <div className="dialog-body wizard-body">
          {request === null ? (
            agents === null ? (
              <p className="muted searching" role="status">
                <span className="spinner" aria-hidden="true" /> {t("agents.searching")}
              </p>
            ) : (
              <ul className="agent-list">
                {agentChoices(agents).map((id) => {
                  const found = agents.find((a) => a.id === id);
                  return (
                    <li key={id}>
                      <label>
                        <input type="radio" name="cards-agent" checked={agent === id} onChange={() => setAgent(id)} disabled={busy} />{" "}
                        <strong>{id === MANUAL ? t("agents.manual") : t(AGENT_KEY[id as Agent["id"]])}</strong>{" "}
                        <span className="muted">
                          —{" "}
                          {id === MANUAL
                            ? t("agents.manual_how")
                            : `${t(found?.how === "link" ? "agents.how_link" : "agents.how_paste")} (${found?.found ?? ""})`}
                        </span>
                      </label>
                      {id === "claude_app" && agent === id && <p className="muted small hint">{t("agents.claude_app_trust")}</p>}
                      {found?.how === "paste" && agent === id && <p className="small hint paste-tip">{t("agents.paste_tip", { keys: PASTE_KEYS })}</p>}
                    </li>
                  );
                })}
              </ul>
            )
          ) : (
            <div className="manual">
              <p className="small">{t("cards.manual_intro")}</p>
              <textarea
                className="manual-request"
                readOnly
                rows={12}
                aria-label={t("manual.request")}
                value={request}
                onFocus={(e) => e.currentTarget.select()}
              />
              <div className="row start">
                <button type="button" onClick={() => void copy()}>
                  {t("manual.copy")}
                </button>
                <span className="muted small" role="status">
                  {copied ? t("manual.copied") : ""}
                </span>
              </div>
            </div>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        <div className="row">
          {request === null ? (
            <>
              <button type="button" onClick={() => onClose(null)} disabled={busy}>
                {t("wizard.cancel")}
              </button>
              <button type="button" className="primary" onClick={() => void open()} disabled={busy || !agent}>
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
