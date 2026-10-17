// The setup wizard (Task-A05-03): a dialog that sets up a taskbag in four steps.
//   1. What a taskbag is, and choose the folder where `.allomos` goes.
//   2. That folder has `.allomos` (or Create made it): Next. It has none: Create. Previous goes back.
//   3. The project source folder, starting as the folder from step 1.
//   4. Connect with the agents (the agents panel).
// Opening a folder that has no `.allomos` starts at step 2 with that folder. A taskbag opened
// directly, or File → Connect with the Agents…, starts at step 3 or 4 (owner, 2026-10-12: the
// wizard replaces the page's own setup cards).
// The wizard runs no Git (decision 0017): the agent connected in step 4 asks the person before
// setting up Git, and commits the taskbag itself.

import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "./i18n";
import { AgentsPanel } from "./AgentsPanel";
import { MoveCardsPanel } from "./MoveCardsDialog";
import { cardRows } from "./moveCardsFlow";
import type { CardRow } from "./moveCardsFlow";
import { updateNotice } from "./versionFlow";
import type { ConnectResult } from "./AgentsPanel";
import type { Messenger } from "./messenger";
import { defaultSource, resumeAt, stepFourPage, taskbagStep } from "./setupFlow";
import type { StepFourPage, TaskbagStep } from "./setupFlow";
import { isProblem } from "./types";
import type { Opened, SetupState } from "./types";

type Step =
  /** Opening an existing taskbag at step 3 or 4: reading it first. */
  | { n: 0 }
  | { n: 1 }
  /** `newer`: the found taskbag needs a newer Allomos; Next is disabled (Task-A01-25). `older`: it
   * was written by an older Allomos; Next becomes Move the tasks in… (Task-A01-45). */
  | {
      n: 2;
      folder: string;
      at: TaskbagStep;
      created: Opened | null;
      newer?: { found: string; supported: string } | null;
      older?: { found: string } | null;
    }
  /** `created`: the taskbag was just made in step 2, so step 3 says where it is, with the Git note
   * (approver, 2026-10-17). */
  | { n: 3; opened: Opened; source: string; created?: boolean }
  /** `change`: the project folder was not just chosen in step 3, so it may be changed here. */
  | { n: 4; opened: Opened; project: string; change: boolean };

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
  onElsewhere,
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
  /** The chosen taskbag is already open in another window, which came forward (Task-A01-27). */
  onElsewhere: () => void;
}) {
  const { t, rich, problem, language } = useI18n();
  const [step, setStep] = useState<Step>(resume ? { n: 0 } : { n: 1 });
  const [busy, setBusy] = useState(false);
  /** The bottom row of step 4, where the agents panel puts its buttons. */
  const [footer, setFooter] = useState<HTMLDivElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  /** Step 4 shows what the project needs (Task-A05-09): worked out when step 4 is reached. */
  const [four, setFour] = useState<{ page: StepFourPage; setup: SetupState | null; found: CardRow[]; tasks: number } | null>(null);
  /** Connect an agent anyway: the agents page, whatever the project needs. */
  const [anyway, setAnyway] = useState(false);
  const fourRoot = step.n === 4 ? step.opened.root : null;
  useEffect(() => {
    setFour(null);
    setAnyway(false);
    if (!fourRoot) return;
    let live = true;
    void (async () => {
      try {
        const [setup, check, fresh] = await Promise.all([
          messenger.projectSetup(fourRoot),
          messenger.versionCheck(fourRoot).catch(() => null),
          messenger.openTaskbag(fourRoot),
        ]);
        if (!live) return;
        const found = cardRows(fresh);
        const page = stepFourPage(setup, updateNotice(check) !== null, found.length, false);
        setFour({ page, setup, found, tasks: fresh.loaded.taskbag?.tasks.length ?? 0 });
      } catch {
        // Unsure what the project needs: the agents page, as before.
        if (live) setFour({ page: "agents", setup: null, found: [], tasks: 0 });
      }
    })();
    return () => {
      live = false;
    };
  }, [messenger, fourRoot]);
  const fourPage: StepFourPage | null = anyway ? "agents" : (four?.page ?? null);

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
        // Already open in another window: that window comes forward instead (owner, 2026-10-12).
        if (at.kind === "found" && (await messenger.showProjectElsewhere(at.root).catch(() => false))) {
          onElsewhere();
          return;
        }
        let newer: { found: string; supported: string } | null = null;
        let older: { found: string } | null = null;
        if (at.kind === "found") {
          const f = await messenger.taskbagFormat(at.root).catch(() => null);
          if (f?.format.state === "newer") newer = { found: f.format.found, supported: f.current };
          if (f?.format.state === "older") older = { found: f.format.found };
        }
        setStep({ n: 2, folder, at, created: null, newer, older });
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

  // An open taskbag: show step 3 or 4.
  const resumeRoot = resume?.root;
  const resumeWanted = resume?.at;
  useEffect(() => {
    if (!resumeRoot || !resumeWanted) return;
    let live = true;
    void (async () => {
      try {
        const o = await messenger.openTaskbag(resumeRoot);
        const setup = await messenger.projectSetup(resumeRoot);
        if (!live) return;
        if (resumeAt(setup, resumeWanted) === 4 && setup.project_folder) {
          setStep({ n: 4, opened: o, project: setup.project_folder, change: true });
        } else {
          setStep({ n: 3, opened: o, source: defaultSource(setup) });
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

  /** Step 2 → 3 for a folder with no taskbag: Create taskbag (in the bottom row, where Next is on
   * the other steps; approver, 2026-10-17) makes it and goes on to step 3. */
  const create = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "missing") return;
      const o = await messenger.createTaskbag(step.at.suggestedRoot, folderName(step.folder), language);
      onOpened(o, true);
      const setup = await messenger.projectSetup(o.root);
      setStep({ n: 3, opened: o, source: defaultSource(setup), created: true });
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

  /** Step 2 → 3 for an older Allomos's taskbag (Task-A01-45): rename it aside and create a new
   * one; step 4's request then asks the agent to move the tasks across. */
  const moveIn = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "found") return;
      const o = await messenger.moveTaskbagAside(step.at.root);
      onOpened(o, false);
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
      onSetup(setup);
      setStep({ n: 4, opened: step.opened, project: setup.project_folder ?? step.source, change: false });
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
              <p className="muted">{t("wizard.git_agent")}</p>
              {step.older && <p className="banner update">{t("move.wizard_old", { found: step.older.found })}</p>}
              {step.newer && (
                <p className="banner error" role="alert">
                  {problem({ code: "taskbag_newer", params: step.newer })}{" "}
                  <button type="button" className="link" onClick={() => messenger.openDownloadPage().catch(fail)}>
                    {t("update.download")}
                  </button>
                </p>
              )}
            </>
          )}

          {step.n === 2 && step.at.kind === "missing" && (
            <>
              <h3>{t("wizard.missing_title")}</h3>
              <p>{rich("wizard.missing_body", { dir, folder: <code>{step.folder}</code> })}</p>
            </>
          )}

          {step.n === 3 && (
            <>
              {step.created && (
                <>
                  <p className="banner update" role="status">
                    {/* One run of text: the banner lays out its children side by side. */}
                    <span>{rich("wizard.created_body", { path: <code>{step.opened.root}</code> })}</span>
                  </p>
                  <p className="muted">{t("wizard.git_agent")}</p>
                </>
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

          {step.n === 4 && fourPage === null && (
            <p className="muted searching" role="status">
              <span className="spinner" aria-hidden="true" /> {t("wizard.checking")}
            </p>
          )}

          {step.n === 4 && fourPage === "cards" && four && (
            <>
              <h3>{t("cards.title")}</h3>
              <MoveCardsPanel
                messenger={messenger}
                root={step.opened.root}
                found={four.found}
                actionsIn={footer}
                cancelLabel={t("agents.later")}
                onClose={(message) => onClose({ message })}
                extra={
                  <p>
                    <button type="button" className="link" onClick={() => setAnyway(true)}>
                      {t("wizard.connect_anyway")}
                    </button>
                  </p>
                }
              />
            </>
          )}

          {step.n === 4 && fourPage === "done" && four && (
            <>
              <h3>{t("done.title")}</h3>
              <p>{t("done.body")}</p>
              <p>{rich("project.line", { path: <code>{step.project}</code> })}</p>
              <p className="muted">{t("agents.taskbag", { path: step.opened.root })}</p>
              <p>
                {t("agentline.line", {
                  state: `Codex ${four.setup?.agents.codex ? "✓" : "✗"}  Claude Code ${four.setup?.agents.claude ? "✓" : "✗"}`,
                })}
              </p>
              <p>{t("done.tasks", { count: four.tasks })}</p>
              <p>
                <button type="button" className="link" onClick={() => setAnyway(true)}>
                  {t("wizard.connect_anyway")}
                </button>
              </p>
              {footer &&
                createPortal(
                  <button type="button" className="primary" onClick={() => onClose({ message: null })}>
                    {t("manual.done")}
                  </button>,
                  footer,
                )}
            </>
          )}

          {step.n === 4 && fourPage === "agents" && (
            <>
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

        {/* Step 4's buttons (Not now, Connect; Open; Done) sit where Cancel and Next are on the other steps. */}
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
            {step.n === 2 && step.at.kind === "found" && step.older && (
              <button type="button" className="primary" onClick={() => void moveIn()} disabled={busy}>
                {t("move.button")}
              </button>
            )}
            {step.n === 2 && step.at.kind === "found" && !step.older && (
              <button type="button" className="primary" onClick={() => void toSource()} disabled={busy || Boolean(step.newer)}>
                {t("wizard.next")}
              </button>
            )}
            {step.n === 2 && step.at.kind === "missing" && (
              <button type="button" className="primary" onClick={() => void create()} disabled={busy}>
                {t("wizard.create")}
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
