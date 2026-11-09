// The app's one settings dialog (Task-A05-cookie962, approver 2026-11-08): a list of sections on
// the left in two groups, the chosen one on the right; Close and Esc close it. The **Allomos**
// group (AppSettings.tsx) holds what affects the whole app on this computer; the group named after
// the tab's taskbag holds that taskbag's own settings, saved in its project.json. Settings… in the
// menu opens it at the Allomos group; Settings beside the taskbag's name, at the taskbag's group.
// A tab without a taskbag shows only the Allomos group. Each section's top line says which group
// it is in. The taskbag's group (Task-A05-camera758, approver 2026-10-25): General shows the taskbag's name, with Rename (Task-A05-chalk937), its
// format and whether it is up to date, with Update the Taskbag… when it is older (the same check as
// the Update the Taskbag notice), and its location with Show in Finder (Task-A05-camera758). Tasks
// sets the language tasks are written in, has Translate the taskbag… (Task-A05-board831), and shows
// the project's label table, where its own labels can be changed (Task-A05-yeti951, decision 0020).
// Handoff holds the handoff rules (Task-A05-yacht960). Heartbeat holds the heartbeat interval and
// the lost limit (Task-A01-osprey500). Project folders binds, renames, locates
// and unbinds the taskbag's folders (Task-A05-plaza078). Agents shows in which folders each agent
// is set up, with Set up… for the others (Task-A05-bag963). Audit holds the identity level
// (Task-A01-frolic587, decision 0021 § 6); its other parts come with A01-tail719.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { AgentsSection } from "./AgentsSection";
import { AppGeneralSection, ThisComputerPart } from "./AppSettings";
import { HandoffSection } from "./HandoffSettings";
import { HeartbeatSection } from "./HeartbeatSettings";
import { AuditSection } from "./AuditSettings";
import { ProjectFolders } from "./ProjectFolders";
import { LANGUAGE_NAMES, useI18n, type Key } from "./i18n";
import { LanguageMenu } from "./LanguageMenu";
import { TranslateDialog } from "./TranslateDialog";
import { addStep, ladderChanged, ladderProblem, removeStep, renameStep, startLadder, type LadderStep } from "./ladderFlow";
import { BUILT_IN_NAMES, labelMeaning, labelTable, labelText } from "./stopFlow";
import type { AppSettingsMessenger, Messenger } from "./messenger";
import { middleEllipsis } from "./paths";
import { isProblem, type LabelEntry, type VersionCheck } from "./types";
import { movingTasks, updateNotice, upgradeFrom } from "./versionFlow";

const APP_SECTIONS = ["general", "computer"] as const;
const SECTIONS = ["general", "folders", "agents", "tasks", "handoff", "heartbeat", "audit"] as const;
type Section = (typeof SECTIONS)[number];
/** The chosen section: one of the Allomos group's or of the taskbag's. */
type Chosen = { group: "app"; section: (typeof APP_SECTIONS)[number] } | { group: "taskbag"; section: Section };

/** The location in its monospace font, shortened in the middle to the characters that fit its
 * width (measured, and again when the dialog's width changes). */
function Location({ root }: { root: string }) {
  const ref = useRef<HTMLElement>(null);
  const [fit, setFit] = useState(root.length);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      const probe = document.createElement("span");
      probe.textContent = "0".repeat(20);
      probe.style.visibility = "hidden";
      probe.style.position = "absolute";
      el.appendChild(probe);
      const per = probe.getBoundingClientRect().width / 20;
      probe.remove();
      const width = el.parentElement?.clientWidth ?? 0;
      if (per > 0 && width > 0) setFit(Math.max(12, Math.floor(width / per) - 1));
    };
    measure();
    const watch = new ResizeObserver(measure);
    if (el.parentElement) watch.observe(el.parentElement);
    return () => watch.disconnect();
  }, [root]);
  return (
    <code ref={ref} className="settings-location" title={root}>
      {middleEllipsis(root, fit)}
    </code>
  );
}

/** The name, and Rename: a field with Save and Cancel while renaming. */
function NameRow({ name, onRename }: { name: string; onRename: (name: string) => Promise<void> }) {
  const { t, problem } = useI18n();
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const save = async () => {
    if (draft === null) return;
    setBusy(true);
    setError(null);
    try {
      await onRename(draft);
      setDraft(null);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : String(e));
    } finally {
      setBusy(false);
    }
  };
  if (draft === null) {
    return (
      <div className="settings-row">
        <span className="settings-value">{name}</span>
        <button type="button" className="link reveal" onClick={() => setDraft(name)}>
          {t("settings.rename")}
        </button>
      </div>
    );
  }
  return (
    <form
      className="settings-row"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <input aria-label={t("settings.name")} value={draft} autoFocus onChange={(e) => setDraft(e.target.value)} />
      <button type="submit" className="primary" disabled={busy || draft.trim() === "" || draft.trim() === name}>
        {t("settings.save")}
      </button>
      <button type="button" onClick={() => setDraft(null)} disabled={busy}>
        {t("settings.cancel")}
      </button>
      {error && <p className="banner error">{error}</p>}
    </form>
  );
}

/** The format and whether it is up to date, with Update the Taskbag… when it is older. */
function FormatRow({ messenger, root, format, onUpdate }: { messenger: Messenger; root: string; format: string; onUpdate: () => void }) {
  const { t } = useI18n();
  const [check, setCheck] = useState<VersionCheck | null>(null);
  useEffect(() => {
    let live = true;
    void messenger.versionCheck(root).then((c) => live && setCheck(c), () => undefined);
    return () => {
      live = false;
    };
  }, [messenger, root]);
  const older = check?.move_in ?? null;
  const needsUpdate = movingTasks(updateNotice(check)) || upgradeFrom(check) !== null;
  return (
    <div className="settings-row">
      <code>{older?.found ?? format}</code>
      {/* No check (no agent connected yet) means nothing to update: the taskbag was read. */}
      {needsUpdate && check ? (
        <>
          <span className="muted small">{t("settings.format_older", { current: check.format_current })}</span>
          <button type="button" className="link reveal" onClick={onUpdate}>
            {t("settings.update_taskbag")}
          </button>
        </>
      ) : (
        <span className="muted small">{t("settings.format_current")}</span>
      )}
    </div>
  );
}

/** The language tasks are written in: follow the interface, or any language, from the searchable
 * language menu (Task-A05-board831). */
function TaskLanguage({ value, onChange }: { value: string | undefined; onChange: (value: string) => Promise<void> }) {
  const { t, language, problem } = useI18n();
  const [error, setError] = useState<string | null>(null);
  const current = value && value !== "ui" ? value : "ui";
  return (
    <>
      <LanguageMenu
        value={current}
        label={t("settings.task_language")}
        first={{ value: "ui", label: t("settings.follow_interface", { language: LANGUAGE_NAMES[language] }) }}
        onChange={(code) => {
          setError(null);
          onChange(code).catch((err) => setError(isProblem(err) ? problem(err) : String(err)));
        }}
      />
      <p className="muted small">{t("settings.task_language_note")}</p>
      {error && <p className="banner error">{error}</p>}
    </>
  );
}

/** Translate the taskbag… (Task-A05-board831): the button, its line, and the dialog. */
function Translate({ messenger, root }: { messenger: Messenger; root: string }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  return (
    <>
      <div className="settings-row">
        <button type="button" onClick={() => setOpen(true)}>
          {t("translate.button")}
        </button>
      </div>
      <p className="muted small">{t("translate.button_note")}</p>
      {said && (
        <p className="banner ok small" role="status">
          <span>{said}</span>
        </p>
      )}
      {open && (
        <TranslateDialog
          messenger={messenger}
          root={root}
          onClose={(message) => {
            setOpen(false);
            setSaid(message);
          }}
        />
      )}
    </>
  );
}

/** The project's label table (decision 0020, Task-A01-petal376): every label with its meaning, the
 * five built-in ones first and marked; then the project's own, with Change: rename, add and remove
 * them (the built-in labels cannot be changed). */
function LabelTable({ labels, onSave }: { labels: LabelEntry[]; onSave: (steps: LadderStep[]) => Promise<void> }) {
  const { t, problem } = useI18n();
  const [steps, setSteps] = useState<LadderStep[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const table = labelTable({ labels });
  const builtIn = table.filter((e) => e.built_in);
  const own = table.filter((e) => !e.built_in).map((e) => e.name);
  const row = (e: LabelEntry) => (
    <li key={e.name}>
      <span className={e.built_in ? "label-chip" : "label-chip own"}>{labelText(e.name, t)}</span>
      {e.built_in && <span className="muted small"> {t("settings.built_in")}</span>}
      {labelMeaning(e, t) && <div className="muted small">{labelMeaning(e, t)}</div>}
    </li>
  );
  const why = steps && ladderProblem(steps, BUILT_IN_NAMES);
  const save = async () => {
    if (!steps) return;
    setBusy(true);
    setError(null);
    try {
      await onSave(steps);
      setSteps(null);
    } catch (e) {
      setError(isProblem(e) ? problem(e) : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="label-table">
      <p className="muted small">{t("settings.labels_note")}</p>
      <ul>{builtIn.map(row)}</ul>
      <h4>{t("settings.own_labels")}</h4>
      {steps === null ? (
        <div className="settings-row">
          {own.length > 0 ? <ul>{table.filter((e) => !e.built_in).map(row)}</ul> : <span className="muted small">{t("settings.no_own_labels")}</span>}
          <button type="button" className="link reveal" onClick={() => setSteps(startLadder(own))}>
            {t("settings.change")}
          </button>
        </div>
      ) : (
        <div className="ladder-edit">
          <ol>
            {steps.map((s, i) => (
              <li key={i}>
                <input aria-label={t("settings.label_n", { n: i + 1 })} value={s.to} onChange={(e) => setSteps(renameStep(steps, i, e.target.value))} />
                <button type="button" className="link reveal" onClick={() => setSteps(removeStep(steps, i))}>
                  {t("settings.label_remove")}
                </button>
              </li>
            ))}
          </ol>
          <button type="button" className="link reveal" onClick={() => setSteps(addStep(steps))}>
            {t("settings.label_add")}
          </button>
          <p className="muted small">{t("settings.own_labels_note")}</p>
          {why && <p className="muted small">{t(`settings.labels_${why}` as Key)}</p>}
          {error && <p className="banner error">{error}</p>}
          <div className="row">
            <button type="button" className="primary" disabled={busy || why !== null || !ladderChanged(own, steps)} onClick={() => void save()}>
              {t("settings.save")}
            </button>
            <button type="button" disabled={busy} onClick={() => setSteps(null)}>
              {t("settings.cancel")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

/** The taskbag group's facts and what it does: the taskbag the tab shows. */
export interface TaskbagGroup {
  name: string;
  /** The project's label table. */
  labels: LabelEntry[];
  taskLanguage: string | undefined;
  /** `project.json`'s format, for example `allomos/5`. */
  format: string;
  root: string;
  onShowInFileManager: () => void;
  /** Rename the project; rejects with the Problem that refused it. */
  onRename: (name: string) => Promise<void>;
  /** Update the Taskbag…, as the notice's button. */
  onUpdate: () => void;
  /** Set up…: Connect with the Agents… for this project folder. */
  onSetUpAgent: (folder: string) => void;
  /** After a change saved here: read the taskbag again. */
  onChanged: () => Promise<void>;
}

export function SettingsDialog({
  messenger,
  app,
  taskbag,
  at,
  fileManagerKey,
  onClose,
}: {
  messenger: Messenger;
  /** The Allomos group's settings; none outside the desktop app. */
  app: Omit<AppSettingsMessenger, "takeSettingsRequest"> | undefined;
  /** The tab's taskbag; none on a blank tab. */
  taskbag: TaskbagGroup | null;
  /** Which group it opens at. */
  at: "app" | "taskbag";
  /** `top.show_in_finder`, `top.show_in_explorer` or `top.show_in_files`, for this computer. */
  fileManagerKey: Key;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [chosen, setChosen] = useState<Chosen>(() =>
    (at === "taskbag" && taskbag) || !app ? { group: "taskbag", section: "general" } : { group: "app", section: "general" },
  );
  // Esc closes the dialog, and only the dialog (not the task pane behind it).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // An open language menu, or a dialog opened from here, takes its own Esc.
      if ((e.target as Element | null)?.closest?.("[data-own-escape]")) return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);
  // The taskbag closed while the dialog was open: back to the Allomos group.
  const shown: Chosen = chosen.group === "taskbag" && !taskbag ? { group: "app", section: "general" } : chosen;
  const isChosen = (c: Chosen) => c.group === shown.group && c.section === shown.section;
  const item = (c: Chosen, label: string) => (
    <button
      key={`${c.group}.${c.section}`}
      type="button"
      className={isChosen(c) ? "chosen" : ""}
      aria-current={isChosen(c) ? "page" : undefined}
      onClick={() => setChosen(c)}
    >
      {label}
    </button>
  );

  return (
    <div className="overlay" role="presentation">
      <section className="dialog settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <h2 id="settings-title">{t("app_settings.title")}</h2>
        <div className="settings-body">
          <nav className="settings-sections" aria-label={t("settings.sections")}>
            {app && (
              <div className="settings-group" role="group" aria-labelledby="settings-group-app">
                <div className="settings-group-name" id="settings-group-app">
                  Allomos
                </div>
                {item({ group: "app", section: "general" }, t("settings.general"))}
                {item({ group: "app", section: "computer" }, t("this_computer.title"))}
              </div>
            )}
            {taskbag && (
              <div className="settings-group" role="group" aria-labelledby="settings-group-taskbag">
                <div className="settings-group-name" id="settings-group-taskbag" title={taskbag.name}>
                  {taskbag.name}
                </div>
                {SECTIONS.map((s) => item({ group: "taskbag", section: s }, t(`settings.${s}` as Key)))}
              </div>
            )}
          </nav>
          <div className="settings-section">
            <p className={shown.group === "app" ? "settings-scope app" : "settings-scope taskbag"}>
              {shown.group === "app" ? t("settings.scope_app") : t("settings.scope_taskbag")}
            </p>
            {app && shown.group === "app" && shown.section === "general" && <AppGeneralSection settings={app} />}
            {app && shown.group === "app" && shown.section === "computer" && <ThisComputerPart settings={app} />}
            {taskbag && shown.group === "taskbag" && (
              <TaskbagSection messenger={messenger} taskbag={taskbag} section={shown.section} fileManagerKey={fileManagerKey} />
            )}
          </div>
        </div>
        <div className="row">
          <button type="button" className="primary" onClick={onClose}>
            {t("settings.close")}
          </button>
        </div>
      </section>
    </div>
  );
}

/** One section of the taskbag's group: its content as before the two dialogs became one. */
function TaskbagSection({ messenger, taskbag, section, fileManagerKey }: { messenger: Messenger; taskbag: TaskbagGroup; section: Section; fileManagerKey: Key }) {
  const { t } = useI18n();
  const { name, labels, taskLanguage, format, root, onShowInFileManager, onRename, onUpdate, onSetUpAgent, onChanged } = taskbag;
  return (
    <>
      {section === "general" && (
        <>
          <h3>{t("settings.name")}</h3>
          <NameRow name={name} onRename={onRename} />
          <h3>{t("settings.format")}</h3>
          <FormatRow messenger={messenger} root={root} format={format} onUpdate={onUpdate} />
          <h3>{t("settings.location")}</h3>
          <Location root={root} />
          <button type="button" className="link reveal" onClick={onShowInFileManager}>
            {t(fileManagerKey)}
          </button>
        </>
      )}
      {section === "tasks" && (
        <>
          <h3>{t("settings.task_language")}</h3>
          <TaskLanguage
            value={taskLanguage}
            onChange={async (value) => {
              await messenger.setTaskLanguage(root, value);
              await onChanged();
            }}
          />
          <Translate messenger={messenger} root={root} />
          <h3>{t("settings.labels")}</h3>
          <LabelTable
            labels={labels}
            onSave={async (steps) => {
              await messenger.changeLadder(root, steps);
              await onChanged();
            }}
          />
        </>
      )}
      {section === "agents" && <AgentsSection messenger={messenger} root={root} onSetUp={onSetUpAgent} />}
      {section === "handoff" && <HandoffSection messenger={messenger} root={root} />}
      {section === "heartbeat" && <HeartbeatSection messenger={messenger} root={root} onChanged={onChanged} />}
      {section === "audit" && <AuditSection messenger={messenger} root={root} fileManagerKey={fileManagerKey} />}
      {section === "folders" && <ProjectFolders messenger={messenger} root={root} fileManagerKey={fileManagerKey} onChanged={onChanged} />}
    </>
  );
}
