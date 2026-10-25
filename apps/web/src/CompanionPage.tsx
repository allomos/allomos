// What a phone or another computer sees when it opens the desktop app's address (Task-A04-02-01):
// this interface outside the desktop app, where there is no app to talk to. It pairs the device
// once (Task-A04-02-02, pairFlow.ts): a device name, the person's approval on the computer, then
// the 9-digit code the desktop app shows. Paired, it shows the taskbag the desktop app has open
// with the desktop app's own interface (Task-A04-02-03: `App` with the companion's messenger,
// `lanMessenger.ts`), or lets the person choose when several are open. When the desktop app
// cannot be reached, it shows no data, says to open Allomos on the computer, and connects again
// by itself once it can.

import { useCallback, useEffect, useMemo, useState } from "react";
import { App } from "./App";
import { RETRY_MS, WATCH_MS, shownRoot, type OpenTaskbag } from "./companionFlow";
import { useI18n } from "./i18n";
import { lanMessenger } from "./lanMessenger";
import { NAME_MAX, ask, authHeaders, cleanCode, nameOk, readToken, saveToken, status, typeCode } from "./pairFlow";
import { withDecisions } from "./stopFlow";

type Stage =
  | { stage: "connecting" }
  | { stage: "no_app" }
  | { stage: "name"; removed?: boolean }
  | { stage: "waiting"; request: string }
  | { stage: "code"; request: string; note?: string }
  | { stage: "refused" }
  | { stage: "expired" }
  | { stage: "paired"; device: string; taskbags: OpenTaskbag[] };

const store = () => (typeof window === "undefined" ? undefined : window.localStorage);

export function CompanionPage() {
  const { t } = useI18n();
  const [stage, setStage] = useState<Stage>({ stage: "connecting" });
  const [version, setVersion] = useState("");
  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [token, setToken] = useState(() => readToken(store()));
  const [computer, setComputer] = useState("");
  /** The taskbag chosen when several are open. */
  const [chosen, setChosen] = useState<string | null>(null);

  const showPaired = useCallback(async (token: string, device: string) => {
    const r = await fetch("/api/taskbags", { headers: authHeaders(token) });
    const list = r.ok ? ((await r.json()) as OpenTaskbag[]) : [];
    setStage({ stage: "paired", device, taskbags: list });
  }, []);

  /** Ask the desktop app who this device is, and go on from there. */
  const start = useCallback(() => {
    const token = readToken(store());
    setToken(token);
    return fetch("/api/hello", { headers: authHeaders(token) })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((v: { version: string; device: string | null; computer?: string }) => {
        setVersion(v.version);
        setComputer(v.computer ?? "");
        if (token && v.device) return showPaired(token, v.device);
        // A token the desktop app no longer knows: the device was removed there.
        if (token) saveToken(store(), null);
        setStage({ stage: "name", removed: !!token });
      })
      .catch(() => setStage({ stage: "no_app" }));
  }, [showPaired]);

  useEffect(() => {
    void start();
  }, [start]);

  // The desktop app is closed (or not reachable): look again every few seconds (R83).
  const down = stage.stage === "no_app";
  useEffect(() => {
    if (!down) return;
    const timer = window.setInterval(() => void start(), RETRY_MS);
    return () => window.clearInterval(timer);
  }, [down, start]);

  // Paired: follow which taskbags the desktop app has open (a window opened or closed).
  const pairedDevice = stage.stage === "paired" ? stage.device : null;
  useEffect(() => {
    if (!pairedDevice || !token) return;
    const timer = window.setInterval(() => {
      fetch("/api/taskbags", { headers: authHeaders(token) })
        .then((r) => {
          if (r.status === 403) return start();
          if (!r.ok) return setStage({ stage: "no_app" });
          return r.json().then((list: OpenTaskbag[]) =>
            setStage((s) => (s.stage === "paired" && JSON.stringify(s.taskbags) !== JSON.stringify(list) ? { ...s, taskbags: list } : s)),
          );
        })
        .catch(() => setStage({ stage: "no_app" }));
    }, WATCH_MS);
    return () => window.clearInterval(timer);
  }, [pairedDevice, token, start]);

  // The desktop app's own interface reads through this (one per paired token). Losing the desktop
  // app or the pairing drops what it showed at once (R83: no old data).
  const messenger = useMemo(
    () =>
      token
        ? lanMessenger(
            token,
            (why) => (why === "no_app" ? setStage({ stage: "no_app" }) : void start()),
            undefined,
            withDecisions,
          )
        : null,
    [token, start],
  );

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
        setToken(r.token);
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

  const root = stage.stage === "paired" ? shownRoot(stage.taskbags, chosen) : null;
  if (stage.stage === "paired" && root && messenger) {
    return (
      <App
        key={root}
        messenger={messenger}
        companion={{ root, computer, onOthers: stage.taskbags.length > 1 ? () => setChosen(null) : undefined }}
      />
    );
  }

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
              <ul className="companion-taskbags">
                {stage.taskbags.map((b) => (
                  <li key={b.root}>
                    <button type="button" onClick={() => setChosen(b.root)}>
                      {b.name || b.root}
                    </button>
                  </li>
                ))}
              </ul>
            </>
          ) : (
            <p className="muted">{t("companion.none_open")}</p>
          )}
          {computer && <p className="muted small">{t("companion.from", { computer })}</p>}
        </>
      )}
      <p className="muted small">{t("companion.warning")}</p>
    </main>
  );
}
