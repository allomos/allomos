// Update the taskbag… (Task-A00-30; named so by the approver, 2026-10-17; one button for card files
// and an older Allomos's taskbag since Task-A01-45, approver 2026-10-16): from the notice "N tasks
// keep their card files in the project folder", the notice "This taskbag was written by an older
// Allomos" (after the old folder is renamed aside), or File → Update the Taskbag…. The dialog
// first says what Allomos found (each task, its title and where its card file is now, or the older
// taskbag) and what updating does. The person chooses an agent (the ones found, or Manual);
// Allomos opens it in the project folder with the request for whichever applies typed in (`alm
// move-in` prints it), and sends nothing. The agent shows its plan in Allomos with `alm confirm`,
// and changes nothing until the person chooses Go ahead. Manual shows the same request with Copy.
// It also renews an out-of-date Allomos section in the instruction files (Task-A03-27: one of the
// three cases the taskbag needs updating). Which agents it offers follows the approver's rule
// (2026-10-18, `updateTaskbagFlow.ts`): only the agent used last while it is not registered under
// `## Agents set up`, else every agent. A taskbag with no tasks is not a reason to show it, but
// File → Update the Taskbag… then asks the agent to look for the tasks the project already keeps.
// Old handoff files in the taskbag's `handoffs/` are a case too (Task-A03-polar770): the agent
// moves what is still useful into the tasks they name and removes them.
// `MoveCardsPanel` is the dialog's content, also shown as the setup wizard's step 4 when a
// connected project has card files to move in (Task-A05-09).

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { AGENT_KEY, AgentPicker, PASTE_KEYS, useAgents } from "./AgentPicker";
import { MANUAL } from "./agentPickerFlow";
import type { OfferFacts } from "./AgentPicker";
import { lastUsed, readOpened, saveOpened } from "./updateTaskbagFlow";
import { agentOpened } from "./ProgressDialog";
import { COPIED_MS } from "./manualFlow";
import { isProblem } from "./types";
import type { CardRow, MoveIn } from "./moveCardsFlow";

export function MoveCardsDialog({
  messenger,
  root,
  found,
  handoffs,
  moveIn,
  section,
  noTasks,
  onClose,
}: {
  messenger: Messenger;
  /** The open taskbag. */
  root: string;
  /** An older taskbag's handoff files, to move into their tasks (Task-A03-polar770). */
  handoffs?: string[];
  /** The instruction files whose Allomos section is out of date, as one text (Task-A03-27). */
  section?: string | null;
  /** The taskbag has no tasks yet: the agent looks for the project's own (Task-A03-24). */
  noTasks?: boolean;
  /** What Allomos found: the tasks whose card file is still outside the taskbag. */
  found: CardRow[];
  /** An older taskbag renamed aside, whose tasks are to be moved in (Task-A01-45). */
  moveIn?: MoveIn | null;
  /** The dialog closes; `message` says what happened (null: Cancel). */
  onClose: (message: string | null) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="overlay" role="presentation">
      <section className="dialog wizard" role="dialog" aria-modal="true" aria-labelledby="cards-title">
        <h2 id="cards-title">{t("cards.title")}</h2>
        <MoveCardsPanel messenger={messenger} root={root} found={found} handoffs={handoffs} moveIn={moveIn} section={section} noTasks={noTasks} onClose={onClose} />
      </section>
    </div>
  );
}

/** What Allomos found, what updating does, the agent picker, and Open (Cancel and Open, or Done
 * after Manual). */
export function MoveCardsPanel({
  messenger,
  root,
  found,
  handoffs = [],
  moveIn,
  section = null,
  noTasks = false,
  onClose,
  actionsIn,
  cancelLabel,
  extra,
}: {
  messenger: Messenger;
  root: string;
  found: CardRow[];
  /** An older taskbag's handoff files: the request moves them into their tasks (Task-A03-polar770). */
  handoffs?: string[];
  /** An older taskbag renamed aside: the dialog asks for its tasks to be moved in (Task-A01-45). */
  moveIn?: MoveIn | null;
  /** The instruction files whose Allomos section is out of date: the request renews it too. */
  section?: string | null;
  /** The taskbag has no tasks yet: the request asks the agent to look for the project's own. */
  noTasks?: boolean;
  onClose: (message: string | null) => void;
  /** Where the buttons go (the wizard's bottom row); otherwise a row of their own. */
  actionsIn?: HTMLElement | null;
  /** The cancel button's words (default: Cancel). */
  cancelLabel?: string;
  /** Shown under the explanation (the wizard's Connect an agent anyway). */
  extra?: ReactNode;
}) {
  const { t, problem, language } = useI18n();
  // The agent used last and who is registered, for the choice of agents (Task-A03-27).
  const [facts, setFacts] = useState<OfferFacts | null>(null);
  useEffect(() => {
    let live = true;
    setFacts(null);
    void Promise.all([messenger.lastAgent(root).catch(() => null), messenger.agentsSetUp(root).catch(() => null)]).then(
      ([last, entries]) => live && setFacts({ last: lastUsed(last, readOpened(root)), entries }),
    );
    return () => {
      live = false;
    };
  }, [messenger, root]);
  // The shared agent picker (Task-A05-07), narrowed by that rule.
  const { agents, slow, choice: agent, setChoice: setAgent, tool, offer } = useAgents(messenger, facts);
  /** The one agent offered, when the rule offers only it. */
  const only = offer && offer.kind !== "all" ? offer.group : null;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** Manual: the request to copy. */
  const [request, setRequest] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  /** The list of what Allomos found, in a popup of its own (approver, 2026-10-17). */
  const [listing, setListing] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  /** Whether card files are part of it: some were found, or nothing else is to be done (then the
   * agent looks for card files no link names, as before). */
  const cardsPart = found.length > 0 || (!section && !noTasks && handoffs.length === 0);

  const fail = (e: unknown) => setError(isProblem(e) ? problem(e) : t("error.unexpected", { detail: String(e) }));

  const open = async () => {
    if (!agent) return;
    setBusy(true);
    setError(null);
    try {
      if (agent === MANUAL) {
        setRequest(await messenger.moveTasksInRequest(root, language));
        return;
      }
      await messenger.moveTasksIn(agent, root, language);
      saveOpened(root, agent);
      agentOpened(root, moveIn ? "move_in" : section ? "update" : "cards");
      const name = t(AGENT_KEY[agent]);
      const kind = moveIn ? "move" : section ? "update" : cardsPart ? "cards" : handoffs.length > 0 ? "old_handoffs" : "empty";
      if (tool?.how === "paste") {
        onClose(t(`${kind}.asked_paste`, { agent: name, keys: PASTE_KEYS }));
      } else {
        onClose(t(`${kind}.asked`, { agent: name }));
      }
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (request === null) return;
    setError(null);
    try {
      await messenger.copyText(request);
      setCopied(true);
      window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
    } catch (e) {
      fail(e);
    }
  };

  const actions =
    request === null ? (
      <>
        <button type="button" onClick={() => onClose(null)} disabled={busy}>
          {cancelLabel ?? t("wizard.cancel")}
        </button>
        <button type="button" className="primary" onClick={() => void open()} disabled={busy || !agent}>
          {t("cards.open")}
        </button>
      </>
    ) : (
      <button type="button" className="primary" onClick={() => onClose(null)}>
        {t("manual.done")}
      </button>
    );

  const chooser = (
    <>
      {request === null ? (
        <AgentPicker
          agents={agents}
          slow={slow}
          choice={agent}
          onChoose={setAgent}
          disabled={busy}
          name="cards-agent"
          manualTo={only ?? undefined}
        />
      ) : (
        <div className="manual">
          <p className="small">{only ? t("cards.manual_intro_to", { agent: only }) : t("cards.manual_intro")}</p>
          <textarea
            className="manual-request"
            readOnly
            rows={12}
            aria-label={t("manual.request")}
            value={request}
            onFocus={(e) => e.currentTarget.select()}
          />
          <div className="row start">
            <button type="button" onClick={() => void copy()}>
              {t("manual.copy")}
            </button>
            <span className="muted small" role="status">
              {copied ? t("manual.copied") : ""}
            </span>
          </div>
        </div>
      )}
    </>
  );

  return (
    <>
      {request === null && moveIn && (
        <>
          <p>{t("move.found", { found: moveIn.found, old: moveIn.old })}</p>
          <p className="muted small">{t("move.body")}</p>
        </>
      )}
      {request === null && !moveIn && section && (
        <>
          <p>{t("update.notice_section", { files: section })}</p>
          {!cardsPart && <p className="muted small">{t("update.body")}</p>}
        </>
      )}
      {request === null && !moveIn && cardsPart && (
        <>
          <p>{found.length > 0 ? t("cards.found", { count: found.length }) : t("cards.found_none")}</p>
          {found.length > 0 && (
            <p>
              <button type="button" onClick={() => setListing(true)}>
                {t("cards.show_list", { count: found.length })}
              </button>
            </p>
          )}
          <p>{t("cards.update")}</p>
          <p className="muted small">{t("cards.body")}</p>
        </>
      )}

      {request === null && !moveIn && handoffs.length > 0 && (
        <>
          <p>{t("old_handoffs.found", { count: handoffs.length })}</p>
          <ul className="card-found small">
            {handoffs.map((h) => (
              <li key={h}>
                <code>{h}</code>
              </li>
            ))}
          </ul>
          <p className="muted small">{t("old_handoffs.body")}</p>
        </>
      )}

      {request === null && !moveIn && noTasks && (
        <>
          <p>{t("empty.found")}</p>
          <p className="muted small">{t("empty.body")}</p>
        </>
      )}
      {/* Only the agent used last, not registered yet (Task-A03-27): say why. */}
      {request === null && offer?.kind === "only" && <p className="small">{t("cards.only_agent", { agent: offer.group })}</p>}
      {request === null && offer?.kind === "manual" && <p className="small">{t("cards.only_manual", { agent: offer.group })}</p>}

      {extra}

      {/* In the wizard its body already scrolls; on its own, this part scrolls. */}
      {actionsIn === undefined ? <div className="dialog-body wizard-body">{chooser}</div> : chooser}

      {error && <p className="banner error">{error}</p>}

      {actionsIn ? createPortal(actions, actionsIn) : actionsIn === undefined && <div className="row">{actions}</div>}

      {listing &&
        createPortal(
          <div className="overlay above" role="presentation">
            <section className="dialog card-list-dialog" role="dialog" aria-modal="true" aria-labelledby="cards-list-title">
              <h2 id="cards-list-title">{t("cards.list_title", { count: found.length })}</h2>
              <ul className="dialog-body card-found small">
                {found.map((c) => (
                  <li key={c.task}>
                    <strong>{c.task}</strong> {c.title} — <code>{c.path}</code>
                    {c.reported && <span className="muted"> ({t("cards.reported")})</span>}
                  </li>
                ))}
              </ul>
              <div className="row">
                <button type="button" className="primary" onClick={() => setListing(false)}>
                  {t("progress.close")}
                </button>
              </div>
            </section>
          </div>,
          document.body,
        )}
    </>
  );
}
