// The Heartbeat section of the taskbag's settings (Task-A01-osprey500, decision 0022 § 3): what
// a typed value means before it is sent. The same rule as `alm heartbeat-settings`
// (crates/core/src/heartbeat.rs `parse_minutes`): whole minutes, at least 1. A value that breaks it
// is refused with the `heartbeat_minutes_invalid` words, naming the setting as the section shows
// it. Kept apart from React so it can be tested with Node alone.

/** The settings as the desktop app sends them (apps/desktop/src-tauri/src/heartbeat_settings.rs). */
export interface HeartbeatSettings {
  /** As written in project.json; null where the default applies. */
  interval_minutes: number | null;
  lost_after_minutes: number | null;
  /** Written only as false; null where the default (on) applies. */
  push_work: boolean | null;
  applied_interval_minutes: number;
  applied_lost_after_minutes: number;
  applied_push_work: boolean;
  default_interval_minutes: number;
  default_lost_after_minutes: number;
}

/** A typed value: whole minutes (at least 1), or the problem that refuses it. */
export type Checked = { ok: true; minutes: number } | { ok: false; problem: { code: "heartbeat_minutes_invalid"; params: { field: string; value: string } } };

export function checkMinutes(field: string, text: string): Checked {
  const t = text.trim();
  const minutes = /^\d+$/.test(t) ? Number(t) : NaN;
  if (Number.isSafeInteger(minutes) && minutes >= 1) return { ok: true, minutes };
  return { ok: false, problem: { code: "heartbeat_minutes_invalid", params: { field, value: text } } };
}

/** What to send for the fields: only a value that differs from what applies (so an unchanged
 * default stays unwritten), or the first problem. */
export function changeOf(
  s: HeartbeatSettings,
  interval: { field: string; text: string },
  lostAfter: { field: string; text: string },
  pushWork: boolean = s.applied_push_work,
): { ok: true; change: { interval?: string; lostAfter?: string; pushWork?: boolean } } | Extract<Checked, { ok: false }> {
  const i = checkMinutes(interval.field, interval.text);
  if (!i.ok) return i;
  const l = checkMinutes(lostAfter.field, lostAfter.text);
  if (!l.ok) return l;
  const change: { interval?: string; lostAfter?: string; pushWork?: boolean } = {};
  if (i.minutes !== s.applied_interval_minutes) change.interval = String(i.minutes);
  if (l.minutes !== s.applied_lost_after_minutes) change.lostAfter = String(l.minutes);
  if (pushWork !== s.applied_push_work) change.pushWork = pushWork;
  return { ok: true, change };
}

/** Whether all the settings are already their defaults with nothing written, so Restore Defaults
 * has nothing to do. */
export function atDefaults(s: HeartbeatSettings): boolean {
  return s.interval_minutes === null && s.lost_after_minutes === null && s.push_work === null;
}

/** alm tells (as `alm heartbeat-settings` does): a lost limit not longer than the interval lets a
 * task be listed as lost between two regular heartbeats. Allowed, only said. */
export function limitTooShort(intervalMinutes: number, lostAfterMinutes: number): boolean {
  return lostAfterMinutes <= intervalMinutes;
}
