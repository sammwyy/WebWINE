import type { RuntimeBridge } from "../runtime-bridge.js";
import { openProcessConsole } from "./process-console.js";

export function openProcessWindow(path: string, runtime: RuntimeBridge) {
  openProcessConsole(path, runtime);
}
