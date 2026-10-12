// The setup wizard (Task-A05-03): a dialog that sets up a taskbag in four steps.
//   1. What a taskbag is, and choose the folder where `.allomos` goes.
//   2. That folder has `.allomos` (or Create made it): Next. It has none: Create. Previous goes back.
//   3. The project source folder, starting as the folder from step 1.
//   4. Connect with the agents (the agents panel).
// Opening a folder that has no `.allomos` starts at step 2 with that folder. A taskbag opened
// directly, or File → Connect with the Agents…, starts at step 3 or 4 (owner, 2026-10-12: the
// wizard replaces the page's own setup cards).

import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import { AgentsPanel } from "./AgentsPanel";
import type { ConnectResult } from "./AgentsPanel";
import type { Messenger } from "./messenger";
import { defaultSource, resumeAt, taskbagStep } from "./setupFlow";
import type { TaskbagStep } from "./setupFlow";
import { isProblem } from "./types";
import type { Opened, SetupState } from "./types";

/** What the wizard did with Git before connecting an agent, told on the next page. */
type GitNote =
  | { kind: "init"; folder: string }
  | { kind: "committed"; repo: string }
  | { kind: "missing"; folder: string };

type Step =
  /** Opening an existing taskbag at step 3 or 4: reading it first. */
  | { n: 0 }
  | { n: 1 }
  | { n: 2; folder: string; at: TaskbagStep; gitRoot: string | null; created: Opened | null }
  | { n: 3; opened: Opened; source: string; git: GitNote | null }
  /** `change`: the project folder was not just chosen in step 3, so it may be changed here. */
  | { n: 4; opened: Opened; project: string; git: GitNote | null; change: boolean };

/** The project's name: the folder holding `.allomos/`. */
function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  if (parts.at(-1) === "taskbag" && parts.at(-2) === ".allomos") parts.splice(-2);
  return parts.at(-1) ?? "Project";
}

export function SetupWizard({
  messenger,
  folder: startFolder,
  resume,
  onOpened,
  onSetup,
  onClose,
}: {
  messenger: Messenger;
  /** Start at step 2 with this folder (it has no `.allomos` yet). */
  folder?: string;
  /** Start at step 3 or 4 for the open taskbag at `root` (step 3 while no project folder is known). */
  resume?: { root: string; at: 3 | 4 };
  /** A taskbag is ready: the window shows it behind the dialog. `created`: it was just made. */
  onOpened: (o: Opened, created: boolean) => void;
  /** The project folder was recorded: the window's copy of the setup state is out of date. */
  onSetup: (state: SetupState) => void;
  /** The dialog closes. `connected`: step 4 was reached and closed (asked, or Not now). */
  onClose: (connected: ConnectResult | null) => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [step, setStep] = useState<Step>(resume ? { n: 0 } : { n: 1 });
  const [busy, setBusy] = useState(false);
  /** The bottom row of step 4, where the agents panel puts its buttons. */
  const [footer, setFooter] = useState<HTMLDivElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  /** Step 2 for `folder`: does it have `.allomos`? */
  const look = useCallback(
    async (folder: string) => {
      setError(null);
      try {
        const info = await messenger.inspectFolder(folder);
        const at = taskbagStep(info.inspection);
        if (at.kind === "not_a_folder") {
          setError(t("error.not_a_folder", { path: at.path }));
          return;
        }
        setStep({ n: 2, folder, at, gitRoot: info.git_root, created: null });
      } catch (e) {
        fail(e);
      }
    },
    // `fail` and `t` only word an error; looking again when the language changes is not needed.
    [messenger],
  );

  useEffect(() => {
    if (startFolder) void look(startFolder);
  }, [startFolder, look]);

  /** Keep the taskbag in Git without asking (owner, 2026-10-12): what was done, to tell on the
   * page. A failure is said, not a stop. */
  const keep = async (root: string, folder: string): Promise<GitNote | null> => {
    try {
      const kept = await messenger.keepInGit(root);
      if (kept.initialized) return { kind: "init", folder: kept.repo };
      if (kept.committed) return { kind: "committed", repo: kept.repo };
    } catch (e) {
      if (isProblem(e) && e.code === "git_missing") return { kind: "missing", folder };
      fail(e);
    }
    return null;
  };
  const keepRef = useRef(keep);
  keepRef.current = keep;

  // An open taskbag: commit it before connecting an agent, then show step 3 or 4.
  const resumeRoot = resume?.root;
  const resumeWanted = resume?.at;
  useEffect(() => {
    if (!resumeRoot || !resumeWanted) return;
    let live = true;
    void (async () => {
      try {
        const o = await messenger.openTaskbag(resumeRoot);
        const setup = await messenger.projectSetup(resumeRoot);
        const git = await keepRef.current(resumeRoot, setup.allomos_folder);
        if (!live) return;
        if (resumeAt(setup, resumeWanted) === 4 && setup.project_folder) {
          setStep({ n: 4, opened: o, project: setup.project_folder, git, change: true });
        } else {
          setStep({ n: 3, opened: o, source: defaultSource(setup), git });
        }
      } catch (e) {
        if (live) setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      }
    })();
    return () => {
      live = false;
    };
    // Read once per opening; the language only words an error.
  }, [messenger, resumeRoot, resumeWanted]);

  const choose = async () => {
    const path = await messenger.pickFolder(t("picker.title"));
    if (path) await look(path);
  };

  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const create = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "missing") return;
      const o = await messenger.createTaskbag(step.at.suggestedRoot, folderName(step.folder), language);
      onOpened(o, true);
      setStep({ ...step, at: { kind: "found", root: o.root, buttons: ["next"] }, created: o });
    });

  /** Step 2 → 3: open the taskbag, and offer its recorded project folder or the step-1 folder. */
  const toSource = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "found") return;
      const o = step.created ?? (await messenger.openTaskbag(step.at.root));
      if (!step.created) onOpened(o, false);
      const setup = await messenger.projectSetup(o.root);
      const git = await keep(o.root, setup.allomos_folder);
      setStep({ n: 3, opened: o, source: defaultSource(setup), git });
    });

  const pickSource = async () => {
    if (step.n !== 3) return;
    const path = await messenger.pickFolder(t("folder.pick"));
    if (path) setStep({ ...step, source: path });
  };

  /** Step 3 → 4: record the project source folder. */
  const toConnect = () =>
    run(async () => {
      if (step.n !== 3) return;
      const setup = await messenger.setProjectFolder(step.opened.root, step.source);
      onSetup(setup);
      // Commit the recorded project folder before the agent is connected (owner, 2026-10-12).
      // What happened with Git was told on step 3 already, unless Git started only now.
      const git = await keep(step.opened.root, setup.allomos_folder);
      const tell = git?.kind === "init" ? git : null;
      setStep({ n: 4, opened: step.opened, project: setup.project_folder ?? step.source, git: tell, change: false });
    });

  const dir = <code>.allomos</code>;

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard" role="dialog" aria-modal="true" aria-labelledby="wizard-title">
        <h2 id="wizard-title">{t("wizard.title")}</h2>
        {step.n > 0 && <p className="muted small">{t("wizard.step", { n: step.n })}</p>}

        <div className="dialog-body wizard-body">
          {step.n === 1 && (
            <>
              <h3>{t("wizard.what_title")}</h3>
              <p>{rich("wizard.what_body", { dir })}</p>
              <p className="muted">{rich("wizard.choose_hint", { dir })}</p>
              <div className="row start">
                <button type="button" className="primary" onClick={() => void choose()} disabled={busy}>
                  {t("wizard.choose")}
                </button>
              </div>
            </>
          )}

          {step.n === 2 && step.at.kind === "found" && (
            <>
              <h3>{t(step.created ? "wizard.created_title" : "wizard.found_title")}</h3>
              <p>{rich(step.created ? "wizard.created_body" : "wizard.found_body", { path: <code>{step.at.root}</code> })}</p>
              {/* No Change… here (owner, 2026-10-12): Previous goes back to choose another folder. */}
              <GitLine repo={step.gitRoot} />
            </>
          )}

          {step.n === 2 && step.at.kind === "missing" && (
            <>
              <h3>{t("wizard.missing_title")}</h3>
              <p>{rich("wizard.missing_body", { dir, folder: <code>{step.folder}</code> })}</p>
              <GitLine repo={step.gitRoot} />
              <div className="row start">
                <button type="button" className="primary" onClick={() => void create()} disabled={busy}>
                  {t("wizard.create")}
                </button>
              </div>
            </>
          )}

          {step.n === 3 && (
            <>
              <GitNoteView note={step.git} />
              <h3>{t("wizard.source_title")}</h3>
              <p>{t("wizard.source_body")}</p>
              <p>
                {rich("project.line", { path: <code>{step.source}</code> })}
              </p>
              <div className="row start">
                <button type="button" onClick={() => void pickSource()} disabled={busy}>
                  {t("wizard.source_other")}
                </button>
              </div>
            </>
          )}

          {step.n === 4 && (
            <>
              <GitNoteView note={step.git} />
              <AgentsPanel
                messenger={messenger}
                project={step.project}
                taskbag={step.opened.root}
                projectFixed={!step.change}
                actionsIn={footer}
                onProjectChange={async (folder) => onSetup(await messenger.setProjectFolder(step.opened.root, folder))}
                onDone={(result) => onClose(result)}
              />
            </>
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        {/* Step 4's buttons (Not now, Connect) sit where Cancel and Next are on the other steps. */}
        {step.n === 4 && <div className="row" ref={setFooter} />}

        {step.n !== 4 && (
          <div className="row">
            <button type="button" onClick={() => onClose(null)} disabled={busy}>
              {t("wizard.cancel")}
            </button>
            {step.n === 2 && (
              <button type="button" onClick={() => setStep({ n: 1 })} disabled={busy}>
                {t("wizard.back")}
              </button>
            )}
            {step.n === 3 && (
              <button type="button" onClick={() => void look(step.opened.root)} disabled={busy}>
                {t("wizard.back")}
              </button>
            )}
            {step.n === 2 && step.at.kind === "found" && (
              <button type="button" className="primary" onClick={() => void toSource()} disabled={busy}>
                {t("wizard.next")}
              </button>
            )}
            {step.n === 3 && (
              <button type="button" className="primary" onClick={() => void toConnect()} disabled={busy}>
                {t("wizard.next")}
              </button>
            )}
          </div>
        )}
      </section>
    </div>
  );
}

/** What the wizard did with Git: made a repository (say what Git is for), found none, or committed. */
function GitNoteView({ note }: { note: GitNote | null }) {
  const { t, rich } = useI18n();
  if (!note) return null;
  if (note.kind === "init") {
    // Allomos made a repository without asking: say what happened and how to learn more.
    return (
      <div className="git-note" role="status">
        <strong>{t("wizard.git_init_title")}</strong>
        <p>{rich("wizard.git_init", { folder: <code>{note.folder}</code> })}</p>
        <p className="muted">{t("wizard.git_help")}</p>
      </div>
    );
  }
  if (note.kind === "missing") {
    return (
      <div className="git-note warn" role="status">
        <strong>{t("wizard.git_missing_title")}</strong>
        <p>{rich("wizard.git_missing", { folder: <code>{note.folder}</code> })}</p>
        <p className="muted">{t("wizard.git_missing_help")}</p>
      </div>
    );
  }
  return <p className="banner ok">{rich("wizard.git_committed", { repo: <code>{note.repo}</code> })}</p>;
}

/** What happens with Git when the wizard goes on from step 2 (owner, 2026-10-12). */
function GitLine({ repo }: { repo: string | null }) {
  const { t, rich } = useI18n();
  return <p className="muted">{repo ? rich("wizard.git", { repo: <code>{repo}</code> }) : t("wizard.no_git")}</p>;
}
