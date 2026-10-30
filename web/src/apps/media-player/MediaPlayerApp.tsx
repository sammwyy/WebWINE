/**
 * Media Player — modelled on Windows Media Player 12 (Windows 7, also shipped
 * with Vista-era looks and Windows 10): library view (navigation bar, command
 * bar, library tree, track list, Play pane) and a Now Playing view, with the
 * classic glossy black transport bar. Its look is fixed (only the window frame
 * follows the theme), like the real one.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import * as jsmediatags from "jsmediatags";
import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge, DirectoryEntry } from "@/core/bridge/runtime-bridge";
import { basename, formatSize } from "@/shared/lib/utils";
import { iconUrl } from "@/shared/lib/personalization";
import { useThemeVariant } from "@/shared/lib/theme-registry";
import { ContextMenu, type MenuItem } from "@/modules/desktop/ContextMenu";
import { dirname, isMedia, isVideoFile } from "./lib/utils";
import { MediaLibraryTree, type LibraryView } from "./components/MediaSidebar";
import { AudioOverlay } from "./components/AudioOverlay";
import { MediaControls } from "./components/MediaControls";
import { GridRegular } from "@fluentui/react-icons";
import "./wmp.css";

const MUSIC_DIR = "C:\\Users\\guest\\Music";
const VIDEOS_DIR = "C:\\Users\\guest\\Videos";

export async function openMediaPlayer(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Windows Media Player";
  let winId = "";
  winId = useWindowStore.getState().openWindow({
    title: path ? `${name} - WebWINE: Media Player` : "WebWINE: Media Player",
    icon: iconUrl("apps/mediaplayer"),
    width: 900,
    height: 580,
    minWidth: 560,
    minHeight: 360,
    surface: "none",
    content: <MediaPlayerApp initialPath={path} runtime={runtime} winId={() => winId} />,
  });
}

interface Metadata {
  title?: string;
  artist?: string;
  album?: string;
  pictureUrl?: string;
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
  const [library, setLibrary] = useState<DirectoryEntry[]>([]);
  const [queue, setQueue] = useState<DirectoryEntry[]>([]);
  const [currentIndex, setCurrentIndex] = useState(-1);
  const [currentUrl, setCurrentUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<LibraryView>("music");
  const [mode, setMode] = useState<"library" | "nowplaying">(initialPath ? "nowplaying" : "library");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const videoRef = useRef<HTMLVideoElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(0.8);
  const [isMuted, setIsMuted] = useState(false);
  const [shuffle, setShuffle] = useState(false);
  const [repeat, setRepeat] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [metadata, setMetadata] = useState<Metadata | null>(null);

  /** WMP 11 look on XP / Vista, WMP 12 on 7 / 10 (see `variants` in the theme). */
  const variant = useThemeVariant("mediaPlayer", "wmp12");
  const current = queue[currentIndex];
  const isVideo = current ? isVideoFile(current.name) : false;

  /* load the library (Music + Videos, or the launched file's folder) */
  useEffect(() => {
    let alive = true;
    (async () => {
      setLoading(true);
      try {
        const dirs = initialPath ? [dirname(initialPath)] : [MUSIC_DIR, VIDEOS_DIR];
        const lists = await Promise.all(dirs.map((d) => runtime.listDir(d).catch(() => [] as DirectoryEntry[])));
        if (!alive) return;
        const media = lists
          .flat()
          .filter((e) => e.kind === "file" && isMedia(e.name))
          .sort((a, b) => a.name.localeCompare(b.name));
        if (initialPath && !media.some((e) => e.path.toLowerCase() === initialPath.toLowerCase())) {
          media.unshift({ name: basename(initialPath), path: initialPath, kind: "file", size: 0 });
        }
        setLibrary(media);
        if (initialPath) {
          setQueue(media);
          setCurrentIndex(Math.max(0, media.findIndex((e) => e.path.toLowerCase() === initialPath.toLowerCase())));
          setView(isVideoFile(initialPath) ? "videos" : "music");
        }
      } catch (err) {
        if (alive) setError(String(err));
      } finally {
        if (alive) setLoading(false);
      }
    })();
    return () => {
      alive = false;
    };
  }, [initialPath, runtime]);

  /* load the current item */
  useEffect(() => {
    if (!current) {
      setCurrentUrl(null);
      return;
    }
    let alive = true;
    setError(null);
    const id = winId?.();
    if (id) useWindowStore.getState().setTitle(id, `${current.name} - WebWINE: Media Player`);

    runtime
      .readFile(current.path)
      .then((bytes) => {
        if (!alive) return;
        const blob = new Blob([bytes]);
        setCurrentUrl(URL.createObjectURL(blob));
        if (!isVideoFile(current.name)) {
          jsmediatags.read(blob as never, {
            onSuccess: (tag) => {
              if (!alive) return;
              const { title, artist, album, picture } = tag.tags;
              const pictureUrl = picture
                ? URL.createObjectURL(new Blob([new Uint8Array(picture.data)], { type: picture.format }))
                : undefined;
              setMetadata({ title, artist, album, pictureUrl });
            },
            onError: () => {},
          });
        }
      })
      .catch((err) => alive && setError(String(err)));

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
  }, [current, runtime, winId]);

  useEffect(() => {
    const onChange = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  /* playback */
  const playList = (items: DirectoryEntry[], startPath: string) => {
    setQueue(items);
    setCurrentIndex(Math.max(0, items.findIndex((e) => e.path === startPath)));
    if (isVideoFile(startPath)) setMode("nowplaying");
  };

  const togglePlay = () => {
    const v = videoRef.current;
    if (!v) {
      if (visible.length > 0) playList(visible, selected ?? visible[0].path);
      return;
    }
    if (v.paused) void v.play();
    else v.pause();
  };

  const stop = () => {
    if (!videoRef.current) return;
    videoRef.current.pause();
    videoRef.current.currentTime = 0;
  };

  const step = (dir: 1 | -1) => {
    if (queue.length === 0) return;
    if (shuffle) setCurrentIndex(Math.floor(Math.random() * queue.length));
    else setCurrentIndex((i) => (i + dir + queue.length) % queue.length);
  };

  const onEnded = () => {
    if (shuffle || repeat || currentIndex < queue.length - 1) step(1);
  };

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) stageRef.current?.requestFullscreen().catch(() => {});
    else document.exitFullscreen().catch(() => {});
  };

  /* library list */
  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return library.filter((e) => {
      if (view === "music" && isVideoFile(e.name)) return false;
      if (view === "videos" && !isVideoFile(e.name)) return false;
      return !q || e.name.toLowerCase().includes(q);
    });
  }, [library, search, view]);

  const viewLabel = view === "music" ? "Music" : view === "videos" ? "Videos" : "Playlists";
  const crumbs = mode === "nowplaying" ? ["Now Playing"] : ["Library", viewLabel, view === "music" ? "All music" : `All ${viewLabel.toLowerCase()}`];

  const openDropdown = (e: React.MouseEvent<HTMLElement>, items: MenuItem[]) => {
    const r = e.currentTarget.getBoundingClientRect();
    setMenu({ x: r.left, y: r.bottom, items });
  };

  const title = metadata?.title || (current ? current.name.replace(/\.[^.]+$/, "") : "");

  return (
    <div className={`wmp ${variant} ${mode}`}>
      {/* WMP 11 (XP / Vista): black glossy header with the main task tabs */}
      {variant === "wmp11" && (
        <div className="wmp11-head">
          <div className="wmp-navbtns">
            <button type="button" className="wmp-round" title="Back" disabled={mode === "library"} onClick={() => setMode("library")}>
              ◀
            </button>
            <button type="button" className="wmp-round" title="Forward" disabled={mode === "nowplaying" || !current} onClick={() => setMode("nowplaying")}>
              ▶
            </button>
          </div>
          <div className="wmp11-tabs">
            <button type="button" className={mode === "nowplaying" ? "active" : ""} onClick={() => setMode("nowplaying")}>
              Now Playing
            </button>
            <button type="button" className={mode === "library" ? "active" : ""} onClick={() => setMode("library")}>
              Library
            </button>
            <button type="button" disabled>Rip</button>
            <button type="button" disabled>Burn</button>
            <button type="button" disabled>Sync</button>
          </div>
        </div>
      )}

      {/* WMP 12 (7 / 10): navigation bar with breadcrumb and Play/Burn/Sync tabs */}
      {variant === "wmp12" && mode === "library" && (
      <div className="wmp-nav">
        <div className="wmp-navbtns">
          <button type="button" className="wmp-round" title="Back" disabled>
            ◀
          </button>
          <button type="button" className="wmp-round" title="Go to Now Playing" disabled={!current} onClick={() => setMode("nowplaying")}>
            ▶
          </button>
        </div>
        <div className="wmp-crumbs">
          {crumbs.map((c, i) => (
            <span key={c} className="wmp-crumb" onClick={() => i === 0 && mode === "library" && setView("music")}>
              {c}
              {i < crumbs.length - 1 && <span className="wmp-crumb-sep">▸</span>}
            </span>
          ))}
        </div>
        <div className="wmp-tabs">
          <span className="active">Play</span>
          <span>Burn</span>
          <span>Sync</span>
        </div>
      </div>
      )}

      {/* command bar */}
      {mode === "library" && (
        <div className="wmp-commands">
          <button
            type="button"
            className="wmp-cmd"
            onClick={(e) =>
              openDropdown(e, [
                { label: "Sort by name", action: () => setLibrary((l) => [...l].sort((a, b) => a.name.localeCompare(b.name))) },
                { label: "Sort by size", action: () => setLibrary((l) => [...l].sort((a, b) => b.size - a.size)) },
              ])
            }
          >
            Organize ▾
          </button>
          <button type="button" className="wmp-cmd" disabled>
            Stream ▾
          </button>
          <button
            type="button"
            className="wmp-cmd"
            disabled={visible.length === 0}
            onClick={() => playList(visible, visible[0].path)}
          >
            Create playlist ▾
          </button>
          {variant === "wmp11" && <span className="wmp11-crumbs">{crumbs.join("  ▸  ")}</span>}
          <div className="flex-1" />
          <input
            className="wmp-search"
            type="search"
            placeholder="Search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      )}

      <div className="wmp-main">
        {mode === "library" && (
          <>
            <MediaLibraryTree view={view} onSelect={setView} counts={{
              music: library.filter((e) => !isVideoFile(e.name)).length,
              videos: library.filter((e) => isVideoFile(e.name)).length,
              playlists: queue.length,
            }} />

            <div className="wmp-list" tabIndex={0}>
              <div className="wmp-list-head wmp-cols">
                <span>#</span>
                <span>Title</span>
                <span>Type</span>
                <span className="text-right">Size</span>
              </div>
              <div className="wmp-list-body" onClick={() => setSelected(null)}>
                {loading && <div className="wmp-empty">Loading library…</div>}
                {!loading && visible.length === 0 && (
                  <div className="wmp-empty">
                    {view === "playlists"
                      ? "Click Create playlist, or double-click an item to start playing."
                      : `There are no items in ${viewLabel}. Add files to ${view === "videos" ? VIDEOS_DIR : MUSIC_DIR}.`}
                  </div>
                )}
                {(view === "playlists" ? queue : visible).map((item, i) => {
                  const playing = current?.path === item.path;
                  return (
                    <div
                      key={item.path}
                      className={`wmp-row wmp-cols ${selected === item.path ? "selected" : ""} ${playing ? "playing" : ""}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelected(item.path);
                      }}
                      onDoubleClick={() => playList(view === "playlists" ? queue : visible, item.path)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setSelected(item.path);
                        setMenu({
                          x: e.clientX,
                          y: e.clientY,
                          items: [
                            { label: "Play", action: () => playList(view === "playlists" ? queue : visible, item.path) },
                            { label: "Add to play list", action: () => setQueue((q) => (q.some((x) => x.path === item.path) ? q : [...q, item])) },
                          ],
                        });
                      }}
                    >
                      <span>{playing && isPlaying ? "♪" : i + 1}</span>
                      <span className="truncate">{item.name.replace(/\.[^.]+$/, "")}</span>
                      <span>{isVideoFile(item.name) ? "Video" : "Audio"}</span>
                      <span className="text-right">{item.size ? formatSize(item.size) : ""}</span>
                    </div>
                  );
                })}
              </div>
            </div>

            <aside className="wmp-pane">
              <div className="wmp-pane-title">{queue.length > 0 ? "Unsaved list" : "Play list"}</div>
              <div className="wmp-pane-list">
                {queue.length === 0 && <div className="wmp-pane-hint">Drag items here to create a playlist, or double-click an item.</div>}
                {queue.map((item, i) => (
                  <div
                    key={item.path}
                    className={`wmp-pane-item ${i === currentIndex ? "playing" : ""}`}
                    onDoubleClick={() => setCurrentIndex(i)}
                  >
                    {item.name.replace(/\.[^.]+$/, "")}
                  </div>
                ))}
              </div>
              <div className="wmp-pane-foot">
                {queue.length} item{queue.length === 1 ? "" : "s"}
                {queue.length > 0 && (
                  <span className="wmp-link" onClick={() => { setQueue([]); setCurrentIndex(-1); }}>
                    Clear list
                  </span>
                )}
              </div>
            </aside>
          </>
        )}

        {/* the media element stays mounted so music keeps playing in the library view */}
        <div ref={stageRef} className={`wmp-stage ${mode === "nowplaying" ? "" : "hidden-stage"}`}>
          {error && <div className="wmp-stage-msg text-[#ff8a8a]">{error}</div>}
          {!current && !error && <div className="wmp-stage-msg">Nothing is playing. Go back to the library and double-click an item.</div>}
          {currentUrl && (
            <video
              ref={videoRef}
              src={currentUrl}
              autoPlay
              onClick={togglePlay}
              onDoubleClick={toggleFullscreen}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              onTimeUpdate={() => setCurrentTime(videoRef.current?.currentTime || 0)}
              onLoadedMetadata={() => {
                setDuration(videoRef.current?.duration || 0);
                if (videoRef.current) {
                  videoRef.current.volume = volume;
                  videoRef.current.muted = isMuted;
                }
              }}
              onEnded={onEnded}
              className="absolute inset-0 w-full h-full outline-none object-contain"
            />
          )}
          {currentUrl && !isVideo && <AudioOverlay metadata={metadata} fallbackName={current?.name || ""} />}
          {variant === "wmp12" && (
            <button type="button" className="wmp-np-switch" title="Switch to Library" onClick={() => setMode("library")}>
              <GridRegular />
            </button>
          )}
        </div>
      </div>

      <MediaControls
        title={title}
        subtitle={metadata?.artist}
        isPlaying={isPlaying}
        currentTime={currentTime}
        duration={duration}
        volume={volume}
        isMuted={isMuted}
        shuffle={shuffle}
        repeat={repeat}
        hasMedia={Boolean(currentUrl)}
        canPlay={Boolean(currentUrl) || visible.length > 0}
        nowPlaying={mode === "nowplaying"}
        showViewToggle={variant === "wmp11" || mode === "library"}
        isFullscreen={isFullscreen}
        onPlayToggle={togglePlay}
        onStop={stop}
        onPrev={() => step(-1)}
        onNext={() => step(1)}
        onShuffle={() => setShuffle((v) => !v)}
        onRepeat={() => setRepeat((v) => !v)}
        onMuteToggle={() => {
          if (videoRef.current) videoRef.current.muted = !isMuted;
          setIsMuted(!isMuted);
        }}
        onVolumeChange={(v) => {
          setVolume(v);
          if (videoRef.current) {
            videoRef.current.volume = v;
            if (v > 0 && isMuted) {
              videoRef.current.muted = false;
              setIsMuted(false);
            }
          }
        }}
        onSeek={(t) => {
          if (videoRef.current) videoRef.current.currentTime = t;
          setCurrentTime(t);
        }}
        onToggleView={() => setMode((m) => (m === "library" ? "nowplaying" : "library"))}
        onFullscreenToggle={toggleFullscreen}
      />

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setMenu(null)} />}
    </div>
  );
}
