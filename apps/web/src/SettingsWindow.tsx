// The app's own Settings window (Task-A01-marble320, approver 2026-10-29): settings for the whole
// app, apart from each taskbag's own. Its General section has **When Allomos starts**: reopen the
// windows and tabs (the default), open the last taskbag, or open a blank window. A choice is saved
// at once. Escape closes the window.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { AppSettingsMessenger, Messenger } from "./messenger";
import type { StartWith } from "./tabsFlow";
import { isProblem } from "./types";

const CHOICES: { value: StartWith; key: "app_settings.reopen" | "app_settings.last" | "app_settings.blank" }[] = [
  { value: "reopen", key: "app_settings.reopen" },
  { value: "last", key: "app_settings.last" },
  { value: "blank", key: "app_settings.blank" },
];

export function SettingsWindow({ messenger, settings }: { messenger: Pick<Messenger, "closeWindow">; settings: AppSettingsMessenger }) {
  const { t, problem } = useI18n();
  const [startWith, setStartWith] = useState<StartWith | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    settings
      .appSettings()
      .then((s) => setStartWith(s.start_with))
      .catch(() => setStartWith("reopen"));
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void messenger.closeWindow().catch(() => undefined);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [messenger, settings]);
  useEffect(() => {
    document.title = t("app_settings.title");
  }, [t]);
  const choose = (value: StartWith) => {
    const before = startWith;
    setStartWith(value);
    setError(null);
    settings.setStartWith(value).catch((e: unknown) => {
      setStartWith(before);
      setError(isProblem(e) ? problem(e) : String(e));
    });
  };
  return (
    <main className="settings-window">
      <h1>{t("settings.general")}</h1>
      <fieldset className="settings-choice">
        <legend>{t("app_settings.start")}</legend>
        {CHOICES.map((c) => (
          <label key={c.value}>
            <input type="radio" name="start_with" value={c.value} checked={startWith === c.value} disabled={startWith === null} onChange={() => choose(c.value)} />
            {t(c.key)}
          </label>
        ))}
      </fieldset>
      {error && (
        <p className="banner error" role="alert">
          {error}
        </p>
      )}
    </main>
  );
}
