// "Connect with the agents" (Task-A05-03; before it, Task-A03-03 and Task-A05-02 were two
// panels): lists the AI agents on this computer, lets the person pick one (owner, 2026-10-12) and
// the project folder, then opens it with the request typed in: add Allomos's instructions, then
// look for the project's existing tasks and, if it finds some, ask with `alm confirm` before
// moving them. Allomos never sends a request; the person does. **Manual** (Task-A05-06), offered
// even when no known agent is found, shows the same request with Copy, for any other AI.

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { Agent, InstructionFiles } from "./types";
import { movingTasks, updateNotice } from "./versionFlow";
import { COPIED_MS, MANUAL, MANUAL_LOOK_MS, connectActions, firstChoice, manualWritten } from "./manualFlow";
import type { Seen } from "./manualFlow";

export const AGENT_KEY: Record<Agent["id"], Key> = {
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
export const PASTE_KEYS = /Mac/.test(navigator.userAgent) ? "⌘V" : "Ctrl+V";

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
  actionsIn,
  onProjectChange,
  onDone,
}: {
  messenger: Messenger;
  project: string;
  taskbag: string;
  /** The project folder was just chosen (the wizard's step 3): shown, without Change…. */
  projectFixed?: boolean;
  /** Where the buttons go: the wizard's bottom row, where Next is on the other steps. */
  actionsIn?: HTMLElement | null;
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
  /** Something is out of date with this Allomos (Task-A01-25): the request is the update request,
   * and Allomos offers no "write the files for me" (owner, 2026-10-12: the agent updates them).
   * `move`: an older Allomos's taskbag, whose tasks the agent moves in (Task-A01-45). */
  const [updating, setUpdating] = useState<"section" | "move" | null>(null);

  useEffect(() => {
    let live = true;
    messenger
      .versionCheck(taskbag)
      .then((c) => {
        if (!live) return;
        const notice = updateNotice(c);
        setUpdating(notice === null ? null : movingTasks(notice) ? "move" : "section");
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger, taskbag]);

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
        setTicked(new Set([firstChoice(GROUPS.filter((g) => first[g.name]).map((g) => g.name))]));
      })
      .catch(() => {
        if (!live) return;
        setAgents([]);
        setTicked(new Set([MANUAL]));
      })
      .finally(() => window.clearTimeout(timer));
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [messenger]);

  // One agent (owner, 2026-10-12): choosing a group replaces the choice.
  const toggleGroup = (name: string) => setTicked(new Set([name]));
  const chosen = GROUPS.filter((g) => ticked.has(g.name) && picked[g.name]).map((g) => picked[g.name]);
  const manual = ticked.has(MANUAL);
  const choice = [...ticked][0] ?? null;
  const buttons = connectActions(agents === null ? null : choice, agents?.length ?? 0, updating !== null);

  // Manual: the request (the same automatic sending types in), what the project looked like when
  // it was shown, and what the AI has written since.
  const [request, setRequest] = useState<string | null>(null);
  const [before, setBefore] = useState<Seen | null>(null);
  const [written, setWritten] = useState<string[]>([]);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  useEffect(() => {
    if (!manual) return;
    let live = true;
    setRequest(null);
    setCopied(false);
    void (async () => {
      try {
        const [text, seen] = await Promise.all([messenger.manualRequest(project, taskbag, language), seenNow()]);
        if (!live) return;
        setRequest(text);
        setBefore(seen);
        setWritten([]);
      } catch (e) {
        if (live) fail(e);
      }
    })();
    return () => {
      live = false;
    };
    // `seenNow` and `fail` read the same props; ask again only when the request would change.
  }, [messenger, manual, project, taskbag, language, updating]);

  // Watch for what the automatic path watches for: a new instruction file, or one that now tells
  // about Allomos. Done stays available either way (Task-A05-06).
  useEffect(() => {
    if (!manual || !before) return;
    let live = true;
    const look = async () => {
      const now = await seenNow().catch(() => null);
      if (live && now) setWritten(manualWritten(before, now));
    };
    const timer = window.setInterval(() => void look(), MANUAL_LOOK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, manual, before, project, taskbag]);

  const copy = async () => {
    if (request === null) return;
    setError(null);
    try {
      await messenger.copyText(request);
      setCopied(true);
      window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
    } catch (e) {
      fail(e);
    }
  };

  const doneManual = () =>
    onDone({
      message: written.length > 0 ? t("manual.finished", { files: written.join(", ") }) : t("manual.pending"),
      watch: before ? { project, before: before.files } : undefined,
    });

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

  /** The project's instruction files and which mention Allomos (for Manual). */
  const seenNow = async (): Promise<Seen> => {
    const [files, setup] = await Promise.all([messenger.instructionFiles(project), messenger.projectSetup(taskbag)]);
    return { files, agents: setup.agents };
  };

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
          ? t(updating === "move" ? "move.asked_paste" : updating ? "update.asked_paste" : "connect.asked_paste", {
              agent: t(AGENT_KEY[chosen[0]]),
              keys: PASTE_KEYS,
            })
          : t(updating === "move" ? "move.asked" : updating ? "update.asked" : "connect.asked", { agent: t(AGENT_KEY[chosen[0]]) }),
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

  // Not now on the left, the main button on the right, as Cancel and Next on the wizard's other steps.
  const actions = (
    <>
      <button type="button" onClick={() => onDone({ message: null })} disabled={busy}>
        {t("agents.later")}
      </button>
      {buttons.write && (
        <button type="button" onClick={() => void write()} disabled={busy}>
          {t("agents.write")}
        </button>
      )}
      {buttons.primary === "connect" && (
        <button type="button" className="primary" onClick={() => void ask()} disabled={busy || chosen.length === 0}>
          {t("agents.ask")}
        </button>
      )}
      {buttons.primary === "done" && (
        <button type="button" className="primary" onClick={doneManual} disabled={busy}>
          {t("manual.done")}
        </button>
      )}
    </>
  );

  return (
    <section className="card agents">
      <h2>{t(updating === "move" ? "move.title" : updating ? "update.title" : "connect.title")}</h2>
      <p>{t(updating === "move" ? "move.body" : updating ? "update.body" : "connect.body")}</p>
      {!updating && <p>{t("connect.tasks")}</p>}
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
      ) : (
        <>
        {agents.length === 0 && <p>{t(updating ? "update.no_agent" : "agents.none")}</p>}
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
                        {/* How it will open: only for the chosen tool (owner, 2026-10-12). */}
                        {on && picked[g.name] === a.id && (
                          <span className="muted">
                            — {t(a.how === "link" ? "agents.how_link" : "agents.how_paste")} ({a.found})
                          </span>
                        )}
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
          {/* Manual (Task-A05-06): any AI, even when none is found; the person pastes the request. */}
          <li>
            <label>
              <input
                type="radio"
                name="agent-group"
                checked={manual}
                onChange={() => toggleGroup(MANUAL)}
                disabled={busy}
              />{" "}
              <strong>{t("agents.manual")}</strong> <span className="muted">— {t("agents.manual_how")}</span>
            </label>
            {manual && (
              <div className="manual">
                <p className="small">{rich("manual.intro", { path: <code>{project}</code> })}</p>
                <textarea
                  className="manual-request"
                  readOnly
                  rows={8}
                  aria-label={t("manual.request")}
                  value={request ?? ""}
                  onFocus={(e) => e.currentTarget.select()}
                />
                <div className="row start">
                  <button type="button" onClick={() => void copy()} disabled={busy || request === null}>
                    {t("manual.copy")}
                  </button>
                  <span className="muted small" role="status">
                    {copied ? t("manual.copied") : ""}
                  </span>
                </div>
                <p className="small" role="status">
                  {written.length > 0 ? t("manual.seen", { files: written.join(", ") }) : t("manual.waiting")}
                </p>
              </div>
            )}
          </li>
        </ul>
        </>
      )}
      {error && <p className="banner error">{error}</p>}
      {actionsIn ? createPortal(actions, actionsIn) : <div className="row actions">{actions}</div>}
    </section>
  );
}
