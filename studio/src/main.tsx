import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { PublicDemo } from "./PublicDemo";
import { chooseSourceKind } from "./lib/source-factory";

// The static web build (npm run build:web) sets VITE_DEMO_MODE=replay. The
// desktop Tauri shell never sets it, so it always gets the full App, which
// talks to the bundled engine. This is the same construction site
// getDemoSource() reads; here it picks the surface, there it picks the data.
const isReplay = chooseSourceKind(import.meta.env as unknown as Record<string, string | undefined>) === "replay";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isReplay ? <PublicDemo /> : <App />}
  </React.StrictMode>,
);
