// The app's own About window (Task-A07-28), the same on macOS and Windows: the Allomos icon, the
// name and version, the slogan, www.allomos.com as a real link that opens in the default browser,
// and the licence. Its words follow the interface's language. Escape closes it.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { AboutInfo, Messenger } from "./messenger";

export function AboutWindow({ messenger }: { messenger: Pick<Messenger, "aboutInfo" | "openWebsite" | "closeWindow"> }) {
  const { t } = useI18n();
  const [info, setInfo] = useState<AboutInfo | null>(null);
  useEffect(() => {
    messenger.aboutInfo().then(setInfo).catch(() => undefined);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void messenger.closeWindow().catch(() => undefined);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [messenger]);
  useEffect(() => {
    document.title = t("brand.name");
  }, [t]);
  return (
    <main className="about-window">
      {/* The Allomos mark, as in the top bar and the app icon. */}
      <svg className="about-icon brand-mark" viewBox="100 100 824 824" aria-hidden="true">
        <circle className="ring jade" cx="265" cy="512" r="105" />
        <circle className="jade" cx="759" cy="512" r="138" />
        <circle className="ring terracotta" cx="512" cy="265" r="105" />
        <circle className="terracotta" cx="512" cy="759" r="138" />
        <path className="spark" d="M512 392 L552 472 L632 512 L552 552 L512 632 L472 552 L392 512 L472 472 Z" />
      </svg>
      <h1>{t("brand.name")}</h1>
      {info && <p className="about-version">{t("about.version", { version: info.version })}</p>}
      <p className="about-tagline">{t("about.tagline")}</p>
      <button
        type="button"
        className="link about-website"
        aria-label={t("about.website_aria")}
        onClick={() => void messenger.openWebsite().catch(() => undefined)}
      >
        www.allomos.com
      </button>
      <p className="about-licence small">{t("about.licence")}</p>
    </main>
  );
}
