/**
 * Persistent WebWINE configuration: one localStorage entry per key,
 * `ww_config_<key>` → JSON value. Storage failures (private mode, quota)
 * never break the shell; the value just isn't remembered.
 */

const PREFIX = "ww_config_";

export function getConfig<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

export function setConfig<T>(key: string, value: T): void {
  try {
    if (value === undefined || value === null) localStorage.removeItem(PREFIX + key);
    else localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    /* storage unavailable or full — keep the in-memory value only */
  }
}
