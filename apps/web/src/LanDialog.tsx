// File → Devices on Your Network… (Task-A04-02-01): "Allow devices on your network to view", off by
// default and remembered (approver, 2026-10-25). While on, the desktop app serves this interface
// over self-signed HTTPS on the local network: the dialog shows the address, its QR code for a
// phone, and the certificate's fingerprint, and says why the browser warns once and why the
// system may ask about the firewall. Pairing (A04-02-02) and the companion's view (A04-02-03)
// come next; until then another device sees the page but no taskbag data.

import { useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { LanStatus, Messenger } from "./messenger";

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
              <p className="muted small">{t("lan.pairing_next")}</p>
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
