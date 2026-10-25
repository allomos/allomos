// What a phone or another computer sees when it opens the desktop app's address (Task-A04-02-01):
// this interface outside the desktop app, where there is no app to talk to. It pairs the device
// once (Task-A04-02-02, pairFlow.ts): a device name, the person's approval on the computer, then
// the 9-digit code the desktop app shows. Paired, it lists the taskbags the desktop app has open;
// viewing them here comes with A04-02-03.

import { useCallback, useEffect, useState } from "react";
import { useI18n } from "./i18n";
import { NAME_MAX, ask, authHeaders, cleanCode, nameOk, readToken, saveToken, status, typeCode } from "./pairFlow";

type Stage =
  | { stage: "connecting" }
  | { stage: "no_app" }
  | { stage: "name"; removed?: boolean }
  | { stage: "waiting"; request: string }
  | { stage: "code"; request: string; note?: string }
  | { stage: "refused" }
  | { stage: "expired" }
  | { stage: "paired"; device: string; taskbags: string[] };

const store = () => (typeof window === "undefined" ? undefined : window.localStorage);

export function CompanionPage() {
  const { t } = useI18n();
  const [stage, setStage] = useState<Stage>({ stage: "connecting" });
  const [version, setVersion] = useState("");
  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);

  const showPaired = useCallback(async (token: string, device: string) => {
    const r = await fetch("/api/taskbags", { headers: authHeaders(token) });
    const list = r.ok ? ((await r.json()) as { name: string }[]) : [];
    setStage({ stage: "paired", device, taskbags: list.map((b) => b.name) });
  }, []);

  useEffect(() => {
    const token = readToken(store());
    fetch("/api/hello", { headers: authHeaders(token) })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((v: { version: string; device: string | null }) => {
        setVersion(v.version);
        if (token && v.device) return showPaired(token, v.device);
        // A token the desktop app no longer knows: the device was removed there.
        if (token) saveToken(store(), null);
        setStage({ stage: "name", removed: !!token });
      })
      .catch(() => setStage({ stage: "no_app" }));
  }, [showPaired]);

  // While the person decides on the computer, ask every 2 seconds.
  const request = stage.stage === "waiting" ? stage.request : null;
  useEffect(() => {
    if (!request) return;
    const timer = window.setInterval(() => {
      status(fetch, request)
        .then((s) => {
          if (s.status === "approved") setStage({ stage: "code", request });
          else if (s.status === "refused") setStage({ stage: "refused" });
          else if (s.status === "expired") setStage({ stage: "expired" });
        })
        .catch(() => setStage({ stage: "no_app" }));
    }, 2000);
    return () => window.clearInterval(timer);
  }, [request]);

  const sendName = async () => {
    setBusy(true);
    try {
      setStage({ stage: "waiting", request: await ask(fetch, name) });
    } catch {
      setStage({ stage: "no_app" });
    } finally {
      setBusy(false);
    }
  };

  const sendCode = async (request: string) => {
    const clean = cleanCode(code);
    if (!clean) return;
    setBusy(true);
    try {
      const r = await typeCode(fetch, request, clean);
      setCode("");
      if (r.ok) {
        saveToken(store(), r.token);
        await showPaired(r.token, r.device);
      } else if (r.error === "wrong_code") {
        setStage({ stage: "code", request, note: t("companion.wrong_code", { tries: r.tries_left }) });
      } else if (r.error === "new_code") {
        setStage({ stage: "code", request, note: t("companion.new_code") });
      } else if (r.error === "not_approved") {
        setStage({ stage: "expired" });
      } else {
        setStage({ stage: "no_app" });
      }
    } finally {
      setBusy(false);
    }
  };

  const again = () => {
    setName("");
    setStage({ stage: "name" });
  };

  return (
    <main className="companion">
      <h1>{t("brand.name")}</h1>
      {stage.stage === "connecting" && <p className="muted">{t("companion.connecting")}</p>}
      {stage.stage === "no_app" && <p className="banner error">{t("companion.no_app")}</p>}
      {version && stage.stage !== "no_app" && <p className="muted small">{t("companion.connected", { version })}</p>}
      {stage.stage === "name" && (
        <form
          className="form companion-pair"
          onSubmit={(e) => {
            e.preventDefault();
            if (nameOk(name)) void sendName();
          }}
        >
          {stage.removed && <p className="banner">{t("companion.removed")}</p>}
          <p>{t("companion.pair_intro")}</p>
          <label>
            {t("companion.device_name")}
            <input value={name} maxLength={NAME_MAX} autoFocus onChange={(e) => setName(e.target.value)} placeholder={t("companion.device_placeholder")} />
          </label>
          <p className="muted small">{t("companion.name_hint")}</p>
          <div className="row">
            <button type="submit" className="primary" disabled={busy || !nameOk(name)}>
              {t("companion.ask")}
            </button>
          </div>
        </form>
      )}
      {stage.stage === "waiting" && <p className="banner">{t("companion.waiting")}</p>}
      {stage.stage === "code" && (
        <form
          className="form companion-pair"
          onSubmit={(e) => {
            e.preventDefault();
            void sendCode(stage.request);
          }}
        >
          <p>{t("companion.type_code")}</p>
          {stage.note && <p className="banner">{stage.note}</p>}
          <label>
            {t("companion.code")}
            <input value={code} inputMode="numeric" autoComplete="one-time-code" autoFocus onChange={(e) => setCode(e.target.value)} placeholder="482 913 076" />
          </label>
          <div className="row">
            <button type="submit" className="primary" disabled={busy || !cleanCode(code)}>
              {t("companion.pair")}
            </button>
          </div>
        </form>
      )}
      {(stage.stage === "refused" || stage.stage === "expired") && (
        <>
          <p className="banner">{t(stage.stage === "refused" ? "companion.refused" : "companion.expired")}</p>
          <div className="row">
            <button type="button" className="primary" onClick={again}>
              {t("companion.ask_again")}
            </button>
          </div>
        </>
      )}
      {stage.stage === "paired" && (
        <>
          <p className="banner">{t("companion.paired", { device: stage.device })}</p>
          {stage.taskbags.length > 0 ? (
            <>
              <p>{t("companion.open_taskbags")}</p>
              <ul>
                {stage.taskbags.map((b, n) => (
                  <li key={n}>{b}</li>
                ))}
              </ul>
            </>
          ) : (
            <p className="muted">{t("companion.none_open")}</p>
          )}
          <p className="muted small">{t("companion.view_next")}</p>
        </>
      )}
      <p className="muted small">{t("companion.warning")}</p>
    </main>
  );
}
