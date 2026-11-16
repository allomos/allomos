// Risk management (decision 0027 § 6, Task-A01-beetle749): the Risk management section of the
// taskbag's settings. From top to bottom: the fixed rules for what counts as high risk (H1 to H7,
// only shown: a project cannot remove them), the rules this project adds (one sentence each,
// written for the agent that classifies a task; added, changed and removed here), and the two
// limits, off by default. Saved in project.json's `risk`; every change is saved at once.

import { useEffect, useState } from "react";
import { useI18n, type Key } from "./i18n";
import type { Messenger } from "./messenger";
import { addRule, changeRule, removeRule, type RiskSettings } from "./riskFlow";
import { isProblem } from "./types";

export function RiskSection({ messenger, root }: { messenger: Messenger; root: string }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<RiskSettings | null>(null);
  const [adding, setAdding] = useState("");
  /** The rule being reworded: its place and the text typed so far. */
  const [editing, setEditing] = useState<{ index: number; text: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    messenger.riskSettings(root).then(setSettings, (e) => setError(isProblem(e) ? problem(e) : String(e)));
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  const run = async (change: Parameters<Messenger["setRiskSettings"]>[1]) => {
    setBusy(true);
    setError(null);
    try {
      setSettings(await messenger.setRiskSettings(root, change));
      return true;
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const rules = settings?.rules ?? [];
  const added = addRule(rules, adding);
  const add = async () => {
    if (added && (await run({ rules: added }))) setAdding("");
  };
  const changed = editing ? changeRule(rules, editing.index, editing.text) : null;
  const saveEdit = async () => {
    if (changed && (await run({ rules: changed }))) setEditing(null);
  };
  const off = busy || !settings;

  return (
    <div className="handoff-settings risk-settings">
      <p className="muted small">{t("risk_settings.body")}</p>

      <h3>{t("risk_settings.fixed")}</h3>
      <p className="muted small">{t("risk_settings.fixed_note")}</p>
      <ul className="risk-rules fixed">
        {(settings?.fixed ?? []).map((id) => (
          <li key={id}>
            <code>{id}</code>
            <span>{t(`risk_settings.${id.toLowerCase()}` as Key)}</span>
          </li>
        ))}
      </ul>

      <h3>{t("risk_settings.own")}</h3>
      <p className="muted small">{t("risk_settings.own_note")}</p>
      {settings && rules.length === 0 && <p className="muted small">{t("risk_settings.own_none")}</p>}
      {rules.length > 0 && (
        <ul className="risk-rules own">
          {rules.map((rule, index) =>
            editing?.index === index ? (
              <li key={index}>
                <input
                  type="text"
                  autoFocus
                  aria-label={t("risk_settings.rule_label")}
                  value={editing.text}
                  onChange={(e) => setEditing({ index, text: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.nativeEvent.isComposing) void saveEdit();
                    if (e.key === "Escape") {
                      e.stopPropagation();
                      setEditing(null);
                    }
                  }}
                  disabled={busy}
                />
                <button type="button" onClick={() => void saveEdit()} disabled={busy || !changed}>
                  {t("handoff_settings.save")}
                </button>
                <button type="button" className="link small" onClick={() => setEditing(null)} disabled={busy}>
                  {t("risk_settings.cancel")}
                </button>
              </li>
            ) : (
              <li key={index}>
                <span>{rule}</span>
                <button type="button" className="link small" onClick={() => setEditing({ index, text: rule })} disabled={off}>
                  {t("risk_settings.change")}
                </button>
                <button
                  type="button"
                  className="link small"
                  onClick={() => {
                    setEditing(null);
                    void run({ rules: removeRule(rules, index) });
                  }}
                  disabled={off}
                >
                  {t("handoff_settings.remove")}
                </button>
              </li>
            ),
          )}
        </ul>
      )}
      <div className="row start risk-add">
        <input
          type="text"
          aria-label={t("risk_settings.add_placeholder")}
          placeholder={t("risk_settings.add_placeholder")}
          value={adding}
          onChange={(e) => setAdding(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) void add();
          }}
          disabled={off}
        />
        <button type="button" onClick={() => void add()} disabled={off || !added}>
          {t("handoff_settings.add")}
        </button>
      </div>

      <h3>{t("risk_settings.limits")}</h3>
      <p className="muted small">{t("risk_settings.limits_note")}</p>
      <div className="form">
        <label className="choice">
          <input
            type="checkbox"
            checked={settings?.high_needs_human_check ?? false}
            onChange={(e) => void run({ highNeedsHumanCheck: e.target.checked })}
            disabled={off}
          />
          {t("risk_settings.human_check")}
        </label>
        <label className="choice">
          <input
            type="checkbox"
            checked={settings?.high_needs_approver_merge ?? false}
            onChange={(e) => void run({ highNeedsApproverMerge: e.target.checked })}
            disabled={off}
          />
          {t("risk_settings.approver_merge")}
        </label>
      </div>
      {error && <p className="banner error">{error}</p>}
    </div>
  );
}
