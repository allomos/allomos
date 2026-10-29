// Taskbags as tabs (Task-A01-marble320, approver 2026-10-29): a project window's tab bar above the
// taskbag view, like a browser's. Each tab is one taskbag (or the start page); + opens a new tab on
// the start page; dragging a tab along the bar changes the order, dragging it out of the window
// makes it a window of its own or moves it into the Allomos window it is let go on; a dot marks a
// taskbag that needs the person. Every tab keeps its own view, so switching tabs loses nothing.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent } from "react";
import { App } from "./App";
import { useI18n } from "./i18n";
import type { Messenger, TabsMessenger } from "./messenger";
import { projectFolder } from "./paths";
import { TabActive } from "./tabActive";
import { frontAfterClose, moveTab, outsideWindow, placeAt, startTabs, tabShowing, type TabEntry, type TabStart } from "./tabsFlow";

/** What a tab's view needs from its window. */
interface Shell {
  isFront(id: number): boolean;
  report(id: number, root: string | null, name: string | null): void;
  showElsewhere(id: number, root: string): Promise<boolean>;
}

/** Each tab's own messenger: it tells the window what the tab shows, finds a taskbag in the
 * window's other tabs before other windows, and hears menu commands and window focus only while
 * the tab is in front. */
function tabMessenger(base: Messenger, id: number, shell: Shell): Messenger {
  const front = () => shell.isFront(id);
  const gate =
    (listen: (handler: () => void) => Promise<() => void>) =>
    (handler: () => void) =>
      listen(() => {
        if (front()) handler();
      });
  return {
    ...base,
    setWindowProject: (root, name) => Promise.resolve(shell.report(id, root, name)),
    showProjectElsewhere: (root) => shell.showElsewhere(id, root),
    setConnectEnabled: (enabled) => (front() ? base.setConnectEnabled(enabled) : Promise.resolve()),
    onWindowFocus: gate(base.onWindowFocus),
    onClearRecentMenu: gate(base.onClearRecentMenu),
    onConnectMenu: gate(base.onConnectMenu),
    onMoveTasksInMenu: gate(base.onMoveTasksInMenu),
    onLanMenu: gate(base.onLanMenu),
    onCheckUpdatesMenu: gate(base.onCheckUpdatesMenu),
  };
}

/** A tab's name: its project's, else its folder's, else the start page's. */
function tabName(tab: TabEntry, startPage: string): string {
  if (tab.name) return tab.name;
  if (tab.root) return projectFolder(tab.root).split(/[\\/]/).filter(Boolean).pop() ?? startPage;
  return startPage;
}

export function WindowTabs({ messenger, tabs: api }: { messenger: Messenger; tabs: TabsMessenger }) {
  const { t } = useI18n();
  const [tabs, setTabs] = useState<TabEntry[]>([]);
  const [front, setFront] = useState<number | null>(null);
  const [started, setStarted] = useState(false);
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const frontRef = useRef(front);
  frontRef.current = front;
  const nextId = useRef(1);
  const views = useRef(new Map<number, { messenger: Messenger; onNeeds: (needs: boolean) => void }>());

  const entry = (start: TabStart): TabEntry => ({ id: nextId.current++, start, root: null, name: null, needs: false });
  const add = useCallback((start: TabStart) => {
    const tab = entry(start);
    setTabs((list) => [...list, tab]);
    setFront(tab.id);
  }, []);
  /** Close tab `id`; the window closes with its last tab. */
  const close = useCallback(
    (id: number) => {
      const list = tabsRef.current;
      const rest = list.filter((tab) => tab.id !== id);
      views.current.delete(id);
      if (rest.length === 0) {
        void api.closeWindow().catch(() => undefined);
        return;
      }
      setFront(frontAfterClose(list, id, frontRef.current));
      setTabs(rest);
    },
    [api],
  );

  // The tabs the window opens with: restored, from Open Recent, dragged in, or the start page; in
  // the first window at start-up with tabs restored, a folder given on the command line opens in a
  // tab of its own.
  useEffect(() => {
    let live = true;
    void api
      .windowStart()
      .then(async (start) => {
        if (!live) return;
        const list = startTabs(start).map(entry);
        setTabs(list);
        setFront(list[Math.min(start.active, list.length - 1)].id);
        setStarted(true);
        if (start.startup && start.tabs.length > 0) {
          const folder = await messenger.startupFolder().catch(() => null);
          if (live && folder) add({ how: "open", root: folder });
        }
      })
      .catch(() => {
        if (!live) return;
        add({ how: "startup", startWith: "last" });
        setStarted(true);
      });
    return () => {
      live = false;
    };
  }, [api, messenger, add]);

  // The app hears the tabs, in their order, and the one in front: the window is named after it,
  // and the windows and tabs are kept for the next start.
  const frontAt = Math.max(0, tabs.findIndex((tab) => tab.id === front));
  const shown = useMemo(() => JSON.stringify(tabs.map((tab) => ({ root: tab.root, name: tab.name }))), [tabs]);
  useEffect(() => {
    if (started) void api.setWindowTabs(JSON.parse(shown), frontAt).catch(() => undefined);
  }, [api, started, shown, frontAt]);

  // File → New Tab and Open Recent, a tab asked for elsewhere, and a tab dragged in.
  useEffect(() => {
    const stops = [
      api.onTabOpen((folder) => add(folder ? { how: "open", root: folder } : { how: "blank" })),
      api.onTabSelect((index) => {
        const tab = tabsRef.current[index];
        if (tab) setFront(tab.id);
      }),
      api.onTabReceive((root) => add(root ? { how: "moved", root } : { how: "blank" })),
    ];
    return () => {
      for (const stop of stops) void stop.then((f) => f()).catch(() => undefined);
    };
  }, [api, add]);

  // Each tab's messenger and dot callback, made once so its view's effects do not start again.
  const shell = useMemo<Shell>(
    () => ({
      isFront: (id) => frontRef.current === id,
      report: (id, root, name) =>
        setTabs((list) => list.map((tab) => (tab.id === id && (tab.root !== root || tab.name !== name) ? { ...tab, root, name } : tab))),
      async showElsewhere(id, root) {
        // A tab opened for a folder (Open Recent, the command line) that another tab already shows
        // closes, and that tab comes to the front.
        const caller = tabsRef.current.find((tab) => tab.id === id);
        const fresh = caller?.root === null && caller.start.how === "open";
        const here = tabShowing(tabsRef.current, root, id);
        if (here) {
          if (fresh) close(id);
          setFront(here.id);
          return true;
        }
        const elsewhere = await messenger.showProjectElsewhere(root).catch(() => false);
        if (elsewhere && fresh) close(id);
        return elsewhere;
      },
    }),
    [messenger, close],
  );
  const viewOf = (id: number) => {
    let view = views.current.get(id);
    if (!view) {
      view = {
        messenger: tabMessenger(messenger, id, shell),
        onNeeds: (needs) => setTabs((list) => list.map((tab) => (tab.id === id && tab.needs !== needs ? { ...tab, needs } : tab))),
      };
      views.current.set(id, view);
    }
    return view;
  };

  // Dragging a tab: along the bar it changes the order; let go outside the window, it leaves.
  const barRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ id: number; x: number; y: number; moved: boolean } | null>(null);
  const [dragging, setDragging] = useState<{ id: number; out: boolean } | null>(null);
  const onTabDown = (e: PointerEvent<HTMLDivElement>, id: number) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest(".tab-close")) return;
    setFront(id);
    drag.current = { id, x: e.clientX, y: e.clientY, moved: false };
    // The bar holds the pointer: the tab itself moves in the page while it is dragged.
    try {
      barRef.current?.setPointerCapture(e.pointerId);
    } catch {
      /* no such pointer any more: the drag ends with the next pointer event */
    }
  };
  const onBarMove = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    if (!d.moved && Math.abs(e.clientX - d.x) < 5 && Math.abs(e.clientY - d.y) < 5) return;
    d.moved = true;
    const out = outsideWindow(e.clientX, e.clientY, window.innerWidth, window.innerHeight);
    setDragging({ id: d.id, out });
    if (out || !barRef.current) return;
    const edges = [...barRef.current.querySelectorAll<HTMLElement>("[data-tab]")].map((el) => el.getBoundingClientRect());
    const from = tabsRef.current.findIndex((tab) => tab.id === d.id);
    const to = placeAt(edges, from, e.clientX);
    if (from >= 0 && to !== from) setTabs((list) => moveTab(list, from, to));
  };
  const onBarUp = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    drag.current = null;
    setDragging(null);
    if (!d?.moved || !outsideWindow(e.clientX, e.clientY, window.innerWidth, window.innerHeight)) return;
    const tab = tabsRef.current.find((x) => x.id === d.id);
    if (!tab) return;
    void api
      .dropTab(tab.root)
      .then((left) => {
        if (left) close(tab.id);
      })
      .catch(() => undefined);
  };
  const onBarCancel = () => {
    drag.current = null;
    setDragging(null);
  };
  // ← and → move between tabs, as in a list of tabs.
  const onTabKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    const at = tabsRef.current.findIndex((tab) => tab.id === frontRef.current);
    const next = tabsRef.current[at + (e.key === "ArrowRight" ? 1 : -1)];
    if (!next) return;
    e.preventDefault();
    setFront(next.id);
    barRef.current?.querySelector<HTMLElement>(`[data-tab="${next.id}"]`)?.focus();
  };

  const startPage = t("tabs.start_page");
  return (
    <div className="window">
      <div
        className="tabbar"
        role="tablist"
        aria-label={t("tabs.list")}
        ref={barRef}
        onPointerMove={onBarMove}
        onPointerUp={onBarUp}
        onPointerCancel={onBarCancel}
      >
        {tabs.map((tab) => {
          const name = tabName(tab, startPage);
          const isFront = tab.id === front;
          const drag = dragging?.id === tab.id ? (dragging.out ? " leaving" : " dragging") : "";
          return (
            <div
              key={tab.id}
              data-tab={tab.id}
              role="tab"
              aria-selected={isFront}
              tabIndex={isFront ? 0 : -1}
              className={`tab${isFront ? " front" : ""}${drag}`}
              title={tab.root ? projectFolder(tab.root) : name}
              onPointerDown={(e) => onTabDown(e, tab.id)}
              onKeyDown={onTabKey}
            >
              {tab.needs && <span className="tab-dot" role="img" aria-label={t("tabs.needs_you")} title={t("tabs.needs_you")} />}
              <span className="tab-name">{name}</span>
              <button type="button" className="tab-close" aria-label={t("tabs.close")} title={t("tabs.close")} onClick={() => close(tab.id)}>
                ×
              </button>
            </div>
          );
        })}
        <button type="button" className="tab-new" aria-label={t("tabs.new")} title={t("tabs.new")} onClick={() => add({ how: "blank" })}>
          +
        </button>
      </div>
      {/* The views in the order the tabs opened, so reordering the tabs moves nothing on the page. */}
      {[...tabs]
        .sort((a, b) => a.id - b.id)
        .map((tab) => (
          <div key={tab.id} className="tab-view" role="tabpanel" hidden={tab.id !== front}>
            <TabActive.Provider value={tab.id === front}>
              <App messenger={viewOf(tab.id).messenger} start={tab.start} onNeeds={viewOf(tab.id).onNeeds} />
            </TabActive.Provider>
          </div>
        ))}
    </div>
  );
}
