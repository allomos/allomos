// The settings dialog's **Allomos** group (Task-A01-marble320, Task-A05-cookie962): settings for
// the whole app on this computer, apart from each taskbag's own. Its General section has **When
// Allomos starts**: reopen the windows and tabs (the default), open the last taskbag, or open a
// blank window. A choice is saved at once. Then the updates switches (Task-A07-meter647, split by
// Task-A07-trunk220, both on by default): **Check for updates automatically** off, the app no
// longer asks GitHub by itself; **Download updates automatically** (nested, disabled while the check
// is off) off, nothing is downloaded in the background; Check for Updates… still works and still
// downloads. **This computer** (Task-A01-tail719, decision 0021 § 6): this computer's
// signing key (its ID and public key, Export public key…) and the Last seen history, the last
// checkpoint this computer saw of each project and whether that taskbag still has it. **Replace
// key…** (Task-A01-ring220) makes a new key after an inline confirmation; the old one is retired in
// every recent folder's taskbag that has it, so its changes can still be checked.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { AppSettingsMessenger } from "./messenger";
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

type Settings = Omit<AppSettingsMessenger, "takeSettingsRequest">;

function errorText(e: unknown, problem: ReturnType<typeof useI18n>["problem"]) {
  return isProblem(e) ? problem(e) : String(e);
}

/** Allomos → General: When Allomos starts, and the two update switches. */
export function AppGeneralSection({ settings }: { settings: Settings }) {
  const { t, problem } = useI18n();
  const [startWith, setStartWith] = useState<StartWith | null>(null);
  const [autoCheck, setAutoCheck] = useState<boolean | null>(null);
  const [autoDownload, setAutoDownload] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    settings
      .appSettings()
      .then((s) => {
        setStartWith(s.start_with);
        setAutoCheck(s.auto_check);
        setAutoDownload(s.auto_download);
      })
      .catch(() => {
        setStartWith("reopen");
        setAutoCheck(true);
        setAutoDownload(true);
      });
  }, [settings]);
  const choose = (value: StartWith) => {
    const before = startWith;
    setStartWith(value);
    setError(null);
    settings.setStartWith(value).catch((e: unknown) => {
      setStartWith(before);
      setError(errorText(e, problem));
    });
  };
  const switchAutoCheck = (on: boolean) => {
    const before = autoCheck;
    setAutoCheck(on);
    setError(null);
    settings.setAutoCheck(on).catch((e: unknown) => {
      setAutoCheck(before);
      setError(errorText(e, problem));
    });
  };
  const switchAutoDownload = (on: boolean) => {
    const before = autoDownload;
    setAutoDownload(on);
    setError(null);
    settings.setAutoDownload(on).catch((e: unknown) => {
      setAutoDownload(before);
      setError(errorText(e, problem));
    });
  };
  return (
    <>
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
          <input type="checkbox" name="auto_check" checked={autoCheck ?? true} disabled={autoCheck === null} onChange={(e) => switchAutoCheck(e.target.checked)} />
          {t("app_settings.auto_check")}
        </label>
        <p className="muted small">{t("app_settings.auto_check_help")}</p>
        {/* Nested under the check: disabled while the check is off (Task-A07-trunk220). */}
        <div style={{ marginInlineStart: "1.5rem" }}>
          <label>
            <input type="checkbox" name="auto_download" checked={autoDownload ?? true} disabled={autoCheck !== true} onChange={(e) => switchAutoDownload(e.target.checked)} />
            {t("app_settings.auto_download")}
          </label>
          <p className="muted small">{t("app_settings.auto_download_help")}</p>
        </div>
      </fieldset>
      {error && (
        <p className="banner error" role="alert">
          {error}
        </p>
      )}
    </>
  );
}

/** Allomos → This computer: the signing key and the Last seen history. */
export function ThisComputerPart({ settings }: { settings: Settings }) {
  const { problem } = useI18n();
  const [error, setError] = useState<string | null>(null);
  const [computer, setComputer] = useState<ThisComputer | null>(null);
  const [exported, setExported] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [replaced, setReplaced] = useState<KeyReplaced | null>(null);
  const readComputer = () => settings.thisComputer(readRecent()).then(setComputer, (e: unknown) => setError(errorText(e, problem)));
  useEffect(() => {
    void readComputer();
    // Read once when the section opens; the language only words an error.
  }, [settings]);
  const exportKey = () => {
    setError(null);
    setExported(false);
    settings
      .exportPublicKey(`allomos-public-key-${computer?.key?.id ?? "none"}.json`)
      .then(setExported, (e: unknown) => setError(errorText(e, problem)));
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
        (e: unknown) => setError(errorText(e, problem)),
      )
      .finally(() => setReplacing(false));
  };
  return (
    <>
      {computer && (
        <ThisComputerSection
          computer={computer}
          exported={exported}
          onExport={exportKey}
          replace={{
            confirming,
            replacing,
            replaced,
            onAsk: () => {
              setReplaced(null);
              setConfirming(true);
            },
            onCancel: () => setConfirming(false),
            onReplace: replaceKey,
          }}
        />
      )}
      {error && (
        <p className="banner error" role="alert">
          {error}
        </p>
      )}
    </>
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
      <h3>{t("this_computer.key")}</h3>
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
      <h3>{t("this_computer.seen")}</h3>
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
