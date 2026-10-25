// The Agents section of the taskbag's settings (Task-A05-bag963, approver 2026-10-25): one row per
// AI agent saying in how many of the taskbag's project folders it is set up, naming the folders
// where it is not, each with Set up…, which opens the same flow as Connect with the Agents…
// (`AgentsPanel`) for that folder. With no folder bound, the taskbag's one project folder is the
// folder, as the agent line under the taskbag's name shows it (`AgentLine`).

import { useEffect, useState } from "react";
import { agentRows, rowText, type SectionFolder } from "./agentsSectionFlow";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { isProblem } from "./types";

/** How often the section reads again: the instruction files are outside the taskbag, and an
 * agent set up from here writes them a little later. */
const CHECK_MS = 4000;

/** The last part of a path, for the one project folder's name. */
const lastPart = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

export function AgentsSection({ messenger, root, onSetUp }: { messenger: Messenger; root: string; onSetUp: (folder: string) => void }) {
  const { t, problem } = useI18n();
  const [folders, setFolders] = useState<SectionFolder[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const read = async () => {
      try {
        const bound = await messenger.projectFolders(root);
        const list = bound.length > 0 ? bound : [await messenger.agentStatus(root)].map((s) => ({ name: lastPart(s.project), full: s.project, exists: true }));
        const read = await Promise.all(
          list.map(async (f): Promise<SectionFolder> => ({
            name: f.name,
            full: f.full,
            exists: f.exists,
            setup: f.exists ? await messenger.agentSetupOf(f.full) : null,
          })),
        );
        if (live) {
          setFolders(read);
          setError(null);
        }
      } catch (e) {
        if (live) setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));
      }
    };
    void read();
    const timer = window.setInterval(() => void read(), CHECK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
    // The language only words an error.
  }, [messenger, root]);

  return (
    <div className="agents-section">
      <p className="muted small">{t("agents_section.body")}</p>
      {folders !== null && (
        <ul className="folder-rows">
          {agentRows(folders).map((row) => {
            const text = rowText(row);
            return (
              <li key={row.agent}>
                <div className="folder-head">
                  <strong>{row.name}</strong>{" "}
                  <span className={`folder-badge ${row.missing.length === 0 && row.total > 0 ? "ok" : "warn"}`}>{t(text.key, { done: text.done, total: text.total })}</span>
                </div>
                {row.total > 1 &&
                  row.missing.map((f) => (
                    <div key={f.name} className="folder-actions">
                      <span className="muted small">{t(f.exists ? "agents_section.missing" : "agents_section.not_found", { name: f.name })}</span>
                      {f.exists && (
                        <button type="button" className="link reveal" onClick={() => onSetUp(f.full)}>
                          {t("agents_section.set_up_action")}
                        </button>
                      )}
                    </div>
                  ))}
                {row.total === 1 && row.missing[0]?.exists && (
                  <div className="folder-actions">
                    <button type="button" className="link reveal" onClick={() => onSetUp(row.missing[0].full)}>
                      {t("agents_section.set_up_action")}
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {error && <p className="banner error">{error}</p>}
    </div>
  );
}
