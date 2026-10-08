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

/** The tools of one agent, in the order the first is chosen: the terminal before the app. */
const GROUPS: { name: string; tools: Agent["id"][] }[] = [
  { name: "Claude", tools: ["claude_cli", "claude_app"] },
  { name: "Codex", tools: ["codex_cli", "codex_app"] },
];

/** Show "Looking for AI agents…" only when finding them is slow. */
const SPINNER_AFTER_MS = 200;

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
  const { t, problem, language } = useI18n();
  const [agents, setAgents] = useState<Agent[] | null>(null);
  const [slow, setSlow] = useState(false);
  /** Per group (by name): the chosen tool, or none when the group is not ticked. */
  const [picked, setPicked] = useState<Record<string, Agent["id"]>>({});
  const [ticked, setTicked] = useState<Set<string>>(new Set());
  const [project, setProject] = useState(startProject);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const timer = window.setTimeout(() => live && setSlow(true), SPINNER_AFTER_MS);
    messenger
      .findAgents()
      .then((found) => {
        if (!live) return;
        setAgents(found);
        // In each group the first found tool is chosen; only the first group is ticked.
        const first: Record<string, Agent["id"]> = {};
        for (const g of GROUPS) {
          const tool = g.tools.find((id) => found.some((a) => a.id === id));
          if (tool) first[g.name] = tool;
        }
        setPicked(first);
        setTicked(new Set(GROUPS.filter((g) => first[g.name]).slice(0, 1).map((g) => g.name)));
      })
      .catch(() => live && setAgents([]))
      .finally(() => window.clearTimeout(timer));
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [messenger]);

  const toggleGroup = (name: string) =>
    setTicked((set) => {
      const next = new Set(set);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  const chosen = GROUPS.filter((g) => ticked.has(g.name) && picked[g.name]).map((g) => picked[g.name]);

  const changeProject = async () => {
    const path = await messenger.pickFolder(t("agents.project"));
    if (path) setProject(path);
  };

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  const ask = async () => {
    setBusy(true);
    setError(null);
    try {
      const opened = await messenger.askAgents(chosen, project, seabag, language);
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
      {agents === null ? (
        slow && (
          <p className="muted searching" role="status">
            <span className="spinner" aria-hidden="true" /> {t("agents.searching")}
          </p>
        )
      ) : agents.length === 0 ? (
        <p>{t("agents.none")}</p>
      ) : (
        <ul className="agent-list">
          {GROUPS.map((g) => {
            const tools = g.tools
              .map((id) => agents.find((a) => a.id === id))
              .filter((a): a is Agent => a !== undefined);
            if (tools.length === 0) return null;
            const on = ticked.has(g.name);
            return (
              <li key={g.name}>
                <label>
                  <input type="checkbox" checked={on} onChange={() => toggleGroup(g.name)} disabled={busy} />{" "}
                  <strong>{g.name}</strong>
                </label>
                <ul className="agent-tools">
                  {tools.map((a) => (
                    <li key={a.id}>
                      <label className={on ? undefined : "muted"}>
                        {tools.length > 1 && (
                          <>
                            <input
                              type="radio"
                              name={`tool-${g.name}`}
                              checked={picked[g.name] === a.id}
                              onChange={() => setPicked((p) => ({ ...p, [g.name]: a.id }))}
                              disabled={busy || !on}
                            />{" "}
                          </>
                        )}
                        {t(AGENT_KEY[a.id])}{" "}
                        <span className="muted">
                          — {t(a.how === "link" ? "agents.how_link" : "agents.how_paste")} ({a.found})
                        </span>
                      </label>
                    </li>
                  ))}
                </ul>
              </li>
            );
          })}
        </ul>
      )}
      {error && <p className="banner error">{error}</p>}
      <div className="row">
        {agents && agents.length > 0 && (
          <button type="button" className="primary" onClick={() => void ask()} disabled={busy || chosen.length === 0}>
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
