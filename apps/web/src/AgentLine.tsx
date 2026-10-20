// "Do the AI agents know about this project?" (Task-A03-04): a short note beside Show in Finder.
// Allomos works it out itself from the project's instruction files (AGENTS.md, CLAUDE.md);
// agents write no flag. Since 2026-10-10 (owner) it no longer says who last used `alm` or holds
// tasks: a holder copied from an old board read as an agent "working now". Allomos runs no Git
// (decision 0017), so it also says how many changes made here are not shared yet.
//
// Since Task-A03-02 the line is one chip per agent: the agent that reported its quota last
// (`alm quota`) comes first with its quota, coloured by the share left; the others are dimmed.
// Hovering a chip lists every reported account (quotaFlow.ts).

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import { localTime } from "./handoffFlow";
import type { Messenger } from "./messenger";
import { agentName, chips, freshness, popupRows, tone } from "./quotaFlow";
import type { AgentStatus, QuotaReport } from "./types";

/** How often the line checks again: the instruction files are outside the taskbag. */
const CHECK_MS = 4000;

export function AgentLine({
  messenger,
  root,
  onTell,
}: {
  messenger: Messenger;
  root: string;
  /** Opens the "Tell AI agents" panel for this project. */
  onTell: () => void;
}) {
  const { t } = useI18n();
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [reports, setReports] = useState<QuotaReport[]>([]);
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let live = true;
    const check = () => {
      setNow(new Date());
      void messenger
        .agentStatus(root)
        .then((s) => live && setStatus(s))
        .catch(() => undefined);
      void messenger
        .quotaReports()
        .then((r) => live && setReports(r))
        .catch(() => undefined);
    };
    check();
    const timer = window.setInterval(check, CHECK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, root]);

  if (!status) return null;
  const { codex, claude } = status.setup;
  const mark = (ok: boolean) => (ok ? "✓" : "✗");
  const list = chips(reports, status.setup);
  const anyCurrent = list.some((c) => c.current);

  const popup = (
    <div className="quota-popup" role="tooltip">
      <strong>{t("agentline.popup_title")}</strong>
      {reports.length === 0 ? (
        <p className="muted small">{t("agentline.popup_none")}</p>
      ) : (
        <ul>
          {popupRows(reports).map((r) => {
            const stale = freshness(r, now).state === "stale";
            return (
              <li key={`${r.agent}/${r.account ?? ""}/${r.window}`} className={stale ? "stale" : undefined}>
                <span>
                  {agentName(r.agent)}
                  {r.account ? ` · ${r.account}` : ""}
                </span>
                <span className={`quota-${tone(r.left)}`}>
                  {t("agentline.popup_left", { left: r.left, window: t(r.window === "week" ? "agentline.window_week" : "agentline.window_5h") })}
                </span>
                {r.resets && <span>{t("agentline.popup_resets", { time: r.resets.replace("T", " ").slice(0, 16) })}</span>}
                <span className="muted">
                  {t(stale ? "agentline.popup_when_reported" : "agentline.popup_updated", { time: localTime(r.at) })}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );

  return (
    <div className="agent-line">
      {reports.length === 0 && !codex && !claude ? (
        <span>{t("agentline.line", { state: t("agentline.none") })}</span>
      ) : (
        <>
          <span>{t("agentline.label")}</span>
          {list.map((c) => {
            const fresh = c.quota ? freshness(c.quota, now) : null;
            const classes = ["agent-chip"];
            if (anyCurrent && !c.current) classes.push("dimmed");
            if (c.quota) classes.push(fresh?.state === "stale" ? "stale" : `quota-${tone(c.quota.left)}`);
            return (
              <span key={c.agent} className={classes.join(" ")} tabIndex={0}>
                {c.quota
                  ? t("agentline.quota", { name: c.name, left: c.quota.left })
                  : `${c.name}${c.told === null ? "" : ` ${mark(c.told)}`}`}
                {fresh && fresh.state !== "fresh" && ` · ${t("agentline.age", { minutes: fresh.minutes })}`}
                {popup}
              </span>
            );
          })}
        </>
      )}
      {!(codex && claude) && (
        <button type="button" className="link" onClick={onTell}>
          {t("agentline.tell")}
        </button>
      )}
      {status.unshared > 0 && (
        <span title={t("agentline.unshared_hint")}>{t("agentline.unshared", { count: status.unshared })}</span>
      )}
    </div>
  );
}
