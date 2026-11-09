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
import { codexActionAtStart, codexFirstChecked, codexOffer, ruleText, rulesList } from "./codexRulesFlow";
import type { CodexRules } from "./codexRulesFlow";
import { isProblem } from "./types";
import { FIRST_FREQUENCY, FREQUENCIES, TERMINAL_AGENTS, firstScope, frequencyText } from "./monitorFlow";
import type { Frequency, OpenedFrom, Scope, ScopeOptions, TerminalAgent } from "./monitorFlow";

export function MonitorDialog({
  messenger,
  root,
  options,
  from,
  onStart,
  onClose,
}: {
  messenger: Messenger;
  /** The taskbag, where a change to Codex's rules is recorded (Task-A03-pier032). */
  root: string;
  /** The Which tasks field's choices, counted from the taskbag as it is now. */
  options: ScopeOptions;
  from: OpenedFrom;
  onStart: (scope: Scope, agent: TerminalAgent, quiet: boolean, every: Frequency) => void;
  onClose: () => void;
}) {
  const { t, language, problem } = useI18n();
  // Every choice must run unattended (approver, 2026-11-04): this step runs only the agents
  // Allomos starts itself; the others are shown with why not.
  const unavailable = useMemo<Unavailable>(
    () => ({ claude_app: t("monitor.later_app"), codex_app: t("monitor.later_codex_app"), [MANUAL]: t("monitor.later_manual") }),
    [t],
  );
  const { agents, slow, choice: agent, setChoice: setAgent } = useAgents(messenger, undefined, unavailable);
  const how: Partial<Record<Choice, string>> = {
    claude_cli: t("monitor.how_terminal"),
    codex_cli: t("monitor.how_terminal"),
    antigravity_cli: t("monitor.how_terminal"),
  };
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

  // Allomos's rules for Codex (Task-A03-pier032): read when Codex is chosen; written only at Start.
  const [codex, setCodex] = useState<CodexRules | null>(null);
  const [codexOn, setCodexOn] = useState(true);
  const [codexError, setCodexError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  useEffect(() => {
    if (agent !== "codex_cli") return;
    let live = true;
    messenger
      .codexRulesStatus()
      .then((r) => {
        if (!live) return;
        setCodex(r);
        setCodexOn(codexFirstChecked(codexOffer(r)));
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [messenger, agent]);
  const offer = agent === "codex_cli" && codex ? codexOffer(codex) : null;
  const never = t("monitor.codex_rules_never");
  const nothing = t("monitor.codex_rules_none");
  const start = async () => {
    if (!canStart || starting) return;
    const action = offer ? codexActionAtStart(offer, codexOn) : null;
    if (action) {
      setStarting(true);
      try {
        await messenger.codexRulesApply(root, action);
      } catch (e) {
        setCodexError(t("monitor.codex_rules_failed", { problem: isProblem(e) ? problem(e) : String(e) }));
        setStarting(false);
        return;
      }
    }
    onStart(scope, agent as TerminalAgent, quiet, every);
  };

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
          <p className="banner update">
            {t(agent === "claude_cli" || agent === "codex_cli" || agent === "antigravity_cli" ? `monitor.permissions_${agent}` : "build.auto_mode")}
          </p>
          {offer && codex && (
            <div className="codex-rules">
              {offer === "newer" ? (
                <p className="small hint">{t("monitor.codex_rules_newer", { path: codex.path, version: codex.state === "newer" ? codex.version : codex.shipped })}</p>
              ) : (
                <>
                  {codex.state === "older" && (
                    <p className="small hint">
                      {t("monitor.codex_rules_changed_by", {
                        from: codex.version,
                        to: codex.shipped,
                        added: rulesList(codex.changes.added, never, nothing),
                        removed: rulesList(codex.changes.removed, never, nothing),
                      })}
                    </p>
                  )}
                  {codex.state === "changed" && (
                    <p className="small hint">
                      {t("monitor.codex_rules_edited", {
                        path: codex.path,
                        version: codex.shipped,
                        added: rulesList(codex.changes.added, never, nothing),
                        removed: rulesList(codex.changes.removed, never, nothing),
                      })}
                    </p>
                  )}
                  <label>
                    <input type="checkbox" checked={codexOn} onChange={(e) => setCodexOn(e.target.checked)} />{" "}
                    <strong>
                      {offer === "changed"
                        ? t("monitor.codex_rules_replace", { version: codex.shipped })
                        : t(`monitor.codex_rules_${offer}`, { path: codex.path, version: codex.shipped })}
                    </strong>
                  </label>
                  <p className="muted small">{t("monitor.codex_rules_note")}</p>
                  <details>
                    <summary>{t("monitor.codex_rules_show")}</summary>
                    <ul className="small">
                      {codex.rules.map((r) => (
                        <li key={`${r.decision} ${r.command}`}>
                          <code>{ruleText(r, never)}</code>
                        </li>
                      ))}
                    </ul>
                  </details>
                </>
              )}
              {codexError && <p className="error small">{codexError}</p>}
            </div>
          )}
          <p className="muted small">{t("monitor.backlog_note")}</p>
        </div>

        <div className="row">
          <button type="button" onClick={onClose}>
            {t("wizard.cancel")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={!canStart || starting}
            onClick={() => void start()}
          >
            {t("monitor.start")}
          </button>
        </div>
      </section>
    </div>
  );
}
