import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ElementLabels } from "./ElementLabels";
import { I18nProvider } from "./i18n";
import { desktopMessenger } from "./messenger";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <I18nProvider systemLanguages={desktopMessenger.systemLanguages}>
      <App messenger={desktopMessenger} />
    </I18nProvider>
  </StrictMode>,
);

// Number labels (Ctrl+Shift+S) get a root of their own, so they still work if the app fails.
createRoot(document.body.appendChild(document.createElement("div"))).render(<ElementLabels />);
