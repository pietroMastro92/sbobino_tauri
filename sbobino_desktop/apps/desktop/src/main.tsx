import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

const standaloneSettingsWindow =
  new URLSearchParams(window.location.search).get("window") === "settings";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App standaloneSettingsWindow={standaloneSettingsWindow} />
  </React.StrictMode>,
);
