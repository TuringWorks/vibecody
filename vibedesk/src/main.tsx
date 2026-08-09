import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { migrateLegacyVibexStorage } from "./migrate";
import "./styles/tokens.css";
import "./App.css";
// Settings screens ship their own CSS with the shared package; it must come
// after tokens.css, whose variables it consumes.
import "@vibe/shared/settings/settings.css";

// Carry pre-rename (VibeX) theme prefs forward before the app reads them.
migrateLegacyVibexStorage();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
