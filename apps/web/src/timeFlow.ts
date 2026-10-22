// Times as the person reads them, kept apart from React so they can be tested with Node alone
// (`npm --prefix apps/web test`).

/** `2026-10-12T11:30Z` (or with seconds, `…T11:30:05Z`, as events have) as the person's local
 * date and time; the text as it is when it is not one. */
export function localTime(stamp: string): string {
  const d = new Date(stamp.replace(/T(\d\d:\d\d)Z$/, "T$1:00Z"));
  if (Number.isNaN(d.getTime())) return stamp;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
