// New tasks wait for the approver (Task-A00-wheat560, approver 2026-11-25): a switch in the Tasks
// section of the taskbag's settings, the same setting `alm new-task-settings` shows and changes
// (project.json's `new_tasks`). On, an agent leaves a task it added in backlog until the approver
// agrees, except in a goal run; alm words its next steps by it and stops nothing.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";

/** The setting as the desktop app sends it (apps/desktop/src-tauri/src/new_task_settings.rs). */
export interface NewTaskSettings {
  wait_for_approval: boolean;
}

export function NewTaskSection({ messenger, root }: { messenger: Messenger; root: string }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<NewTaskSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  useEffect(() => {
    messenger.newTaskSettings(root).then(setSettings, fail);
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  const change = async (on: boolean) => {
    setBusy(true);
    setError(null);
    try {
      setSettings(await messenger.setNewTaskSettings(root, on));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="handoff-settings">
      <div className="form">
        <label className="choice">
          <input
            type="checkbox"
            checked={settings?.wait_for_approval ?? true}
            onChange={(e) => void change(e.target.checked)}
            disabled={busy || !settings}
          />
          {t("new_task_settings.wait")}
        </label>
      </div>
      <p className="muted small">{t("new_task_settings.body")}</p>
      {error && <p className="banner error">{error}</p>}
    </div>
  );
}
