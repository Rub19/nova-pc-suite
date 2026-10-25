import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
document.documentElement.dataset.theme =
  localStorage.getItem("nova-theme") === "light" ? "light" : "dark";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
