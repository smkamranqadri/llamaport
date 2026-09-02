import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initAppearance } from "./theme";

// Before the first render: the window opening in one palette and correcting itself a
// moment later is worse than a moment of nothing.
initAppearance();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
