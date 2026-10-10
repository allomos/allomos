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
import type { Opened } from "./types";

type Step =
  | { n: 1 }
  | { n: 2; folder: string; at: TaskbagStep; gitRoot: string | null; created: Opened | null }
  | { n: 3; opened: Opened; source: string }
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
  onClose,
}: {
  messenger: Messenger;
  /** Start at step 2 with this folder (it has no `.allomos` yet). */
  folder?: string;
  /** A taskbag is ready: the window shows it behind the dialog. `created`: it was just made. */
  onOpened: (o: Opened, created: boolean) => void;
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
      setStep({ n: 3, opened: o, source: defaultSource(setup) });
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
              <h3>{t("wizard.found_title")}</h3>
              <p>{rich("wizard.found_body", { path: <code>{step.at.root}</code> })}</p>
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
              {step.gitRoot && <p className="muted">{rich("wizard.git", { repo: <code>{step.gitRoot}</code> })}</p>}
              <div className="row start">
                <button type="button" className="primary" onClick={() => void create()} disabled={busy}>
                  {t("wizard.create")}
                </button>
              </div>
            </>
          )}

          {step.n === 3 && (
            <>
              <h3>{t("wizard.source_title")}</h3>
              <p>{t("wizard.source_body")}</p>
              <p>
                <code>{step.source}</code>
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
              onProjectChange={async (folder) => {
                await messenger.setProjectFolder(step.opened.root, folder);
              }}
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
