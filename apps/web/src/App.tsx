import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { LANGUAGES, LANGUAGE_NAMES, useI18n } from "./i18n";
import type { Language, Message } from "./i18n";
import { AgentLine } from "./AgentLine";
import { AgentsPanel } from "./AgentsPanel";
import type { ConnectResult } from "./AgentsPanel";
import { ConfirmDialog } from "./ConfirmDialog";
import { SetupWizard } from "./SetupWizard";
import { newFiles, offerConnect, stillMissing } from "./setupFlow";
import { TaskPane } from "./TaskPane";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { InstructionFiles, Issue, Iteration, Opened, Problem, SetupState, Task } from "./types";

/** The file manager's name on this computer, for the button's words. */
const FILE_MANAGER_KEY = /Mac/.test(navigator.userAgent)
  ? "top.show_in_finder"
  : /Windows/.test(navigator.userAgent)
    ? "top.show_in_explorer"
    : "top.show_in_files";

const LAST_ROOT_KEY = "allomos.lastRoot";
/** Recently opened project folders, newest first, for File → Open Recent. */
const RECENT_KEY = "allomos.recentFolders";
const RECENT_MAX = 10;

/** The project folder the user picked, from a taskbag root (`<folder>/.allomos/taskbag`). */
function projectFolder(root: string): string {
  return root.replace(/[\\/]\.allomos[\\/]taskbag[\\/]?$/, "");
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
/** How often the window checks whether the taskbag's files changed, for example through `alm`. */
const FOLLOW_MS = 1000;
/** After agents are connected: how often, and for how long, the window looks for a new AGENTS.md
 * or CLAUDE.md, to say that running agent sessions have not read it (Task-A05-03). */
const WATCH_FILES_MS = 2000;
const WATCH_FILES_FOR_MS = 30 * 60 * 1000;

/** A banner's text: an interface message, or a problem sent by Rust. */
type Note = Message | { problem: Problem } | { text: string };

function errorNote(e: unknown): Note {
  return isProblem(e) ? { problem: e } : { key: "error.unexpected", params: { detail: String(e) } };
}

export function App({ messenger }: { messenger: Messenger }) {
  const { t, problem, language } = useI18n();
  const [opened, setOpened] = useState<Opened | null>(null);
  const openedRef = useRef(opened);
  openedRef.current = opened;
  /** The setup wizard (Task-A05-03), at step 1, or at step 2 for a folder without `.allomos`. */
  const [wizard, setWizard] = useState<{ folder?: string; n: number } | null>(null);
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
    setNotices(message ? [message] : []);
    setAgentsFor(null);
    setSetup(null);
    setSkipped(false);
    setError(null);
    try {
      localStorage.setItem(LAST_ROOT_KEY, o.root);
    } catch {
      /* storage unavailable: nothing to remember */
    }
    const folder = projectFolder(o.root);
    changeRecent((list) => [folder, ...list.filter((f) => f !== folder)]);
  }, [changeRecent]);

  /** The agents panel, Connect with the agents (Task-A05-03): tell AI agents about Allomos, and
   * let the first one find the project's existing tasks and offer to move them in. */
  const [agentsFor, setAgentsFor] = useState<{ project: string; taskbag: string } | null>(null);

  /** The open-folder flow (docs/product/OPEN-FOLDER-FLOW.md): the open taskbag's project folder,
   * agent instructions and tasks, and whether the person put off connecting agents this time. */
  const [setup, setSetup] = useState<SetupState | null>(null);
  const setupRef = useRef(setup);
  setupRef.current = setup;
  const [skipped, setSkipped] = useState(false);
  /** Instruction files that were missing when agents were connected: watched until they appear. */
  const [watch, setWatch] = useState<{ project: string; before: InstructionFiles; since: number } | null>(null);
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
          show(await messenger.openTaskbag(i.root), null);
        } else if (i.kind === "empty" || i.kind === "no_taskbag") {
          // No `.allomos` yet: the wizard asks, starting at step 2 with this folder.
          setWizard((w) => ({ folder: i.folder, n: (w?.n ?? 0) + 1 }));
        } else {
          setError({ key: "error.not_a_folder", params: { path: i.path } });
        }
      } catch (e) {
        setError(errorNote(e));
      }
    },
    [messenger, show],
  );

  // The rest of the flow after opening a taskbag directly (not through the wizard): the project
  // folder, then Connect with the agents; checked again whenever the taskbag's files change.
  const root0 = opened?.root;
  const taskCount = opened?.loaded.taskbag?.tasks.length;
  useEffect(() => {
    if (root0) void refreshSetup(root0);
  }, [root0, taskCount, refreshSetup]);
  useEffect(() => {
    if (!opened || !setup || agentsFor || wizard) return;
    // Without a project folder, the page asks for one first.
    if (setup.project_folder && offerConnect(setup, skipped)) {
      setAgentsFor({ project: setup.project_folder, taskbag: opened.root });
    }
  }, [opened, setup, agentsFor, skipped, wizard]);
  // File → Connect with the Agents… only while a taskbag is open.
  const hasTaskbag = Boolean(opened);
  useEffect(() => {
    void messenger.setConnectEnabled(hasTaskbag).catch(() => undefined);
  }, [messenger, hasTaskbag]);

  /** Connect with the agents closed (from the page or the wizard's last step). */
  const connected = (result: ConnectResult) => {
    // Asked or not now: not offered again until the folder is opened again.
    setSkipped(true);
    setAgentsFor(null);
    const message = result.message;
    if (message) setNotices((list) => [...list, { text: message }]);
    if (result.watch) setWatch({ ...result.watch, since: Date.now() });
  };

  // The restart notice: an AGENTS.md or CLAUDE.md that did not exist before agents were connected
  // has appeared. Agent sessions read these files when they start, so only new ones know.
  useEffect(() => {
    if (!watch) return;
    let live = true;
    const look = async () => {
      if (Date.now() - watch.since > WATCH_FILES_FOR_MS) return setWatch(null);
      const now = await messenger.instructionFiles(watch.project).catch(() => null);
      if (!live || !now) return;
      const files = newFiles(watch.before, now);
      if (files.length > 0) {
        setNotices((list) => [...list, { key: "notice.restart_agents", params: { files: files.join(", ") } }]);
        // Each file is told once; keep watching for the other while it is still missing.
        const before = { agents_md: watch.before.agents_md || now.agents_md, claude_md: watch.before.claude_md || now.claude_md };
        setWatch(stillMissing(watch.before, now) ? { ...watch, before } : null);
      }
    };
    void look();
    const timer = window.setInterval(() => void look(), WATCH_FILES_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, watch]);
  /** The project folder for the agents panel: the recorded one, else the folder holding `.allomos/`. */
  const agentsProject = (root: string) => setupRef.current?.project_folder ?? projectFolder(root);

  // Follow changes made outside the window (by `alm` or by hand): reload when the files change.
  const root = opened?.root;
  useEffect(() => {
    if (!root) return;
    let last: string | null = null;
    let busy = false;
    const check = async () => {
      if (busy) return;
      busy = true;
      try {
        const now = await messenger.taskbagStamp(root);
        if (last !== null && now !== last) {
          const fresh = await messenger.openTaskbag(root);
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

  /** With nothing open, the wizard from step 1; otherwise pick a folder and open it (the wizard
   * takes over when it has no `.allomos`). */
  const choose = async () => {
    if (!openedRef.current) {
      setWizard((w) => ({ n: (w?.n ?? 0) + 1 }));
      return;
    }
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

  // Tasks are written in the interface's language unless the taskbag says otherwise (Task-A01-21):
  // `alm task-language` reads the language saved here.
  useEffect(() => {
    void messenger.saveUiLanguage(language).catch(() => undefined);
  }, [messenger, language]);

  // The menu's words follow the interface's language.
  useEffect(() => {
    void messenger
      .setMenuTexts({
        file: t("menu.file"),
        openFolder: t("menu.open_folder"),
        openRecent: t("menu.open_recent"),
        clearMenu: t("menu.clear_recent"),
        noRecent: t("menu.no_recent"),
        connect: t("menu.connect"),
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
    // File → Connect with the Agents… (enabled while a taskbag is open).
    const stopConnect = messenger.onConnectMenu(() => {
      const root = openedRef.current?.root;
      if (root) setAgentsFor({ project: agentsProject(root), taskbag: root });
    });
    return () => {
      void stopOpen.then((f) => f()).catch(() => undefined);
      void stopClear.then((f) => f()).catch(() => undefined);
      void stopConnect.then((f) => f()).catch(() => undefined);
    };
  }, [messenger, openFolder, changeRecent]);

  return (
    <div className="app">
      <header className="top">
        <div className="brand" aria-label={t("brand.name")}>
          {/* The Allomos mark (owner's choice: layout Y2, colours D4): a ring where a ball was, a solid ball where it
              arrived, a spark for the jump. Same drawing as the app icon's small form. */}
          <svg viewBox="100 100 824 824" aria-hidden="true">
            <circle className="ring jade" cx="265" cy="512" r="105" />
            <circle className="jade" cx="759" cy="512" r="138" />
            <circle className="ring terracotta" cx="512" cy="265" r="105" />
            <circle className="terracotta" cx="512" cy="759" r="138" />
            <path className="spark" d="M512 392 L552 472 L632 512 L552 552 L512 632 L472 552 L392 512 L472 472 Z" />
          </svg>
          {/* The product's name in the interface's language: Allomos, 换肩, アロモス. */}
          <span>{t("brand.name")}</span>
        </div>
        {opened?.loaded.taskbag && (
          <div className="where">
            <strong>{opened.loaded.taskbag.project.name}</strong>
            <code title={opened.root}>{opened.root}</code>
            {/* Whether the AI agents are told about this project, then Show in Finder at the right (owner, 2026-10-10). */}
            <div className="where-row">
              <AgentLine
                messenger={messenger}
                root={opened.root}
                onTell={() => setAgentsFor({ project: agentsProject(opened.root), taskbag: opened.root })}
              />
              <button
                type="button"
                className="link reveal"
                onClick={() => messenger.showInFileManager(opened.root).catch((e) => setError(errorNote(e)))}
              >
                {t(FILE_MANAGER_KEY)}
              </button>
            </div>
          </div>
        )}
        <LanguagePicker />
        {/* With nothing open, the welcome page's Start button is the only way in. */}
        {opened && (
          <button type="button" className="primary" onClick={choose}>
            {t("top.open_another")}
          </button>
        )}
      </header>

      <main className="main">
        {error && (
          <Banner kind="error" onClose={() => setError(null)}>
            {noteText(error)}
          </Banner>
        )}
        {notices.map((n, i) => (
          <Banner key={i} kind="ok" onClose={() => setNotices((list) => list.filter((_, j) => j !== i))}>
            {noteText(n)}
          </Banner>
        ))}
        {agentsFor && (
          <AgentsPanel
            key={agentsFor.taskbag}
            messenger={messenger}
            project={agentsFor.project}
            taskbag={agentsFor.taskbag}
            onProjectChange={async (folder) => {
              const state = await messenger.setProjectFolder(agentsFor.taskbag, folder);
              setSetup(state);
            }}
            onDone={connected}
          />
        )}

        {opened && setup && !setup.project_folder && !wizard && (
          <ProjectFolderStep
            allomosFolder={setup.allomos_folder}
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

        {!opened && <Welcome onOpen={() => setWizard((w) => ({ n: (w?.n ?? 0) + 1 }))} />}

        {wizard && (
          <SetupWizard
            key={wizard.n}
            messenger={messenger}
            folder={wizard.folder}
            onOpened={(o, created) =>
              show(o, created ? { key: o.readme_added ? "notice.created_here_readme" : "notice.created_nested" } : null)
            }
            onSetup={setSetup}
            onClose={(result) => {
              setWizard(null);
              if (result) connected(result);
              // Read again: the wizard may have recorded the project folder.
              const root = openedRef.current?.root;
              if (root) void refreshSetup(root);
            }}
          />
        )}

        {/* Questions an agent asks with `alm confirm` about the folder this window shows (Task-A05-02). */}
        {/* `alm confirm` asks about the folder holding `.allomos/`, where its --taskbag points. */}
        {opened && (
          <ConfirmDialog
            messenger={messenger}
            project={projectFolder(opened.root)}
            onCancelled={() => setNotices((list) => [...list, { key: "confirm.cancelled" }])}
          />
        )}

        {opened && <TaskbagView opened={opened} messenger={messenger} onError={(e) => setError(errorNote(e))} />}
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

/** Step 2 of the flow: no project folder is recorded and none was found beside `.allomos/`. */
function ProjectFolderStep({
  allomosFolder,
  onChoose,
  pick,
}: {
  allomosFolder: string;
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
      <p>{rich("folder.body", { folder: <code>{allomosFolder}</code> })}</p>
      <div className="row">
        <button type="button" className="primary" disabled={busy} onClick={() => void choose(allomosFolder)}>
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

/** The first screen with nothing open: the road show's opening slide (slogan, a task shifting
 * between agents and a teammate around the taskbag), with one button, Start, which opens the setup
 * wizard (Task-A05-03). */
function Welcome({ onOpen }: { onOpen: () => void }) {
  const { t } = useI18n();
  return (
    <section className="welcome">
      <div className="welcome-grid">
        <div className="welcome-text">
          <p className="eyebrow">{t("welcome.eyebrow", { name: t("brand.name") })}</p>
          {/* Three phrases: Chinese and Japanese break only between them (Safari's engine, which
              draws the macOS window, will not break a run of CJK text marked keep-all at all). */}
          <h1>
            <span className="phrase">{t("welcome.title1")}</span>
            <span className="phrase">{t("welcome.title2")}</span>
            <em className="phrase">{t("welcome.keep")}</em>
          </h1>
          <p className="lede">{t("welcome.lede")}</p>
          <div className="welcome-start">
            <button type="button" className="primary start" onClick={onOpen}>
              {t("welcome.open")}
            </button>
            <span className="muted">{t("welcome.hint")}</span>
          </div>
        </div>
        <HopFigure />
      </div>
    </section>
  );
}

/** The road show's drawing: a task shifts from one agent to another when a quota runs out, then to
 * a teammate; all of them use the same taskbag. */
function HopFigure() {
  const { t } = useI18n();
  return (
    <figure className="hop-figure">
      <svg className="hop" viewBox="0 0 560 330" role="img" aria-labelledby="hop-title">
        <title id="hop-title">{t("fig.title")}</title>
        <defs>
          <marker id="hop-head" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto">
            <path className="head" d="M0,0 L10,5 L0,10 z" />
          </marker>
        </defs>
        <path className="arc" d="M120,92 Q187,14 250,92" markerEnd="url(#hop-head)" />
        <path className="arc" d="M310,92 Q373,14 440,92" markerEnd="url(#hop-head)" />
        <text className="why" x="185" y="40" textAnchor="middle">{t("fig.quota")}</text>
        <text className="why" x="375" y="40" textAnchor="middle">{t("fig.handoff")}</text>
        <line className="tie" x1="95" y1="148" x2="246" y2="242" />
        <line className="tie" x1="280" y1="148" x2="280" y2="188" />
        <line className="tie" x1="465" y1="148" x2="314" y2="242" />
        <rect className="who" x="20" y="96" width="150" height="52" rx="10" />
        <rect className="who" x="205" y="96" width="150" height="52" rx="10" />
        <rect className="who person" x="390" y="96" width="150" height="52" rx="10" />
        <text className="name" x="95" y="119" textAnchor="middle">Claude Code</text>
        <text className="sub" x="95" y="137" textAnchor="middle">{t("fig.account", { n: 1 })}</text>
        <text className="name" x="280" y="119" textAnchor="middle">Codex</text>
        <text className="sub" x="280" y="137" textAnchor="middle">{t("fig.account", { n: 2 })}</text>
        <text className="name" x="465" y="119" textAnchor="middle">{t("fig.teammate")}</text>
        <text className="sub" x="465" y="137" textAnchor="middle">{t("fig.their_agents")}</text>
        <path className="bag" d="M246,226 Q240,262 241,300 C241,320 319,320 319,300 Q320,262 314,226 Q300,218 292,207 L268,207 Q260,218 246,226 Z" />
        <path className="bag" d="M268,207 Q260,192 273,190 Q280,197 287,190 Q300,192 292,207 Z" />
        <path className="seam" d="M241,300 C241,310 319,310 319,300" />
        <path className="cord" d="M252,224 C246,252 246,282 252,310" />
        <path className="cord" d="M265,208 Q280,214 295,208" />
        <path className="cord" d="M293,210 Q301,220 298,231" />
        <rect className="toggle" x="294" y="230" width="8" height="12" rx="3" />
        <text className="stencil" x="283" y="268" textAnchor="middle">{t("fig.taskbag")}</text>
        <text className="bagp" x="330" y="268">.allomos/taskbag/</text>
        <text className="bagp faint" x="330" y="285">{t("fig.in_repo")}</text>
      </svg>
    </figure>
  );
}

/** How the task list is grouped; remembered in the window's storage (Task-A01-15). */
type GroupBy = "area" | "iteration";
const GROUP_BY_KEY = "allomos.groupBy";

function readGroupBy(): GroupBy {
  try {
    return localStorage.getItem(GROUP_BY_KEY) === "iteration" ? "iteration" : "area";
  } catch {
    return "area";
  }
}

/** One group of the list as shown: an area or an iteration with its tasks. */
type Group = { key: string; id: string; name: string; sub?: string; empty: string; tasks: Task[] };

function TaskbagView({
  opened,
  messenger,
  onError,
}: {
  opened: Opened;
  messenger: Messenger;
  onError: (e: unknown) => void;
}) {
  const { t, rich } = useI18n();
  const { taskbag: box, issues } = opened.loaded;
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

  // The task shown in the pane, by ID, so it stays selected when the taskbag reloads.
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
                  {rich("box.no_iterations", { add: <code>alm add iteration</code>, plan: <code>alm plan</code> })}
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
