import { createRoot } from "react-dom/client";

import "@/shared/styles/main.css";
import "./registry/themes";
import { App } from "./App";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(<App />);
}
