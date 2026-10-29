// Taskbags as tabs (Task-A01-marble320, approver 2026-10-29): a project window's tab bar above the
// taskbag view, like a browser's. Each tab is one taskbag (or the start page); + opens a new tab on
// the start page; a dot marks a taskbag that needs the person. Every tab keeps its own view, so
// switching tabs loses nothing.
//
// Dragging a tab looks like Chrome (Task-A01-fresco512): along the bar the tab follows the pointer
// and the others slide aside to open the gap where it will land; pulled out of the bar it tears off
// and its window follows the pointer (the app moves it, `tab_drag.rs`); over a tab bar, that bar
// opens a gap for it, and let go there it joins at the gap. Escape puts it back.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent } from "react";
import { App } from "./App";
import { useI18n } from "./i18n";
import type { Messenger, TabsMessenger } from "./messenger";
import { projectFolder } from "./paths";
import { TabActive } from "./tabActive";
import { frontAfterClose, insertAt, moveTab, placeAt, shiftFor, startTabs, tabShowing, tearsOff, type TabEntry, type TabStart } from "./tabsFlow";

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
  /** A dragged tab over this bar (from another window, or this window's own torn-off tab) is shown
   * in front (Task-A01-desk708): that tab (one made here for it, `made`, or the torn-off tab), and
   * the tab in front before, which comes back when it leaves. Set at once, before React renders. */
  const hoverTab = useRef<{ id: number; made: boolean; prev: number | null } | null>(null);
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
  // A tab dragged over the bar from another window is not the window's until it is let go.
  const [hover, setHover] = useState<{ gap: number; left: number; width: number; name: string | null; id: number; made: boolean } | null>(null);
  const hoverRef = useRef(hover);
  hoverRef.current = hover;
  const madeId = hover?.made ? hover.id : null;
  const kept = useMemo(() => tabs.filter((tab) => tab.id !== madeId), [tabs, madeId]);
  const keptFront = madeId === null ? front : hoverTab.current?.prev ?? front;
  const frontAt = Math.max(0, kept.findIndex((tab) => tab.id === keptFront));
  const shown = useMemo(() => JSON.stringify(kept.map((tab) => ({ root: tab.root, name: tab.name }))), [kept]);
  useEffect(() => {
    if (started) void api.setWindowTabs(JSON.parse(shown), frontAt).catch(() => undefined);
  }, [api, started, shown, frontAt]);

  // File → New Tab and Open Recent, and a tab asked for elsewhere.
  useEffect(() => {
    const stops = [
      api.onTabOpen((folder) => add(folder ? { how: "open", root: folder } : { how: "blank" })),
      api.onTabSelect((index) => {
        const tab = tabsRef.current[index];
        if (tab) setFront(tab.id);
      }),
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

  // Dragging a tab (Task-A01-fresco512).
  const barRef = useRef<HTMLDivElement>(null);
  /** The pointer held on a tab: where it went down, where on the tab, and whether the tab tore off. */
  const drag = useRef<{ id: number; x: number; y: number; grabX: number; grabY: number; moved: boolean; torn: boolean } | null>(null);
  /** Along the bar: the dragged tab's place, how far it has moved, the place it will land, its
   * width, and every tab's edges when the drag started. */
  const [local, setLocal] = useState<{ id: number; from: number; dx: number; gap: number; width: number; edges: DOMRect[] } | null>(null);
  /** A tab of this window torn off and being dragged: hidden here until it lands. */
  const [away, setAway] = useState<{ id: number } | null>(null);
  const awayRef = useRef(away);
  awayRef.current = away;
  /** For one frame after a drop the tabs jump to their new places without sliding. */
  const [settle, setSettle] = useState(false);
  useEffect(() => {
    if (!settle) return;
    const frame = requestAnimationFrame(() => setSettle(false));
    return () => cancelAnimationFrame(frame);
  }, [settle]);

  /** The shown tabs' edges in the bar (a torn-off tab is hidden). */
  const shownEdges = () =>
    barRef.current ? [...barRef.current.querySelectorAll<HTMLElement>("[data-tab]:not(.away)")].map((el) => el.getBoundingClientRect()) : [];
  /** Bring the torn-off tab back, at `at` among the shown tabs, or where it was. */
  const bringBack = (at: number | null) => {
    const a = awayRef.current;
    if (!a) return;
    if (at !== null) {
      setTabs((list) => {
        const tab = list.find((x) => x.id === a.id);
        if (!tab) return list;
        return insertAt(
          list.filter((x) => x.id !== a.id),
          tab,
          at,
        );
      });
      setSettle(true);
    }
    setFront(a.id);
    setAway(null);
  };

  // A dragged tab over this window's tab bar, let go on it, or called off.
  useEffect(() => {
    const stops = [
      api.onTabHover(({ x, root, name }) => {
        let h = hoverTab.current;
        if (!h) {
          // It shows in front: this window's own torn-off tab, or a tab made here for it.
          const own = awayRef.current;
          if (own) {
            h = { id: own.id, made: false, prev: frontRef.current };
          } else {
            const tab = entry(root ? { how: "moved", root } : { how: "blank" });
            setTabs((list) => [...list, tab]);
            h = { id: tab.id, made: true, prev: frontRef.current };
          }
          hoverTab.current = h;
          setFront(h.id);
        }
        const bar = barRef.current?.getBoundingClientRect();
        const edges = shownEdges();
        const gap = placeAt(edges, -1, x);
        const width = Math.min(200, edges[0]?.width ?? 200);
        const left = (gap < edges.length ? edges[gap].left : (edges[edges.length - 1]?.right ?? 8) + 2) - (bar?.left ?? 0);
        setHover({ gap, left, width, name, id: h.id, made: h.made });
      }),
      api.onTabHoverEnd(() => {
        const h = hoverTab.current;
        hoverTab.current = null;
        setHover(null);
        if (!h) return;
        if (h.made) {
          views.current.delete(h.id);
          setTabs((list) => list.filter((tab) => tab.id !== h.id));
        }
        setFront(h.prev);
      }),
      api.onTabDropHere((root) => {
        const gap = hoverRef.current?.gap ?? tabsRef.current.length;
        const h = hoverTab.current;
        hoverTab.current = null;
        setHover(null);
        // The tab shown while it was over the bar stays, where the gap was, without loading again.
        const made = h?.made ? tabsRef.current.find((tab) => tab.id === h.id) : undefined;
        const tab = made ?? entry(root ? { how: "moved", root } : { how: "blank" });
        setTabs((list) => insertAt(list.filter((x) => x.id !== tab.id), tab, gap));
        setFront(tab.id);
        setSettle(true);
      }),
      api.onDragCancelled(() => {
        drag.current = null;
        hoverTab.current = null;
        setHover(null);
        bringBack(null);
      }),
    ];
    return () => {
      for (const stop of stops) void stop.then((f) => f()).catch(() => undefined);
    };
  }, [api]);

  const onTabDown = (e: PointerEvent<HTMLDivElement>, id: number) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest(".tab-close")) return;
    setFront(id);
    const rect = e.currentTarget.getBoundingClientRect();
    // Held at the same place in a new window's bar, whose first tab starts at the bar's padding.
    const grabX = 8 + (e.clientX - rect.left);
    const grabY = 6 + (e.clientY - rect.top);
    drag.current = { id, x: e.clientX, y: e.clientY, grabX, grabY, moved: false, torn: false };
    // The bar holds the pointer: the tab itself moves in the page while it is dragged.
    try {
      barRef.current?.setPointerCapture(e.pointerId);
    } catch {
      /* no such pointer any more: the drag ends with the next pointer event */
    }
  };
  const tear = (d: NonNullable<typeof drag.current>) => {
    d.torn = true;
    setLocal(null);
    const tab = tabsRef.current.find((x) => x.id === d.id);
    if (!tab) return;
    const only = tabsRef.current.length === 1;
    void api
      .dragTear(tab.root, tab.name, d.grabX, d.grabY)
      .then((started) => {
        if (!started) {
          drag.current = null;
          return;
        }
        // Its window comes with it; here the tab is hidden until it lands (the window's only tab
        // stays: the window itself moves).
        if (!only) {
          setAway({ id: tab.id });
          setFront(frontAfterClose(tabsRef.current, tab.id, tab.id));
        }
      })
      .catch(() => {
        drag.current = null;
      });
  };
  const onBarMove = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.torn) return;
    if (!d.moved && Math.abs(e.clientX - d.x) < 5 && Math.abs(e.clientY - d.y) < 5) return;
    const bar = barRef.current?.getBoundingClientRect();
    if (!bar) return;
    if (tearsOff(e.clientX, e.clientY, bar.top, bar.bottom, window.innerWidth)) {
      tear(d);
      return;
    }
    d.moved = true;
    setLocal((was) => {
      const from = was?.from ?? tabsRef.current.findIndex((x) => x.id === d.id);
      const edges = was?.edges ?? shownEdges();
      const width = (edges[from]?.width ?? 0) + 2;
      return { id: d.id, from, dx: e.clientX - d.x, gap: placeAt(edges, from, e.clientX), width, edges };
    });
  };
  const onBarUp = () => {
    const d = drag.current;
    drag.current = null;
    if (!d) return;
    if (d.torn) {
      const at = hoverRef.current?.gap ?? null;
      void api
        .dragEnd()
        .then((how) => {
          hoverTab.current = null;
          setHover(null);
          if (how === "moved") {
            setAway(null);
            close(d.id);
          } else {
            bringBack(at);
          }
        })
        .catch(() => bringBack(null));
      return;
    }
    const l = local;
    setLocal(null);
    if (l && l.gap !== l.from) {
      setTabs((list) => moveTab(list, l.from, l.gap));
      setSettle(true);
    }
  };
  const onBarCancel = () => {
    const d = drag.current;
    drag.current = null;
    setLocal(null);
    if (d?.torn) void api.dragCancel().catch(() => undefined);
  };
  // Escape while dragging puts the tab back; a torn-off tab's drag may have the keyboard in another
  // window, which calls it off too (the app ignores it when nothing is dragged).
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (drag.current && !drag.current.torn) {
        drag.current = null;
        setLocal(null);
        return;
      }
      void api.dragCancel().catch(() => undefined);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [api]);
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
  /** How far each shown tab slides: aside for the dragged tab's gap, or the dragged tab itself. */
  const slide = (shownAt: number, id: number): number => {
    if (local) return id === local.id ? local.dx : shiftFor(shownAt, local.from, local.gap, local.width);
    if (hover) return shiftFor(shownAt, -1, hover.gap, hover.width + 2);
    return 0;
  };
  let shownAt = 0;
  return (
    <div className="window">
      <div
        className={`tabbar${settle ? " settle" : ""}`}
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
          // Hidden in the bar: a torn-off tab, and a tab made for one over the bar (drawn where it lands).
          const isAway = away?.id === tab.id || madeId === tab.id;
          const dx = isAway ? 0 : slide(shownAt, tab.id);
          if (!isAway) shownAt += 1;
          return (
            <div
              key={tab.id}
              data-tab={tab.id}
              role="tab"
              aria-selected={isFront}
              tabIndex={isFront ? 0 : -1}
              className={`tab${isFront ? " front" : ""}${local?.id === tab.id ? " dragging" : ""}${isAway ? " away" : ""}`}
              style={dx ? { transform: `translateX(${dx}px)` } : undefined}
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
        {hover && (
          // Where the dragged tab will land.
          <div className="tab front tab-phantom" aria-hidden="true" style={{ left: hover.left, width: hover.width }}>
            <span className="tab-name">{hover.name ?? startPage}</span>
          </div>
        )}
        <button
          type="button"
          className="tab-new"
          aria-label={t("tabs.new")}
          title={t("tabs.new")}
          style={hover && hover.gap >= shownAt ? { transform: `translateX(${hover.width + 2}px)` } : undefined}
          onClick={() => add({ how: "blank" })}
        >
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
