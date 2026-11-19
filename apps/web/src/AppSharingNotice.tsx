import { useEffect, useState } from "react";
import { fieldName } from "./historyFlow";
import { useI18n } from "./i18n";
import { AgentPicker, AGENT_KEY, PASTE_KEYS, useAgents } from "./AgentPicker";
import { MANUAL, requestWay } from "./agentPickerFlow";
import type { Messenger } from "./messenger";
import type { Opened } from "./types";
import { isProblem } from "./types";

/** The summary follows the same taskbag reload as the task list; opening an agent never
 * clears it. Only the agent's explicit shared event does. */
export function AppSharingNotice({ messenger, opened }: { messenger: Messenger; opened: Opened }) {
  const { t } = useI18n();
  const [asking, setAsking] = useState(false);
  const [app, setApp] = useState(opened.app_changes);
  useEffect(() => {
    let live = true;
    // An in-place task edit advances its stamp without a full taskbag reload. Read just the
    // sharing summary in the background as well, so the very first edit shows the notice.
    void messenger.appChanges(opened.root).then((next) => { if (live) setApp(next); }).catch(() => {});
    return () => { live = false; };
  }, [messenger, opened]);
  if (!app?.count) return null;
  const titles = new Map(opened.loaded.taskbag?.tasks.map((task) => [task.id, task.title]));
  return <><section className="banner update app-sharing" aria-label={t("sharing.count", { count: app.count })}>
    <details>
      <summary>{t("sharing.count", { count: app.count })}</summary>
      <ul>{app.entries.map((e) => <li key={e.event}>
        {e.id ? [e.id, titles.get(e.id)].filter(Boolean).join(" · ") : t("sharing.project")} · {e.fields.map((f) => fieldName(f, t)).join(", ")} · <time dateTime={e.at}>{new Date(e.at).toLocaleString()}</time>
      </li>)}</ul>
    </details>
    <button type="button" onClick={() => setAsking(true)}>{t("sharing.ask")}</button>
  </section>
    {asking && <SharingDialog messenger={messenger} root={opened.root} onClose={() => setAsking(false)} />}
  </>;
}

function SharingDialog({ messenger, root, onClose }: { messenger: Messenger; root: string; onClose: () => void }) {
  const { t, problem, language } = useI18n();
  const { agents, slow, choice, setChoice, tool } = useAgents(messenger);
  const [busy, setBusy] = useState(false);
  const [request, setRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy) { e.stopPropagation(); onClose(); }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [busy, onClose]);
  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  const open = async () => {
    if (!choice) return;
    setBusy(true); setError(null);
    try {
      if (choice === MANUAL) setRequest(await messenger.shareAppChangesRequest(root, language));
      else {
        await messenger.shareAppChanges(choice, root, language);
        const way = requestWay(tool);
        setMessage(t(way === "typed" ? "sharing.asked" : way === "paste" ? "sharing.asked_paste" : "sharing.asked_sent", { agent: t(AGENT_KEY[choice]), keys: PASTE_KEYS }));
      }
    } catch (e) { fail(e); }
    finally { setBusy(false); }
  };
  return <div className="overlay" role="presentation"><section className="dialog" role="dialog" aria-modal="true" aria-labelledby="sharing-title">
    <h2 id="sharing-title">{t("sharing.ask")}</h2>
    <p>{t("sharing.explain")}</p>
    {error && <p role="alert">{error}</p>}
    {message ? <p role="status">{message}</p> : request !== null ? <div className="manual">
      <textarea className="manual-request" aria-label={t("sharing.request")} readOnly value={request} />
      <button type="button" onClick={() => { void messenger.copyText(request).then(() => setCopied(true)).catch(fail); }}>{t(copied ? "manual.copied" : "manual.copy")}</button>
    </div> : <AgentPicker agents={agents} slow={slow} choice={choice} onChoose={setChoice} disabled={busy} name="sharing-agent" />}
    <div className="actions">
      <button type="button" onClick={onClose} disabled={busy}>{t(message || request !== null ? "manual.done" : "wizard.cancel")}</button>
      {!message && request === null && <button type="button" className="primary" onClick={() => void open()} disabled={busy || !choice}>{t("cards.open")}</button>}
    </div>
  </section></div>;
}
