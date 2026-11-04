// Monitor ready tasks… (Task-A03-moth285, approver 2026-11-03/04): the dialog the button opens.
// Which tasks (the active iteration, every ready task, or one iteration, each with its count of
// ready tasks nobody holds), how often to check when none is ready, the shared agent picker and
// the run mode (quiet first). The counts follow the taskbag while the dialog is open; the first
// choice is worked out again each time it opens, never remembered.

import { useEffect, useMemo, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AgentPicker, useAgents } from "./AgentPicker";
import type { Unavailable } from "./AgentPicker";
import { MANUAL } from "./agentPickerFlow";
import type { Choice } from "./agentPickerFlow";
import { Markdown } from "./Markdown";
import { FIRST_FREQUENCY, FREQUENCIES, TERMINAL_AGENTS, firstScope, frequencyText } from "./monitorFlow";
import type { Frequency, OpenedFrom, Scope, ScopeOptions, TerminalAgent } from "./monitorFlow";

export function MonitorDialog({
  messenger,
  options,
  from,
  onStart,
  onClose,
}: {
  messenger: Messenger;
  /** The Which tasks field's choices, counted from the taskbag as it is now. */
  options: ScopeOptions;
  from: OpenedFrom;
  onStart: (scope: Scope, agent: TerminalAgent, quiet: boolean, every: Frequency) => void;
  onClose: () => void;
}) {
  const { t, language } = useI18n();
  // Every choice must run unattended (approver, 2026-11-04): this step runs only the agents
  // Allomos starts itself; the others are shown with why not.
  const unavailable = useMemo<Unavailable>(
    () => ({ claude_app: t("monitor.later_app"), codex_app: t("monitor.later_codex_app"), [MANUAL]: t("monitor.later_manual") }),
    [t],
  );
  const { agents, slow, choice: agent, setChoice: setAgent } = useAgents(messenger, undefined, unavailable);
  const how: Partial<Record<Choice, string>> = { claude_cli: t("monitor.how_terminal"), codex_cli: t("monitor.how_terminal") };
  const [scope, setScope] = useState<Scope>(() => firstScope(options, from));
  const selectable = options.other.items.filter((i) => i.ready > 0);
  const [otherId, setOtherId] = useState<string | null>(() => (scope.kind === "iteration" && !scope.current ? scope.id : (selectable[0]?.id ?? null)));
  const [every, setEvery] = useState<Frequency>(FIRST_FREQUENCY);
  const [quiet, setQuiet] = useState(true);
  const [modes, setModes] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    messenger
      .runModesText(language)
      .then((text) => live && setModes(text))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger, language]);

  const isOther = scope.kind === "iteration" && !scope.current;
  const chooseOther = (id: string | null = otherId) => {
    if (id) setScope({ kind: "iteration", id, current: false });
  };
  const canStart = agent !== null && TERMINAL_AGENTS.includes(agent);
  const freqLabel = (f: Frequency) => {
    const { unit, n } = frequencyText(f);
    return t(`monitor.freq_${unit}`, { n });
  };

  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard move-cards monitor-dialog" role="dialog" aria-modal="true" aria-labelledby="monitor-title">
        <h2 id="monitor-title">{t("monitor.title")}</h2>
        <div className="dialog-body wizard-body">
          <p className="muted small">{t("monitor.intro")}</p>

          <h3>{t("monitor.which")}</h3>
          <ul className="monitor-scopes">
            {options.current && (
              <li className={scope.kind === "iteration" && scope.current ? "on" : undefined}>
                <label>
                  <input
                    type="radio"
                    name="monitor-scope"
                    checked={scope.kind === "iteration" && scope.current}
                    onChange={() => setScope({ kind: "iteration", id: options.current!.id, current: true })}
                  />{" "}
                  <span>
                    <strong>{t("monitor.current", { id: options.current.id })}</strong>
                    {t("monitor.current_ready", { n: options.current.ready })}
                    <br />
                    <span className="muted small">{t("monitor.current_note")}</span>
                  </span>
                </label>
              </li>
            )}
            <li className={scope.kind === "all" ? "on" : undefined}>
              <label>
                <input type="radio" name="monitor-scope" checked={scope.kind === "all"} onChange={() => setScope({ kind: "all" })} />{" "}
                <span>
                  <strong>{t("monitor.all")}</strong>
                  {t("monitor.all_count", { n: options.all })}
                </span>
              </label>
            </li>
            <li className={[isOther ? "on" : "", options.other.none || selectable.length === 0 ? "dim" : ""].filter(Boolean).join(" ") || undefined}>
              <label>
                <input
                  type="radio"
                  name="monitor-scope"
                  checked={isOther}
                  disabled={options.other.none || selectable.length === 0}
                  onChange={() => chooseOther()}
                />{" "}
                <strong>{t(options.other.name === "other" ? "monitor.other" : "monitor.one")}</strong>
              </label>
              {options.other.none ? (
                <span className="muted small scope-note">{t("monitor.no_iterations")}</span>
              ) : (
                <select
                  className={isOther ? undefined : "idle"}
                  aria-label={t("monitor.iteration_choose")}
                  value={otherId ?? ""}
                  disabled={selectable.length === 0}
                  // Clicking the list first chooses this option too.
                  onMouseDown={() => chooseOther()}
                  onFocus={() => chooseOther()}
                  onChange={(e) => {
                    setOtherId(e.target.value);
                    chooseOther(e.target.value);
                  }}
                >
                  {otherId === null && <option value="" />}
                  {options.other.items.map((i) => (
                    <option key={i.id} value={i.id} disabled={i.ready === 0}>
                      {t("monitor.iteration_item", { place: i.place, id: i.id, n: i.ready })}
                    </option>
                  ))}
                </select>
              )}
            </li>
          </ul>

          <h3>
            <label htmlFor="monitor-every">{t("monitor.frequency")}</label>
          </h3>
          <select id="monitor-every" className="monitor-every" value={every} onChange={(e) => setEvery(Number(e.target.value) as Frequency)}>
            {FREQUENCIES.map((f) => (
              <option key={f} value={f}>
                {freqLabel(f)}
              </option>
            ))}
          </select>
          <p className="muted small">{t("monitor.frequency_note")}</p>

          <h3>{t("monitor.agent")}</h3>
          <AgentPicker agents={agents} slow={slow} choice={agent} onChoose={setAgent} name="monitor-agent" unavailable={unavailable} how={how} />

          <h3>{t("monitor.mode")}</h3>
          <ul className="agent-list">
            {([true, false] as const).map((q) => (
              <li key={String(q)}>
                <label>
                  <input type="radio" name="monitor-mode" checked={quiet === q} onChange={() => setQuiet(q)} />{" "}
                  <strong>{t(q ? "build.mode_quiet" : "build.mode_normal")}</strong>{" "}
                  <span className="muted">— {t(q ? "build.mode_quiet_how" : "build.mode_normal_how")}</span>
                </label>
                {quiet === q && <p className="small hint mode-effect">{t(q ? "monitor.mode_quiet_effect" : "monitor.mode_normal_effect")}</p>}
              </li>
            ))}
          </ul>
          {modes && (
            <details className="run-modes">
              <summary>{t("build.mode_more")}</summary>
              <Markdown source={modes} onLink={() => undefined} />
            </details>
          )}
          <p className="banner update">{t("build.auto_mode")}</p>
          <p className="muted small">{t("monitor.backlog_note")}</p>
        </div>

        <div className="row">
          <button type="button" onClick={onClose}>
            {t("wizard.cancel")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={!canStart}
            onClick={() => canStart && onStart(scope, agent as TerminalAgent, quiet, every)}
          >
            {t("monitor.start")}
          </button>
        </div>
      </section>
    </div>
  );
}
