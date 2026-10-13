// What the **Install the alm command** offer says (Task-A07-12). Kept apart from React so it can be
// tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { AlmStatus } from "./types";

export type AlmLine = {
  key: "alm.offer_missing" | "alm.offer_other" | "alm.replaces_file" | "alm.shadowed";
  params?: Record<string, string>;
};

/** The offer's sentences, in order; empty for no offer. The app never installs by itself: the
 * person clicks, and a real file it would replace is named first (owner, 2026-10-13). */
export function almOfferLines(status: AlmStatus | null): AlmLine[] {
  if (!status?.offer) return [];
  const lines: AlmLine[] = [
    status.found ? { key: "alm.offer_other", params: { found: status.found } } : { key: "alm.offer_missing" },
  ];
  if (status.replaces_file) lines.push({ key: "alm.replaces_file", params: { file: status.replaces_file } });
  if (status.shadowed_by) lines.push({ key: "alm.shadowed", params: { found: status.shadowed_by } });
  return lines;
}
