import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import Menu from "./Menu";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./index.css";

const label = getCurrentWindow().label;
const Root = label === "menu" ? Menu : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <Root />
    </ErrorBoundary>
  </React.StrictMode>,
);
