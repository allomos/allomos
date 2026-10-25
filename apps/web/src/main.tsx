import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { AboutWindow } from "./AboutWindow";
import { CompanionPage } from "./CompanionPage";
import { ElementLabels } from "./ElementLabels";
import { I18nProvider } from "./i18n";
import { desktopMessenger } from "./messenger";
import "./styles.css";

// `?about`: the app's own About window (Task-A07-28); otherwise a project window. Outside the
// desktop app (a phone opening the address of "Devices on your network", Task-A04-02-01) there is
// no app to talk to: the companion page.
const about = new URLSearchParams(window.location.search).has("about");
const inDesktopApp = "__TAURI_INTERNALS__" in window;
const browserLanguages = () => Promise.resolve([...(navigator.languages ?? [])]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <I18nProvider systemLanguages={inDesktopApp ? desktopMessenger.systemLanguages : browserLanguages}>
      {!inDesktopApp ? <CompanionPage /> : about ? <AboutWindow messenger={desktopMessenger} /> : <App messenger={desktopMessenger} />}
    </I18nProvider>
  </StrictMode>,
);

// Number labels (Ctrl+Shift+S) get a root of their own, so they still work if the app fails.
createRoot(document.body.appendChild(document.createElement("div"))).render(<ElementLabels />);
