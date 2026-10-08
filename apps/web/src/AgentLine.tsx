// "Do the AI agents know about this project?" (Task-A03-04): one line under the top bar.
// FlipHopper works it out itself: from the project's instruction files (AGENTS.md, CLAUDE.md)
// and from who last changed the seabag with `fh`. Agents write no flag.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import type { AgentStatus } from "./types";

/** How often the line checks again: the instruction files are outside the seabag. */
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
  const { t, language } = useI18n();
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
  let use: string;
  if (status.last) {
    const when = new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(
      new Date(status.last.when * 1000),
    );
    use = t("agentline.last", { who: status.last.who, when });
  } else if (status.holders.length > 0) {
    use = t("agentline.holding", { who: status.holders.join(", ") });
  } else {
    use = t("agentline.unused");
  }

  return (
    <div className="agent-line">
      <span>
        {t("agentline.label")}: {codex || claude ? `${agents} — ${use}` : t("agentline.none")}
      </span>
      {!(codex && claude) && (
        <button type="button" className="link" onClick={onTell}>
          {t("agentline.tell")}
        </button>
      )}
    </div>
  );
}
