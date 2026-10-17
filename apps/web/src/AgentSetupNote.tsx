// A newly installed agent sets itself up once (Task-A03-24, approver 2026-10-16): every found agent
// with no entry under `## Agents set up` in the taskbag's project notes shows "Not set up for this
// project yet." with Copy prompt. Shown in Connect with the agents, both on the agent list and on
// the page for a project already connected (most projects a new agent arrives in). The app reads
// only that section and checks nothing else.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { useAgents } from "./AgentPicker";
import { GROUPS, foundGroups } from "./agentPickerFlow";
import { agentFilesCommand, notSetUp } from "./agentSetupFlow";
import { COPIED_MS, MANUAL_LOOK_MS } from "./manualFlow";

/** The agents (Claude, Codex) not yet set up, read when shown and again while shown, so a note
 * goes away once the agent adds its line; and a `note(group)` to draw under an agent's name. */
export function useSetupNotes(messenger: Messenger, project: string, taskbag: string, onError: (e: unknown) => void) {
  const { t } = useI18n();
  const [setUp, setSetUp] = useState<string[] | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    let live = true;
    // Unreadable: no note (nothing to say).
    const look = () =>
      messenger
        .agentsSetUp(taskbag)
        .then((entries) => live && setSetUp(entries))
        .catch(() => undefined);
    void look();
    const every = window.setInterval(() => void look(), MANUAL_LOOK_MS);
    return () => {
      live = false;
      window.clearInterval(every);
    };
  }, [messenger, taskbag]);
  const need = setUp === null ? [] : notSetUp(GROUPS.map((g) => g.name), setUp);

  const copy = async (group: string) => {
    try {
      await messenger.copyText(t("agents.setup_prompt", { command: agentFilesCommand(project, taskbag) }));
      setCopied(group);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setCopied(null), COPIED_MS);
    } catch (e) {
      onError(e);
    }
  };

  const note = (group: string, disabled?: boolean) =>
    need.includes(group) ? (
      <div className="row start setup-note">
        <span className="muted small">{t("agents.not_set_up")}</span>
        <button type="button" className="link" onClick={() => void copy(group)} disabled={disabled}>
          {t("agents.copy_prompt")}
        </button>
        <span className="muted small" role="status">
          {copied === group ? t("manual.copied") : ""}
        </span>
      </div>
    ) : null;
  return { need, note };
}

/** For a project already connected: each found agent not yet set up, with its note. */
export function AgentSetupNotes({
  messenger,
  project,
  taskbag,
  onError,
}: {
  messenger: Messenger;
  project: string;
  taskbag: string;
  onError: (e: unknown) => void;
}) {
  const { agents } = useAgents(messenger);
  const { need, note } = useSetupNotes(messenger, project, taskbag, onError);
  const groups = foundGroups(agents ?? []).filter((g) => need.includes(g.name));
  if (groups.length === 0) return null;
  return (
    <ul className="agent-list setup-notes">
      {groups.map((g) => (
        <li key={g.name}>
          <strong>{g.name}</strong>
          {note(g.name)}
        </li>
      ))}
    </ul>
  );
}
