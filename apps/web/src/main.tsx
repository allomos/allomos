import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AboutWindow } from "./AboutWindow";
import { CompanionPage } from "./CompanionPage";
import { ElementLabels } from "./ElementLabels";
import { I18nProvider } from "./i18n";
import { desktopMessenger, desktopTabs } from "./messenger";
import { SettingsWindow } from "./SettingsWindow";
import { WindowTabs } from "./WindowTabs";
import "./styles.css";

// `?about`: the app's own About window (Task-A07-28); `?settings`: its Settings window
// (Task-A01-marble320); otherwise a project window, with its tabs. Outside the desktop app (a phone opening the address of "Devices on your network", Task-A04-02-01) there is
// no app to talk to: the companion page.
const query = new URLSearchParams(window.location.search);
const about = query.has("about");
const settings = query.has("settings");
const inDesktopApp = "__TAURI_INTERNALS__" in window;
const browserLanguages = () => Promise.resolve([...(navigator.languages ?? [])]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <I18nProvider systemLanguages={inDesktopApp ? desktopMessenger.systemLanguages : browserLanguages}>
      {!inDesktopApp ? (
        <CompanionPage />
      ) : about ? (
        <AboutWindow messenger={desktopMessenger} />
      ) : settings ? (
        <SettingsWindow messenger={desktopMessenger} settings={desktopTabs} />
      ) : (
        <WindowTabs messenger={desktopMessenger} tabs={desktopTabs} />
      )}
    </I18nProvider>
  </StrictMode>,
);

// Number labels (Ctrl+Shift+S) get a root of their own, so they still work if the app fails.
createRoot(document.body.appendChild(document.createElement("div"))).render(<ElementLabels />);
