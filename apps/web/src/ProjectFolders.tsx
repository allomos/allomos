// The Project folders section of the taskbag's settings (Task-A05-plaza078; decision 0019): one
// row per bound folder with its path, how many tasks name it and whether its AGENTS.md is set up;
// Add a Project Folder… (Add Another Folder… once one is bound, Task-A05-opal549), Rename, Locate… for a folder not found, Show in Finder, and Unbind, which first
// only says what it will do (approver, 2026-10-25). The app calls the same store code as
// `alm folder`.

import { useEffect, useState } from "react";
import type { FolderInfo } from "./folderFlow";
import { useI18n, type Key } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";

export function ProjectFolders({
  messenger,
  root,
  fileManagerKey,
  onChanged,
}: {
  messenger: Messenger;
  root: string;
  fileManagerKey: Key;
  /** After a change: read the taskbag again (tasks may have changed). */
  onChanged: () => Promise<void>;
}) {
  const { t, problem } = useI18n();
  const [folders, setFolders] = useState<FolderInfo[] | null>(null);
  const [renaming, setRenaming] = useState<{ name: string; draft: string } | null>(null);
  const [unbinding, setUnbinding] = useState<FolderInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const say = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
  useEffect(() => {
    messenger.projectFolders(root).then(setFolders, say);
    // Read once when the section opens; the language only words an error.
  }, [messenger, root]);

  const run = async (work: () => Promise<FolderInfo[]>) => {
    setBusy(true);
    setError(null);
    try {
      setFolders(await work());
      await onChanged();
      return true;
    } catch (e) {
      say(e);
      return false;
    } finally {
      setBusy(false);
    }
  };
  const bind = async () => {
    const path = await messenger.pickFolder(t("folders.bind_title"));
    if (path) await run(() => messenger.bindFolder(root, path));
  };
  const locate = async (name: string) => {
    const path = await messenger.pickFolder(t("folders.locate_title", { name }));
    if (path) await run(() => messenger.locateFolder(root, name, path));
  };
  const badge = (f: FolderInfo) =>
    !f.exists ? (
      <span className="folder-badge missing">{t("folders.not_found")}</span>
    ) : f.section ? (
      <span className="folder-badge ok">{t("folders.set_up")}</span>
    ) : (
      <span className="folder-badge warn">{t("folders.no_section")}</span>
    );

  return (
    <div className="project-folders">
      {/* Task-A05-opal549: says up front that one taskbag serves many folders (approver, 2026-10-26). */}
      <p className="banner folders-note small">
        <span>
          <strong>{t("folders.note_title")}</strong> {t("folders.note")}
        </span>
      </p>
      {folders !== null && folders.length === 0 && <p className="muted small">{t("folders.none")}</p>}
      <ul className="folder-rows">
        {(folders ?? []).map((f) => (
          <li key={f.name}>
            {renaming?.name === f.name ? (
              <form
                className="settings-row"
                onSubmit={(e) => {
                  e.preventDefault();
                  void run(() => messenger.renameFolder(root, f.name, renaming.draft.trim())).then((ok) => ok && setRenaming(null));
                }}
              >
                <input aria-label={t("folders.rename_label", { name: f.name })} value={renaming.draft} autoFocus onChange={(e) => setRenaming({ name: f.name, draft: e.target.value })} />
                <button type="submit" className="primary" disabled={busy || !renaming.draft.trim() || renaming.draft.trim() === f.name}>
                  {t("settings.save")}
                </button>
                <button type="button" onClick={() => setRenaming(null)} disabled={busy}>
                  {t("settings.cancel")}
                </button>
              </form>
            ) : (
              <>
                <div className="folder-head">
                  <strong>{f.name}</strong> <code className="muted">{f.path}</code> {badge(f)}
                </div>
                <div className="folder-meta muted small">{t("folders.tasks", { count: f.tasks, open: f.open })}</div>
                <div className="folder-actions">
                  {f.exists ? (
                    <>
                      <button type="button" className="link reveal" onClick={() => messenger.showFolderInFileManager(root, f.name).catch(say)}>
                        {t(fileManagerKey)}
                      </button>
                      <button type="button" className="link reveal" onClick={() => setRenaming({ name: f.name, draft: f.name })} disabled={busy}>
                        {t("settings.rename")}
                      </button>
                    </>
                  ) : (
                    <button type="button" className="link reveal" onClick={() => void locate(f.name)} disabled={busy}>
                      {t("folders.locate")}
                    </button>
                  )}
                  <button type="button" className="link reveal" onClick={() => setUnbinding(f)} disabled={busy}>
                    {t("folders.unbind")}
                  </button>
                </div>
              </>
            )}
            {unbinding?.name === f.name && (
              <div className="banner unbind-confirm" role="alertdialog" aria-label={t("folders.unbind_title", { name: f.name })}>
                <strong>{t("folders.unbind_title", { name: f.name })}</strong>
                <ul className="small">
                  <li>{t("folders.unbind_tasks", { count: f.tasks, open: f.open })}</li>
                  <li>{t("folders.unbind_files")}</li>
                  <li>{t("folders.unbind_section")}</li>
                </ul>
                <div className="row start">
                  <button type="button" onClick={() => setUnbinding(null)} disabled={busy}>
                    {t("settings.cancel")}
                  </button>
                  <button
                    type="button"
                    className="primary"
                    onClick={() => void run(() => messenger.unbindFolder(root, f.name)).then(() => setUnbinding(null))}
                    disabled={busy}
                  >
                    {t("folders.unbind")}
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      {error && <p className="banner error">{error}</p>}
      {/* Add a Project Folder… while none is bound, Add Another Folder… after (approver, 2026-10-26). */}
      <button type="button" className="with-icon" onClick={() => void bind()} disabled={busy}>
        <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
          <path d="M2.5 5.5a1 1 0 0 1 1-1h4l1.5 2h7.5a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1z" />
          <path d="M10 9v5M7.5 11.5h5" />
        </svg>
        {t((folders ?? []).length === 0 ? "folders.bind" : "folders.bind_another")}
      </button>
    </div>
  );
}
