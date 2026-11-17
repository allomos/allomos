// File size (Task-A03-dapper008, approver 2026-11-16): the File size section of the taskbag's
// settings shows and changes the line limit `alm file-size-settings` shows and changes
// (project.json's `file_size`). A source file over it may only get shorter, task by task; each
// project's own tests check that (`alm guide file-size`). Allomos keeps the number and measures no
// file.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";

/** The settings as the desktop app sends them (apps/desktop/src-tauri/src/file_size_settings.rs). */
export interface FileSizeSettings {
  /** As written; null where the default applies. */
  limit_lines: number | null;
  applied_limit_lines: number;
  default_limit_lines: number;
}

export function FileSizeSection({ messenger, root }: { messenger: Messenger; root: string }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<FileSizeSettings | null>(null);
  const [limit, setLimit] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  const fill = (s: FileSizeSettings) => {
    setSettings(s);
    setLimit(String(s.applied_limit_lines));
  };
  useEffect(() => {
    messenger.fileSizeSettings(root).then(fill, (e) => setError(isProblem(e) ? problem(e) : String(e)));
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  const run = async (work: () => Promise<FileSizeSettings>) => {
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      fill(await work());
      setSaved(true);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };

  // The number is checked where it is kept (alm's rule, a whole number of lines, at least 1).
  const save = () => void run(() => messenger.setFileSizeSettings(root, { limit: limit.trim() }));

  return (
    <div className="handoff-settings">
      <p className="muted small">{t("file_size_settings.body")}</p>
      <div className="form">
        <label>
          {t("file_size_settings.limit")}
          <span className="inline">
            <input type="number" min={1} step={1} value={limit} onChange={(e) => setLimit(e.target.value)} disabled={busy || !settings} />{" "}
            {t("file_size_settings.lines")}
            <span className="muted small">{settings?.limit_lines == null ? ` ${t("heartbeat_settings.default_mark")}` : ""}</span>
          </span>
        </label>
      </div>
      {error && <p className="banner error">{error}</p>}
      {saved && !error && <p className="muted small">{t("handoff_settings.saved")}</p>}
      <div className="row start">
        <button type="button" className="primary" onClick={save} disabled={busy || !settings}>
          {t("handoff_settings.save")}
        </button>
        <button
          type="button"
          onClick={() => void run(() => messenger.setFileSizeSettings(root, { clear: true }))}
          disabled={busy || !settings || settings.limit_lines == null}
          title={settings ? t("file_size_settings.defaults_title", { lines: settings.default_limit_lines }) : undefined}
        >
          {t("handoff_settings.defaults")}
        </button>
      </div>
    </div>
  );
}
