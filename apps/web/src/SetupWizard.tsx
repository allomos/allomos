// The setup wizard (Task-A05-03): a dialog that sets up a taskbag in four steps.
//   1. What a taskbag is, and choose the folder where `.allomos` goes.
//   2. That folder has `.allomos`: only Change. It has none: Create.
//   3. The project source folder, starting as the folder from step 1.
//   4. Connect with the agents (the agents panel).
// Opening a folder that has no `.allomos` starts at step 2 with that folder.

import { useCallback, useEffect, useState } from "react";
import { useI18n } from "./i18n";
import { AgentsPanel } from "./AgentsPanel";
import type { ConnectResult } from "./AgentsPanel";
import type { Messenger } from "./messenger";
import { defaultSource, taskbagStep } from "./setupFlow";
import type { TaskbagStep } from "./setupFlow";
import { isProblem } from "./types";
import type { Opened, SetupState } from "./types";

/** What the wizard did with Git on step 2's Next, told on step 3. */
type GitNote =
  | { kind: "init"; folder: string }
  | { kind: "committed"; repo: string }
  | { kind: "missing"; folder: string };

type Step =
  | { n: 1 }
  | { n: 2; folder: string; at: TaskbagStep; gitRoot: string | null; created: Opened | null }
  | { n: 3; opened: Opened; source: string; git: GitNote | null }
  | { n: 4; opened: Opened; project: string };

/** The project's name: the folder holding `.allomos/`. */
function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  if (parts.at(-1) === "taskbag" && parts.at(-2) === ".allomos") parts.splice(-2);
  return parts.at(-1) ?? "Project";
}

export function SetupWizard({
  messenger,
  folder: startFolder,
  onOpened,
  onSetup,
  onClose,
}: {
  messenger: Messenger;
  /** Start at step 2 with this folder (it has no `.allomos` yet). */
  folder?: string;
  /** A taskbag is ready: the window shows it behind the dialog. `created`: it was just made. */
  onOpened: (o: Opened, created: boolean) => void;
  /** The project folder was recorded: the window's copy of the setup state is out of date. */
  onSetup: (state: SetupState) => void;
  /** The dialog closes. `connected`: step 4 was reached and closed (asked, or Not now). */
  onClose: (connected: ConnectResult | null) => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [step, setStep] = useState<Step>({ n: 1 });
  const [busy, setBusy] = useState(false);
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
      setStep({ ...step, at: { kind: "found", root: o.root, buttons: ["change"] }, created: o });
    });

  /** Step 2 → 3: open the taskbag, and offer its recorded project folder or the step-1 folder. */
  const toSource = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "found") return;
      const o = step.created ?? (await messenger.openTaskbag(step.at.root));
      if (!step.created) onOpened(o, false);
      const setup = await messenger.projectSetup(o.root);
      // Keep the taskbag in Git without asking (owner, 2026-10-12): a failure is said, not a stop.
      let git: GitNote | null = null;
      try {
        const kept = await messenger.keepInGit(o.root);
        if (kept.initialized) git = { kind: "init", folder: kept.repo };
        else if (kept.committed) git = { kind: "committed", repo: kept.repo };
      } catch (e) {
        if (isProblem(e) && e.code === "git_missing") git = { kind: "missing", folder: setup.allomos_folder };
        else fail(e);
      }
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
      // Without Git, step 3 has already said so.
      await messenger.keepInGit(step.opened.root).catch((e) => {
        if (!(isProblem(e) && e.code === "git_missing")) fail(e);
      });
      setStep({ n: 4, opened: step.opened, project: setup.project_folder ?? step.source });
    });

  const dir = <code>.allomos</code>;

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard" role="dialog" aria-modal="true" aria-labelledby="wizard-title">
        <h2 id="wizard-title">{t("wizard.title")}</h2>
        <p className="muted small">{t("wizard.step", { n: step.n })}</p>

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
              <GitLine repo={step.gitRoot} />
              <div className="row start">
                <button type="button" onClick={() => void choose()} disabled={busy}>
                  {t("wizard.change")}
                </button>
              </div>
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
              {step.git?.kind === "init" && (
                // Allomos made a repository without asking: say what happened and how to learn more.
                <div className="git-note" role="status">
                  <strong>{t("wizard.git_init_title")}</strong>
                  <p>{rich("wizard.git_init", { folder: <code>{step.git.folder}</code> })}</p>
                  <p className="muted">{t("wizard.git_help")}</p>
                </div>
              )}
              {step.git?.kind === "missing" && (
                <div className="git-note warn" role="status">
                  <strong>{t("wizard.git_missing_title")}</strong>
                  <p>{rich("wizard.git_missing", { folder: <code>{step.git.folder}</code> })}</p>
                  <p className="muted">{t("wizard.git_missing_help")}</p>
                </div>
              )}
              {step.git?.kind === "committed" && (
                <p className="banner ok">{rich("wizard.git_committed", { repo: <code>{step.git.repo}</code> })}</p>
              )}
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
            <AgentsPanel
              messenger={messenger}
              project={step.project}
              taskbag={step.opened.root}
              projectFixed
              onDone={(result) => onClose(result)}
            />
          )}
        </div>

        {error && <p className="banner error">{error}</p>}

        {step.n !== 4 && (
          <div className="row">
            <button type="button" onClick={() => onClose(null)} disabled={busy}>
              {t("wizard.cancel")}
            </button>
            {step.n === 2 && !step.created && (
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

/** What happens with Git when the wizard goes on from step 2 (owner, 2026-10-12). */
function GitLine({ repo }: { repo: string | null }) {
  const { t, rich } = useI18n();
  return <p className="muted">{repo ? rich("wizard.git", { repo: <code>{repo}</code> }) : t("wizard.no_git")}</p>;
}
