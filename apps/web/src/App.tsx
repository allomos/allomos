import { useCallback, useEffect, useState } from "react";
import { LANGUAGES, LANGUAGE_NAMES, useI18n } from "./i18n";
import type { Language, Message } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";
import type { Area, Issue, Opened, Problem, Task } from "./types";

const LAST_ROOT_KEY = "glidebox.lastRoot";

function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  const last = parts[parts.length - 1] ?? "Project";
  return last === ".glidebox" ? parts[parts.length - 2] ?? "Project" : last;
}

type Pending = { folder: string; suggestedRoot: string; gitRoot: string | null };

/** A banner's text: an interface message, or a problem sent by Rust. */
type Note = Message | { problem: Problem };

function errorNote(e: unknown): Note {
  return isProblem(e) ? { problem: e } : { key: "error.unexpected", params: { detail: String(e) } };
}

export function App({ messenger }: { messenger: Messenger }) {
  const { t, rich, problem } = useI18n();
  const [opened, setOpened] = useState<Opened | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);
  const [notice, setNotice] = useState<Note | null>(null);
  const [error, setError] = useState<Note | null>(null);
  const noteText = (n: Note) => ("problem" in n ? problem(n.problem) : t(n.key, n.params));

  const show = useCallback((o: Opened, message: Note | null) => {
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
          show(await messenger.createTaskBox(i.folder, folderName(i.folder)), { key: "notice.created_here" });
        } else if (i.kind === "no_task_box") {
          setPending({ folder: i.folder, suggestedRoot: i.suggested_root, gitRoot: info.git_root });
        } else {
          setError({ key: "error.not_a_folder", params: { path: i.path } });
        }
      } catch (e) {
        setError(errorNote(e));
      }
    },
    [messenger, show],
  );

  // On start: a folder given on the command line wins; otherwise reopen the last one.
  useEffect(() => {
    void (async () => {
      let start = await messenger.startupFolder().catch(() => null);
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

  const createNested = async () => {
    if (!pending) return;
    try {
      show(
        await messenger.createTaskBox(pending.suggestedRoot, folderName(pending.folder)),
        { key: "notice.created_nested" },
      );
    } catch (e) {
      setError(errorNote(e));
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
        <LanguagePicker />
        <button type="button" className="primary" onClick={choose}>
          {opened ? t("top.open_another") : t("top.choose_folder")}
        </button>
      </header>

      <main className="main">
        {error && <p className="banner error">{noteText(error)}</p>}
        {notice && <p className="banner ok">{noteText(notice)}</p>}

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

        {opened && <TaskBoxView opened={opened} />}
      </main>
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

function Welcome({ onChoose }: { onChoose: () => void }) {
  const { t, rich } = useI18n();
  return (
    <section className="welcome">
      <h1>{t("welcome.title")}</h1>
      <p>{rich("welcome.body", { dir: <code>.glidebox/</code> })}</p>
      <button type="button" className="primary" onClick={onChoose}>
        {t("top.choose_folder")}
      </button>
    </section>
  );
}

function TaskBoxView({ opened }: { opened: Opened }) {
  const { t } = useI18n();
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
              area={{ id: "?", name: t("box.unknown_area"), order: 9999 }}
              tasks={box.tasks.filter((t) => !box.areas.find((a) => a.id === t.area))}
            />
          )}
          {box.areas.length === 0 && box.tasks.length === 0 && (
            <p className="muted">{t("box.empty")}</p>
          )}
        </div>
      ) : (
        <p className="muted">{t("box.no_project")}</p>
      )}
    </>
  );
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

function AreaSection({ area, tasks }: { area: Area; tasks: Task[] }) {
  const { t } = useI18n();
  const done = tasks.filter((t) => t.state === "done").length;
  const pct = tasks.length ? Math.round((done / tasks.length) * 100) : 0;
  return (
    <section className="area">
      <header className="area-head">
        <h2>
          <span className="id">{area.id}</span> {area.name}
        </h2>
        <span className="count">{t("area.count", { done, total: tasks.length })}</span>
        <div className="meter" role="img" aria-label={t("area.meter", { pct })}>
          <div style={{ width: `${pct}%` }} />
        </div>
      </header>
      {tasks.length === 0 ? (
        <p className="muted small">{t("area.no_tasks")}</p>
      ) : (
        <ul className="tasks">
          {tasks.map((task) => (
            <li key={task.id} className="task">
              <span className="id">{task.id}</span>
              <span className="title">{task.title}</span>
              <span className={`state s-${task.state}`}>{t(`state.${task.state}`)}</span>
              <span className="meta">
                {task.label && <span className="label">{task.label}</span>}
                {task.holder && <span>{t("task.held_by", { holder: task.holder })}</span>}
                {task.waiting_on && <span>{t("task.waiting_on", { what: task.waiting_on })}</span>}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
