import { useCallback, useEffect, useState } from "react";
import type { Messenger } from "./messenger";
import type { Area, Issue, Opened, State, Task } from "./types";

const STATE_TEXT: Record<State, string> = {
  backlog: "Backlog",
  ready: "Ready",
  in_progress: "In progress",
  waiting: "Waiting",
  done: "Done",
  dropped: "Dropped",
};

const LAST_ROOT_KEY = "glidebox.lastRoot";

function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  const last = parts[parts.length - 1] ?? "Project";
  return last === ".glidebox" ? parts[parts.length - 2] ?? "Project" : last;
}

type Pending = { folder: string; suggestedRoot: string; gitRoot: string | null };

export function App({ messenger }: { messenger: Messenger }) {
  const [opened, setOpened] = useState<Opened | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const show = useCallback((o: Opened, message: string | null) => {
    setOpened(o);
    setPending(null);
    setNotice(message);
    setError(null);
    try {
      localStorage.setItem(LAST_ROOT_KEY, o.root);
    } catch {
      /* storage unavailable: nothing to remember */
    }
  }, []);

  const openFolder = useCallback(
    async (path: string) => {
      setError(null);
      try {
        const info = await messenger.inspectFolder(path);
        const i = info.inspection;
        if (i.kind === "found") {
          show(await messenger.openTaskBox(i.root), null);
        } else if (i.kind === "empty") {
          show(await messenger.createTaskBox(i.folder, folderName(i.folder)), "Created a starter project in this empty folder.");
        } else if (i.kind === "no_task_box") {
          setPending({ folder: i.folder, suggestedRoot: i.suggested_root, gitRoot: info.git_root });
        } else {
          setError(`${i.path} is not a folder.`);
        }
      } catch (e) {
        setError(String(e));
      }
    },
    [messenger, show],
  );

  useEffect(() => {
    let last: string | null = null;
    try {
      last = localStorage.getItem(LAST_ROOT_KEY);
    } catch {
      last = null;
    }
    if (last) void openFolder(last);
  }, [openFolder]);

  const choose = async () => {
    const path = await messenger.pickFolder();
    if (path) await openFolder(path);
  };

  const createNested = async () => {
    if (!pending) return;
    try {
      show(
        await messenger.createTaskBox(pending.suggestedRoot, folderName(pending.folder)),
        "Created a task box in .glidebox/ with a starter project.",
      );
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="app">
      <header className="top">
        <div className="brand" aria-label="Glidebox">
          <svg viewBox="0 0 28 28" aria-hidden="true">
            <rect x="2" y="6" width="24" height="18" rx="3" />
            <path d="M5 11 L13 16 L23 21" />
          </svg>
          <span>Glidebox</span>
        </div>
        {opened?.loaded.task_box && (
          <div className="where">
            <strong>{opened.loaded.task_box.project.name}</strong>
            <code title={opened.root}>{opened.root}</code>
          </div>
        )}
        <button type="button" className="primary" onClick={choose}>
          {opened ? "Open another folder…" : "Choose a folder…"}
        </button>
      </header>

      <main className="main">
        {error && <p className="banner error">{error}</p>}
        {notice && <p className="banner ok">{notice}</p>}

        {pending && (
          <section className="card">
            <h2>No task box in this folder yet</h2>
            <p>
              <code>{pending.folder}</code> has other files but no glidebox task box.
            </p>
            {pending.gitRoot && (
              <p className="muted">
                It is inside the Git repository <code>{pending.gitRoot}</code>. Your task files will be versioned by that
                repository; glidebox does not create a second one and does not run Git.
              </p>
            )}
            <div className="row">
              <button type="button" className="primary" onClick={createNested}>
                Create task box in .glidebox/
              </button>
              <button type="button" onClick={() => setPending(null)}>
                Cancel
              </button>
            </div>
          </section>
        )}

        {!opened && !pending && <Welcome onChoose={choose} />}

        {opened && <TaskBoxView opened={opened} />}
      </main>
    </div>
  );
}

function Welcome({ onChoose }: { onChoose: () => void }) {
  return (
    <section className="welcome">
      <h1>Your project's tasks, in a folder you choose.</h1>
      <p>
        Pick a folder. If it already holds a task box, glidebox shows its tasks grouped by area. If the folder is empty,
        glidebox creates a small starter project in it. In a folder with other files, such as a code repository,
        glidebox offers to create the task box in <code>.glidebox/</code>.
      </p>
      <button type="button" className="primary" onClick={onChoose}>
        Choose a folder…
      </button>
    </section>
  );
}

function TaskBoxView({ opened }: { opened: Opened }) {
  const { task_box: box, issues } = opened.loaded;
  return (
    <>
      {issues.length > 0 && <Issues issues={issues} />}
      {box ? (
        <div className="areas">
          {box.areas.map((area) => (
            <AreaSection key={area.id} area={area} tasks={box.tasks.filter((t) => t.area === area.id)} />
          ))}
          {box.tasks.some((t) => !box.areas.find((a) => a.id === t.area)) && (
            <AreaSection
              area={{ id: "?", name: "Tasks with an unknown area", order: 9999 }}
              tasks={box.tasks.filter((t) => !box.areas.find((a) => a.id === t.area))}
            />
          )}
          {box.areas.length === 0 && box.tasks.length === 0 && (
            <p className="muted">This task box has no areas or tasks yet.</p>
          )}
        </div>
      ) : (
        <p className="muted">The project file could not be read, so no tasks are shown. Fix the problem above.</p>
      )}
    </>
  );
}

function Issues({ issues }: { issues: Issue[] }) {
  return (
    <section className="issues" aria-label="Problems in this task box">
      <h2>
        {issues.length} {issues.length === 1 ? "problem" : "problems"} in this task box
      </h2>
      <ul>
        {issues.map((i, n) => (
          <li key={n}>
            <code>{i.file}</code>
            {i.field && (
              <>
                {" "}
                · field <code>{i.field}</code>
              </>
            )}
            <span>{i.message}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function AreaSection({ area, tasks }: { area: Area; tasks: Task[] }) {
  const done = tasks.filter((t) => t.state === "done").length;
  const pct = tasks.length ? Math.round((done / tasks.length) * 100) : 0;
  return (
    <section className="area">
      <header className="area-head">
        <h2>
          <span className="id">{area.id}</span> {area.name}
        </h2>
        <span className="count">
          {done} of {tasks.length} done
        </span>
        <div className="meter" role="img" aria-label={`${pct}% done`}>
          <div style={{ width: `${pct}%` }} />
        </div>
      </header>
      {tasks.length === 0 ? (
        <p className="muted small">No tasks in this area.</p>
      ) : (
        <ul className="tasks">
          {tasks.map((t) => (
            <li key={t.id} className="task">
              <span className="id">{t.id}</span>
              <span className="title">{t.title}</span>
              <span className={`state s-${t.state}`}>{STATE_TEXT[t.state]}</span>
              <span className="meta">
                {t.label && <span className="label">{t.label}</span>}
                {t.holder && <span>held by {t.holder}</span>}
                {t.waiting_on && <span>waiting on {t.waiting_on}</span>}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
