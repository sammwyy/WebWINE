import { useEffect, useState, useRef } from "react";
import * as jsmediatags from "jsmediatags";
import { useWindowStore } from "@/state/windowStore";
import { WindowTitlebar } from "@/modules/windows/WindowTitlebar";
import type {
  RuntimeBridge,
  DirectoryEntry,
} from "@/core/bridge/runtime-bridge";
import { NavigationRegular, MoviesAndTvRegular } from "@fluentui/react-icons";
import { basename } from "@/shared/lib/utils";

// Refactored sub-components & utils
import { dirname, isMedia, isVideoFile } from "./lib/utils";
import { MediaSidebar } from "./components/MediaSidebar";
import { AudioOverlay } from "./components/AudioOverlay";
import { MediaControls } from "./components/MediaControls";

export async function openMediaPlayer(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Media Player";
  const icon = `${import.meta.env.BASE_URL}theme/icons/apps/mediaplayer.webp`;

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: path ? `${name} - WebWINE: Media Player` : "WebWINE: Media Player",
    icon,
    width: 850,
    height: 550,
    hideTitlebar: true,
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

  // Media Controls State
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [isMuted, setIsMuted] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [metadata, setMetadata] = useState<{
    title?: string;
    artist?: string;
    album?: string;
    pictureUrl?: string;
  } | null>(null);
  const controlsTimeoutRef = useRef<NodeJS.Timeout | null>(null);

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
          const musicEntries = await runtime
            .listDir("C:\\Users\\guest\\Music")
            .catch(() => []);
          const videoEntries = await runtime
            .listDir("C:\\Users\\guest\\Videos")
            .catch(() => []);
          entries = [...musicEntries, ...videoEntries];
        }

        if (!alive) return;

        let mediaEntries = entries.filter(
          (e) => e.kind === "file" && isMedia(e.name),
        );
        mediaEntries.sort((a, b) => a.name.localeCompare(b.name));

        if (targetFile) {
          // Ensure target is in the list
          const targetEntry = mediaEntries.find(
            (e) => e.name.toLowerCase() === targetFile,
          );
          if (!targetEntry) {
            mediaEntries.unshift({
              name: basename(initialPath),
              path: initialPath,
              kind: "file",
              size: 0,
            });
          }
        }

        setPlaylist(mediaEntries);

        if (mediaEntries.length > 0) {
          if (targetFile) {
            const idx = mediaEntries.findIndex(
              (e) => e.name.toLowerCase() === targetFile,
            );
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

    return () => {
      alive = false;
    };
  }, [initialPath, runtime]);

  // Handle Fullscreen changes
  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () =>
      document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  const handleMouseMove = () => {
    setControlsVisible(true);
    if (controlsTimeoutRef.current) clearTimeout(controlsTimeoutRef.current);
    controlsTimeoutRef.current = setTimeout(() => {
      if (isPlaying) setControlsVisible(false);
    }, 2500);
  };

  const togglePlay = () => {
    if (videoRef.current) {
      if (videoRef.current.paused) videoRef.current.play();
      else videoRef.current.pause();
    }
  };

  const toggleMute = () => {
    if (videoRef.current) {
      videoRef.current.muted = !isMuted;
      setIsMuted(!isMuted);
    }
  };

  const handleVolumeChange = (val: number) => {
    setVolume(val);
    if (videoRef.current) {
      videoRef.current.volume = val;
      if (val > 0 && isMuted) {
        videoRef.current.muted = false;
        setIsMuted(false);
      }
    }
  };

  const handleSeek = (time: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = time;
      setCurrentTime(time);
    }
  };

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      containerRef.current?.requestFullscreen().catch(() => {});
    } else {
      document.exitFullscreen().catch(() => {});
    }
  };

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
        useWindowStore
          .getState()
          .setTitle(id, `${currentMedia.name} - WebWINE: Media Player`);
      }
    }

    runtime
      .readFile(currentMedia.path)
      .then((bytes) => {
        if (!alive) return;
        const blob = new Blob([bytes]);
        const url = URL.createObjectURL(blob);
        setCurrentUrl(url);

        if (!isVideoFile(currentMedia.name)) {
          jsmediatags.read(blob as any, {
            onSuccess: (tag) => {
              if (!alive) return;
              const { title, artist, album, picture } = tag.tags;
              let pictureUrl = undefined;
              if (picture) {
                const picBlob = new Blob([new Uint8Array(picture.data)], {
                  type: picture.format,
                });
                pictureUrl = URL.createObjectURL(picBlob);
              }
              setMetadata({ title, artist, album, pictureUrl });
            },
            onError: (error) => {
              console.warn("Error reading tags:", error);
            },
          });
        }

        setLoading(false);
      })
      .catch((err) => {
        if (!alive) return;
        setError(String(err));
        setLoading(false);
      });

    return () => {
      alive = false;
      setCurrentUrl((url) => {
        if (url) URL.revokeObjectURL(url);
        return null;
      });
      setMetadata((prev) => {
        if (prev?.pictureUrl) URL.revokeObjectURL(prev.pictureUrl);
        return null;
      });
    };
  }, [currentMedia, runtime, winId]);

  return (
    <div className="flex w-full h-full bg-[#111111] select-none font-[var(--system-font)] text-[#f2f2f2] overflow-hidden">
      {/* Sidebar Playlist */}
      <MediaSidebar
        sidebarOpen={sidebarOpen}
        initialPath={initialPath}
        playlist={playlist}
        currentIndex={currentIndex}
        loading={loading}
        onSelect={(idx) => setCurrentIndex(idx)}
      />

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0 bg-[#000000] relative">
        {/* Top Bar overlays over video slightly or sits on top */}
        <WindowTitlebar
          windowId={winId?.() || ""}
          className="!bg-[#111111] !border-[#222222] !border-b !h-12 !px-4"
        >
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="w-8 h-8 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors window-controls"
            title="Toggle Sidebar"
          >
            <NavigationRegular fontSize={20} />
          </button>
          <div className="flex-1 truncate font-semibold text-[13px] ml-2">
            {metadata?.title
              ? `${metadata.title}${metadata.artist ? ` - ${metadata.artist}` : ""}`
              : currentMedia?.name || "Media Player"}
          </div>
        </WindowTitlebar>

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
            <div
              ref={containerRef}
              className="absolute inset-0 flex flex-col items-center justify-center bg-black overflow-hidden group"
              onMouseMove={handleMouseMove}
              onMouseLeave={() => setControlsVisible(false)}
            >
              <video
                ref={videoRef}
                src={currentUrl}
                autoPlay
                onClick={togglePlay}
                onPlay={() => setIsPlaying(true)}
                onPause={() => setIsPlaying(false)}
                onTimeUpdate={() =>
                  setCurrentTime(videoRef.current?.currentTime || 0)
                }
                onLoadedMetadata={() => {
                  setDuration(videoRef.current?.duration || 0);
                  if (videoRef.current) {
                    videoRef.current.volume = volume;
                    videoRef.current.muted = isMuted;
                  }
                }}
                onEnded={() => {
                  if (currentIndex < playlist.length - 1) {
                    setCurrentIndex(currentIndex + 1);
                  }
                }}
                className="max-w-full max-h-full w-full h-full outline-none object-contain"
              />

              {/* Audio Center Info Overlay */}
              {!isVideo && (
                <AudioOverlay
                  metadata={metadata}
                  fallbackName={currentMedia?.name || ""}
                />
              )}

              {/* Custom Fluent UI Controls */}
              <MediaControls
                controlsVisible={controlsVisible}
                isPlaying={isPlaying}
                currentTime={currentTime}
                duration={duration}
                volume={volume}
                isMuted={isMuted}
                isFullscreen={isFullscreen}
                onPlayToggle={togglePlay}
                onMuteToggle={toggleMute}
                onFullscreenToggle={toggleFullscreen}
                onSeek={handleSeek}
                onVolumeChange={handleVolumeChange}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
