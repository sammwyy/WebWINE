/** Web Audio beep helper used by Win32 MessageBeep emulation. */

let audioCtx: AudioContext | null = null;

export function beep(freq: number, durationMs: number): void {
  try {
    audioCtx ??= new AudioContext();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = "square";
    osc.frequency.value = freq > 0 ? freq : 800;
    gain.gain.value = 0.08;
    osc.connect(gain).connect(audioCtx.destination);
    const now = audioCtx.currentTime;
    osc.start(now);
    osc.stop(now + Math.min(Math.max(durationMs, 50), 2000) / 1000);
  } catch {
    // Audio context not available in this environment.
  }
}
