// The taskbag's settings (Task-A05-camera758, approver 2026-10-25): opened from Settings beside the
// taskbag's name in the window header. A list of sections on the left, the chosen one on the right;
// Close and Esc close it. General shows the taskbag's name, with Rename (Task-A05-chalk937), its
// format and whether it is up to date, with Update the Taskbag… when it is older (the same check as
// the Update the Taskbag notice), and its location with Show in Finder (Task-A05-camera758). The
// other sections (Project folders, Tasks, Handoff, Agents) come with their own tasks.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useI18n, type Key } from "./i18n";
import type { Messenger } from "./messenger";
import { middleEllipsis } from "./paths";
import { isProblem, type VersionCheck } from "./types";
import { movingTasks, updateNotice } from "./versionFlow";

const SECTIONS = ["general"] as const;
type Section = (typeof SECTIONS)[number];

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
  const needsUpdate = movingTasks(updateNotice(check));
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

export function TaskbagSettings({
  messenger,
  name,
  format,
  root,
  fileManagerKey,
  onShowInFileManager,
  onRename,
  onUpdate,
  onClose,
}: {
  messenger: Messenger;
  name: string;
  /** `project.json`'s format, for example `allomos/5`. */
  format: string;
  root: string;
  /** `top.show_in_finder`, `top.show_in_explorer` or `top.show_in_files`, for this computer. */
  fileManagerKey: Key;
  onShowInFileManager: () => void;
  /** Rename the project; rejects with the Problem that refused it. */
  onRename: (name: string) => Promise<void>;
  /** Update the Taskbag…, as the notice's button. */
  onUpdate: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [section, setSection] = useState<Section>("general");
  // Esc closes the dialog, and only the dialog (not the task pane behind it).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  return (
    <div className="overlay" role="presentation">
      <section className="dialog settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <h2 id="settings-title">{t("settings.title", { name })}</h2>
        <div className="settings-body">
          <nav className="settings-sections" aria-label={t("settings.sections")}>
            {SECTIONS.map((s) => (
              <button key={s} type="button" className={s === section ? "chosen" : ""} aria-current={s === section ? "page" : undefined} onClick={() => setSection(s)}>
                {t(`settings.${s}` as Key)}
              </button>
            ))}
          </nav>
          <div className="settings-section">
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
