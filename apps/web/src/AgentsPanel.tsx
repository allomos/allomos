// "Connect with the agents" (Task-A05-03; before it, Task-A03-03 and Task-A05-02 were two
// panels): lists the AI agents on this computer, lets the person pick one (owner, 2026-10-12) and
// the project folder, then opens it with the request typed in: add Allomos's instructions, then
// look for the project's existing tasks and, if it finds some, ask with `alm confirm` before
// moving them. Allomos never sends a request; the person does.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { Agent, InstructionFiles } from "./types";

const AGENT_KEY: Record<Agent["id"], Key> = {
  claude_cli: "agents.claude_cli",
  claude_app: "agents.claude_app",
  codex_cli: "agents.codex_cli",
  codex_app: "agents.codex_app",
};

/** The tools of one agent, in the order the first is chosen: the app before the terminal
 * (owner, 2026-10-10; until then the terminal came first). */
const GROUPS: { name: string; tools: Agent["id"][] }[] = [
  { name: "Claude", tools: ["claude_app", "claude_cli"] },
  { name: "Codex", tools: ["codex_app", "codex_cli"] },
];

/** The keys that paste in a terminal on this computer. */
const PASTE_KEYS = /Mac/.test(navigator.userAgent) ? "⌘V" : "Ctrl+V";

/** Show "Looking for AI agents…" only when finding them is slow. */
const SPINNER_AFTER_MS = 200;

/** What happened when the panel closes. */
export interface ConnectResult {
  /** The message to show, or null. */
  message: string | null;
  /** Agents were asked, or the files were written: which instruction files the project folder
   * had just before, so the window can say when a new one appears (the restart notice). */
  watch?: { project: string; before: InstructionFiles };
}

export function AgentsPanel({
  messenger,
  project: startProject,
  taskbag,
  projectFixed,
  onProjectChange,
  onDone,
}: {
  messenger: Messenger;
  project: string;
  taskbag: string;
  /** The project folder was just chosen (the wizard's step 3): shown, without Change…. */
  projectFixed?: boolean;
  /** Records a project folder the person chose with Change… (the open-folder flow). */
  onProjectChange?: (folder: string) => Promise<void>;
  /** Closes the panel. */
  onDone: (result: ConnectResult) => void;
}) {
  const { t, rich, problem, language } = useI18n();
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

  // One agent (owner, 2026-10-12): choosing a group replaces the choice.
  const toggleGroup = (name: string) => setTicked(new Set([name]));
  const chosen = GROUPS.filter((g) => ticked.has(g.name) && picked[g.name]).map((g) => picked[g.name]);

  const changeProject = async () => {
    const path = await messenger.pickFolder(t("agents.project"));
    if (!path) return;
    try {
      await onProjectChange?.(path);
      setProject(path);
    } catch (e) {
      fail(e);
    }
  };

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  /** Which instruction files the project folder has now; when unknown, assume both, so no notice. */
  const filesNow = () =>
    messenger.instructionFiles(project).catch((): InstructionFiles => ({ agents_md: true, claude_md: true }));

  const ask = async () => {
    setBusy(true);
    setError(null);
    try {
      const before = await filesNow();
      await messenger.connectAgents(chosen, project, taskbag, language);
      onDone({
        // One agent (owner, 2026-10-12); a terminal that takes the request from the clipboard is told so.
        message: (agents ?? []).some((x) => x.id === chosen[0] && x.how === "paste")
          ? t("connect.asked_paste", { agent: t(AGENT_KEY[chosen[0]]), keys: PASTE_KEYS })
          : t("connect.asked", { agent: t(AGENT_KEY[chosen[0]]) }),
        watch: { project, before },
      });
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
      const before = await filesNow();
      const files = await messenger.writeAgentFiles(project);
      onDone({
        message: files.length > 0 ? t("agents.written", { files: files.join(", ") }) : null,
        watch: { project, before },
      });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card agents">
      <h2>{t("connect.title")}</h2>
      <p>{t("connect.body")}</p>
      <p>{t("connect.tasks")}</p>
      <div className="row project-row">
        <span>{rich("project.line", { path: <code>{project}</code> })}</span>
        {!projectFixed && (
          <button type="button" className="link" onClick={() => void changeProject()} disabled={busy}>
            {t("agents.change")}
          </button>
        )}
      </div>
      <p className="muted">{t("agents.taskbag", { path: taskbag })}</p>
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
                  <input
                    type="radio"
                    name="agent-group"
                    checked={on}
                    onChange={() => toggleGroup(g.name)}
                    disabled={busy}
                  />{" "}
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
                        {/* The Claude app asks once per folder; cancelling it starts the session in no folder. */}
                        {a.id === "claude_app" && <span className="muted small hint">{t("agents.claude_app_trust")}</span>}
                        {/* The chosen tool takes the request from the clipboard: say how, right here. */}
                        {on && picked[g.name] === a.id && a.how === "paste" && (
                          <span className="small hint paste-tip">{t("agents.paste_tip", { keys: PASTE_KEYS })}</span>
                        )}
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
        <button type="button" onClick={() => onDone({ message: null })} disabled={busy}>
          {t("agents.later")}
        </button>
      </div>
    </section>
  );
}
