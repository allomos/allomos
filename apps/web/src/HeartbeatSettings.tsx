// Heartbeat settings (Task-A01-osprey500, decision 0022 § 3): the Heartbeat section of the
// taskbag's settings shows and changes the two numbers `alm heartbeat-settings` shows and changes
// (project.json's `heartbeat`): how often a session holding a task beats, after how long without
// one a held task is lost, and whether work in progress is pushed at each heartbeat
// (Task-A01-butter895). After a save the taskbag is read again, so the task list's Lost
// tags follow the new limit.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { atDefaults, changeOf, limitTooShort, type HeartbeatSettings } from "./heartbeatSettingsFlow";
import { isProblem } from "./types";

export function HeartbeatSection({ messenger, root, onChanged }: { messenger: Messenger; root: string; onChanged: () => Promise<void> }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<HeartbeatSettings | null>(null);
  const [interval, setInterval] = useState("");
  const [lostAfter, setLostAfter] = useState("");
  const [pushWork, setPushWork] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  const fill = (s: HeartbeatSettings) => {
    setSettings(s);
    setInterval(String(s.applied_interval_minutes));
    setLostAfter(String(s.applied_lost_after_minutes));
    setPushWork(s.applied_push_work);
  };
  useEffect(() => {
    messenger.heartbeatSettings(root).then(fill, (e) => setError(isProblem(e) ? problem(e) : String(e)));
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  const run = async (work: () => Promise<HeartbeatSettings>) => {
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      fill(await work());
      await onChanged();
      setSaved(true);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };

  const save = () => {
    if (!settings) return;
    const checked = changeOf(settings, { field: t("heartbeat_settings.interval_name"), text: interval }, { field: t("heartbeat_settings.lost_after_name"), text: lostAfter }, pushWork);
    if (!checked.ok) {
      setSaved(false);
      setError(problem(checked.problem));
      return;
    }
    void run(() => messenger.setHeartbeatSettings(root, checked.change));
  };

  const mark = (written: number | null | undefined) => (written == null ? ` ${t("heartbeat_settings.default_mark")}` : "");
  const i = Number(interval);
  const l = Number(lostAfter);

  return (
    <div className="handoff-settings">
      <p className="muted small">{t("heartbeat_settings.body")}</p>
      <div className="form">
        <label>
          {t("heartbeat_settings.interval")}
          <span className="inline">
            <input type="number" min={1} step={1} value={interval} onChange={(e) => setInterval(e.target.value)} disabled={busy || !settings} />{" "}
            {t("heartbeat_settings.minutes")}
            <span className="muted small">{mark(settings?.interval_minutes)}</span>
          </span>
        </label>
        <label>
          {t("heartbeat_settings.lost_after")}
          <span className="inline">
            <input type="number" min={1} step={1} value={lostAfter} onChange={(e) => setLostAfter(e.target.value)} disabled={busy || !settings} />{" "}
            {t("heartbeat_settings.minutes")}
            <span className="muted small">{mark(settings?.lost_after_minutes)}</span>
          </span>
        </label>
        <label className="choice">
          <input type="checkbox" checked={pushWork} onChange={(e) => setPushWork(e.target.checked)} disabled={busy || !settings} />
          {t("heartbeat_settings.push_work")}
        </label>
        <p className="muted small">{t("heartbeat_settings.push_work_note")}</p>
      </div>
      {settings && i >= 1 && l >= 1 && limitTooShort(i, l) && <p className="muted small">{t("heartbeat_settings.too_short")}</p>}
      {error && <p className="banner error">{error}</p>}
      {saved && !error && <p className="muted small">{t("handoff_settings.saved")}</p>}
      <div className="row start">
        <button type="button" className="primary" onClick={save} disabled={busy || !settings}>
          {t("handoff_settings.save")}
        </button>
        <button
          type="button"
          onClick={() => void run(() => messenger.setHeartbeatSettings(root, { clear: true }))}
          disabled={busy || !settings || atDefaults(settings)}
          title={settings ? t("heartbeat_settings.defaults_title", { interval: settings.default_interval_minutes, lost: settings.default_lost_after_minutes }) : undefined}
        >
          {t("handoff_settings.defaults")}
        </button>
      </div>
    </div>
  );
}
