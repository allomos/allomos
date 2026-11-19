// A failed check's screenshots on the Evidence screen (Task-A01-fondue470, decision 0029 § 6):
// under the check's line, a small picture of each, read from the task's `evidence/` folder in the
// taskbag; clicking one shows it large over the window (Esc, × or a click outside closes it).
// Where the pictures cannot be read (the companions) or this computer does not have one yet (the
// taskbag not brought up to date), its file name is shown instead.

import { useEffect, useRef, useState } from "react";
import { useI18n } from "./i18n";
import { screenshotType, type EvidenceReader } from "./evidenceFlow";

export function EvidenceShots({ task, check, files, reader }: { task: string; check: string; files: string[]; reader?: EvidenceReader }) {
  const { t } = useI18n();
  const urls = useShots(task, files, reader);
  const [open, setOpen] = useState<string | null>(null);
  if (files.length === 0) return null;
  return (
    <div className="evidence-shots">
      {files.map((f) => {
        const url = urls[f];
        const alt = t("evidence.screenshot_alt", { check, name: f });
        return url ? (
          <button key={f} type="button" className="evidence-shot" title={t("evidence.screenshot_open")} onClick={() => setOpen(f)}>
            <img src={url} alt={alt} />
          </button>
        ) : (
          <span key={f} className="evidence-shot-missing muted" title={`tasks/${task}/evidence/${f}`}>
            {url === null ? t("evidence.screenshot_missing", { name: f }) : t("evidence.screenshot_name", { name: f })}
          </span>
        );
      })}
      {open && urls[open] && (
        <ShotLarge url={urls[open]!} alt={t("evidence.screenshot_alt", { check, name: open })} onClose={() => setOpen(null)} />
      )}
    </div>
  );
}

/** Each file's picture as an object URL; `null`: this computer does not have it; missing: not
 * read (yet, or there is no reader). The URLs are let go when the files change or the screen goes. */
function useShots(task: string, files: string[], reader?: EvidenceReader): Record<string, string | null> {
  const [urls, setUrls] = useState<Record<string, string | null>>({});
  const key = `${task}/${files.join("/")}`;
  const read = useRef(reader);
  read.current = reader;
  const can = Boolean(reader);
  useEffect(() => {
    const r = read.current;
    setUrls({});
    if (!r || files.length === 0) return;
    let live = true;
    const made: string[] = [];
    for (const f of files) {
      r.screenshot(task, f).then(
        (bytes) => {
          if (!live) return;
          const url = URL.createObjectURL(new Blob([bytes], { type: screenshotType(f) }));
          made.push(url);
          setUrls((u) => ({ ...u, [f]: url }));
        },
        () => live && setUrls((u) => ({ ...u, [f]: null })),
      );
    }
    return () => {
      live = false;
      made.forEach((u) => URL.revokeObjectURL(u));
    };
    // `files` is read through `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, can]);
  return urls;
}

function ShotLarge({ url, alt, onClose }: { url: string; alt: string; onClose: () => void }) {
  const { t } = useI18n();
  const closeRef = useRef<HTMLButtonElement>(null);
  const closeNow = useRef(onClose);
  closeNow.current = onClose;
  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closeNow.current();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
  return (
    <div className="overlay above evidence-shot-overlay" role="dialog" aria-modal="true" aria-label={alt} onClick={onClose}>
      <figure className="evidence-shot-large" onClick={(e) => e.stopPropagation()}>
        <button ref={closeRef} type="button" className="evidence-shot-close" aria-label={t("evidence.screenshot_close")} onClick={onClose}>
          ×
        </button>
        <img src={url} alt={alt} />
        <figcaption className="small muted">{alt}</figcaption>
      </figure>
    </div>
  );
}
