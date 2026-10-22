// Handoff settings (Task-A03-06): when agents must write a handoff, the quota and context gates
// (`alm handoff-rules`). Opened from File → Handoff Settings… (Task-A03-polar770: the Handoffs tab
// that had its button is gone; a handoff lives in its task, as its handoff note).

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { HandoffSettings } from "./types";

/** When agents must write a handoff (owner, 2026-10-12: in the app and `alm`). */
export function HandoffSettingsDialog({ messenger, root, onClose }: { messenger: Messenger; root: string; onClose: () => void }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<HandoffSettings | null>(null);
  const [quota, setQuota] = useState("");
  const [context, setContext] = useState("");
  const [writeWhen, setWriteWhen] = useState("");
  const [agent, setAgent] = useState("");
  const [agentQuota, setAgentQuota] = useState("");
  const [agentContext, setAgentContext] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const fill = (s: HandoffSettings) => {
    setSettings(s);
    setQuota(String(s.applied.quota_gate));
    setContext(String(s.applied.context_gate));
    setWriteWhen(s.applied.write_when);
  };
  useEffect(() => {
    messenger.handoffSettings(root).then(fill, (e) => setError(isProblem(e) ? problem(e) : String(e)));
    // Read once when the dialog opens; the language only words an error.
  }, [messenger, root]);

  const run = async (work: () => Promise<HandoffSettings>) => {
    setBusy(true);
    setError(null);
    try {
      fill(await work());
      return true;
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    const ok = await run(() => messenger.setHandoffRules(root, { quota, context, writeWhen }));
    if (ok) onClose();
  };
  const addOverride = () =>
    void run(() =>
      messenger.setHandoffRules(root, {
        agent,
        quota: agentQuota || undefined,
        context: agentContext || undefined,
      }),
    ).then((ok) => {
      if (ok) {
        setAgent("");
        setAgentQuota("");
        setAgentContext("");
      }
    });
  const overrides = Object.entries(settings?.rules.overrides ?? {});
  // An override's empty gate follows the project's: show that value too, so 5% is not a mystery.
  const gate = (v: number | undefined, project: number | undefined) =>
    v === undefined ? t("handoff_settings.project_value", { value: project ?? "—" }) : `${v}%`;

  return (
    <div className="overlay" role="presentation">
      <section className="dialog handoff-settings" role="dialog" aria-modal="true" aria-labelledby="handoff-settings-title">
        <h2 id="handoff-settings-title">{t("handoff_settings.title")}</h2>
        <p className="muted">{t("handoff_settings.body")}</p>
        <div className="dialog-body form">
          <label>
            {t("handoff_settings.quota")}
            <span className="inline">
              <input type="number" min={0} max={100} value={quota} onChange={(e) => setQuota(e.target.value)} disabled={busy} /> %
            </span>
          </label>
          <label>
            {t("handoff_settings.context")}
            <span className="inline">
              <input type="number" min={0} max={100} value={context} onChange={(e) => setContext(e.target.value)} disabled={busy} /> %
            </span>
          </label>
          <label>
            {t("handoff_settings.write_when")}
            <input type="text" value={writeWhen} onChange={(e) => setWriteWhen(e.target.value)} disabled={busy} />
          </label>
          <h3>{t("handoff_settings.overrides")}</h3>
          <p className="muted small">{t("handoff_settings.overrides_body")}</p>
          {overrides.length > 0 && (
            <ul className="overrides">
              {overrides.map(([name, g]) => (
                <li key={name}>
                  <code>{name}</code> · {t("handoff_settings.quota_label")} {gate(g.quota_gate, settings?.applied.quota_gate)} · {t("handoff_settings.context_label")}{" "}
                  {gate(g.context_gate, settings?.applied.context_gate)}{" "}
                  <button type="button" className="link small" onClick={() => void run(() => messenger.setHandoffRules(root, { agent: name, clear: true }))} disabled={busy}>
                    {t("handoff_settings.remove")}
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="row start override-row">
            <input type="text" placeholder={t("handoff_settings.agent_placeholder")} value={agent} onChange={(e) => setAgent(e.target.value)} disabled={busy} />
            <input type="number" min={0} max={100} placeholder={t("handoff_settings.quota_short")} value={agentQuota} onChange={(e) => setAgentQuota(e.target.value)} disabled={busy} />
            <input type="number" min={0} max={100} placeholder={t("handoff_settings.context_short")} value={agentContext} onChange={(e) => setAgentContext(e.target.value)} disabled={busy} />
            <button type="button" onClick={addOverride} disabled={busy || !agent.trim() || (!agentQuota && !agentContext)}>
              {t("handoff_settings.add")}
            </button>
          </div>
        </div>
        {error && <p className="banner error">{error}</p>}
        <div className="row">
          <button type="button" onClick={() => void run(() => messenger.setHandoffRules(root, { clear: true }))} disabled={busy}>
            {t("handoff_settings.defaults")}
          </button>
          <button type="button" onClick={onClose} disabled={busy}>
            {t("handoff_settings.cancel")}
          </button>
          <button type="button" className="primary" onClick={() => void save()} disabled={busy || !settings}>
            {t("handoff_settings.save")}
          </button>
        </div>
      </section>
    </div>
  );
}
