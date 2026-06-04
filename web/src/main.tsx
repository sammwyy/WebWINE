import { createRoot } from "react-dom/client";
import { FluentProvider, webDarkTheme } from "@fluentui/react-components";

import { App } from "./App";
import "@/shared/styles/main.css";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <FluentProvider theme={webDarkTheme}>
      <App />
    </FluentProvider>,
  );
}
