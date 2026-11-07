// Project settings → Audit (decision 0021 § 6). So far its Identity part (Task-A01-frolic587):
// where the identity behind a pseudonym comes from, and whether approving and checking ask for a
// fresh sign-in. The history only ever holds a pseudonym ID; who is behind it stays on this
// computer (local project) or in the cloud. A local-only project keeps the pseudonym: the
// sign-in levels are shown greyed out, with why. Below it, Separation of duties
// (Task-A01-grotto036): a checkbox, off by default; turning it off asks why. Task A01-tail719 adds
// the section's other parts.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { AuditSettings, IdentityLevel } from "./types";
import { LEVELS, needsReason, separationNeedsReason } from "./auditFlow";


export function AuditSection({ messenger, root, fileManagerKey }: { messenger: Messenger; root: string; fileManagerKey: Key }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<AuditSettings | null>(null);
  const [level, setLevel] = useState<IdentityLevel>("pseudonym");
  const [reauth, setReauth] = useState(false);
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  const [separate, setSeparate] = useState(false);
  const [sepReason, setSepReason] = useState("");
  const [sepSaved, setSepSaved] = useState(false);

  const fill = (s: AuditSettings) => {
    setSettings(s);
    setLevel(s.identity.level);
    setReauth(s.identity.reauthenticate);
    setReason("");
    setSeparate(s.separation_of_duties);
    setSepReason("");
  };
  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  useEffect(() => {
    messenger.auditSettings(root).then(fill, say);
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  if (!settings) return error ? <p className="banner error">{error}</p> : null;
  const id = settings.identity;
  const signIn = level !== "pseudonym";
  const changed = level !== id.level || reauth !== id.reauthenticate;
  const asksReason = needsReason(id, level, reauth);

  const save = async () => {
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      fill(await messenger.setIdentity(root, { level, reauthenticate: signIn ? reauth : false, reason: reason.trim() || undefined }));
      setSaved(true);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };

  const sepChanged = separate !== settings.separation_of_duties;
  const sepAsksReason = separationNeedsReason(settings.separation_of_duties, separate);
  const saveSeparation = async () => {
    setBusy(true);
    setError(null);
    setSepSaved(false);
    try {
      fill(await messenger.setSeparation(root, separate, sepReason.trim() || undefined));
      setSepSaved(true);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="audit-settings">
      <h3>{t("audit.identity")}</h3>
      <p className="muted small">{t("audit.identity_help")}</p>
      <div className="form" role="radiogroup" aria-label={t("audit.identity")}>
        {LEVELS.map((l) => {
          const unavailable = l !== "pseudonym" && !id.sign_in_available && l !== id.level;
          return (
            <label key={l} className={`choice${unavailable ? " muted" : ""}`}>
              <input type="radio" name="identity-level" value={l} checked={level === l} disabled={busy || unavailable} onChange={() => setLevel(l)} />
              {t(`audit.level_${l}` as Key)}
              {unavailable && <span className="muted small"> · {t("audit.needs_cloud")}</span>}
            </label>
          );
        })}
        <label className={`choice${signIn ? "" : " muted"}`}>
          <input type="checkbox" checked={signIn && reauth} disabled={busy || !signIn} onChange={(e) => setReauth(e.target.checked)} />
          {t("audit.reauthenticate")}
        </label>
        {asksReason && (
          <label className="warning">
            {t("audit.reason")}
            <input type="text" value={reason} onChange={(e) => setReason(e.target.value)} disabled={busy} />
          </label>
        )}
      </div>
      {id.written && <p className="muted small">{t("audit.unknown_level", { value: id.written })}</p>}
      {settings.identities_file && (
        <p className="muted small">
          {t("audit.identities_where")}{" "}
          <button type="button" className="link small" onClick={() => void messenger.showIdentitiesFile().catch(say)}>
            {t(fileManagerKey)}
          </button>
        </p>
      )}
      {saved && <p className="muted small">{t("handoff_settings.saved")}</p>}
      {changed && (
        <div className="row start">
          <button type="button" onClick={() => void save()} disabled={busy || (asksReason && !reason.trim())}>
            {t("handoff_settings.save")}
          </button>
        </div>
      )}
      <h3>{t("audit.separation")}</h3>
      <p className="muted small">{t("audit.separation_help")}</p>
      <div className="form">
        <label className="choice">
          <input type="checkbox" checked={separate} disabled={busy} onChange={(e) => setSeparate(e.target.checked)} />
          {t("audit.separation_switch")}
        </label>
        {sepAsksReason && (
          <label className="warning">
            {t("audit.reason")}
            <input type="text" value={sepReason} onChange={(e) => setSepReason(e.target.value)} disabled={busy} />
          </label>
        )}
      </div>
      {sepSaved && <p className="muted small">{t("handoff_settings.saved")}</p>}
      {sepChanged && (
        <div className="row start">
          <button type="button" onClick={() => void saveSeparation()} disabled={busy || (sepAsksReason && !sepReason.trim())}>
            {t("handoff_settings.save")}
          </button>
        </div>
      )}
      {error && <p className="banner error">{error}</p>}
    </div>
  );
}
