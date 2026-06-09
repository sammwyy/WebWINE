export function dirname(path: string) {
  const parts = path.split("\\");
  parts.pop();
  return parts.join("\\") || path;
}

const VIDEO_EXTS = [".mp4", ".webm", ".mkv", ".avi", ".mov"];
const AUDIO_EXTS = [".mp3", ".wav", ".ogg", ".flac", ".m4a"];
const MEDIA_EXTS = [...VIDEO_EXTS, ...AUDIO_EXTS];

export function isMedia(name: string) {
  const lower = name.toLowerCase();
  return MEDIA_EXTS.some((ext) => lower.endsWith(ext));
}

export function isVideoFile(name: string) {
  const lower = name.toLowerCase();
  return VIDEO_EXTS.some((ext) => lower.endsWith(ext));
}

export const formatTime = (time: number) => {
  if (isNaN(time)) return "0:00";
  const minutes = Math.floor(time / 60);
  const seconds = Math.floor(time % 60);
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
};
