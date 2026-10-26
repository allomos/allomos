// The setup wizard (Task-A05-03): a dialog that sets up a taskbag in four steps.
//   1. What a taskbag is, and choose the folder where `.allomos` goes.
//   2. That folder has `.allomos` (or Create made it): Next. It has none: Create, with the new
//      taskbag's task language chosen first (Task-A05-bingo729). Previous goes back.
//   3. The project folders: the main one, starting as the folder from step 1, and any others
//      (Task-A05-king717), all bound on Next.
//   4. Connect with the agents (the agents panel).
// Opening a folder that has no `.allomos` starts at step 2 with that folder. A taskbag opened
// directly, or File → Connect with the Agents…, starts at step 3 or 4 (owner, 2026-10-12: the
// wizard replaces the page's own setup cards).
// The wizard runs no Git (decision 0017): the agent connected in step 4 asks the person before
// setting up Git, and commits the taskbag itself.

import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { LANGUAGE_NAMES, useI18n } from "./i18n";
import { AgentsPanel } from "./AgentsPanel";
import { LanguageMenu } from "./LanguageMenu";
import { MoveCardsPanel } from "./MoveCardsDialog";
import { cardRows, tasksInStart } from "./moveCardsFlow";
import type { CardRow, MoveIn } from "./moveCardsFlow";
import { updateNotice } from "./versionFlow";
import type { ConnectResult } from "./AgentsPanel";
import type { Messenger } from "./messenger";
import { addFolder, defaultSource, holdsTaskbag, resumeAt, stepFourPage, taskbagStep, withMain } from "./setupFlow";
import type { StepFourPage, TaskbagStep } from "./setupFlow";
import { isProblem } from "./types";
import type { Opened, SetupState } from "./types";

type Step =
  /** Opening an existing taskbag at step 3 or 4: reading it first. */
  | { n: 0 }
  | { n: 1 }
  /** `newer`: the found taskbag needs a newer Allomos; Next is disabled (Task-A01-25). `older`: it
   * was written by an older Allomos; Next becomes Update the taskbag… (Task-A01-45). */
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
  | {
      n: 3;
      opened: Opened;
      source: string;
      created?: boolean;
      /** The other folders to bind on Next (Task-A05-king717). */
      more: string[];
      /** The folder holding `.allomos/`: the main folder is marked while it is this one. */
      home: string;
    }
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
  /** `project`: step 4 for this folder, without Change… (the settings' Agents section, Task-A05-bag963). */
  resume?: { root: string; at: 3 | 4; project?: string };
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
  /** Step 2's Task language for a taskbag it creates: `ui` (follow the interface) or a code. */
  const [taskLanguage, setTaskLanguage] = useState("ui");
  /** The bottom row of step 4, where the agents panel puts its buttons. */
  const [footer, setFooter] = useState<HTMLDivElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  /** Step 4 shows what the project needs (Task-A05-09): worked out when step 4 is reached. */
  const [four, setFour] = useState<{
    page: StepFourPage;
    setup: SetupState | null;
    found: CardRow[];
    moveIn: MoveIn | null;
    tasks: number;
    /** The instruction files whose Allomos section is out of date (Task-A03-27). */
    section: string | null;
  } | null>(null);
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
        // An older taskbag already renamed aside: its tasks are moved in from the same panel as
        // card files (Task-A01-45).
        const { moveIn } = tasksInStart(check);
        const notice = updateNotice(check);
        const section = notice?.kind === "section" ? notice.files : null;
        const page = stepFourPage(setup, notice !== null, found.length, false, moveIn !== null);
        setFour({ page, setup, found, moveIn, tasks: fresh.loaded.taskbag?.tasks.length ?? 0, section });
      } catch {
        // Unsure what the project needs: the agents page, as before.
        if (live) setFour({ page: "agents", setup: null, found: [], moveIn: null, tasks: 0, section: null });
      }
    })();
    return () => {
      live = false;
    };
  }, [messenger, fourRoot]);
  // Set up… for one folder (the settings' Agents section) is always the agents page.
  const fourPage: StepFourPage | null = anyway || resume?.project ? "agents" : (four?.page ?? null);

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
  const resumeProject = resume?.project;
  useEffect(() => {
    if (!resumeRoot || !resumeWanted) return;
    let live = true;
    void (async () => {
      try {
        const o = await messenger.openTaskbag(resumeRoot);
        const setup = await messenger.projectSetup(resumeRoot);
        if (!live) return;
        if (resumeWanted === 4 && resumeProject) {
          setStep({ n: 4, opened: o, project: resumeProject, change: false });
        } else if (resumeAt(setup, resumeWanted) === 4 && setup.project_folder) {
          setStep({ n: 4, opened: o, project: setup.project_folder, change: true });
        } else {
          setStep({ n: 3, opened: o, source: defaultSource(setup), more: [], home: setup.allomos_folder });
        }
      } catch (e) {
        if (live) setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      }
    })();
    return () => {
      live = false;
    };
    // Read once per opening; the language only words an error.
  }, [messenger, resumeRoot, resumeWanted, resumeProject]);

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
   * the other steps; approver, 2026-10-17) makes it, saves the chosen task language into it (as
   * `alm task-language` does; Follow the interface, the default, writes nothing, Task-A05-bingo729),
   * and goes on to step 3. */
  const create = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "missing") return;
      let o = await messenger.createTaskbag(step.at.suggestedRoot, folderName(step.folder), language);
      if (taskLanguage !== "ui") {
        await messenger.setTaskLanguage(o.root, taskLanguage);
        o = await messenger.openTaskbag(o.root);
      }
      onOpened(o, true);
      const setup = await messenger.projectSetup(o.root);
      setStep({ n: 3, opened: o, source: defaultSource(setup), created: true, more: [], home: setup.allomos_folder });
    });

  /** Step 2 → 3: open the taskbag, and offer its recorded project folder or the step-1 folder. */
  const toSource = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "found") return;
      const o = step.created ?? (await messenger.openTaskbag(step.at.root));
      if (!step.created) onOpened(o, false);
      const setup = await messenger.projectSetup(o.root);
      setStep({ n: 3, opened: o, source: defaultSource(setup), more: [], home: setup.allomos_folder });
    });

  /** Step 2 → 3 for an older Allomos's taskbag (Task-A01-45): rename it aside and create a new
   * one; step 4's Update the taskbag then asks the agent to move the tasks across. */
  const moveIn = () =>
    run(async () => {
      if (step.n !== 2 || step.at.kind !== "found") return;
      const o = await messenger.moveTaskbagAside(step.at.root);
      onOpened(o, false);
      const setup = await messenger.projectSetup(o.root);
      setStep({ n: 3, opened: o, source: defaultSource(setup), more: [], home: setup.allomos_folder });
    });

  const pickSource = async () => {
    if (step.n !== 3) return;
    const path = await messenger.pickFolder(t("folder.pick"));
    if (path) setStep({ ...step, source: path, more: withMain(path, step.more) });
  };

  /** Step 3's Add Another Folder… (Task-A05-king717): nothing is bound until Next. */
  const addMore = async () => {
    if (step.n !== 3) return;
    const path = await messenger.pickFolder(t("folders.bind_title"));
    if (path) setStep({ ...step, more: addFolder(step.source, step.more, path) });
  };

  /** Step 3 → 4: record the main project folder, then bind each other folder (the same store code
   * as `alm folder add` and Settings → Project folders). A folder that could not be bound stays
   * listed, with the error; those bound already leave the list, so Next does not bind them twice. */
  const toConnect = () =>
    run(async () => {
      if (step.n !== 3) return;
      let setup = await messenger.setProjectFolder(step.opened.root, step.source);
      onSetup(setup);
      const left = [...step.more];
      while (left.length > 0) {
        try {
          await messenger.bindFolder(step.opened.root, left[0]);
        } catch (e) {
          setStep({ ...step, more: left });
          throw e;
        }
        left.shift();
      }
      if (step.more.length > 0) {
        setup = await messenger.projectSetup(step.opened.root);
        onSetup(setup);
      }
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
              {/* Only when the wizard creates the taskbag (approver, 2026-10-26, Task-A05-bingo729). */}
              <h3>{t("settings.task_language")}</h3>
              <LanguageMenu
                value={taskLanguage}
                label={t("settings.task_language")}
                first={{ value: "ui", label: t("settings.follow_interface", { language: LANGUAGE_NAMES[language] }) }}
                disabled={busy}
                onChange={(code) => setTaskLanguage(code)}
              />
              <p className="muted small">{t("wizard.task_language_hint")}</p>
            </>
          )}

          {step.n === 3 && (
            <>
              {step.created && (
                <>
                  <p className="banner ok" role="status">
                    {/* One run of text: the banner lays out its children side by side. */}
                    <span>{rich("wizard.created_body", { path: <code>{step.opened.root}</code> })}</span>
                  </p>
                  <p className="muted">{t("wizard.git_agent")}</p>
                </>
              )}
              {/* Design B (approver, 2026-10-26, Task-A05-king717): a list of folders, the first
                  the main one; more can be added now or later in Settings. */}
              <h3>{t("wizard.source_title")}</h3>
              <p>{t("wizard.source_body")}</p>
              <ul className="wizard-folders">
                <li>
                  <code>{step.source}</code>
                  {holdsTaskbag(step.source, step.home) && <span className="folder-badge ok">{t("wizard.folders_holds")}</span>}
                  <button type="button" className="link" onClick={() => void pickSource()} disabled={busy}>
                    {t("wizard.source_other")}
                  </button>
                </li>
                {step.more.map((path) => (
                  <li key={path}>
                    <code>{path}</code>
                    <button
                      type="button"
                      className="link"
                      onClick={() => setStep({ ...step, more: step.more.filter((m) => m !== path) })}
                      disabled={busy}
                    >
                      {t("wizard.folders_remove")}
                    </button>
                  </li>
                ))}
              </ul>
              <div className="row start">
                <button type="button" onClick={() => void addMore()} disabled={busy}>
                  {t("folders.bind_another")}
                </button>
              </div>
              <p className="muted small">{t("wizard.folders_hint")}</p>
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
                moveIn={four.moveIn}
                section={four.section}
                noTasks={four.tasks === 0}
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
            {/* Previous: back to step 1 after Create taskbag (approver, 2026-10-17); otherwise to step 2. */}
            {step.n === 3 && (
              <button type="button" onClick={() => (step.created ? setStep({ n: 1 }) : void look(step.opened.root))} disabled={busy}>
                {t("wizard.back")}
              </button>
            )}
            {step.n === 2 && step.at.kind === "found" && step.older && (
              <button type="button" className="primary" onClick={() => void moveIn()} disabled={busy}>
                {t("cards.button")}
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
