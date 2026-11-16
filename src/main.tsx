import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/rubik/600.css";
import "@fontsource/rubik/700.css";
import "@fontsource/pixelify-sans/500.css";
import "@fontsource/pixelify-sans/700.css";
import "./styles.css";
import { StoreProvider } from "./store";
import { App } from "./App";

// No browser context menu or text selection outside inputs: feels like an app, not a web page.
window.addEventListener("contextmenu", (e) => {
  const el = e.target as HTMLElement;
  if (!el.closest("input, textarea")) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <StoreProvider>
      <App />
    </StoreProvider>
  </React.StrictMode>,
);

// Icon-only buttons explain themselves on hover: reuse their accessible name as the tooltip
document.addEventListener("mouseover", (e) => {
  const el = (e.target as HTMLElement | null)?.closest?.("button[aria-label]:not([title])");
  if (el) el.setAttribute("title", el.getAttribute("aria-label") ?? "");
});
