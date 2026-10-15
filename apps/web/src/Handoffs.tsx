// Handoffs (Task-A03-06): the notes a session leaves so that whoever continues, an agent or a
// person, does not start blind. The Handoffs tab (after Iterations) lists them newest first; a
// person can write one, and change when agents must write them (Handoff settings).

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import type { Messenger } from "./messenger";
import { filterHandoffs, localTime, writers } from "./handoffFlow";
import { handoffMatches, snippet } from "./searchFlow";
import { Highlight, ListBanners, Marked } from "./Search";
import { filterBanner, filterGroups, filterName, handoffGroups, searchBanner, taskCount, type StateFilter } from "./stopFlow";
import { isProblem } from "./types";
import type { Handoff, HandoffSettings, Task } from "./types";

/** The Handoffs tab. */
export function HandoffsView({
  handoffs,
  terms,
  tasks,
  stateFilter,
  onShowAll,
  task,
  onTask,
  onOpenTask,
  onWrite,
  onSettings,
  onLink,
  openKeys,
  onToggle,
  onShown,
  refreshLine,
}: {
  handoffs: Handoff[];
  /** The search box's words (Task-A01-36): only handoffs holding them are shown, the words marked. */
  terms: string[];
  tasks: Task[];
  /** The list's Show filter (Task-A01-44): a handoff is shown when it names a task the filter keeps. */
  stateFilter: StateFilter;
  /** Turns the Show filter off. */
  onShowAll: () => void;
  /** The task filter, kept by the window so the task pane can open the tab on one task. */
  task: string | null;
  onTask: (task: string | null) => void;
  onOpenTask: (id: string) => void;
  onWrite: () => void;
  onSettings: () => void;
  onLink: (href: string) => void;
  /** The open handoffs, by file; each opens and closes on its own, remembered per project
   * (Task-A01-26). */
  openKeys: string[];
  onToggle: (file: string) => void;
  /** The handoffs shown now, so Collapse all / Expand all act on them. */
  onShown: (files: string[]) => void;
  /** When the list was last read, with Refresh now (Task-A01-47), shown under the banners. */
  refreshLine?: ReactNode;
}) {
  const { t } = useI18n();
  const [writer, setWriter] = useState<string | null>(null);
  const [latestOnly, setLatestOnly] = useState(true);
  // While searching, older handoffs are searched too: "Latest only" would hide what was asked for.
  const searching = terms.length > 0;
  const listed = filterHandoffs(handoffs, { task, writer, latestOnly: latestOnly && !searching }).filter((h) =>
    handoffMatches(h, terms),
  );
  // Task-A01-44: under the Show filter, each handoff is the group of the tasks it names; one naming
  // none the filter keeps is not drawn, and the banner says how many were left out.
  const kept = filterGroups(handoffGroups(listed, tasks), stateFilter);
  const keptFiles = new Set(kept.map((g) => g.file));
  const shown = stateFilter === "all" ? listed : listed.filter((h) => keptFiles.has(h.file));
  // Task-A01-48: under a filter, the search banner counts the handoffs the filter and the search
  // keep together, and says the filter is on.
  const searchWords =
    searching && shown.length > 0
      ? searchBanner("handoff", shown.length, terms.join(" "), t, stateFilter === "all" ? undefined : filterName(stateFilter, t))
      : [];
  const filterWords =
    listed.length > 0 ? filterBanner(stateFilter, { kind: "handoff", tasks: taskCount(kept), groups: kept.length, total: listed.length }, t) : [];
  const shownFiles = shown.map((h) => h.file).join("\n");
  useEffect(() => onShown(shownFiles ? shownFiles.split("\n") : []), [shownFiles]);
  const titleOf = (id: string) => tasks.find((x) => x.id === id)?.title ?? "";

  return (
    <section className="handoffs" aria-label={t("handoffs.title")}>
      <ListBanners search={searchWords} filter={filterWords} onShowAll={onShowAll} />
      {refreshLine}
      <div className="row handoff-tools">
        <label>
          {t("handoffs.task")}{" "}
          <select value={task ?? ""} onChange={(e) => onTask(e.target.value || null)}>
            <option value="">{t("handoffs.any")}</option>
            {tasks.map((x) => (
              <option key={x.id} value={x.id}>
                {x.id} {x.title}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("handoffs.writer")}{" "}
          <select value={writer ?? ""} onChange={(e) => setWriter(e.target.value || null)}>
            <option value="">{t("handoffs.any")}</option>
            {writers(handoffs).map((w) => (
              <option key={w} value={w}>
                {w}
              </option>
            ))}
          </select>
        </label>
        <label>
          <input type="checkbox" checked={latestOnly} disabled={searching} onChange={(e) => setLatestOnly(e.target.checked)} />{" "}
          {t("handoffs.latest_only")}
        </label>
        <span className="spacer" />
        {/* A button at the far right (owner, 2026-10-12). */}
        <button type="button" onClick={onSettings}>
          {t("handoffs.settings")}
        </button>
      </div>
      {listed.length > 0 && shown.length === 0 ? null : shown.length === 0 ? (
        <p className="muted">{handoffs.length === 0
            ? t("handoffs.none")
            : searching
              ? t("search.nothing", { query: terms.join(" ") })
              : t("handoffs.none_match")}</p>
      ) : (
        <ul className="handoff-list">
          {shown.map((h) => {
            const open = openKeys.includes(h.file);
            return (
            <li key={h.file} className={open ? "open" : undefined}>
              <button type="button" className="handoff-head" aria-expanded={open} onClick={() => onToggle(h.file)}>
                <span className="when">{localTime(h.updated)}</span>
                <span className="who">{h.writer || t("handoffs.unknown_writer")}</span>
                <span className="what">
                  <Highlight text={h.summary || t("handoffs.empty")} terms={terms} />
                </span>
              </button>
              <div className="handoff-tasks">
                {h.tasks.length === 0 ? (
                  <span className="muted small">{t("handoffs.no_task")}</span>
                ) : (
                  h.tasks.map((id) => (
                    <button key={id} type="button" className="link small" title={titleOf(id)} onClick={() => onOpenTask(id)}>
                      {id}
                    </button>
                  ))
                )}
              </div>
              {/* A closed handoff found by its text shows the words around the first match. */}
              {!open && searching && <BodyHit body={h.body} terms={terms} />}
              {open && (
                <div className="handoff-body">
                  <Markdown source={h.body} onLink={onLink} />
                  <p className="muted small">
                    <code>{h.file}</code>
                  </p>
                </div>
              )}
            </li>
            );
          })}
        </ul>
      )}
      {/* A link below the list, outside it, at the far right (owner, 2026-10-12). */}
      <button type="button" className="link write-handoff" onClick={onWrite}>
        {t("handoffs.write")}
      </button>
    </section>
  );
}

/** The line under a closed handoff found by its text (Task-A01-36). */
function BodyHit({ body, terms }: { body: string; terms: string[] }) {
  const parts = snippet(body, terms);
  return parts ? (
    <p className="search-snippet small">
      <Marked parts={parts} />
    </p>
  ) : null;
}

/** A task's latest handoff, in the task pane. */
export function LatestHandoff({ handoff, onAll }: { handoff: Handoff | null; onAll: () => void }) {
  const { t } = useI18n();
  return (
    <section>
      <h3>{t("pane.latest_handoff")}</h3>
      {handoff ? (
        <>
          <p className="small">
            <span className="muted">
              {localTime(handoff.updated)} · {handoff.writer || t("handoffs.unknown_writer")}
            </span>
            <br />
            {handoff.summary || t("handoffs.empty")}
          </p>
          <button type="button" className="link small" onClick={onAll}>
            {t("pane.all_handoffs")}
          </button>
        </>
      ) : (
        <p className="muted small">{t("pane.no_handoff")}</p>
      )}
    </section>
  );
}

/** When agents must write a handoff, and its template (owner, 2026-10-12: in the app and `alm`). */
export function HandoffSettingsDialog({ messenger, root, onClose }: { messenger: Messenger; root: string; onClose: () => void }) {
  const { t, problem } = useI18n();
  const [settings, setSettings] = useState<HandoffSettings | null>(null);
  const [quota, setQuota] = useState("");
  const [context, setContext] = useState("");
  const [writeWhen, setWriteWhen] = useState("");
  const [template, setTemplate] = useState("");
  const [agent, setAgent] = useState("");
  const [agentQuota, setAgentQuota] = useState("");
  const [agentContext, setAgentContext] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const fill = (s: HandoffSettings) => {
    setSettings(s);
    setQuota(String(s.applied.quota_gate));
    setContext(String(s.applied.context_gate));
    setWriteWhen(s.applied.write_when);
    setTemplate(s.template);
  };
  useEffect(() => {
    messenger.handoffSettings(root).then(fill, (e) => setError(isProblem(e) ? problem(e) : String(e)));
    // Read once when the dialog opens; the language only words an error.
  }, [messenger, root]);

  const run = async (work: () => Promise<HandoffSettings>) => {
    setBusy(true);
    setError(null);
    try {
      fill(await work());
      return true;
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    const ok = await run(async () => {
      await messenger.setHandoffRules(root, { quota, context, writeWhen });
      return messenger.setHandoffTemplate(root, template);
    });
    if (ok) onClose();
  };
  const addOverride = () =>
    void run(() =>
      messenger.setHandoffRules(root, {
        agent,
        quota: agentQuota || undefined,
        context: agentContext || undefined,
      }),
    ).then((ok) => {
      if (ok) {
        setAgent("");
        setAgentQuota("");
        setAgentContext("");
      }
    });
  const overrides = Object.entries(settings?.rules.overrides ?? {});
  // An override's empty gate follows the project's: show that value too, so 5% is not a mystery.
  const gate = (v: number | undefined, project: number | undefined) =>
    v === undefined ? t("handoff_settings.project_value", { value: project ?? "—" }) : `${v}%`;

  return (
    <div className="overlay" role="presentation">
      <section className="dialog handoff-settings" role="dialog" aria-modal="true" aria-labelledby="handoff-settings-title">
        <h2 id="handoff-settings-title">{t("handoff_settings.title")}</h2>
        <p className="muted">{t("handoff_settings.body")}</p>
        <div className="dialog-body form">
          <label>
            {t("handoff_settings.quota")}
            <span className="inline">
              <input type="number" min={0} max={100} value={quota} onChange={(e) => setQuota(e.target.value)} disabled={busy} /> %
            </span>
          </label>
          <label>
            {t("handoff_settings.context")}
            <span className="inline">
              <input type="number" min={0} max={100} value={context} onChange={(e) => setContext(e.target.value)} disabled={busy} /> %
            </span>
          </label>
          <label>
            {t("handoff_settings.write_when")}
            <input type="text" value={writeWhen} onChange={(e) => setWriteWhen(e.target.value)} disabled={busy} />
          </label>
          <h3>{t("handoff_settings.overrides")}</h3>
          <p className="muted small">{t("handoff_settings.overrides_body")}</p>
          {overrides.length > 0 && (
            <ul className="overrides">
              {overrides.map(([name, g]) => (
                <li key={name}>
                  <code>{name}</code> · {t("handoff_settings.quota_label")} {gate(g.quota_gate, settings?.applied.quota_gate)} · {t("handoff_settings.context_label")}{" "}
                  {gate(g.context_gate, settings?.applied.context_gate)}{" "}
                  <button type="button" className="link small" onClick={() => void run(() => messenger.setHandoffRules(root, { agent: name, clear: true }))} disabled={busy}>
                    {t("handoff_settings.remove")}
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="row start override-row">
            <input type="text" placeholder={t("handoff_settings.agent_placeholder")} value={agent} onChange={(e) => setAgent(e.target.value)} disabled={busy} />
            <input type="number" min={0} max={100} placeholder={t("handoff_settings.quota_short")} value={agentQuota} onChange={(e) => setAgentQuota(e.target.value)} disabled={busy} />
            <input type="number" min={0} max={100} placeholder={t("handoff_settings.context_short")} value={agentContext} onChange={(e) => setAgentContext(e.target.value)} disabled={busy} />
            <button type="button" onClick={addOverride} disabled={busy || !agent.trim() || (!agentQuota && !agentContext)}>
              {t("handoff_settings.add")}
            </button>
          </div>
          <h3>{t("handoff_settings.template")}</h3>
          <textarea rows={12} value={template} onChange={(e) => setTemplate(e.target.value)} disabled={busy} spellCheck={false} />
        </div>
        {error && <p className="banner error">{error}</p>}
        <div className="row">
          <button type="button" onClick={() => void run(() => messenger.setHandoffRules(root, { clear: true }))} disabled={busy}>
            {t("handoff_settings.defaults")}
          </button>
          <button type="button" onClick={onClose} disabled={busy}>
            {t("handoff_settings.cancel")}
          </button>
          <button type="button" className="primary" onClick={() => void save()} disabled={busy || !settings}>
            {t("handoff_settings.save")}
          </button>
        </div>
      </section>
    </div>
  );
}

/** A person writes a handoff (owner, 2026-10-12): part 1 is for anyone; part 2 is optional. */
export function HandoffForm({
  messenger,
  root,
  tasks,
  task,
  onClose,
}: {
  messenger: Messenger;
  root: string;
  tasks: Task[];
  /** A task to start with, for example the one in the task pane. */
  task: string | null;
  onClose: (written: boolean) => void;
}) {
  const { t, problem } = useI18n();
  const [writer, setWriter] = useState(() => {
    try {
      return localStorage.getItem(WRITER_KEY) ?? "";
    } catch {
      return "";
    }
  });
  const [chosen, setChosen] = useState<string[]>(task ? [task] : []);
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    messenger.handoffSettings(root).then(
      (s) => setBody((b) => b || s.template),
      () => undefined,
    );
  }, [messenger, root]);

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await messenger.writeHandoff(root, writer.trim(), chosen, body);
      try {
        localStorage.setItem(WRITER_KEY, writer.trim());
      } catch {
        /* the name is asked again next time */
      }
      onClose(true);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
    } finally {
      setBusy(false);
    }
  };
  const toggle = (id: string) => setChosen((list) => (list.includes(id) ? list.filter((x) => x !== id) : [...list, id]));
  const open = tasks.filter((x) => x.state !== "done" && x.state !== "dropped");

  return (
    <div className="overlay" role="presentation">
      <section className="dialog handoff-form" role="dialog" aria-modal="true" aria-labelledby="handoff-form-title">
        <h2 id="handoff-form-title">{t("handoff_form.title")}</h2>
        <p className="muted">{t("handoff_form.body")}</p>
        <div className="dialog-body form">
          <label>
            {t("handoff_form.writer")}
            <input type="text" value={writer} onChange={(e) => setWriter(e.target.value)} disabled={busy} />
          </label>
          <fieldset>
            <legend>{t("handoff_form.tasks")}</legend>
            <div className="task-checks">
              {open.map((x) => (
                <label key={x.id}>
                  <input type="checkbox" checked={chosen.includes(x.id)} onChange={() => toggle(x.id)} disabled={busy} /> {x.id} {x.title}
                </label>
              ))}
            </div>
          </fieldset>
          <label>
            {t("handoff_form.text")}
            <textarea rows={14} value={body} onChange={(e) => setBody(e.target.value)} disabled={busy} />
          </label>
        </div>
        {error && <p className="banner error">{error}</p>}
        <div className="row">
          <button type="button" onClick={() => onClose(false)} disabled={busy}>
            {t("handoff_form.cancel")}
          </button>
          <button type="button" className="primary" onClick={() => void save()} disabled={busy || !writer.trim() || !body.trim()}>
            {t("handoff_form.save")}
          </button>
        </div>
      </section>
    </div>
  );
}

/** The person's name for their handoffs, remembered on this computer. */
const WRITER_KEY = "allomos.handoffWriter";
