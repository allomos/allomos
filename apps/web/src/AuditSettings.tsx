// Project settings → Audit (decision 0021 § 6). So far its Identity part (Task-A01-frolic587):
// where the identity behind a pseudonym comes from, and whether approving and checking ask for a
// fresh sign-in. The history only ever holds a pseudonym ID; who is behind it stays on this
// computer (local project) or in the cloud. A local-only project keeps the pseudonym: the
// sign-in levels are shown greyed out, with why. Below it, Separation of duties
// (Task-A01-grotto036): a checkbox, off by default; turning it off asks why. Then Retention
// (Task-A01-stem986): keep the history forever or N years, and the legal hold; a shorter period
// and the hold ask why. Task A01-tail719 adds the rest, top to bottom: Always recorded (a
// read-only list), Outside witness (its switch waits for the witness itself; meanwhile the
// taskbag's checkpoints and Make a checkpoint now), Remove personal details… under Retention
// (`alm redact`'s rule), and Change types (fixed, read-only). Every change is an `audit-settings`
// event; turning a control off is shown in the warning colour and asks why.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { AuditSettings, IdentityLevel, RedactDone } from "./types";
import { canRedact, LEVELS, needsReason, REDACT_FIELDS, redactFields, retentionNeedsReason, separationNeedsReason, yearsFrom } from "./auditFlow";

/** Always recorded (decision 0021 § 1.1): no switches. */
const ALWAYS: Key[] = [
  "audit.always_who",
  "audit.always_approvals",
  "audit.always_checks",
  "audit.always_links",
  "audit.always_effects",
  "audit.always_class",
  "audit.always_reasons",
  "audit.always_exceptions",
  "audit.always_ai",
  "audit.always_traces",
  "audit.always_signed",
];

const CHANGE_TYPES: Key[] = ["audit.type_standard", "audit.type_normal", "audit.type_emergency"];
const RISKS: Key[] = ["audit.risk_low", "audit.risk_medium", "audit.risk_high"];

const when = (utc: string) => {
  const d = new Date(utc);
  return Number.isNaN(d.getTime()) ? utc : d.toLocaleString();
};


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
  const [forever, setForever] = useState(true);
  const [yearsText, setYearsText] = useState("7");
  const [hold, setHold] = useState(false);
  const [retReason, setRetReason] = useState("");
  const [retSaved, setRetSaved] = useState(false);
  const [redacting, setRedacting] = useState(false);
  const [redTask, setRedTask] = useState("");
  const [redTicked, setRedTicked] = useState<string[]>([]);
  const [redTyped, setRedTyped] = useState("");
  const [redReason, setRedReason] = useState("");
  const [redBy, setRedBy] = useState("");
  const [redDone, setRedDone] = useState<RedactDone | null>(null);
  const [checkpointMade, setCheckpointMade] = useState(false);

  const fill = (s: AuditSettings) => {
    setSettings(s);
    setLevel(s.identity.level);
    setReauth(s.identity.reauthenticate);
    setReason("");
    setSeparate(s.separation_of_duties);
    setSepReason("");
    setForever(s.retention_years === null);
    if (s.retention_years !== null) setYearsText(String(s.retention_years));
    setHold(s.legal_hold);
    setRetReason("");
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

  const years = forever ? null : yearsFrom(yearsText);
  const yearsBad = !forever && years === null;
  const retNow = { years: settings.retention_years, hold: settings.legal_hold };
  const retChanged = !yearsBad && (years !== retNow.years || hold !== retNow.hold);
  const retAsksReason = !yearsBad && retentionNeedsReason(retNow, years, hold);
  const saveRetention = async () => {
    setBusy(true);
    setError(null);
    setRetSaved(false);
    try {
      fill(await messenger.setRetention(root, years, hold, retReason.trim() || undefined));
      setRetSaved(true);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };

  const makeCheckpoint = async () => {
    setBusy(true);
    setError(null);
    setCheckpointMade(false);
    try {
      fill(await messenger.makeCheckpoint(root));
      setCheckpointMade(true);
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };

  const redFields = redactFields(redTicked, redTyped);
  const redOk = canRedact(redTask, redFields, redReason, settings.legal_hold);
  const redact = async () => {
    setBusy(true);
    setError(null);
    setRedDone(null);
    try {
      setRedDone(await messenger.redactTask(root, redTask.trim(), redFields, redReason.trim(), redBy.trim() || undefined));
      setRedReason("");
    } catch (e) {
      say(e);
    } finally {
      setBusy(false);
    }
  };
  const cp = settings.checkpoints;

  return (
    <div className="audit-settings">
      <h3>{t("audit.always")}</h3>
      <p className="muted small">{t("audit.always_help")}</p>
      <ul className="audit-always small">
        {ALWAYS.map((k) => (
          <li key={k}>{t(k)}</li>
        ))}
      </ul>
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
      <h3>{t("audit.witness")}</h3>
      <p className="muted small">{t("audit.witness_help")}</p>
      <div className="form">
        <label className="choice muted">
          <input type="checkbox" checked={false} disabled />
          {t("audit.witness_switch")}
          <span className="muted small"> · {t("audit.witness_not_yet")}</span>
        </label>
        <p className="small">
          {cp.last ? t("audit.checkpoints", { count: cp.count, when: when(cp.last.at), receipts: cp.receipts }) : t("audit.checkpoints_none")}
        </p>
        {settings.signing === "none" && <p className="muted small">{t("audit.checkpoint_no_key")}</p>}
      </div>
      {checkpointMade && <p className="muted small">{t("audit.checkpoint_made")}</p>}
      <div className="row start">
        <button type="button" onClick={() => void makeCheckpoint()} disabled={busy || settings.signing === "none"}>
          {t("audit.checkpoint_now")}
        </button>
      </div>
      <h3>{t("audit.retention")}</h3>
      <p className="muted small">{t("audit.retention_help")}</p>
      <div className="form" role="radiogroup" aria-label={t("audit.retention_keep")}>
        <span>{t("audit.retention_keep")}</span>
        <label className="choice">
          <input type="radio" name="retention" checked={forever} disabled={busy} onChange={() => setForever(true)} />
          {t("audit.retention_forever")}
        </label>
        <label className="choice">
          <input type="radio" name="retention" checked={!forever} disabled={busy} onChange={() => setForever(false)} />
          <input type="number" min={1} max={100} value={yearsText} disabled={busy || forever} onChange={(e) => setYearsText(e.target.value)} aria-label={t("audit.retention_years")} />{" "}
          {t("audit.retention_years")}
        </label>
        <p className="muted small">{t("audit.retention_law")}</p>
        {yearsBad && <p className="warning small">{t("problem.retention_invalid", { value: yearsText })}</p>}
        <label className="choice">
          <input type="checkbox" checked={hold} disabled={busy} onChange={(e) => setHold(e.target.checked)} />
          {t("audit.legal_hold")}
        </label>
        <p className="muted small">{t("audit.legal_hold_help")}</p>
        {retAsksReason && (
          <label className="warning">
            {t("audit.reason")}
            <input type="text" value={retReason} onChange={(e) => setRetReason(e.target.value)} disabled={busy} />
          </label>
        )}
      </div>
      {retSaved && <p className="muted small">{t("handoff_settings.saved")}</p>}
      {retChanged && (
        <div className="row start">
          <button type="button" onClick={() => void saveRetention()} disabled={busy || (retAsksReason && !retReason.trim())}>
            {t("handoff_settings.save")}
          </button>
        </div>
      )}
      <div className="row start">
        <button type="button" onClick={() => setRedacting(!redacting)} aria-expanded={redacting} disabled={busy}>
          {t("audit.redact")}
        </button>
      </div>
      {redacting && (
        <div className="form audit-redact">
          <p className="muted small">{t("audit.redact_help")}</p>
          {settings.legal_hold && <p className="warning small">{t("audit.redact_hold")}</p>}
          <label>
            {t("audit.redact_task")}
            <input type="text" value={redTask} onChange={(e) => setRedTask(e.target.value)} disabled={busy} placeholder="A01-01" />
          </label>
          <fieldset className="settings-choice">
            <legend>{t("audit.redact_fields")}</legend>
            {REDACT_FIELDS.map((f) => (
              <label key={f} className="choice">
                <input
                  type="checkbox"
                  checked={redTicked.includes(f)}
                  disabled={busy}
                  onChange={(e) => setRedTicked(e.target.checked ? [...redTicked, f] : redTicked.filter((x) => x !== f))}
                />
                {t(`audit.redact_field_${f}` as Key)}
              </label>
            ))}
            <label>
              {t("audit.redact_other")}
              <input type="text" value={redTyped} onChange={(e) => setRedTyped(e.target.value)} disabled={busy} />
            </label>
          </fieldset>
          <label className="warning">
            {t("audit.reason")}
            <input type="text" value={redReason} onChange={(e) => setRedReason(e.target.value)} disabled={busy} />
          </label>
          <label>
            {t("audit.redact_approved_by")}
            <input type="text" value={redBy} onChange={(e) => setRedBy(e.target.value)} disabled={busy} />
          </label>
          <p className="muted small">{t("audit.redact_approved_by_help")}</p>
          <div className="row start">
            <button type="button" onClick={() => void redact()} disabled={busy || !redOk}>
              {t("audit.redact_remove")}
            </button>
          </div>
          {redDone &&
            (redDone.redaction ? (
              <p className="small">
                {t("audit.redact_done", { id: redDone.redaction.id, fields: redDone.redaction.fields.join(", "), count: redDone.redaction.files.length })}
              </p>
            ) : (
              <p className="muted small">{t("audit.redact_nothing")}</p>
            ))}
        </div>
      )}
      <h3>{t("audit.change_types")}</h3>
      <p className="muted small">{t("audit.change_types_help")}</p>
      <ul className="audit-always small">
        {CHANGE_TYPES.map((k) => (
          <li key={k}>{t(k)}</li>
        ))}
      </ul>
      <ul className="audit-always small">
        {RISKS.map((k) => (
          <li key={k}>{t(k)}</li>
        ))}
      </ul>
      <p className="muted small">{t("audit.change_types_fixed")}</p>
      {error && <p className="banner error">{error}</p>}
    </div>
  );
}
