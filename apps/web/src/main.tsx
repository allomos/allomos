import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { desktopMessenger } from "./messenger";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App messenger={desktopMessenger} />
  </StrictMode>,
);
