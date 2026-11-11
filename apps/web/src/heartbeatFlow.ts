// The last heartbeat of a held task and whether it is lost, as the task list and the task pane show
// them (Task-A01-kernel761; decision 0022 § 3). The same rule as `alm next`
// (crates/core/src/heartbeat.rs `lost`): only the holder's own heartbeat counts; the limit counts
// from the later of the heartbeat and the end of its busy window; while a busy window runs the task
// is never lost; a holder that never beat counts from the task's last change. The app only shows
// this: it never takes a task over. Kept apart from React so it can be tested with Node alone.

import type { Key } from "./i18n";
import type { Project, Task } from "./types";

/** The default limit, in minutes (decision 0022 § 3). */
export const LOST_AFTER_MINUTES = 30;

/** The default heartbeat interval, in minutes (decision 0022 § 3). */
export const INTERVAL_MINUTES = 10;

/** The project's limit in minutes; a missing or non-positive value takes the default. */
export function lostAfterMinutes(project?: Project | null): number {
  const m = project?.heartbeat?.lost_after_minutes;
  return typeof m === "number" && m > 0 ? Math.floor(m) : LOST_AFTER_MINUTES;
}

export interface HeartbeatFact {
  /** The holder's last heartbeat (UTC); null when it never beat. */
  at: string | null;
  /** Whole minutes since `at`; null with no heartbeat. */
  minutes: number | null;
  /** The end of a busy window that is still running (UTC); null otherwise. */
  busyUntil: string | null;
  /** What the holder last said it is doing (`alm beat --doing`); null when it said nothing. */
  doing: string | null;
  /** When it said so (UTC); null when not known. */
  doingAt: string | null;
  lost: boolean;
  /** The limit that applied, in minutes. */
  lostAfter: number;
}

const parse = (stamp: string | null | undefined): number | null => {
  if (!stamp) return null;
  const ms = new Date(stamp.replace(/T(\d\d:\d\d)Z$/, "T$1:00Z")).getTime();
  return Number.isNaN(ms) ? null : ms;
};

/** The project's heartbeat interval in minutes; a missing or non-positive value takes the default. */
export function intervalMinutes(project?: Project | null): number {
  const m = project?.heartbeat?.interval_minutes;
  return typeof m === "number" && m > 0 ? Math.floor(m) : INTERVAL_MINUTES;
}

/** The heartbeat fact of a task; null when it is not in progress or nobody holds it (no heartbeat
 * to show). `lastChange`: when the task last changed (UTC), for a holder that never beat. */
export function heartbeatFact(task: Task, lostAfter: number, now: Date, lastChange?: string): HeartbeatFact | null {
  return task.state === "in_progress" ? heldFact(task, lostAfter, now, lastChange) : null;
}

/** The same fact for a holder in any state, for Take over… (Task-A01-rocket260): a waiting or blocked
 * task keeps its holder, whose last heartbeat is still worth saying; only an in-progress task can be
 * lost. Null when nobody holds it. */
export function heldFact(task: Task, lostAfter: number, now: Date, lastChange?: string): HeartbeatFact | null {
  if (!task.holder) return null;
  const mine = task.heartbeat && task.heartbeat.by === task.holder ? task.heartbeat : null;
  const at = parse(mine?.at);
  const until = parse(mine?.busy_until);
  const nowMs = now.getTime();
  const running = until !== null && until > nowMs;
  const since = at !== null || until !== null ? Math.max(at ?? -Infinity, until ?? -Infinity) : parse(lastChange);
  const lost = task.state === "in_progress" && !running && since !== null && nowMs - since > lostAfter * 60_000;
  return {
    at: at !== null ? mine!.at : null,
    minutes: at !== null ? Math.max(0, Math.floor((nowMs - at) / 60_000)) : null,
    busyUntil: running ? mine!.busy_until! : null,
    doing: mine?.doing ?? null,
    doingAt: mine?.doing ? (mine.doing_at ?? null) : null,
    lost,
    lostAfter,
  };
}

type T = (key: Key, params?: Record<string, string | number>) => string;

/** How long ago, in the largest whole unit: just now, minutes, hours, days. */
export function agoText(minutes: number, t: T): string {
  if (minutes < 1) return t("heartbeat.just_now");
  if (minutes < 60) return t("heartbeat.minutes_ago", { n: minutes });
  if (minutes < 24 * 60) return t("heartbeat.hours_ago", { n: Math.floor(minutes / 60) });
  return t("heartbeat.days_ago", { n: Math.floor(minutes / (24 * 60)) });
}

/** A time as the person reads it: the local clock time on the same local day as `now`, otherwise
 * the local date and time. */
export function clockText(stamp: string, now: Date): string {
  const ms = parse(stamp);
  if (ms === null) return stamp;
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  const clock = `${p(d.getHours())}:${p(d.getMinutes())}`;
  const sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
  return sameDay ? clock : `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${clock}`;
}

/** What the holder last said it is doing, with the time it said so; null when it said nothing. */
export function doingText(fact: HeartbeatFact, now: Date, t: T): string | null {
  if (!fact.doing) return null;
  return fact.doingAt ? t("heartbeat.doing_at", { doing: fact.doing, time: clockText(fact.doingAt, now) }) : t("heartbeat.doing", { doing: fact.doing });
}

/** The heartbeat line: "12 min ago (14:02)", or "No record"; then "· busy until 16:00" while a
 * busy window runs, and "· doing: … (14:00)" when the holder said what it is doing. The Lost tag is
 * shown beside it, not in this text. */
export function heartbeatText(fact: HeartbeatFact, now: Date, t: T): string {
  const when =
    fact.at !== null && fact.minutes !== null
      ? t("heartbeat.ago", { ago: agoText(fact.minutes, t), time: clockText(fact.at, now) })
      : t("heartbeat.none");
  const busy = fact.busyUntil ? t("heartbeat.busy", { time: clockText(fact.busyUntil, now) }) : null;
  return [when, busy, doingText(fact, now, t)].filter(Boolean).join(" · ");
}
