// "Tell AI agents about FlipHopper?" (Task-A03-03): lists the AI agents on this computer, lets the
// person pick one or more and the project folder, then opens each with the request typed in.
// FlipHopper never sends the request; the person does.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { Agent } from "./types";

const AGENT_KEY: Record<Agent["id"], Key> = {
  claude_cli: "agents.claude_cli",
  claude_app: "agents.claude_app",
  codex_cli: "agents.codex_cli",
  codex_app: "agents.codex_app",
};

export function AgentsPanel({
  messenger,
  project: startProject,
  seabag,
  onDone,
}: {
  messenger: Messenger;
  project: string;
  seabag: string;
  /** Closes the panel, with the message to show (or null). */
  onDone: (message: string | null) => void;
}) {
  const { t, problem } = useI18n();
  const [agents, setAgents] = useState<Agent[] | null>(null);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [project, setProject] = useState(startProject);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    messenger
      .findAgents()
      .then((found) => {
        if (!live) return;
        setAgents(found);
        // The first one is ticked, so one click asks it.
        setChosen(new Set(found.slice(0, 1).map((a) => a.id)));
      })
      .catch(() => live && setAgents([]));
    return () => {
      live = false;
    };
  }, [messenger]);

  const toggle = (id: string) =>
    setChosen((set) => {
      const next = new Set(set);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const changeProject = async () => {
    const path = await messenger.pickFolder(t("agents.project"));
    if (path) setProject(path);
  };

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  const ask = async () => {
    setBusy(true);
    setError(null);
    try {
      const opened = await messenger.askAgents([...chosen], project, seabag);
      const names = (agents ?? []).filter((a) => opened.includes(a.id)).map((a) => t(AGENT_KEY[a.id]));
      onDone(t("agents.asked", { agents: names.join(", ") }));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const write = async () => {
    setBusy(true);
    setError(null);
    try {
      const files = await messenger.writeAgentFiles(project);
      onDone(files.length > 0 ? t("agents.written", { files: files.join(", ") }) : null);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card agents">
      <h2>{t("agents.title")}</h2>
      <p>{t("agents.body")}</p>
      <div className="row">
        <span>
          {t("agents.project")}: <code>{project}</code>
        </span>
        <button type="button" className="link" onClick={() => void changeProject()} disabled={busy}>
          {t("agents.change")}
        </button>
      </div>
      <p className="muted">{t("agents.seabag", { path: seabag })}</p>
      {agents === null ? null : agents.length === 0 ? (
        <p>{t("agents.none")}</p>
      ) : (
        <ul className="agent-list">
          {agents.map((a) => (
            <li key={a.id}>
              <label>
                <input type="checkbox" checked={chosen.has(a.id)} onChange={() => toggle(a.id)} disabled={busy} />{" "}
                <strong>{t(AGENT_KEY[a.id])}</strong>{" "}
                <span className="muted">
                  — {t(a.how === "link" ? "agents.how_link" : "agents.how_paste")} ({a.found})
                </span>
              </label>
            </li>
          ))}
        </ul>
      )}
      {error && <p className="banner error">{error}</p>}
      <div className="row">
        {agents && agents.length > 0 && (
          <button type="button" className="primary" onClick={() => void ask()} disabled={busy || chosen.size === 0}>
            {t("agents.ask")}
          </button>
        )}
        {agents && agents.length === 0 && (
          <button type="button" className="primary" onClick={() => void write()} disabled={busy}>
            {t("agents.write")}
          </button>
        )}
        <button type="button" onClick={() => onDone(null)} disabled={busy}>
          {t("agents.later")}
        </button>
      </div>
    </section>
  );
}
