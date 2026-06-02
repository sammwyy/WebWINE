import type { RuntimeBridge } from "../runtime-bridge.js";
import { openPeInspector } from "./pe-inspector.js";

export function openProcessWindow(path: string, runtime: RuntimeBridge) {
  openPeInspector(path, runtime);
}
