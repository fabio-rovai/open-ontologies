import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// AppShell (mounted by App) is the single shell for both build targets: it
// reads DemoSource through the demo-store and picks its own chrome based on
// the same VITE_DEMO_MODE construction site source-factory.ts uses to pick
// live vs. replay data. There is no longer a second, parallel shell here for
// the static web build to fall back to.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
