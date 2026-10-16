// The start page's Build a new app… (Task-A03-25-02). The person picks an empty folder, describes
// the app, chooses the agent (the ones found, or Manual) and quiet or normal mode. Start creates
// the taskbag there as the setup wizard does (Task-A05-03), records the folder as the project
// folder, and opens the agent with one request typed in: the wizard's connect request, then the
// goal prompt (`alm guide goal`) with the description and the run mode. The person presses Enter;
// Allomos sends nothing (approver, 2026-10-16). Manual shows the same request with Copy. The new
// project opens behind the dialog and follows the taskbag as the agent works.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, PASTE_KEYS } from "./AgentsPanel";
import { Markdown } from "./Markdown";
import { COPIED_MS } from "./manualFlow";
import { FIRST_MODE, MANUAL, agentChoices, canStart, checkFolder, firstAgent } from "./buildAppFlow";
import type { FolderCheck, RunMode } from "./buildAppFlow";
import { isProblem } from "./types";
import type { Agent, Opened, SetupState } from "./types";

/** The project's name: the folder's own name. */
function folderName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? "Project";
}

export function BuildAppDialog({
  messenger,
  onOpened,
  onSetup,
  onClose,
}: {
  messenger: Messenger;
  /** The taskbag was just created: the window shows it behind the dialog. */
  onOpened: (o: Opened) => void;
  /** The project folder was recorded. */
  onSetup: (state: SetupState) => void;
  /** The dialog closes; `message` says what happened (null: Cancel before anything was made). */
  onClose: (message: string | null) => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [folder, setFolder] = useState<FolderCheck | null>(null);
  const [description, setDescription] = useState("");
  const [agents, setAgents] = useState<Agent[] | null>(null);
  const [agent, setAgent] = useState<string | null>(null);
  const [mode, setMode] = useState<RunMode>(FIRST_MODE);
  const [modes, setModes] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** The taskbag made by Start, kept so trying again does not make it twice. */
  const [created, setCreated] = useState<Opened | null>(null);
  /** Manual, after Start: the request to copy. */
  const [request, setRequest] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  useEffect(() => {
    let live = true;
    messenger
      .findAgents()
      .then((found) => {
        if (!live) return;
        setAgents(found);
        setAgent(firstAgent(found));
      })
      .catch(() => {
        if (!live) return;
        setAgents([]);
        setAgent(MANUAL);
      });
    return () => {
      live = false;
    };
  }, [messenger]);

  useEffect(() => {
    let live = true;
    messenger
      .runModesText(language)
      .then((text) => live && setModes(text))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger, language]);

  const choose = async () => {
    const path = await messenger.pickFolder(t("build.folder_pick"));
    if (!path) return;
    setError(null);
    try {
      setFolder(checkFolder((await messenger.inspectFolder(path)).inspection));
    } catch (e) {
      fail(e);
    }
  };

  const tool = agents?.find((a) => a.id === agent) ?? null;
  const quiet = mode === "quiet";

  const start = async () => {
    if (folder?.kind !== "ok" || !agent) return;
    setBusy(true);
    setError(null);
    try {
      // As the setup wizard: the taskbag in `.allomos/taskbag/`, and the folder as the project folder.
      let o = created;
      if (!o) {
        o = await messenger.createTaskbag(folder.root, folderName(folder.folder), language);
        setCreated(o);
        onOpened(o);
        onSetup(await messenger.setProjectFolder(o.root, folder.folder));
      }
      if (agent === MANUAL) {
        setRequest(await messenger.buildAppRequest(folder.folder, o.root, language, description, quiet));
        return;
      }
      await messenger.buildApp(agent, folder.folder, o.root, language, description, quiet);
      const name = t(AGENT_KEY[agent as Agent["id"]]);
      onClose(tool?.how === "paste" ? t("build.asked_paste", { agent: name, keys: PASTE_KEYS }) : t("build.asked", { agent: name }));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

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

  const locked = busy || created !== null;

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard build-app" role="dialog" aria-modal="true" aria-labelledby="build-title">
        <h2 id="build-title">{t("build.title")}</h2>
        <p className="muted small">{t("build.intro")}</p>

        <div className="dialog-body wizard-body">
          {request === null ? (
            <>
              <h3>{t("build.folder_title")}</h3>
              {folder === null && <p className="muted">{t("build.folder_hint")}</p>}
              {folder?.kind === "ok" && <p>{rich("build.folder_ok", { path: <code>{folder.folder}</code> })}</p>}
              {folder?.kind === "not_empty" && (
                <p className="banner error">{rich("build.folder_not_empty", { path: <code>{folder.folder}</code> })}</p>
              )}
              {folder?.kind === "not_a_folder" && <p className="banner error">{t("error.not_a_folder", { path: folder.path })}</p>}
              <div className="row start">
                <button type="button" onClick={() => void choose()} disabled={locked}>
                  {t("wizard.choose")}
                </button>
              </div>

              <h3>
                <label htmlFor="build-description">{t("build.description_title")}</label>
              </h3>
              <textarea
                id="build-description"
                className="build-description"
                rows={5}
                value={description}
                placeholder={t("build.description_placeholder")}
                onChange={(e) => setDescription(e.currentTarget.value)}
                disabled={locked}
              />

              <h3>{t("build.agent_title")}</h3>
              {agents === null ? (
                <p className="muted searching" role="status">
                  <span className="spinner" aria-hidden="true" /> {t("agents.searching")}
                </p>
              ) : (
                <ul className="agent-list">
                  {agentChoices(agents).map((id) => {
                    const found = agents.find((a) => a.id === id);
                    return (
                      <li key={id}>
                        <label>
                          <input
                            type="radio"
                            name="build-agent"
                            checked={agent === id}
                            onChange={() => setAgent(id)}
                            disabled={busy}
                          />{" "}
                          <strong>{id === MANUAL ? t("agents.manual") : t(AGENT_KEY[id as Agent["id"]])}</strong>{" "}
                          <span className="muted">
                            —{" "}
                            {id === MANUAL
                              ? t("agents.manual_how")
                              : `${t(found?.how === "link" ? "agents.how_link" : "agents.how_paste")} (${found?.found ?? ""})`}
                          </span>
                        </label>
                        {id === "claude_app" && agent === id && <p className="muted small hint">{t("agents.claude_app_trust")}</p>}
                        {found?.how === "paste" && agent === id && (
                          <p className="small hint paste-tip">{t("agents.paste_tip", { keys: PASTE_KEYS })}</p>
                        )}
                      </li>
                    );
                  })}
                </ul>
              )}

              <h3>{t("build.mode_title")}</h3>
              <ul className="agent-list">
                {(["quiet", "normal"] as const).map((m) => (
                  <li key={m}>
                    <label>
                      <input type="radio" name="build-mode" checked={mode === m} onChange={() => setMode(m)} disabled={busy} />{" "}
                      <strong>{t(m === "quiet" ? "build.mode_quiet" : "build.mode_normal")}</strong>{" "}
                      <span className="muted">— {t(m === "quiet" ? "build.mode_quiet_how" : "build.mode_normal_how")}</span>
                    </label>
                  </li>
                ))}
              </ul>
              {modes && (
                <details className="run-modes">
                  <summary>{t("build.mode_more")}</summary>
                  <Markdown source={modes} onLink={() => undefined} />
                </details>
              )}
              <p className="banner update">{t("build.auto_mode")}</p>
            </>
          ) : (
            <div className="manual">
              <p className="small">{rich("build.manual_intro", { path: <code>{folder?.kind === "ok" ? folder.folder : ""}</code> })}</p>
              <textarea
                className="manual-request"
                readOnly
                rows={12}
                aria-label={t("manual.request")}
                value={request}
                onFocus={(e) => e.currentTarget.select()}
              />
              <div className="row start">
                <button type="button" onClick={() => void copy()}>
                  {t("manual.copy")}
                </button>
                <span className="muted small" role="status">
                  {copied ? t("manual.copied") : ""}
                </span>
              </div>
            </div>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        <div className="row">
          {request === null ? (
            <>
              <button type="button" onClick={() => onClose(created ? t("build.not_started") : null)} disabled={busy}>
                {t("wizard.cancel")}
              </button>
              <button
                type="button"
                className="primary"
                onClick={() => void start()}
                disabled={busy || !canStart(folder, description, agent)}
              >
                {t("build.start")}
              </button>
            </>
          ) : (
            <button type="button" className="primary" onClick={() => onClose(t("build.manual_done"))}>
              {t("manual.done")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
