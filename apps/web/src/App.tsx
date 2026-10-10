import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { LANGUAGES, LANGUAGE_NAMES, useI18n } from "./i18n";
import type { Language, Message } from "./i18n";
import { AgentLine } from "./AgentLine";
import { AgentsPanel } from "./AgentsPanel";
import { ConfirmDialog } from "./ConfirmDialog";
import { TaskPane } from "./TaskPane";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { Issue, Iteration, Opened, Problem, SetupState, Task } from "./types";

/** The file manager's name on this computer, for the button's words. */
const FILE_MANAGER_KEY = /Mac/.test(navigator.userAgent)
  ? "top.show_in_finder"
  : /Windows/.test(navigator.userAgent)
    ? "top.show_in_explorer"
    : "top.show_in_files";

const LAST_ROOT_KEY = "fliphopper.lastRoot";
/** Recently opened project folders, newest first, for File → Open Recent. */
const RECENT_KEY = "fliphopper.recentFolders";
const RECENT_MAX = 10;

/** The project folder the user picked, from a seabag root (`<folder>/.fliphopper/seabag`). */
function projectFolder(root: string): string {
  return root.replace(/[\\/]\.fliphopper[\\/]seabag[\\/]?$/, "");
}

function readRecent(): string[] {
  try {
    const list: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    return Array.isArray(list) ? list.filter((x): x is string => typeof x === "string").slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

function saveRecent(list: string[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    /* storage unavailable: the menu still shows this session's list */
  }
}
/** How often the window checks whether the seabag's files changed, for example through `fh`. */
const FOLLOW_MS = 1000;

function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  // The project is named after the picked folder, not the `.fliphopper/seabag` inside it.
  if (parts.at(-1) === "seabag" && parts.at(-2) === ".fliphopper") parts.splice(-2);
  return parts.at(-1) ?? "Project";
}

type Pending = { folder: string; suggestedRoot: string; gitRoot: string | null };

/** A banner's text: an interface message, or a problem sent by Rust. */
type Note = Message | { problem: Problem } | { text: string };

function errorNote(e: unknown): Note {
  return isProblem(e) ? { problem: e } : { key: "error.unexpected", params: { detail: String(e) } };
}

export function App({ messenger }: { messenger: Messenger }) {
  const { t, rich, problem, language } = useI18n();
  // Read when a seabag is created (for its README), without reopening the folder when it changes.
  const languageRef = useRef(language);
  languageRef.current = language;
  const [opened, setOpened] = useState<Opened | null>(null);
  const openedRef = useRef(opened);
  openedRef.current = opened;
  const [pending, setPending] = useState<Pending | null>(null);
  const [notices, setNotices] = useState<Note[]>([]);
  const [error, setError] = useState<Note | null>(null);
  const [recent, setRecent] = useState<string[]>(readRecent);
  const changeRecent = useCallback((change: (list: string[]) => string[]) => {
    setRecent((list) => {
      const next = change(list).slice(0, RECENT_MAX);
      saveRecent(next);
      return next;
    });
  }, []);
  const noteText = (n: Note) => ("problem" in n ? problem(n.problem) : "text" in n ? n.text : t(n.key, n.params));

  const show = useCallback((o: Opened, message: Note | null) => {
    setOpened(o);
    setPending(null);
    setNotices(message ? [message] : []);
    setAgentsFor(null);
    setSetup(null);
    setSkipped({ md: false, migrate: false });
    setError(null);
    try {
      localStorage.setItem(LAST_ROOT_KEY, o.root);
    } catch {
      /* storage unavailable: nothing to remember */
    }
    const folder = projectFolder(o.root);
    changeRecent((list) => [folder, ...list.filter((f) => f !== folder)]);
  }, [changeRecent]);

  /** The agents panel: tell AI agents about FlipHopper (Task-A03-03), or ask one agent to find the
   * project's tasks and move them into FlipHopper (`convert`, Task-A05-02). */
  const [agentsFor, setAgentsFor] = useState<{ project: string; seabag: string; convert?: boolean } | null>(null);

  /** The open-folder flow (docs/product/OPEN-FOLDER-FLOW.md): the open seabag's project folder,
   * agent instructions and tasks, and the steps the person put off with "Not now" this time. */
  const [setup, setSetup] = useState<SetupState | null>(null);
  const setupRef = useRef(setup);
  setupRef.current = setup;
  const [skipped, setSkipped] = useState({ md: false, migrate: false });
  const refreshSetup = useCallback(
    async (root: string) => {
      try {
        const state = await messenger.projectSetup(root);
        if (openedRef.current?.root === root) setSetup(state);
      } catch (e) {
        setError(errorNote(e));
      }
    },
    [messenger],
  );

  const openFolder = useCallback(
    async (path: string) => {
      setError(null);
      try {
        const info = await messenger.inspectFolder(path);
        const i = info.inspection;
        if (i.kind === "found") {
          show(await messenger.openSeabag(i.root), null);
        } else if (i.kind === "empty") {
          const o = await messenger.createSeabag(i.suggested_root, folderName(i.folder), languageRef.current);
          show(o, { key: o.readme_added ? "notice.created_here_readme" : "notice.created_here" });
        } else if (i.kind === "no_seabag") {
          setPending({ folder: i.folder, suggestedRoot: i.suggested_root, gitRoot: info.git_root });
          setAgentsFor(null);
        } else {
          setError({ key: "error.not_a_folder", params: { path: i.path } });
        }
      } catch (e) {
        setError(errorNote(e));
      }
    },
    [messenger, show],
  );

  // Steps 2–5 of the flow, one at a time: after opening, and whenever the seabag's files change.
  const root0 = opened?.root;
  const taskCount = opened?.loaded.seabag?.tasks.length;
  useEffect(() => {
    if (root0) void refreshSetup(root0);
  }, [root0, taskCount, refreshSetup]);
  useEffect(() => {
    if (!opened || !setup || agentsFor) return;
    const project = setup.project_folder;
    if (!project) return; // step 2 asks on the page
    if (!(setup.agents.codex && setup.agents.claude) && !skipped.md) {
      setAgentsFor({ project, seabag: opened.root });
    } else if (!setup.has_tasks && !skipped.migrate) {
      setAgentsFor({ project, seabag: opened.root, convert: true });
    }
  }, [opened, setup, agentsFor, skipped]);
  // File → Move Tasks Into FlipHopper… only while the open seabag has a project folder.
  const hasProjectFolder = Boolean(opened && setup?.project_folder);
  useEffect(() => {
    void messenger.setConvertEnabled(hasProjectFolder).catch(() => undefined);
  }, [messenger, hasProjectFolder]);
  /** The project folder for the agents panel: the recorded one, else the folder holding `.fliphopper/`. */
  const agentsProject = (root: string) => setupRef.current?.project_folder ?? projectFolder(root);

  // Follow changes made outside the window (by `fh` or by hand): reload when the files change.
  const root = opened?.root;
  useEffect(() => {
    if (!root) return;
    let last: string | null = null;
    let busy = false;
    const check = async () => {
      if (busy) return;
      busy = true;
      try {
        const now = await messenger.seabagStamp(root);
        if (last !== null && now !== last) {
          const fresh = await messenger.openSeabag(root);
          setOpened((o) => (o?.root === root ? fresh : o));
        }
        last = now;
      } catch {
        /* the folder may be gone for a moment; try again next time */
      } finally {
        busy = false;
      }
    };
    void check();
    const timer = window.setInterval(() => void check(), FOLLOW_MS);
    return () => window.clearInterval(timer);
  }, [messenger, root]);

  // On start: a folder given on the command line wins; otherwise reopen the last one.
  useEffect(() => {
    void (async () => {
      let start: string | null;
      try {
        start = await messenger.startupFolder();
      } catch (e) {
        // A folder was given but is not one: say so instead of reopening the last folder.
        setError(errorNote(e));
        return;
      }
      if (!start) {
        try {
          start = localStorage.getItem(LAST_ROOT_KEY);
        } catch {
          start = null;
        }
      }
      if (start) await openFolder(start);
    })();
  }, [messenger, openFolder]);

  const choose = async () => {
    const path = await messenger.pickFolder(t("picker.title"));
    if (path) await openFolder(path);
  };

  // File → Open Folder… (⌘O / Ctrl+O) does what the "choose a folder" button does.
  const chooseRef = useRef(choose);
  chooseRef.current = choose;
  useEffect(() => {
    const stop = messenger.onOpenFolderMenu(() => void chooseRef.current());
    return () => void stop.then((f) => f()).catch(() => undefined);
  }, [messenger]);

  // The menu's words follow the interface's language.
  useEffect(() => {
    void messenger
      .setMenuTexts({
        file: t("menu.file"),
        openFolder: t("menu.open_folder"),
        openRecent: t("menu.open_recent"),
        clearMenu: t("menu.clear_recent"),
        noRecent: t("menu.no_recent"),
        agents: t("menu.agents"),
        convert: t("menu.convert"),
      })
      .catch(() => undefined);
  }, [messenger, t]);

  // File → Open Recent: show the list; open a chosen folder (a folder that is gone leaves the
  // list and shows the usual message); Clear Menu empties it.
  useEffect(() => {
    void messenger.setRecentFolders(recent).catch(() => undefined);
  }, [messenger, recent]);
  useEffect(() => {
    const stopOpen = messenger.onOpenRecentMenu((folder) => {
      void (async () => {
        const info = await messenger.inspectFolder(folder).catch(() => null);
        if (info?.inspection.kind === "not_a_folder") changeRecent((list) => list.filter((f) => f !== folder));
        await openFolder(folder);
      })();
    });
    const stopClear = messenger.onClearRecentMenu(() => changeRecent(() => []));
    const stopAgents = messenger.onAgentsMenu(() => {
      const root = openedRef.current?.root;
      if (root) setAgentsFor({ project: agentsProject(root), seabag: root });
    });
    // File → Move Tasks Into FlipHopper… (enabled only with a recorded project folder).
    const stopConvert = messenger.onConvertMenu(() => {
      const root = openedRef.current?.root;
      const project = setupRef.current?.project_folder;
      if (root && project) setAgentsFor({ project, seabag: root, convert: true });
    });
    return () => {
      void stopOpen.then((f) => f()).catch(() => undefined);
      void stopClear.then((f) => f()).catch(() => undefined);
      void stopAgents.then((f) => f()).catch(() => undefined);
      void stopConvert.then((f) => f()).catch(() => undefined);
    };
  }, [messenger, openFolder, changeRecent]);

  const createNested = async () => {
    if (!pending) return;
    try {
      const o = await messenger.createSeabag(pending.suggestedRoot, folderName(pending.folder), language);
      show(o, { key: "notice.created_nested" });
    } catch (e) {
      setError(errorNote(e));
    }
  };

  return (
    <div className="app">
      <header className="top">
        <div className="brand" aria-label="FlipHopper">
          {/* The FlipHopper mark (owner's choice: layout Y2, colours D4): a ring where a ball was, a solid ball where it
              arrived, a spark for the jump. Same drawing as the app icon's small form. */}
          <svg viewBox="100 100 824 824" aria-hidden="true">
            <circle className="ring jade" cx="265" cy="512" r="105" />
            <circle className="jade" cx="759" cy="512" r="138" />
            <circle className="ring terracotta" cx="512" cy="265" r="105" />
            <circle className="terracotta" cx="512" cy="759" r="138" />
            <path className="spark" d="M512 392 L552 472 L632 512 L552 552 L512 632 L472 552 L392 512 L472 472 Z" />
          </svg>
          <span>FlipHopper</span>
        </div>
        {opened?.loaded.seabag && (
          <div className="where">
            <strong>{opened.loaded.seabag.project.name}</strong>
            <code title={opened.root}>{opened.root}</code>
            <button
              type="button"
              className="link"
              onClick={() => messenger.showInFileManager(opened.root).catch((e) => setError(errorNote(e)))}
            >
              {t(FILE_MANAGER_KEY)}
            </button>
          </div>
        )}
        <LanguagePicker />
        <button type="button" className="primary" onClick={choose}>
          {opened ? t("top.open_another") : t("top.choose_folder")}
        </button>
      </header>

      <main className="main">
        {error && (
          <Banner kind="error" onClose={() => setError(null)}>
            {noteText(error)}
          </Banner>
        )}
        {opened?.loaded.seabag && (
          <AgentLine
            messenger={messenger}
            root={opened.root}
            onTell={() => setAgentsFor({ project: agentsProject(opened.root), seabag: opened.root })}
          />
        )}
        {notices.map((n, i) => (
          <Banner key={i} kind="ok" onClose={() => setNotices((list) => list.filter((_, j) => j !== i))}>
            {noteText(n)}
          </Banner>
        ))}
        {agentsFor && (
          <AgentsPanel
            key={agentsFor.seabag}
            messenger={messenger}
            project={agentsFor.project}
            seabag={agentsFor.seabag}
            convert={agentsFor.convert}
            onProjectChange={async (folder) => {
              const state = await messenger.setProjectFolder(agentsFor.seabag, folder);
              setSetup(state);
            }}
            onDone={(message) => {
              // Asked or not now: this step is done until the folder is opened again.
              setSkipped((s) => (agentsFor.convert ? { ...s, migrate: true } : { ...s, md: true }));
              setAgentsFor(null);
              if (message) setNotices((list) => [...list, { text: message }]);
            }}
          />
        )}

        {opened && setup && !setup.project_folder && (
          <ProjectFolderStep
            fliphopperFolder={setup.fliphopper_folder}
            onChoose={async (folder) => {
              try {
                setSetup(await messenger.setProjectFolder(opened.root, folder));
              } catch (e) {
                setError(errorNote(e));
              }
            }}
            pick={() => messenger.pickFolder(t("folder.pick"))}
          />
        )}

        {pending && (
          <section className="card">
            <h2>{t("pending.title")}</h2>
            <p>{rich("pending.body", { folder: <code>{pending.folder}</code> })}</p>
            {pending.gitRoot && <p className="muted">{rich("pending.git", { repo: <code>{pending.gitRoot}</code> })}</p>}
            <div className="row">
              <button type="button" className="primary" onClick={createNested}>
                {t("pending.create")}
              </button>
              <button type="button" onClick={() => setPending(null)}>
                {t("pending.cancel")}
              </button>
            </div>
          </section>
        )}

        {!opened && !pending && <Welcome onChoose={choose} />}

        {/* Questions an agent asks with `fh confirm` about the folder this window shows (Task-A05-02). */}
        {/* `fh confirm` asks about the folder holding `.fliphopper/`, where its --seabag points. */}
        {opened && (
          <ConfirmDialog
            messenger={messenger}
            project={projectFolder(opened.root)}
            onCancelled={() => setNotices((list) => [...list, { key: "confirm.cancelled" }])}
          />
        )}

        {opened && <SeabagView opened={opened} messenger={messenger} onError={(e) => setError(errorNote(e))} />}
      </main>
    </div>
  );
}

/** A message above the task list, with a button that clears it (owner, 2026-10-10). */
function Banner({ kind, onClose, children }: { kind: "ok" | "error"; onClose: () => void; children: ReactNode }) {
  const { t } = useI18n();
  return (
    <div className={`banner ${kind}`} role={kind === "error" ? "alert" : "status"}>
      <span>{children}</span>
      <button type="button" className="banner-close" aria-label={t("banner.close")} title={t("banner.close")} onClick={onClose}>
        ×
      </button>
    </div>
  );
}

/** Step 2 of the flow: no project folder is recorded and none was found beside `.fliphopper/`. */
function ProjectFolderStep({
  fliphopperFolder,
  onChoose,
  pick,
}: {
  fliphopperFolder: string;
  onChoose: (folder: string) => Promise<void>;
  pick: () => Promise<string | null>;
}) {
  const { t, rich } = useI18n();
  const [busy, setBusy] = useState(false);
  const choose = async (folder: string | null) => {
    if (!folder) return;
    setBusy(true);
    await onChoose(folder);
    setBusy(false);
  };
  return (
    <section className="card">
      <h2>{t("folder.title")}</h2>
      <p>{rich("folder.body", { folder: <code>{fliphopperFolder}</code> })}</p>
      <div className="row">
        <button type="button" className="primary" disabled={busy} onClick={() => void choose(fliphopperFolder)}>
          {t("folder.this")}
        </button>
        <button type="button" disabled={busy} onClick={() => void pick().then(choose)}>
          {t("folder.other")}
        </button>
      </div>
    </section>
  );
}

function LanguagePicker() {
  const { t, choice, setChoice } = useI18n();
  return (
    <label className="language">
      <span className="visually-hidden">{t("language.label")}</span>
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="10" cy="10" r="7.5" />
        <path d="M2.5 10h15M10 2.5c2.2 2.3 2.2 12.7 0 15M10 2.5c-2.2 2.3-2.2 12.7 0 15" />
      </svg>
      <select value={choice ?? ""} onChange={(e) => setChoice((e.target.value || null) as Language | null)}>
        <option value="">{t("language.system")}</option>
        {LANGUAGES.map((l) => (
          <option key={l} value={l} lang={l}>
            {LANGUAGE_NAMES[l]}
          </option>
        ))}
      </select>
    </label>
  );
}

function Welcome({ onChoose }: { onChoose: () => void }) {
  const { t, rich } = useI18n();
  return (
    <section className="welcome">
      <h1>{t("welcome.title")}</h1>
      <p>{rich("welcome.body", { dir: <code>.fliphopper/seabag/</code> })}</p>
      <button type="button" className="primary" onClick={onChoose}>
        {t("top.choose_folder")}
      </button>
    </section>
  );
}

/** How the task list is grouped; remembered in the window's storage (Task-A01-15). */
type GroupBy = "area" | "iteration";
const GROUP_BY_KEY = "fliphopper.groupBy";

function readGroupBy(): GroupBy {
  try {
    return localStorage.getItem(GROUP_BY_KEY) === "iteration" ? "iteration" : "area";
  } catch {
    return "area";
  }
}

/** One group of the list as shown: an area or an iteration with its tasks. */
type Group = { key: string; id: string; name: string; sub?: string; empty: string; tasks: Task[] };

function SeabagView({
  opened,
  messenger,
  onError,
}: {
  opened: Opened;
  messenger: Messenger;
  onError: (e: unknown) => void;
}) {
  const { t, rich } = useI18n();
  const { seabag: box, issues } = opened.loaded;
  const [groupBy, setGroupBy] = useState<GroupBy>(readGroupBy);
  const choose = (g: GroupBy) => {
    setGroupBy(g);
    try {
      localStorage.setItem(GROUP_BY_KEY, g);
    } catch {
      /* storage unavailable: the choice lasts until the window closes */
    }
  };
  const iterations = box?.iterations ?? [];
  const iterationName = (id: string | null) => (id ? (iterations.find((i) => i.id === id)?.name ?? id) : null);

  // The groups in the order they are shown; the task pane's arrow keys follow this order.
  const groups: Group[] = [];
  if (box && groupBy === "iteration") {
    for (const it of iterations) {
      groups.push({
        key: `i-${it.id}`,
        id: it.id,
        name: it.name,
        sub: iterationSub(it, t),
        empty: t("iteration.no_tasks"),
        tasks: box.tasks.filter((task) => task.iteration === it.id),
      });
    }
    // Tasks with no iteration, or one that does not exist (reported above), come last.
    const unplanned = box.tasks.filter((task) => !iterations.some((i) => i.id === task.iteration));
    if (unplanned.length > 0) groups.push({ key: "unplanned", id: "", name: t("box.unplanned"), empty: "", tasks: unplanned });
  } else if (box) {
    for (const area of box.areas) {
      groups.push({
        key: `a-${area.id}`,
        id: area.id,
        name: area.name,
        empty: t("area.no_tasks"),
        tasks: box.tasks.filter((task) => task.area === area.id),
      });
    }
    const lost = box.tasks.filter((task) => !box.areas.find((a) => a.id === task.area));
    if (lost.length > 0) groups.push({ key: "lost", id: "?", name: t("box.unknown_area"), empty: "", tasks: lost });
  }
  const shown = groups.flatMap((g) => g.tasks);

  // The task shown in the pane, by ID, so it stays selected when the seabag reloads.
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = box?.tasks.find((task) => task.id === selectedId) ?? null;
  const shownRef = useRef(shown);
  shownRef.current = shown;
  // Up and down move through the tasks as listed; Esc closes the pane.
  useEffect(() => {
    if (!selectedId) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, select, textarea") || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "Escape") {
        setSelectedId(null);
        return;
      }
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      const list = shownRef.current;
      const at = list.findIndex((task) => task.id === selectedId);
      const next = list[Math.max(0, Math.min(list.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)))];
      if (next) {
        e.preventDefault();
        setSelectedId(next.id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedId]);
  useEffect(() => {
    if (selectedId) document.getElementById(`task-${selectedId}`)?.scrollIntoView({ block: "nearest" });
  }, [selectedId, groupBy]);

  const openLink = (href: string) => {
    if (selected) messenger.openTaskLink(opened.root, selected.id, href).catch(onError);
  };

  return (
    <>
      {issues.length > 0 && <Issues issues={issues} />}
      {box ? (
        <>
          {box.tasks.length > 0 && (
            <div className="group-by" role="group" aria-label={t("group.label")}>
              {(["area", "iteration"] as const).map((g) => (
                <button key={g} type="button" aria-pressed={groupBy === g} onClick={() => choose(g)}>
                  {t(g === "area" ? "group.by_area" : "group.by_iteration")}
                </button>
              ))}
            </div>
          )}
          {/* The list scrolls on its own, so its scroll bar stays beside the list, not beside the pane. */}
          <div className={selected ? "list-layout with-pane" : "list-layout"}>
            <div className="areas">
              {groupBy === "iteration" && iterations.length === 0 && box.tasks.length > 0 && (
                <p className="muted small">
                  {rich("box.no_iterations", { add: <code>fh add iteration</code>, plan: <code>fh plan</code> })}
                </p>
              )}
              {groups.map((g) => (
                <TaskGroup
                  key={g.key}
                  id={g.id}
                  name={g.name}
                  sub={g.sub}
                  empty={g.empty}
                  tasks={g.tasks}
                  iterationName={groupBy === "area" ? iterationName : undefined}
                  selectedId={selectedId}
                  onSelect={(id) => setSelectedId((now) => (now === id ? null : id))}
                />
              ))}
              {groupBy === "area" && box.areas.length === 0 && box.tasks.length === 0 && (
                <p className="muted">{t("box.empty")}</p>
              )}
            </div>
            {selected && (
              <TaskPane
                task={selected}
                area={box.areas.find((a) => a.id === selected.area)}
                iteration={iterations.find((i) => i.id === selected.iteration)}
                onClose={() => setSelectedId(null)}
                onLink={openLink}
              />
            )}
          </div>
        </>
      ) : (
        <p className="muted">{t("box.no_project")}</p>
      )}
    </>
  );
}

/** An iteration's release and dates, for the line under its name. */
function iterationSub(it: Iteration, t: ReturnType<typeof useI18n>["t"]): string {
  const parts: string[] = [];
  if (it.release) parts.push(t("iteration.release", { release: it.release }));
  if (it.start && it.end) parts.push(t("iteration.dates", { start: it.start, end: it.end }));
  else if (it.start) parts.push(t("iteration.from", { start: it.start }));
  else if (it.end) parts.push(t("iteration.until", { end: it.end }));
  return parts.join(" · ");
}

function Issues({ issues }: { issues: Issue[] }) {
  const { t, rich, problem } = useI18n();
  return (
    <section className="issues" aria-label={t("issues.label")}>
      <h2>{issues.length === 1 ? t("issues.title_one") : t("issues.title_other", { count: issues.length })}</h2>
      <ul>
        {issues.map((i, n) => (
          <li key={n}>
            <code>{i.file}</code>
            {i.field && <> · {rich("issues.field", { field: <code>{i.field}</code> })}</>}
            <span>{problem(i.problem)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** One group of the task list: an area or an iteration, with its progress and its tasks. */
function TaskGroup({
  id,
  name,
  sub,
  empty,
  tasks,
  iterationName,
  selectedId,
  onSelect,
}: {
  id: string;
  name: string;
  sub?: string;
  empty: string;
  tasks: Task[];
  /** Given in the by-area view, so each task says which iteration it is planned in. */
  iterationName?: (id: string | null) => string | null;
  selectedId: string | null;
  /** Clicking a task opens it in the task pane; clicking it again closes the pane. */
  onSelect: (id: string) => void;
}) {
  const { t } = useI18n();
  const done = tasks.filter((t) => t.state === "done").length;
  const pct = tasks.length ? Math.round((done / tasks.length) * 100) : 0;
  return (
    <section className="area">
      <header className="area-head">
        <h2>
          {id && <span className="id">{id}</span>} {name}
        </h2>
        <span className="count">{t("area.count", { done, total: tasks.length })}</span>
        {sub && <span className="sub">{sub}</span>}
        <div className="meter" role="img" aria-label={t("area.meter", { pct })}>
          <div style={{ width: `${pct}%` }} />
        </div>
      </header>
      {tasks.length === 0 ? (
        <p className="muted small">{empty}</p>
      ) : (
        <ul className="tasks">
          {tasks.map((task) => {
            const iteration = iterationName?.(task.iteration);
            return (
              <li
                key={task.id}
                id={`task-${task.id}`}
                className={task.id === selectedId ? "task selected" : "task"}
                tabIndex={0}
                aria-current={task.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(task.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onSelect(task.id);
                  }
                }}
              >
                <span className="id">{task.id}</span>
                <span className="title">{task.title}</span>
                <span className={`state s-${task.state}`}>{t(`state.${task.state}`)}</span>
                <span className="meta">
                  {task.label && <span className="label">{task.label}</span>}
                  {iteration && <span>{t("task.iteration", { iteration })}</span>}
                  {task.holder && <span>{t("task.held_by", { holder: task.holder })}</span>}
                  {task.waiting_on && <span>{t("task.waiting_on", { what: task.waiting_on })}</span>}
                  {task.handoff_note && <span>{t("task.handoff_note", { note: task.handoff_note })}</span>}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
