// File → Devices on Your Network… (Task-A04-02-01): "Allow devices on your network to view", off by
// default and remembered (approver, 2026-10-25). While on, the desktop app serves this interface
// over self-signed HTTPS on the local network: the dialog shows the address, its QR code for a
// phone, and the certificate's fingerprint, and says why the browser warns once and why the
// system may ask about the firewall. Pairing (A04-02-02): a device asking to connect is approved or
// refused here, the approved one's 9-digit code is shown until it is typed or runs out, and the
// paired devices are listed, each with Remove. The companion's view comes with A04-02-03.

import { useCallback, useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { LanStatus, Messenger, PairingView } from "./messenger";

/** `m:ss` for a countdown in seconds. */
export function countdown(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function Pairing({ messenger }: { messenger: Messenger }) {
  const { t } = useI18n();
  const [view, setView] = useState<PairingView | null>(null);
  const reload = useCallback(() => void messenger.lanPairing().then(setView, () => setView(null)), [messenger]);
  useEffect(() => {
    reload();
    // Once a second: the code's countdown, and "last seen".
    const timer = window.setInterval(reload, 1000);
    const stop = messenger.onPairingChanged(reload);
    return () => {
      window.clearInterval(timer);
      void stop.then((f) => f());
    };
  }, [messenger, reload]);
  if (!view) return null;
  const when = (utc: string) => new Date(utc).toLocaleString();
  return (
    <div className="lan-pairing">
      <h3>{t("lan.pairing")}</h3>
      {view.asking.length === 0 && <p className="muted small">{t("lan.pairing_how")}</p>}
      {view.asking.map((a) => (
        <div key={a.id} className="lan-ask banner">
          {a.code === null ? (
            <>
              <p>{t("lan.asks", { name: a.name })}</p>
              <div className="row">
                <button type="button" className="primary" onClick={() => void messenger.answerPairing(a.id, true).then(setView)}>
                  {t("lan.approve")}
                </button>
                <button type="button" onClick={() => void messenger.answerPairing(a.id, false).then(setView)}>
                  {t("lan.refuse")}
                </button>
              </div>
            </>
          ) : (
            <>
              <p>{t("lan.type_code", { name: a.name })}</p>
              <p className="lan-code" aria-label={t("lan.code_label")}>{a.code}</p>
              <p className="muted small">{t("lan.code_rules", { time: countdown(a.expires_in ?? 0), tries: a.tries_left ?? 0 })}</p>
            </>
          )}
        </div>
      ))}
      <h3>{t("lan.devices")}</h3>
      {view.devices.length === 0 ? (
        <p className="muted small">{t("lan.no_devices")}</p>
      ) : (
        <ul className="lan-devices">
          {view.devices.map((d) => (
            <li key={d.id}>
              <span>
                <strong>{d.name}</strong>
                <span className="muted small">
                  {" "}
                  {t("lan.paired_at", { when: when(d.paired_at) })}
                  {d.last_seen ? ` · ${t("lan.last_seen", { when: when(d.last_seen) })}` : ""}
                </span>
              </span>
              <button type="button" onClick={() => void messenger.removePairedDevice(d.id).then(setView)}>
                {t("lan.remove")}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function LanDialog({ messenger, onClose }: { messenger: Messenger; onClose: () => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<LanStatus | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void messenger.lanStatus().then(setStatus, () => setStatus(null));
  }, [messenger]);

  const toggle = async (on: boolean) => {
    setBusy(true);
    try {
      setStatus(await messenger.setLanViewing(on));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" role="presentation">
      <section className="dialog lan-dialog" role="dialog" aria-modal="true" aria-labelledby="lan-title">
        <h2 id="lan-title">{t("lan.title")}</h2>
        <div className="dialog-body form">
          <label className="lan-switch">
            <input type="checkbox" checked={!!status?.on} disabled={busy || !status} onChange={(e) => void toggle(e.target.checked)} />{" "}
            {t("lan.switch")}
          </label>
          <p className="muted small">{t("lan.body")}</p>
          {status?.error && <p className="banner error">{status.error}</p>}
          {status?.on && (
            <>
              {status.url ? (
                <div className="lan-address">
                  {/* The SVG comes from the app itself (qrcode crate), never from the network. */}
                  {status.qr_svg && <div className="lan-qr" aria-label={t("lan.qr", { url: status.url })} dangerouslySetInnerHTML={{ __html: status.qr_svg }} />}
                  <p>
                    {t("lan.open")} <code>{status.url}</code>
                  </p>
                </div>
              ) : (
                <p className="banner">{t("lan.no_network", { port: status.port ?? "" })}</p>
              )}
              {status.port !== null && status.port !== status.default_port && (
                <p className="muted small">{t("lan.port_moved", { port: status.port, default: status.default_port })}</p>
              )}
              <p className="small">{t("lan.warning")}</p>
              {status.fingerprint && (
                <p className="muted small">
                  {t("lan.fingerprint")} <code className="lan-fingerprint">{status.fingerprint}</code>
                </p>
              )}
              <p className="muted small">{t("lan.firewall")}</p>
              <Pairing messenger={messenger} />
            </>
          )}
        </div>
        <div className="row">
          <button type="button" className="primary" onClick={onClose}>
            {t("lan.close")}
          </button>
        </div>
      </section>
    </div>
  );
}
