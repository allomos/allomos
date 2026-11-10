// Monitor ready tasks… (Task-A03-moth285, approver 2026-11-03/04): the dialog the button opens.
// Which tasks (the active iteration, every ready task, or one iteration, each with its count of
// ready tasks nobody holds), how often to check when none is ready, the agents (several can be
// chosen, Task-A03-fruit467) and the run mode (quiet first). The counts follow the taskbag while the dialog is open; the first
// choice is worked out again each time it opens, never remembered.

import { useEffect, useMemo, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, useAgents } from "./AgentPicker";
import type { Unavailable } from "./AgentPicker";
import { MANUAL, foundGroups } from "./agentPickerFlow";
import type { Agent, Iteration, Task } from "./types";
import { Markdown } from "./Markdown";
import { codexActionAtStart, codexFirstChecked, codexOffer, ruleText, rulesList } from "./codexRulesFlow";
import type { CodexRules } from "./codexRulesFlow";
import { isProblem } from "./types";
import { ALL_LEVELS, FIRST_FREQUENCY, FREQUENCIES, TERMINAL_AGENTS, accountKey, accountRows, agentLabel, baseAgent, firstScope, frequencyText, scopeOptions, suggestedAccountName } from "./monitorFlow";
import type { AccountRow, AgentAccount, Frequency, Levels, OpenedFrom, Scope, TerminalAgent } from "./monitorFlow";
import { useAgentAccounts } from "./useAgentAccounts";
import { labelOf } from "./agentAccounts";
import { levelsHold } from "./difficultyFlow";
import type { Key } from "./i18n";

/** The agents an extra account can be added to (Task-A03-oven548). */
const ACCOUNT_AGENTS: readonly string[] = ["claude_cli", "codex_cli"];
/** The AGENT_KEY name of a monitor agent (an account's agent is its agent's name). */
const nameKey = (a: string) => AGENT_KEY[baseAgent(a) as Agent["id"]];

/** The difficulty levels a range can start or end at, with their names (Task-A03-wave335). */
const LEVEL_NUMBERS = [1, 2, 3, 4, 5] as const;
const LEVEL_KEYS = {
  1: "difficulty.level.1",
  2: "difficulty.level.2",
  3: "difficulty.level.3",
  4: "difficulty.level.4",
  5: "difficulty.level.5",
} as const;

export function MonitorDialog({
  messenger,
  root,
  tasks,
  areas,
  iterations,
  activeIteration,
  from,
  onStart,
  onClose,
}: {
  messenger: Messenger;
  /** The taskbag, where a change to Codex's rules is recorded (Task-A03-pier032). */
  root: string;
  /** The taskbag as it is now: the Which tasks field's choices are counted from it, and only the
   * tasks the chosen agents' difficulty levels take are counted (Task-A03-wave335). */
  tasks: Task[];
  areas: { id: string }[];
  iterations: Iteration[];
  activeIteration: string | null | undefined;
  from: OpenedFrom;
  /** `agents` chosen, in the order they are shown; `offered`: every one that could be chosen,
   * offered again in the status bar while the run goes on; `sessions`: how many sessions each
   * chosen agent may have at work at once (Task-A03-weasel276); `difficulties`: the levels each
   * chosen agent takes tasks from (Task-A03-wave335). */
  onStart: (
    scope: Scope,
    agents: TerminalAgent[],
    quiet: boolean,
    every: Frequency,
    offered: TerminalAgent[],
    sessions: Partial<Record<TerminalAgent, number>>,
    difficulties: Partial<Record<TerminalAgent, Levels>>,
  ) => void;
  onClose: () => void;
}) {
  const { t, language, problem } = useI18n();
  // Every choice must run unattended (approver, 2026-11-04): this step runs only the agents
  // Allomos starts itself; the others are shown with why not.
  const unavailable = useMemo<Unavailable>(
    () => ({ claude_app: t("monitor.later_app"), codex_app: t("monitor.later_codex_app"), [MANUAL]: t("monitor.later_manual") }),
    [t],
  );
  const { agents, slow, choice: first } = useAgents(messenger, undefined, unavailable);
  // The extra accounts (Task-A03-oven548): each one is a row of its own.
  const { accounts: accountList, reload: reloadAccounts } = useAgentAccounts(messenger);
  const extra = useMemo<AgentAccount[]>(() => accountList?.accounts ?? [], [accountList]);
  const firstName = t("monitor.account_first");
  const rowsOf = (agent: string): AccountRow[] => accountRows(agent, extra, firstName);
  /** The monitor agents that can be chosen, in the picker's order: each terminal agent found, then
   * its accounts. */
  const offered = useMemo<TerminalAgent[]>(
    () =>
      agents
        ? foundGroups(agents)
            .flatMap((g) => g.tools.map((a) => a.id))
            .filter((id) => TERMINAL_AGENTS.includes(id))
            .flatMap((id) => rowsOf(id).map((r) => r.key))
        : [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [agents, extra, firstName],
  );
  // The first agent the picker would choose starts checked; the person adds or takes off others.
  const [chosen, setChosen] = useState<TerminalAgent[] | null>(null);
  useEffect(() => {
    if (chosen === null && agents !== null) setChosen(first && TERMINAL_AGENTS.includes(first) ? [first as TerminalAgent] : []);
  }, [agents, first, chosen]);
  const picked = offered.filter((a) => chosen?.includes(a));
  const toggle = (a: TerminalAgent, on: boolean) => setChosen((c) => (on ? [...(c ?? []), a] : (c ?? []).filter((x) => x !== a)));
  // The agent whose account is being added, if any (Task-A03-oven548).
  const [adding, setAdding] = useState<string | null>(null);
  // How many sessions each agent may have at work at once (Task-A03-weasel276): kept as typed,
  // read as a whole number of one or more (one when not).
  const [sessionText, setSessionText] = useState<Partial<Record<TerminalAgent, string>>>({});
  const countOf = (a: TerminalAgent): number => {
    const n = Math.floor(Number(sessionText[a] ?? 1));
    return n >= 1 ? n : 1;
  };
  // The difficulty levels each agent takes tasks from (Task-A03-wave335): every level unless set.
  const [levels, setLevels] = useState<Partial<Record<TerminalAgent, Levels>>>({});
  const levelsOfAgent = (a: TerminalAgent): Levels => levels[a] ?? ALL_LEVELS;
  // The counts of the Which tasks choices: the ready tasks any chosen agent may take (all when
  // none is chosen yet).
  const options = scopeOptions(tasks, areas, iterations, activeIteration, (task) =>
    picked.length === 0 || picked.some((a) => levelsHold(levelsOfAgent(a), task)),
  );
  const codexChosen = picked.some((a) => baseAgent(a) === "codex_cli");
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
    if (!codexChosen) return;
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
  }, [messenger, codexChosen]);
  const offer = codexChosen && codex ? codexOffer(codex) : null;
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
    onStart(
      scope,
      picked,
      quiet,
      every,
      offered,
      Object.fromEntries(picked.map((a) => [a, countOf(a)])) as Partial<Record<TerminalAgent, number>>,
      Object.fromEntries(picked.map((a) => [a, levelsOfAgent(a)])) as Partial<Record<TerminalAgent, Levels>>,
    );
  };

  const isOther = scope.kind === "iteration" && !scope.current;
  const chooseOther = (id: string | null = otherId) => {
    if (id) setScope({ kind: "iteration", id, current: false });
  };
  const canStart = picked.length > 0;
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
          <AgentChecklist
            agents={agents}
            slow={slow}
            chosen={picked}
            onToggle={toggle}
            unavailable={unavailable}
            sessionText={sessionText}
            onSessionText={(a, text) => setSessionText((s) => ({ ...s, [a]: text }))}
            countOf={countOf}
            levelsOf={levelsOfAgent}
            onLevels={(a, l) => setLevels((s) => ({ ...s, [a]: l }))}
            rowsOf={rowsOf}
            onAdd={(a) => setAdding(a)}
          />
          {adding && (
            <AddAccount
              messenger={messenger}
              agent={adding}
              folder={accountList?.defaults[adding]}
              suggested={suggestedAccountName(extra, adding)}
              onCancel={() => setAdding(null)}
              onAdded={(key) => {
                setAdding(null);
                setChosen((c) => [...(c ?? []), key]);
                void reloadAccounts();
              }}
            />
          )}

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
          {picked.map((a) => (
            <p key={a} className="banner update">
              {picked.length > 1 && <strong>{labelOf(a, t(nameKey(a)))}: </strong>}
              {t(`monitor.permissions_${baseAgent(a)}` as Key)}
            </p>
          ))}
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

/** The agents, grouped as the shared picker shows them (Claude, Codex, Antigravity), each tool a
 * box to check (Task-A03-fruit467): any number of the terminal agents. The ones that cannot run
 * unattended are shown, not offered, each with why; Manual last, the same way. A terminal agent
 * with extra accounts has one box per account (Task-A03-oven548), and Claude Code and Codex an
 * "Add an account…" line. */
function AgentChecklist({
  agents,
  slow,
  chosen,
  onToggle,
  unavailable,
  sessionText,
  onSessionText,
  countOf,
  levelsOf,
  onLevels,
  rowsOf,
  onAdd,
}: {
  agents: Agent[] | null;
  slow: boolean;
  chosen: TerminalAgent[];
  onToggle: (agent: TerminalAgent, on: boolean) => void;
  unavailable: Unavailable;
  sessionText: Partial<Record<TerminalAgent, string>>;
  onSessionText: (agent: TerminalAgent, text: string) => void;
  countOf: (agent: TerminalAgent) => number;
  levelsOf: (agent: TerminalAgent) => Levels;
  onLevels: (agent: TerminalAgent, levels: Levels) => void;
  rowsOf: (agent: string) => AccountRow[];
  onAdd: (agent: string) => void;
}) {
  const { t } = useI18n();
  if (agents === null) {
    return slow ? (
      <p className="muted searching" role="status">
        <span className="spinner" aria-hidden="true" /> {t("agents.searching")}
      </p>
    ) : null;
  }
  return (
    <>
      <p className="muted small">{t("monitor.agents_note")}</p>
      <ul className="agent-list">
        {foundGroups(agents).map((g) => (
          <li key={g.name}>
            <strong>{g.name}</strong>
            <ul className="agent-tools">
              {g.tools.flatMap((a) => {
                const why = unavailable[a.id];
                const usable = !why && TERMINAL_AGENTS.includes(a.id);
                if (!usable) {
                  return [
                    <li key={a.id}>
                      <label className="muted">
                        <input type="checkbox" checked={false} disabled />{" "}
                        {t(AGENT_KEY[a.id])}
                        {why && <span className="muted small hint">{why}</span>}
                      </label>
                    </li>,
                  ];
                }
                const rows = rowsOf(a.id);
                const items = rows.map((row) => {
                  const on = chosen.includes(row.key);
                  const label = row.name ? agentLabel(t(AGENT_KEY[a.id]), row.name) : t(AGENT_KEY[a.id]);
                  return (
                    <li key={row.key}>
                      <label>
                        <input type="checkbox" checked={on} onChange={(e) => onToggle(row.key, e.target.checked)} /> {label}{" "}
                        {on && (
                          <span className="muted">
                            — {t("monitor.how_terminal")} ({a.found})
                          </span>
                        )}
                      </label>
                      {on && (
                        <span className="monitor-sessions small">
                          <span className="muted">{t("monitor.sessions_label")}</span>{" "}
                          <input
                            type="number"
                            min={1}
                            step={1}
                            inputMode="numeric"
                            aria-label={t("monitor.sessions_aria", { agent: label })}
                            value={sessionText[row.key] ?? "1"}
                            onChange={(e) => onSessionText(row.key, e.target.value)}
                            onBlur={() => onSessionText(row.key, String(countOf(row.key)))}
                          />
                        </span>
                      )}
                      {on && <LevelsPicker agent={row.key} name={label} levels={levelsOf(row.key)} onChange={(l) => onLevels(row.key, l)} />}
                    </li>
                  );
                });
                if (ACCOUNT_AGENTS.includes(a.id)) {
                  items.push(
                    <li key={`${a.id}-add`}>
                      <button type="button" className="link" onClick={() => onAdd(a.id)}>
                        {t("monitor.add_account", { agent: t(AGENT_KEY[a.id]) })}
                      </button>
                    </li>,
                  );
                }
                return items;
              })}
            </ul>
          </li>
        ))}
        <li>
          <label className="muted">
            <input type="checkbox" checked={false} disabled />{" "}
            <strong>{t("agents.manual")}</strong>
            {unavailable[MANUAL] && <span className="muted small hint own-line">{unavailable[MANUAL]}</span>}
          </label>
        </li>
      </ul>
      {chosen.length > 0 && <p className="muted small">{t("monitor.sessions_note")}</p>}
      {chosen.length > 0 && <p className="muted small">{t("monitor.levels_note")}</p>}
    </>
  );
}

/** Adds an extra account of Claude Code or Codex (Task-A03-oven548): the folder the agent keeps
 * that account in, picked by the person, and the name they give it. Allomos checks the folder and
 * stores only its path and the name; it never reads the login. `onAdded` gets the new monitor agent. */
function AddAccount({
  messenger,
  agent,
  folder,
  suggested,
  onAdded,
  onCancel,
}: {
  messenger: Messenger;
  agent: string;
  folder: string | undefined;
  suggested: string;
  onAdded: (key: string) => void;
  onCancel: () => void;
}) {
  const { t, problem } = useI18n();
  const [path, setPath] = useState<string | null>(null);
  const [name, setName] = useState(suggested);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pick = async () => {
    const picked = await messenger.pickFolder(t("monitor.account_pick_title"), folder);
    if (picked) setPath(picked);
  };
  const add = async () => {
    if (!path || !name.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const account = await messenger.addAgentAccount(agent, path, name.trim());
      onAdded(accountKey(agent, account.id));
    } catch (e) {
      setError(isProblem(e) ? problem(e) : String(e));
      setBusy(false);
    }
  };
  return (
    <div className="codex-rules add-account">
      <h3>{t("monitor.account_title", { agent: t(AGENT_KEY[agent as Agent["id"]]) })}</h3>
      <p className="muted small">{t("monitor.account_note")}</p>
      <p>
        <button type="button" onClick={() => void pick()}>
          {t("monitor.account_pick")}
        </button>{" "}
        {path && <span className="small">{path}</span>}
      </p>
      <p>
        <label className="small">
          {t("monitor.account_name")}{" "}
          <input type="text" value={name} maxLength={40} onChange={(e) => setName(e.target.value)} />
        </label>
      </p>
      {error && <p className="error small">{error}</p>}
      <div className="row">
        <button type="button" onClick={onCancel}>
          {t("wizard.cancel")}
        </button>
        <button type="button" className="primary" disabled={!path || !name.trim() || busy} onClick={() => void add()}>
          {t("monitor.account_add")}
        </button>
      </div>
    </div>
  );
}

/** One agent's difficulty range: the easiest and the hardest level it takes tasks from, each
 * 1 to 5; the hardest is never below the easiest (Task-A03-wave335). */
export function LevelsPicker({
  agent,
  name,
  levels,
  onChange,
}: {
  agent: TerminalAgent;
  name: string;
  levels: Levels;
  onChange: (levels: Levels) => void;
}) {
  const { t } = useI18n();
  return (
    <span className="monitor-sessions monitor-levels small" data-agent={agent}>
      <span className="muted">{t("monitor.levels_label")}</span>{" "}
      <select
        aria-label={t("monitor.levels_from_aria", { agent: name })}
        value={levels.low}
        onChange={(e) => {
          const low = Number(e.target.value);
          onChange({ low, high: Math.max(low, levels.high) });
        }}
      >
        {LEVEL_NUMBERS.map((n) => (
          <option key={n} value={n}>
            {t(LEVEL_KEYS[n])}
          </option>
        ))}
      </select>{" "}
      <span className="muted">–</span>{" "}
      <select
        aria-label={t("monitor.levels_to_aria", { agent: name })}
        value={levels.high}
        onChange={(e) => {
          const high = Number(e.target.value);
          onChange({ low: Math.min(levels.low, high), high });
        }}
      >
        {LEVEL_NUMBERS.map((n) => (
          <option key={n} value={n}>
            {t(LEVEL_KEYS[n])}
          </option>
        ))}
      </select>
    </span>
  );
}
