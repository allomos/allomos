import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import type { VersionCheck } from "./types";
import { updateNotice, updateTaskbagLines, upgradeFrom } from "./versionFlow";

/** How often the notice checks again: an agent's update shows without reopening. */
const VERSION_CHECK_MS = 4000;

/** The one **Update the taskbag…** banner (Task-A03-train969): every reason the taskbag needs
 * updating, each on a line of its own, under a single button that opens the dialog covering them
 * all (Task-A03-27). The reasons: Allomos's instructions are out of date (Task-A01-25); the taskbag
 * was written by an older Allomos (Task-A01-45: the old folder is renamed aside, a new taskbag
 * created, and an agent moves the tasks across); card files still in the project folder
 * (Task-A00-30, A01-51); old handoff files (Task-A03-polar770). The version is asked when the
 * folder opens (every way of opening one) and every few seconds while it is open. Allomos never
 * changes the tasks or the instructions itself. */
export function UpdateTaskbagNotice({
  messenger,
  root,
  cards,
  handoffs,
  onUpdate,
  onOlder,
}: {
  messenger: Messenger;
  root: string;
  /** How many tasks keep their card files in the project folder. */
  cards: number;
  /** How many old handoff files the taskbag still keeps. */
  handoffs: number;
  /** Update the taskbag…: rename an older taskbag aside (unless it already is), then open the
   * dialog that asks an agent for whichever update applies. */
  onUpdate: () => Promise<void>;
  /** Whether this notice says the taskbag is from an older Allomos, so the problem list does not
   * say it again. */
  onOlder: (shown: boolean) => void;
}) {
  const { t } = useI18n();
  const [check, setCheck] = useState<VersionCheck | null>(null);
  useEffect(() => {
    let live = true;
    setCheck(null);
    const look = () =>
      messenger
        .versionCheck(root)
        .then((c) => live && setCheck(c))
        .catch(() => undefined);
    void look();
    const timer = window.setInterval(() => void look(), VERSION_CHECK_MS);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [messenger, root]);
  const [busy, setBusy] = useState(false);
  const notice = updateNotice(check);
  const older = notice?.kind === "move_in";
  useEffect(() => {
    onOlder(older);
    return () => onOlder(false);
  }, [older, onOlder]);
  const lines = updateTaskbagLines(notice, cards, handoffs, upgradeFrom(check), check?.format_current);
  if (lines.length === 0) return null;
  const update = () => {
    setBusy(true);
    void onUpdate().finally(() => {
      setBusy(false);
      setCheck(null);
    });
  };
  return (
    <div className="banner update" role="status">
      <span>
        {lines.map((l) => (
          <span key={l.key} className="banner-line">
            {t(l.key, l.params)}
          </span>
        ))}
      </span>
      <button type="button" className="primary" onClick={update} disabled={busy}>
        {t("cards.button")}
      </button>
    </div>
  );
}
