import { useEffect, useState, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge, DirectoryEntry } from "@/core/bridge/runtime-bridge";
import { basename } from "@/shared/lib/utils";
import { resolveIcon } from "@/shared/lib/icons/icon-resolver";
import { 
  NavigationRegular,
  MoviesAndTvRegular,
  MusicNote1Regular,
  VideoRegular,
  PlayRegular
} from "@fluentui/react-icons";

function dirname(path: string) {
  const parts = path.split("\\");
  parts.pop();
  return parts.join("\\") || path;
}

const VIDEO_EXTS = [".mp4", ".webm", ".mkv", ".avi", ".mov"];
const AUDIO_EXTS = [".mp3", ".wav", ".ogg", ".flac", ".m4a"];
const MEDIA_EXTS = [...VIDEO_EXTS, ...AUDIO_EXTS];

function isMedia(name: string) {
  const lower = name.toLowerCase();
  return MEDIA_EXTS.some(ext => lower.endsWith(ext));
}

function isVideoFile(name: string) {
  const lower = name.toLowerCase();
  return VIDEO_EXTS.some(ext => lower.endsWith(ext));
}

export async function openMediaPlayer(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Media Player";

  const isVideo = isVideoFile(name);
  const defaultIcon = isVideo 
    ? `${import.meta.env.BASE_URL}theme/icons/places/video.webp`
    : `${import.meta.env.BASE_URL}theme/icons/places/music.webp`;

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime,
  );

  const icon = resolved?.src || defaultIcon;

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: path ? `${name} - WebWINE: Media Player` : "WebWINE: Media Player",
    icon,
    width: 850,
    height: 550,
    content: (
      <MediaPlayerApp
        initialPath={path}
        runtime={runtime}
        winId={() => winId}
      />
    ),
  });
}

export function MediaPlayerApp({
  initialPath,
  runtime,
  winId,
}: {
  initialPath: string;
  runtime: RuntimeBridge;
  winId?: () => string;
}) {
  const [playlist, setPlaylist] = useState<DirectoryEntry[]>([]);
  const [currentIndex, setCurrentIndex] = useState(-1);
  const [currentUrl, setCurrentUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const currentMedia = playlist[currentIndex];
  const isVideo = currentMedia ? isVideoFile(currentMedia.name) : false;

  useEffect(() => {
    let alive = true;

    async function init() {
      try {
        setLoading(true);
        setError(null);

        let targetFile = "";
        let entries: DirectoryEntry[] = [];

        if (initialPath) {
          const isFile = basename(initialPath).includes(".");
          if (isFile) {
            const dirPath = dirname(initialPath);
            targetFile = basename(initialPath).toLowerCase();
            entries = await runtime.listDir(dirPath).catch(() => []);
          } else {
            entries = await runtime.listDir(initialPath).catch(() => []);
          }
        } else {
          // No path provided, grab user's Music and Videos libraries
          const musicEntries = await runtime.listDir("C:\\Users\\guest\\Music").catch(() => []);
          const videoEntries = await runtime.listDir("C:\\Users\\guest\\Videos").catch(() => []);
          entries = [...musicEntries, ...videoEntries];
        }

        if (!alive) return;

        let mediaEntries = entries.filter(e => e.kind === "file" && isMedia(e.name));
        mediaEntries.sort((a, b) => a.name.localeCompare(b.name));

        if (targetFile) {
          // Ensure target is in the list
          const targetEntry = mediaEntries.find(e => e.name.toLowerCase() === targetFile);
          if (!targetEntry) {
            mediaEntries.unshift({ name: basename(initialPath), path: initialPath, kind: "file", size: 0 });
          }
        }

        setPlaylist(mediaEntries);
        
        if (mediaEntries.length > 0) {
          if (targetFile) {
            const idx = mediaEntries.findIndex(e => e.name.toLowerCase() === targetFile);
            setCurrentIndex(Math.max(0, idx));
          } else {
            setCurrentIndex(0);
          }
        }

        if (mediaEntries.length <= 1 && initialPath) {
          setSidebarOpen(false); // Hide sidebar if only one item and explicitly launched
        }

      } catch (err) {
        if (alive) setError(String(err));
      } finally {
        if (alive) setLoading(false);
      }
    }

    init();

    return () => { alive = false; };
  }, [initialPath, runtime]);

  useEffect(() => {
    if (!currentMedia) {
      setCurrentUrl(null);
      return;
    }
    
    let alive = true;
    setLoading(true);
    setError(null);
    
    if (winId) {
      const id = winId();
      if (id) {
        useWindowStore.getState().setTitle(id, `${currentMedia.name} - WebWINE: Media Player`);
      }
    }

    runtime.readFile(currentMedia.path).then(bytes => {
      if (!alive) return;
      const blob = new Blob([bytes]);
      const url = URL.createObjectURL(blob);
      setCurrentUrl(url);
      setLoading(false);
    }).catch(err => {
      if (!alive) return;
      setError(String(err));
      setLoading(false);
    });

    return () => {
      alive = false;
      setCurrentUrl(url => {
        if (url) URL.revokeObjectURL(url);
        return null;
      });
    };
  }, [currentMedia, runtime, winId]);

  return (
    <div className="flex w-full h-full bg-[#111111] select-none font-[var(--system-font)] text-[#f2f2f2] overflow-hidden">
      
      {/* Sidebar Playlist */}
      <div 
        className={`flex-none flex flex-col bg-[#1a1a1a] border-r border-[#333333] transition-all duration-300 ease-in-out ${sidebarOpen ? "w-64" : "w-0 opacity-0 overflow-hidden"}`}
      >
        <div className="h-12 px-4 flex items-center border-b border-[#333333] font-semibold text-[13px] whitespace-nowrap">
          {initialPath ? "Folder Playlist" : "Media Library"}
        </div>
        <div className="flex-1 overflow-y-auto py-2 px-1 custom-scrollbar">
          {playlist.length === 0 && !loading && (
            <div className="text-center text-[#888] text-[12px] mt-4">
              No media found.
            </div>
          )}
          {playlist.map((item, idx) => {
            const active = idx === currentIndex;
            return (
              <div 
                key={item.path}
                onClick={() => setCurrentIndex(idx)}
                className={`flex items-center gap-3 px-3 py-2 mx-1 rounded cursor-pointer transition-colors text-[13px] ${
                  active 
                    ? "bg-[#333333] text-white" 
                    : "text-[#cccccc] hover:bg-[#2b2b2b]"
                }`}
              >
                <div className="flex-none flex items-center justify-center w-4 text-[#888888]">
                  {active ? <PlayRegular fontSize={14} className="text-white" /> : (isVideoFile(item.name) ? <VideoRegular fontSize={14} /> : <MusicNote1Regular fontSize={14} />)}
                </div>
                <div className="flex-1 truncate" title={item.name}>
                  {item.name}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0 bg-[#000000] relative">
        
        {/* Top Bar overlays over video slightly or sits on top */}
        <div className="h-12 flex-none flex items-center bg-[#111111] px-4 gap-4 z-10 border-b border-[#222222]">
          <button 
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="w-8 h-8 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Toggle Sidebar"
          >
            <NavigationRegular fontSize={20} />
          </button>
          <div className="flex-1 truncate font-semibold text-[13px]">
            {currentMedia?.name || "Media Player"}
          </div>
        </div>

        {/* Video / Audio Area */}
        <div className="flex-1 relative flex items-center justify-center">
          {loading && !currentUrl && (
            <div className="absolute inset-0 flex items-center justify-center text-[#888888]">
              Loading...
            </div>
          )}
          {error && (
            <div className="absolute inset-0 flex items-center justify-center text-[#ff6b6b] text-center p-4">
              {error}
            </div>
          )}
          {!loading && !error && !currentUrl && (
            <div className="absolute inset-0 flex flex-col items-center justify-center text-[#888888]">
              <MoviesAndTvRegular fontSize={48} className="mb-2 text-[#444]" />
              <span>Select media from the sidebar</span>
            </div>
          )}
          {currentUrl && (
            <video
              src={currentUrl}
              controls
              autoPlay
              onEnded={() => {
                // Auto play next
                if (currentIndex < playlist.length - 1) {
                  setCurrentIndex(currentIndex + 1);
                }
              }}
              className="max-w-full max-h-full w-full h-full outline-none"
              style={{
                 objectFit: isVideo ? "contain" : "contain",
              }}
            />
          )}
        </div>
      </div>

    </div>
  );
}

// Add simple custom scrollbar styles to the global context or use generic classes
// custom-scrollbar is a class we might need to define or rely on standard webwine scrollbars.
