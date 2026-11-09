// The last heartbeat of a held task and its Lost tag (Task-A01-kernel761; wording and look approved
// by the approver, 2026-11-09). The text and the rule are in heartbeatFlow.ts; this redraws them
// each minute so the age keeps up while the window stays open.

import { useEffect, useState } from "react";
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

function LostBadge({ minutes }: { minutes: number }) {
  const { t } = useI18n();
  return (
    <span className="lost-tag" title={t("heartbeat.lost_title", { minutes })}>
      {t("heartbeat.lost")}
    </span>
  );
}
