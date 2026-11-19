// The Evidence screen (Task-A01-herb903, decision 0029 points 1 and 2): at the top of a task to
// check, in the task pane and in the For You window, everything a person needs to let the work
// through or send it back, without scrolling about: what they are asked to do and the risk, the
// check steps, a summary of the change, the test results, the AI decisions to review, and the
// handoff note folded. A part with nothing in it is not shown. The order comes from the core.

import type { ReactNode } from "react";
import { useI18n } from "./i18n";
import { Markdown } from "./Markdown";
import { linkText } from "./historyFlow";
import { askText } from "./queueFlow";
import { decisionsToReview, humanCheckNote, toReview } from "./stopFlow";
import { riskClass, riskText } from "./taskRiskFlow";
import { codeLinks, evidenceParts, isEvidenceHeading, type EvidencePart } from "./evidenceFlow";
import type { Task } from "./types";

export function EvidenceScreen({
  task,
  ask,
  onLink,
  decisions,
  actions,
}: {
  task: Task;
  /** What the person is asked to do, as the For You window words it; left out, it is worked out
   * from the task (check, and review when it carries AI decisions to review). */
  ask?: ReactNode;
  onLink: (href: string) => void;
  /** The AI decisions box and the folded details, as the pane draws them; empty: none. */
  decisions: ReactNode;
  /** The pane's Checked button, drawn under the screen. */
  actions: ReactNode;
}) {
  const { t } = useI18n();
  const parts = evidenceParts(task);
  const reason = task.evidence?.risk_reason;
  const summary = task.evidence?.summary;
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
                  </li>
                ))}
              </ul>
            )}
          </section>
        );
      case "tests":
        return (
          <section key={p} className="evidence-part">
            <h4>{t("evidence.tests")}</h4>
            <p className="small">
              {t("evidence.tests_none")}{" "}
              <button type="button" className="link" onClick={(e) => showEvidence(e.currentTarget)}>
                {t("evidence.tests_see")}
              </button>
            </p>
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
