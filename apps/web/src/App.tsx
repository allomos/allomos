import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode, RefObject } from "react";
import { LANGUAGES, LANGUAGE_NAMES, useI18n } from "./i18n";
import type { Language, Message } from "./i18n";
import { AgentLine } from "./AgentLine";
import type { ConnectResult } from "./AgentsPanel";
import { ConfirmDialog } from "./ConfirmDialog";
import { ProgressDialog } from "./ProgressDialog";
import { SetupWizard } from "./SetupWizard";
import { BuildAppDialog } from "./BuildAppDialog";
import { MoveCardsDialog } from "./MoveCardsDialog";
import { cardRows, cardsNotice } from "./moveCardsFlow";
import { directStep, neverConnected, newFiles, stillMissing } from "./setupFlow";
import { updateNotice } from "./versionFlow";
import { almDismissFor, almOffer } from "./almOfferFlow";
import { UPDATE_LOOK_MS, updateNote } from "./updateFlow";
import { HandoffForm, HandoffSettingsDialog, HandoffsView } from "./Handoffs";
import { latestFor } from "./handoffFlow";
import { setKeys, toggleKey, visibleItems, withKey, withoutKey } from "./foldFlow";
import { collapsedWhileSearching, handoffMatches, searchGroups, searchTerms, taskMatches } from "./searchFlow";
import { DescriptionHit, HandoffHits, Highlight, ListBanners, RefreshLine, SearchBox, isSearchShortcut } from "./Search";
import { whatChanged, type Refreshed } from "./refreshFlow";
import {
  FILTER_STATES,
  filterGroups,
  hiddenByFilter,
  hiddenText,
  readFilter,
  stopText,
  taskListBanners,
  toCheck,
  toReview,
  type Hidden,
  type StateFilter,
} from "./stopFlow";
import { useFold } from "./useFold";
import { afterClick } from "./paneFlow";
import { projectFolder } from "./paths";
import { RecentDialog } from "./RecentDialog";
import { keepListed, noteOpened, parseRecentInfo, type RecentInfoMap } from "./recentFlow";
import { AiDecidedBadge, HumanCheckBadge, TaskPane } from "./TaskPane";
import { CreateForm, type AddRequest } from "./CreateForm";
import { baseChoice, type Creating } from "./createFlow";
import { GroupEditForm } from "./GroupEditForm";
import { toIterationChange, type Editing, type GroupDraft } from "./groupEditFlow";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { AlmStatus, UpdateStatus, Edit, InstructionFiles, Issue, Iteration, NewIteration, NextIds, Opened, Problem, SetupState, Task, TaskHistory, VersionCheck, Handoff } from "./types";

/** The file manager's name on this computer, for the button's words. */
const FILE_MANAGER_KEY = /Mac/.test(navigator.userAgent)
  ? "top.show_in_finder"
  : /Windows/.test(navigator.userAgent)
    ? "top.show_in_explorer"
    : "top.show_in_files";

const LAST_ROOT_KEY = "allomos.lastRoot";
/** This window was opened with New window… (Task-A01-27): it starts on the start page. */
const NEW_WINDOW = new URLSearchParams(window.location.search).has("new");
/** Recently opened project folders, newest first, for File → Open Recent. */
const RECENT_KEY = "allomos.recentFolders";
const RECENT_MAX = 10;

function readRecent(): string[] {
  try {
    const list: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    return Array.isArray(list) ? list.filter((x): x is string => typeof x === "string").slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

/** Each recent folder's project name and when it was last opened, for the start page's Open recent
 * dialog (Task-A01-43). Kept only for folders still in the list. */
const RECENT_INFO_KEY = "allomos.recentInfo";

function readRecentInfo(): RecentInfoMap {
  try {
    return parseRecentInfo(localStorage.getItem(RECENT_INFO_KEY));
  } catch {
    return {};
  }
}

function saveRecentInfo(info: RecentInfoMap) {
  try {
    localStorage.setItem(RECENT_INFO_KEY, JSON.stringify(info));
  } catch {
    /* storage unavailable: the dialog shows folder names only */
  }
}

function saveRecent(list: string[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    /* storage unavailable: the menu still shows this session's list */
  }
  saveRecentInfo(keepListed(readRecentInfo(), list));
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
  /** The setup wizard (Task-A05-03): at step 1, at step 2 for a folder without `.allomos`, or at
   * step 3 or 4 for the open taskbag. `n` counts openings, so each starts fresh. */
  const [wizard, setWizard] = useState<{ folder?: string; resume?: { root: string; at: 3 | 4 }; n: number } | null>(
    null,
  );
  const openWizard = useCallback(
    (how: { folder?: string; resume?: { root: string; at: 3 | 4 } }) => setWizard((w) => ({ ...how, n: (w?.n ?? 0) + 1 })),
    [],
  );
  const [notices, setNotices] = useState<Note[]>([]);
  /** The start page's Build a new app… dialog (Task-A03-25-02). */
  const [building, setBuilding] = useState(false);
  /** Update the taskbag… is open (Task-A00-30): the notice's button or File → Update the Taskbag…. */
  const [movingCards, setMovingCards] = useState(false);
  // The update notice says the taskbag is from an older Allomos; the problem list then leaves it out.
  const [olderShown, setOlderShown] = useState(false);
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

  /** Back to the start page: no taskbag open, and none to reopen at the next start. */
  const closeTaskbag = () => {
    void messenger.setWindowProject(null, null).catch(() => undefined);
    setOpened(null);
    setSetup(null);
    setNotices([]);
    setError(null);
    try {
      localStorage.removeItem(LAST_ROOT_KEY);
    } catch {
      /* storage unavailable: nothing to forget */
    }
  };

  /** Counts openings, so opening the same folder again reads its setup again (the wizard may be due). */
  const [openings, setOpenings] = useState(0);
  // When the list was last read and how (Task-A01-47): opened, automatically, or by Refresh now.
  const [refreshed, setRefreshed] = useState<Refreshed | null>(null);
  // The taskbag's stamp at the last read, shared by the once-a-second follow and the window's own
  // reads, so a read the window made itself is not read again and reported as automatic.
  const stampRef = useRef<string | null>(null);
  const show = useCallback((o: Opened, message: Note | null) => {
    setOpened(o);
    setRefreshed({ how: "opened", at: new Date() });
    setOpenings((n) => n + 1);
    setNotices(message ? [message] : []);
    setSetup(null);
    setSkipped(false);
    setError(null);
    try {
      localStorage.setItem(LAST_ROOT_KEY, o.root);
    } catch {
      /* storage unavailable: nothing to remember */
    }
    const folder = projectFolder(o.root);
    saveRecentInfo(noteOpened(readRecentInfo(), folder, o.loaded.taskbag?.project.name ?? null, new Date()));
    changeRecent((list) => [folder, ...list.filter((f) => f !== folder)]);
    // The app knows which window shows which project, and the window is named after it (Task-A01-27).
    void messenger.setWindowProject(o.root, o.loaded.taskbag?.project.name ?? null).catch(() => undefined);
  }, [changeRecent, messenger]);

  /** The open-folder flow (docs/product/OPEN-FOLDER-FLOW.md): the open taskbag's project folder,
   * agent instructions and tasks, and whether the person put off connecting agents this time. */
  const [setup, setSetup] = useState<SetupState | null>(null);
  const [skipped, setSkipped] = useState(false);
  /** Instruction files that were missing when agents were connected: watched until they appear. */
  const [watch, setWatch] = useState<{ project: string; before: InstructionFiles; since: number } | null>(null);
  const refreshSetup = useCallback(
    async (root: string) => {
      try {
        const state = await messenger.projectSetup(root);
        if (openedRef.current?.root === root) setSetup(state);
      } catch (e) {
        // A newer format is already in the problem list above the tasks (Task-A01-25); an older
        // Allomos's taskbag in the notice offering to move its tasks in (Task-A01-45).
        if (isProblem(e) && ["taskbag_newer", "taskbag_to_move_in"].includes(e.code)) return;
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
          // Already open in another window: that window comes forward instead (owner, 2026-10-12).
          if (await messenger.showProjectElsewhere(i.root).catch(() => false)) return;
          show(await messenger.openTaskbag(i.root), null);
        } else if (i.kind === "empty" || i.kind === "no_taskbag") {
          // No `.allomos` yet: the wizard asks, starting at step 2 with this folder.
          openWizard({ folder: i.folder });
        } else {
          setError({ key: "error.not_a_folder", params: { path: i.path } });
        }
      } catch (e) {
        setError(errorNote(e));
      }
    },
    [messenger, show, openWizard],
  );

  // The rest of the flow after opening a taskbag directly (not through the wizard): the wizard opens
  // at the step that is missing, step 3 (project folder) or 4 (connect with the agents); checked
  // again whenever the taskbag's files change (owner, 2026-10-12: no setup cards on the page).
  const root0 = opened?.root;
  const taskCount = opened?.loaded.taskbag?.tasks.length;
  useEffect(() => {
    if (root0) void refreshSetup(root0);
  }, [root0, taskCount, openings, refreshSetup]);
  useEffect(() => {
    if (!opened || !setup || wizard) return;
    const at = directStep(setup, skipped);
    if (at) openWizard({ resume: { root: opened.root, at } });
  }, [opened, setup, skipped, wizard, openWizard]);
  // File → Connect with the Agents… only while a taskbag is open.
  const hasTaskbag = Boolean(opened);
  useEffect(() => {
    hasTaskbagRef.current = hasTaskbag;
    void messenger.setConnectEnabled(hasTaskbag).catch(() => undefined);
  }, [messenger, hasTaskbag]);

  /** Connect with the agents closed (the wizard's last step). */
  const connected = (result: ConnectResult) => {
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

  // Follow changes made outside the window (by `alm` or by hand): reload when the files change.
  const root = opened?.root;
  useEffect(() => {
    if (!root) return;
    stampRef.current = null;
    let busy = false;
    const check = async () => {
      if (busy) return;
      busy = true;
      try {
        const now = await messenger.taskbagStamp(root);
        if (stampRef.current !== null && now !== stampRef.current) {
          const fresh = await messenger.openTaskbag(root);
          if (openedRef.current?.root === root) {
            setOpened(fresh);
            // "Automatically" only when the check found a change and read again (Task-A01-47).
            setRefreshed({ how: "auto", at: new Date() });
          }
        }
        stampRef.current = now;
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

  // On start: a folder given on the command line wins; otherwise reopen the last one. A window
  // from New window… starts on the start page (Task-A01-27).
  useEffect(() => {
    if (NEW_WINDOW) return;
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
        // The remembered folder was deleted or moved: forget it quietly and show the start page
        // (Task-A01-28). Its project folder leaves Open Recent too, if that is gone as well.
        if (start && (await messenger.inspectFolder(start).catch(() => null))?.inspection.kind === "not_a_folder") {
          try {
            localStorage.removeItem(LAST_ROOT_KEY);
          } catch {
            /* storage unavailable: nothing to forget */
          }
          const folder = projectFolder(start);
          if ((await messenger.inspectFolder(folder).catch(() => null))?.inspection.kind === "not_a_folder") {
            changeRecent((list) => list.filter((f) => f !== folder));
          }
          return;
        }
      }
      if (start) await openFolder(start);
    })();
  }, [messenger, openFolder, changeRecent]);

  // The menu is shared by every window: the window in front sets Connect with the Agents… and
  // Open Recent (which other windows may have changed) whenever it comes forward.
  const hasTaskbagRef = useRef(false);
  useEffect(() => {
    const stop = messenger.onWindowFocus(() => {
      void messenger.setConnectEnabled(hasTaskbagRef.current).catch(() => undefined);
      setRecent(readRecent());
    });
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
        newWindow: t("menu.new_window"),
        openRecent: t("menu.open_recent"),
        clearMenu: t("menu.clear_recent"),
        noRecent: t("menu.no_recent"),
        connect: t("menu.connect"),
        moveCards: t("menu.move_cards"),
      })
      .catch(() => undefined);
  }, [messenger, t]);

  // File → Open Recent: show the list; open a chosen folder (a folder that is gone leaves the
  // list and shows the usual message); Clear Menu empties it.
  useEffect(() => {
    void messenger.setRecentFolders(recent).catch(() => undefined);
  }, [messenger, recent]);
  const openRecent = useCallback(
    async (folder: string) => {
      const info = await messenger.inspectFolder(folder).catch(() => null);
      if (info?.inspection.kind === "not_a_folder") changeRecent((list) => list.filter((f) => f !== folder));
      await openFolder(folder);
    },
    [messenger, openFolder, changeRecent],
  );
  useEffect(() => {
    const stopOpen = messenger.onOpenRecentMenu((folder) => void openRecent(folder));
    const stopClear = messenger.onClearRecentMenu(() => changeRecent(() => []));
    // File → Connect with the Agents… (enabled while a taskbag is open): the wizard at step 4, or
    // step 3 while no project folder is known.
    const stopConnect = messenger.onConnectMenu(() => {
      const root = openedRef.current?.root;
      if (root) openWizard({ resume: { root, at: 4 } });
    });
    // File → Update the Taskbag… (enabled while a taskbag is open, Task-A00-30).
    const stopCards = messenger.onMoveCardsMenu(() => {
      if (openedRef.current) setMovingCards(true);
    });
    return () => {
      void stopOpen.then((f) => f()).catch(() => undefined);
      void stopClear.then((f) => f()).catch(() => undefined);
      void stopConnect.then((f) => f()).catch(() => undefined);
      void stopCards.then((f) => f()).catch(() => undefined);
    };
  }, [messenger, openRecent, changeRecent, openWizard]);

  // The start page's Open recent… dialog (Task-A01-43): the same list as File → Open Recent, read
  // afresh when it opens (another window may have changed it).
  const [recentDialog, setRecentDialog] = useState<RecentInfoMap | null>(null);
  const recentButton = useRef<HTMLButtonElement>(null);
  const showRecentDialog = () => {
    setRecent(readRecent());
    setRecentDialog(readRecentInfo());
  };
  const closeRecentDialog = useCallback(() => {
    setRecentDialog(null);
    recentButton.current?.focus();
  }, []);
  // A taskbag opened some other way (File › Open Recent) closes it.
  useEffect(() => {
    if (opened) setRecentDialog(null);
  }, [opened]);

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
          {/* The product's name in the interface's language: Allomos, 换肩, 肩代. */}
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
                onTell={() => openWizard({ resume: { root: opened.root, at: 4 } })}
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
        {/* With nothing open, the welcome page's Start button is the only way in (and Build a new
            app…, Task-A03-25-02). New window… opens
            another window on the start page (owner, 2026-10-12). */}
        {opened && (
          <button type="button" className="primary" onClick={() => void messenger.newWindow().catch((e) => setError(errorNote(e)))}>
            {t("top.new_window")}
          </button>
        )}
      </header>

      <main className="main">
        <UpdateOffer messenger={messenger} onError={(e) => setError(errorNote(e))} />
        <AlmOffer messenger={messenger} onError={(e) => setError(errorNote(e))} />
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
        {/* Out of date with this Allomos (Task-A01-25), or an older Allomos's taskbag whose tasks
            are to be moved in (Task-A01-45): an agent does it, from the Connect panel. */}
        {opened && (
          <VersionNotice
            messenger={messenger}
            root={opened.root}
            onUpdate={() => openWizard({ resume: { root: opened.root, at: 4 } })}
            onMoveIn={async () => {
              // Move the tasks in…: the old folder aside, a new taskbag in its place, then the
              // Connect step, whose request asks the agent to move the tasks across.
              try {
                const fresh = await messenger.moveTaskbagAside(opened.root);
                show(fresh, null);
                openWizard({ resume: { root: fresh.root, at: 4 } });
              } catch (e) {
                setError(errorNote(e));
              }
            }}
            onOlder={setOlderShown}
          />
        )}
        {/* Card files still in the project folder (Task-A00-30): named by the tasks' Card: links
            (Task-A01-51) or reported by an agent. */}
        {opened && cardsNotice(opened) > 0 && (
          <div className="banner update" role="status">
            <span>{t("cards.notice", { count: cardsNotice(opened) })}</span>
            <button type="button" className="primary" onClick={() => setMovingCards(true)}>
              {t("cards.button")}
            </button>
          </div>
        )}
        {opened && movingCards && (
          <MoveCardsDialog
            messenger={messenger}
            root={opened.root}
            found={cardRows(opened)}
            onClose={(message) => {
              setMovingCards(false);
              if (message) setNotices((list) => [...list, { text: message }]);
            }}
          />
        )}
        {!opened && (
          <Welcome
            onOpen={() => openWizard({})}
            onBuild={() => setBuilding(true)}
            onRecent={showRecentDialog}
            recentRef={recentButton}
          />
        )}
        {building && (
          <BuildAppDialog
            messenger={messenger}
            onOpened={(o) => {
              show(o, { key: o.readme_added ? "notice.created_here_readme" : "notice.created_nested" });
              // The dialog asks the agent itself: the setup wizard does not open on its own.
              setSkipped(true);
            }}
            onSetup={setSetup}
            onClose={(message) => {
              setBuilding(false);
              if (message) setNotices((list) => [...list, { text: message }]);
            }}
          />
        )}
        {!opened && recentDialog && (
          <RecentDialog
            messenger={messenger}
            folders={recent}
            info={recentDialog}
            onOpen={(folder) => {
              setRecentDialog(null);
              void openRecent(folder);
            }}
            // Removes it from the recent list only (and so from File → Open Recent); the folder stays.
            onForget={(folder) => changeRecent((list) => list.filter((f) => f !== folder))}
            onClose={closeRecentDialog}
          />
        )}

        {wizard && (
          <SetupWizard
            key={wizard.n}
            messenger={messenger}
            folder={wizard.folder}
            resume={wizard.resume}
            onOpened={(o, created) =>
              show(o, created ? { key: o.readme_added ? "notice.created_here_readme" : "notice.created_nested" } : null)
            }
            onSetup={setSetup}
            onElsewhere={() => setWizard(null)}
            onClose={(result) => {
              setWizard(null);
              if (!result) {
                // Cancel (steps 1–3, owner 2026-10-12): back to the start page, and the next start
                // shows it too. The taskbag stays on disk.
                closeTaskbag();
                return;
              }
              // Connect or Not now: the taskbag stays open; the wizard does not open by itself
              // again until the folder is opened again.
              setSkipped(true);
              connected(result);
              const root = openedRef.current?.root;
              if (!root) return;
              void (async () => {
                // Not now while no agent was ever told about Allomos: back to the start page,
                // as Cancel (owner, 2026-10-12).
                const now = await messenger.projectSetup(root).catch(() => null);
                if (!result.watch && now && neverConnected(now)) closeTaskbag();
                // Otherwise read again: the wizard may have recorded the project folder.
                else if (now && openedRef.current?.root === root) setSetup(now);
              })();
            }}
          />
        )}

        {/* What an agent reports with `alm progress` while it works (Task-A05-08); before the
            questions, so a question shows on top. */}
        {opened && <ProgressDialog messenger={messenger} project={projectFolder(opened.root)} />}

        {/* Questions an agent asks with `alm confirm` about the folder this window shows (Task-A05-02). */}
        {/* `alm confirm` asks about the folder holding `.allomos/`, where its --taskbag points. */}
        {opened && (
          <ConfirmDialog
            messenger={messenger}
            project={projectFolder(opened.root)}
            onCancelled={() => setNotices((list) => [...list, { key: "confirm.cancelled" }])}
          />
        )}

        {opened && (
          <TaskbagView
            opened={opened}
            messenger={messenger}
            onError={(e) => setError(errorNote(e))}
            hideOlder={olderShown}
            onReload={async () => {
              // After a change made in this window: read again, and note the stamp so the follow
              // does not read it a second time and call it automatic (Task-A01-47).
              const stamp = await messenger.taskbagStamp(opened.root).catch(() => null);
              const fresh = await messenger.openTaskbag(opened.root);
              if (openedRef.current?.root !== fresh.root) return;
              setOpened(fresh);
              if (stamp !== null) stampRef.current = stamp;
            }}
            refreshed={refreshed}
            onRefresh={async () => {
              // Refresh now (Task-A01-47): always read again, then say what differs from before.
              const root = opened.root;
              const before = openedRef.current?.loaded.taskbag ?? null;
              const stamp = await messenger.taskbagStamp(root).catch(() => null);
              const fresh = await messenger.openTaskbag(root);
              if (openedRef.current?.root !== root) return;
              setOpened(fresh);
              if (stamp !== null) stampRef.current = stamp;
              setRefreshed({ how: "manual", at: new Date(), changed: whatChanged(before, fresh.loaded.taskbag) });
            }}
          />
        )}
      </main>
    </div>
  );
}

/** How often the update notice checks again: an agent's update shows without reopening. */
const VERSION_CHECK_MS = 4000;

/** "Allomos's instructions are out of date", with **Update the instructions…** (Task-A01-25), or
 * "This taskbag was written by an older Allomos", with **Move the tasks in…** (Task-A01-45: the
 * old folder is renamed aside, a new taskbag created, and an agent moves the tasks across). Asked
 * when the folder opens (every way of opening one) and every few seconds while it is open.
 * Allomos never changes the tasks or the instructions itself. */
function VersionNotice({
  messenger,
  root,
  onUpdate,
  onMoveIn,
  onOlder,
}: {
  messenger: Messenger;
  root: string;
  /** Open the Connect step (its request is the move-in or the update request). */
  onUpdate: () => void;
  /** Rename the older taskbag aside and create a new one, then open the Connect step. */
  onMoveIn: () => Promise<void>;
  /** Whether this notice says the taskbag is from an older Allomos, so the problem list does not
   * say it again. */
  onOlder: (shown: boolean) => void;
}) {
  const { t } = useI18n();
  const [check, setCheck] = useState<VersionCheck | null>(null);
  useEffect(() => {
    let live = true;
    setCheck(null);
    const look = () =>
      messenger
        .versionCheck(root)
        .then((c) => live && setCheck(c))
        .catch(() => undefined);
    void look();
    const timer = window.setInterval(() => void look(), VERSION_CHECK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, root]);
  const [busy, setBusy] = useState(false);
  const notice = updateNotice(check);
  const older = notice?.kind === "move_in";
  useEffect(() => {
    onOlder(older);
    return () => onOlder(false);
  }, [older, onOlder]);
  if (!notice) return null;
  const moveIn = () => {
    setBusy(true);
    void onMoveIn().finally(() => {
      setBusy(false);
      setCheck(null);
    });
  };
  return (
    <div className="banner update" role="status">
      <span>
        {notice.kind === "move_in"
          ? t("move.notice_old", { found: notice.found })
          : notice.kind === "moving_in"
            ? t("move.notice_waiting", { old: notice.old })
            : t("update.notice_section", { files: notice.files })}
      </span>
      <button type="button" className="primary" onClick={notice.kind === "move_in" ? moveIn : onUpdate} disabled={busy}>
        {t(notice.kind === "section" ? "update.button" : "move.button")}
      </button>
    </div>
  );
}

/** **Install the alm command** (Task-A07-12, A07-14): in a built app (Mac or Windows) whose own `alm`
 * is not the one Terminal finds. Asked once when the window opens. Nothing is installed unless the
 * person clicks; "Don't ask again" is remembered for this mismatch only (the app asks again when
 * Terminal's `alm` changes), × hides it until the next start. */
function AlmOffer({ messenger, onError }: { messenger: Messenger; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<AlmStatus | null>(null);
  const [hidden, setHidden] = useState(false);
  const [busy, setBusy] = useState(false);
  const [installed, setInstalled] = useState(false);
  useEffect(() => {
    let live = true;
    messenger
      .almCommandStatus()
      .then((s) => live && setStatus(s))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger]);
  if (installed) {
    return (
      <Banner kind="ok" onClose={() => setInstalled(false)}>
        {t("alm.installed")}
      </Banner>
    );
  }
  const offer = almOffer(status);
  if (hidden || !offer) return null;
  const install = () => {
    setBusy(true);
    messenger
      .installAlmCommand()
      .then((r) => {
        if (r.cancelled) return;
        setStatus(r.status);
        setInstalled(true);
      })
      .catch(onError)
      .finally(() => setBusy(false));
  };
  const dismiss = () => {
    void messenger.dismissAlmOffer(almDismissFor(status)).catch(() => undefined);
    setHidden(true);
  };
  return (
    <div className="banner update" role="status">
      <span>
        {offer.lines.map((l) => t(l.key, l.params)).join(" ")}
        {offer.paths.map((l) => (
          <span key={l.key} className="alm-path">
            {t(l.key, l.params)}
          </span>
        ))}
      </span>
      <button type="button" className="primary" onClick={install} disabled={busy}>
        {t(offer.button)}
      </button>
      <button type="button" onClick={dismiss} disabled={busy} title={t("alm.dismiss_hint")}>
        {t("alm.dismiss")}
      </button>
      <button type="button" className="banner-close" aria-label={t("banner.close")} title={t("banner.close")} onClick={() => setHidden(true)}>
        ×
      </button>
    </div>
  );
}

/** **A newer Allomos is available** (Task-A07-16). The app asks GitHub at start and then once a
 * day; Check for Updates… asks now and always answers. Download opens the releases page; "Skip
 * this version" is remembered; × hides this version in this window. */
function UpdateOffer({ messenger, onError }: { messenger: Messenger; onError: (e: unknown) => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [asked, setAsked] = useState(false);
  const [hidden, setHidden] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const look = (ask: boolean) =>
      messenger
        .updateStatus(ask)
        .then((s) => {
          if (!live) return;
          setStatus(s);
          setAsked(ask);
        })
        .catch(() => undefined);
    void look(false);
    const timer = window.setInterval(() => void look(false), UPDATE_LOOK_MS);
    const stopMenu = messenger.onCheckUpdatesMenu(() => void look(true));
    return () => {
      live = false;
      window.clearInterval(timer);
      void stopMenu.then((f) => f()).catch(() => undefined);
    };
  }, [messenger]);
  // The menu item: Check for Updates…, or Download Allomos <version>… once one is known.
  const available = status?.available ?? null;
  useEffect(() => {
    const text = available ? t("menu.download_update", { latest: available }) : t("menu.check_updates");
    void messenger.setUpdateItem(text, Boolean(available)).catch(() => undefined);
  }, [messenger, t, available]);
  const note = updateNote(status, asked, hidden);
  if (!note) return null;
  const close = () => {
    setAsked(false);
    if (note.kind === "offer") setHidden(note.latest);
  };
  if (note.kind === "current") {
    return (
      <Banner kind="ok" onClose={close}>
        {t("update.up_to_date", { current: note.current })}
      </Banner>
    );
  }
  if (note.kind === "failed") {
    return (
      <Banner kind="error" onClose={close}>
        {t("update.check_failed")}
      </Banner>
    );
  }
  const skip = () => {
    void messenger.skipUpdate(note.latest).catch(() => undefined);
    close();
  };
  return (
    <div className="banner update" role="status">
      <svg className="banner-icon" viewBox="0 0 20 20" aria-hidden="true">
        <path d="M10 3v10M5.5 8.5 10 13l4.5-4.5M4 16.5h12" />
      </svg>
      <span>{t("update.available", { latest: note.latest, current: note.current })}</span>
      <button type="button" className="primary" onClick={() => messenger.openDownloadPage().catch(onError)}>
        {t("update.download")}
      </button>
      <button type="button" onClick={skip}>
        {t("update.skip")}
      </button>
      <button type="button" className="banner-close" aria-label={t("banner.close")} title={t("banner.close")} onClick={close}>
        ×
      </button>
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
 * wizard (Task-A05-03). Under the card, Open recent… (Task-A01-43). */
function Welcome({
  onOpen,
  onBuild,
  onRecent,
  recentRef,
}: {
  onOpen: () => void;
  /** Build a new app… (Task-A03-25-02). */
  onBuild: () => void;
  onRecent: () => void;
  recentRef: RefObject<HTMLButtonElement | null>;
}) {
  const { t } = useI18n();
  return (
    <div className="welcome-wrap">
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
              {/* Start from a description instead (Task-A03-25-02). */}
              <button type="button" className="start build" onClick={onBuild}>
                {t("build.button")}
              </button>
            </div>
          </div>
          <HopFigure />
        </div>
      </section>
      {/* Under the card, at its left edge (owner, 2026-10-15; Task-A01-43). */}
      <button type="button" ref={recentRef} className="welcome-recent" onClick={onRecent}>
        {t("welcome.open_recent")}
      </button>
    </div>
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

/** How the task list is grouped, or the Handoffs tab after Iterations (Task-A03-06); remembered
 * in the window's storage (Task-A01-15). */
type GroupBy = "area" | "iteration" | "handoffs";
const GROUP_BY_KEY = "allomos.groupBy";
const STATE_FILTER_KEY = "allomos.stateFilter";

function readGroupBy(): GroupBy {
  try {
    const g = localStorage.getItem(GROUP_BY_KEY);
    return g === "iteration" || g === "handoffs" ? g : "area";
  } catch {
    return "area";
  }
}

/** One group of the list as shown: an area or an iteration with its tasks. */
type Group = {
  key: string;
  id: string;
  name: string;
  sub?: string;
  empty: string;
  tasks: Task[];
  /** Every task of the group when `tasks` is filtered by state. */
  counted?: Task[];
  /** Where a new task added in this group goes; none for tasks with an unknown area. */
  newTask?: { area: string | null; iteration: string | null };
  /** The area or iteration this group is, for its Edit form (Task-A01-29); none for Unplanned and unknown areas. */
  edit?: Editing;
};

function TaskbagView({
  opened,
  messenger,
  onError,
  hideOlder,
  onReload,
  refreshed,
  onRefresh,
}: {
  opened: Opened;
  messenger: Messenger;
  onError: (e: unknown) => void;
  /** The update notice already says the taskbag is from an older Allomos. */
  hideOlder: boolean;
  /** Reads the taskbag again now, after a change made in this window. */
  onReload: () => Promise<void>;
  /** When the list was last read and how (Task-A01-47). */
  refreshed: Refreshed | null;
  /** Refresh now: reads the taskbag again and reports what changed. */
  onRefresh: () => Promise<void>;
}) {
  const { t, rich } = useI18n();
  const { taskbag: box, issues } = opened.loaded;
  const shownIssues = hideOlder ? issues.filter((i) => i.problem.code !== "taskbag_to_move_in") : issues;
  const [groupBy, setGroupBy] = useState<GroupBy>(readGroupBy);
  const choose = (g: GroupBy) => {
    setGroupBy(g);
    try {
      localStorage.setItem(GROUP_BY_KEY, g);
    } catch {
      /* storage unavailable: the choice lasts until the window closes */
    }
  };
  // Handoffs (Task-A03-06), read again whenever the taskbag reloads (their files are in its stamp).
  const [handoffs, setHandoffs] = useState<Handoff[]>([]);
  useEffect(() => {
    let live = true;
    messenger
      .listHandoffs(opened.root)
      .then((list) => live && setHandoffs(list))
      .catch(() => live && setHandoffs([]));
    return () => {
      live = false;
    };
  }, [messenger, opened]);
  const [handoffTask, setHandoffTask] = useState<string | null>(null);
  // Collapsed groups and open handoffs, remembered per project (Task-A01-26).
  const [collapsed, setCollapsed] = useFold(opened.root, "groups");
  const [openHandoffs, setOpenHandoffs] = useFold(opened.root, "handoffs");
  // The handoffs the tab shows now, for Collapse all / Expand all.
  const [shownHandoffs, setShownHandoffs] = useState<string[]>([]);
  const [handoffDialog, setHandoffDialog] = useState<"settings" | "write" | null>(null);
  const iterations = box?.iterations ?? [];
  const iterationName = (id: string | null) => (id ? (iterations.find((i) => i.id === id)?.name ?? id) : null);
  // The state filter (decision 0016): waiting and blocked tasks apart, or any one state; or every
  // task that needs a person (Task-A03-18).
  const [stateFilter, setStateFilter] = useState<StateFilter>(() => {
    try {
      return readFilter(localStorage.getItem(STATE_FILTER_KEY));
    } catch {
      return "all";
    }
  });
  const chooseFilter = (f: StateFilter) => {
    setStateFilter(f);
    try {
      localStorage.setItem(STATE_FILTER_KEY, f);
    } catch {
      /* storage unavailable: the choice lasts until the window closes */
    }
  };
  const stop = (task: Task) => stopText(task, box?.tasks ?? [], t);
  // The search box (Task-A01-36): one search for the task list and the Handoffs tab, emptied when
  // another project opens. Cmd/Ctrl+F focuses it.
  const [query, setQuery] = useState("");
  const terms = searchTerms(query);
  const searching = terms.length > 0;
  const searchRef = useRef<HTMLInputElement>(null);
  useEffect(() => setQuery(""), [opened.root]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!isSearchShortcut(e)) return;
      e.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

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
        newTask: { area: null, iteration: it.id },
        edit: { kind: "iteration", id: it.id },
      });
    }
    // Tasks with no iteration, or one that does not exist (reported above), come last.
    const unplanned = box.tasks.filter((task) => !iterations.some((i) => i.id === task.iteration));
    if (unplanned.length > 0)
      groups.push({ key: "unplanned", id: "", name: t("box.unplanned"), empty: "", tasks: unplanned, newTask: { area: null, iteration: null } });
  } else if (box) {
    for (const area of box.areas) {
      groups.push({
        key: `a-${area.id}`,
        id: area.id,
        name: area.name,
        empty: t("area.no_tasks"),
        tasks: box.tasks.filter((task) => task.area === area.id),
        newTask: { area: area.id, iteration: null },
        edit: { kind: "area", id: area.id },
      });
    }
    const lost = box.tasks.filter((task) => !box.areas.find((a) => a.id === task.area));
    if (lost.length > 0) groups.push({ key: "lost", id: "?", name: t("box.unknown_area"), empty: "", tasks: lost });
  }
  // With a state filter, each group lists only the tasks in that state (its count stays whole)
  // and says how many it hides (Task-A01-42); a group with none left is not drawn, and the banner
  // under the tabs says how many were left out (Task-A01-44).
  const filtered = filterGroups(groups, stateFilter);
  // A search keeps only the tasks that match, and groups with none are left out; each group left
  // holds a match, so it opens even if it was collapsed (the collapsed list itself is kept).
  const found = searchGroups(filtered, terms);
  const folded = collapsedWhileSearching(collapsed, found, terms);
  const foundHandoffs = searching ? handoffs.filter((h) => handoffMatches(h, terms)) : [];
  // Task-A01-48: with a filter on, a search lists only the tasks passing both (the search narrows
  // `filtered`, not the whole list); tasks it finds that the filter hides are only counted.
  const matches = searching && box ? box.tasks.filter((task) => taskMatches(task, terms)).length : 0;
  // Task-A01-44, A01-48: the banners under the tabs, the search's on top and the filter's under it.
  const banners = box
    ? taskListBanners({ kind: groupBy === "iteration" ? "iteration" : "area", filter: stateFilter, query: searching ? query : "", groups, filtered, found, matches }, t)
    : { search: [], filter: [] };
  // Nothing matches the filter: only the banner shows (Task-A01-44).
  const nothingShown = !searching && stateFilter !== "all" && filtered.length === 0;
  // Tasks in collapsed groups are hidden, so the arrow keys skip them.
  const shown = visibleItems(found, folded);
  // Task-A01-47: when the list was last read and how, with Refresh now, under the banners.
  const refreshLine = refreshed && <RefreshLine refreshed={refreshed} onRefresh={() => onRefresh().catch(onError)} />;
  const foldAll = (on: boolean) => {
    if (groupBy === "handoffs") setOpenHandoffs((keys) => setKeys(keys, shownHandoffs, !on));
    else setCollapsed((keys) => setKeys(keys, found.map((g) => g.key), on));
  };

  // The task shown in the pane, by ID, so it stays selected when the taskbag reloads.
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = box?.tasks.find((task) => task.id === selectedId) ?? null;
  // The selected task's history (Task-A01-17), read again whenever the taskbag reloads. Kept with
  // its task's ID, so another task never shows it and a reload does not blank it while reading.
  const [history, setHistory] = useState<{ id: string; history: TaskHistory | null } | null>(null);
  useEffect(() => {
    if (!selectedId) return;
    let live = true;
    messenger
      .taskHistory(opened.root, selectedId)
      .then((h) => live && setHistory({ id: selectedId, history: h }))
      .catch(() => live && setHistory({ id: selectedId, history: null }));
    return () => {
      live = false;
    };
  }, [messenger, opened, selectedId]);
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

  // The person's name for the task pane's changes (Task-A01-18): asked once, kept in the app's settings.
  const [ownerName, setOwnerName] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    messenger
      .ownerName()
      .then((n) => live && setOwnerName(n))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger]);
  const saveOwnerName = async (name: string) => {
    await messenger.saveOwnerName(name);
    setOwnerName(name);
  };
  const editTask = async (id: string, edit: Edit) => {
    await messenger.editTask(opened.root, id, edit);
    await onReload().catch(() => undefined);
  };
  // The task pane's "New…" iteration (Task-A01-39): added, then the task planned in it.
  const planInNewIteration = async (id: string, iteration: NewIteration) => {
    await messenger.addIterationAndPlan(opened.root, id, iteration);
    await onReload().catch(() => undefined);
  };
  const reviewDecisions = async (id: string) => {
    await messenger.reviewAiDecisions(opened.root, id);
    await onReload().catch(() => undefined);
  };

  // New tasks, areas and iterations (Task-A01-19): one form open at a time, with the next free IDs
  // suggested, read again whenever the taskbag reloads.
  const [creating, setCreating] = useState<Creating | null>(null);
  const [nextIds, setNextIds] = useState<NextIds | null>(null);
  useEffect(() => {
    let live = true;
    messenger
      .nextIds(opened.root)
      .then((n) => live && setNextIds(n))
      .catch(() => live && setNextIds(null));
    return () => {
      live = false;
    };
  }, [messenger, opened]);
  const add = async (request: AddRequest) => {
    let shown: string | null = null;
    if (request.kind === "task") {
      shown = (await messenger.addTask(opened.root, request.task)).id;
    } else if (request.kind === "area") {
      await messenger.addArea(opened.root, request.id, request.name);
    } else {
      await messenger.addIteration(opened.root, request.iteration);
    }
    setCreating(null);
    await onReload().catch(() => undefined);
    // The new task opens in the pane, so it can be changed further right away.
    if (shown) setSelectedId(shown);
  };
  const createForm = (c: Creating) =>
    box && (
      <CreateForm
        key={c.kind === "task" ? c.group : c.kind}
        creating={c}
        areas={box.areas}
        iterations={iterations}
        next={nextIds}
        base={baseChoice(box.tasks, opened.branches)}
        ownerName={ownerName}
        onSaveName={saveOwnerName}
        onAdd={add}
        onClose={() => setCreating(null)}
      />
    );
  // Renaming, changing and deleting areas and iterations (Task-A01-29): one Edit form open at a
  // time, in its group; opening it closes a New form, and the other way round.
  const [editing, setEditing] = useState<Editing | null>(null);
  const startCreating = (c: Creating) => {
    setEditing(null);
    setCreating(c);
  };
  const startEditing = (e: Editing) => {
    setCreating(null);
    setEditing(e);
  };
  const saveGroup = async (e: Editing, draft: GroupDraft) => {
    if (e.kind === "area") await messenger.renameArea(opened.root, e.id, draft.name.trim());
    else await messenger.changeIteration(opened.root, e.id, toIterationChange(draft));
    setEditing(null);
    await onReload().catch(() => undefined);
  };
  const deleteGroup = async (e: Editing) => {
    if (e.kind === "area") await messenger.deleteArea(opened.root, e.id);
    else await messenger.deleteIteration(opened.root, e.id);
    setEditing(null);
    await onReload().catch(() => undefined);
  };
  const editForm = (e: Editing) => {
    const item = e.kind === "area" ? box?.areas.find((a) => a.id === e.id) : iterations.find((i) => i.id === e.id);
    return (
      item && (
        <GroupEditForm
          key={`${e.kind}-${e.id}`}
          editing={e}
          item={item}
          ownerName={ownerName}
          onSaveName={saveOwnerName}
          onSave={(draft) => saveGroup(e, draft)}
          onDelete={() => deleteGroup(e)}
          onClose={() => setEditing(null)}
        />
      )
    );
  };
  const editingIn = (g: Group) => (g.edit && editing?.kind === g.edit.kind && editing.id === g.edit.id ? editing : null);
  // Which group's New task form is open, if any.
  const creatingIn = (key: string) => (creating?.kind === "task" && creating.group === key ? creating : null);

  const openLink = (href: string) => {
    if (selected) messenger.openTaskLink(opened.root, selected.id, href).catch(onError);
  };

  return (
    <>
      {/* The update notice above already says the taskbag is from an older Allomos (owner, 2026-10-12: once only). */}
      {shownIssues.length > 0 && <Issues issues={shownIssues} onDownload={() => messenger.openDownloadPage().catch(onError)} />}
      {box ? (
        <>
          {box && (
            <div className="list-tools">
              <div className="group-by" role="group" aria-label={t("group.label")}>
                {(["area", "iteration", "handoffs"] as const).map((g) => (
                  <button key={g} type="button" aria-pressed={groupBy === g} onClick={() => choose(g)}>
                    {t(g === "area" ? "group.by_area" : g === "iteration" ? "group.by_iteration" : "group.handoffs")}
                  </button>
                ))}
              </div>
              {/* Task-A01-44: the Handoffs tab follows the filter too. */}
              <label className="state-filter small">
                {t("filter.label")}{" "}
                <select value={stateFilter} onChange={(e) => chooseFilter(e.target.value as StateFilter)}>
                  <option value="all">{t("filter.all")}</option>
                  {/* Task-A03-18: what `alm human` lists, to check and to answer. */}
                  <option value="human">{t("filter.human")}</option>
                  {FILTER_STATES.map((s) => (
                    <option key={s} value={s}>
                      {t(`state.${s}`)}
                    </option>
                  ))}
                </select>
              </label>
              <SearchBox ref={searchRef} value={query} onChange={setQuery} />
              {/* Task-A01-26: for the list shown now. */}
              <span className="fold-all">
                <button type="button" className="link small" onClick={() => foldAll(true)}>
                  {t("fold.collapse_all")}
                </button>
                <button type="button" className="link small" onClick={() => foldAll(false)}>
                  {t("fold.expand_all")}
                </button>
              </span>
            </div>
          )}
          {/* The list scrolls on its own, so its scroll bar stays beside the list, not beside the pane. */}
          <div className={selected ? "list-layout with-pane" : "list-layout"}>
            {groupBy === "handoffs" ? (
              <HandoffsView
                handoffs={handoffs}
                terms={terms}
                tasks={box.tasks}
                stateFilter={stateFilter}
                onShowAll={() => chooseFilter("all")}
                task={handoffTask}
                onTask={setHandoffTask}
                onOpenTask={(id) => {
                  choose("area");
                  // Its area may be collapsed: open it, so the task is seen in the list.
                  const task = box.tasks.find((x) => x.id === id);
                  if (task) {
                    const key = box.areas.some((a) => a.id === task.area) ? `a-${task.area}` : "lost";
                    setCollapsed((keys) => withoutKey(keys, key));
                  }
                  setSelectedId(id);
                }}
                openKeys={openHandoffs}
                onToggle={(file) => setOpenHandoffs((keys) => toggleKey(keys, file))}
                onShown={setShownHandoffs}
                onWrite={() => setHandoffDialog("write")}
                onSettings={() => setHandoffDialog("settings")}
                onLink={openLink}
                refreshLine={refreshLine}
              />
            ) : (
            <div className="areas">
              <ListBanners search={banners.search} filter={banners.filter} onShowAll={() => chooseFilter("all")} />
              {refreshLine}
              {!searching && !nothingShown && groupBy === "iteration" && iterations.length === 0 && box.tasks.length > 0 && (
                <p className="muted small">
                  {rich("box.no_iterations", { add: <code>alm add iteration</code>, plan: <code>alm plan</code> })}
                </p>
              )}
              {found.map((g) => (
                <TaskGroup
                  key={g.key}
                  id={g.id}
                  name={g.name}
                  sub={g.sub}
                  empty={g.empty}
                  tasks={g.tasks}
                  counted={g.counted}
                  hidden={!searching && stateFilter !== "all" ? hiddenByFilter(stateFilter, g.counted ?? []) : undefined}
                  onShowAll={() => chooseFilter("all")}
                  stop={stop}
                  iterationName={groupBy === "area" ? iterationName : undefined}
                  selectedId={selectedId}
                  onSelect={(id) => setSelectedId((now) => afterClick(now, id))}
                  terms={terms}
                  collapsed={folded.includes(g.key)}
                  onToggle={() => setCollapsed((keys) => toggleKey(keys, g.key))}
                  onNewTask={
                    g.newTask && box.areas.length > 0
                      ? () => startCreating({ kind: "task", group: g.key, area: g.newTask!.area, iteration: g.newTask!.iteration })
                      : undefined
                  }
                  form={creatingIn(g.key) && createForm(creatingIn(g.key)!)}
                  onEdit={g.edit && (() => startEditing(g.edit!))}
                  editForm={editingIn(g) && editForm(editingIn(g)!)}
                />
              ))}
              {!searching && groupBy === "area" && box.areas.length === 0 && box.tasks.length === 0 && (
                <p className="muted">{t("box.empty")}</p>
              )}
              {/* Task-A01-36: handoffs the search found, after the tasks; or that nothing was found. */}
              {searching && (
                <HandoffHits
                  handoffs={foundHandoffs}
                  terms={terms}
                  onOpen={(file) => {
                    setHandoffTask(null);
                    setOpenHandoffs((keys) => withKey(keys, file));
                    choose("handoffs");
                  }}
                />
              )}
              {/* Matches the filter hides are not "nothing": the filter banner says so (Task-A01-48). */}
              {searching && matches === 0 && foundHandoffs.length === 0 && (
                <p className="muted search-none" role="status">
                  {t("search.nothing", { query: query.trim() })}
                </p>
              )}
              {/* Task-A01-19: a new area at the end of the by-area list, a new iteration at the end of the by-iteration list. */}
              {searching || nothingShown ? null : creating && creating.kind !== "task" && creating.kind === groupBy ? (
                createForm(creating)
              ) : (
                <p className="add-group">
                  <button type="button" className="link small" onClick={() => startCreating({ kind: groupBy === "iteration" ? "iteration" : "area" })}>
                    {t(groupBy === "iteration" ? "create.new_iteration_button" : "create.new_area_button")}
                  </button>
                </p>
              )}
            </div>
            )}
            {selected && (
              <TaskPane
                task={selected}
                tasks={box.tasks}
                area={box.areas.find((a) => a.id === selected.area)}
                iteration={iterations.find((i) => i.id === selected.iteration)}
                onClose={() => setSelectedId(null)}
                onLink={openLink}
                latest={latestFor(handoffs, selected.id)}
                history={history?.id === selected.id ? history.history : null}
                labels={box.project.labels}
                iterations={iterations}
                ownerName={ownerName}
                onSaveName={saveOwnerName}
                onEdit={(edit) => editTask(selected.id, edit)}
                onReview={() => reviewDecisions(selected.id)}
                nextIterationId={nextIds?.iteration ?? null}
                onNewIteration={(iteration) => planInNewIteration(selected.id, iteration)}
                onAllHandoffs={() => {
                  setHandoffTask(selected.id);
                  choose("handoffs");
                }}
              />
            )}
            {handoffDialog === "settings" && (
              <HandoffSettingsDialog messenger={messenger} root={opened.root} onClose={() => setHandoffDialog(null)} />
            )}
            {handoffDialog === "write" && (
              <HandoffForm
                messenger={messenger}
                root={opened.root}
                tasks={box.tasks}
                task={handoffTask ?? selectedId}
                onClose={() => setHandoffDialog(null)}
              />
            )}
          </div>
        </>
      ) : (
        <p className="muted">
          {t(
            issues.some((i) => i.problem.code === "taskbag_to_move_in")
              ? "box.move_in"
              : issues.some((i) => i.problem.code === "taskbag_newer")
                ? "box.needs_newer_app"
                : "box.no_project",
          )}
        </p>
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

function Issues({ issues, onDownload }: { issues: Issue[]; onDownload: () => void }) {
  const { t, rich, problem } = useI18n();
  // A taskbag from a newer Allomos is not a broken file: same look, but titled "Please update
  // Allomos" and without the file and field (owner, 2026-10-12).
  const newer = issues.some((i) => i.problem.code === "taskbag_newer");
  return (
    <section className="issues" aria-label={t("issues.label")}>
      <h2>
        {newer
          ? t("update.newer_title")
          : issues.length === 1
            ? t("issues.title_one")
            : t("issues.title_other", { count: issues.length })}
      </h2>
      <ul>
        {issues.map((i, n) => (
          <li key={n}>
            {i.problem.code !== "taskbag_newer" && (
              <>
                <code>{i.file}</code>
                {i.field && <> · {rich("issues.field", { field: <code>{i.field}</code> })}</>}
              </>
            )}
            <span>{problem(i.problem)}</span>
            {/* A newer taskbag: where to get the newer Allomos (owner, 2026-10-12). */}
            {i.problem.code === "taskbag_newer" && (
              <button type="button" className="link" onClick={onDownload}>
                {t("update.download")}
              </button>
            )}
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
  counted,
  hidden,
  onShowAll,
  stop,
  iterationName,
  selectedId,
  onSelect,
  terms,
  collapsed,
  onToggle,
  onNewTask,
  form,
  onEdit,
  editForm,
}: {
  id: string;
  name: string;
  sub?: string;
  empty: string;
  tasks: Task[];
  /** Every task of the group, for its count and progress, when `tasks` is filtered by state. */
  counted?: Task[];
  /** What the state filter hides in this group (Task-A01-42); not given without a filter or while searching. */
  hidden?: Hidden;
  /** Turns the state filter off. */
  onShowAll?: () => void;
  /** Why a stopped task is stopped: "Waiting for you: …", "Blocked by A01-17 (in progress)". */
  stop?: (task: Task) => string | null;
  /** Given in the by-area view, so each task says which iteration it is planned in. */
  iterationName?: (id: string | null) => string | null;
  selectedId: string | null;
  /** Clicking a task opens it in the task pane; clicking it again closes the pane. */
  onSelect: (id: string) => void;
  /** The search's words, marked in each task (Task-A01-36); empty when not searching. */
  terms: string[];
  /** Only the header shows: name, count and progress (Task-A01-26). */
  collapsed: boolean;
  onToggle: () => void;
  /** Opens the New task form in this group (Task-A01-19); not given where no task can be added. */
  onNewTask?: () => void;
  /** The New task form, when it is open in this group. */
  form?: ReactNode;
  /** Opens this area's or iteration's Edit form (Task-A01-29); not given for Unplanned or an unknown area. */
  onEdit?: () => void;
  /** The Edit form, when it is open in this group. */
  editForm?: ReactNode;
}) {
  const { t } = useI18n();
  const all = counted ?? tasks;
  const done = all.filter((t) => t.state === "done").length;
  const pct = all.length ? Math.round((done / all.length) * 100) : 0;
  return (
    <section className={collapsed ? "area collapsed" : "area"}>
      <header className="area-head">
        <h2>
          <button type="button" className="fold" aria-expanded={!collapsed} onClick={onToggle}>
            {id && <span className="id">{id}</span>} {name}
          </button>
        </h2>
        <span className="count">{t("area.count", { done, total: all.length })}</span>
        {sub && <span className="sub">{sub}</span>}
        <div className="meter" role="img" aria-label={t("area.meter", { pct })}>
          <div style={{ width: `${pct}%` }} />
        </div>
        {((onNewTask && !form) || (onEdit && !editForm)) && (
          <span className="group-actions">
            {onNewTask && !form && (
              <button type="button" className="link small new-task" onClick={onNewTask} aria-label={t("create.new_task_aria", { group: name })}>
                {t("create.new_task_button")}
              </button>
            )}
            {onEdit && !editForm && (
              <button type="button" className="link small" onClick={onEdit} aria-label={t("group_edit.edit_aria", { group: name })}>
                {t("group_edit.edit_button")}
              </button>
            )}
          </span>
        )}
      </header>
      {editForm}
      {form}
      {collapsed ? null : tasks.length === 0 ? (
        !hidden?.count && <p className="muted small">{empty}</p>
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
                <span className="id">
                  <Highlight text={task.id} terms={terms} />
                </span>
                <span className="title">
                  <Highlight text={task.title} terms={terms} />
                </span>
                <span className="state-cell">
                  {toCheck(task) && <HumanCheckBadge task={task} />}
                  {toReview(task) && <AiDecidedBadge task={task} />}
                  <span className={`state s-${task.state}`}>{t(`state.${task.state}`)}</span>
                </span>
                <span className="meta">
                  {task.label && <span className="label">{task.label}</span>}
                  {iteration && <span>{t("task.iteration", { iteration })}</span>}
                  {task.holder && <span>{t("task.held_by", { holder: task.holder })}</span>}
                  {stop?.(task) && <span className={`stop s-${task.state}-text`}>{stop(task)}</span>}
                  {task.handoff_note && <span>{t("task.handoff_note", { note: task.handoff_note })}</span>}
                  <DescriptionHit text={task.description ?? ""} terms={terms} />
                </span>
              </li>
            );
          })}
        </ul>
      )}
      {!collapsed && hidden && hidden.count > 0 && (
        <p className="filter-hidden small">
          <span>{hiddenText(hidden, t)}</span>{" "}
          <button type="button" className="link small" onClick={onShowAll}>
            {t("filter.show_all")}
          </button>
        </p>
      )}
    </section>
  );
}
