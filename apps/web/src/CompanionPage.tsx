// What a phone or another computer sees when it opens the desktop app's address (Task-A04-02-01):
// this interface outside the desktop app, where there is no app to talk to. For now it says it is
// connected and why the browser warned; pairing (A04-02-02) and the read-only view of the taskbag
// (A04-02-03) come next.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";

export function CompanionPage() {
  const { t } = useI18n();
  const [hello, setHello] = useState<{ version: string } | null | "failed">(null);
  useEffect(() => {
    fetch("/api/hello")
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((v: { version: string }) => setHello(v))
      .catch(() => setHello("failed"));
  }, []);
  return (
    <main className="companion">
      <h1>{t("brand.name")}</h1>
      {hello === null ? (
        <p className="muted">{t("companion.connecting")}</p>
      ) : hello === "failed" ? (
        <p className="banner error">{t("companion.no_app")}</p>
      ) : (
        <>
          <p>{t("companion.connected", { version: hello.version })}</p>
          <p className="muted small">{t("companion.pairing_next")}</p>
        </>
      )}
      <p className="muted small">{t("companion.warning")}</p>
    </main>
  );
}
