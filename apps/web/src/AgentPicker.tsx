// The shared agent picker (Task-A05-07, approver 2026-10-17): the choice first made in Connect
// with the agents, now used by every feature that opens an AI agent. Agents are grouped (Claude,
// Codex), each with its app and terminal as sub-choices; how the chosen tool will open is shown
// only for it; Manual comes last. Each feature keeps its own request, buttons and Manual text:
// `useAgents` finds the agents and holds the choice, `AgentPicker` draws it.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useI18n } from "./i18n";
import type { Key } from "./i18n";
import type { Messenger } from "./messenger";
import type { Agent } from "./types";
import { MANUAL, chooseGroup, firstChoice, foundGroups, groupOf } from "./agentPickerFlow";
import type { Choice } from "./agentPickerFlow";
import { offer as offerFor, offered } from "./updateTaskbagFlow";
import type { LastUsed, Offer } from "./updateTaskbagFlow";

export const AGENT_KEY: Record<Agent["id"], Key> = {
  claude_cli: "agents.claude_cli",
  claude_app: "agents.claude_app",
  codex_cli: "agents.codex_cli",
  codex_app: "agents.codex_app",
};

/** The keys that paste in a terminal on this computer. */
export const PASTE_KEYS = /Mac/.test(navigator.userAgent) ? "⌘V" : "Ctrl+V";

/** Show "Looking for AI agents…" only when finding them is slow. */
const SPINNER_AFTER_MS = 200;

/** What Update the taskbag…'s choice of agents is worked out from (Task-A03-27): the agent used
 * last, and the entries under `## Agents set up` (null: unreadable). */
export interface OfferFacts {
  last: LastUsed | null;
  entries: string[] | null;
}

/** The agents on this computer and the person's choice; Manual alone when finding them fails.
 * `facts` narrows them by the rule of Update the taskbag… (Task-A03-27); null while they are still
 * being read (shown as looking); left out: every agent found. */
export function useAgents(messenger: Messenger, facts?: OfferFacts | null) {
  const [found, setFound] = useState<Agent[] | null>(null);
  const [slow, setSlow] = useState(false);
  const [choice, setChoice] = useState<Choice | null>(null);
  useEffect(() => {
    let live = true;
    const timer = window.setTimeout(() => live && setSlow(true), SPINNER_AFTER_MS);
    messenger
      .findAgents()
      .then((list) => live && setFound(list))
      .catch(() => live && setFound([]))
      .finally(() => window.clearTimeout(timer));
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [messenger]);
  const offer = useMemo<Offer | null>(
    () => (found === null || facts === null ? null : facts === undefined ? { kind: "all" } : offerFor(found, facts.last, facts.entries)),
    [found, facts],
  );
  const agents = useMemo(() => (found === null || offer === null ? null : offered(found, offer)), [found, offer]);
  useEffect(() => {
    if (agents) setChoice(firstChoice(agents));
  }, [agents]);
  /** The chosen tool as found (null for Manual or while looking). */
  const tool = agents?.find((a) => a.id === choice) ?? null;
  return { agents, slow, choice, setChoice, tool, offer };
}

export function AgentPicker({
  agents,
  slow,
  choice,
  onChoose,
  disabled,
  name,
  manual,
  manualTo,
}: {
  /** The agents found; null while looking. */
  agents: Agent[] | null;
  /** Looking takes a while: show "Looking for AI agents…". */
  slow: boolean;
  choice: Choice | null;
  onChoose: (choice: Choice) => void;
  disabled?: boolean;
  /** The radio buttons' name, unique in the window. */
  name: string;
  /** Shown under Manual while it is chosen (for example the request with Copy). */
  manual?: ReactNode;
  /** The one agent the request is for (Update the taskbag…, Task-A03-27): Manual says to paste
   * it into that agent rather than any. */
  manualTo?: string;
}) {
  const { t } = useI18n();
  /** The tool last chosen in each group, so choosing the group again keeps it. */
  const [last, setLast] = useState<Partial<Record<string, Agent["id"]>>>({});
  useEffect(() => {
    const group = groupOf(choice);
    if (choice && choice !== MANUAL && group) setLast((l) => ({ ...l, [group]: choice }));
  }, [choice]);

  if (agents === null) {
    return slow ? (
      <p className="muted searching" role="status">
        <span className="spinner" aria-hidden="true" /> {t("agents.searching")}
      </p>
    ) : null;
  }
  const chosenGroup = groupOf(choice);
  return (
    <ul className="agent-list">
      {foundGroups(agents).map((g) => {
        const on = chosenGroup === g.name;
        return (
          <li key={g.name}>
            <label>
              <input
                type="radio"
                name={`${name}-group`}
                checked={on}
                onChange={() => onChoose(chooseGroup(agents, g.name, last))}
                disabled={disabled}
              />{" "}
              <strong>{g.name}</strong>
            </label>
            <ul className="agent-tools">
              {g.tools.map((a) => {
                const picked = on && choice === a.id;
                return (
                  <li key={a.id}>
                    <label className={on ? undefined : "muted"}>
                      {g.tools.length > 1 && (
                        <>
                          <input
                            type="radio"
                            name={`${name}-tool-${g.name}`}
                            checked={on ? choice === a.id : (last[g.name] ?? g.tools[0].id) === a.id}
                            onChange={() => onChoose(a.id)}
                            disabled={disabled || !on}
                          />{" "}
                        </>
                      )}
                      {t(AGENT_KEY[a.id])}{" "}
                      {/* How it will open: only for the chosen tool (owner, 2026-10-12). */}
                      {picked && (
                        <span className="muted">
                          — {t(a.how === "link" ? "agents.how_link" : "agents.how_paste")} ({a.found})
                        </span>
                      )}
                      {/* The Claude app asks once per folder; cancelling it starts the session in no folder. */}
                      {a.id === "claude_app" && <span className="muted small hint">{t("agents.claude_app_trust")}</span>}
                      {/* The chosen tool takes the request from the clipboard: say how, right here. */}
                      {picked && a.how === "paste" && (
                        <span className="small hint paste-tip">{t("agents.paste_tip", { keys: PASTE_KEYS })}</span>
                      )}
                    </label>
                  </li>
                );
              })}
            </ul>
          </li>
        );
      })}
      {/* Manual (Task-A05-06): any AI, even when none is found; the person pastes the request. */}
      <li>
        <label>
          <input type="radio" name={`${name}-group`} checked={choice === MANUAL} onChange={() => onChoose(MANUAL)} disabled={disabled} />{" "}
          <strong>{t("agents.manual")}</strong>{" "}
          <span className="muted">— {manualTo ? t("agents.manual_how_to", { agent: manualTo }) : t("agents.manual_how")}</span>
        </label>
        {choice === MANUAL && manual}
      </li>
    </ul>
  );
}
