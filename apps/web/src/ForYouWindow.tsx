// The For You window (Task-A01-oak609, decision 0028): a window of its own, beside the project
// windows. On the left, what needs the person in the project it shows, one row per task, in the
// order `alm human` lists it (the core's queue); on the right, the chosen task's pane, which first
// says in one sentence what the person is asked to do there, then shows what that task's state
// asks to read (decision 0025), the rest folded. A row the person has handled leaves the list,
// and the next one is chosen. The tasks not classified fold into one line, folded at first (§ 7).
// It opens on the project in front in the window it was asked from (for_you.rs).

import { useCallback, useEffect, useRef, useState } from "react";
import { TaskLanguageProvider, useI18n } from "./i18n";
import type { ForYouProject, Messenger } from "./messenger";
import { evidenceReader } from "./evidenceFlow";
import { projectFolder } from "./paths";
import { askText, askedNow, forYouTitle, nextChoice, riskWord, splitQueue, todoText, waitedText, type QueueRow } from "./queueFlow";
import { labelNames, toCheck, withBlocked } from "./stopFlow";
import { lostAfterMinutes } from "./heartbeatFlow";
import { TaskPane } from "./TaskPane";
import { isProblem, type NextIds, type Opened, type Task, type TaskHistory } from "./types";
import type { FolderInfo } from "./folderFlow";

/** How often the window looks again when nothing told it the taskbag changed (and for `alm
 * confirm` questions, which are not in the taskbag). */
const LOOK_MS = 5000;

export function ForYouWindow({ messenger }: { messenger: Messenger }) {
  const [project, setProject] = useState<ForYouProject | null>(null);
  useEffect(() => {
    let live = true;
    messenger
      .forYouProject()
      .then((p) => live && p && setProject(p))
      .catch(() => undefined);
    const stop = messenger.onForYouProject((p) => live && setProject(p));
    return () => {
      live = false;
      void stop.then((un) => un()).catch(() => undefined);
    };
  }, [messenger]);
  return project ? <ForYouProjectView key={project.root} messenger={messenger} project={project} /> : <ForYouEmpty />;
}

function ForYouEmpty() {
  const { t } = useI18n();
  useEffect(() => {
    document.title = t("for_you.name");
  }, [t]);
  return (
    <main className="for-you">
      <p className="muted for-you-none">{t("for_you.no_project")}</p>
    </main>
  );
}

function ForYouProjectView({ messenger, project }: { messenger: Messenger; project: ForYouProject }) {
  const { t, problem } = useI18n();
  const root = project.root;
  const [opened, setOpened] = useState<Opened | null>(null);
  const [rows, setRows] = useState<QueueRow[] | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rowsRef = useRef<QueueRow[]>([]);
  const chosenRef = useRef<string | null>(null);
  chosenRef.current = chosen;
  const stampRef = useRef<string | null>(null);

  const say = useCallback((e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) })), [problem, t]);

  /** Read the taskbag and the queue again. */
  const read = useCallback(async () => {
    const [fresh, asked] = await Promise.all([
      messenger.openTaskbag(root),
      messenger.confirmRequests(projectFolder(root)).catch(() => []),
    ]);
    const list = await messenger.humanQueue(root, askedNow(asked));
    stampRef.current = await messenger.taskbagStamp(root).catch(() => null);
    setOpened(withBlocked(fresh));
    setChosen(nextChoice(rowsRef.current, list, chosenRef.current));
    rowsRef.current = list;
    setRows(list);
  }, [messenger, root]);

  useEffect(() => {
    void read().catch(say);
    let live = true;
    let stop: (() => void) | undefined;
    messenger.followTaskbag(root, () => void read().catch(() => undefined)).then(
      (un) => (live ? (stop = un) : un()),
      () => undefined,
    );
    // Questions from `alm confirm` are not in the taskbag, and a missed notice must not leave the
    // list stale: look again now and then, reading everything only when something changed.
    const timer = window.setInterval(() => {
      void (async () => {
        const stamp = await messenger.taskbagStamp(root).catch(() => null);
        const asked = askedNow(await messenger.confirmRequests(projectFolder(root)).catch(() => []));
        const known = rowsRef.current.filter((r) => r.todo.includes("approve")).map((r) => r.task);
        const askedChanged = asked.map((a) => a.task).sort().join() !== [...new Set(known)].sort().join();
        if (stamp !== stampRef.current || askedChanged) await read();
      })().catch(() => undefined);
    }, LOOK_MS);
    return () => {
      live = false;
      stop?.();
      window.clearInterval(timer);
    };
  }, [messenger, root, read, say]);

  const name = opened?.loaded.taskbag?.project.name ?? project.name;
  useEffect(() => {
    const title = forYouTitle(name, t);
    document.title = title;
    void messenger.setForYouTitle(title).catch(() => undefined);
  }, [messenger, name, t]);

  return (
    <TaskLanguageProvider taskLanguage={opened?.loaded.taskbag?.project.task_language}>
      <main className="for-you">
        {error && (
          <p className="banner error" role="alert">
            {error}{" "}
            <button type="button" className="link" onClick={() => setError(null)}>
              {t("pane.close")}
            </button>
          </p>
        )}
        {rows === null || !opened ? (
          <p className="muted for-you-none">{t("for_you.reading")}</p>
        ) : (
          <ForYouBody
            messenger={messenger}
            opened={opened}
            rows={rows}
            chosen={chosen}
            onChoose={setChosen}
            onChanged={() => read().catch(() => undefined)}
            onError={say}
          />
        )}
      </main>
    </TaskLanguageProvider>
  );
}

function ForYouBody({
  messenger,
  opened,
  rows,
  chosen,
  onChoose,
  onChanged,
  onError,
}: {
  messenger: Messenger;
  opened: Opened;
  rows: QueueRow[];
  chosen: string | null;
  onChoose: (task: string | null) => void;
  /** Read again after a change made in the pane. */
  onChanged: () => Promise<void>;
  onError: (e: unknown) => void;
}) {
  const { t } = useI18n();
  const root = opened.root;
  const box = opened.loaded.taskbag;
  const tasks = box?.tasks ?? [];
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const { shown, unclassified } = splitQueue(rows);
  const [unfolded, setUnfolded] = useState(false);
  const now = new Date();

  // What the pane needs besides the task: read once, or again for each task.
  const [ownerName, setOwnerName] = useState<string | null>(null);
  const [folders, setFolders] = useState<FolderInfo[]>([]);
  const [nextIds, setNextIds] = useState<NextIds | null>(null);
  const [history, setHistory] = useState<{ id: string; history: TaskHistory | null } | null>(null);
  useEffect(() => {
    messenger.ownerName().then(setOwnerName).catch(() => undefined);
  }, [messenger]);
  useEffect(() => {
    let live = true;
    messenger.projectFolders(root).then((f) => live && setFolders(f), () => live && setFolders([]));
    messenger.nextIds(root).then((n) => live && setNextIds(n), () => live && setNextIds(null));
    return () => {
      live = false;
    };
  }, [messenger, root, opened]);
  useEffect(() => {
    if (!chosen) return;
    let live = true;
    messenger.taskHistory(root, chosen).then(
      (h) => live && setHistory({ id: chosen, history: h }),
      () => live && setHistory({ id: chosen, history: null }),
    );
    return () => {
      live = false;
    };
  }, [messenger, root, chosen, opened]);
  // The chosen row is folded away: show the fold open, so the chosen row is in view.
  useEffect(() => {
    if (chosen && unclassified.some((r) => r.task === chosen)) setUnfolded(true);
  }, [chosen, unclassified]);

  const after = async (change: Promise<unknown>) => {
    try {
      await change;
    } finally {
      await onChanged();
    }
  };

  const row = (r: QueueRow) => {
    const task = byId.get(r.task);
    const waited = waitedText(r.waiting_since, now, t);
    return (
      <li key={r.task}>
        <button
          type="button"
          className={`for-you-row${r.task === chosen ? " selected" : ""}`}
          aria-current={r.task === chosen ? "true" : undefined}
          onClick={() => onChoose(r.task)}
        >
          <span className="for-you-title">
            <span className="id">{r.task}</span> {task?.title ?? ""}
          </span>
          <span className="for-you-todo">{todoText(r, t)}</span>
          <span className="for-you-facts small">
            <span className={`risk risk-${["high", "medium", "low"].includes(r.risk ?? "") ? r.risk : "none"}`}>{riskWord(r.risk, t)}</span>
            {waited && <span className="muted"> · {waited}</span>}
            {r.blocks > 0 && <span className="muted"> · {t("for_you.blocks", { n: r.blocks })}</span>}
          </span>
        </button>
      </li>
    );
  };

  const waitingRows = shown.filter((r) => r.agent_waiting);
  const finishedRows = shown.filter((r) => !r.agent_waiting);
  const chosenRow = rows.find((r) => r.task === chosen) ?? null;
  const selected: Task | null = chosen ? (byId.get(chosen) ?? null) : null;
  const ask = chosenRow && (
    <>
      {askText(chosenRow, t)}
      {chosenRow.todo.includes("approve") && <span className="muted small"> {t("for_you.approve_where")}</span>}
      {chosenRow.todo.includes("approve_afterwards") && <span className="muted small"> {t("for_you.afterwards_where")}</span>}
    </>
  );

  return (
    <div className="for-you-layout">
      <nav className="for-you-list" aria-label={t("for_you.list_aria")}>
        <p className="for-you-count muted small">{t("for_you.count", { n: rows.length })}</p>
        {rows.length === 0 && <p className="muted for-you-none">{t("for_you.nothing")}</p>}
        {waitingRows.length > 0 && (
          <section>
            <h2>{t("for_you.group.waiting")}</h2>
            <ol>{waitingRows.map(row)}</ol>
          </section>
        )}
        {(finishedRows.length > 0 || unclassified.length > 0) && (
          <section>
            <h2>{t("for_you.group.finished")}</h2>
            <ol>{finishedRows.map(row)}</ol>
            {unclassified.length > 0 && (
              <>
                <button type="button" className="for-you-fold" aria-expanded={unfolded} onClick={() => setUnfolded((u) => !u)}>
                  <span aria-hidden="true">{unfolded ? "▾" : "▸"}</span> {t("for_you.unclassified", { n: unclassified.length })}
                </button>
                {unfolded && <ol>{unclassified.map(row)}</ol>}
              </>
            )}
          </section>
        )}
      </nav>
      <div className="for-you-pane">
        {selected ? (
          <>
            {/* A task to check says it inside its Evidence screen (decision 0029); others above the pane. */}
            {ask && !toCheck(selected) && (
              <p className="for-you-ask" role="status">
                {ask}
              </p>
            )}
            <TaskPane
              task={selected}
              version={opened.versions?.[selected.id] ?? null}
              tasks={tasks}
              area={box?.areas.find((a) => a.id === selected.area)}
              iteration={box?.iterations.find((i) => i.id === selected.iteration)}
              onClose={() => onChoose(null)}
              // A blocker's link shows that task here too, whether or not it needs the person.
              onOpenTask={(id) => onChoose(id)}
              onLink={(href) => messenger.openTaskLink(root, selected.id, href).catch(onError)}
              history={history?.id === selected.id ? history.history : null}
              labels={labelNames(box?.project, tasks)}
              iterations={box?.iterations ?? []}
              ownerName={ownerName}
              onSaveName={async (n) => {
                await messenger.saveOwnerName(n);
                setOwnerName(n);
              }}
              onEdit={(edit) => after(messenger.editTask(root, selected.id, edit))}
              onReview={() => after(messenger.reviewAiDecisions(root, selected.id))}
              onApprove={(shown, expires, ready) => after(messenger.approveTask(root, selected.id, shown, expires, ready))}
              onWithdraw={(id, reason) => after(messenger.withdrawApproval(root, id, reason))}
              nextIterationId={nextIds?.iteration ?? null}
              onNewIteration={(iteration) => after(messenger.addIterationAndPlan(root, selected.id, iteration))}
              folders={folders}
              onOpenSession={() => messenger.openAgentSession(root, selected.id)}
              lostAfter={lostAfterMinutes(box?.project)}
              lastChange={opened.last_changed?.[selected.id]}
              ask={ask}
              reader={evidenceReader(messenger, root)}
            />
          </>
        ) : (
          <p className="muted for-you-none">{rows.length > 0 ? t("for_you.choose") : ""}</p>
        )}
      </div>
    </div>
  );
}
