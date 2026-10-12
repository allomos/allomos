// "Do the AI agents know about this project?" (Task-A03-04): a short note beside Show in Finder.
// Allomos works it out itself from the project's instruction files (AGENTS.md, CLAUDE.md);
// agents write no flag. Since 2026-10-10 (owner) it no longer says who last used `alm` or holds
// tasks: a holder copied from an old board read as an agent "working now".

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import type { AgentStatus } from "./types";

/** How often the line checks again: the instruction files are outside the taskbag. */
const CHECK_MS = 4000;

export function AgentLine({
  messenger,
  root,
  onTell,
}: {
  messenger: Messenger;
  root: string;
  /** Opens the "Tell AI agents" panel for this project. */
  onTell: () => void;
}) {
  const { t } = useI18n();
  const [status, setStatus] = useState<AgentStatus | null>(null);

  useEffect(() => {
    let live = true;
    const check = () =>
      messenger
        .agentStatus(root)
        .then((s) => live && setStatus(s))
        .catch(() => undefined);
    void check();
    const timer = window.setInterval(() => void check(), CHECK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, root]);

  if (!status) return null;
  const { codex, claude } = status.setup;
  const mark = (ok: boolean) => (ok ? "✓" : "✗");
  const agents = `Codex ${mark(codex)}  Claude Code ${mark(claude)}`;

  return (
    <div className="agent-line">
      <span>
        {t("agentline.line", { state: codex || claude ? agents : t("agentline.none") })}
      </span>
      {!(codex && claude) && (
        <button type="button" className="link" onClick={onTell}>
          {t("agentline.tell")}
        </button>
      )}
    </div>
  );
}
