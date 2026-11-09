// The last heartbeat of a held task and its Lost tag (Task-A01-kernel761; wording and look approved
// by the approver, 2026-11-09). The text and the rule are in heartbeatFlow.ts; this redraws them
// each minute so the age keeps up while the window stays open.

import { useEffect, useState } from "react";
import type { KeyboardEvent } from "react";
import { useI18n } from "./i18n";
import { heartbeatFact, heartbeatText } from "./heartbeatFlow";
import type { Task } from "./types";

/** The time now, renewed every minute. */
function useMinute(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 60_000);
    return () => clearInterval(id);
  }, []);
  return now;
}

/** "12 min ago (14:02) · busy until 16:00", with the Lost tag after it when `withTag`; nothing for a
 * task nobody holds. */
export function HeartbeatLine({ task, lostAfter, lastChange, withTag }: { task: Task; lostAfter: number; lastChange?: string; withTag?: boolean }) {
  const { t } = useI18n();
  const now = useMinute();
  const fact = heartbeatFact(task, lostAfter, now, lastChange);
  if (!fact) return null;
  return (
    <span className="heartbeat">
      {heartbeatText(fact, now, t)}
      {withTag && fact.lost && <> <LostBadge minutes={fact.lostAfter} /></>}
    </span>
  );
}

/** The Lost tag alone, for beside the holder's name; nothing while the task is not lost. */
export function LostTag({ task, lostAfter, lastChange }: { task: Task; lostAfter: number; lastChange?: string }) {
  const now = useMinute();
  const fact = heartbeatFact(task, lostAfter, now, lastChange);
  return fact?.lost ? <LostBadge minutes={fact.lostAfter} /> : null;
}

/** Take over… (Task-A01-voyage603), after the Lost tag: opens the dialog that explains the takeover
 * and opens a new session; nothing while the task is not lost. */
export function TakeOverButton({
  task,
  lostAfter,
  lastChange,
  onKeyDown,
  onTakeOver,
}: {
  task: Task;
  lostAfter: number;
  lastChange?: string;
  onKeyDown?: (e: KeyboardEvent) => void;
  onTakeOver: () => void;
}) {
  const { t } = useI18n();
  const now = useMinute();
  if (!heartbeatFact(task, lostAfter, now, lastChange)?.lost) return null;
  return (
    <button
      type="button"
      className="link with-icon proceed takeover"
      title={t("takeover.button_title")}
      onKeyDown={onKeyDown}
      onClick={(e) => {
        e.stopPropagation();
        onTakeOver();
      }}
    >
      <svg className="button-icon" viewBox="0 0 20 20" aria-hidden="true">
        <path d="M6.5 4.5v11l9-5.5z" />
      </svg>
      {t("takeover.button")}
    </button>
  );
}

function LostBadge({ minutes }: { minutes: number }) {
  const { t } = useI18n();
  return (
    <span className="lost-tag" title={t("heartbeat.lost_title", { minutes })}>
      {t("heartbeat.lost")}
    </span>
  );
}
