// Translate the taskbag… (Task-A05-board831, approver 2026-10-26: designs 1, 2 and 5), from the
// taskbag's settings → Tasks. Laid out like Update the taskbag…: the language, from the searchable
// language menu, with what the agent rewrites and that the task language changes too; then the
// agent (the shared picker, Manual included). Open opens the agent with the request typed in
// (`alm translate <code>` prints the same), and the window's progress popup follows the job.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, AgentPicker, PASTE_KEYS, useAgents } from "./AgentPicker";
import { MANUAL } from "./agentPickerFlow";
import { LanguageMenu } from "./LanguageMenu";
import { agentOpened } from "./ProgressDialog";
import { COPIED_MS } from "./manualFlow";
import { isProblem } from "./types";

export function TranslateDialog({
  messenger,
  root,
  onClose,
}: {
  messenger: Messenger;
  root: string;
  /** The dialog closes; `message` says what happened (null: Cancel). */
  onClose: (message: string | null) => void;
}) {
  const { t, problem, language } = useI18n();
  const [target, setTarget] = useState<{ code: string; name: string } | null>(null);
  const { agents, slow, choice: agent, setChoice: setAgent, tool } = useAgents(messenger);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** Manual: the request to copy. */
  const [request, setRequest] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);
  // Esc cancels, unless the language menu is open (it closes first).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || (e.target as Element | null)?.closest?.(".language-drop")) return;
      e.stopPropagation();
      onClose(null);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);
  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  const open = async () => {
    if (!agent || !target) return;
    setBusy(true);
    setError(null);
    try {
      if (agent === MANUAL) {
        setRequest(await messenger.translateRequest(root, language, target.code, target.name));
        return;
      }
      await messenger.translateTaskbag(agent, root, language, target.code, target.name);
      agentOpened(root, "translate");
      const name = t(AGENT_KEY[agent]);
      onClose(tool?.how === "paste" ? t("translate.asked_paste", { agent: name, keys: PASTE_KEYS }) : t("translate.asked", { agent: name }));
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
    <div className="overlay above" role="presentation" data-own-escape="">
      <section className="dialog wizard move-cards" role="dialog" aria-modal="true" aria-labelledby="translate-title">
        <h2 id="translate-title">{t("translate.title")}</h2>
        <div className="dialog-body wizard-body">
          {request === null ? (
            <>
              <h3>{t("translate.language")}</h3>
              <LanguageMenu
                value={target?.code ?? null}
                label={t("translate.language")}
                disabled={busy}
                onChange={(code, name) => setTarget({ code, name })}
              />
              <p className="muted small">{t("translate.what")}</p>
              <p className="small">
                <span className="folder-badge">{t("translate.tag")}</span>{" "}
                {target ? t("translate.sets_language", { language: target.name }) : t("translate.sets_language_any")}
              </p>
              <h3>{t("translate.agent")}</h3>
              <AgentPicker agents={agents} slow={slow} choice={agent} onChoose={setAgent} disabled={busy} name="translate-agent" />
            </>
          ) : (
            <div className="manual">
              <p className="small">{t("translate.manual_intro")}</p>
              <textarea className="manual-request" readOnly rows={12} aria-label={t("manual.request")} value={request} onFocus={(e) => e.currentTarget.select()} />
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
              <button type="button" className="primary" onClick={() => void open()} disabled={busy || !agent || !target}>
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
