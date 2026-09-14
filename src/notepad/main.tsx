import React from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import NotepadApp from "./NotepadApp";
import {
  applyTheme,
  getStoredTheme,
  syncThemeFromSettings,
} from "@/lib/utils/theme";
import type { Theme } from "@/bindings";
import "@/App.css";
import "@/i18n";

// A separate webview from the settings window, so the notepad has to set
// `data-theme` on its own document: last-known theme before render (shared
// localStorage) to avoid a flash, reconcile with the persisted setting in
// case the notepad booted first, then follow live changes.
applyTheme(getStoredTheme());
syncThemeFromSettings();
listen<Theme>("theme-changed", (event) => applyTheme(event.payload));

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <NotepadApp />
  </React.StrictMode>,
);
