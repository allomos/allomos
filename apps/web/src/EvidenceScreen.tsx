// The Evidence screen (Task-A01-herb903, decision 0029 points 1 and 2): at the top of a task to
// check, in the task pane and in the For You window, everything a person needs to let the work
// through or send it back, without scrolling about: what they are asked to do and the risk, the
// check steps, a summary of the change, the test results, the AI decisions to review, and the
// handoff note folded. A part with nothing in it is not shown. The order lives in evidenceFlow.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import { linkText } from "./historyFlow";
import { askText } from "./queueFlow";
import { decisionsToReview, humanCheckNote, toReview } from "./stopFlow";
import { riskClass, riskText } from "./taskRiskFlow";
import {
  changeTotals,
  checkResults,
  codeLinks,
  commitSubject,
  evidenceParts,
  isEvidenceHeading,
  MOST_FILES,
  resultKey,
  riskReason,
  summarySentence,
  type CodeChanges,
  type EvidencePart,
} from "./evidenceFlow";
import type { Task, TaskHistory } from "./types";

export function EvidenceScreen({
  task,
  history,
  ask,
  onLink,
  decisions,
  actions,
  readChanges,
}: {
  task: Task;
  /** The task's recorded changes, for the reason given with its risk; null until read. */
  history: TaskHistory | null;
  /** What the person is asked to do, as the For You window words it; left out, it is worked out
   * from the task (check, and review when it carries AI decisions to review). */
  ask?: ReactNode;
  onLink: (href: string) => void;
  /** The AI decisions box and the folded details, as the pane draws them; empty: none. */
  decisions: ReactNode;
  /** The pane's Checked button, drawn under the screen. */
  actions: ReactNode;
  /** Reads the recorded commits' first lines and changed files from the project's own git (only
   * reads); left out where git cannot be read (the companions): the commits show as links only. */
  readChanges?: (hashes: string[]) => Promise<CodeChanges>;
}) {
  const { t } = useI18n();
  const parts = evidenceParts(task, Boolean(decisions));
  const reason = riskReason(history);
  const summary = summarySentence(task.handoff_note);
  const results = checkResults(history);
  const refs = codeLinks(task).map((l) => l.ref);
  const changes = useChanges(refs, readChanges);
  const ownAsk = askText({ todo: toReview(task) ? ["check", "review"] : ["check"], decisions: decisionsToReview(task).length }, t);

  const part = (p: EvidencePart): ReactNode => {
    switch (p) {
      case "ask_risk":
        return (
          <div key={p} className="evidence-part evidence-ask">
            <p className="for-you-ask">{ask ?? ownAsk}</p>
            <p>
              {t("task.risk")}: <span className={riskClass(task)}>{riskText(task, t)}</span>
              {reason && <span className="muted"> — {reason}</span>}
            </p>
          </div>
        );
      case "check_steps":
        return (
          <section key={p} className="evidence-part">
            <h4>{t("pane.human_check_steps")}</h4>
            {task.human_check_steps?.trim() && <Markdown source={task.human_check_steps} onLink={onLink} />}
            {humanCheckNote(task) && (
              <p className="small">
                <span className="muted">{t("pane.without_human_check")}: </span>
                {humanCheckNote(task)}
              </p>
            )}
          </section>
        );
      case "summary":
        return (
          <section key={p} className="evidence-part">
            <h4>{t("evidence.summary")}</h4>
            {summary && <Markdown source={summary} onLink={onLink} />}
            {codeLinks(task).length > 0 && (
              <ul className="evidence-commits small">
                {codeLinks(task).map((l) => (
                  <li key={`${l.kind}-${l.ref}`}>
                    {l.url ? (
                      <a
                        href={l.url}
                        title={l.url}
                        onClick={(e) => {
                          e.preventDefault();
                          onLink(l.url!);
                        }}
                      >
                        {linkText(l, t)}
                      </a>
                    ) : (
                      <span title={t("pane.link_no_url")}>{linkText(l, t)}</span>
                    )}
                    <CommitSubject subject={commitSubject(changes, l.ref)} />
                  </li>
                ))}
              </ul>
            )}
            {changes && changes.files.length > 0 && <ChangedFiles files={changes.files} />}
          </section>
        );
      case "tests":
        return (
          <section key={p} className="evidence-part">
            <h4>{t("evidence.tests")}</h4>
            {results.length > 0 ? (
              <ul className="evidence-tests small">
                {results.map((r) => (
                  <li key={r.name} className={`evidence-test result-${r.result}`}>
                    <span className="evidence-result">{resultKey(r.result) ? t(resultKey(r.result)!) : r.result}</span>
                    <span className="evidence-test-name">{r.name}</span>
                    {r.detail && <span className="muted"> — {r.detail}</span>}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="small">
                {t("evidence.tests_none")}{" "}
                <button type="button" className="link" onClick={(e) => showEvidence(e.currentTarget)}>
                  {t("evidence.tests_see")}
                </button>
              </p>
            )}
          </section>
        );
      case "decisions":
        return (
          <div key={p} className="evidence-part evidence-decisions">
            {decisions}
          </div>
        );
      case "handoff":
        return (
          <details key={p} className="evidence-part evidence-handoff">
            <summary>{t("pane.handoff")}</summary>
            <Markdown source={task.handoff_note ?? ""} onLink={onLink} />
          </details>
        );
    }
  };

  return (
    <section className="evidence" aria-label={t("evidence.aria", { id: task.id })}>
      <h3>{t("evidence.title")}</h3>
      {parts.map(part)}
      {actions && <div className="evidence-actions">{actions}</div>}
    </section>
  );
}

/** The recorded commits read from git; null while read, or when they cannot be. */
function useChanges(refs: string[], read?: (hashes: string[]) => Promise<CodeChanges>): CodeChanges | null {
  const key = refs.join(" ");
  const [got, setGot] = useState<{ key: string; changes: CodeChanges } | null>(null);
  // The reader is made anew at every render; only a change of the commits reads git again.
  const reader = useRef(read);
  reader.current = read;
  const can = Boolean(read);
  useEffect(() => {
    const read = reader.current;
    if (!read || !key) return;
    let live = true;
    read(key.split(" ")).then(
      (changes) => live && setGot({ key, changes }),
      () => live && setGot(null),
    );
    return () => {
      live = false;
    };
  }, [key, can]);
  return got?.key === key ? got.changes : null;
}

/** A commit's first line after its link; this computer not having it is said so. */
function CommitSubject({ subject }: { subject: string | null | undefined }) {
  const { t } = useI18n();
  if (subject === undefined) return null;
  return subject === null ? <span className="muted"> — {t("evidence.not_here")}</span> : <> {subject}</>;
}

/** The files the commits changed together, each with the lines added and removed. */
function ChangedFiles({ files }: { files: CodeChanges["files"] }) {
  const { t } = useI18n();
  const total = changeTotals(files);
  return (
    <div className="evidence-files small">
      <p>{t("evidence.files", { files: String(total.files), added: String(total.added), removed: String(total.removed) })}</p>
      <ul>
        {files.slice(0, MOST_FILES).map((f) => (
          <li key={f.path}>
            <code>{f.path}</code>{" "}
            {f.added === null ? (
              <span className="muted">{t("evidence.binary")}</span>
            ) : (
              <>
                <span className="lines-added">+{f.added}</span> <span className="lines-removed">−{f.removed}</span>
              </>
            )}
          </li>
        ))}
      </ul>
      {files.length > MOST_FILES && <p className="muted">{t("evidence.more_files", { n: String(files.length - MOST_FILES) })}</p>}
    </div>
  );
}

/** Opens the pane's folded description and scrolls to its Evidence section; when it has none,
 * to the description itself. */
function showEvidence(from: HTMLElement) {
  const fold = from.closest(".task-pane")?.querySelector<HTMLDetailsElement>("details.pane-description");
  if (!fold) return;
  fold.open = true;
  requestAnimationFrame(() => {
    const marks = Array.from(fold.querySelectorAll<HTMLElement>("h3, h4, h5, h6, strong"));
    const target = marks.find((m) => isEvidenceHeading(m.textContent ?? "")) ?? fold;
    target.scrollIntoView({ block: "start", behavior: "smooth" });
  });
}
