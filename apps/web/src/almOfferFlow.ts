// What the **Install the alm command** offer says (Task-A07-12, A07-15). Kept apart from React so it
// can be tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { AlmStatus } from "./types";

export type AlmLine = {
  key:
    | "alm.offer_missing"
    | "alm.offer_development"
    | "alm.offer_other"
    | "alm.replaces_file"
    | "alm.shadowed"
    | "alm.in_use"
    | "alm.in_use_link"
    | "alm.own";
  params?: Record<string, string>;
};

export type AlmOffer = {
  /** The sentences, in order. */
  lines: AlmLine[];
  /** The `alm` in use and the app's own, one per line. */
  paths: AlmLine[];
  button: "alm.install" | "alm.switch";
};

/** The offer; `null` for no offer. The app never changes the link by itself: the person clicks.
 * The latest install, app or development build, decides which `alm` Terminal uses (owner, 2026-10-14). */
export function almOffer(status: AlmStatus | null): AlmOffer | null {
  if (!status?.offer) return null;
  const kind = status.kind ?? (status.found ? "other" : "missing");
  const lines: AlmLine[] = [
    { key: kind === "missing" ? "alm.offer_missing" : kind === "development" ? "alm.offer_development" : "alm.offer_other" },
  ];
  if (status.replaces_file) lines.push({ key: "alm.replaces_file", params: { file: status.replaces_file } });
  if (status.shadowed_by) lines.push({ key: "alm.shadowed", params: { found: status.shadowed_by } });
  const paths: AlmLine[] = [];
  if (status.found) {
    paths.push(
      status.leads_to
        ? { key: "alm.in_use_link", params: { found: status.found, target: status.leads_to } }
        : { key: "alm.in_use", params: { found: status.found } },
    );
  }
  if (status.own) paths.push({ key: "alm.own", params: { own: status.own } });
  return { lines, paths, button: kind === "development" ? "alm.switch" : "alm.install" };
}

/** What "Don't ask again" remembers (Task-A07-14): the mismatch this offer is about, the path
 * Terminal's `alm` leads to (empty when Terminal finds none). The app asks again when it changes. */
export function almDismissFor(status: AlmStatus | null): string {
  return status?.mismatch ?? "";
}
