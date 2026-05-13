import { getCurrentWindow } from "@tauri-apps/api/window";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Menu from "./Menu";
import Settings from "./Settings";
import "./index.css";

const label = getCurrentWindow().label;
const Root = label === "menu" ? Menu : label === "settings" ? Settings : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
