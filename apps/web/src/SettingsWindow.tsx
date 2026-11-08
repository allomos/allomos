// The app's own Settings window (Task-A01-marble320, approver 2026-10-29): settings for the whole
// app, apart from each taskbag's own. Its General section has **When Allomos starts**: reopen the
// windows and tabs (the default), open the last taskbag, or open a blank window. A choice is saved
// at once. Then **Automatic updates** (Task-A07-meter647, on by default): off, the app no longer
// asks GitHub by itself nor downloads in the background; Check for Updates… still works. Escape
// closes the window. Below it, **This computer** (Task-A01-tail719, decision 0021
// § 6): this computer's signing key (its ID and public key, Export public key…) and the Last seen
// history, the last checkpoint this computer saw of each project and whether that taskbag still
// has it. **Replace key…** (Task-A01-ring220) makes a new key after an inline confirmation; the old
// one is retired in every recent folder's taskbag that has it, so its changes can still be checked.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { AppSettingsMessenger, Messenger } from "./messenger";
import type { StartWith } from "./tabsFlow";
import { isProblem } from "./types";
import type { KeyReplaced, ThisComputer } from "./types";
import { readRecent, readRecentInfo } from "./recentFlow";
import { replacedSummary, seenState } from "./auditFlow";

const CHOICES: { value: StartWith; key: "app_settings.reopen" | "app_settings.last" | "app_settings.blank" }[] = [
  { value: "reopen", key: "app_settings.reopen" },
  { value: "last", key: "app_settings.last" },
  { value: "blank", key: "app_settings.blank" },
];

export function SettingsWindow({ messenger, settings }: { messenger: Pick<Messenger, "closeWindow">; settings: AppSettingsMessenger }) {
  const { t, problem } = useI18n();
  const [startWith, setStartWith] = useState<StartWith | null>(null);
  const [autoUpdate, setAutoUpdate] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [computer, setComputer] = useState<ThisComputer | null>(null);
  const [exported, setExported] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [replaced, setReplaced] = useState<KeyReplaced | null>(null);
  const readComputer = () => settings.thisComputer(readRecent()).then(setComputer, (e: unknown) => setError(isProblem(e) ? problem(e) : String(e)));
  useEffect(() => {
    void readComputer();
    // Read once when the window opens; the language only words an error.
  }, [settings]);
  useEffect(() => {
    settings
      .appSettings()
      .then((s) => {
        setStartWith(s.start_with);
        setAutoUpdate(s.auto_update);
      })
      .catch(() => {
        setStartWith("reopen");
        setAutoUpdate(true);
      });
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
  const switchAutoUpdate = (on: boolean) => {
    const before = autoUpdate;
    setAutoUpdate(on);
    setError(null);
    settings.setAutoUpdate(on).catch((e: unknown) => {
      setAutoUpdate(before);
      setError(isProblem(e) ? problem(e) : String(e));
    });
  };
  const exportKey = () => {
    setError(null);
    setExported(false);
    settings
      .exportPublicKey(`allomos-public-key-${computer?.key?.id ?? "none"}.json`)
      .then(setExported, (e: unknown) => setError(isProblem(e) ? problem(e) : String(e)));
  };
  const replaceKey = () => {
    setError(null);
    setExported(false);
    setReplacing(true);
    settings
      .replaceKey(readRecent())
      .then(
        (r) => {
          setReplaced(r);
          setConfirming(false);
          return readComputer();
        },
        (e: unknown) => setError(isProblem(e) ? problem(e) : String(e)),
      )
      .finally(() => setReplacing(false));
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
      <fieldset className="settings-choice">
        <legend>{t("app_settings.updates")}</legend>
        <label>
          <input type="checkbox" name="auto_update" checked={autoUpdate ?? true} disabled={autoUpdate === null} onChange={(e) => switchAutoUpdate(e.target.checked)} />
          {t("app_settings.auto_update")}
        </label>
        <p className="muted small">{t("app_settings.auto_update_help")}</p>
      </fieldset>
      <h1>{t("this_computer.title")}</h1>
      {computer && (
        <ThisComputerSection
          computer={computer}
          exported={exported}
          onExport={exportKey}
          replace={{ confirming, replacing, replaced, onAsk: () => {
              setReplaced(null);
              setConfirming(true);
            }, onCancel: () => setConfirming(false), onReplace: replaceKey }}
        />
      )}
      {error && (
        <p className="banner error" role="alert">
          {error}
        </p>
      )}
    </main>
  );
}

interface ReplaceState {
  confirming: boolean;
  replacing: boolean;
  replaced: KeyReplaced | null;
  onAsk: () => void;
  onCancel: () => void;
  onReplace: () => void;
}

function ThisComputerSection({ computer, exported, onExport, replace }: { computer: ThisComputer; exported: boolean; onExport: () => void; replace: ReplaceState }) {
  const { t } = useI18n();
  const summary = replace.replaced && replacedSummary(replace.replaced);
  const names = readRecentInfo();
  const when = (utc: string) => {
    const d = new Date(utc);
    return Number.isNaN(d.getTime()) ? utc : d.toLocaleString();
  };
  return (
    <section className="this-computer">
      <h2>{t("this_computer.key")}</h2>
      <p className="muted small">{t("this_computer.key_help")}</p>
      {computer.key ? (
        <>
          <p className="small">
            {t("this_computer.key_id")} <code>{computer.key.id}</code>
          </p>
          <p className="small">
            {t("this_computer.public_key")} <code>{computer.key.public_key}</code>
          </p>
          {computer.store !== "none" && <p className="muted small">{t(`this_computer.store_${computer.store}`)}</p>}
          <div className="row start">
            <button type="button" onClick={onExport}>
              {t("this_computer.export")}
            </button>
            {!replace.confirming && (
              <button type="button" onClick={replace.onAsk}>
                {t("this_computer.replace")}
              </button>
            )}
          </div>
          {exported && <p className="muted small">{t("this_computer.exported")}</p>}
          {replace.confirming && (
            <div className="replace-key">
              <p className="small">{t("this_computer.replace_help")}</p>
              <div className="row start">
                <button type="button" className="warning" disabled={replace.replacing} onClick={replace.onReplace}>
                  {t("this_computer.replace_confirm")}
                </button>
                <button type="button" disabled={replace.replacing} onClick={replace.onCancel}>
                  {t("this_computer.replace_cancel")}
                </button>
              </div>
            </div>
          )}
          {replace.replaced && summary && (
            <p className="muted small" role="status">
              {t("this_computer.replaced", { old: replace.replaced.old, count: summary.replaced })}
            </p>
          )}
          {summary && summary.failed.length > 0 && (
            <p className="warning small">{t("this_computer.replaced_failed", { taskbags: summary.failed.join(", ") })}</p>
          )}
        </>
      ) : (
        <p className="muted small">{t(computer.store === "none" ? "this_computer.no_store" : "this_computer.no_key_yet")}</p>
      )}
      <h2>{t("this_computer.seen")}</h2>
      <p className="muted small">{t("this_computer.seen_help")}</p>
      {computer.seen.length === 0 ? (
        <p className="muted small">{t("this_computer.seen_none")}</p>
      ) : (
        <ul className="seen-list small">
          {computer.seen.map((s) => {
            const state = seenState(s.still_there);
            const name = (s.folder && names[s.folder]?.name) || s.folder || s.project;
            return (
              <li key={s.project} className={state === "gone" ? "seen-gone" : undefined}>
                {t("this_computer.seen_line", { name, checkpoint: s.checkpoint, when: when(s.seen_at) })}{" "}
                · {t(`this_computer.seen_${state}`)}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
