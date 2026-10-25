// The taskbag's settings (Task-A05-camera758, approver 2026-10-25): opened from Settings beside the
// taskbag's name in the window header. A list of sections on the left, the chosen one on the right;
// Close and Esc close it. This task builds General, with the taskbag's location and Show in Finder;
// the other sections (Project folders, Tasks, Handoff, Agents) come with their own tasks.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useI18n, type Key } from "./i18n";
import { middleEllipsis } from "./paths";

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

export function TaskbagSettings({
  name,
  root,
  fileManagerKey,
  onShowInFileManager,
  onClose,
}: {
  name: string;
  root: string;
  /** `top.show_in_finder`, `top.show_in_explorer` or `top.show_in_files`, for this computer. */
  fileManagerKey: Key;
  onShowInFileManager: () => void;
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
