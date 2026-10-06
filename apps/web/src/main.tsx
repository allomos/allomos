import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
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
